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
pub struct AniListClient {
    http: reqwest::Client,
    config: AniListConfig,
}

impl AniListClient {
    /// Create a new AniList client from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AniListError::Http`] if the underlying HTTP client cannot be built
    /// (e.g. invalid TLS configuration).
    pub fn new(config: AniListConfig) -> Result<Self, AniListError> {
        let http = reqwest::ClientBuilder::new().build()?;
        Ok(Self { http, config })
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &AniListConfig {
        &self.config
    }

    // ── GraphQL execution engine ───────────────────────────────────────────────

    /// Execute a GraphQL query and deserialize the `data` field.
    async fn graphql<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: Value,
    ) -> Result<T, AniListError> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let resp = self
            .http
            .post(&self.config.base_url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await?;

        let gql_resp: GraphQlResponse<T> = resp.json().await?;

        if let Some(errors) = gql_resp.errors {
            if !errors.is_empty() {
                return Err(AniListError::GraphQL(errors));
            }
        }

        gql_resp.data.ok_or(AniListError::NoData)
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
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        Ok(media_page_to_movies(resp))
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
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        Ok(media_page_to_tv(resp))
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
        let resp: StaffPageResponse = self.graphql(query::SEARCH_STAFF, vars).await?;
        let pi = &resp.page.page_info;
        Ok(PaginatedResponse {
            page: pi.current_page.unwrap_or(1) as u32,
            total_pages: pi.last_page.unwrap_or(1) as u32,
            total_results: pi.total.unwrap_or(0) as u32,
            results: resp
                .page
                .staff
                .into_iter()
                .map(|s| crate::unified::conversions::anilist::staff_to_person(s))
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
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
            "formatIn": Value::Null,
        });
        let resp: MediaPageResponse = self.graphql(query::SEARCH_ANIME, vars).await?;
        let pi = &resp.page.page_info;
        let results = resp
            .page
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
        let resp: MediaDetailResponse = self.graphql(query::MEDIA_DETAILS, vars).await?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_movie_details(resp.media))
    }

    /// Get full details for an anime TV series by AniList ID.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, AniListError> {
        let vars = json!({ "id": id });
        let resp: MediaDetailResponse = self.graphql(query::MEDIA_DETAILS, vars).await?;
        Ok(crate::unified::conversions::anilist::anilist_media_detail_to_tv_details(resp.media))
    }

    /// Get full details for a staff member by AniList ID.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, AniListError> {
        let vars = json!({ "id": id });
        let resp: StaffDetailResponse = self.graphql(query::STAFF_DETAILS, vars).await?;
        Ok(crate::unified::conversions::anilist::staff_detail_to_person_details(resp.staff))
    }

    // ── Discovery ──────────────────────────────────────────────────────────────

    /// Get trending anime movies. AniList has no time window concept; `time_window` is ignored.
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
        let resp: MediaPageResponse = self.graphql(query::LIST_TRENDING_ANIME, vars).await?;
        Ok(media_page_to_movies(resp))
    }

    /// Get trending anime series. AniList has no time window concept; `time_window` is ignored.
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
        let resp: MediaPageResponse = self.graphql(query::LIST_TRENDING_ANIME, vars).await?;
        Ok(media_page_to_tv(resp))
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
        let resp: MediaPageResponse = self.graphql(query::LIST_POPULAR_ANIME, vars).await?;
        Ok(media_page_to_movies(resp))
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
        let resp: MediaPageResponse = self.graphql(query::LIST_TOP_SCORED_ANIME, vars).await?;
        Ok(media_page_to_movies(resp))
    }
}

// ── Page conversion helpers ───────────────────────────────────────────────────

fn media_page_to_movies(resp: MediaPageResponse) -> PaginatedResponse<UnifiedMovie> {
    let pi = &resp.page.page_info;
    PaginatedResponse {
        page: pi.current_page.unwrap_or(1) as u32,
        total_pages: pi.last_page.unwrap_or(1) as u32,
        total_results: pi.total.unwrap_or(0) as u32,
        results: resp
            .page
            .media
            .into_iter()
            .map(anilist_media_to_movie)
            .collect(),
    }
}

fn media_page_to_tv(resp: MediaPageResponse) -> PaginatedResponse<UnifiedTvShow> {
    let pi = &resp.page.page_info;
    PaginatedResponse {
        page: pi.current_page.unwrap_or(1) as u32,
        total_pages: pi.last_page.unwrap_or(1) as u32,
        total_results: pi.total.unwrap_or(0) as u32,
        results: resp
            .page
            .media
            .into_iter()
            .map(anilist_media_to_tv)
            .collect(),
    }
}
