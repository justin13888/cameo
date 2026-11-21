//! TMDB person and multi-search type → unified model conversions.

use super::{
    parse_date, resolve_backdrop, resolve_date, resolve_poster, resolve_profile, tmdb_media_id,
};
use crate::{
    generated::tmdb::types,
    unified::{conversions::native_id, genre::Genre, media_id::MediaId, models::*},
};

impl From<types::SearchPersonResponseResultsItem> for UnifiedPerson {
    fn from(p: types::SearchPersonResponseResultsItem) -> Self {
        UnifiedPerson {
            provider_id: tmdb_media_id(p.id),
            name: p.name.into_option().unwrap_or_default(),
            known_for_department: p.known_for_department.into_option(),
            profile_url: resolve_profile(&p.profile_path),
            popularity: p.popularity.into_option(),
            gender: p.gender.into_option().and_then(Gender::from_tmdb),
            adult: p.adult.into_option(),
        }
    }
}

impl From<types::PersonDetailsResponse> for UnifiedPersonDetails {
    fn from(p: types::PersonDetailsResponse) -> Self {
        UnifiedPersonDetails {
            person: UnifiedPerson {
                provider_id: tmdb_media_id(p.id),
                name: p.name.into_option().unwrap_or_default(),
                known_for_department: p.known_for_department.into_option(),
                profile_url: resolve_profile(&p.profile_path),
                popularity: p.popularity.into_option(),
                gender: p.gender.into_option().and_then(Gender::from_tmdb),
                adult: p.adult.into_option(),
            },
            biography: p.biography.into_option(),
            birthday: resolve_date(&p.birthday),
            deathday: parse_date(p.deathday.as_ref().and_then(serde_json::Value::as_str)),
            place_of_birth: p.place_of_birth.into_option(),
            imdb_id: p.imdb_id.into_option(),
            homepage: p.homepage.and_then(|v| v.as_str().map(String::from)),
            also_known_as: p.also_known_as.into_option().unwrap_or_default(),
        }
    }
}

