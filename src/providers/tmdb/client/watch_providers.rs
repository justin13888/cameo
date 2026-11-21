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
    /// [`TmdbError::Request`] when `movie_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn movie_watch_providers(
        &self,
        movie_id: u64,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let wire_id = requested_id(movie_id)?;
        let provider_id = MediaId::tmdb(movie_id);
        let body = self
            .execute(|| self.inner.movie_watch_providers(wire_id))
            .await?;
        Ok(UnifiedWatchProviders {
            provider_id,
            results: parse_watch_provider_results(body.results.into_option().map(|r| r.additional)),
        })
    }

    /// Get streaming providers for a TV show.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is zero or above `i32::MAX`
    /// (nothing is sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn tv_watch_providers(
        &self,
        series_id: u64,
    ) -> Result<UnifiedWatchProviders, TmdbError> {
        let wire_id = requested_id(series_id)?;
        let provider_id = MediaId::tmdb(series_id);
        let body = self
            .execute(|| self.inner.tv_series_watch_providers(wire_id))
            .await?;
        Ok(UnifiedWatchProviders {
            provider_id,
            results: parse_watch_provider_results(body.results.into_option().map(|r| r.additional)),
        })
    }
}
