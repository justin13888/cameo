use super::{CameoClient, CameoClientError};
use crate::unified::{
    media_id::MediaId,
    models::{UnifiedEpisode, UnifiedSeasonDetails},
};

impl CameoClient {
    /// Get details for a specific season of a TV show.
    pub async fn season_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, CameoClientError> {
        Ok(self
            .route(&self.season, show_id)?
            .season_details(show_id, season_number)
            .await?)
    }

    /// Get details for a specific episode of a TV show.
    pub async fn episode_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, CameoClientError> {
        Ok(self
            .route(&self.season, show_id)?
            .episode_details(show_id, season_number, episode_number)
            .await?)
    }
}
