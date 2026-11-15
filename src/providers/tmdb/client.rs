use std::collections::HashMap;

use super::{
    config::{TmdbAuth, TmdbConfig},
    error::TmdbError,
    models::{TmdbCredits, TmdbImages},
};
use crate::{
    core::{config::TimeWindow, pagination::Page},
    generated::tmdb::{self, Credential, SecretString, types},
    providers::rate_limit::RateLimiter,
    unified::{
        conversions::tmdb::TmdbListItem,
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        genre::Genre,
        media_id::MediaId,
        models::{
            UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedSeasonDetails, UnifiedStreamingService, UnifiedTvShow,
            UnifiedTvShowDetails, UnifiedWatchProviderEntry, UnifiedWatchProviders,
        },
    },
};

const TMDB_BASE_URL: &str = "https://api.themoviedb.org";

/// The security scheme a v4 bearer token is registered under
/// (`scripts/tmdb-spec-patch.jq` declares it).
const V4_BEARER_SCHEME: &str = "v4_bearer";

/// The security scheme a v3 API key is registered under
/// (`scripts/tmdb-spec-patch.jq` declares it).
const V3_API_KEY_SCHEME: &str = "v3_api_key";

/// High-level TMDB API client wrapping the generated (spargen) client.
///
/// Adds bearer token authentication, rate limiting, and ergonomic pagination.
///
/// # Cloning
///
/// `TmdbClient` is cheaply cloneable — the underlying HTTP client,
/// configuration, and rate limiter are all reference-counted so clones share
/// the same connection pool and the same rate-limit budget.
///
/// # Debug output
///
/// The `Debug` output never contains the API credential: it shows the
/// (redacted) [`TmdbConfig`] and the rate limiter, and omits the underlying
/// generated client (whose own `Debug` redacts the registered credential).
#[derive(Clone)]
pub struct TmdbClient {
    inner: tmdb::Client,
    config: TmdbConfig,
    rate_limiter: RateLimiter,
}

impl std::fmt::Debug for TmdbClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TmdbClient")
            .field("config", &self.config)
            .field("rate_limiter", &self.rate_limiter)
            .finish_non_exhaustive()
    }
}

impl TmdbClient {
    /// Create a new TMDB client from the given configuration.
    pub fn new(config: TmdbConfig) -> Result<Self, TmdbError> {
        config.validate().map_err(TmdbError::InvalidConfig)?;

        if let TmdbAuth::V4Bearer(token) = &config.auth {
            // Reject a token that cannot travel in a header up front, rather
            // than failing every request.
            reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| TmdbError::InvalidConfig(format!("invalid API token: {e}")))?;
        }

        let http_client = crate::providers::http::build_client(
            config.connect_timeout,
            config.request_timeout,
            reqwest::header::HeaderMap::new(),
        )
        .map_err(TmdbError::from)?;

        let base_url = config.base_url.as_deref().unwrap_or(TMDB_BASE_URL);
        let inner = tmdb::Client::with_client(http_client, base_url)
            .map_err(|e| TmdbError::InvalidConfig(format!("invalid base URL: {e}")))?;
        // The spec patch declares both credentials as security schemes: a v4
        // token travels as `Authorization: Bearer`, a v3 key as the `api_key`
        // query parameter. The generated client redacts either in `Debug`.
        let inner = match &config.auth {
            TmdbAuth::V4Bearer(token) => inner.with_credential(
                V4_BEARER_SCHEME,
                Credential::Bearer(SecretString::from(token.clone())),
            ),
            TmdbAuth::V3ApiKey(key) => inner.with_credential(
                V3_API_KEY_SCHEME,
                Credential::ApiKey(SecretString::from(key.clone())),
            ),
        };
        let rate_limiter = RateLimiter::new(config.rate_limit);

