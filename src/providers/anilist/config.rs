use std::time::Duration;

use crate::providers::{rate_limit::RateLimit, retry::RetryPolicy};

const DEFAULT_PER_PAGE: u32 = 20;

/// AniList's maximum page size.
const MAX_PER_PAGE: u32 = 50;

/// AniList's documented limit: 90 requests per minute.
const DEFAULT_RATE_LIMIT: RateLimit = RateLimit::Limited {
    max_requests: 90,
    per: Duration::from_secs(60),
};

/// Configuration for the [`AniListClient`](super::client::AniListClient).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "an AniListConfig does nothing unless passed to AniListClient::new or CameoClient::builder"]
pub struct AniListConfig {
    /// Base URL for the AniList GraphQL endpoint.
    ///
    /// Defaults to `https://graphql.anilist.co`. Override for testing. Must be
    /// an http(s) URL; anything else is rejected when the client is
    /// constructed.
    pub base_url: String,
    /// Number of results returned per page (default: 20).
    ///
    /// Must be within 1–50 (the AniList API limit).
    /// [`with_per_page`](Self::with_per_page) clamps into that range; a value
    /// set directly on the field outside it is rejected when the client is
    /// constructed.
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

    /// Override the GraphQL endpoint URL (useful for testing against a mock
    /// server).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the number of results per page.
    ///
    /// Valid range is 1–50 (AniList API limit); values outside this range are clamped.
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page.clamp(1, MAX_PER_PAGE);
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

    /// Validate the configuration, returning a clear message on the first
    /// problem found. Called by
    /// [`AniListClient::new`](super::client::AniListClient::new).
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_PER_PAGE).contains(&self.per_page) {
            return Err(format!(
                "per_page must be within 1..={MAX_PER_PAGE}, got {}",
                self.per_page
            ));
        }
        validate_base_url(&self.base_url)?;
        self.rate_limit.validate()
    }
}

/// Check that `base_url` is a non-empty, parseable http(s) URL.
fn validate_base_url(base_url: &str) -> Result<(), String> {
    if base_url.is_empty() {
        return Err("base_url must not be empty".to_string());
    }
    let url =
        reqwest::Url::parse(base_url).map_err(|e| format!("invalid base_url {base_url:?}: {e}"))?;
    match url.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!(
            "base_url must be an http(s) URL, got scheme {scheme:?} in {base_url:?}"
        )),
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

    #[test]
    fn with_base_url_overrides_default() {
        let cfg = AniListConfig::new().with_base_url("http://localhost:1234");
        assert_eq!(cfg.base_url, "http://localhost:1234");
        assert_eq!(cfg.per_page, AniListConfig::default().per_page);
    }

    #[test]
    fn validate_accepts_default_config() {
        assert!(AniListConfig::new().validate().is_ok());
    }

    #[test]
    fn validate_rejects_per_page_out_of_range() {
        for per_page in [0, 51, u32::MAX] {
            let mut cfg = AniListConfig::new();
            cfg.per_page = per_page;
            let err = cfg.validate().expect_err("out-of-range per_page");
            assert!(err.contains("per_page"), "{err}");
        }
        for per_page in [1, 50] {
            let mut cfg = AniListConfig::new();
            cfg.per_page = per_page;
            assert!(cfg.validate().is_ok(), "per_page {per_page}");
        }
    }

    #[test]
    fn validate_rejects_bad_base_url() {
        for base_url in ["", "not a url", "ftp://graphql.anilist.co"] {
            let err = AniListConfig::new()
                .with_base_url(base_url)
                .validate()
                .expect_err("bad base_url");
            assert!(err.contains("base_url"), "{base_url:?}: {err}");
        }
    }
}
