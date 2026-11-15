use std::time::Duration;

use crate::providers::{rate_limit::RateLimit, retry::RetryPolicy};

/// The TheTVDB v4 API root.
const DEFAULT_BASE_URL: &str = "https://api4.thetvdb.com/v4";

/// The default language for translated fields (ISO 639-2, TheTVDB's codes).
const DEFAULT_LANGUAGE: &str = "eng";

/// TheTVDB publishes no request-rate limit, so the client paces itself to a
/// polite 20 requests per second by default.
const DEFAULT_RATE_LIMIT: RateLimit = RateLimit::Limited {
    max_requests: 20,
    per: Duration::from_secs(1),
};

/// Prints a fixed `<redacted>` placeholder in place of a secret.
struct Redacted;

impl std::fmt::Debug for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Configuration for the [`TvdbClient`](super::client::TvdbClient).
///
/// TheTVDB v4 authenticates with a project API key (and, for user-supported
/// keys, a subscriber PIN), exchanged at `POST /login` for a bearer token the
/// client keeps and renews when TheTVDB answers `401`.
///
/// The `Debug` output redacts the API key and PIN; all other fields are shown
/// as-is.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
#[must_use = "a TvdbConfig does nothing unless passed to TvdbClient::new or CameoClient::builder"]
pub struct TvdbConfig {
    /// The project API key sent as `apikey` to `POST /login`.
    pub api_key: String,
    /// The subscriber PIN sent as `pin` to `POST /login`, required only for
    /// user-supported API keys.
    pub pin: Option<String>,
    /// API root, without a trailing slash (defaults to
    /// `https://api4.thetvdb.com/v4`). Override for testing.
    pub base_url: String,
    /// Language for translated names, overviews and episode titles, as a
    /// three-letter ISO 639-2 code (TheTVDB's own codes, e.g. `"eng"`,
    /// `"jpn"`). Defaults to `"eng"`; validated at client construction.
    pub language: String,
    /// Client-side request-rate limit. Defaults to 20 requests per second;
    /// set [`RateLimit::Unlimited`] to disable client-side limiting. Invalid
    /// values are rejected when the client is constructed.
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

impl std::fmt::Debug for TvdbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Destructure so adding a field forces a decision about printing it.
        let Self {
            api_key: _,
            pin,
            base_url,
            language,
            rate_limit,
            retry,
            connect_timeout,
            request_timeout,
        } = self;
        f.debug_struct("TvdbConfig")
            .field("api_key", &Redacted)
            .field("pin", &pin.as_ref().map(|_| Redacted))
            .field("base_url", base_url)
            .field("language", language)
            .field("rate_limit", rate_limit)
            .field("retry", retry)
            .field("connect_timeout", connect_timeout)
            .field("request_timeout", request_timeout)
            .finish()
    }
}

impl TvdbConfig {
    /// Create a config authenticating with a TheTVDB project API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            pin: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            language: DEFAULT_LANGUAGE.to_string(),
            rate_limit: DEFAULT_RATE_LIMIT,
            retry: RetryPolicy::default(),
            connect_timeout: None,
            request_timeout: None,
        }
    }

    /// Set the subscriber PIN for a user-supported API key.
    pub fn with_pin(mut self, pin: impl Into<String>) -> Self {
        self.pin = Some(pin.into());
        self
    }

    /// Override the API root (useful for testing with mock servers).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the language for translated fields (three-letter ISO 639-2 code,
    /// e.g. `"jpn"`).
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set the client-side request-rate limit.
    pub fn with_rate_limit(mut self, limit: RateLimit) -> Self {
        self.rate_limit = limit;
        self
    }

    /// Set the retry policy for transient failures.
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

    /// Check every field the client depends on, returning the first problem.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.api_key.trim().is_empty() {
            return Err("TheTVDB `api_key` must not be empty".to_string());
        }
        if self.pin.as_deref().is_some_and(|p| p.trim().is_empty()) {
            return Err("TheTVDB `pin` must not be empty when set".to_string());
        }
        let is_language =
            self.language.len() == 3 && self.language.bytes().all(|b| b.is_ascii_lowercase());
        if !is_language {
            return Err(format!(
                "TheTVDB `language` must be a lowercase three-letter ISO 639-2 code, got `{}`",
                self.language
            ));
        }
        match reqwest::Url::parse(&self.base_url) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => {}
            _ => {
                return Err(format!(
                    "TheTVDB `base_url` must be an absolute http(s) URL, got `{}`",
                    self.base_url
                ));
            }
        }
        self.rate_limit.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_public_api_in_english() {
        let cfg = TvdbConfig::new("key");
        assert_eq!(cfg.base_url, "https://api4.thetvdb.com/v4");
        assert_eq!(cfg.language, "eng");
        assert_eq!(cfg.pin, None);
        assert_eq!(cfg.validate(), Ok(()));
    }

    #[test]
    fn debug_redacts_the_key_and_pin() {
        let cfg = TvdbConfig::new("SECRETKEY").with_pin("SECRETPIN");
        let rendered = format!("{cfg:?} {cfg:#?}");
        assert!(!rendered.contains("SECRETKEY"), "{rendered}");
        assert!(!rendered.contains("SECRETPIN"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    #[test]
    fn validate_rejects_each_bad_field() {
        let bad = [
            TvdbConfig::new(" "),
            TvdbConfig::new("key").with_pin(""),
            TvdbConfig::new("key").with_language("en"),
            TvdbConfig::new("key").with_language("ENG"),
            TvdbConfig::new("key").with_base_url("not a url"),
            TvdbConfig::new("key").with_base_url("ftp://example.com"),
            TvdbConfig::new("key").with_rate_limit(RateLimit::Limited {
                max_requests: 0,
                per: Duration::from_secs(1),
            }),
        ];
        for cfg in bad {
            assert!(cfg.validate().is_err(), "{cfg:?}");
        }
    }

    #[test]
    fn setters_store_their_values() {
        let cfg = TvdbConfig::new("key")
            .with_pin("1234")
            .with_base_url("http://localhost:1")
            .with_language("jpn")
            .with_rate_limit(RateLimit::Unlimited)
            .with_retry(RetryPolicy::disabled())
            .with_connect_timeout(Duration::from_secs(1))
            .with_request_timeout(Duration::from_secs(2));
        assert_eq!(cfg.pin.as_deref(), Some("1234"));
        assert_eq!(cfg.base_url, "http://localhost:1");
        assert_eq!(cfg.language, "jpn");
        assert_eq!(cfg.rate_limit, RateLimit::Unlimited);
        assert_eq!(cfg.retry, RetryPolicy::disabled());
        assert_eq!(cfg.connect_timeout, Some(Duration::from_secs(1)));
        assert_eq!(cfg.request_timeout, Some(Duration::from_secs(2)));
        assert_eq!(cfg.validate(), Ok(()));
    }
}
