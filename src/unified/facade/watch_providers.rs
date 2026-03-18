use super::{CameoClient, CameoClientError};
use crate::unified::{models::UnifiedWatchProviders, traits::WatchProviderTrait};

impl WatchProviderTrait for CameoClient {
    type Error = CameoClientError;

    async fn movie_watch_providers(
        &self,
        id: i32,
    ) -> Result<UnifiedWatchProviders, CameoClientError> {
        let _ = id;
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, "dispatching movie_watch_providers to tmdb");
            return Ok(client.movie_watch_providers(id).await?);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn tv_watch_providers(&self, id: i32) -> Result<UnifiedWatchProviders, CameoClientError> {
        let _ = id;
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, "dispatching tv_watch_providers to tmdb");
            return Ok(client.tv_watch_providers(id).await?);
        }

        Err(CameoClientError::NoProviders)
    }
}
