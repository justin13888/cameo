use super::{CameoClient, CameoClientError};
use crate::unified::models::UnifiedWatchProviders;

impl CameoClient {
    /// Get streaming availability for a movie.
    pub async fn movie_watch_providers(
        &self,
        id: i32,
    ) -> Result<UnifiedWatchProviders, CameoClientError> {
        let media_id = self.media_id_for(&self.watch, id)?;
        Ok(self
            .require(&self.watch)?
            .movie_watch_providers(&media_id)
            .await?)
    }

    /// Get streaming availability for a TV show.
    pub async fn tv_watch_providers(
        &self,
        id: i32,
    ) -> Result<UnifiedWatchProviders, CameoClientError> {
        let media_id = self.media_id_for(&self.watch, id)?;
        Ok(self
            .require(&self.watch)?
            .tv_watch_providers(&media_id)
            .await?)
    }
}
