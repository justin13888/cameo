//! AniList GraphQL client implementation.
//!
//! [`AniListClient`] is split by concern: the GraphQL transport (sending,
//! retrying, and classifying responses) lives in `graphql`, and the endpoint
//! methods in `search`, `details` (details, credits, and external ids), and
//! `discovery` (lists, recommendations, and filtered discovery).

use serde_json::Value;

use super::{
    config::AniListConfig,
    convert::{format_in_value, page_vars},
    discover::AnimeFormat,
    error::AniListError,
};
use crate::providers::{http, rate_limit::RateLimiter};

mod details;
mod discovery;
mod graphql;
mod search;
#[cfg(test)]
mod tests;

/// Low-level AniList GraphQL client.
///
/// Sends typed GraphQL queries to the AniList API and returns deserialized
/// results. No authentication is needed for public data.
///
/// **Rate limit:** AniList enforces 90 requests per minute. This client paces
/// requests with a token-bucket limiter matching that limit by default (see
/// [`AniListConfig::with_rate_limit`](super::config::AniListConfig::with_rate_limit)).
///
/// # Cloning
///
/// `AniListClient` is cheaply cloneable — the underlying `reqwest::Client` is
/// reference-counted, so clones share the same connection pool.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use cameo::providers::anilist::{AniListClient, AniListConfig};
/// use cameo::unified::SearchProvider;
///
/// let client = AniListClient::new(AniListConfig::new())?;
/// let results = client.search_movies("Your Name", None).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AniListClient {
    http: reqwest::Client,
    config: AniListConfig,
    rate_limiter: RateLimiter,
}

impl AniListClient {
    /// Create a new AniList client from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::InvalidConfig`] if `per_page` is outside 1..=50,
    /// `base_url` is empty or not a parseable http(s) URL, or the rate-limit
    /// configuration is invalid, or [`AniListError::Http`] if the underlying
    /// HTTP client cannot be built (e.g. invalid TLS configuration).
    pub fn new(config: AniListConfig) -> Result<Self, AniListError> {
        config.validate().map_err(AniListError::InvalidConfig)?;
        let http = http::build_client(
            config.connect_timeout,
            config.request_timeout,
            reqwest::header::HeaderMap::new(),
        )
        .map_err(AniListError::from_reqwest)?;
        let rate_limiter = RateLimiter::new(config.rate_limit);
        Ok(Self {
            http,
            config,
            rate_limiter,
        })
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &AniListConfig {
        &self.config
    }

    /// The `page` and `perPage` variables for `page` at the configured page
    /// size.
    fn page_vars(&self, page: Option<u32>) -> (i64, i64) {
        page_vars(page, self.config.per_page)
    }

    /// The variables of a list request at `page` filtered to `formats`
    /// (`page`, `perPage`, and `formatIn`), with the page number they carry.
    fn list_vars(&self, page: Option<u32>, formats: &[AnimeFormat]) -> (i64, Value) {
        let (page_num, per_page) = self.page_vars(page);
        let vars = serde_json::json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": format_in_value(formats),
        });
        (page_num, vars)
    }
}
