use super::{CameoClient, CameoClientError};
use crate::unified::models::{UnifiedEpisode, UnifiedSeasonDetails};

impl CameoClient {
    /// Get details for a specific season of a TV show.
    pub async fn season_details(
        &self,
        show_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, CameoClientError> {
        let media_id = self.media_id_for(&self.season, show_id)?;
        Ok(self
            .require(&self.season)?
            .season_details(&media_id, season_number)
            .await?)
    }

    /// Get details for a specific episode of a TV show.
    pub async fn episode_details(
        &self,
        show_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, CameoClientError> {
        let media_id = self.media_id_for(&self.season, show_id)?;
        Ok(self
            .require(&self.season)?
            .episode_details(&media_id, season_number, episode_number)
            .await?)
    }
}
