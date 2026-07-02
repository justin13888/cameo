//! Shared HTTP client construction for provider clients.
//!
//! Centralizes the timeout and `User-Agent` defaults so every provider builds
//! its `reqwest::Client` the same way. Without a total timeout a hung
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
