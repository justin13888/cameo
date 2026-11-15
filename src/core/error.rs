use std::time::Duration;

/// The unified error type returned by provider trait implementations.
///
/// Per ADR 0001, provider traits return this single concrete type (rather than a
/// per-provider associated error) so they are `dyn`-compatible. Each provider
/// maps its native error (`TmdbError`, `AniListError`) into `ProviderError` via
/// `From`.
///
/// # Matching on variants
///
/// ```
/// use cameo::core::error::ProviderError;
///
/// fn handle(err: ProviderError) {
///     match err {
///         ProviderError::NotFound => eprintln!("not found"),
///         ProviderError::RateLimited { retry_after } => {
///             eprintln!("rate limited; retry after {retry_after:?}");
///         }
///         ProviderError::Api { status, message } => eprintln!("HTTP {status}: {message}"),
///         other => eprintln!("{other}"),
///     }
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The requested resource was not found.
    #[error("resource not found")]
    NotFound,

    /// The provider does not support this operation.
    #[error("operation not supported by this provider")]
    Unsupported,

    /// The provider rate-limited the request.
    #[error("rate limited{}", .retry_after.map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited {
        /// The server-requested wait before retrying, if provided.
        retry_after: Option<Duration>,
    },

    /// Authentication failed (bad or missing credentials).
    #[error("authentication failed: {0}")]
    Auth(String),

    /// The provider returned an error status.
    #[error("API error (HTTP {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Human-readable message.
        message: String,
    },

    /// A transport-level failure (connection, DNS, TLS, timeout).
    #[error("transport error: {0}")]
    Transport(String),

    /// A response body could not be deserialized.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// The caller supplied invalid input or configuration.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Any other provider error.
    #[error("provider error: {0}")]
    Other(String),
}

#[cfg(feature = "tmdb")]
impl From<crate::providers::tmdb::TmdbError> for ProviderError {
    fn from(err: crate::providers::tmdb::TmdbError) -> Self {
        use crate::providers::tmdb::{TmdbError, error::RequestErrorKind};
        match err {
            // The `TransportError` description is already redacted: it never
            // carries the request URL's `api_key`.
            TmdbError::Http(e) => ProviderError::Transport(e.to_string()),
            TmdbError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            e @ TmdbError::PageOutOfRange { .. } => ProviderError::Deserialization(e.to_string()),
            TmdbError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            TmdbError::UnsupportedSource(_) => ProviderError::Unsupported,
            TmdbError::Request { kind, message } => match kind {
                RequestErrorKind::InvalidRequest => ProviderError::InvalidInput(message),
                RequestErrorKind::PaginationCycle => {
                    ProviderError::Other(format!("{kind}: {message}"))
                }
            },
            TmdbError::Api {
                status,
                message,
                retry_after,
                ..
            } => match status {
                401 | 403 => ProviderError::Auth(message),
                404 => ProviderError::NotFound,
                429 => ProviderError::RateLimited { retry_after },
                _ => ProviderError::Api { status, message },
            },
        }
    }
}

