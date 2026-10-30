//! Unified provider-trait implementations for [`AniListClient`].
//!
//! AniList supports search, details, discovery, and external ids (MyAnimeList
//! only). It does not implement the
//! recommendation, season, or watch-availability traits, so the facade reports
//! those capabilities as [`Unsupported`](crate::unified::CameoClientError::Unsupported)
//! when AniList is the only configured provider.

use async_trait::async_trait;

use super::client::AniListClient;
use crate::{
    core::{config::TimeWindow, error::ProviderError, pagination::Page},
    unified::{
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        media_id::{MediaId, NativeId},
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
        traits::{
            Capability, DetailProvider, DiscoveryProvider, ExternalIdProvider, Provider,
            SearchProvider,
        },
    },
};

const ANILIST_CAPABILITIES: &[Capability] = &[
    Capability::Search,
    Capability::Details,
    Capability::Discovery,
    Capability::ExternalIds,
];

/// External sources AniList can look anime up by.
const ANILIST_EXTERNAL_SOURCES: &[ExternalSource] = &[ExternalSource::Mal];

impl Provider for AniListClient {
    fn id(&self) -> &str {
        "anilist"
    }

    fn capabilities(&self) -> &[Capability] {
        ANILIST_CAPABILITIES
    }
}

/// Extract the numeric AniList id from a [`MediaId`], handling namespaced staff
/// ids (`anilist:staff:95061`) as well as plain media ids (`anilist:1`).
fn anilist_id(id: &MediaId) -> Result<i32, ProviderError> {
    if id.provider() != "anilist" {
        return Err(ProviderError::InvalidInput(format!(
            "expected an anilist id, got {id}"
        )));
    }
    let numeric = match id.native() {
        NativeId::Num(n) => *n,
        // e.g. "staff:95061" -> 95061
        NativeId::Text(s) => s
            .rsplit(':')
            .next()
            .and_then(|part| part.parse::<u64>().ok())
            .ok_or_else(|| {
                ProviderError::InvalidInput(format!("not a numeric AniList id: {id}"))
            })?,
    };
    i32::try_from(numeric)
        .map_err(|_| ProviderError::InvalidInput(format!("AniList id out of range: {id}")))
}

#[async_trait]
impl SearchProvider for AniListClient {
    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(AniListClient::search_movies(self, query, page).await?)
    }

    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(AniListClient::search_tv_shows(self, query, page).await?)
    }

    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        Ok(AniListClient::search_people(self, query, page).await?)
    }

    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        Ok(AniListClient::search_multi(self, query, page).await?)
    }
}

#[async_trait]
impl DetailProvider for AniListClient {
    async fn movie_details(&self, id: &MediaId) -> Result<UnifiedMovieDetails, ProviderError> {
        Ok(AniListClient::movie_details(self, anilist_id(id)?).await?)
    }

    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError> {
        Ok(AniListClient::tv_show_details(self, anilist_id(id)?).await?)
    }

    async fn person_details(&self, id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError> {
        Ok(AniListClient::person_details(self, anilist_id(id)?).await?)
    }
}

#[async_trait]
impl DiscoveryProvider for AniListClient {
    async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(AniListClient::trending_movies(self, time_window, page).await?)
    }

    async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(AniListClient::trending_tv(self, time_window, page).await?)
    }

    async fn popular_movies(&self, page: Option<u32>) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(AniListClient::popular_movies(self, page).await?)
    }

    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(AniListClient::top_rated_movies(self, page).await?)
    }

    async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(AniListClient::popular_tv_shows(self, page).await?)
    }

    async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(AniListClient::top_rated_tv_shows(self, page).await?)
    }
}

/// External ids are fetched per anime regardless of format, so the movie and
/// TV variants share one query.
#[async_trait]
impl ExternalIdProvider for AniListClient {
    fn external_sources(&self) -> &[ExternalSource] {
        ANILIST_EXTERNAL_SOURCES
    }

    async fn movie_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        Ok(AniListClient::external_ids(self, anilist_id(id)?).await?)
    }

    async fn tv_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        Ok(AniListClient::external_ids(self, anilist_id(id)?).await?)
    }

    async fn find_by_external_id(
        &self,
        id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, ProviderError> {
        if id.source != ExternalSource::Mal {
            return Err(ProviderError::Unsupported);
        }
        let mal_id = id
            .id
            .parse::<i32>()
            .map_err(|_| ProviderError::InvalidInput(format!("not a valid MAL id: {id}")))?;
        Ok(AniListClient::find_by_mal_id(self, mal_id)
            .await?
            .into_iter()
            .collect())
    }
}
