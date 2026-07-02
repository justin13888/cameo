//! AniList GraphQL client implementation.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{
    config::AniListConfig,
    error::AniListError,
    query,
    response::{
        GraphQlResponse, MediaDetailResponse, MediaPageResponse, StaffDetailResponse,
        StaffPageResponse,
    },
};
use crate::{
    core::{config::TimeWindow, pagination::PaginatedResponse},
    unified::{
        conversions::anilist::{anilist_media_to_movie, anilist_media_to_tv},
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
    },
};

/// AniList format strings for anime that map to the "movie" media type.
const MOVIE_FORMATS: &[&str] = &["MOVIE"];

/// AniList format strings for anime that map to the "TV show" media type.
const TV_FORMATS: &[&str] = &["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"];

// ── Client ─────────────────────────────────────────────────────────────────────

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
    rate_limiter: crate::providers::rate_limit::RateLimiter,
}

impl AniListClient {
    /// Create a new AniList client from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::InvalidConfig`] if the rate-limit configuration
    /// is invalid, or [`AniListError::Http`] if the underlying HTTP client
    /// cannot be built (e.g. invalid TLS configuration).
    pub fn new(config: AniListConfig) -> Result<Self, AniListError> {
        config
            .rate_limit
            .validate()
            .map_err(AniListError::InvalidConfig)?;
        let http = crate::providers::http::build_client(
            config.connect_timeout,
            config.request_timeout,
            reqwest::header::HeaderMap::new(),
        )?;
        let rate_limiter = crate::providers::rate_limit::RateLimiter::new(config.rate_limit);
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

    // ── GraphQL execution engine ───────────────────────────────────────────────

    /// Execute a GraphQL query and deserialize the `data` field.
    ///
    /// The response body is read **before** the HTTP status is inspected, so
    /// GraphQL errors and rate-limit headers are surfaced for every status
    /// (AniList returns `404`/`429` as GraphQL errors on an otherwise-2xx
    /// response, and rate-limit info in `Retry-After`).
    ///
    /// # Partial-data policy
    ///
    /// When a response carries **both** `data` and `errors` (a partial
    /// success), the usable `data` is returned and the errors are logged at
    /// `warn` level rather than discarded — failing would throw away good data.
    /// Errors are only surfaced as [`AniListError::GraphQL`] when no `data` is
    /// present. A GraphQL error carrying `status: 404` always maps to
    /// [`AniListError::NotFound`].
    async fn graphql<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T, AniListError> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let policy = self.config.retry;
        let mut retry_index = 0u32;
        loop {
            // Pace requests (including retries) within the rate limit.
            self.rate_limiter.acquire().await;
            match self.graphql_attempt::<T>(&body).await {
                Ok(data) => return Ok(data),
                Err(err) => {
                    if policy.should_retry(retry_index) && err.is_retryable() {
                        let delay = policy.backoff(retry_index, err.retry_after());
                        tracing::debug!(
                            retry = retry_index,
                            ?delay,
                            error = %err,
                            "anilist: retrying transient error"
                        );
                        tokio::time::sleep(delay).await;
                        retry_index += 1;
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }

    /// A single GraphQL request attempt (no rate limiting or retrying).
    async fn graphql_attempt<T: DeserializeOwned>(&self, body: &Value) -> Result<T, AniListError> {
        let resp = self
            .http
            .post(&self.config.base_url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .json(body)
            .send()
            .await?;

        // Capture status and rate-limit headers before consuming the body.
        let status = resp.status();
        let retry_after = super::error::parse_retry_after(resp.headers());
        let text = resp.text().await?;

        // Parse the GraphQL envelope regardless of HTTP status.
        let gql_resp: GraphQlResponse<T> = match serde_json::from_str(&text) {
            Ok(parsed) => parsed,
            Err(parse_err) => {
                // The body was not a GraphQL envelope. An error status is the
                // more useful explanation; otherwise it is a real decode error.
                if !status.is_success() {
                    return Err(AniListError::Status {
                        status: status.as_u16(),
                        retry_after,
                        message: truncate_body(&text),
                    });
                }
                return Err(AniListError::Deserialization(parse_err));
            }
        };

        if let Some(errors) = gql_resp.errors.filter(|e| !e.is_empty()) {
            // Derive not-found from the structured `status`, not the message.
            if errors.iter().any(|e| e.status == Some(404)) {
                return Err(AniListError::NotFound);
            }
            // Partial-data policy: prefer usable data over failing.
            if let Some(data) = gql_resp.data {
                tracing::warn!(?errors, "anilist: returning partial data alongside errors");
                return Ok(data);
            }
            tracing::warn!(?errors, "anilist: graphql errors");
            return Err(AniListError::GraphQL {
                errors,
                retry_after,
            });
        }

        if let Some(data) = gql_resp.data {
            return Ok(data);
        }

        // No data and no errors: distinguish an error status from an empty body.
        if !status.is_success() {
            return Err(AniListError::Status {
                status: status.as_u16(),
                retry_after,
                message: truncate_body(&text),
            });
        }
        Err(AniListError::NoData)
    }

    // ── Helper: build JSON array for format_in variable ───────────────────────

    fn format_in_value(formats: &[&str]) -> Value {
        Value::Array(
            formats
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        )
    }

    fn page_vars(page: Option<u32>, per_page: u32) -> (i64, i64) {
        (page.unwrap_or(1) as i64, per_page as i64)
    }

    // ── Search ─────────────────────────────────────────────────────────────────

    /// Search for anime movies by title.
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(MOVIE_FORMATS),
        });
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_movies)"
        );
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        media_page_to_movies(resp)
    }

