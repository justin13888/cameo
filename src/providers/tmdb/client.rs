use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::core::config::TimeWindow;
use crate::core::pagination::PaginatedResponse;
use crate::generated::tmdb::{self, types};

use super::error::TmdbError;

const TMDB_BASE_URL: &str = "https://api.themoviedb.org";
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

/// High-level TMDB API client wrapping the generated progenitor client.
///
/// Adds bearer token authentication, rate limiting, and ergonomic pagination.
pub struct TmdbClient {
    inner: tmdb::Client,
    config: TmdbConfig,
    #[allow(dead_code)]
    rate_limiter: Arc<Semaphore>,
}

impl TmdbClient {
    /// Create a new TMDB client from the given configuration.
    pub fn new(config: TmdbConfig) -> Result<Self, TmdbError> {
        if config.api_token.is_empty() {
            return Err(TmdbError::InvalidConfig(
                "API token must not be empty".into(),
            ));
        }

        let mut headers = reqwest::header::HeaderMap::new();
        let auth_value = format!("Bearer {}", config.api_token);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&auth_value)
                .map_err(|e| TmdbError::InvalidConfig(format!("invalid API token: {e}")))?,
        );

        let http_client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .map_err(TmdbError::Http)?;

        let base_url = config.base_url.as_deref().unwrap_or(TMDB_BASE_URL);
        let inner = tmdb::Client::new_with_client(base_url, http_client);
        let rate_limiter = Arc::new(Semaphore::new(config.rate_limit as usize));

        Ok(Self {
            inner,
            config,
            rate_limiter,
        })
    }

    /// Returns a reference to the underlying generated client for direct access.
    pub fn inner(&self) -> &tmdb::Client {
        &self.inner
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &TmdbConfig {
        &self.config
    }

    // ── Helper accessors for default config values ──

    fn language(&self) -> Option<&str> {
        self.config.language.as_deref()
    }

    fn region(&self) -> Option<&str> {
        self.config.region.as_deref()
    }

    fn include_adult(&self) -> Option<bool> {
        self.config.include_adult
    }

    // ── Search ──

    /// Search for movies by title.
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::SearchMovieResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .search_movie(
                self.include_adult(),
                self.language(),
                page.map(|p| p as i32),
                None, // primary_release_year
                query,
                self.region(),
                None, // year
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Search for TV shows by name.
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::SearchTvResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .search_tv(
                None, // first_air_date_year
                self.include_adult(),
                self.language(),
                page.map(|p| p as i32),
                query,
                None, // year
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Search for people by name.
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::SearchPersonResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .search_person(
                self.include_adult(),
                self.language(),
                page.map(|p| p as i32),
                query,
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Multi-search across movies, TV shows, and people.
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::SearchMultiResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .search_multi(
                self.include_adult(),
                self.language(),
                page.map(|p| p as i32),
                query,
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    // ── Details ──

    /// Get detailed information about a movie.
    pub async fn movie_details(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieDetailsResponse, TmdbError> {
        let resp = self
            .inner
            .movie_details(movie_id, None, self.language())
            .await?;
        Ok(resp.into_inner())
    }

    /// Get detailed information about a movie with appended responses.
    pub async fn movie_details_with_append(
        &self,
        movie_id: i32,
        append: &str,
    ) -> Result<types::MovieDetailsResponse, TmdbError> {
        let resp = self
            .inner
            .movie_details(movie_id, Some(append), self.language())
            .await?;
        Ok(resp.into_inner())
    }

    /// Get detailed information about a TV series.
    pub async fn tv_series_details(
        &self,
        series_id: i32,
    ) -> Result<types::TvSeriesDetailsResponse, TmdbError> {
        let resp = self
            .inner
            .tv_series_details(series_id, None, self.language())
            .await?;
        Ok(resp.into_inner())
    }

    /// Get detailed information about a person.
    pub async fn person_details(
        &self,
        person_id: i32,
    ) -> Result<types::PersonDetailsResponse, TmdbError> {
        let resp = self
            .inner
            .person_details(person_id, None, self.language())
            .await?;
        Ok(resp.into_inner())
    }

    // ── Credits ──

    /// Get the cast and crew for a movie.
    pub async fn movie_credits(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieCreditsResponse, TmdbError> {
        let resp = self
            .inner
            .movie_credits(movie_id, self.language())
            .await?;
        Ok(resp.into_inner())
    }

    /// Get the cast and crew for a TV series.
    pub async fn tv_series_credits(
        &self,
        series_id: i32,
    ) -> Result<types::TvSeriesAggregateCreditsResponse, TmdbError> {
        let resp = self
            .inner
            .tv_series_aggregate_credits(series_id, self.language())
            .await?;
        Ok(resp.into_inner())
    }

    // ── Trending ──

    /// Get trending movies (always returns page 1; trending endpoints don't support pagination).
    pub async fn trending_movies(
        &self,
        time_window: TimeWindow,
        _page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TrendingMoviesResponseResultsItem>, TmdbError> {
        let tw = match time_window {
            TimeWindow::Day => types::TrendingMoviesTimeWindow::Day,
            TimeWindow::Week => types::TrendingMoviesTimeWindow::Week,
        };
        let resp = self.inner.trending_movies(tw, self.language()).await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get trending TV shows (always returns page 1; trending endpoints don't support pagination).
    pub async fn trending_tv(
        &self,
        time_window: TimeWindow,
        _page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TrendingTvResponseResultsItem>, TmdbError> {
        let tw = match time_window {
            TimeWindow::Day => types::TrendingTvTimeWindow::Day,
            TimeWindow::Week => types::TrendingTvTimeWindow::Week,
        };
        let resp = self.inner.trending_tv(tw, self.language()).await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    // ── Popular / Top Rated ──

    /// Get popular movies.
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::MoviePopularListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .movie_popular_list(
                self.language(),
                page.map(|p| p as i32),
                self.region(),
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get top-rated movies.
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::MovieTopRatedListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .movie_top_rated_list(
                self.language(),
                page.map(|p| p as i32),
                self.region(),
            )
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    // ── Genres ──

    /// Get the list of official movie genres.
    pub async fn movie_genres(
        &self,
    ) -> Result<types::GenreMovieListResponse, TmdbError> {
        let resp = self.inner.genre_movie_list(self.language()).await?;
        Ok(resp.into_inner())
    }

    /// Get the list of official TV show genres.
    pub async fn tv_genres(
        &self,
    ) -> Result<types::GenreTvListResponse, TmdbError> {
        let resp = self.inner.genre_tv_list(self.language()).await?;
        Ok(resp.into_inner())
    }

    // ── Images ──

    /// Get images for a movie.
    pub async fn movie_images(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieImagesResponse, TmdbError> {
        let resp = self
            .inner
            .movie_images(
                movie_id,
                None, // include_image_language
                self.language(),
            )
            .await?;
        Ok(resp.into_inner())
    }

    // ── Discover Builders ──

    /// Create a builder for discovering movies with filters.
    pub fn discover_movies(&self) -> super::builders::DiscoverMoviesBuilder<'_> {
        super::builders::DiscoverMoviesBuilder::new(self)
    }

    /// Create a builder for discovering TV shows with filters.
    pub fn discover_tv(&self) -> super::builders::DiscoverTvBuilder<'_> {
        super::builders::DiscoverTvBuilder::new(self)
    }
}
