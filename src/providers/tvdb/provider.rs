//! Unified provider-trait implementations for [`TvdbClient`].
//!
//! TheTVDB supports search, details, seasons, and external ids.
//!
//! - **Search:** series and movies. `search_multi` returns both and omits
//!   every other kind (people, companies); `search_people` is
//!   [`ProviderError::Unsupported`].
//! - **Details:** series and movies; `person_details` is unsupported.
//! - **Seasons:** built from the series' default episode order, where
//!   specials are season `0`. Each call fetches the whole episode list, since
//!   TheTVDB pages it by series rather than by season. The unified
//!   [`UnifiedEpisode`] has no absolute number; read it from
//!   [`TvdbClient::series_episodes`].
//! - **External ids:** a title's IMDb, TMDB, MyAnimeList and Wikidata ids
//!   from its TheTVDB remote ids. Find-by-external-id is not implemented, so
//!   [`ExternalIdProvider::external_sources`] is empty.
//!
//! Discovery, recommendations, credits and watch availability are not
//! implemented; the facade reports them as
//! [`Unsupported`](crate::unified::CameoClientError::Unsupported) for `tvdb:*`
//! ids.

use async_trait::async_trait;

use super::{
    client::{TvdbClient, TvdbSearchType},
    convert::{
        episode_to_unified, external_ids, movie_to_details, search_to_movie, search_to_result,
        search_to_tv_show, season_details, series_to_details,
    },
    error::TvdbError,
};
use crate::{
    core::{error::ProviderError, pagination::Page},
    unified::{
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        media_id::MediaId,
        models::{
            UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
        },
        traits::{
            Capability, DetailProvider, ExternalIdProvider, Provider, SearchProvider,
            SeasonProvider,
        },
    },
};

const TVDB_CAPABILITIES: &[Capability] = &[
    Capability::Search,
    Capability::Details,
    Capability::Seasons,
    Capability::ExternalIds,
];

impl Provider for TvdbClient {
    fn id(&self) -> &str {
        "tvdb"
    }

    fn capabilities(&self) -> &[Capability] {
        TVDB_CAPABILITIES
    }
}

/// The numeric TheTVDB id of a `tvdb:<n>` [`MediaId`].
fn tvdb_id(id: &MediaId) -> Result<u64, ProviderError> {
    if id.provider() != "tvdb" {
        return Err(ProviderError::InvalidInput(format!(
            "expected a tvdb id, got {id}"
        )));
    }
    id.as_u64()
        .filter(|&n| n != 0)
        .ok_or_else(|| ProviderError::InvalidInput(format!("not a numeric TheTVDB id: {id}")))
}

#[async_trait]
impl SearchProvider for TvdbClient {
    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        let lang = &self.config().language;
        Ok(self
            .search(query, Some(TvdbSearchType::Movie), page)
            .await?
            .filter_map(|r| search_to_movie(&r, lang)))
    }

    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        let lang = &self.config().language;
        Ok(self
            .search(query, Some(TvdbSearchType::Series), page)
            .await?
            .filter_map(|r| search_to_tv_show(&r, lang)))
    }

    /// People are not searched: TheTVDB person details are not implemented,
    /// so a person id could not be looked up.
    async fn search_people(
        &self,
        _query: &str,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        Err(ProviderError::Unsupported)
    }

    /// Series and movies; every other kind of result is omitted.
    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        let lang = &self.config().language;
        Ok(self
            .search(query, None, page)
            .await?
            .filter_map(|r| search_to_result(&r, lang)))
    }
}

#[async_trait]
impl DetailProvider for TvdbClient {
    async fn movie_details(&self, id: &MediaId) -> Result<UnifiedMovieDetails, ProviderError> {
        let movie = self.movie_extended(tvdb_id(id)?).await?;
        Ok(movie_to_details(&movie, &self.config().language))
    }

    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError> {
        let series = self.series_extended(tvdb_id(id)?).await?;
        Ok(series_to_details(&series, &self.config().language))
    }

    async fn person_details(&self, _id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError> {
        Err(ProviderError::Unsupported)
    }
}

#[async_trait]
impl SeasonProvider for TvdbClient {
    /// The season in the default episode order; season `0` is the specials.
    async fn season_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, ProviderError> {
        let episodes = self.all_series_episodes(tvdb_id(show_id)?).await?;
        season_details(show_id.clone(), season_number, &episodes)
            .ok_or_else(|| TvdbError::NotFound.into())
    }

    async fn episode_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, ProviderError> {
        let episodes = self.all_series_episodes(tvdb_id(show_id)?).await?;
        episodes
            .iter()
            .find(|e| e.season_number == season_number && e.number == episode_number)
            .map(episode_to_unified)
            .ok_or_else(|| TvdbError::NotFound.into())
    }
}

#[async_trait]
impl ExternalIdProvider for TvdbClient {
    /// Empty: find-by-external-id is not implemented for TheTVDB.
    fn external_sources(&self) -> &[ExternalSource] {
        &[]
    }

    async fn movie_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        let movie = self.movie_extended(tvdb_id(id)?).await?;
        Ok(external_ids(&movie.remote_ids))
    }

    /// The series' remote ids, plus its own TheTVDB id.
    async fn tv_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        let series = self.series_extended(tvdb_id(id)?).await?;
        Ok(external_ids(&series.remote_ids).with_tvdb(series.id))
    }

    async fn find_by_external_id(
        &self,
        _id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, ProviderError> {
        Err(ProviderError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::media_id::NativeId;

    #[test]
    fn tvdb_id_accepts_only_numeric_tvdb_ids() {
        assert_eq!(tvdb_id(&MediaId::tvdb(81189)), Ok(81189));
        for bad in [
            MediaId::tmdb(1),
            MediaId::tvdb(0),
            MediaId::new("tvdb", NativeId::Text("series-1".into())),
        ] {
            assert!(
                matches!(tvdb_id(&bad), Err(ProviderError::InvalidInput(_))),
                "{bad}"
            );
        }
    }

    fn client() -> TvdbClient {
        TvdbClient::new(crate::providers::tvdb::TvdbConfig::new("key")).expect("valid config")
    }

    #[test]
    fn advertises_exactly_its_implemented_capabilities() {
        assert_eq!(
            client().capabilities(),
            [
                Capability::Search,
                Capability::Details,
                Capability::Seasons,
                Capability::ExternalIds,
            ]
        );
    }

    /// Reverse lookup is not implemented, which is unsupported rather than
    /// an empty match.
    #[tokio::test]
    async fn find_by_external_id_is_unsupported() {
        let err = client()
            .find_by_external_id(&crate::unified::external_ids::ExternalId::imdb("tt1"))
            .await
            .unwrap_err();
        assert_eq!(err, ProviderError::Unsupported);
    }
}
