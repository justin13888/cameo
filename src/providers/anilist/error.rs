use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::Deserialize;

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
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

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

    /// One or more GraphQL errors were returned with no usable `data`.
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

    /// Invalid configuration (rejected at construction time).
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl AniListError {
    /// Whether this failure is transient and worth retrying: a 429/5xx status
    /// (surfaced either as `Status` or as a GraphQL error), or a connect/timeout
    /// transport error.
    pub(crate) fn is_retryable(&self) -> bool {
        fn retryable_status(status: i64) -> bool {
            status == 429 || (500..=599).contains(&status)
        }
        match self {
            AniListError::Status { status, .. } => retryable_status(i64::from(*status)),
            AniListError::GraphQL { errors, .. } => {
                errors.iter().filter_map(|e| e.status).any(retryable_status)
            }
            AniListError::Http(e) => e.is_timeout() || e.is_connect(),
            _ => false,
        }
    }

    /// The server-supplied `Retry-After` delay, if any.
    pub(crate) fn retry_after(&self) -> Option<Duration> {
        match self {
            AniListError::Status { retry_after, .. }
            | AniListError::GraphQL { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

/// Parse an AniList `Retry-After` header (integer seconds) into a [`Duration`].
///
/// AniList sends `Retry-After` (seconds) on 429 responses alongside the
/// `X-RateLimit-*` family.
pub(crate) fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}
