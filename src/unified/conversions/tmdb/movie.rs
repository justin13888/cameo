//! TMDB movie type → unified model conversions.

use super::{resolve_backdrop, resolve_date, resolve_logo, resolve_poster};
use crate::{
    generated::tmdb::types,
    unified::{genre::Genre, media_id::MediaId, models::*},
};

macro_rules! impl_movie_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedMovie {
            fn from(m: $ty) -> Self {
                UnifiedMovie {
                    provider_id: MediaId::tmdb(m.id as u64),
                    title: m.title.unwrap_or_default(),
                    original_title: m.original_title,
                    overview: m.overview,
                    release_date: resolve_date(&m.release_date),
                    poster_url: resolve_poster(&m.poster_path),
                    backdrop_url: resolve_backdrop(&m.backdrop_path),
                    genres: m
                        .genre_ids
                        .iter()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: Some(m.popularity),
                    vote_average: Some(m.vote_average),
                    vote_count: Some(m.vote_count.max(0) as u64),
                    original_language: m.original_language,
                    adult: Some(m.adult),
                }
            }
        }
    };
}

impl_movie_from!(types::SearchMovieResponseResultsItem);
impl_movie_from!(types::TrendingMoviesResponseResultsItem);
impl_movie_from!(types::MoviePopularListResponseResultsItem);
impl_movie_from!(types::MovieTopRatedListResponseResultsItem);
impl_movie_from!(types::DiscoverMovieResponseResultsItem);
impl_movie_from!(types::MovieSimilarResponseResultsItem);

impl From<types::MovieDetailsResponseProductionCompaniesItem> for Company {
    fn from(c: types::MovieDetailsResponseProductionCompaniesItem) -> Self {
        Company {
            id: u64::try_from(c.id).ok(),
            name: c.name.unwrap_or_default(),
            logo_url: resolve_logo(&c.logo_path),
            origin_country: c.origin_country,
        }
    }
}

impl From<types::MovieDetailsResponse> for UnifiedMovieDetails {
    fn from(m: types::MovieDetailsResponse) -> Self {
        let belongs_to_collection = m.belongs_to_collection.as_ref().map(parse_collection);

        UnifiedMovieDetails {
            movie: UnifiedMovie {
                provider_id: MediaId::tmdb(m.id as u64),
                title: m.title.unwrap_or_default(),
                original_title: m.original_title,
                overview: m.overview,
                release_date: resolve_date(&m.release_date),
                poster_url: resolve_poster(&m.poster_path),
                backdrop_url: resolve_backdrop(&m.backdrop_path),
                genres: m.genres.iter().map(|g| Genre::from_tmdb_id(g.id)).collect(),
                popularity: Some(m.popularity),
                vote_average: Some(m.vote_average),
                vote_count: Some(m.vote_count.max(0) as u64),
                original_language: m.original_language,
                adult: Some(m.adult),
            },
            tagline: m.tagline,
            runtime: if m.runtime > 0 {
                Some(m.runtime.max(0) as u32)
            } else {
                None
            },
            budget: if m.budget > 0 {
                Some(m.budget.max(0) as u64)
            } else {
                None
            },
            revenue: if m.revenue > 0 {
                Some(m.revenue.max(0) as u64)
            } else {
                None
            },
            status: m.status,
            homepage: m.homepage,
            imdb_id: m.imdb_id,
            production_companies: m
                .production_companies
                .into_iter()
                .map(Company::from)
                .collect(),
            production_countries: m
                .production_countries
                .iter()
                .filter_map(|c| c.name.clone())
                .collect(),
            spoken_languages: m
                .spoken_languages
                .iter()
                .filter_map(|l| l.english_name.clone())
                .collect(),
            video: m.video,
            belongs_to_collection,
        }
    }
}

/// Parse TMDB's loosely-typed `belongs_to_collection` object into a [`Collection`].
fn parse_collection(v: &serde_json::Value) -> Collection {
    Collection {
        id: v.get("id").and_then(serde_json::Value::as_u64),
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        poster_url: resolve_poster(
            &v.get("poster_path")
                .and_then(|x| x.as_str())
                .map(String::from),
        ),
        backdrop_url: resolve_backdrop(
            &v.get("backdrop_path")
                .and_then(|x| x.as_str())
                .map(String::from),
        ),
    }
}
