//! Unified provider-trait implementations for [`AniListClient`].
//!
//! AniList supports search, details, discovery, recommendations, credits, and
//! external ids.
//!
//! - **Credits:** [`CreditsProvider::movie_credits`] and
//!   [`CreditsProvider::tv_show_credits`] share one query, built from the
//!   anime's characters (with their Japanese voice actors) and staff; see
//!   [`AniListClient::media_credits`].
//! - **External ids:** MyAnimeList only, via
//!   [`AniListClient::external_ids`] and [`AniListClient::find_by_mal_id`].
//! - **Recommendations:** [`RecommendationProvider::movie_recommendations`] and
//!   [`RecommendationProvider::tv_recommendations`] return AniList's community
//!   recommendations filtered to movies / series (see
//!   [`AniListClient::movie_recommendations`]). AniList has no separate
//!   "similar titles" list, so `similar_movies` and `similar_tv_shows` return
//!   [`ProviderError::Unsupported`].
//! - **Seasons:** not implemented. AniList "seasons" are *broadcast* seasons
//!   (e.g. Spring 2024, available as a discover filter via
//!   [`AniListClient::discover_anime`]); each season of a series is its own
//!   AniList media entry, so there is no TV-season/episode structure to map to
//!   [`SeasonProvider`](crate::unified::SeasonProvider).
//! - **Watch availability:** not implemented.
//!
//! The facade reports unimplemented capabilities as
//! [`Unsupported`](crate::unified::CameoClientError::Unsupported) for
//! `anilist:*` ids.

use async_trait::async_trait;

