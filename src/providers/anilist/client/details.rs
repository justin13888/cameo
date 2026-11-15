//! Single-item lookups: media and staff details, credits, and external ids.

use serde_json::json;

use super::AniListClient;
use crate::{
    providers::anilist::{
        convert::{
            Kind, anilist_media_detail_to_movie_details, anilist_media_detail_to_tv_details,
            classify, credits::anilist_credits_to_unified, staff_detail_to_person_details,
        },
        error::AniListError,
        query,
        response::{
            MediaCreditsResponse, MediaDetailResponse, MediaIdsResponse, StaffDetailResponse,
        },
    },
    unified::{
        conversions::native_id,
        external_ids::{ExternalIds, ExternalMatch},
        media_id::MediaId,
        models::{UnifiedCredits, UnifiedMovieDetails, UnifiedPersonDetails, UnifiedTvShowDetails},
    },
};

/// Characters and staff fetched per [`AniListClient::media_credits`] request.
///
/// Below AniList's 50-item maximum because each character also nests its
/// voice actors, which counts against AniList's per-query complexity limit.
const CREDITS_PER_PAGE: u32 = 25;

/// The most requests [`AniListClient::media_credits`] makes for one anime
/// (at [`CREDITS_PER_PAGE`], up to 500 characters and 500 staff).
const CREDITS_MAX_PAGES: u32 = 20;

