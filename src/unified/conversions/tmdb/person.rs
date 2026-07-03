//! TMDB person and multi-search type → unified model conversions.

use super::{resolve_backdrop, resolve_date, resolve_poster, resolve_profile};
use crate::{
    generated::tmdb::types,
    unified::{genre::Genre, media_id::MediaId, models::*},
};

impl From<types::SearchPersonResponseResultsItem> for UnifiedPerson {
    fn from(p: types::SearchPersonResponseResultsItem) -> Self {
        UnifiedPerson {
            provider_id: MediaId::tmdb(p.id as u64),
            name: p.name.unwrap_or_default(),
            known_for_department: p.known_for_department,
            profile_url: resolve_profile(&p.profile_path),
            popularity: Some(p.popularity),
            gender: Gender::from_tmdb(p.gender),
            adult: Some(p.adult),
        }
    }
}

impl From<types::PersonDetailsResponse> for UnifiedPersonDetails {
    fn from(p: types::PersonDetailsResponse) -> Self {
        UnifiedPersonDetails {
            person: UnifiedPerson {
                provider_id: MediaId::tmdb(p.id as u64),
                name: p.name.unwrap_or_default(),
                known_for_department: p.known_for_department,
                profile_url: resolve_profile(&p.profile_path),
                popularity: Some(p.popularity),
                gender: Gender::from_tmdb(p.gender),
                adult: Some(p.adult),
            },
            biography: p.biography,
            birthday: resolve_date(&p.birthday),
            deathday: resolve_date(
                &p.deathday
                    .as_ref()
                    .and_then(|v| v.as_str().map(String::from)),
            ),
            place_of_birth: p.place_of_birth,
            imdb_id: p.imdb_id,
            homepage: p.homepage.and_then(|v| v.as_str().map(String::from)),
            also_known_as: p.also_known_as,
        }
    }
}

/// Convert a TMDB multi-search item into a [`UnifiedSearchResult`].
///
/// Shared unknown-media policy: an item whose category is absent or not one of
/// `movie` / `tv` / `person` is **omitted** (`None`) rather than coerced to an
/// arbitrary type. AniList follows the same policy for its `format` field.
pub(crate) fn multi_search_result(
    item: types::SearchMultiResponseResultsItem,
) -> Option<UnifiedSearchResult> {
    match item.media_type.as_deref() {
        Some("movie") => Some(UnifiedSearchResult::Movie(UnifiedMovie {
            provider_id: MediaId::tmdb(item.id as u64),
            title: item.title.or(item.name).unwrap_or_default(),
            original_title: item.original_title,
            overview: item.overview,
            release_date: resolve_date(&item.release_date),
            poster_url: resolve_poster(&item.poster_path),
            backdrop_url: resolve_backdrop(&item.backdrop_path),
            genres: item
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: Some(item.popularity),
            vote_average: Some(item.vote_average),
            vote_count: Some(item.vote_count.max(0) as u64),
            original_language: item.original_language,
            adult: Some(item.adult),
        })),
        Some("tv") => Some(UnifiedSearchResult::TvShow(UnifiedTvShow {
            provider_id: MediaId::tmdb(item.id as u64),
            name: item.name.or(item.title).unwrap_or_default(),
            original_name: item.original_name,
            overview: item.overview,
            // TMDB multi-search reports a single `release_date` for both movies and shows.
            first_air_date: resolve_date(&item.release_date),
            poster_url: resolve_poster(&item.poster_path),
            backdrop_url: resolve_backdrop(&item.backdrop_path),
            genres: item
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: Some(item.popularity),
            vote_average: Some(item.vote_average),
            vote_count: Some(item.vote_count.max(0) as u64),
            original_language: item.original_language,
            origin_country: Vec::new(),
            adult: Some(item.adult),
        })),
        Some("person") => Some(UnifiedSearchResult::Person(UnifiedPerson {
            provider_id: MediaId::tmdb(item.id as u64),
            name: item.name.or(item.title).unwrap_or_default(),
            known_for_department: None,
            profile_url: resolve_poster(&item.poster_path), // multi uses poster_path
            popularity: Some(item.popularity),
            gender: None,
            adult: Some(item.adult),
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multi_item(id: i64, media_type: Option<&str>) -> types::SearchMultiResponseResultsItem {
        types::SearchMultiResponseResultsItem {
            id,
            media_type: media_type.map(String::from),
            title: Some("Title".to_string()),
            name: Some("Name".to_string()),
            original_title: None,
            original_name: None,
            overview: None,
            release_date: Some("1999-10-15".to_string()),
            poster_path: None,
            backdrop_path: None,
            genre_ids: vec![18],
            popularity: 1.0,
            vote_average: 8.0,
            vote_count: 100,
            original_language: Some("en".to_string()),
            adult: false,
            video: false,
        }
    }

    #[test]
    fn multi_movie_maps_to_movie() {
        match multi_search_result(multi_item(550, Some("movie"))) {
            Some(UnifiedSearchResult::Movie(m)) => {
                assert_eq!(m.title, "Title");
                assert_eq!(m.genres, vec![Genre::Drama]);
            }
            other => panic!("expected Movie, got {other:?}"),
        }
    }

    #[test]
    fn multi_tv_maps_to_tv() {
        assert!(matches!(
            multi_search_result(multi_item(1396, Some("tv"))),
            Some(UnifiedSearchResult::TvShow(_))
        ));
    }

    #[test]
    fn multi_person_maps_to_person() {
        assert!(matches!(
            multi_search_result(multi_item(287, Some("person"))),
            Some(UnifiedSearchResult::Person(_))
        ));
    }

    #[test]
    fn multi_unknown_media_type_is_dropped() {
        // Shared unknown-media policy: absent or unrecognized category -> omitted.
        assert!(multi_search_result(multi_item(1, Some("collection"))).is_none());
        assert!(multi_search_result(multi_item(2, None)).is_none());
    }

    #[test]
    fn search_person_to_unified() {
        let raw = types::SearchPersonResponseResultsItem {
            id: 287,
            name: Some("Brad Pitt".to_string()),
            original_name: None,
            known_for_department: Some("Acting".to_string()),
            profile_path: Some("/kU3B75TyRiCgE270EyZnHjfivoq.jpg".to_string()),
            popularity: 25.3,
            adult: false,
            gender: 2,
            known_for: vec![],
        };
        let unified: UnifiedPerson = raw.into();
        assert_eq!(unified.provider_id, "tmdb:287");
        assert_eq!(unified.name, "Brad Pitt");
        assert_eq!(unified.known_for_department.as_deref(), Some("Acting"));
        assert_eq!(unified.gender, Some(Gender::Male));
        assert_eq!(unified.adult, Some(false));
        assert!(
            unified
                .profile_url
                .unwrap()
                .starts_with("https://image.tmdb.org/t/p/")
        );
    }
}
