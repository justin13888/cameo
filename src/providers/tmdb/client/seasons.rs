//! Season and episode endpoints.

use super::{TmdbClient, requested_id};
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
    /// [`TmdbError::Request`] when `series_id` is zero or negative (nothing is
    /// sent), plus the usual request errors.
    #[tracing::instrument(skip(self))]
    pub async fn season_details(
        &self,
        series_id: i32,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, TmdbError> {
        let show_id = MediaId::tmdb(requested_id(series_id)?);
        let params = tmdb::TvSeasonDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner
                    .tv_season_details(series_id, season_number as i32, params.clone())
            })
            .await?;
        body.season_number.get_or_insert(i64::from(season_number));
        Ok(season_details_from(body, show_id))
    }

    /// Get episode details for a TV show.
    #[tracing::instrument(skip(self))]
    pub async fn episode_details(
        &self,
        series_id: i32,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, TmdbError> {
        let params = tmdb::TvEpisodeDetailsParams {
            language: self.language(),
            ..Default::default()
        };
        let mut body = self
            .execute(|| {
                self.inner.tv_episode_details(
                    series_id,
                    season_number as i32,
                    episode_number as i32,
                    params.clone(),
                )
            })
            .await?;
        body.episode_number.get_or_insert(i64::from(episode_number));
        Ok(body.into())
    }
}
