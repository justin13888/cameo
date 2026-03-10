//! TMDB person and multi-search type → unified model conversions.

use super::{resolve_backdrop, resolve_poster, resolve_profile};
use crate::{generated::tmdb::types, unified::models::*};

impl From<types::SearchPersonResponseResultsItem> for UnifiedPerson {
    fn from(p: types::SearchPersonResponseResultsItem) -> Self {
        UnifiedPerson {
            provider_id: format!("tmdb:{}", p.id),
            name: p.name.unwrap_or_default(),
            known_for_department: p.known_for_department,
            profile_url: resolve_profile(&p.profile_path),
            popularity: p.popularity,
            gender: Some(p.gender as i32),
            adult: p.adult,
        }
    }
}

impl From<types::PersonDetailsResponse> for UnifiedPersonDetails {
    fn from(p: types::PersonDetailsResponse) -> Self {
        UnifiedPersonDetails {
            person: UnifiedPerson {
                provider_id: format!("tmdb:{}", p.id),
                name: p.name.unwrap_or_default(),
                known_for_department: p.known_for_department,
                profile_url: resolve_profile(&p.profile_path),
                popularity: p.popularity,
                gender: Some(p.gender as i32),
                adult: p.adult,
            },
            biography: p.biography,
            birthday: p.birthday,
            deathday: p.deathday.and_then(|v| v.as_str().map(String::from)),
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
                provider_id: format!("tmdb:{}", item.id),
                title: item.title.or(item.name).unwrap_or_default(),
                original_title: item.original_title,
                overview: item.overview,
                release_date: item.release_date,
                poster_url: resolve_poster(&item.poster_path),
                backdrop_url: resolve_backdrop(&item.backdrop_path),
                genres: item
                    .genre_ids
                    .iter()
                    .map(|&id| crate::unified::genre::Genre::from_tmdb_id(id))
                    .collect(),
                popularity: item.popularity,
                vote_average: item.vote_average,
                vote_count: item.vote_count.max(0) as u64,
                original_language: item.original_language,
                adult: item.adult,
            }),
            Some("tv") => UnifiedSearchResult::TvShow(UnifiedTvShow {
                provider_id: format!("tmdb:{}", item.id),
                name: item.name.or(item.title).unwrap_or_default(),
                original_name: item.original_name,
                overview: item.overview,
                first_air_date: item.release_date, // multi-search uses release_date for both
                poster_url: resolve_poster(&item.poster_path),
                backdrop_url: resolve_backdrop(&item.backdrop_path),
                genres: item
                    .genre_ids
                    .iter()
                    .map(|&id| crate::unified::genre::Genre::from_tmdb_id(id))
                    .collect(),
                popularity: item.popularity,
                vote_average: item.vote_average,
                vote_count: item.vote_count.max(0) as u64,
                original_language: item.original_language,
                origin_country: Vec::new(),
                adult: item.adult,
            }),
            other => {
                if other != Some("person") {
                    tracing::warn!(media_type = ?other, "unknown media_type in multi-search result, defaulting to Person");
                }
                UnifiedSearchResult::Person(UnifiedPerson {
                    provider_id: format!("tmdb:{}", item.id),
                    name: item.name.or(item.title).unwrap_or_default(),
                    known_for_department: None,
                    profile_url: resolve_poster(&item.poster_path), // multi uses poster_path
                    popularity: item.popularity,
                    gender: None,
                    adult: item.adult,
                })
            }
        }
    }
}
