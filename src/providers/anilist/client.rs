//! AniList GraphQL client implementation.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{
    config::AniListConfig,
    discover::{AnimeFormat, DiscoverAnimeBuilder},
    error::AniListError,
    query,
    response::{
        GraphQlResponse, MediaCreditsResponse, MediaDetailResponse, MediaIdsResponse,
        MediaPageData, MediaPageResponse, MediaRecommendationsResponse, PageInfo,
        RecommendationConnection, StaffDetailResponse, StaffPageResponse,
    },
};
use crate::{
    core::{config::TimeWindow, pagination::Page},
    unified::{
        conversions::{
            anilist::{
                anilist_media_to_movie, anilist_media_to_search_result, anilist_media_to_tv,
                credits::anilist_credits_to_unified,
            },
            native_id,
        },
        external_ids::{ExternalIds, ExternalMatch},
        media_id::MediaId,
        models::{
            UnifiedCredits, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
    },
};

/// AniList formats for anime that map to the "movie" media type.
const MOVIE_FORMATS: &[AnimeFormat] = AnimeFormat::MOVIES;

/// AniList formats for anime that map to the "TV show" media type.
const TV_FORMATS: &[AnimeFormat] = AnimeFormat::TV;

/// The unified media type an AniList anime format maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A movie: a format in [`MOVIE_FORMATS`].
    Movie,
    /// A TV show: a format in [`TV_FORMATS`].
    Tv,
}

/// Classify an AniList `format` value (e.g. `"MOVIE"`, `"TV"`).
///
/// The single source of truth shared by the `formatIn` lists the movie and TV
/// queries send ([`MOVIE_FORMATS`], [`TV_FORMATS`]) and the conversions that
/// sort mixed results: a format in neither list (e.g. `MUSIC`) or an unknown
/// value is `None`, so no result set reports it as a movie or a show.
pub(crate) fn classify(format: &str) -> Option<Kind> {
    let in_list = |formats: &[AnimeFormat]| formats.iter().any(|f| f.as_str() == format);
    if in_list(MOVIE_FORMATS) {
        Some(Kind::Movie)
    } else if in_list(TV_FORMATS) {
        Some(Kind::Tv)
    } else {
        None
    }
}

/// Characters and staff fetched per [`AniListClient::media_credits`] request.
///
/// Below AniList's 50-item maximum because each character also nests its
/// voice actors, which counts against AniList's per-query complexity limit.
const CREDITS_PER_PAGE: u32 = 25;

