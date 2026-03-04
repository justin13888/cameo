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
    /// Maximum concurrent requests per second (defaults to 40).
    pub rate_limit: u32,
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
    pub fn with_rate_limit(mut self, rate_limit: u32) -> Self {
        self.rate_limit = rate_limit;
        self
    }
}
