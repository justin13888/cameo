//! Discovery endpoints: trending, popular, and top-rated lists, and the
//! discover builders.

use super::TmdbClient;
use crate::{
    core::{config::TimeWindow, pagination::Page},
    generated::tmdb::{self, types},
    providers::tmdb::{
        builders::{DiscoverMoviesBuilder, DiscoverTvBuilder},
        convert::{TmdbPaged, single_page},
        error::TmdbError,
    },
    unified::models::{UnifiedMovie, UnifiedTvShow},
};

impl TmdbClient {
    // ── Trending ──

    /// Get trending movies.
    ///
    /// # Note
    ///
    /// TMDB trending endpoints are not paginated, so this method takes no
    /// page argument and always returns the first (and only) page of
    /// trending results: the page reports `has_next == false` and
    /// `total_pages == 1`, whatever TMDB's body claims.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn trending_movies(
        &self,
        time_window: TimeWindow,
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
        Ok(single_page(body.into_page(None)?).map(Into::into))
    }

    /// Get trending TV shows.
    ///
    /// # Note
    ///
    /// TMDB trending endpoints are not paginated, so this method takes no
    /// page argument and always returns the first (and only) page of
    /// trending results: the page reports `has_next == false` and
    /// `total_pages == 1`, whatever TMDB's body claims.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
    #[tracing::instrument(skip(self))]
    pub async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
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
        Ok(single_page(body.into_page(None)?).map(Into::into))
    }

    // ── Popular / Top Rated ──

    /// Get popular movies.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Get top-rated movies.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Get popular TV shows.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    /// Get top-rated TV shows.
    ///
    /// `page` is clamped as [`TmdbClient`'s pages](TmdbClient#pages) are.
    ///
    /// # Errors
    ///
    /// The [usual request errors](TmdbClient#errors).
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }

    // ── Discover Builders ──

    /// Create a builder for discovering movies with filters.
    pub fn discover_movies(&self) -> DiscoverMoviesBuilder<'_> {
        DiscoverMoviesBuilder::new(self)
    }

    /// Create a builder for discovering TV shows with filters.
    pub fn discover_tv(&self) -> DiscoverTvBuilder<'_> {
        DiscoverTvBuilder::new(self)
    }

    /// Run a built discover-movie query through [`Self::execute`], so it is
    /// rate limited, retried, and error-decoded like every other endpoint.
    /// [`DiscoverMoviesBuilder::execute`] is its only caller.
    pub(crate) async fn discover_movies_page(
        &self,
        query: tmdb::DiscoverMovieParams,
    ) -> Result<Page<UnifiedMovie>, TmdbError> {
        let page = query.page;
        let body = self
            .execute(|| self.inner.discover_movie(query.clone()))
            .await?;
        Ok(body.into_page(page)?.map(Into::into))
    }

    /// Run a built discover-TV query through [`Self::execute`], so it is rate
    /// limited, retried, and error-decoded like every other endpoint.
    /// [`DiscoverTvBuilder::execute`] is its only caller.
    pub(crate) async fn discover_tv_page(
        &self,
        query: tmdb::DiscoverTvParams,
    ) -> Result<Page<UnifiedTvShow>, TmdbError> {
        let page = query.page;
        let body = self
            .execute(|| self.inner.discover_tv(query.clone()))
            .await?;
        Ok(body.into_page(page)?.map(Into::into))
    }
}
