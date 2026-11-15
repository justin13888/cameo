//! Title and staff search.

use serde_json::json;

use super::AniListClient;
use crate::{
    core::pagination::Page,
    providers::anilist::{
        convert::{
            MOVIE_FORMATS, TV_FORMATS, anilist_media_to_movie, anilist_media_to_search_result,
            anilist_media_to_tv, media_page, media_page_root, page_from, staff_to_person,
        },
        error::AniListError,
        query,
        response::StaffPageResponse,
    },
    unified::models::{UnifiedMovie, UnifiedPerson, UnifiedSearchResult, UnifiedTvShow},
};

impl AniListClient {
    /// Search for anime movies by title.
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, AniListError> {
        let (page_num, mut vars) = self.list_vars(page, MOVIE_FORMATS);
        vars["query"] = json!(query);
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_movies)"
        );
        let page = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_movie)
    }

    /// Search for anime series (TV, OVA, ONA, Special) by title.
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, AniListError> {
        let (page_num, mut vars) = self.list_vars(page, TV_FORMATS);
        vars["query"] = json!(query);
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_tv_shows)"
        );
        let page = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_tv)
    }

    /// Search for staff (voice actors, directors, animators, etc.) by name.
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, AniListError> {
        let (page_num, per_page) = self.page_vars(page);
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(query, page = page_num, "anilist: graphql SEARCH_STAFF");
        let page_data = self
            .graphql(query::SEARCH_STAFF, vars, |r: StaffPageResponse| r.page)
            .await?
            .ok_or(AniListError::NoData)?;
        let results = page_data
            .staff
            .into_iter()
            .filter_map(staff_to_person)
            .collect();
        page_from(&page_data.page_info, results)
    }

    /// Search across all anime formats. Returns movies and TV shows mixed.
    ///
    /// Each result is classified the way the movie and TV list queries filter:
    /// `MOVIE` is a movie, `TV`, `TV_SHORT`, `ONA`, `OVA`, and `SPECIAL` are TV
    /// shows, and anything else (e.g. `MUSIC`, or no format) is omitted, so a
    /// page may hold fewer items than the configured page size while the
    /// pagination metadata keeps describing AniList's unfiltered results.
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, AniListError> {
        let (page_num, per_page) = self.page_vars(page);
        // Omit formatIn entirely (do not pass null) so AniList searches all formats.
        let vars = json!({
            "query": query,
            "page": page_num,
            "perPage": per_page,
        });
        tracing::debug!(
            query,
            page = page_num,
            "anilist: graphql SEARCH_ANIME (search_multi)"
        );
        let page = self
            .graphql(query::SEARCH_ANIME, vars, media_page_root)
            .await?;
        media_page(page, anilist_media_to_search_result)
    }
}