/// The most requests [`AniListClient::media_credits`] makes for one anime
/// (at [`CREDITS_PER_PAGE`], up to 500 characters and 500 staff).
const CREDITS_MAX_PAGES: u32 = 20;

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

    fn format_in_value(formats: &[AnimeFormat]) -> Value {
        Value::Array(
            formats
                .iter()
                .map(|f| Value::String(f.as_str().to_string()))
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
        page_from(
            pi,
            page_data
                .staff
                .into_iter()
                .filter_map(crate::unified::conversions::anilist::staff_to_person)
                .collect(),
        )
    }

    /// Search across all anime formats. Returns movies and TV shows mixed.
    ///
    /// Each result is classified the way the movie and TV list queries filter:
    /// `MOVIE` is a movie, `TV`, `TV_SHORT`, `ONA`, `OVA`, and `SPECIAL` are TV
    /// shows, and anything else (e.g. `MUSIC`, or no format) is omitted, so a
    /// page may hold fewer items than the configured page size while the
    /// pagination metadata keeps describing AniList's unfiltered results.
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
            .filter_map(anilist_media_to_search_result)
            .collect();
        page_from(pi, results)
    }

    // ── Details ────────────────────────────────────────────────────────────────

    /// Get full details for an anime movie by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList id.
    pub async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (movie_details)");
        let mut media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut media.id, id);
        crate::unified::conversions::anilist::anilist_media_detail_to_movie_details(media)
            .ok_or(AniListError::NotFound)
    }

    /// Get full details for an anime TV series by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList id.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (tv_show_details)");
        let mut media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut media.id, id);
        crate::unified::conversions::anilist::anilist_media_detail_to_tv_details(media)
            .ok_or(AniListError::NotFound)
    }

    /// Get full details for a staff member by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList staff id.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql STAFF_DETAILS (person_details)");
        let mut staff = self
            .graphql(query::STAFF_DETAILS, vars, |r: StaffDetailResponse| r.staff)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut staff.id, id);
        crate::unified::conversions::anilist::staff_detail_to_person_details(staff)
            .ok_or(AniListError::NotFound)
    }

    // ── Credits ────────────────────────────────────────────────────────────────

    /// Get the cast and crew of an anime by AniList ID.
    ///
    /// The cast is built from `Media.characters`: one credit per Japanese
    /// voice actor of each character, ordered main characters first, with the
    /// character's position as its billing `order`. The crew is built from
    /// `Media.staff`, with each staff edge's role as the `job`. Person ids are
    /// staff ids (`anilist:staff:<id>`), usable with
    /// [`person_details`](Self::person_details).
    ///
    /// Both connections are paged 25 entries at a time and walked until
    /// AniList reports no next page, up to 20 requests (500 characters and
    /// 500 staff); past that cap the remaining credits are dropped and a
    /// warning is logged. Movies and series share this query.
    ///
    /// # Errors
    ///
    /// [`AniListError::NotFound`] when no anime has this id (a zero or
    /// negative id never names one, so it fails without a request).
    pub async fn media_credits(&self, id: i32) -> Result<UnifiedCredits, AniListError> {
        let title = MediaId::anilist(native_id(i64::from(id)).ok_or(AniListError::NotFound)?);
        let mut characters = Vec::new();
        let mut staff = Vec::new();
        let mut more_characters = true;
        let mut more_staff = true;
        let mut page: u32 = 1;
        while more_characters || more_staff {
            if page > CREDITS_MAX_PAGES {
                tracing::warn!(
                    id,
                    max_pages = CREDITS_MAX_PAGES,
                    "anilist: credits truncated at the page cap"
                );
                break;
            }
            let vars = json!({
                "id": id,
                "characterPage": page,
                "staffPage": page,
                "perPage": CREDITS_PER_PAGE,
            });
            tracing::debug!(id, page, "anilist: graphql MEDIA_CREDITS");
            let media = self
                .graphql(query::MEDIA_CREDITS, vars, |r: MediaCreditsResponse| {
                    r.media
                })
                .await?
                .ok_or(AniListError::NotFound)?;
            if more_characters {
                let conn = media.characters;
                more_characters = conn
                    .as_ref()
                    .and_then(|c| c.page_info.as_ref())
                    .is_some_and(|p| p.has_next_page);
                characters.extend(conn.into_iter().flat_map(|c| c.edges).flatten());
            }
            if more_staff {
                let conn = media.staff;
                more_staff = conn
                    .as_ref()
                    .and_then(|c| c.page_info.as_ref())
                    .is_some_and(|p| p.has_next_page);
                staff.extend(conn.into_iter().flat_map(|c| c.edges).flatten());
            }
            page += 1;
        }
        Ok(anilist_credits_to_unified(title, characters, staff))
    }

    // ── External IDs ───────────────────────────────────────────────────────────

    /// Get the external ids of an anime by AniList ID: its MyAnimeList id
    /// (when AniList knows one) and `anilist: Some(id)`.
    ///
    /// AniList tracks no IMDb, TheTVDB, Wikidata, or TMDB ids, so those stay
    /// `None`. Uses a minimal `{ id idMal }` query rather than the full detail
    /// selection.
    ///
    /// # Errors
    ///
    /// [`AniListError::NotFound`] when no anime has this id.
    pub async fn external_ids(&self, id: i32) -> Result<ExternalIds, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_EXTERNAL_IDS");
        let media = self
            .graphql(query::MEDIA_EXTERNAL_IDS, vars, |r: MediaIdsResponse| {
                r.media
            })
            .await?
            .ok_or(AniListError::NotFound)?;
        let mut ids = ExternalIds::new();
        ids.anilist = native_id(i64::from(media.id));
        ids.mal = media.id_mal.map(i64::from).and_then(native_id);
        Ok(ids)
    }

    /// Find the anime with the given MyAnimeList ID.
    ///
    /// The match uses the same classification as
    /// [`search_multi`](Self::search_multi): a
    /// [`MatchKind::Movie`](crate::unified::MatchKind::Movie) for the movie
    /// formats (`MOVIE`) and a
    /// [`MatchKind::TvShow`](crate::unified::MatchKind::TvShow) for the TV
    /// formats (`TV`, `TV_SHORT`, `ONA`, `OVA`, `SPECIAL`).
    ///
    /// Returns `Ok(None)` when AniList has no anime with that MAL id, and when
    /// the anime's format is absent or in neither list (e.g. `MUSIC`).
    pub async fn find_by_mal_id(&self, mal_id: i32) -> Result<Option<ExternalMatch>, AniListError> {
        let vars = json!({ "idMal": mal_id });
        tracing::debug!(mal_id, "anilist: graphql MEDIA_BY_MAL_ID");
        let media = match self
            .graphql(query::MEDIA_BY_MAL_ID, vars, |r: MediaIdsResponse| r.media)
            .await
        {
            Ok(media) => media,
            Err(AniListError::NotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(media.and_then(|media| {
            let id = MediaId::anilist(native_id(i64::from(media.id))?);
            Some(match media.format.as_deref().and_then(classify)? {
                Kind::Movie => ExternalMatch::movie(id),
                Kind::Tv => ExternalMatch::tv_show(id),
            })
        }))
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

    // ── Recommendations ────────────────────────────────────────────────────────

    /// Get community recommendations for an anime, keeping only recommended
    /// **movies** (format `MOVIE`).
    ///
    /// Recommendations are ordered by community rating (`RATING_DESC`).
    /// AniList cannot filter recommendations by format server-side, so
    /// non-movie entries (and entries with no format, or deleted/hidden
    /// recommended media) are dropped locally: a page may hold **fewer** items
    /// than the configured page size, or none, while `total_results`,
    /// `total_pages`, and `has_next_page` keep describing AniList's unfiltered
    /// recommendation list.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::NotFound`] if no anime has this id.
    pub async fn movie_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let recs = self.recommendations(id, page).await?;
        Ok(recommendations_page(recs)?.filter_map(|item| match item {
            UnifiedSearchResult::Movie(movie) => Some(movie),
            _ => None,
        }))
    }

    /// Get community recommendations for an anime, keeping only recommended
    /// **series** (the TV formats `TV`, `TV_SHORT`, `ONA`, `OVA`, `SPECIAL`).
    ///
    /// Uses the same classification as [`search_multi`](Self::search_multi);
    /// see [`movie_recommendations`](Self::movie_recommendations) for how
    /// local filtering affects page sizes and pagination metadata.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::NotFound`] if no anime has this id.
    pub async fn tv_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let recs = self.recommendations(id, page).await?;
        Ok(recommendations_page(recs)?.filter_map(|item| match item {
            UnifiedSearchResult::TvShow(show) => Some(show),
            _ => None,
        }))
    }

    /// Fetch one page of raw recommendations for an anime.
    async fn recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Option<RecommendationConnection>, AniListError> {
        let (page_num, per_page) = Self::page_vars(page, self.config.per_page);
        let vars = json!({
            "id": id,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(
            id,
            page = page_num,
            "anilist: graphql MEDIA_RECOMMENDATIONS"
        );
        let media = self
            .graphql(
                query::MEDIA_RECOMMENDATIONS,
                vars,
                |r: MediaRecommendationsResponse| r.media,
            )
            .await?
            .ok_or(AniListError::NotFound)?;
        Ok(media.recommendations)
    }

    // ── Filtered discovery ─────────────────────────────────────────────────────

    /// Create a builder for discovering anime with filters (genres, country,
    /// broadcast season, format, status, adult flag, sort order).
    ///
    /// See [`DiscoverAnimeBuilder`] for the available filters and how the
    /// movie/TV terminal methods choose formats.
    pub fn discover_anime(&self) -> DiscoverAnimeBuilder<'_> {
        DiscoverAnimeBuilder::new(self)
    }

    /// Run [`query::DISCOVER_ANIME`] and convert the page to movies.
    pub(crate) async fn discover_movies_page(
        &self,
        vars: Value,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        tracing::debug!(
            page = %vars["page"],
            "anilist: graphql DISCOVER_ANIME (discover_anime movies)"
        );
        let page = self
            .graphql(query::DISCOVER_ANIME, vars, media_page_root)
            .await?;
        media_page_to_movies(page)
    }

    /// Run [`query::DISCOVER_ANIME`] and convert the page to TV shows.
    pub(crate) async fn discover_tv_page(
        &self,
        vars: Value,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        tracing::debug!(
            page = %vars["page"],
            "anilist: graphql DISCOVER_ANIME (discover_anime tv)"
        );
        let page = self
            .graphql(query::DISCOVER_ANIME, vars, media_page_root)
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

/// Convert a recommendation connection to a page of classified media,
/// skipping null nodes, null recommended media, and entries whose format is
/// absent or that [`classify`] places in neither list (e.g. `MUSIC`).
/// A missing connection is an empty first page whose totals are unknown (`0`).
fn recommendations_page(
    conn: Option<RecommendationConnection>,
) -> Result<Page<UnifiedSearchResult>, AniListError> {
    let Some(conn) = conn else {
        return Ok(Page::with_has_next(1, Vec::new(), false, 0, 0));
    };
    let pi = &conn.page_info;
    let results = conn
        .nodes
        .into_iter()
        .flatten()
        .filter_map(|node| node.media_recommendation)
        .filter_map(anilist_media_to_search_result)
        .collect();
    page_from(pi, results)
}

/// Build a [`Page`] from AniList's `pageInfo`.
///
/// A missing `currentPage` reads as `1`, and a missing `lastPage` or `total`
/// as `0` (unknown), as [`Page`] documents.
///
/// # Errors
///
/// [`AniListError::PageOutOfRange`] when AniList reports a negative
/// `currentPage`, `lastPage`, or `total`.
fn page_from<T>(pi: &PageInfo, results: Vec<T>) -> Result<Page<T>, AniListError> {
    Ok(Page::with_has_next(
        page_field("currentPage", pi.current_page)?.unwrap_or(1),
        results,
        pi.has_next_page,
        page_field("lastPage", pi.last_page)?.unwrap_or(0),
        page_field("total", pi.total)?.unwrap_or(0),
    ))
}

/// A reported `pageInfo` field as a `u32`: `None` when AniList omitted it,
/// and [`AniListError::PageOutOfRange`] when it is negative.
fn page_field(field: &'static str, value: Option<i32>) -> Result<Option<u32>, AniListError> {
    value
        .map(|v| {
            u32::try_from(v).map_err(|_| AniListError::PageOutOfRange {
                field,
                value: i64::from(v),
            })
        })
        .transpose()
}

/// The id a details response is converted under: the id AniList reported
/// when it is valid, otherwise (zero or negative) the requested id.
fn details_id(reported: &mut i32, requested: i32) {
    if native_id(i64::from(*reported)).is_none() {
        *reported = requested;
    }
}

fn media_page_to_movies(page: Option<MediaPageData>) -> Result<Page<UnifiedMovie>, AniListError> {
    let page = page.ok_or(AniListError::NoData)?;
    let pi = &page.page_info;
    page_from(
        pi,
        page.media
            .into_iter()
            .filter_map(anilist_media_to_movie)
            .collect(),
    )
}

fn media_page_to_tv(page: Option<MediaPageData>) -> Result<Page<UnifiedTvShow>, AniListError> {
    let page = page.ok_or(AniListError::NoData)?;
    let pi = &page.page_info;
    page_from(
        pi,
        page.media
            .into_iter()
            .filter_map(anilist_media_to_tv)
            .collect(),
    )
}

// ── Characterization tests ────────────────────────────────────────────────────

/// Pins every observable behaviour of [`AniListClient`] against a mock
/// GraphQL endpoint: the response decision table of
/// [`graphql_attempt`](AniListClient::graphql_attempt), the request each
/// method sends, the page-metadata defaults, and the retry loop.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate, matchers::method};

    use super::*;
    use crate::{
        core::pagination::PageToken,
        providers::{rate_limit::RateLimit, retry::RetryPolicy},
    };

    #[test]
    fn classify_matches_the_format_in_lists() {
        for f in MOVIE_FORMATS {
            assert_eq!(classify(f.as_str()), Some(Kind::Movie), "{f:?}");
        }
        for f in TV_FORMATS {
            assert_eq!(classify(f.as_str()), Some(Kind::Tv), "{f:?}");
        }
    }

    #[test]
    fn classify_formats_outside_both_lists_are_none() {
        assert_eq!(classify(AnimeFormat::Music.as_str()), None);
        assert_eq!(classify("UNKNOWN"), None);
        assert_eq!(classify(""), None);
    }

    // ── Harness ────────────────────────────────────────────────────────────────

    /// A retry policy with two retries and near-zero backoff.
    const TWO_FAST_RETRIES: RetryPolicy =
        RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(1));

    /// A client for `server` with retrying and rate limiting disabled, after
    /// applying `configure`.
    fn client_with(
        server: &MockServer,
        configure: impl FnOnce(AniListConfig) -> AniListConfig,
    ) -> AniListClient {
        let config = AniListConfig::new_with_base_url(server.uri())
            .with_retry(RetryPolicy::disabled())
            .with_rate_limit(RateLimit::Unlimited);
        AniListClient::new(configure(config)).expect("valid config")
    }

    fn client_for(server: &MockServer) -> AniListClient {
        client_with(server, |c| c)
    }

    /// Start a server answering every POST with `template`, verifying on drop
    /// that it was hit exactly `hits` times.
    async fn serve(template: ResponseTemplate, hits: u64) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(template)
            .expect(hits)
            .mount(&server)
            .await;
        server
    }

    fn json_response(status: u16, body: Value) -> ResponseTemplate {
        ResponseTemplate::new(status).set_body_json(body)
    }

    fn raw_response(status: u16, body: &str) -> ResponseTemplate {
        ResponseTemplate::new(status).set_body_raw(body.to_owned(), "text/plain")
    }

    async fn requests(server: &MockServer) -> Vec<Request> {
        server
            .received_requests()
            .await
            .expect("request recording is enabled")
    }

    fn body_of(request: &Request) -> Value {
        serde_json::from_slice(&request.body).expect("JSON request body")
    }

    /// The JSON body of the single request `server` received, after checking
    /// the request's method and JSON headers.
    async fn only_request(server: &MockServer) -> Value {
        let requests = requests(server).await;
        assert_eq!(requests.len(), 1, "expected exactly one request");
        let request = &requests[0];
        assert_eq!(request.method.as_str(), "POST");
        for header in ["content-type", "accept"] {
            assert_eq!(
                request.headers.get(header).and_then(|v| v.to_str().ok()),
                Some("application/json"),
                "{header} header"
            );
        }
        body_of(request)
    }

    // ── Fixtures ───────────────────────────────────────────────────────────────

    fn media(id: i32, format: &str) -> Value {
        json!({
            "id": id,
            "title": { "romaji": "Romaji", "english": "English", "native": "Native" },
            "format": format
        })
    }

    fn page_info() -> Value {
        json!({ "total": 41, "currentPage": 2, "lastPage": 3, "hasNextPage": true })
    }

    fn media_page(items: Vec<Value>) -> Value {
        json!({ "data": { "Page": { "pageInfo": page_info(), "media": items } } })
    }

    fn staff_page() -> Value {
        json!({ "data": { "Page": {
            "pageInfo": page_info(),
            "staff": [{ "id": 3, "name": { "full": "Person" } }]
        } } })
    }

    fn details() -> Value {
        json!({ "data": { "Media": media(5, "MOVIE") } })
    }

    fn staff_details() -> Value {
        json!({ "data": { "Staff": { "id": 3, "name": { "full": "Person" } } } })
    }

    fn ids(format: &str) -> Value {
        json!({ "data": { "Media": { "id": 5, "idMal": 9, "format": format } } })
    }

    fn recommendations(nodes: Vec<Value>) -> Value {
        json!({ "data": { "Media": { "recommendations": {
            "pageInfo": page_info(),
            "nodes": nodes
        } } } })
    }

    fn credits() -> Value {
        json!({ "data": { "Media": {
            "id": 5,
            "characters": { "pageInfo": { "hasNextPage": false }, "edges": [] },
            "staff": { "pageInfo": { "hasNextPage": false }, "edges": [] }
        } } })
    }

    fn with_errors(mut body: Value, errors: Value) -> Value {
        body["errors"] = errors;
        body
    }

    fn graphql_errors(err: &AniListError) -> &[crate::providers::anilist::error::AniListGqlError] {
        match err {
            AniListError::GraphQL { errors, .. } => errors,
            other => panic!("expected AniListError::GraphQL, got {other:?}"),
        }
    }

    // ── graphql_attempt decision table ─────────────────────────────────────────

    #[tokio::test]
    async fn success_with_data_returns_the_root() {
        let server = serve(json_response(200, details()), 1).await;
        let movie = client_for(&server).movie_details(5).await.expect("ok");
        assert_eq!(movie.movie.provider_id, "anilist:5");
    }

    #[tokio::test]
    async fn error_status_404_is_not_found_even_with_data() {
        let body = with_errors(
            details(),
            json!([{ "message": "Not Found.", "status": 404 }]),
        );
        let server = serve(json_response(200, body), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn errors_with_data_return_the_partial_data() {
        for status in [400, 500] {
            let body = with_errors(
                details(),
                json!([{ "message": "partial", "status": status }]),
            );
            let server = serve(json_response(200, body), 1).await;
            // Retries are enabled: a usable root is returned on the first attempt.
            let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
            let movie = client.movie_details(5).await.expect("partial data");
            assert_eq!(movie.movie.provider_id, "anilist:5");
        }
    }

    #[tokio::test]
    async fn errors_without_data_are_graphql_errors_with_retry_after() {
        let template = json_response(
            200,
            json!({ "errors": [{ "message": "bad query", "status": 400 }] }),
        )
        .insert_header("Retry-After", "7");
        let server = serve(template, 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        let errors = graphql_errors(&err);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "bad query");
        assert_eq!(errors[0].status, Some(400));
        assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
    }

    #[tokio::test]
    async fn errors_with_a_null_root_are_graphql_errors() {
        let body = json!({
            "data": { "Media": null },
            "errors": [{ "message": "failed", "status": 200 }]
        });
        let server = serve(json_response(200, body), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert_eq!(graphql_errors(&err)[0].status, Some(200));
    }

    #[tokio::test]
    async fn statusless_errors_inherit_only_a_non_2xx_status() {
        let body = json!({ "errors": [{ "message": "no status" }] });

        let server = serve(json_response(400, body.clone()), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert_eq!(graphql_errors(&err)[0].status, Some(400));

        let server = serve(json_response(200, body.clone()), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert_eq!(graphql_errors(&err)[0].status, None);

        // An inherited 404 is not-found.
        let server = serve(json_response(404, body), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn an_empty_errors_array_is_treated_as_no_errors() {
        let server = serve(json_response(200, with_errors(details(), json!([]))), 1).await;
        let movie = client_for(&server).movie_details(5).await.expect("ok");
        assert_eq!(movie.movie.provider_id, "anilist:5");

        let body = json!({ "data": { "Media": null }, "errors": [] });
        let server = serve(json_response(200, body), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");

        let server = serve(json_response(200, json!({ "errors": [] })), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");
    }

    #[tokio::test]
    async fn a_non_json_error_response_is_a_status_error() {
        let template =
            raw_response(502, "  <html>Bad Gateway</html>\n").insert_header("Retry-After", "3");
        let server = serve(template, 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        match err {
            AniListError::Status {
                status,
                retry_after,
                message,
            } => {
                assert_eq!(status, 502);
                assert_eq!(retry_after, Some(Duration::from_secs(3)));
                assert_eq!(message, "<html>Bad Gateway</html>");
            }
            other => panic!("expected Status, got {other:?}"),
        }

        let server = serve(raw_response(503, ""), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(&err, AniListError::Status { status: 503, message, .. } if message == "empty response body"),
            "got {err:?}"
        );

        let long = "x".repeat(300);
        let server = serve(raw_response(500, &long), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        let expected = format!("{}…", "x".repeat(200));
        assert!(
            matches!(&err, AniListError::Status { status: 500, message, .. } if *message == expected),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn an_undecodable_success_response_is_a_deserialization_error() {
        let server = serve(raw_response(200, "not json"), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Deserialization(_)),
            "got {err:?}"
        );

        // Valid JSON of the wrong shape is a decode error too...
        let body = json!({ "data": { "Media": { "id": "five" } } });
        let server = serve(json_response(200, body.clone()), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Deserialization(_)),
            "got {err:?}"
        );

        // ...unless the status is an error, which is the better explanation.
        let server = serve(json_response(500, body), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Status { status: 500, .. }),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn no_data_and_no_errors_on_an_error_status_is_a_status_error() {
        let server = serve(json_response(503, json!({})), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(&err, AniListError::Status { status: 503, message, .. } if message == "{}"),
            "got {err:?}"
        );

        let server = serve(json_response(500, json!({ "data": { "Media": null } })), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Status { status: 500, .. }),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn no_data_and_no_errors_on_success_is_no_data() {
        for body in [json!({}), json!({ "data": null })] {
            // Even a details call reports NoData, not NotFound.
            let server = serve(json_response(200, body.clone()), 1).await;
            let err = client_for(&server).movie_details(5).await.unwrap_err();
            assert!(matches!(err, AniListError::NoData), "got {err:?}");

            let server = serve(json_response(200, body), 1).await;
            let err = client_for(&server).popular_movies(None).await.unwrap_err();
            assert!(matches!(err, AniListError::NoData), "got {err:?}");
        }
    }

    // ── truncate_body ──────────────────────────────────────────────────────────

    #[test]
    fn truncate_body_trims_surrounding_whitespace() {
        assert_eq!(truncate_body("  \n body text \t\n"), "body text");
    }

    #[test]
    fn truncate_body_names_an_empty_body() {
        assert_eq!(truncate_body(""), "empty response body");
        assert_eq!(truncate_body(" \n\t "), "empty response body");
    }

    #[test]
    fn truncate_body_keeps_200_chars_and_marks_the_cut() {
        let exact = "a".repeat(200);
        assert_eq!(truncate_body(&exact), exact);
        assert_eq!(
            truncate_body(&"a".repeat(201)),
            format!("{}…", "a".repeat(200))
        );
        // Trimming happens before counting.
        assert_eq!(truncate_body(&format!("   {exact}   ")), exact);
    }

    #[test]
    fn truncate_body_counts_chars_not_bytes() {
        let exact = "é".repeat(200);
        assert_eq!(truncate_body(&exact), exact);
        assert_eq!(
            truncate_body(&"é".repeat(201)),
            format!("{}…", "é".repeat(200))
        );
    }

    // ── Request bodies ─────────────────────────────────────────────────────────

    /// The formats a TV-typed list request filters on.
    fn tv_formats() -> Value {
        json!(["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"])
    }

    /// Run `$call` against a server answering `$response`, with `perPage` 7,
    /// and assert the one request sent carried `$query` and exactly `$vars`.
    macro_rules! assert_sends {
        ($response:expr, |$client:ident| $call:expr, $query:expr, $vars:expr $(,)?) => {{
            let server = serve(json_response(200, $response), 1).await;
            let $client = client_with(&server, |c| c.with_per_page(7));
            $call.await.expect("call succeeds");
            let body = only_request(&server).await;
            assert_eq!(body["query"], $query, "query text");
            assert_eq!(body["variables"], $vars, "variables");
            assert_eq!(
                body.as_object().map(|o| o.len()),
                Some(2),
                "body holds only query and variables"
            );
        }};
    }

    #[tokio::test]
    async fn search_requests_filter_formats_by_media_type() {
        assert_sends!(
            media_page(vec![]),
            |c| c.search_movies("naruto", None),
            query::SEARCH_ANIME,
            json!({ "query": "naruto", "page": 1, "perPage": 7, "formatIn": ["MOVIE"] }),
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.search_tv_shows("naruto", Some(3)),
            query::SEARCH_ANIME,
            json!({ "query": "naruto", "page": 3, "perPage": 7, "formatIn": tv_formats() }),
        );
    }

    #[tokio::test]
    async fn search_multi_and_people_omit_the_format_filter() {
        assert_sends!(
            media_page(vec![]),
            |c| c.search_multi("naruto", Some(2)),
            query::SEARCH_ANIME,
            json!({ "query": "naruto", "page": 2, "perPage": 7 }),
        );
        assert_sends!(
            staff_page(),
            |c| c.search_people("kaji", None),
            query::SEARCH_STAFF,
            json!({ "query": "kaji", "page": 1, "perPage": 7 }),
        );
    }

    #[tokio::test]
    async fn list_requests_send_page_per_page_and_formats() {
        let movies = json!({ "page": 1, "perPage": 7, "formatIn": ["MOVIE"] });
        let tv = json!({ "page": 4, "perPage": 7, "formatIn": tv_formats() });
        assert_sends!(
            media_page(vec![]),
            |c| c.trending_movies(TimeWindow::Week, None),
            query::LIST_TRENDING_ANIME,
            movies.clone(),
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.trending_tv(TimeWindow::Day, Some(4)),
            query::LIST_TRENDING_ANIME,
            tv.clone(),
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.popular_movies(None),
            query::LIST_POPULAR_ANIME,
            movies.clone(),
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.popular_tv_shows(Some(4)),
            query::LIST_POPULAR_ANIME,
            tv.clone(),
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.top_rated_movies(None),
            query::LIST_TOP_SCORED_ANIME,
            movies,
        );
        assert_sends!(
            media_page(vec![]),
            |c| c.top_rated_tv_shows(Some(4)),
            query::LIST_TOP_SCORED_ANIME,
            tv,
        );
    }

    #[tokio::test]
    async fn per_page_defaults_to_20() {
        let server = serve(json_response(200, media_page(vec![])), 1).await;
        client_for(&server)
            .popular_movies(None)
            .await
            .expect("call succeeds");
        let body = only_request(&server).await;
        assert_eq!(body["variables"]["perPage"], 20);
        assert_eq!(body["variables"]["page"], 1);
    }

    #[tokio::test]
    async fn id_requests_send_only_the_id() {
        assert_sends!(
            details(),
            |c| c.movie_details(5),
            query::MEDIA_DETAILS,
            json!({ "id": 5 }),
        );
        assert_sends!(
            details(),
            |c| c.tv_show_details(5),
            query::MEDIA_DETAILS,
            json!({ "id": 5 }),
        );
        assert_sends!(
            staff_details(),
            |c| c.person_details(3),
            query::STAFF_DETAILS,
            json!({ "id": 3 }),
        );
        assert_sends!(
            ids("TV"),
            |c| c.external_ids(5),
            query::MEDIA_EXTERNAL_IDS,
            json!({ "id": 5 }),
        );
        assert_sends!(
            ids("TV"),
            |c| c.find_by_mal_id(9),
            query::MEDIA_BY_MAL_ID,
            json!({ "idMal": 9 }),
        );
    }

    #[tokio::test]
    async fn recommendation_and_credit_requests_send_their_paging() {
        let vars = json!({ "id": 5, "page": 1, "perPage": 7 });
        assert_sends!(
            recommendations(vec![]),
            |c| c.movie_recommendations(5, None),
            query::MEDIA_RECOMMENDATIONS,
            vars.clone(),
        );
        assert_sends!(
            recommendations(vec![]),
            |c| c.tv_recommendations(5, Some(1)),
            query::MEDIA_RECOMMENDATIONS,
            vars,
        );
        // Credits page with their own fixed size, not the configured one.
        assert_sends!(
            credits(),
            |c| c.media_credits(5),
            query::MEDIA_CREDITS,
            json!({ "id": 5, "characterPage": 1, "staffPage": 1, "perPage": 25 }),
        );
    }

    #[tokio::test]
    async fn a_negative_credits_id_is_not_found_without_a_request() {
        let server = serve(json_response(200, credits()), 0).await;
        let err = client_for(&server).media_credits(-1).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    // ── Page mapping ───────────────────────────────────────────────────────────

    fn assert_passed_through<T>(page: &Page<T>) {
        assert_eq!(page.page, 2);
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.total_results, 41);
        assert!(page.has_next);
        assert_eq!(page.next, Some(PageToken::Page(3)));
    }

    fn assert_defaults<T>(page: &Page<T>) {
        assert_eq!(page.page, 1);
        // Omitted totals are unknown, which `Page` documents as `0`.
        assert_eq!(page.total_pages, 0);
        assert_eq!(page.total_results, 0);
        assert!(!page.has_next);
        assert_eq!(page.next, None);
    }

    #[tokio::test]
    async fn page_metadata_is_passed_through() {
        let server = serve(json_response(200, media_page(vec![media(1, "MOVIE")])), 3).await;
        let client = client_for(&server);
        let movies = client.popular_movies(None).await.expect("ok");
        assert_passed_through(&movies);
        assert_eq!(movies.results.len(), 1);
        assert_passed_through(&client.popular_tv_shows(None).await.expect("ok"));
        assert_passed_through(&client.search_multi("x", None).await.expect("ok"));

        let node = json!({ "mediaRecommendation": media(7, "MOVIE") });
        let server = serve(json_response(200, recommendations(vec![node])), 1).await;
        let recs = client_for(&server)
            .movie_recommendations(5, None)
            .await
            .expect("ok");
        assert_passed_through(&recs);
        assert_eq!(recs.results[0].provider_id, "anilist:7");

        let server = serve(json_response(200, staff_page()), 1).await;
        let people = client_for(&server)
            .search_people("x", None)
            .await
            .expect("ok");
        assert_passed_through(&people);
        assert_eq!(people.results[0].provider_id, "anilist:staff:3");
    }

    #[tokio::test]
    async fn missing_page_metadata_takes_defaults() {
        let media = json!({ "data": { "Page": { "pageInfo": {} } } });
        let server = serve(json_response(200, media), 3).await;
        let client = client_for(&server);
        let movies = client.top_rated_movies(None).await.expect("ok");
        assert_defaults(&movies);
        assert!(movies.results.is_empty());
        assert_defaults(&client.search_tv_shows("x", None).await.expect("ok"));
        assert_defaults(&client.search_multi("x", None).await.expect("ok"));

        let staff = json!({ "data": { "Page": { "pageInfo": {} } } });
        let server = serve(json_response(200, staff), 1).await;
        assert_defaults(
            &client_for(&server)
                .search_people("x", None)
                .await
                .expect("ok"),
        );

        let recs = json!({ "data": { "Media": { "recommendations": { "pageInfo": {} } } } });
        let server = serve(json_response(200, recs), 1).await;
        assert_defaults(
            &client_for(&server)
                .tv_recommendations(5, None)
                .await
                .expect("ok"),
        );

        // A missing recommendation connection is an empty first page.
        let none = json!({ "data": { "Media": { "recommendations": null } } });
        let server = serve(json_response(200, none), 1).await;
        let page = client_for(&server)
            .movie_recommendations(5, None)
            .await
            .expect("ok");
        assert_defaults(&page);
        assert!(page.results.is_empty());
    }

    #[tokio::test]
    async fn a_missing_page_is_no_data() {
        for body in [json!({ "data": {} }), json!({ "data": { "Page": null } })] {
            let server = serve(json_response(200, body), 4).await;
            let client = client_for(&server);
            let errors = [
                client.popular_movies(None).await.map(drop).unwrap_err(),
                client
                    .search_tv_shows("x", None)
                    .await
                    .map(drop)
                    .unwrap_err(),
                client.search_multi("x", None).await.map(drop).unwrap_err(),
                client.search_people("x", None).await.map(drop).unwrap_err(),
            ];
            for err in errors {
                assert!(matches!(err, AniListError::NoData), "got {err:?}");
            }
        }
    }

    #[tokio::test]
    async fn a_null_root_on_single_item_lookups_is_not_found() {
        let media = json!({ "data": { "Media": null } });
        let server = serve(json_response(200, media.clone()), 5).await;
        let client = client_for(&server);
        let errors = [
            client.movie_details(5).await.map(drop).unwrap_err(),
            client.tv_show_details(5).await.map(drop).unwrap_err(),
            client.external_ids(5).await.map(drop).unwrap_err(),
            client
                .movie_recommendations(5, None)
                .await
                .map(drop)
                .unwrap_err(),
            client.media_credits(5).await.map(drop).unwrap_err(),
        ];
        for err in errors {
            assert!(matches!(err, AniListError::NotFound), "got {err:?}");
        }

        let server = serve(json_response(200, json!({ "data": { "Staff": null } })), 1).await;
        let err = client_for(&server).person_details(3).await.unwrap_err();
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");

        // A MAL lookup reports absence as an empty match instead.
        let server = serve(json_response(200, media), 1).await;
        assert_eq!(
            client_for(&server).find_by_mal_id(9).await.expect("ok"),
            None
        );
    }

    #[tokio::test]
    async fn search_multi_classifies_by_format_and_drops_formatless_media() {
        let mut formatless = media(3, "TV");
        formatless["format"] = Value::Null;
        let body = media_page(vec![media(1, "MOVIE"), media(2, "OVA"), formatless]);
        let server = serve(json_response(200, body), 1).await;
        let page = client_for(&server)
            .search_multi("x", None)
            .await
            .expect("ok");
        assert_eq!(page.results.len(), 2);
        assert!(matches!(page.results[0], UnifiedSearchResult::Movie(_)));
        assert!(matches!(page.results[1], UnifiedSearchResult::TvShow(_)));
    }

    #[tokio::test]
    async fn external_ids_map_mal_and_anilist_ids() {
        let server = serve(json_response(200, ids("TV")), 1).await;
        let ids = client_for(&server).external_ids(5).await.expect("ok");
        assert_eq!(ids.anilist, Some(5));
        assert_eq!(ids.mal, Some(9));
        assert_eq!(ids.imdb, None);

        let body = json!({ "data": { "Media": { "id": 5, "idMal": -1 } } });
        let server = serve(json_response(200, body), 1).await;
        let ids = client_for(&server).external_ids(5).await.expect("ok");
        assert_eq!(ids.mal, None, "a negative MAL id is dropped");
    }

    #[tokio::test]
    async fn find_by_mal_id_classifies_movie_and_series() {
        let movie = MediaId::anilist(5);
        let server = serve(json_response(200, ids("MOVIE")), 1).await;
        let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
        assert_eq!(found, Some(ExternalMatch::movie(movie.clone())));

        for format in TV_FORMATS {
            let body = json!({ "data": { "Media": { "id": 5, "format": format.as_str() } } });
            let server = serve(json_response(200, body), 1).await;
            let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
            assert_eq!(found, Some(ExternalMatch::tv_show(movie.clone())));
        }

        // A format `classify` places in neither list is not a match.
        for format in [json!("MUSIC"), json!("NOT_A_FORMAT"), Value::Null] {
            let body = json!({ "data": { "Media": { "id": 5, "format": format } } });
            let server = serve(json_response(200, body), 1).await;
            let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
            assert_eq!(found, None, "format {format}");
        }

        let not_found = json!({ "errors": [{ "message": "Not Found.", "status": 404 }] });
        let server = serve(json_response(404, not_found), 1).await;
        assert_eq!(
            client_for(&server).find_by_mal_id(9).await.expect("ok"),
            None
        );
    }

    // ── Retry loop ─────────────────────────────────────────────────────────────

    /// Call `movie_details` against a server answering `template`, with two
    /// fast retries, asserting the server is hit exactly `attempts` times.
    async fn attempts_for(template: ResponseTemplate, attempts: u64) -> AniListError {
        let server = serve(template, attempts).await;
        let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
        let err = client.movie_details(5).await.unwrap_err();
        drop(server); // verifies the attempt count
        err
    }

    #[tokio::test]
    async fn rate_limit_and_server_errors_are_retried_until_the_policy_gives_up() {
        let rate_limited = json_response(
            429,
            json!({ "errors": [{ "message": "Too Many Requests.", "status": 429 }] }),
        )
        .insert_header("Retry-After", "0");
        let err = attempts_for(rate_limited, 3).await;
        assert_eq!(graphql_errors(&err)[0].status, Some(429));

        let err = attempts_for(raw_response(503, "unavailable"), 3).await;
        assert!(
            matches!(err, AniListError::Status { status: 503, .. }),
            "got {err:?}"
        );

        let gql_5xx = json_response(
            200,
            json!({ "data": { "Media": null }, "errors": [{ "message": "boom", "status": 502 }] }),
        );
        let err = attempts_for(gql_5xx, 3).await;
        assert_eq!(graphql_errors(&err)[0].status, Some(502));
    }

    #[tokio::test]
    async fn non_transient_errors_are_not_retried() {
        let bad_request = json_response(
            200,
            json!({ "errors": [{ "message": "x", "status": 400 }] }),
        );
        assert!(matches!(
            attempts_for(bad_request, 1).await,
            AniListError::GraphQL { .. }
        ));
        let not_found = json_response(404, json!({ "errors": [{ "message": "Not Found." }] }));
        assert!(matches!(
            attempts_for(not_found, 1).await,
            AniListError::NotFound
        ));
        assert!(matches!(
            attempts_for(raw_response(200, "not json"), 1).await,
            AniListError::Deserialization(_)
        ));
        assert!(matches!(
            attempts_for(raw_response(404, "gone"), 1).await,
            AniListError::Status { status: 404, .. }
        ));
    }

    #[tokio::test]
    async fn a_disabled_retry_policy_makes_one_attempt() {
        let server = serve(raw_response(503, "unavailable"), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(
            matches!(err, AniListError::Status { status: 503, .. }),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn a_retry_recovers_with_the_next_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(raw_response(503, "unavailable"))
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(json_response(200, details()))
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
        let movie = client.movie_details(5).await.expect("recovers");
        assert_eq!(movie.movie.provider_id, "anilist:5");
        // Every attempt sends the same request.
        let sent = requests(&server).await;
        assert_eq!(body_of(&sent[0]), body_of(&sent[1]));
    }

    #[tokio::test]
    async fn every_attempt_acquires_the_rate_limiter() {
        const WINDOW: Duration = Duration::from_millis(300);
        let server = serve(raw_response(503, "unavailable"), 3).await;
        // One request per window: the first attempt spends the only token, so
        // each of the two retries waits for a refill.
        let client = client_with(&server, |c| {
            c.with_retry(TWO_FAST_RETRIES)
                .with_rate_limit(RateLimit::per(1, WINDOW))
        });
        let started = Instant::now();
        let err = client.movie_details(5).await.unwrap_err();
        let elapsed = started.elapsed();
        assert!(matches!(err, AniListError::Status { status: 503, .. }));
        // Backoff is ~1ms per retry, so only the limiter explains the wait.
        assert!(
            elapsed >= WINDOW * 2 - Duration::from_millis(20),
            "three attempts took only {elapsed:?}"
        );
    }

    fn reported_page_info(current: Option<i32>, last: Option<i32>, total: Option<i32>) -> PageInfo {
        serde_json::from_value(json!({
            "currentPage": current,
            "lastPage": last,
            "total": total,
            "hasNextPage": true,
        }))
        .expect("page info")
    }

    /// Omitted `pageInfo` fields read as `1`/`0`/`0`; reported ones pass
    /// through.
    #[test]
    fn page_from_defaults_omitted_fields() {
        let page = page_from(&reported_page_info(None, None, None), vec![1]).expect("in range");
        assert_eq!((page.page, page.total_pages, page.total_results), (1, 0, 0));
        let page =
            page_from(&reported_page_info(Some(2), Some(5), Some(90)), vec![1]).expect("in range");
        assert_eq!(
            (page.page, page.total_pages, page.total_results),
            (2, 5, 90)
        );
    }

    /// A negative page number or total is a typed error naming the field,
    /// never wrapped into a huge `u32`.
    #[test]
    fn page_from_rejects_negative_fields() {
        let cases = [
            ("currentPage", reported_page_info(Some(-1), None, None)),
            ("lastPage", reported_page_info(Some(1), Some(-1), None)),
            (
                "total",
                reported_page_info(Some(1), Some(1), Some(i32::MIN)),
            ),
        ];
        for (expected, pi) in cases {
            match page_from(&pi, Vec::<u8>::new()) {
                Err(AniListError::PageOutOfRange { field, value }) => {
                    assert_eq!(field, expected);
                    assert!(value < 0, "{value}");
                }
                other => panic!("expected PageOutOfRange for {expected}, got {other:?}"),
            }
        }
    }

    /// Media entries with an invalid id are dropped from a page.
    #[test]
    fn media_pages_skip_invalid_ids() {
        let page: MediaPageData = serde_json::from_value(json!({
            "pageInfo": { "currentPage": 1, "lastPage": 1, "total": 3, "hasNextPage": false },
            "media": [{ "id": 1 }, { "id": 0 }, { "id": -2 }],
        }))
        .expect("media page");
        let movies = media_page_to_movies(Some(page)).expect("in range");
        let ids: Vec<_> = movies
            .results
            .iter()
            .map(|m| m.provider_id.to_string())
            .collect();
        assert_eq!(ids, ["anilist:1"]);
    }

    /// An invalid id in a details response gives way to the requested id.
    #[test]
    fn details_id_replaces_only_invalid_ids() {
        for reported in [0, -3] {
            let mut id = reported;
            details_id(&mut id, 5);
            assert_eq!(id, 5, "reported {reported}");
        }
        let mut id = 9;
        details_id(&mut id, 5);
        assert_eq!(id, 9);
    }

    /// A details response with an invalid id is converted under the
    /// requested id instead of a wrapped one.
    #[tokio::test]
    async fn movie_details_with_invalid_reported_id_uses_requested_id() {
        let server = serve(
            ResponseTemplate::new(200).set_body_json(json!({
                "data": { "Media": { "id": -1, "format": "MOVIE" } }
            })),
            1,
        )
        .await;
        let client = client_with(&server, |c| c);
        let movie = client.movie_details(5).await.expect("details");
        assert_eq!(movie.movie.provider_id, "anilist:5");
    }
}
