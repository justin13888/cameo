//! Shared HTTP helpers for provider clients.
//!
//! Centralizes the timeout and `User-Agent` defaults so every provider builds
//! its `reqwest::Client` the same way, plus the `Retry-After` parsing and
//! retryable-status classification both providers apply to responses. Without a total timeout a hung
//! connection blocks the caller indefinitely, and some upstreams throttle
//! clients that send no identifying `User-Agent`.

use std::time::Duration;

/// Default timeout for establishing a TCP + TLS connection.
pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default timeout for a complete request (connect through full response body).
pub(crate) const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The `User-Agent` header sent by all cameo provider clients (`cameo/<version>`).
pub(crate) const USER_AGENT: &str = concat!("cameo/", env!("CARGO_PKG_VERSION"));

/// Build a [`reqwest::Client`] with cameo's timeout and `User-Agent` defaults.
///
/// `connect_timeout` / `request_timeout` override the defaults when `Some`.
/// `default_headers` are attached to every request (used for TMDB bearer auth).
pub(crate) fn build_client(
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
    default_headers: reqwest::header::HeaderMap,
) -> reqwest::Result<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .connect_timeout(connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT))
        .timeout(request_timeout.unwrap_or(DEFAULT_REQUEST_TIMEOUT))
        .default_headers(default_headers)
        .build()
}

/// Parse a `Retry-After` header (integer seconds) into a [`Duration`].
///
/// TMDB and AniList both send an integer number of seconds (AniList alongside
/// its `X-RateLimit-*` family on 429 responses). HTTP-date form is not parsed
/// (returns `None`).
pub(crate) fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Whether an HTTP `status` marks a transient failure worth retrying: `429`
/// or any `5xx`.
pub(crate) fn is_retryable_status(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

/// Fixtures shared by the provider error-mapping tests.
#[cfg(test)]
pub(crate) mod test_support {
    /// A `reqwest::Error` built synchronously (an unparseable URL), so the
    /// transport branches can be exercised without a network.
    pub(crate) fn reqwest_builder_error() -> reqwest::Error {
        match reqwest::Client::new().get("not a url").build() {
            Ok(request) => panic!("expected a builder error, built {request:?}"),
            Err(e) => e,
        }
    }

    /// A `serde_json::Error` from an unparseable document.
    pub(crate) fn json_error() -> serde_json::Error {
        match serde_json::from_str::<u32>("not json") {
            Ok(v) => panic!("expected a JSON error, parsed {v}"),
            Err(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

    use super::*;

    fn headers(value: &'static str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static(value));
        h
    }

    #[test]
    fn parse_retry_after_reads_integer_seconds() {
        assert_eq!(
            parse_retry_after(&headers("7")),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            parse_retry_after(&headers(" 12 ")),
            Some(Duration::from_secs(12))
        );
    }

    #[test]
    fn parse_retry_after_rejects_absent_and_non_integer() {
        assert_eq!(parse_retry_after(&HeaderMap::new()), None);
        assert_eq!(parse_retry_after(&headers("-1")), None);
        assert_eq!(
            parse_retry_after(&headers("Wed, 21 Oct 2015 07:28:00 GMT")),
            None
        );
    }

    #[test]
    fn retryable_status_is_429_and_5xx_only() {
        for status in [429, 500, 503, 599] {
            assert!(is_retryable_status(status), "{status}");
        }
        for status in [200, 400, 401, 404, 428, 430, 499, 600] {
            assert!(!is_retryable_status(status), "{status}");
        }
    }
}
