//! Recommendation endpoints: recommended and similar movies and TV shows.

use super::TmdbClient;
use crate::{
    core::pagination::Page,
    generated::tmdb,
    providers::tmdb::{convert::TmdbPaged, error::TmdbError},
    unified::models::{UnifiedMovie, UnifiedTvShow},
};

impl TmdbClient {
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
        Ok(body
            .into_page(Self::tmdb_page(page))?
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
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
        Ok(body.into_page(Self::tmdb_page(page))?.map(Into::into))
    }
}
