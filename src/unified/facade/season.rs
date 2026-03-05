use async_trait::async_trait;

use super::{CameoClient, CameoClientError};
use crate::unified::{
    models::{UnifiedEpisode, UnifiedSeasonDetails},
    traits::SeasonProvider,
};

#[async_trait]
impl SeasonProvider for CameoClient {
    type Error = CameoClientError;

    async fn season_details(
        &self,
        show_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(show_id, season_number, "dispatching season_details to tmdb");
            return Ok(client.tv_season_details(show_id, season_number).await?);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn episode_details(
        &self,
        show_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(
                show_id,
                season_number,
                episode_number,
                "dispatching episode_details to tmdb"
            );
            return Ok(client
                .tv_episode_details(show_id, season_number, episode_number)
                .await?);
        }

        Err(CameoClientError::NoProviders)
    }
}
