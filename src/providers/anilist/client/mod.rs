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
/// # Ids
///
/// Id parameters are AniList's own `i32` media or staff ids. A details
/// method fetches the media with that id whatever its format:
/// [`movie_details`](Self::movie_details) on a TV series' id returns that
/// series as movie details, and [`tv_show_details`](Self::tv_show_details) on
/// a movie's id returns that movie as TV show details.
///
/// # Pages
///
/// A paged method's `page: None` asks for the first page. A page number is
/// sent to AniList unchanged, not clamped (unlike TMDB's, which are clamped
/// to `1..=500`). Each page holds up to [`AniListConfig::per_page`] items.
///
/// # Errors
///
/// Every endpoint method can fail with these errors, called "the usual
/// request errors" below:
///
/// * [`AniListError::Http`] when no response was received (connection, DNS,
///   TLS, or timeout).
/// * [`AniListError::Status`] when AniList answers with an error status and
///   no usable GraphQL body, and [`AniListError::GraphQL`] when it answers
///   with GraphQL errors and no usable data (rate limiting arrives this way
///   as status `429`).
/// * [`AniListError::Deserialization`] when the body is not a GraphQL
///   response, [`AniListError::NoData`] when a successful response has no
///   data, and [`AniListError::PageOutOfRange`] when a paged response reports
///   a negative page number or total.
///
/// A lookup by id whose record does not exist fails with
/// [`AniListError::NotFound`], unlike TMDB, which reports a missing record as
/// an HTTP `404` status. Transient failures (`429`, `5xx`, connect and timeout
/// errors) are retried under [`AniListConfig::retry`] before the last one is
/// returned.
///
/// # Inherent methods and the capability traits
///
/// `AniListClient` also implements the unified capability traits
/// ([`SearchProvider`](crate::unified::SearchProvider),
/// [`DetailProvider`](crate::unified::DetailProvider), and the rest), several
/// of whose methods share a name with an inherent method here but differ in
/// signature:
///
/// | | Inherent method | Trait method |
/// |---|---|---|
/// | Ids | native `i32` (e.g. `movie_details(1)`) | [`&MediaId`](crate::unified::MediaId) (e.g. `movie_details(&MediaId::anilist(1))`) |
/// | Errors | [`AniListError`] | [`ProviderError`](crate::ProviderError) |
///
/// Method-call syntax (`client.search_movies(..)`) always resolves to the
/// inherent method, even with the trait in scope. Call a trait method with
/// the fully qualified form:
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use cameo::{AniListClient, AniListConfig, DetailProvider, MediaId};
///
/// let client = AniListClient::new(AniListConfig::new())?;
/// // Inherent: native id, `AniListError`.
/// let native = client.movie_details(21519).await?;
/// // Trait: `MediaId`, `ProviderError`.
/// let unified = DetailProvider::movie_details(&client, &MediaId::anilist(21519)).await?;
/// # let _ = (native, unified);
/// # Ok(())
/// # }
/// ```
///
/// # Cloning
///
/// `AniListClient` is cheap to clone. Clones share the same HTTP client (and
/// so the same connection pool) and the same rate limiter (and so the same
/// rate-limit budget); each clone holds its own copy of the
/// [`AniListConfig`].
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