    /// Search for anime series (TV, OVA, ONA, Special) by title.
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(TV_FORMATS),
        });
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_tv_shows)"
        );
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        media_page_to_tv(resp)
    }

    /// Search for staff (voice actors, directors, animators, etc.) by name.
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedPerson>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(query, page = page_num, "anilist: graphql SEARCH_STAFF");
        let resp: StaffPageResponse = self.graphql(query::SEARCH_STAFF, vars).await?;
        let page_data = resp.page.ok_or(AniListError::NoData)?;
        let pi = &page_data.page_info;
        Ok(PaginatedResponse {
            page: pi.current_page.unwrap_or(1) as u32,
            total_pages: pi.last_page.unwrap_or(1) as u32,
            total_results: pi.total.unwrap_or(0) as u32,
            results: page_data
                .staff
                .into_iter()
                .map(crate::unified::conversions::anilist::staff_to_person)
                .collect(),
        })
    }

    /// Search across all anime formats. Returns movies and TV shows mixed.
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedSearchResult>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        // Omit formatIn entirely (do not pass null) so AniList searches all formats.
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_multi)"
        );
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        let page_data = resp.page.ok_or(AniListError::NoData)?;
        let pi = &page_data.page_info;
        let results = page_data
            .media
            .into_iter()
            .map(crate::unified::conversions::anilist::anilist_media_to_search_result)
            .collect();
        Ok(PaginatedResponse {
            page: pi.current_page.unwrap_or(1) as u32,
            total_pages: pi.last_page.unwrap_or(1) as u32,
            total_results: pi.total.unwrap_or(0) as u32,
            results,
        })
    }

    // ── Details ────────────────────────────────────────────────────────────────

    /// Get full details for an anime movie by AniList ID.
    pub async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (movie_details)");
        let resp: MediaDetailResponse = self.graphql(query::MEDIA_DETAILS, vars).await?;
        let media = resp.media.ok_or(AniListError::NotFound)?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_movie_details(media))
    }

    /// Get full details for an anime TV series by AniList ID.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (tv_show_details)");
        let resp: MediaDetailResponse = self.graphql(query::MEDIA_DETAILS, vars).await?;
        let media = resp.media.ok_or(AniListError::NotFound)?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_tv_details(media))
    }

    /// Get full details for a staff member by AniList ID.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql STAFF_DETAILS (person_details)");
        let resp: StaffDetailResponse = self.graphql(query::STAFF_DETAILS, vars).await?;
        let staff = resp.staff.ok_or(AniListError::NotFound)?;
        Ok(crate::unified::conversions::anilist::staff_detail_to_person_details(staff))
    }

    // ── Discovery ──────────────────────────────────────────────────────────────

    /// Get trending anime movies.
    ///
    /// # Note
    ///
    /// AniList has no time-window concept. The `time_window` argument is
    /// accepted for trait compatibility but is always ignored — AniList
    /// returns a single global trending list regardless of the requested window.
    pub async fn trending_movies(
        &self,
        _time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(MOVIE_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TRENDING_ANIME (trending_movies)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_TRENDING_ANIME, vars).await?;
        media_page_to_movies(resp)
    }

    /// Get trending anime series.
    ///
    /// # Note
    ///
    /// AniList has no time-window concept. The `time_window` argument is
    /// accepted for trait compatibility but is always ignored — AniList
    /// returns a single global trending list regardless of the requested window.
    pub async fn trending_tv(
        &self,
        _time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(TV_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TRENDING_ANIME (trending_tv)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_TRENDING_ANIME, vars).await?;
        media_page_to_tv(resp)
    }

    /// Get popular anime movies (sorted by popularity).
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(MOVIE_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_POPULAR_ANIME (popular_movies)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_POPULAR_ANIME, vars).await?;
        media_page_to_movies(resp)
    }

    /// Get top-scored anime movies (sorted by average score).
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(MOVIE_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TOP_SCORED_ANIME (top_rated_movies)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_TOP_SCORED_ANIME, vars).await?;
        media_page_to_movies(resp)
    }

    /// Get popular anime TV shows (sorted by popularity).
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(TV_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_POPULAR_ANIME (popular_tv_shows)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_POPULAR_ANIME, vars).await?;
        media_page_to_tv(resp)
    }

    /// Get top-scored anime TV shows (sorted by average score).
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "page": page_num,
            "perPage": per_page,
            "formatIn": Self::format_in_value(TV_FORMATS),
        });
        tracing::debug!(
            page = page_num,
            "anilist: graphql LIST_TOP_SCORED_ANIME (top_rated_tv_shows)"
        );
        let resp: MediaPageResponse = self.graphql(query::LIST_TOP_SCORED_ANIME, vars).await?;
        media_page_to_tv(resp)
    }
}

