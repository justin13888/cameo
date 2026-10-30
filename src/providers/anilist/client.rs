//! AniList GraphQL client implementation.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{
    config::AniListConfig,
    error::AniListError,
    query,
    response::{
        GraphQlResponse, MediaDetailResponse, MediaPageData, MediaPageResponse,
        StaffDetailResponse, StaffPageResponse,
    },
};
use crate::{
    core::{config::TimeWindow, pagination::Page},
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

    /// Execute a GraphQL query and extract the root field the caller needs.
    ///
    /// `root` projects the deserialized `data` object onto the root field the
    /// caller needs (e.g. `Media` or `Page`). Classification happens per
    /// attempt, so a transient upstream failure that nulls the root field is
    /// retried like any other 429/5xx.
    ///
    /// The response body is read **before** the HTTP status is inspected, so
    /// GraphQL errors and rate-limit headers are surfaced for every status
    /// (AniList returns `404`/`429` as GraphQL errors on an otherwise-2xx
    /// response, and rate-limit info in `Retry-After`).
    ///
    /// # Error classification
    ///
    /// - A GraphQL error carrying `status: 404` always maps to
    ///   [`AniListError::NotFound`].
    /// - **Partial data:** when the response carries errors but the root field
    ///   is non-null, the root is returned and the errors are logged at `warn`
    ///   level rather than discarded — failing would throw away good data.
    /// - When the response carries errors and the root field is null or
    ///   missing, the errors are surfaced as [`AniListError::GraphQL`] (never
    ///   as not-found / no-data), retried when any status is 429/5xx.
    ///   This holds even when the errors carry only a non-error status (e.g.
    ///   `200`): the server reported errors, so we do not claim the item is
    ///   missing.
    /// - If the HTTP status is non-2xx, GraphQL errors without a `status`
    ///   inherit the HTTP status (before any other check), so e.g. an HTTP 500
    ///   with status-less errors is treated as a 5xx and an HTTP 404 as
    ///   not-found.
    /// - With no errors (absent or empty) and a null root, a non-2xx HTTP
    ///   status yields [`AniListError::Status`] (retried on 429/5xx).
    /// - With no errors, a null root on a 2xx response yields `Ok(None)`; the
    ///   caller decides whether that means not-found (details) or no-data
    ///   (lists).
    async fn graphql<R, T>(
        &self,
        query: &str,
        variables: Value,
        root: fn(R) -> Option<T>,
    ) -> Result<Option<T>, AniListError>
    where
        R: DeserializeOwned,
    {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let policy = self.config.retry;
        let mut retry_index = 0u32;
        loop {
            // Pace requests (including retries) within the rate limit.
            self.rate_limiter.acquire().await;
            match self.graphql_attempt(&body, root).await {
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
    ///
    /// See [`graphql`](Self::graphql) for the classification rules.
    async fn graphql_attempt<R, T>(
        &self,
        body: &Value,
        root: fn(R) -> Option<T>,
    ) -> Result<Option<T>, AniListError>
    where
        R: DeserializeOwned,
    {
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
        let gql_resp: GraphQlResponse<R> = match serde_json::from_str(&text) {
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

        if let Some(mut errors) = gql_resp.errors.filter(|e| !e.is_empty()) {
            // Status-less errors inherit a non-2xx HTTP status so they
            // classify correctly (e.g. HTTP 404 → not found, HTTP 500 → 5xx).
            if !status.is_success() {
                let http_status = i64::from(status.as_u16());
                for e in errors.iter_mut().filter(|e| e.status.is_none()) {
                    e.status = Some(http_status);
                }
            }
            // Derive not-found from the structured `status`, not the message.
            if errors.iter().any(|e| e.status == Some(404)) {
                return Err(AniListError::NotFound);
            }
            // Partial-data policy: prefer a usable root over failing.
            if let Some(value) = gql_resp.data.and_then(root) {
                tracing::warn!(?errors, "anilist: returning partial data alongside errors");
                return Ok(Some(value));
            }
            // The root is null/missing: the errors explain why.
            tracing::warn!(?errors, "anilist: graphql errors");
            return Err(AniListError::GraphQL {
                errors,
                retry_after,
            });
        }

        // No errors (absent or empty).
        let data_present = gql_resp.data.is_some();
        if let Some(value) = gql_resp.data.and_then(root) {
            return Ok(Some(value));
        }
        // Null root / no data on an error status: report the HTTP status.
        if !status.is_success() {
            return Err(AniListError::Status {
                status: status.as_u16(),
                retry_after,
                message: truncate_body(&text),
            });
        }
        if data_present {
            // A successful response with a null root: the caller decides.
            Ok(None)
        } else {
            Err(AniListError::NoData)
        }
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
    ) -> Result<Page<UnifiedMovie>, AniListError> {
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
        let page = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?;
        media_page_to_movies(page)
    }

    /// Search for anime series (TV, OVA, ONA, Special) by title.
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
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
        let page = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?;
        media_page_to_tv(page)
    }

    /// Search for staff (voice actors, directors, animators, etc.) by name.
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(query, page = page_num, "anilist: graphql SEARCH_STAFF");
        let page_data = self
            .graphql(query::SEARCH_STAFF, vars, |r: StaffPageResponse| r.page)
            .await?
            .ok_or(AniListError::NoData)?;
        let pi = &page_data.page_info;
        Ok(Page::with_has_next(
            pi.current_page.unwrap_or(1) as u32,
            page_data
                .staff
                .into_iter()
                .map(crate::unified::conversions::anilist::staff_to_person)
                .collect(),
            pi.has_next_page,
            pi.last_page.unwrap_or(1) as u32,
            pi.total.unwrap_or(0) as u32,
        ))
    }

    /// Search across all anime formats. Returns movies and TV shows mixed.
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, AniListError> {
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
        let page_data = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?
            .ok_or(AniListError::NoData)?;
        let pi = &page_data.page_info;
        let results = page_data
            .media
            .into_iter()
            .filter_map(crate::unified::conversions::anilist::anilist_media_to_search_result)
            .collect();
        Ok(Page::with_has_next(
            pi.current_page.unwrap_or(1) as u32,
            results,
            pi.has_next_page,
            pi.last_page.unwrap_or(1) as u32,
            pi.total.unwrap_or(0) as u32,
        ))
    }

    // ── Details ────────────────────────────────────────────────────────────────

    /// Get full details for an anime movie by AniList ID.
    pub async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (movie_details)");
        let media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_movie_details(media))
    }

    /// Get full details for an anime TV series by AniList ID.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (tv_show_details)");
        let media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_tv_details(media))
    }

    /// Get full details for a staff member by AniList ID.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql STAFF_DETAILS (person_details)");
        let staff = self
            .graphql(query::STAFF_DETAILS, vars, |r: StaffDetailResponse| r.staff)
            .await?
            .ok_or(AniListError::NotFound)?;
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
    ) -> Result<Page<UnifiedMovie>, AniListError> {
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
        let page = self
            .graphql(query::LIST_TRENDING_ANIME, vars, media_page_root)
            .await?;
        media_page_to_movies(page)
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
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
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
        let page = self
            .graphql(query::LIST_TRENDING_ANIME, vars, media_page_root)
            .await?;
        media_page_to_tv(page)
    }

    /// Get popular anime movies (sorted by popularity).
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
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
        let page = self
            .graphql(query::LIST_POPULAR_ANIME, vars, media_page_root)
            .await?;
        media_page_to_movies(page)
    }

    /// Get top-scored anime movies (sorted by average score).
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
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
        let page = self
            .graphql(query::LIST_TOP_SCORED_ANIME, vars, media_page_root)
            .await?;
        media_page_to_movies(page)
    }

    /// Get popular anime TV shows (sorted by popularity).
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
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
        let page = self
            .graphql(query::LIST_POPULAR_ANIME, vars, media_page_root)
            .await?;
        media_page_to_tv(page)
    }

    /// Get top-scored anime TV shows (sorted by average score).
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
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
        let page = self
            .graphql(query::LIST_TOP_SCORED_ANIME, vars, media_page_root)
            .await?;
        media_page_to_tv(page)
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

/// Root-field extractor for queries returning a media `Page`.
fn media_page_root(resp: MediaPageResponse) -> Option<MediaPageData> {
    resp.page
}

fn media_page_to_movies(page: Option<MediaPageData>) -> Result<Page<UnifiedMovie>, AniListError> {
    let page = page.ok_or(AniListError::NoData)?;
    let pi = &page.page_info;
    Ok(Page::with_has_next(
        pi.current_page.unwrap_or(1) as u32,
        page.media.into_iter().map(anilist_media_to_movie).collect(),
        pi.has_next_page,
        pi.last_page.unwrap_or(1) as u32,
        pi.total.unwrap_or(0) as u32,
    ))
}

fn media_page_to_tv(page: Option<MediaPageData>) -> Result<Page<UnifiedTvShow>, AniListError> {
    let page = page.ok_or(AniListError::NoData)?;
    let pi = &page.page_info;
    Ok(Page::with_has_next(
        pi.current_page.unwrap_or(1) as u32,
        page.media.into_iter().map(anilist_media_to_tv).collect(),
        pi.has_next_page,
        pi.last_page.unwrap_or(1) as u32,
        pi.total.unwrap_or(0) as u32,
    ))
}
