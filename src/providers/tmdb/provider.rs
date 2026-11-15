//! Unified provider-trait implementations for [`TmdbClient`].
//!
//! The two-layer pattern (ADR 0001): the inherent methods on [`TmdbClient`]
//! expose the native surface; these impls adapt them to the unified traits,
//! converting to unified models and mapping [`TmdbError`](super::error::TmdbError)
//! into [`ProviderError`](crate::core::error::ProviderError).

use async_trait::async_trait;

use super::client::TmdbClient;
use crate::{
    core::{config::TimeWindow, error::ProviderError, pagination::Page},
    unified::{
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        media_id::MediaId,
        models::{
            UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
            UnifiedWatchProviders,
        },
        traits::{
            Capability, DetailProvider, DiscoveryProvider, ExternalIdProvider, Provider,
            RecommendationProvider, SearchProvider, SeasonProvider, WatchAvailabilityProvider,
        },
    },
};

const TMDB_CAPABILITIES: &[Capability] = &[
    Capability::Search,
    Capability::Details,
    Capability::Discovery,
    Capability::Recommendations,
    Capability::Seasons,
    Capability::WatchAvailability,
    Capability::ExternalIds,
];

/// External sources TMDB's `/find` endpoint accepts.
const TMDB_EXTERNAL_SOURCES: &[ExternalSource] = &[
    ExternalSource::Imdb,
    ExternalSource::Tvdb,
    ExternalSource::Wikidata,
];

impl Provider for TmdbClient {
    fn id(&self) -> &str {
        "tmdb"
    }

    fn capabilities(&self) -> &[Capability] {
        TMDB_CAPABILITIES
    }
}

/// Extract the numeric TMDB id from a [`MediaId`], rejecting non-TMDB or
/// non-numeric ids as [`ProviderError::InvalidInput`].
fn tmdb_id(id: &MediaId) -> Result<i32, ProviderError> {
    if id.provider() != "tmdb" {
        return Err(ProviderError::InvalidInput(format!(
            "expected a tmdb id, got {id}"
        )));
    }
    id.as_u64()
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(|| ProviderError::InvalidInput(format!("not a valid TMDB numeric id: {id}")))
}

#[async_trait]
impl SearchProvider for TmdbClient {
    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::search_movies(self, query, page).await?)
    }

    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::search_tv_shows(self, query, page).await?)
    }

    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        Ok(TmdbClient::search_people(self, query, page).await?)
    }

    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        Ok(TmdbClient::search_multi(self, query, page).await?)
    }
}

#[async_trait]
impl DetailProvider for TmdbClient {
    async fn movie_details(&self, id: &MediaId) -> Result<UnifiedMovieDetails, ProviderError> {
        Ok(TmdbClient::movie_details(self, tmdb_id(id)?).await?)
    }

    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError> {
        Ok(TmdbClient::tv_series_details(self, tmdb_id(id)?).await?)
    }

    async fn person_details(&self, id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError> {
        Ok(TmdbClient::person_details(self, tmdb_id(id)?).await?)
    }
}

#[async_trait]
impl DiscoveryProvider for TmdbClient {
    async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::trending_movies(self, time_window, page).await?)
    }

    async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::trending_tv(self, time_window, page).await?)
    }

    async fn popular_movies(&self, page: Option<u32>) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::popular_movies(self, page).await?)
    }

    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::top_rated_movies(self, page).await?)
    }

    async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::popular_tv_shows(self, page).await?)
    }

    async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::top_rated_tv_shows(self, page).await?)
    }
}

#[async_trait]
impl RecommendationProvider for TmdbClient {
    async fn movie_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::movie_recommendations(self, tmdb_id(id)?, page).await?)
    }

    async fn tv_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::tv_recommendations(self, tmdb_id(id)?, page).await?)
    }

    async fn similar_movies(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(TmdbClient::similar_movies(self, tmdb_id(id)?, page).await?)
    }

    async fn similar_tv_shows(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(TmdbClient::similar_tv_shows(self, tmdb_id(id)?, page).await?)
    }
}

#[async_trait]
impl SeasonProvider for TmdbClient {
    async fn season_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, ProviderError> {
        Ok(TmdbClient::tv_season_details(self, tmdb_id(show_id)?, season_number).await?)
    }

    async fn episode_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, ProviderError> {
        Ok(
            TmdbClient::tv_episode_details(self, tmdb_id(show_id)?, season_number, episode_number)
                .await?,
        )
    }
}

#[async_trait]
impl WatchAvailabilityProvider for TmdbClient {
    async fn movie_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError> {
        Ok(TmdbClient::movie_watch_providers(self, tmdb_id(id)?).await?)
    }

    async fn tv_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError> {
        Ok(TmdbClient::tv_watch_providers(self, tmdb_id(id)?).await?)
    }
}

#[async_trait]
impl ExternalIdProvider for TmdbClient {
    fn external_sources(&self) -> &[ExternalSource] {
        TMDB_EXTERNAL_SOURCES
    }

    async fn movie_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        Ok(TmdbClient::movie_external_ids(self, tmdb_id(id)?).await?)
    }

    async fn tv_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        Ok(TmdbClient::tv_external_ids(self, tmdb_id(id)?).await?)
    }

    async fn find_by_external_id(
        &self,
        id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, ProviderError> {
        Ok(TmdbClient::find_by_external_id(self, id).await?)
    }
}