// ── Page conversion helpers ───────────────────────────────────────────────────

/// Truncate a response body to a bounded length for use in an error message.
fn truncate_body(body: &str) -> String {
    const MAX_CHARS: usize = 200;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "empty response body".to_string();
    }
    let truncated: String = trimmed.chars().take(MAX_CHARS).collect();
    if truncated.len() < trimmed.len() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn media_page_to_movies(
    resp: MediaPageResponse,
) -> Result<PaginatedResponse<UnifiedMovie>, super::error::AniListError> {
    let page = resp.page.ok_or(super::error::AniListError::NoData)?;
    let pi = &page.page_info;
    Ok(PaginatedResponse {
        page: pi.current_page.unwrap_or(1) as u32,
        total_pages: pi.last_page.unwrap_or(1) as u32,
        total_results: pi.total.unwrap_or(0) as u32,
        results: page.media.into_iter().map(anilist_media_to_movie).collect(),
    })
}

fn media_page_to_tv(
    resp: MediaPageResponse,
) -> Result<PaginatedResponse<UnifiedTvShow>, super::error::AniListError> {
    let page = resp.page.ok_or(super::error::AniListError::NoData)?;
    let pi = &page.page_info;
    Ok(PaginatedResponse {
        page: pi.current_page.unwrap_or(1) as u32,
        total_pages: pi.last_page.unwrap_or(1) as u32,
        total_results: pi.total.unwrap_or(0) as u32,
        results: page.media.into_iter().map(anilist_media_to_tv).collect(),
    })
}
