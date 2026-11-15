use std::time::Duration;

use serde::Deserialize;

use crate::{
    core::error::ProviderError,
    providers::{http::is_retryable_status, retry::Retryable},
};

/// A source location within a GraphQL query where an error occurred.
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
pub struct AniListErrorLocation {
    /// 1-indexed line within the query document.
    pub line: i64,
    /// 1-indexed column within the query document.
    pub column: i64,
}

/// A GraphQL error object returned by the AniList API.
///
/// Models the full error shape so callers can inspect the machine-readable
/// `status` (used to derive [`AniListError::NotFound`]) instead of parsing the
/// human-readable `message`, which is fragile against rewording or localization.
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
pub struct AniListGqlError {
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable status code (mirrors the HTTP status; `404` for
    /// not-found, `429` for rate limiting, etc.).
    ///
    /// When AniList omits it on a non-2xx response, the client fills it in
    /// from the HTTP status.
    #[serde(default)]
    pub status: Option<i64>,
    /// Query locations the error refers to.
    #[serde(default)]
    pub locations: Vec<AniListErrorLocation>,
    /// Response path the error refers to (field names / list indices).
    #[serde(default)]
    pub path: Vec<serde_json::Value>,
    /// Implementation-specific error extensions.
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

/// Error type for the AniList provider.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AniListError {
    /// HTTP transport error (connection, DNS, TLS) — no response was received.
    ///
    /// Errors built by this crate (including via `From<reqwest::Error>`) have
    /// the request URL stripped with [`reqwest::Error::without_url`], matching
    /// the TMDB provider's redaction policy.
    #[error("HTTP transport error: {0}")]
    Http(#[source] reqwest::Error),

    /// The server returned an error HTTP status with no usable GraphQL body
    /// (e.g. a 5xx HTML page). Carries the `Retry-After` delay when present.
    #[error("AniList HTTP error {status}: {message}")]
    Status {
        /// HTTP response status code.
        status: u16,
        /// The `Retry-After` delay, if the response supplied one.
        retry_after: Option<Duration>,
        /// A short description (a truncated body, or a generic message).
        message: String,
    },

    /// One or more GraphQL errors were returned with no usable `data` — the
    /// root field the query needed (e.g. `Media` or `Page`) was null or
    /// missing. Errors without a `status` inherit a non-2xx HTTP status.
    ///
    /// Carries the `Retry-After` delay so a retry layer can honor rate limits
    /// signaled via GraphQL errors (AniList returns `429` this way).
    #[error("GraphQL errors: {}", .errors.iter().map(|e| e.message.as_str()).collect::<Vec<_>>().join("; "))]
    GraphQL {
        /// The structured GraphQL error objects.
        errors: Vec<AniListGqlError>,
        /// The `Retry-After` delay, if the response supplied one.
        retry_after: Option<Duration>,
    },

    /// The response body could not be parsed as a GraphQL envelope.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// A successful response contained no `data` field.
    #[error("no data returned from AniList")]
    NoData,

    /// The requested resource was not found (a GraphQL error with `status` 404).
    #[error("not found")]
    NotFound,

    /// A response reported a page number or total that does not fit the
    /// `u32` a [`Page`](crate::core::pagination::Page) carries (negative).
    ///
    /// The value is reported rather than wrapped, so a malformed page is
    /// never passed off as a real one. Not retryable.
    #[error("AniList reported an out-of-range `{field}`: {value}")]
    PageOutOfRange {
        /// The `pageInfo` field: `currentPage`, `lastPage`, or `total`.
        field: &'static str,
        /// The value AniList reported.
        value: i64,
    },

    /// Invalid configuration (rejected at construction time).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Wrap a transport error, stripping its request URL (consistent with
/// `TmdbError`, whose v3 API key travels in the URL).
impl From<reqwest::Error> for AniListError {
    fn from(err: reqwest::Error) -> Self {
        AniListError::Http(err.without_url())
    }
}

impl Retryable for AniListError {
    /// Whether this failure is transient and worth retrying: a 429/5xx status
    /// (surfaced either as `Status` or as a GraphQL error), or a connect/timeout
    /// transport error.
    fn is_retryable(&self) -> bool {
        match self {
            AniListError::Status { status, .. } => is_retryable_status(*status),
            AniListError::GraphQL { errors, .. } => errors
                .iter()
                .filter_map(|e| e.status)
                .any(|s| u16::try_from(s).is_ok_and(is_retryable_status)),
            AniListError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// The server-supplied `Retry-After` delay, if any.
    fn retry_after(&self) -> Option<Duration> {
        match self {
            AniListError::Status { retry_after, .. }
            | AniListError::GraphQL { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl From<AniListError> for ProviderError {
    fn from(err: AniListError) -> Self {
        match err {
            AniListError::Http(e) => ProviderError::Transport(e.without_url().to_string()),
            AniListError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            AniListError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            AniListError::NotFound => ProviderError::NotFound,
            AniListError::NoData => ProviderError::Other("no data returned".to_string()),
            e @ AniListError::PageOutOfRange { .. } => {
                ProviderError::Deserialization(e.to_string())
            }
            AniListError::Status {
                status,
                retry_after,
                message,
            } => ProviderError::from_http_status(status, message, retry_after),
            AniListError::GraphQL {
                ref errors,
                retry_after,
            } => {
                if errors.iter().any(|e| e.status == Some(429)) {
                    ProviderError::RateLimited { retry_after }
                } else if errors.iter().any(|e| e.status == Some(404)) {
                    ProviderError::NotFound
                } else if let Some(status) = most_severe_gql_status(errors) {
                    let message = errors
                        .iter()
                        .map(|e| e.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    // No error carries 404 or 429 here (both are handled
                    // above), so this yields `Auth` or `Api`.
                    ProviderError::from_http_status(status, message, retry_after)
                } else {
                    ProviderError::Other(err.to_string())
                }
            }
        }
    }
}

/// The most severe HTTP-error status among AniList GraphQL errors: the
/// highest `4xx`/`5xx` value (so `5xx` outranks `4xx`). Statuses outside that
/// range (e.g. the `200` AniList attaches to non-fatal errors) are ignored.
fn most_severe_gql_status(errors: &[AniListGqlError]) -> Option<u16> {
    errors
        .iter()
        .filter_map(|e| e.status)
        .filter_map(|s| u16::try_from(s).ok())
        .filter(|s| (400..=599).contains(s))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::http::test_support::{json_error, reqwest_builder_error};

    /// Table-driven: every `AniListError` variant maps to the pinned
    /// `ProviderError`.
    #[test]
    fn maps_every_anilist_error_branch() {
        let retry = Some(Duration::from_secs(5));
        let status = |status| AniListError::Status {
            status,
            retry_after: retry,
            message: "msg".into(),
        };
        for code in [401, 403] {
            match ProviderError::from(status(code)) {
                ProviderError::Auth(m) => assert_eq!(m, "msg", "status {code}"),
                other => panic!("status {code}: expected Auth, got {other:?}"),
            }
        }
        assert!(matches!(
            ProviderError::from(status(404)),
            ProviderError::NotFound
        ));
        match ProviderError::from(status(429)) {
            ProviderError::RateLimited { retry_after } => assert_eq!(retry_after, retry),
            other => panic!("expected RateLimited, got {other:?}"),
        }
        for code in [400, 500, 502] {
            match ProviderError::from(status(code)) {
                ProviderError::Api { status: s, message } => {
                    assert_eq!((s, message.as_str()), (code, "msg"));
                }
                other => panic!("status {code}: expected Api, got {other:?}"),
            }
        }

        let http = reqwest_builder_error();
        let http_text = http.to_string();
        match ProviderError::from(AniListError::Http(http)) {
            ProviderError::Transport(m) => assert_eq!(m, http_text),
            other => panic!("expected Transport, got {other:?}"),
        }

        let json = json_error();
        let json_text = json.to_string();
        match ProviderError::from(AniListError::Deserialization(json)) {
            ProviderError::Deserialization(m) => assert_eq!(m, json_text),
            other => panic!("expected Deserialization, got {other:?}"),
        }

        match ProviderError::from(AniListError::InvalidConfig("per_page".into())) {
            ProviderError::InvalidInput(m) => assert_eq!(m, "per_page"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
        assert!(matches!(
            ProviderError::from(AniListError::NotFound),
            ProviderError::NotFound
        ));
        match ProviderError::from(AniListError::NoData) {
            ProviderError::Other(m) => assert_eq!(m, "no data returned"),
            other => panic!("expected Other, got {other:?}"),
        }
        match ProviderError::from(AniListError::PageOutOfRange {
            field: "total",
            value: -1,
        }) {
            ProviderError::Deserialization(m) => {
                assert_eq!(m, "AniList reported an out-of-range `total`: -1");
            }
            other => panic!("expected Deserialization, got {other:?}"),
        }
    }

    /// A GraphQL 429 keeps the response's `retry_after`; an unclassified
    /// GraphQL error keeps the joined messages in `Other`.
    #[test]
    fn maps_anilist_graphql_retry_after_and_other_message() {
        let errors = |v| serde_json::from_value(v).expect("valid GraphQL error fixture");
        let limited = AniListError::GraphQL {
            errors: errors(serde_json::json!([{ "message": "slow down", "status": 429 }])),
            retry_after: Some(Duration::from_secs(9)),
        };
        match ProviderError::from(limited) {
            ProviderError::RateLimited { retry_after } => {
                assert_eq!(retry_after, Some(Duration::from_secs(9)));
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
        let unclassified = AniListError::GraphQL {
            errors: errors(serde_json::json!([{ "message": "a" }, { "message": "b" }])),
            retry_after: None,
        };
        match ProviderError::from(unclassified) {
            ProviderError::Other(m) => assert_eq!(m, "GraphQL errors: a; b"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn maps_anilist_error() {
        assert!(matches!(
            ProviderError::from(AniListError::NotFound),
            ProviderError::NotFound
        ));
        assert!(matches!(
            ProviderError::from(AniListError::Status {
                status: 429,
                retry_after: Some(Duration::from_secs(5)),
                message: "x".into(),
            }),
            ProviderError::RateLimited {
                retry_after: Some(_)
            }
        ));
        assert!(matches!(
            ProviderError::from(AniListError::InvalidConfig("x".into())),
            ProviderError::InvalidInput(_)
        ));
    }

    fn gql(errors: serde_json::Value) -> AniListError {
        AniListError::GraphQL {
            errors: serde_json::from_value(errors).expect("valid GraphQL error fixture"),
            retry_after: None,
        }
    }

    #[test]
    fn maps_anilist_graphql_5xx_to_api() {
        let err = ProviderError::from(gql(serde_json::json!([
            { "message": "Bad input", "status": 400 },
            { "message": "Internal", "status": 500 }
        ])));
        match err {
            ProviderError::Api { status, message } => {
                assert_eq!(status, 500, "5xx outranks 4xx");
                assert_eq!(message, "Bad input; Internal");
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn maps_anilist_graphql_special_statuses() {
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Internal", "status": 500 },
                { "message": "Too Many Requests.", "status": 429 }
            ]))),
            ProviderError::RateLimited { .. }
        ));
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Not Found.", "status": 404 }
            ]))),
            ProviderError::NotFound
        ));
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Invalid token", "status": 401 }
            ]))),
            ProviderError::Auth(_)
        ));
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Forbidden", "status": 403 }
            ]))),
            ProviderError::Auth(_)
        ));
        // A 5xx outranks an auth status.
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "Forbidden", "status": 403 },
                { "message": "Internal", "status": 502 }
            ]))),
            ProviderError::Api { status: 502, .. }
        ));
        // No HTTP-error status at all stays `Other`.
        assert!(matches!(
            ProviderError::from(gql(serde_json::json!([
                { "message": "no status" },
                { "message": "non-fatal", "status": 200 }
            ]))),
            ProviderError::Other(_)
        ));
    }

    /// GraphQL statuses outside `u16` (or outside 429/5xx) are not retryable.
    #[test]
    fn graphql_retryability_follows_the_shared_status_rule() {
        assert!(gql(serde_json::json!([{ "message": "a", "status": 503 }])).is_retryable());
        assert!(gql(serde_json::json!([{ "message": "a", "status": 429 }])).is_retryable());
        assert!(!gql(serde_json::json!([{ "message": "a", "status": 404 }])).is_retryable());
        assert!(!gql(serde_json::json!([{ "message": "a", "status": 65_965 }])).is_retryable());
        assert!(!gql(serde_json::json!([{ "message": "a", "status": -500 }])).is_retryable());
    }
}
