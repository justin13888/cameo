use std::time::Duration;

use crate::providers::{rate_limit::RateLimit, retry::RetryPolicy};

const DEFAULT_PER_PAGE: u32 = 20;

/// AniList's documented limit: 90 requests per minute.
const DEFAULT_RATE_LIMIT: RateLimit = RateLimit::Limited {
    max_requests: 90,
    per: Duration::from_secs(60),
};

/// Configuration for the [`AniListClient`](super::client::AniListClient).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct AniListConfig {
    /// Base URL for the AniList GraphQL endpoint.
    ///
    /// Defaults to `https://graphql.anilist.co`.  Override for testing.
    pub base_url: String,
    /// Number of results returned per page (default: 20, max: 50).
    pub per_page: u32,
    /// Client-side request-rate limit.
    ///
    /// Defaults to AniList's documented limit of 90 requests per minute. Set
    /// [`RateLimit::Unlimited`] to disable client-side limiting. Invalid values
    /// are rejected when the client is constructed.
    pub rate_limit: RateLimit,
    /// Retry policy for transient failures (429/5xx/transport). Defaults to
    /// [`RetryPolicy::default`]; use [`RetryPolicy::disabled`] to turn it off.
    pub retry: RetryPolicy,
    /// Timeout for establishing a connection. Defaults to 10s when `None`.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a complete request (connect through full response body).
    /// Defaults to 30s when `None`.
    pub request_timeout: Option<Duration>,
}

impl Default for AniListConfig {
    fn default() -> Self {
        Self {
            base_url: "https://graphql.anilist.co".to_string(),
            per_page: DEFAULT_PER_PAGE,
            rate_limit: DEFAULT_RATE_LIMIT,
            retry: RetryPolicy::default(),
            connect_timeout: None,
            request_timeout: None,
        }
    }
}

impl AniListConfig {
    /// Create a new config with default settings.
    ///
    /// No authentication is required for AniList public data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with a custom base URL (useful for testing with mock servers).
    pub fn new_with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::default()
        }
    }

    /// Set the number of results per page.
    ///
    /// Valid range is 1–50 (AniList API limit); values outside this range are clamped.
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page.clamp(1, 50);
        self
    }

    /// Set the client-side request-rate limit.
    ///
    /// Defaults to AniList's documented 90 requests / minute. Use
    /// [`RateLimit::Unlimited`] to disable client-side limiting.
    pub fn with_rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = limit;
        self
    }

    /// Set the retry policy for transient failures.
    ///
    /// Defaults to [`RetryPolicy::default`]; use [`RetryPolicy::disabled`] to
    /// disable retrying.
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Override the connection-establishment timeout (default: 10s).
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Override the total request timeout (default: 30s).
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::AniListConfig;

    #[test]
    fn with_per_page_normal() {
        let cfg = AniListConfig::new().with_per_page(25);
        assert_eq!(cfg.per_page, 25);
    }

    #[test]
    fn with_per_page_clamp_below_min() {
        let cfg = AniListConfig::new().with_per_page(0);
        assert_eq!(cfg.per_page, 1);
    }

    #[test]
    fn with_per_page_clamp_above_max() {
        let cfg = AniListConfig::new().with_per_page(100);
        assert_eq!(cfg.per_page, 50);
    }

    #[test]
    fn with_per_page_boundary_values() {
        assert_eq!(AniListConfig::new().with_per_page(1).per_page, 1);
        assert_eq!(AniListConfig::new().with_per_page(50).per_page, 50);
    }
}
