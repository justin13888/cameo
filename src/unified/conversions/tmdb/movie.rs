//! TMDB movie type → unified model conversions.

use super::{resolve_backdrop, resolve_poster};
use crate::{
    generated::tmdb::types,
    unified::{genre::Genre, models::*},
};

macro_rules! impl_movie_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedMovie {
            fn from(m: $ty) -> Self {
                UnifiedMovie {
                    provider_id: format!("tmdb:{}", m.id),
                    title: m.title.unwrap_or_default(),
                    original_title: m.original_title,
                    overview: m.overview,
                    release_date: m.release_date,
                    poster_url: resolve_poster(&m.poster_path),
                    backdrop_url: resolve_backdrop(&m.backdrop_path),
                    genres: m
                        .genre_ids
                        .iter()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: m.popularity,
                    vote_average: m.vote_average,
                    vote_count: m.vote_count as u64,
                    original_language: m.original_language,
                    adult: m.adult,
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

impl From<types::MovieDetailsResponse> for UnifiedMovieDetails {
    fn from(m: types::MovieDetailsResponse) -> Self {
        let belongs_to_collection = m
            .belongs_to_collection
            .as_ref()
            .and_then(|v| v.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);

        UnifiedMovieDetails {
            movie: UnifiedMovie {
                provider_id: format!("tmdb:{}", m.id),
                title: m.title.unwrap_or_default(),
                original_title: m.original_title,
                overview: m.overview,
                release_date: m.release_date,
                poster_url: resolve_poster(&m.poster_path),
                backdrop_url: resolve_backdrop(&m.backdrop_path),
                genres: m
                    .genres
                    .iter()
                    .filter_map(|g| g.name.as_deref())
                    .map(Genre::from_name)
                    .collect(),
                popularity: m.popularity,
                vote_average: m.vote_average,
                vote_count: m.vote_count as u64,
                original_language: m.original_language,
                adult: m.adult,
            },
            tagline: m.tagline,
            runtime: if m.runtime > 0 {
                Some(m.runtime as u32)
            } else {
                None
            },
            budget: if m.budget > 0 {
                Some(m.budget as u64)
            } else {
                None
            },
            revenue: if m.revenue > 0 {
                Some(m.revenue as u64)
            } else {
                None
            },
            status: m.status,
            homepage: m.homepage,
            imdb_id: m.imdb_id,
            production_companies: m
                .production_companies
                .iter()
                .filter_map(|c| c.name.clone())
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