        Ok(Self {
            inner,
            config,
            rate_limiter,
        })
    }

    /// Returns a reference to the underlying generated client (crate-internal;
    /// the generated types are not part of cameo's public API).
    pub(crate) fn inner(&self) -> &tmdb::Client {
        &self.inner
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &TmdbConfig {
        &self.config
    }

    // ── Helper accessors for default config values ──

    fn language(&self) -> Option<String> {
        self.config.language.clone()
    }

    fn region(&self) -> Option<String> {
        self.config.region.clone()
    }

    fn include_adult(&self) -> Option<bool> {
        self.config.include_adult
    }

    /// Clamp a page to TMDB's supported `1..=500` range and convert to the
    /// generated client's `i32` without a wrapping cast.
    fn tmdb_page(page: Option<u32>) -> Option<i32> {
        page.map(|p| p.clamp(1, 500) as i32)
    }

    /// Wait for a rate-limit slot before issuing a request. Returns once the
    /// token-bucket limiter permits another request (immediately if a token is
    /// available). Used by [`Self::execute`] and the discover builders.
    pub(crate) async fn acquire_rate_limit(&self) {
        self.rate_limiter.acquire().await;
    }

    /// Execute a generated-client call under the rate limiter, unwrap the
    /// response body, and map any failure into a structured [`TmdbError`]
    /// (decoding TMDB's `{status_code, status_message}` body and `Retry-After`
    /// header via [`TmdbError::from_generated`]).
    ///
    /// `make_call` is a closure returning a fresh request future each time it is
    /// invoked, so the retry loop can re-issue it. This is the single seam
    /// through which every endpoint runs — rate limiting and retries are
    /// applied here rather than at each call site. Retries re-acquire the rate
    /// limiter, so they are paced like any other request.
    async fn execute<T, E, F, Fut>(&self, make_call: F) -> Result<T, TmdbError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<tmdb::ResponseValue<T>, tmdb::Error<E>>>,
        E: std::fmt::Debug,
    {
        let policy = self.config.retry;
        let mut retry_index = 0u32;
        loop {
            self.acquire_rate_limit().await;
            let outcome = make_call()
                .await
                .map(tmdb::ResponseValue::into_inner)
                .map_err(TmdbError::from_generated);
            match outcome {
                Ok(value) => return Ok(value),
                Err(err) => {
                    if policy.should_retry(retry_index) && err.is_retryable() {
                        let delay = policy.backoff(retry_index, err.retry_after());
                        tracing::debug!(
                            retry = retry_index,
                            ?delay,
                            error = %err,
                            "tmdb: retrying transient error"
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

    // ── Search ──

    /// Search for movies by title.
    #[tracing::instrument(skip(self))]
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::SearchMovieParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
            region: self.region(),
            ..Default::default()
        };
        let body = self
            .execute(|| self.inner.search_movie(query, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Search for TV shows by name.
    #[tracing::instrument(skip(self))]
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::SearchTvParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
            ..Default::default()
        };
        let body = self
            .execute(|| self.inner.search_tv(query, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Search for people by name.
    #[tracing::instrument(skip(self))]
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, TmdbError> {
        let params = tmdb::SearchPersonParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_person(query, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Multi-search across movies, TV shows, and people.
    #[tracing::instrument(skip(self))]
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, TmdbError> {
        let params = tmdb::SearchMultiParams {
            include_adult: self.include_adult(),
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.search_multi(query, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .filter_map(crate::unified::conversions::tmdb::person::multi_search_result))
    }

    // ── Details ──

    /// Get detailed information about a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_details(&self, movie_id: i32) -> Result<UnifiedMovieDetails, TmdbError> {
        let params = tmdb::MovieDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.movie_details(movie_id, params.clone()))
            .await?;
        // TMDB may send a null/missing id; fall back to the requested one.
        body.id.get_or_insert(i64::from(movie_id));
        Ok(body.into())
    }

    /// Get detailed information about a movie with appended responses.
    #[tracing::instrument(skip(self))]
    pub async fn movie_details_with_append(
        &self,
        movie_id: i32,
        append: &str,
    ) -> Result<UnifiedMovieDetails, TmdbError> {
        let params = tmdb::MovieDetailsParams {
            append_to_response: Some(append.to_owned()),
            language: self.language(),
        };
        let mut body = self
            .execute(|| self.inner.movie_details(movie_id, params.clone()))
            .await?;
        body.id.get_or_insert(i64::from(movie_id));
        Ok(body.into())
    }

    /// Get detailed information about a TV series.
    #[tracing::instrument(skip(self))]
    pub async fn tv_series_details(
        &self,
        series_id: i32,
    ) -> Result<UnifiedTvShowDetails, TmdbError> {
        let params = tmdb::TvSeriesDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.tv_series_details(series_id, params.clone()))
            .await?;
        body.id.get_or_insert(i64::from(series_id));
        Ok(body.into())
    }

    /// Get detailed information about a person.
    #[tracing::instrument(skip(self))]
    pub async fn person_details(&self, person_id: i32) -> Result<UnifiedPersonDetails, TmdbError> {
        let params = tmdb::PersonDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| self.inner.person_details(person_id, params.clone()))
            .await?;
        body.id.get_or_insert(i64::from(person_id));
        Ok(body.into())
    }

    // ── Credits ──

    /// Get the cast and crew for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_credits(&self, movie_id: i32) -> Result<TmdbCredits, TmdbError> {
        self.movie_credits_response(movie_id).await.map(Into::into)
    }

    /// Get the cast and crew for a TV series.
    ///
    /// Comes from `/tv/{id}/aggregate_credits`, which lists each person once.
    /// [`TmdbCredits`] keeps one entry per person, holding only the first of
    /// their characters and the first of their jobs.
    #[tracing::instrument(skip(self))]
    pub async fn tv_series_credits(&self, series_id: i32) -> Result<TmdbCredits, TmdbError> {
        self.tv_series_aggregate_credits_response(series_id)
            .await
            .map(Into::into)
    }

    /// Fetch `/movie/{id}/credits` as TMDB returned it.
    pub(crate) async fn movie_credits_response(
        &self,
        movie_id: i32,
    ) -> Result<types::MovieCreditsResponse, TmdbError> {
        let params = tmdb::MovieCreditsParams {
            language: self.language(),
        };
        self.execute(|| self.inner.movie_credits(movie_id, params.clone()))
            .await
    }

    /// Fetch `/tv/{id}/aggregate_credits` as TMDB returned it, with every
    /// character and job of each person.
    pub(crate) async fn tv_series_aggregate_credits_response(
        &self,
        series_id: i32,
    ) -> Result<types::TvSeriesAggregateCreditsResponse, TmdbError> {
        let params = tmdb::TvSeriesAggregateCreditsParams {
            language: self.language(),
        };
        self.execute(|| {
            self.inner
                .tv_series_aggregate_credits(series_id, params.clone())
        })
        .await
    }

    // ── Trending ──

    /// Get trending movies.
    ///
    /// # Note
    ///
    /// Trending endpoints do not support pagination. The `page` argument is
    /// accepted for API consistency but is ignored by TMDB — this method
    /// always returns the first (and only) page of trending results.
    #[tracing::instrument(skip(self, _page))]
    pub async fn trending_movies(
        &self,
        time_window: TimeWindow,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let tw = match time_window {
            TimeWindow::Day => types::TrendingMoviesTimeWindow::Day,
            TimeWindow::Week => types::TrendingMoviesTimeWindow::Week,
        };
        let params = tmdb::TrendingMoviesParams {
            language: self.language(),
        };
        let body = self
            .execute(|| self.inner.trending_movies(tw.clone(), params.clone()))
            .await?;
        Ok(offset_page(
            None,
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get trending TV shows.
    ///
    /// # Note
    ///
    /// Trending endpoints do not support pagination. The `page` argument is
    /// accepted for API consistency but is ignored by TMDB — this method
    /// always returns the first (and only) page of trending results.
    #[tracing::instrument(skip(self, _page))]
    pub async fn trending_tv(
        &self,
        time_window: TimeWindow,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let tw = match time_window {
            TimeWindow::Day => types::TrendingTvTimeWindow::Day,
            TimeWindow::Week => types::TrendingTvTimeWindow::Week,
        };
        let params = tmdb::TrendingTvParams {
            language: self.language(),
        };
        let body = self
            .execute(|| self.inner.trending_tv(tw.clone(), params.clone()))
            .await?;
        Ok(offset_page(
            None,
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    // ── Popular / Top Rated ──

    /// Get popular movies.
    #[tracing::instrument(skip(self))]
    pub async fn popular_movies(&self, page: Option<u32>) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::MoviePopularListParams {
            language: self.language(),
            page: Self::tmdb_page(page),
            region: self.region(),
        };
        let body = self
            .execute(|| self.inner.movie_popular_list(params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get top-rated movies.
    #[tracing::instrument(skip(self))]
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::MovieTopRatedListParams {
            language: self.language(),
            page: Self::tmdb_page(page),
            region: self.region(),
        };
        let body = self
            .execute(|| self.inner.movie_top_rated_list(params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get popular TV shows.
    #[tracing::instrument(skip(self))]
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::TvSeriesPopularListParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.tv_series_popular_list(params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get top-rated TV shows.
    #[tracing::instrument(skip(self))]
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::TvSeriesTopRatedListParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.tv_series_top_rated_list(params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    // ── Recommendations / Similar ──

    /// Get movie recommendations based on a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_recommendations(
        &self,
        movie_id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::MovieRecommendationsParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        // The spec patch gives this endpoint the similar-movies response shape.
        let body = self
            .execute(|| self.inner.movie_recommendations(movie_id, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(UnifiedMovie::from))
    }

    /// Get TV show recommendations based on a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_recommendations(
        &self,
        series_id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let params = tmdb::TvSeriesRecommendationsParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| {
                self.inner
                    .tv_series_recommendations(series_id, params.clone())
            })
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get movies similar to a given movie.
    #[tracing::instrument(skip(self))]
    pub async fn similar_movies(
        &self,
        movie_id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let params = tmdb::MovieSimilarParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| self.inner.movie_similar(movie_id, params.clone()))
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    /// Get TV shows similar to a given TV show.
    ///
    /// # Note
    ///
    /// The generated `tv_series_similar` endpoint takes a `&str` series ID
    /// (unlike most other endpoints that take `i32`). The integer `series_id`
    /// is converted to a string before calling the generated client so that
    /// the correct path (e.g. `/3/tv/1399/similar`) is constructed.
    #[tracing::instrument(skip(self))]
    pub async fn similar_tv_shows(
        &self,
        series_id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let id_str = series_id.to_string();
        let params = tmdb::TvSeriesSimilarParams {
            language: self.language(),
            page: Self::tmdb_page(page),
        };
        let body = self
            .execute(|| {
                self.inner
                    .tv_series_similar(id_str.as_str(), params.clone())
            })
            .await?;
        Ok(offset_page(
            Self::tmdb_page(page),
            body.page,
            body.results,
            body.total_pages,
            body.total_results,
        )
        .map(Into::into))
    }

    // ── Season / Episode ──

    /// Get season details for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_season_details(
        &self,
        series_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, TmdbError> {
        let params = tmdb::TvSeasonDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner
                    .tv_season_details(series_id, season_number as i32, params.clone())
            })
            .await?;
        body.season_number.get_or_insert(i64::from(season_number));
        let show_id = MediaId::tmdb(series_id as u64);
        Ok(crate::unified::conversions::tmdb::tv::season_details_from(
            body, show_id,
        ))
    }

    /// Get episode details for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn tv_episode_details(
        &self,
        series_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, TmdbError> {
        let params = tmdb::TvEpisodeDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner.tv_episode_details(
                    series_id,
                    season_number as i32,
                    episode_number as i32,
                    params.clone(),
                )
            })
            .await?;
        body.episode_number.get_or_insert(i64::from(episode_number));
        Ok(body.into())
    }

    // ── Watch Providers ──

    /// Get streaming providers for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_watch_providers(
        &self,
        movie_id: i32,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let body = self
            .execute(|| self.inner.movie_watch_providers(movie_id))
            .await?;
        let provider_id = MediaId::tmdb(movie_id as u64);
        let results = match body.results {
            Some(r) => parse_watch_provider_results(serde_json::to_value(r)?)?,
            None => HashMap::new(),
        };
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
        let body = self
            .execute(|| self.inner.tv_series_watch_providers(series_id))
            .await?;
        let provider_id = MediaId::tmdb(series_id as u64);
        let results = match body.results {
            Some(r) => parse_watch_provider_results(serde_json::to_value(r)?)?,
            None => HashMap::new(),
        };
        Ok(UnifiedWatchProviders {
            provider_id,
            results,
        })
    }

    // ── External IDs ──

    /// Get the external ids (IMDb, Wikidata) of a movie.
    ///
    /// The returned [`ExternalIds`] also carries `tmdb: Some(movie_id)`. TMDB
    /// does not track TheTVDB or MyAnimeList ids for movies, so those stay
    /// `None`; empty strings TMDB sends for unknown ids are reported as `None`.
    #[tracing::instrument(skip(self))]
    pub async fn movie_external_ids(&self, movie_id: i32) -> Result<ExternalIds, TmdbError> {
        let body = self
            .execute(|| self.inner.movie_external_ids(movie_id))
            .await?;
        // The requested id, not the body's (TMDB may send a null `id`).
        let mut ids = ExternalIds::new().with_tmdb(movie_id as u64);
        ids.imdb = non_empty(body.imdb_id);
        // The spec leaves the movie `wikidata_id` untyped; it is a string or null.
        ids.wikidata = non_empty(
            body.wikidata_id
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        );
        Ok(ids)
    }

    /// Get the external ids (IMDb, TheTVDB, Wikidata) of a TV series.
    ///
    /// The returned [`ExternalIds`] also carries `tmdb: Some(series_id)`.
    #[tracing::instrument(skip(self))]
    pub async fn tv_external_ids(&self, series_id: i32) -> Result<ExternalIds, TmdbError> {
        let body = self
            .execute(|| self.inner.tv_series_external_ids(series_id))
            .await?;
        // The requested id, not the body's (TMDB may send a null `id`).
        let mut ids = ExternalIds::new().with_tmdb(series_id as u64);
        ids.imdb = non_empty(body.imdb_id);
        ids.tvdb = body.tvdb_id.and_then(|n| u64::try_from(n).ok());
        ids.wikidata = non_empty(body.wikidata_id);
        Ok(ids)
    }

    /// Find TMDB movies and TV shows by an IMDb, TheTVDB, or Wikidata id
    /// (TMDB's `/find` endpoint).
    ///
    /// Only movie and TV-series results are returned (movies first); TMDB's
    /// person, season, and episode matches are dropped because they are not
    /// titles a [`MatchKind`](crate::unified::MatchKind) can describe. TV
    /// results with a missing or malformed `id` are skipped. No match yields
    /// an empty `Vec`.
    ///
    /// # Errors
    ///
    /// [`TmdbError::UnsupportedSource`] for MyAnimeList ids (TMDB does not
    /// index them), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn find_by_external_id(
        &self,
        id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, TmdbError> {
        let source = match id.source {
            ExternalSource::Imdb => types::FindByIdExternalSource::ImdbId,
            ExternalSource::Tvdb => types::FindByIdExternalSource::TvdbId,
            ExternalSource::Wikidata => types::FindByIdExternalSource::WikidataId,
            other => return Err(TmdbError::UnsupportedSource(other)),
        };
        let params = tmdb::FindByIdParams {
            language: self.language(),
        };
        let body = self
            .execute(|| {
                self.inner
                    .find_by_id(id.id.as_str(), source.clone(), params.clone())
            })
            .await?;
        let movies = body
            .movie_results
            .iter()
            .flatten()
            .filter_map(|m| m.id.and_then(|n| u64::try_from(n).ok()))
            .map(|n| ExternalMatch::movie(MediaId::tmdb(n)));
        // `tv_results` is untyped in the spec; only the `id` is needed.
        let shows = body
            .tv_results
            .iter()
            .flatten()
            .filter_map(|v| v.get("id").and_then(serde_json::Value::as_u64))
            .map(|n| ExternalMatch::tv_show(MediaId::tmdb(n)));
        Ok(movies.chain(shows).collect())
    }

    // ── Genres ──

    /// Get the list of official movie genres.
    #[tracing::instrument(skip(self))]
    pub async fn movie_genres(&self) -> Result<Vec<Genre>, TmdbError> {
        let params = tmdb::GenreMovieListParams {
            language: self.language(),
        };
        let resp = self
            .execute(|| self.inner.genre_movie_list(params.clone()))
            .await?;
        Ok(resp
            .genres
            .into_iter()
            .flatten()
            .filter_map(|g| g.id.map(Genre::from_tmdb_id))
            .collect())
    }

    /// Get the list of official TV show genres.
    #[tracing::instrument(skip(self))]
    pub async fn tv_genres(&self) -> Result<Vec<Genre>, TmdbError> {
        let params = tmdb::GenreTvListParams {
            language: self.language(),
        };
        let resp = self
            .execute(|| self.inner.genre_tv_list(params.clone()))
            .await?;
        Ok(resp
            .genres
            .into_iter()
            .flatten()
            .filter_map(|g| g.id.map(Genre::from_tmdb_id))
            .collect())
    }

    // ── Images ──

    /// Get images for a movie.
    #[tracing::instrument(skip(self))]
    pub async fn movie_images(&self, movie_id: i32) -> Result<TmdbImages, TmdbError> {
        let params = tmdb::MovieImagesParams {
            language: self.language(),
            ..Default::default()
        };
        self.execute(|| self.inner.movie_images(movie_id, params.clone()))
            .await
            .map(Into::into)
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

// ── Typed recommendation / watch-provider response models ───────────────────────

/// Build a [`Page`] from TMDB offset-pagination fields, tolerating `null`s,
/// and drop result items without a TMDB id (see [`TmdbListItem`]).
///
/// `requested` is the page number sent to TMDB (`None` when no page was sent).
/// A missing or invalid `page` falls back to `requested` (or `1`), a missing
/// `total_pages` falls back to that page (so `has_next` is `false`), and a
/// missing `total_results` falls back to `0`, and missing `results` read as
/// an empty list.
pub(crate) fn offset_page<T: TmdbListItem>(
    requested: Option<i32>,
    page: Option<i64>,
    results: Option<Vec<T>>,
    total_pages: Option<i64>,
    total_results: Option<i64>,
) -> Page<T> {
    let to_u32 = |v: i64| u32::try_from(v).ok();
    let requested = requested.and_then(|p| u32::try_from(p).ok()).unwrap_or(1);
    let page = page.and_then(to_u32).unwrap_or(requested);
    let total_pages = total_pages.and_then(to_u32).unwrap_or(page);
    let total_results = total_results.and_then(to_u32).unwrap_or(0);
    let results = results
        .unwrap_or_default()
        .into_iter()
        .filter(|item| item.tmdb_id().is_some())
        .collect();
    Page::offset(page, results, total_pages, total_results)
}

/// One streaming service under a watch-provider country entry.
#[derive(serde::Deserialize)]
struct WatchProviderService {
    provider_name: Option<String>,
    logo_path: Option<String>,
}

/// One country's watch-provider lists.
#[derive(serde::Deserialize, Default)]
struct WatchProviderCountry {
    #[serde(default)]
    flatrate: Vec<WatchProviderService>,
    #[serde(default)]
    rent: Vec<WatchProviderService>,
    #[serde(default)]
    buy: Vec<WatchProviderService>,
}

impl From<WatchProviderCountry> for UnifiedWatchProviderEntry {
    fn from(c: WatchProviderCountry) -> Self {
        UnifiedWatchProviderEntry {
            flatrate: unified_services(c.flatrate),
            rent: unified_services(c.rent),
            buy: unified_services(c.buy),
        }
    }
}

/// Map watch-provider services to unified services, dropping any without a name.
fn unified_services(list: Vec<WatchProviderService>) -> Vec<UnifiedStreamingService> {
    use crate::providers::tmdb::image_url::{ImageUrl, LogoSize};
    list.into_iter()
        .filter_map(|s| {
            let name = s.provider_name?;
            Some(UnifiedStreamingService {
                name,
                logo_url: s
                    .logo_path
                    .as_deref()
                    .map(|p| ImageUrl::logo(p, LogoSize::W92)),
            })
        })
        .collect()
}

/// Treat an empty string (TMDB's placeholder for an unknown external id) as absent.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty())
}

/// Parse TMDB watch-provider results (a per-country object) into unified entries
/// via typed serde structs rather than hand-indexed JSON.
fn parse_watch_provider_results(
    value: serde_json::Value,
) -> Result<HashMap<String, UnifiedWatchProviderEntry>, TmdbError> {
    let by_country: HashMap<String, WatchProviderCountry> = serde_json::from_value(value)?;
    Ok(by_country
        .into_iter()
        .map(|(country, entry)| (country, entry.into()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "SECRETKEY123";

    /// The generated client's `Debug` must not expose the credential for
    /// either auth scheme, while still showing which scheme is registered.
    #[test]
    fn generated_client_debug_redacts_credentials() {
        for (config, scheme) in [
            (TmdbConfig::new(SECRET), V4_BEARER_SCHEME),
            (TmdbConfig::new_v3(SECRET), V3_API_KEY_SCHEME),
        ] {
            let client = TmdbClient::new(config).expect("valid config");
            for debug in [
                format!("{:?}", client.inner()),
                format!("{:#?}", client.inner()),
            ] {
                assert!(!debug.contains(SECRET), "credential leaked: {debug}");
                assert!(debug.contains(scheme), "{debug}");
            }
        }
    }

    /// The `_url` accessors never carry the v3 key: it is attached only when a
    /// request is sent.
    #[test]
    fn v3_key_is_not_in_url_accessor() {
        let client = TmdbClient::new(TmdbConfig::new_v3(SECRET)).expect("valid config");
        let url = client
            .inner()
            .movie_details_url(550, None)
            .expect("valid url");
        assert_eq!(url.path(), "/3/movie/550");
        assert!(!url.as_str().contains(SECRET), "{url}");
    }
}