#[cfg(feature = "anilist")]
impl From<crate::providers::anilist::AniListError> for ProviderError {
    fn from(err: crate::providers::anilist::AniListError) -> Self {
        use crate::providers::anilist::AniListError;
        match err {
            AniListError::Http(e) => ProviderError::Transport(e.without_url().to_string()),
            AniListError::Deserialization(e) => ProviderError::Deserialization(e.to_string()),
            AniListError::InvalidConfig(s) => ProviderError::InvalidInput(s),
            AniListError::NotFound => ProviderError::NotFound,
            AniListError::NoData => ProviderError::Other("no data returned".to_string()),
            AniListError::Status {
                status,
                retry_after,
                message,
            } => match status {
                401 | 403 => ProviderError::Auth(message),
                404 => ProviderError::NotFound,
                429 => ProviderError::RateLimited { retry_after },
                _ => ProviderError::Api { status, message },
            },
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
                    match status {
                        401 | 403 => ProviderError::Auth(message),
                        _ => ProviderError::Api { status, message },
                    }
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
#[cfg(feature = "anilist")]
fn most_severe_gql_status(
    errors: &[crate::providers::anilist::error::AniListGqlError],
) -> Option<u16> {
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

    /// Every public `ProviderError` variant is constructible.
    #[test]
    fn all_variants_construct() {
        let variants = [
            ProviderError::NotFound,
            ProviderError::Unsupported,
            ProviderError::RateLimited {
                retry_after: Some(Duration::from_secs(1)),
            },
            ProviderError::Auth("bad token".into()),
            ProviderError::Api {
                status: 500,
                message: "boom".into(),
            },
            ProviderError::Transport("connection reset".into()),
            ProviderError::Deserialization("bad json".into()),
            ProviderError::InvalidInput("negative id".into()),
            ProviderError::Other("misc".into()),
        ];
        // Each renders without panicking.
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    #[cfg(feature = "tmdb")]
    #[test]
    fn maps_tmdb_error() {
        use crate::providers::tmdb::TmdbError;
        let api = |status| TmdbError::Api {
            status,
            status_code: None,
            message: "x".into(),
            retry_after: Some(Duration::from_secs(3)),
        };
        assert!(matches!(
            ProviderError::from(api(404)),
            ProviderError::NotFound
        ));
        assert!(matches!(
            ProviderError::from(api(401)),
            ProviderError::Auth(_)
        ));
        assert!(matches!(
            ProviderError::from(api(429)),
            ProviderError::RateLimited {
                retry_after: Some(_)
            }
        ));
        assert!(matches!(
            ProviderError::from(api(500)),
            ProviderError::Api { status: 500, .. }
        ));
        assert!(matches!(
            ProviderError::from(TmdbError::InvalidConfig("x".into())),
            ProviderError::InvalidInput(_)
        ));
        assert!(matches!(
            ProviderError::from(TmdbError::UnsupportedSource(
                crate::unified::external_ids::ExternalSource::Mal
            )),
            ProviderError::Unsupported
        ));
    }

    /// Every `ProviderError` variant renders its own exact message.
    #[test]
    fn every_variant_display_is_pinned() {
        let cases = [
            (ProviderError::NotFound, "resource not found"),
            (
                ProviderError::Unsupported,
                "operation not supported by this provider",
            ),
            (
                ProviderError::RateLimited { retry_after: None },
                "rate limited",
            ),
            (
                ProviderError::RateLimited {
                    retry_after: Some(Duration::from_secs(7)),
                },
                "rate limited (retry after 7s)",
            ),
            (
                ProviderError::Auth("bad token".into()),
                "authentication failed: bad token",
            ),
            (
                ProviderError::Api {
                    status: 500,
                    message: "boom".into(),
                },
                "API error (HTTP 500): boom",
            ),
            (
                ProviderError::Transport("reset".into()),
                "transport error: reset",
            ),
            (
                ProviderError::Deserialization("bad json".into()),
                "deserialization error: bad json",
            ),
            (
                ProviderError::InvalidInput("negative id".into()),
                "invalid input: negative id",
            ),
            (ProviderError::Other("misc".into()), "provider error: misc"),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    /// A `reqwest::Error` built synchronously (an unparseable URL), so the
    /// transport branches can be exercised without a network.
    #[cfg(any(feature = "tmdb", feature = "anilist"))]
    fn reqwest_builder_error() -> reqwest::Error {
        match reqwest::Client::new().get("not a url").build() {
            Ok(request) => panic!("expected a builder error, built {request:?}"),
            Err(e) => e,
        }
    }

    #[cfg(any(feature = "tmdb", feature = "anilist"))]
    fn json_error() -> serde_json::Error {
        match serde_json::from_str::<u32>("not json") {
            Ok(v) => panic!("expected a JSON error, parsed {v}"),
            Err(e) => e,
        }
    }

    /// Table-driven: every `TmdbError` variant and every status branch maps to
    /// the pinned `ProviderError`, carrying its message or `retry_after`.
    #[cfg(feature = "tmdb")]
    #[test]
    fn maps_every_tmdb_error_branch() {
        use crate::providers::tmdb::{TmdbError, error::RequestErrorKind};
        let retry = Some(Duration::from_secs(3));
        let api = |status| TmdbError::Api {
            status,
            status_code: Some(7),
            message: "msg".into(),
            retry_after: retry,
        };
        for status in [401, 403] {
            match ProviderError::from(api(status)) {
                ProviderError::Auth(m) => assert_eq!(m, "msg", "status {status}"),
                other => panic!("status {status}: expected Auth, got {other:?}"),
            }
        }
        assert!(matches!(
            ProviderError::from(api(404)),
            ProviderError::NotFound
        ));
        match ProviderError::from(api(429)) {
            ProviderError::RateLimited { retry_after } => assert_eq!(retry_after, retry),
            other => panic!("expected RateLimited, got {other:?}"),
        }
        for status in [400, 418, 500, 503] {
            match ProviderError::from(api(status)) {
                ProviderError::Api { status: s, message } => {
                    assert_eq!((s, message.as_str()), (status, "msg"));
                }
                other => panic!("status {status}: expected Api, got {other:?}"),
            }
        }

        let http = TmdbError::from(reqwest_builder_error());
        let http_text = http.to_string();
        match ProviderError::from(http) {
            ProviderError::Transport(m) => {
                assert!(!m.is_empty());
                assert!(
                    http_text.ends_with(&m),
                    "Transport carries the transport description: {m:?} vs {http_text:?}"
                );
            }
            other => panic!("expected Transport, got {other:?}"),
        }

        let json = json_error();
        let json_text = json.to_string();
        match ProviderError::from(TmdbError::Deserialization(json)) {
            ProviderError::Deserialization(m) => assert_eq!(m, json_text),
            other => panic!("expected Deserialization, got {other:?}"),
        }

        match ProviderError::from(TmdbError::InvalidConfig("empty token".into())) {
            ProviderError::InvalidInput(m) => assert_eq!(m, "empty token"),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
        assert!(matches!(
            ProviderError::from(TmdbError::UnsupportedSource(
                crate::unified::external_ids::ExternalSource::Mal
            )),
            ProviderError::Unsupported
        ));
        assert!(matches!(
            ProviderError::from(TmdbError::Request {
                kind: RequestErrorKind::InvalidRequest,
                message: "m".into(),
            }),
            ProviderError::InvalidInput(ref m) if m == "m"
        ));
    }

    /// Table-driven: every `AniListError` variant maps to the pinned
    /// `ProviderError`.
    #[cfg(feature = "anilist")]
    #[test]
    fn maps_every_anilist_error_branch() {
        use crate::providers::anilist::AniListError;
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
    }

    /// A GraphQL 429 keeps the response's `retry_after`; an unclassified
    /// GraphQL error keeps the joined messages in `Other`.
    #[cfg(feature = "anilist")]
    #[test]
    fn maps_anilist_graphql_retry_after_and_other_message() {
        use crate::providers::anilist::AniListError;
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

    #[cfg(feature = "tmdb")]
    #[test]
    fn maps_tmdb_request_error_by_kind() {
        use crate::providers::tmdb::{TmdbError, error::RequestErrorKind};
        let request = |kind| TmdbError::Request {
            kind,
            message: "boom".into(),
        };
        assert!(matches!(
            ProviderError::from(request(RequestErrorKind::InvalidRequest)),
            ProviderError::InvalidInput(ref m) if m == "boom"
        ));
        assert!(matches!(
            ProviderError::from(request(RequestErrorKind::PaginationCycle)),
            ProviderError::Other(ref m) if m == "pagination cycle: boom"
        ));
    }

    #[cfg(feature = "anilist")]
    #[test]
    fn maps_anilist_error() {
        use crate::providers::anilist::AniListError;
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

    #[cfg(feature = "anilist")]
    fn gql(errors: serde_json::Value) -> crate::providers::anilist::AniListError {
        crate::providers::anilist::AniListError::GraphQL {
            errors: serde_json::from_value(errors).expect("valid GraphQL error fixture"),
            retry_after: None,
        }
    }

    #[cfg(feature = "anilist")]
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

    #[cfg(feature = "anilist")]
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
}
