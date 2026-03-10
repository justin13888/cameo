use std::time::Duration;

const DEFAULT_RATE_LIMIT: u32 = 40;

/// Configuration for the TMDB client.
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
    /// Maximum number of **concurrent** in-flight requests (defaults to 40).
    ///
    /// This is a concurrency limit, not a throughput limit. It bounds how many
    /// HTTP requests can be simultaneously in-flight — i.e. how many calls can
    /// be awaiting a response at the same time. It does **not** enforce a
    /// requests-per-second rate.
    ///
    /// Note: TMDB's own rate limit (40 requests per 10 seconds per IP) is
    /// separate and is enforced server-side. Setting `rate_limit` to a value
    /// much higher than that threshold may result in HTTP 429 responses from
    /// the TMDB API.
    pub rate_limit: u32,
    /// Maximum time to wait for a concurrency permit before returning
    /// [`TmdbError::RateLimitExceeded`](crate::providers::tmdb::TmdbError::RateLimitExceeded).
    ///
    /// When `None` (the default), callers block indefinitely until a slot is
    /// available. Set this to a finite duration to fail fast when all
    /// concurrent request slots are occupied.
    pub rate_limit_timeout: Option<Duration>,
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
            rate_limit_timeout: None,
        }
    }

    /// Create a new config with a custom base URL (useful for testing).
    pub fn new_with_base_url(api_token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            base_url: Some(base_url.into()),
            language: None,
            region: None,
            include_adult: None,
            rate_limit: DEFAULT_RATE_LIMIT,
            rate_limit_timeout: None,
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

    /// Set the rate limit (max concurrent requests).
    ///
    /// Minimum value is 1; lower values are clamped.
    pub fn with_rate_limit(mut self, limit: u32) -> Self {
        self.rate_limit = limit.max(1);
        self
    }

    /// Set a timeout for acquiring a concurrency permit.
    ///
    /// When all concurrent request slots are occupied, requests will wait at
    /// most this long before returning
    /// [`TmdbError::RateLimitExceeded`](crate::providers::tmdb::TmdbError::RateLimitExceeded).
    pub fn with_rate_limit_timeout(mut self, timeout: Duration) -> Self {
        self.rate_limit_timeout = Some(timeout);
        self
    }
}
