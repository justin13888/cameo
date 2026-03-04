//! AniList GraphQL client implementation.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{
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

const DEFAULT_PER_PAGE: u32 = 20;

/// AniList format strings for anime that map to the "movie" media type.
const MOVIE_FORMATS: &[&str] = &["MOVIE"];

/// AniList format strings for anime that map to the "TV show" media type.
const TV_FORMATS: &[&str] = &["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"];

// ── Configuration ──────────────────────────────────────────────────────────────

/// Configuration for the [`AniListClient`].
#[derive(Debug, Clone)]
pub struct AniListConfig {
    /// Base URL for the AniList GraphQL endpoint.
    ///
    /// Defaults to `https://graphql.anilist.co`.  Override for testing.
    pub base_url: String,
    /// Number of results returned per page (default: 20, max: 50).
    pub per_page: u32,
}

impl Default for AniListConfig {
    fn default() -> Self {
        Self {
            base_url: "https://graphql.anilist.co".to_string(),
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl AniListConfig {
    /// Create a new config with default settings.
    ///
    /// No authentication is required for AniList public data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with a custom base URL (useful for testing with mock servers).
    pub fn new_with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::default()
        }
    }

    /// Set the number of results per page.
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page;
        self
    }
}

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
/// let client = AniListClient::new(AniListConfig::new());
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
    pub fn new(config: AniListConfig) -> Self {
        let http = reqwest::ClientBuilder::new()
            .build()
            .expect("failed to build reqwest client");
        Self { http, config }
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    use super::*;

    fn mock_media_item(id: i32, title: &str, format: &str) -> Value {
        json!({
            "id": id,
            "title": { "romaji": title, "english": title, "native": title },
            "description": "A test anime.",
            "startDate": { "year": 2020, "month": 1, "day": 1 },
            "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
            "bannerImage": null,
            "genres": ["Action", "Drama"],
            "popularity": 50000,
            "averageScore": 85,
            "episodes": 12,
            "duration": 24,
            "status": "FINISHED",
            "format": format,
            "countryOfOrigin": "JP",
            "isAdult": false
        })
    }

    fn mock_page_response(media: Vec<Value>, total: i32) -> Value {
        json!({
            "data": {
                "Page": {
                    "pageInfo": {
                        "total": total,
                        "currentPage": 1,
                        "lastPage": (total + 19) / 20,
                        "hasNextPage": total > 20,
                        "perPage": 20
                    },
                    "media": media
                }
            }
        })
    }

    fn mock_staff_item(id: i32, name: &str) -> Value {
        json!({
            "id": id,
            "name": { "full": name, "native": name },
            "image": { "large": "https://example.com/profile.jpg" },
            "description": "A voice actor.",
            "primaryOccupations": ["Voice Actor"],
            "languageV2": "Japanese"
        })
    }

    async fn setup_server_with_response(body: Value) -> (MockServer, AniListClient) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        let config = AniListConfig::new_with_base_url(server.uri());
        let client = AniListClient::new(config);
        (server, client)
    }

    #[tokio::test]
    async fn test_search_movies_returns_unified_movies() {
        let media = vec![mock_media_item(1535, "Your Name", "MOVIE")];
        let body = mock_page_response(media, 1);
        let (_server, client) = setup_server_with_response(body).await;

        let result = client.search_movies("Your Name", None).await.unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].title, "Your Name");
        assert_eq!(result.results[0].provider_id, "anilist:1535");
        assert_eq!(result.results[0].vote_average, Some(8.5));
        assert!(!result.results[0].adult);
        assert_eq!(result.page, 1);
    }

    #[tokio::test]
    async fn test_search_tv_shows_returns_unified_tv() {
        let media = vec![mock_media_item(11757, "Sword Art Online", "TV")];
        let body = mock_page_response(media, 1);
        let (_server, client) = setup_server_with_response(body).await;

        let result = client
            .search_tv_shows("Sword Art Online", None)
            .await
            .unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].name, "Sword Art Online");
        assert_eq!(result.results[0].provider_id, "anilist:11757");
    }

    #[tokio::test]
    async fn test_search_people_returns_unified_persons() {
        let body = json!({
            "data": {
                "Page": {
                    "pageInfo": { "total": 1, "currentPage": 1, "lastPage": 1, "hasNextPage": false, "perPage": 20 },
                    "staff": [mock_staff_item(95061, "Yuki Kaji")]
                }
            }
        });
        let (_server, client) = setup_server_with_response(body).await;

        let result = client.search_people("Yuki Kaji", None).await.unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].name, "Yuki Kaji");
        assert_eq!(result.results[0].provider_id, "anilist:staff:95061");
        assert_eq!(
            result.results[0].known_for_department.as_deref(),
            Some("Voice Acting")
        );
    }

    #[tokio::test]
    async fn test_search_multi_returns_movies_and_tv() {
        let media = vec![
            mock_media_item(1, "Movie A", "MOVIE"),
            mock_media_item(2, "Series B", "TV"),
        ];
        let body = mock_page_response(media, 2);
        let (_server, client) = setup_server_with_response(body).await;

        let result = client.search_multi("test", None).await.unwrap();
        assert_eq!(result.results.len(), 2);
        assert!(matches!(result.results[0], UnifiedSearchResult::Movie(_)));
        assert!(matches!(result.results[1], UnifiedSearchResult::TvShow(_)));
    }

    #[tokio::test]
    async fn test_movie_details_returns_details() {
        let body = json!({
            "data": {
                "Media": {
                    "id": 1575,
                    "title": { "romaji": "Spirited Away", "english": "Spirited Away", "native": "千と千尋の神隠し" },
                    "description": "A girl in a spirit world.",
                    "startDate": { "year": 2001, "month": 7, "day": 20 },
                    "endDate": { "year": 2001, "month": 7, "day": 20 },
                    "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
                    "bannerImage": null,
                    "genres": ["Adventure", "Fantasy"],
                    "popularity": 450000,
                    "averageScore": 92,
                    "episodes": 1,
                    "duration": 125,
                    "status": "FINISHED",
                    "format": "MOVIE",
                    "countryOfOrigin": "JP",
                    "isAdult": false,
                    "season": null,
                    "seasonYear": null,
                    "studios": { "nodes": [{ "name": "Studio Ghibli" }] }
                }
            }
        });
        let (_server, client) = setup_server_with_response(body).await;

        let result = client.movie_details(1575).await.unwrap();
        assert_eq!(result.movie.title, "Spirited Away");
        assert_eq!(result.movie.provider_id, "anilist:1575");
        assert_eq!(result.production_companies, vec!["Studio Ghibli"]);
        assert_eq!(result.runtime, Some(125));
    }

    #[tokio::test]
    async fn test_person_details_returns_staff_details() {
        let body = json!({
            "data": {
                "Staff": {
                    "id": 95061,
                    "name": { "full": "Yuki Kaji", "native": "梶裕貴", "alternative": [] },
                    "image": { "large": "https://example.com/profile.jpg" },
                    "description": "A voice actor.",
                    "primaryOccupations": ["Voice Actor"],
                    "gender": "Male",
                    "dateOfBirth": { "year": 1986, "month": 9, "day": 3 },
                    "dateOfDeath": null,
                    "homeTown": "Tokyo, Japan",
                    "siteUrl": "https://anilist.co/staff/95061",
                    "languageV2": "Japanese"
                }
            }
        });
        let (_server, client) = setup_server_with_response(body).await;

        let result = client.person_details(95061).await.unwrap();
        assert_eq!(result.person.name, "Yuki Kaji");
        assert_eq!(result.person.provider_id, "anilist:staff:95061");
        assert_eq!(result.birthday.as_deref(), Some("1986-09-03"));
    }

    #[tokio::test]
    async fn test_graphql_errors_propagate() {
        let body = json!({
            "errors": [{ "message": "Not Found." }]
        });
        let (_server, client) = setup_server_with_response(body).await;

        let err = client.search_movies("anything", None).await.unwrap_err();
        assert!(matches!(err, AniListError::GraphQL(_)));
    }

    #[tokio::test]
    async fn test_trending_movies_ignores_time_window() {
        let media = vec![mock_media_item(1, "Trending Movie", "MOVIE")];
        let body = mock_page_response(media, 1);
        let (_server, client) = setup_server_with_response(body).await;

        // Both time windows should work — AniList ignores the window.
        let day = client.trending_movies(TimeWindow::Day, None).await.unwrap();
        assert_eq!(day.results.len(), 1);
    }
}
