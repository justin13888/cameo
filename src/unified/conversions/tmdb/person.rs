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

impl From<types::SearchMultiResponseResultsItem> for UnifiedSearchResult {
    fn from(item: types::SearchMultiResponseResultsItem) -> Self {
        match item.media_type.as_deref() {
            Some("movie") => UnifiedSearchResult::Movie(UnifiedMovie {
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
            }),
            Some("tv") => UnifiedSearchResult::TvShow(UnifiedTvShow {
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
            }),
            _ => UnifiedSearchResult::Person(UnifiedPerson {
                provider_id: MediaId::tmdb(item.id as u64),
                name: item.name.or(item.title).unwrap_or_default(),
                known_for_department: None,
                profile_url: resolve_poster(&item.poster_path), // multi uses poster_path
                popularity: Some(item.popularity),
                gender: None,
                adult: Some(item.adult),
            }),
        }
    }
}