impl AniListClient {
    /// Get full details for an anime movie by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList id.
    pub async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (movie_details)");
        let mut media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut media.id, id);
        anilist_media_detail_to_movie_details(media).ok_or(AniListError::NotFound)
    }

    /// Get full details for an anime TV series by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList id.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_DETAILS (tv_show_details)");
        let mut media = self
            .graphql(query::MEDIA_DETAILS, vars, |r: MediaDetailResponse| r.media)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut media.id, id);
        anilist_media_detail_to_tv_details(media).ok_or(AniListError::NotFound)
    }

    /// Get full details for a staff member by AniList ID.
    ///
    /// The result carries AniList's reported id, or `id` when the reported
    /// one is not a valid AniList staff id.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql STAFF_DETAILS (person_details)");
        let mut staff = self
            .graphql(query::STAFF_DETAILS, vars, |r: StaffDetailResponse| r.staff)
            .await?
            .ok_or(AniListError::NotFound)?;
        details_id(&mut staff.id, id);
        staff_detail_to_person_details(staff).ok_or(AniListError::NotFound)
    }

    /// Get the cast and crew of an anime by AniList ID.
    ///
    /// The cast is built from `Media.characters`: one credit per Japanese
    /// voice actor of each character, ordered main characters first, with the
    /// character's position as its billing `order`. The crew is built from
    /// `Media.staff`, with each staff edge's role as the `job`. Person ids are
    /// staff ids (`anilist:staff:<id>`), usable with
    /// [`person_details`](Self::person_details).
    ///
    /// Both connections are paged 25 entries at a time and walked until
    /// AniList reports no next page, up to 20 requests (500 characters and
    /// 500 staff); past that cap the remaining credits are dropped and a
    /// warning is logged. Movies and series share this query.
    ///
    /// # Errors
    ///
    /// [`AniListError::NotFound`] when no anime has this id (a zero or
    /// negative id never names one, so it fails without a request).
    pub async fn media_credits(&self, id: i32) -> Result<UnifiedCredits, AniListError> {
        let title = MediaId::anilist(native_id(i64::from(id)).ok_or(AniListError::NotFound)?);
        let mut characters = Vec::new();
        let mut staff = Vec::new();
        let mut more_characters = true;
        let mut more_staff = true;
        let mut page: u32 = 1;
        while more_characters || more_staff {
            if page > CREDITS_MAX_PAGES {
                tracing::warn!(
                    id,
                    max_pages = CREDITS_MAX_PAGES,
                    "anilist: credits truncated at the page cap"
                );
                break;
            }
            let vars = json!({
                "id": id,
                "characterPage": page,
                "staffPage": page,
                "perPage": CREDITS_PER_PAGE,
            });
            tracing::debug!(id, page, "anilist: graphql MEDIA_CREDITS");
            let media = self
                .graphql(query::MEDIA_CREDITS, vars, |r: MediaCreditsResponse| {
                    r.media
                })
                .await?
                .ok_or(AniListError::NotFound)?;
            if more_characters {
                let conn = media.characters;
                more_characters = conn
                    .as_ref()
                    .and_then(|c| c.page_info.as_ref())
                    .is_some_and(|p| p.has_next_page);
                characters.extend(conn.into_iter().flat_map(|c| c.edges).flatten());
            }
            if more_staff {
                let conn = media.staff;
                more_staff = conn
                    .as_ref()
                    .and_then(|c| c.page_info.as_ref())
                    .is_some_and(|p| p.has_next_page);
                staff.extend(conn.into_iter().flat_map(|c| c.edges).flatten());
            }
            page += 1;
        }
        Ok(anilist_credits_to_unified(title, characters, staff))
    }

    /// Get the external ids of an anime by AniList ID: its MyAnimeList id
    /// (when AniList knows one) and `anilist: Some(id)`.
    ///
    /// AniList tracks no IMDb, TheTVDB, Wikidata, or TMDB ids, so those stay
    /// `None`. Uses a minimal `{ id idMal }` query rather than the full detail
    /// selection.
    ///
    /// # Errors
    ///
    /// [`AniListError::NotFound`] when no anime has this id.
    pub async fn external_ids(&self, id: i32) -> Result<ExternalIds, AniListError> {
        let vars = json!({ "id": id });
        tracing::debug!(id, "anilist: graphql MEDIA_EXTERNAL_IDS");
        let media = self
            .graphql(query::MEDIA_EXTERNAL_IDS, vars, |r: MediaIdsResponse| {
                r.media
            })
            .await?
            .ok_or(AniListError::NotFound)?;
        let mut ids = ExternalIds::new();
        ids.anilist = native_id(i64::from(media.id));
        ids.mal = media.id_mal.map(i64::from).and_then(native_id);
        Ok(ids)
    }

    /// Find the anime with the given MyAnimeList ID.
    ///
    /// The match uses the same classification as
    /// [`search_multi`](Self::search_multi): a
    /// [`MatchKind::Movie`](crate::unified::MatchKind::Movie) for the movie
    /// formats (`MOVIE`) and a
    /// [`MatchKind::TvShow`](crate::unified::MatchKind::TvShow) for the TV
    /// formats (`TV`, `TV_SHORT`, `ONA`, `OVA`, `SPECIAL`).
    ///
    /// Returns `Ok(None)` when AniList has no anime with that MAL id, and when
    /// the anime's format is absent or in neither list (e.g. `MUSIC`).
    pub async fn find_by_mal_id(&self, mal_id: i32) -> Result<Option<ExternalMatch>, AniListError> {
        let vars = json!({ "idMal": mal_id });
        tracing::debug!(mal_id, "anilist: graphql MEDIA_BY_MAL_ID");
        let media = match self
            .graphql(query::MEDIA_BY_MAL_ID, vars, |r: MediaIdsResponse| r.media)
            .await
        {
            Ok(media) => media,
            Err(AniListError::NotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(media.and_then(|media| {
            let id = MediaId::anilist(native_id(i64::from(media.id))?);
            Some(match media.format.as_deref().and_then(classify)? {
                Kind::Movie => ExternalMatch::movie(id),
                Kind::Tv => ExternalMatch::tv_show(id),
            })
        }))
    }
}

/// The id a details response is converted under: the id AniList reported
/// when it is valid, otherwise (zero or negative) the requested id.
fn details_id(reported: &mut i32, requested: i32) {
    if native_id(i64::from(*reported)).is_none() {
        *reported = requested;
    }
}

#[cfg(test)]
mod tests {
    use super::details_id;

    /// An invalid id in a details response gives way to the requested id.
    #[test]
    fn details_id_replaces_only_invalid_ids() {
        for reported in [0, -3] {
            let mut id = reported;
            details_id(&mut id, 5);
            assert_eq!(id, 5, "reported {reported}");
        }
        let mut id = 9;
        details_id(&mut id, 5);
        assert_eq!(id, 9);
    }
}