use super::client::AniListClient;
use crate::{
    core::{config::TimeWindow, error::ProviderError, pagination::Page},
    unified::{
        external_ids::{ExternalId, ExternalIds, ExternalMatch, ExternalSource},
        media_id::{MediaId, NativeId},
        models::{
            UnifiedCredits, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
        traits::{
            Capability, CreditsProvider, DetailProvider, DiscoveryProvider, ExternalIdProvider,
            Provider, RecommendationProvider, SearchProvider,
        },
    },
};

const ANILIST_CAPABILITIES: &[Capability] = &[
    Capability::Search,
    Capability::Details,
    Capability::Discovery,
    Capability::Recommendations,
    Capability::ExternalIds,
    Capability::Credits,
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

/// Namespace prefix of an AniList staff id's native part (`staff:95061`).
const STAFF_PREFIX: &str = "staff:";

/// Extract the numeric AniList *media* id from a plain media id (`anilist:1`).
///
/// Returns [`ProviderError::InvalidInput`] for an id from another provider, a
/// namespaced id such as a staff id (`anilist:staff:95061`), or a number
/// outside AniList's `i32` range.
fn anilist_media_id(id: &MediaId) -> Result<i32, ProviderError> {
    require_anilist(id)?;
    match id.native() {
        NativeId::Num(n) => to_i32(*n, id),
        NativeId::Text(_) => Err(ProviderError::InvalidInput(format!(
            "expected an AniList media id (anilist:<n>), got {id}"
        ))),
    }
}

/// Extract the numeric AniList *staff* id from a staff id
/// (`anilist:staff:95061`).
///
/// Returns [`ProviderError::InvalidInput`] for an id from another provider, a
/// plain media id (`anilist:1`), any other namespace, or a number outside
/// AniList's `i32` range.
fn anilist_staff_id(id: &MediaId) -> Result<i32, ProviderError> {
    require_anilist(id)?;
    let numeric = match id.native() {
        NativeId::Text(s) => s
            .strip_prefix(STAFF_PREFIX)
            .and_then(|n| n.parse::<u64>().ok()),
        NativeId::Num(_) => None,
    };
    let numeric = numeric.ok_or_else(|| {
        ProviderError::InvalidInput(format!(
            "expected an AniList staff id (anilist:staff:<n>), got {id}"
        ))
    })?;
    to_i32(numeric, id)
}

/// Reject an id that does not belong to AniList.
fn require_anilist(id: &MediaId) -> Result<(), ProviderError> {
    if id.provider() == "anilist" {
        Ok(())
    } else {
        Err(ProviderError::InvalidInput(format!(
            "expected an anilist id, got {id}"
        )))
    }
}

/// Narrow a parsed AniList id to the `i32` AniList's API takes.
fn to_i32(numeric: u64, id: &MediaId) -> Result<i32, ProviderError> {
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
        Ok(AniListClient::movie_details(self, anilist_media_id(id)?).await?)
    }

    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError> {
        Ok(AniListClient::tv_show_details(self, anilist_media_id(id)?).await?)
    }

    async fn person_details(&self, id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError> {
        Ok(AniListClient::person_details(self, anilist_staff_id(id)?).await?)
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
        Ok(AniListClient::trending_tv_shows(self, time_window, page).await?)
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

/// Credits are fetched per anime regardless of format, so the movie and TV
/// variants share one query.
#[async_trait]
impl CreditsProvider for AniListClient {
    async fn movie_credits(&self, id: &MediaId) -> Result<UnifiedCredits, ProviderError> {
        Ok(AniListClient::media_credits(self, anilist_media_id(id)?).await?)
    }

    async fn tv_show_credits(&self, id: &MediaId) -> Result<UnifiedCredits, ProviderError> {
        Ok(AniListClient::media_credits(self, anilist_media_id(id)?).await?)
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
        Ok(AniListClient::external_ids(self, anilist_media_id(id)?).await?)
    }

    async fn tv_external_ids(&self, id: &MediaId) -> Result<ExternalIds, ProviderError> {
        Ok(AniListClient::external_ids(self, anilist_media_id(id)?).await?)
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

#[async_trait]
impl RecommendationProvider for AniListClient {
    async fn movie_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(AniListClient::movie_recommendations(self, anilist_media_id(id)?, page).await?)
    }

    async fn tv_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(AniListClient::tv_recommendations(self, anilist_media_id(id)?, page).await?)
    }

    /// AniList has no "similar titles" list distinct from recommendations.
    async fn similar_movies(
        &self,
        _id: &MediaId,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Err(ProviderError::Unsupported)
    }

    /// AniList has no "similar titles" list distinct from recommendations.
    async fn similar_tv_shows(
        &self,
        _id: &MediaId,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Err(ProviderError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::anilist::config::AniListConfig;

    fn is_invalid_input<T: std::fmt::Debug>(result: Result<T, ProviderError>) -> bool {
        matches!(result, Err(ProviderError::InvalidInput(_)))
    }

    /// A client whose base URL nothing listens on: every test below must fail
    /// before any request is sent.
    fn offline_client() -> AniListClient {
        let config = AniListConfig::new_with_base_url("http://127.0.0.1:9/graphql");
        AniListClient::new(config).expect("valid config")
    }

    #[test]
    fn media_id_accepts_plain_media_ids() {
        assert_eq!(anilist_media_id(&MediaId::anilist(1)).ok(), Some(1));
    }

    #[test]
    fn media_id_rejects_staff_ids() {
        assert!(is_invalid_input(anilist_media_id(&MediaId::anilist_staff(
            95061
        ))));
    }

    #[test]
    fn media_id_rejects_other_providers_and_out_of_range_ids() {
        assert!(is_invalid_input(anilist_media_id(&MediaId::tmdb(1))));
        let too_big = u64::try_from(i32::MAX).expect("positive") + 1;
        assert!(is_invalid_input(anilist_media_id(&MediaId::anilist(
            too_big
        ))));
    }

    #[test]
    fn staff_id_accepts_staff_ids() {
        assert_eq!(
            anilist_staff_id(&MediaId::anilist_staff(95061)).ok(),
            Some(95061)
        );
    }

    #[test]
    fn staff_id_rejects_media_ids() {
        assert!(is_invalid_input(anilist_staff_id(&MediaId::anilist(1))));
    }

    #[test]
    fn staff_id_rejects_other_namespaces_and_providers() {
        let character = MediaId::new("anilist", NativeId::Text("character:5".to_owned()));
        assert!(is_invalid_input(anilist_staff_id(&character)));
        let bare = MediaId::new("anilist", NativeId::Text("staff:".to_owned()));
        assert!(is_invalid_input(anilist_staff_id(&bare)));
        let tmdb_staff = MediaId::new("tmdb", NativeId::Text("staff:1".to_owned()));
        assert!(is_invalid_input(anilist_staff_id(&tmdb_staff)));
    }

    #[tokio::test]
    async fn media_details_reject_a_staff_id() {
        let client = offline_client();
        let staff = MediaId::anilist_staff(95061);
        assert!(is_invalid_input(
            DetailProvider::movie_details(&client, &staff).await
        ));
        assert!(is_invalid_input(
            DetailProvider::tv_show_details(&client, &staff).await
        ));
    }

    #[tokio::test]
    async fn person_details_reject_a_media_id() {
        let client = offline_client();
        assert!(is_invalid_input(
            DetailProvider::person_details(&client, &MediaId::anilist(1)).await
        ));
    }
}
