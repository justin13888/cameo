use super::{CameoClient, CameoClientError};
use crate::unified::models::{UnifiedEpisode, UnifiedSeasonDetails};

impl CameoClient {
    /// Get details for a specific season of a TV show.
    pub async fn season_details(
        &self,
        show_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, CameoClientError> {
        let _ = (show_id, season_number);
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(show_id, season_number, "dispatching season_details to tmdb");
            return Ok(client.tv_season_details(show_id, season_number).await?);
        }

        Err(CameoClientError::NotConfigured)
    }

    /// Get details for a specific episode of a TV show.
    pub async fn episode_details(
        &self,
        show_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, CameoClientError> {
        let _ = (show_id, season_number, episode_number);
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

        Err(CameoClientError::NotConfigured)
    }
}