/// Convert a TMDB multi-search item into a [`UnifiedSearchResult`].
///
/// Shared unknown-media policy: an item whose category is absent or not one of
/// `movie` / `tv` / `person` is **omitted** (`None`) rather than coerced to an
/// arbitrary type. AniList follows the same policy for its `format` field.
/// An item without a valid TMDB id (missing, zero, or negative) is omitted
/// too: it cannot be fetched or cached.
pub(crate) fn multi_search_result(
    item: types::SearchMultiResponseResultsItem,
) -> Option<UnifiedSearchResult> {
    let id = item.id.into_option().and_then(native_id)?;
    match item.media_type.as_option().map(String::as_str) {
        Some("movie") => Some(UnifiedSearchResult::Movie(UnifiedMovie {
            provider_id: MediaId::tmdb(id),
            title: item
                .title
                .into_option()
                .or(item.name.into_option())
                .unwrap_or_default(),
            original_title: item.original_title.into_option(),
            alternative_titles: Vec::new(),
            overview: item.overview.into_option(),
            release_date: resolve_date(&item.release_date),
            poster_url: resolve_poster(&item.poster_path),
            backdrop_url: resolve_backdrop(&item.backdrop_path),
            genres: item
                .genre_ids
                .as_option()
                .into_iter()
                .flatten()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: item.popularity.into_option(),
            vote_average: item.vote_average.into_option(),
            vote_count: item.vote_count.into_option().map(|c| c.max(0) as u64),
            original_language: item.original_language.into_option(),
            adult: item.adult.into_option(),
            // TMDB multi-search returns neither runtime nor status.
            runtime: None,
            status: None,
        })),
        Some("tv") => Some(UnifiedSearchResult::TvShow(UnifiedTvShow {
            provider_id: MediaId::tmdb(id),
            name: item
                .name
                .into_option()
                .or(item.title.into_option())
                .unwrap_or_default(),
            original_name: item.original_name.into_option(),
            alternative_titles: Vec::new(),
            overview: item.overview.into_option(),
            // TMDB multi-search reports a single `release_date` for both movies and shows.
            first_air_date: resolve_date(&item.release_date),
            poster_url: resolve_poster(&item.poster_path),
            backdrop_url: resolve_backdrop(&item.backdrop_path),
            genres: item
                .genre_ids
                .as_option()
                .into_iter()
                .flatten()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: item.popularity.into_option(),
            vote_average: item.vote_average.into_option(),
            vote_count: item.vote_count.into_option().map(|c| c.max(0) as u64),
            original_language: item.original_language.into_option(),
            origin_country: Vec::new(),
            adult: item.adult.into_option(),
            // TMDB multi-search returns no episode data or status.
            number_of_episodes: None,
            episode_run_time: None,
            status: None,
        })),
        Some("person") => Some(UnifiedSearchResult::Person(UnifiedPerson {
            provider_id: MediaId::tmdb(id),
            name: item
                .name
                .into_option()
                .or(item.title.into_option())
                .unwrap_or_default(),
            known_for_department: None,
            profile_url: resolve_poster(&item.poster_path), // multi uses poster_path
            popularity: item.popularity.into_option(),
            gender: None,
            adult: item.adult.into_option(),
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::tmdb::Nullable;

    fn multi_item(id: i64, media_type: Option<&str>) -> types::SearchMultiResponseResultsItem {
        types::SearchMultiResponseResultsItem {
            id: Nullable::Value(id),
            media_type: media_type.map_or(Nullable::Absent, |t| Nullable::Value(t.to_string())),
            title: Nullable::Value("Title".to_string()),
            name: Nullable::Value("Name".to_string()),
            original_title: Nullable::Absent,
            original_name: Nullable::Absent,
            overview: Nullable::Absent,
            release_date: Nullable::Value("1999-10-15".to_string()),
            poster_path: Nullable::Absent,
            backdrop_path: Nullable::Absent,
            genre_ids: Nullable::Value(vec![18]),
            popularity: Nullable::Value(1.0),
            vote_average: Nullable::Value(8.0),
            vote_count: Nullable::Value(100),
            original_language: Nullable::Value("en".to_string()),
            adult: Nullable::Value(false),
            video: Nullable::Value(false),
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
    fn multi_item_without_id_is_dropped() {
        let mut item = multi_item(550, Some("movie"));
        item.id = Nullable::Absent;
        assert!(multi_search_result(item).is_none());
    }

    /// A zero or negative id is not a TMDB id: the item is dropped, never
    /// wrapped into a huge `u64`.
    #[test]
    fn multi_item_with_invalid_id_is_dropped() {
        for id in [0, -1, i64::MIN] {
            for media_type in ["movie", "tv", "person"] {
                assert!(
                    multi_search_result(multi_item(id, Some(media_type))).is_none(),
                    "{media_type} {id}"
                );
            }
        }
    }

    #[test]
    fn multi_item_unknown_adult_and_counts_are_none() {
        let mut item = multi_item(550, Some("movie"));
        item.adult = Nullable::Absent;
        item.vote_count = Nullable::Absent;
        match multi_search_result(item) {
            Some(UnifiedSearchResult::Movie(m)) => {
                assert_eq!(m.adult, None);
                assert_eq!(m.vote_count, None);
            }
            other => panic!("expected Movie, got {other:?}"),
        }
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
            id: Nullable::Value(287),
            name: Nullable::Value("Brad Pitt".to_string()),
            original_name: Nullable::Absent,
            known_for_department: Nullable::Value("Acting".to_string()),
            profile_path: Nullable::Value("/kU3B75TyRiCgE270EyZnHjfivoq.jpg".to_string()),
            popularity: Nullable::Value(25.3),
            adult: Nullable::Value(false),
            gender: Nullable::Value(2),
            known_for: Nullable::Value(vec![]),
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
