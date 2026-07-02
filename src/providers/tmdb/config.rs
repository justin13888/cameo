use std::time::Duration;

use crate::providers::rate_limit::RateLimit;

/// TMDB's documented limit: ~40 requests per 10 seconds per IP.
const DEFAULT_RATE_LIMIT: RateLimit = RateLimit::Limited {
    max_requests: 40,
    per: Duration::from_secs(10),
};

/// Configuration for the TMDB client.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct TmdbConfig {
    /// TMDB API read access token (v4 auth / bearer token).
    pub api_token: String,
    /// Base URL override (defaults to `https://api.themoviedb.org`).
    pub base_url: Option<String>,
    /// Default language for requests (e.g. `"en-US"`).
    pub language: Option<String>,
    /// Default region for requests (e.g. `"US"`).
    pub region: Option<String>,
    /// Whether to include adult content in results.
    pub include_adult: Option<bool>,
    /// Client-side request-rate limit (a real throughput limiter, not a
    /// concurrency cap).
    ///
    /// Defaults to TMDB's documented limit of 40 requests per 10 seconds. The
    /// limiter paces bursts so the sustained rate stays within this bound; set
    /// [`RateLimit::Unlimited`] to disable it. Invalid values (zero requests or
    /// a zero window) are rejected when the client is constructed.
    pub rate_limit: RateLimit,
    /// Timeout for establishing a connection. Defaults to 10s when `None`.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a complete request (connect through full response body).
    /// Defaults to 30s when `None`.
    pub request_timeout: Option<Duration>,
}

impl TmdbConfig {
    /// Create a new config with the given API token.
    pub fn new(api_token: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            base_url: None,
            language: None,
            region: None,
            include_adult: None,
            rate_limit: DEFAULT_RATE_LIMIT,
            connect_timeout: None,
            request_timeout: None,
        }
    }

    /// Create a new config with a custom base URL (useful for testing).
    pub fn new_with_base_url(api_token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            base_url: Some(base_url.into()),
            ..Self::new(api_token)
        }
    }

    /// Set the default language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set the default region.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set whether to include adult content.
    pub fn with_include_adult(mut self, include_adult: bool) -> Self {
        self.include_adult = Some(include_adult);
        self
    }

    /// Set the client-side request-rate limit.
    ///
    /// Defaults to TMDB's documented 40 requests / 10 seconds. Use
    /// [`RateLimit::Unlimited`] to disable client-side limiting.
    pub fn with_rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = limit;
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
