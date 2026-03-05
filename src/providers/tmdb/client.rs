use std::{collections::HashMap, sync::Arc};

use tokio::sync::Semaphore;

use super::{config::TmdbConfig, error::TmdbError};
use crate::{
    core::{config::TimeWindow, pagination::PaginatedResponse},
    generated::tmdb::{self, types},
    unified::models::{
        UnifiedEpisode, UnifiedMovie, UnifiedSeasonDetails, UnifiedStreamingService,
        UnifiedWatchProviderEntry, UnifiedWatchProviders,
    },
};

const TMDB_BASE_URL: &str = "https://api.themoviedb.org";

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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn movie_credits(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieCreditsResponse, TmdbError> {
        let resp = self.inner.movie_credits(movie_id, self.language()).await?;
        Ok(resp.into_inner())
    }

    /// Get the cast and crew for a TV series.
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self, _page))]
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
    #[tracing::instrument(skip(self, _page))]
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
    #[tracing::instrument(skip(self))]
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::MoviePopularListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .movie_popular_list(self.language(), page.map(|p| p as i32), self.region())
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
    #[tracing::instrument(skip(self))]
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::MovieTopRatedListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .movie_top_rated_list(self.language(), page.map(|p| p as i32), self.region())
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get popular TV shows.
    #[tracing::instrument(skip(self))]
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TvSeriesPopularListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .tv_series_popular_list(self.language(), page.map(|p| p as i32))
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get top-rated TV shows.
    #[tracing::instrument(skip(self))]
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TvSeriesTopRatedListResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .tv_series_top_rated_list(self.language(), page.map(|p| p as i32))
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    // ── Recommendations / Similar ──

    /// Get movie recommendations based on a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_recommendations(
        &self,
        movie_id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, TmdbError> {
        let resp = self
            .inner
            .movie_recommendations(movie_id, self.language(), page.map(|p| p as i32))
            .await?;
        let map = resp.into_inner();
        parse_movie_recs_from_map(map)
    }

    /// Get TV show recommendations based on a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_recommendations(
        &self,
        series_id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TvSeriesRecommendationsResponseResultsItem>, TmdbError>
    {
        let resp = self
            .inner
            .tv_series_recommendations(series_id, self.language(), page.map(|p| p as i32))
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get movies similar to a given movie.
    #[tracing::instrument(skip(self))]
    pub async fn similar_movies(
        &self,
        movie_id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::MovieSimilarResponseResultsItem>, TmdbError> {
        let resp = self
            .inner
            .movie_similar(movie_id, self.language(), page.map(|p| p as i32))
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    /// Get TV shows similar to a given TV show.
    #[tracing::instrument(skip(self))]
    pub async fn similar_tv_shows(
        &self,
        series_id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<types::TvSeriesSimilarResponseResultsItem>, TmdbError> {
        let id_str = series_id.to_string();
        let resp = self
            .inner
            .tv_series_similar(&id_str, self.language(), page.map(|p| p as i32))
            .await?;
        let body = resp.into_inner();
        Ok(PaginatedResponse {
            page: body.page as u32,
            results: body.results,
            total_pages: body.total_pages as u32,
            total_results: body.total_results as u32,
        })
    }

    // ── Season / Episode ──

    /// Get season details for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_season_details(
        &self,
        series_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, TmdbError> {
        let resp = self
            .inner
            .tv_season_details(series_id, season_number as i32, None, self.language())
            .await?;
        let body = resp.into_inner();
        let show_id = format!("tmdb:{series_id}");
        let mut details: UnifiedSeasonDetails = body.into();
        details.show_id = show_id;
        Ok(details)
    }

    /// Get episode details for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_episode_details(
        &self,
        series_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, TmdbError> {
        let resp = self
            .inner
            .tv_episode_details(
                series_id,
                season_number as i32,
                episode_number as i32,
                None,
                self.language(),
            )
            .await?;
        Ok(resp.into_inner().into())
    }

    // ── Watch Providers ──

    /// Get streaming providers for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_watch_providers(
        &self,
        movie_id: i32,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let resp = self.inner.movie_watch_providers(movie_id).await?;
        let body = resp.into_inner();
        let provider_id = format!("tmdb:{movie_id}");
        let results = body
            .results
            .map(|r| parse_watch_provider_results(serde_json::to_value(r).unwrap_or_default()))
            .unwrap_or_default();
        Ok(UnifiedWatchProviders {
            provider_id,
            results,
        })
    }

    /// Get streaming providers for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_watch_providers(
        &self,
        series_id: i32,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let resp = self.inner.tv_series_watch_providers(series_id).await?;
        let body = resp.into_inner();
        let provider_id = format!("tmdb:{series_id}");
        let results = body
            .results
            .map(|r| parse_watch_provider_results(serde_json::to_value(r).unwrap_or_default()))
            .unwrap_or_default();
        Ok(UnifiedWatchProviders {
            provider_id,
            results,
        })
    }

    // ── Genres ──

    /// Get the list of official movie genres.
    #[tracing::instrument(skip(self))]
    pub async fn movie_genres(&self) -> Result<types::GenreMovieListResponse, TmdbError> {
        let resp = self.inner.genre_movie_list(self.language()).await?;
        Ok(resp.into_inner())
    }

    /// Get the list of official TV show genres.
    #[tracing::instrument(skip(self))]
    pub async fn tv_genres(&self) -> Result<types::GenreTvListResponse, TmdbError> {
        let resp = self.inner.genre_tv_list(self.language()).await?;
        Ok(resp.into_inner())
    }

    // ── Images ──

    /// Get images for a movie.
    #[tracing::instrument(skip(self))]
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

// ── Private helpers ───────────────────────────────────────────────────────────

/// Parse movie recommendations from the raw JSON map returned by the generated client.
fn parse_movie_recs_from_map(
    map: serde_json::Map<String, serde_json::Value>,
) -> Result<PaginatedResponse<UnifiedMovie>, TmdbError> {
    use crate::{
        providers::tmdb::image_url::{BackdropSize, ImageUrl, PosterSize},
        unified::genre::Genre,
    };

    let value = serde_json::Value::Object(map);
    let page = value["page"].as_i64().unwrap_or(1) as u32;
    let total_pages = value["total_pages"].as_i64().unwrap_or(1) as u32;
    let total_results = value["total_results"].as_i64().unwrap_or(0) as u32;
    let empty = vec![];
    let arr = value["results"].as_array().unwrap_or(&empty);

    let results = arr
        .iter()
        .filter_map(|v| {
            let id = v["id"].as_i64()?;
            let title = v["title"].as_str().unwrap_or_default().to_string();
            let original_title = v["original_title"].as_str().map(String::from);
            let overview = v["overview"].as_str().map(String::from);
            let release_date = v["release_date"].as_str().map(String::from);
            let poster_url = v["poster_path"]
                .as_str()
                .map(|p| ImageUrl::poster(p, PosterSize::W500));
            let backdrop_url = v["backdrop_path"]
                .as_str()
                .map(|p| ImageUrl::backdrop(p, BackdropSize::W780));
            let genre_ids: Vec<i64> = v["genre_ids"]
                .as_array()
                .map(|a| a.iter().filter_map(|g| g.as_i64()).collect())
                .unwrap_or_default();
            let genres = genre_ids
                .iter()
                .map(|&gid| Genre::from_tmdb_id(gid))
                .collect();
            let popularity = v["popularity"].as_f64();
            let vote_average = v["vote_average"].as_f64();
            let vote_count = v["vote_count"].as_i64().unwrap_or(0) as u64;
            let original_language = v["original_language"].as_str().map(String::from);
            let adult = v["adult"].as_bool().unwrap_or(false);
            Some(UnifiedMovie {
                provider_id: format!("tmdb:{id}"),
                title,
                original_title,
                overview,
                release_date,
                poster_url,
                backdrop_url,
                genres,
                popularity,
                vote_average,
                vote_count,
                original_language,
                adult,
            })
        })
        .collect();

    Ok(PaginatedResponse {
        page,
        total_pages,
        total_results,
        results,
    })
}

/// Parse watch provider results from a serde_json::Value (serialized country map).
fn parse_watch_provider_results(
    value: serde_json::Value,
) -> HashMap<String, UnifiedWatchProviderEntry> {
    use crate::providers::tmdb::image_url::{ImageUrl, LogoSize};

    let Some(obj) = value.as_object() else {
        return HashMap::new();
    };

    obj.iter()
        .map(|(country_code, entry_val)| {
            let flatrate =
                parse_provider_list(&entry_val["flatrate"], &ImageUrl::logo, LogoSize::W92);
            let rent = parse_provider_list(&entry_val["rent"], &ImageUrl::logo, LogoSize::W92);
            let buy = parse_provider_list(&entry_val["buy"], &ImageUrl::logo, LogoSize::W92);
            (
                country_code.clone(),
                UnifiedWatchProviderEntry {
                    flatrate,
                    rent,
                    buy,
                },
            )
        })
        .collect()
}

fn parse_provider_list<F>(
    val: &serde_json::Value,
    logo_fn: &F,
    size: crate::providers::tmdb::image_url::LogoSize,
) -> Vec<UnifiedStreamingService>
where
    F: Fn(&str, crate::providers::tmdb::image_url::LogoSize) -> String,
{
    val.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let name = v["provider_name"].as_str()?.to_string();
                    let logo_url = v["logo_path"].as_str().map(|p| logo_fn(p, size));
                    Some(UnifiedStreamingService { name, logo_url })
                })
                .collect()
        })
        .unwrap_or_default()
}
