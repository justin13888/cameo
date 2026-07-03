use super::{CameoClient, CameoClientError};
use crate::unified::{media_id::MediaId, models::UnifiedWatchProviders};

impl CameoClient {
    /// Get streaming availability for a movie.
    pub async fn movie_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, CameoClientError> {
        Ok(self
            .route(&self.watch, id)?
            .movie_watch_providers(id)
            .await?)
    }

    /// Get streaming availability for a TV show.
    pub async fn tv_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, CameoClientError> {
        Ok(self.route(&self.watch, id)?.tv_watch_providers(id).await?)
    }
}
