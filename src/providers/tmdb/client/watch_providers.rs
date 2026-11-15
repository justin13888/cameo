//! Watch-provider (streaming availability) endpoints.

use super::{TmdbClient, requested_id};
use crate::{
    providers::tmdb::{convert::watch_providers::parse_watch_provider_results, error::TmdbError},
    unified::{media_id::MediaId, models::UnifiedWatchProviders},
};

impl TmdbClient {
    /// Get streaming providers for a movie.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `movie_id` is zero or negative (nothing is
    /// sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn movie_watch_providers(
        &self,
        movie_id: i32,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let provider_id = MediaId::tmdb(requested_id(movie_id)?);
        let body = self
            .execute(|| self.inner.movie_watch_providers(movie_id))
            .await?;
        Ok(UnifiedWatchProviders {
            provider_id,
            results: parse_watch_provider_results(body.results.map(|r| r.additional)),
        })
    }

    /// Get streaming providers for a TV show.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is zero or negative (nothing is
    /// sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn tv_watch_providers(
        &self,
        series_id: i32,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let provider_id = MediaId::tmdb(requested_id(series_id)?);
        let body = self
            .execute(|| self.inner.tv_series_watch_providers(series_id))
            .await?;
        Ok(UnifiedWatchProviders {
            provider_id,
            results: parse_watch_provider_results(body.results.map(|r| r.additional)),
        })
    }
}
