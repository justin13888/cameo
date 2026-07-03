use std::time::Duration;

use crate::providers::{rate_limit::RateLimit, retry::RetryPolicy};

/// TMDB's documented limit: ~40 requests per 10 seconds per IP.
const DEFAULT_RATE_LIMIT: RateLimit = RateLimit::Limited {
    max_requests: 40,
    per: Duration::from_secs(10),
};

/// How the TMDB client authenticates.
///
/// TMDB has two credential systems. The v4 "API Read Access Token" (a bearer
/// JWT) is the modern default; the v3 "API Key" is an older credential many
/// existing accounts still have, sent as an `api_key` query parameter.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum TmdbAuth {
    /// TMDB v4 API Read Access Token, sent as `Authorization: Bearer <token>`.
    V4Bearer(String),
    /// TMDB v3 API Key, sent as an `?api_key=<key>` query parameter.
    V3ApiKey(String),
}

impl TmdbAuth {
    /// The credential string, regardless of scheme.
    pub fn credential(&self) -> &str {
        match self {
            TmdbAuth::V4Bearer(t) | TmdbAuth::V3ApiKey(t) => t,
        }
    }
}

/// Configuration for the TMDB client.
#[non_exhaustive]
#[derive(Debug, Clone)]
#[must_use = "a TmdbConfig does nothing unless passed to TmdbClient::new or CameoClient::builder"]
pub struct TmdbConfig {
    /// Authentication credential (v4 bearer token or v3 API key).
    pub auth: TmdbAuth,
    /// Base URL override (defaults to `https://api.themoviedb.org`).
    pub base_url: Option<String>,
    /// Image CDN base URL override (defaults to `https://image.tmdb.org/t/p/`).
    /// Useful for pointing image URLs at a mock or proxy.
    pub image_base_url: Option<String>,
    /// Default language for requests (e.g. `"en-US"`). Validated as a BCP-47
    /// language tag at client construction.
    pub language: Option<String>,
    /// Default region for requests (e.g. `"US"`). Validated as an ISO 3166-1
    /// alpha-2 code at client construction.
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
    /// Retry policy for transient failures (429/5xx/transport). Defaults to
    /// [`RetryPolicy::default`] (3 retries with exponential backoff); use
    /// [`RetryPolicy::disabled`] to turn retrying off.
    pub retry: RetryPolicy,
    /// Timeout for establishing a connection. Defaults to 10s when `None`.
    pub connect_timeout: Option<Duration>,
    /// Timeout for a complete request (connect through full response body).
    /// Defaults to 30s when `None`.
    pub request_timeout: Option<Duration>,
}

impl TmdbConfig {
    /// Create a new config authenticating with a TMDB v4 bearer token.
    pub fn new(api_token: impl Into<String>) -> Self {
        Self::with_auth(TmdbAuth::V4Bearer(api_token.into()))
    }

    /// Create a new config authenticating with a TMDB v3 API key.
    pub fn new_v3(api_key: impl Into<String>) -> Self {
        Self::with_auth(TmdbAuth::V3ApiKey(api_key.into()))
    }

    /// Create a new config with an explicit [`TmdbAuth`].
    pub fn with_auth(auth: TmdbAuth) -> Self {
        Self {
            auth,
            base_url: None,
            image_base_url: None,
            language: None,
            region: None,
            include_adult: None,
            rate_limit: DEFAULT_RATE_LIMIT,
            retry: RetryPolicy::default(),
            connect_timeout: None,
            request_timeout: None,
        }
    }

    /// Create a new config (v4 bearer) with a custom base URL (useful for testing).
    pub fn new_with_base_url(api_token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            base_url: Some(base_url.into()),
            ..Self::new(api_token)
        }
    }

    /// Override the API base URL (useful for testing against a mock server).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Override the image CDN base URL (e.g. to point at a mock or proxy).
    pub fn with_image_base_url(mut self, url: impl Into<String>) -> Self {
        self.image_base_url = Some(url.into());
        self
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
    /// problem found. Called by [`TmdbClient::new`](super::TmdbClient::new).
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.auth.credential().is_empty() {
            return Err("API credential must not be empty".to_string());
        }
        self.rate_limit.validate()?;
        if let Some(lang) = &self.language {
            validate_language(lang)?;
        }
        if let Some(region) = &self.region {
            validate_region(region)?;
        }
        if let Some(base) = &self.image_base_url
            && !(base.starts_with("http://") || base.starts_with("https://"))
        {
            return Err(format!(
                "image_base_url must be an http(s) URL, got {base:?}"
            ));
        }
        Ok(())
    }
}

/// Validate a BCP-47 language tag such as `"en"` or `"en-US"` (the common
/// `language` or `language-REGION` shapes TMDB accepts).
fn validate_language(lang: &str) -> Result<(), String> {
    let err = || format!("invalid language tag {lang:?} (expected e.g. \"en\" or \"en-US\")");
    let (primary, region) = match lang.split_once('-') {
        Some((p, r)) => (p, Some(r)),
        None => (lang, None),
    };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(err());
    }
    if let Some(region) = region
        && (region.len() != 2 || !region.chars().all(|c| c.is_ascii_uppercase()))
    {
        return Err(err());
    }
    Ok(())
}

/// Validate an ISO 3166-1 alpha-2 region code such as `"US"`.
fn validate_region(region: &str) -> Result<(), String> {
    if region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(format!(
            "invalid region {region:?} (expected an ISO 3166-1 alpha-2 code like \"US\")"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_validation() {
        assert!(validate_language("en").is_ok());
        assert!(validate_language("en-US").is_ok());
        assert!(validate_language("ja").is_ok());
        assert!(validate_language("EN").is_err());
        assert!(validate_language("english").is_err());
        assert!(validate_language("en-us").is_err());
        assert!(validate_language("en-USA").is_err());
    }

    #[test]
    fn region_validation() {
        assert!(validate_region("US").is_ok());
        assert!(validate_region("JP").is_ok());
        assert!(validate_region("us").is_err());
        assert!(validate_region("USA").is_err());
    }

    #[test]
    fn validate_rejects_bad_config() {
        assert!(TmdbConfig::new("").validate().is_err());
        assert!(
            TmdbConfig::new("t")
                .with_language("bogus")
                .validate()
                .is_err()
        );
        assert!(TmdbConfig::new("t").with_region("usa").validate().is_err());
        assert!(
            TmdbConfig::new("t")
                .with_language("en-US")
                .with_region("US")
                .validate()
                .is_ok()
        );
    }
}
