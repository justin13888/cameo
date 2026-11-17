//! Season and episode endpoints.

use super::{TmdbClient, requested_id, wire_int};
use crate::{
    generated::tmdb,
    providers::tmdb::{convert::tv::season_details_from, error::TmdbError},
    unified::{
        media_id::MediaId,
        models::{UnifiedEpisode, UnifiedSeasonDetails},
    },
};

impl TmdbClient {
    /// Get season details for a TV show.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id` is zero, or `series_id` or
    /// `season_number` is above `i32::MAX` (nothing is sent), plus the usual
    /// request errors.
    #[tracing::instrument(skip(self))]
    pub async fn season_details(
        &self,
        series_id: u64,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, TmdbError> {
        let wire_series_id = requested_id(series_id)?;
        let show_id = MediaId::tmdb(series_id);
        let wire_season = wire_int(u64::from(season_number), "season number")?;
        let params = tmdb::TvSeasonDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner
                    .tv_season_details(wire_series_id, wire_season, params.clone())
            })
            .await?;
        body.season_number.get_or_insert(i64::from(season_number));
        Ok(season_details_from(body, show_id))
    }

    /// Get episode details for a TV show.
    ///
    /// # Errors
    ///
    /// [`TmdbError::Request`] when `series_id`, `season_number`, or
    /// `episode_number` is above `i32::MAX` (nothing is sent), plus the usual
    /// request errors.
    #[tracing::instrument(skip(self))]
    pub async fn episode_details(
        &self,
        series_id: u64,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, TmdbError> {
        let series_id = wire_int(series_id, "id")?;
        let wire_season = wire_int(u64::from(season_number), "season number")?;
        let wire_episode = wire_int(u64::from(episode_number), "episode number")?;
        let params = tmdb::TvEpisodeDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner
                    .tv_episode_details(series_id, wire_season, wire_episode, params.clone())
            })
            .await?;
        body.episode_number.get_or_insert(i64::from(episode_number));
        Ok(body.into())
    }
}
