//! Conversions from TMDB generated types to unified models.
//!
//! Since the TMDB API returns genre IDs (not names) in list endpoints,
//! these conversions leave `genres` empty for list results. The unified
//! layer resolves genre names via a cached genre list when needed.

use crate::generated::tmdb::types;
use crate::providers::tmdb::image_url::{BackdropSize, ImageUrl, PosterSize, ProfileSize};
use crate::unified::models::*;

fn resolve_poster(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::poster(p, PosterSize::W500))
}

fn resolve_backdrop(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::backdrop(p, BackdropSize::W780))
}

fn resolve_profile(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::profile(p, ProfileSize::H632))
}

// ── Movie conversions ──

/// Macro to convert movie-like TMDB list result types to UnifiedMovie.
/// All these types share the same field structure.
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
                    genres: Vec::new(), // genre IDs only; resolved by caller
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

impl From<types::MovieDetailsResponse> for UnifiedMovieDetails {
    fn from(m: types::MovieDetailsResponse) -> Self {
        UnifiedMovieDetails {
            movie: UnifiedMovie {
                provider_id: format!("tmdb:{}", m.id),
                title: m.title.unwrap_or_default(),
                original_title: m.original_title,
                overview: m.overview,
                release_date: m.release_date,
                poster_url: resolve_poster(&m.poster_path),
                backdrop_url: resolve_backdrop(&m.backdrop_path),
                genres: m.genres.iter().filter_map(|g| g.name.clone()).collect(),
                popularity: m.popularity,
                vote_average: m.vote_average,
                vote_count: m.vote_count as u64,
                original_language: m.original_language,
                adult: m.adult,
            },
            tagline: m.tagline,
            runtime: if m.runtime > 0 { Some(m.runtime as u32) } else { None },
            budget: if m.budget > 0 { Some(m.budget as u64) } else { None },
            revenue: if m.revenue > 0 { Some(m.revenue as u64) } else { None },
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
        }
    }
}

// ── TV Show conversions ──

macro_rules! impl_tv_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedTvShow {
            fn from(t: $ty) -> Self {
                UnifiedTvShow {
                    provider_id: format!("tmdb:{}", t.id),
                    name: t.name.unwrap_or_default(),
                    original_name: t.original_name,
                    overview: t.overview,
                    first_air_date: t.first_air_date,
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: Vec::new(),
                    popularity: t.popularity,
                    vote_average: t.vote_average,
                    vote_count: t.vote_count as u64,
                    original_language: t.original_language,
                    origin_country: t.origin_country,
                }
            }
        }
    };
}

impl_tv_from!(types::SearchTvResponseResultsItem);
impl_tv_from!(types::TrendingTvResponseResultsItem);

// DiscoverTvResponseResultsItem has vote_average: i64 (not Option<f64>) and no adult field
impl From<types::DiscoverTvResponseResultsItem> for UnifiedTvShow {
    fn from(t: types::DiscoverTvResponseResultsItem) -> Self {
        UnifiedTvShow {
            provider_id: format!("tmdb:{}", t.id),
            name: t.name.unwrap_or_default(),
            original_name: t.original_name,
            overview: t.overview,
            first_air_date: t.first_air_date,
            poster_url: resolve_poster(&t.poster_path),
            backdrop_url: resolve_backdrop(&t.backdrop_path),
            genres: Vec::new(),
            popularity: t.popularity,
            vote_average: if t.vote_average != 0 { Some(t.vote_average as f64) } else { None },
            vote_count: t.vote_count as u64,
            original_language: t.original_language,
            origin_country: t.origin_country,
        }
    }
}

impl From<types::TvSeriesDetailsResponse> for UnifiedTvShowDetails {
    fn from(t: types::TvSeriesDetailsResponse) -> Self {
        UnifiedTvShowDetails {
            show: UnifiedTvShow {
                provider_id: format!("tmdb:{}", t.id),
                name: t.name.unwrap_or_default(),
                original_name: t.original_name,
                overview: t.overview,
                first_air_date: t.first_air_date,
                poster_url: resolve_poster(&t.poster_path),
                backdrop_url: resolve_backdrop(&t.backdrop_path),
                genres: t.genres.iter().filter_map(|g| g.name.clone()).collect(),
                popularity: t.popularity,
                vote_average: t.vote_average,
                vote_count: t.vote_count as u64,
                original_language: t.original_language,
                origin_country: t.origin_country,
            },
            tagline: t.tagline,
            number_of_seasons: t.number_of_seasons as u32,
            number_of_episodes: t.number_of_episodes as u32,
            in_production: t.in_production,
            status: t.status,
            homepage: t.homepage,
            networks: t.networks.iter().filter_map(|n| n.name.clone()).collect(),
            production_companies: t
                .production_companies
                .iter()
                .filter_map(|c| c.name.clone())
                .collect(),
        }
    }
}

// ── Person conversions ──

impl From<types::SearchPersonResponseResultsItem> for UnifiedPerson {
    fn from(p: types::SearchPersonResponseResultsItem) -> Self {
        UnifiedPerson {
            provider_id: format!("tmdb:{}", p.id),
            name: p.name.unwrap_or_default(),
            known_for_department: p.known_for_department,
            profile_url: resolve_profile(&p.profile_path),
            popularity: p.popularity,
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
            },
            biography: p.biography,
            birthday: p.birthday,
            deathday: p.deathday.and_then(|v| v.as_str().map(String::from)),
            place_of_birth: p.place_of_birth,
            imdb_id: p.imdb_id,
            homepage: p.homepage.and_then(|v| v.as_str().map(String::from)),
        }
    }
}

// ── Multi-search conversion ──

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
                genres: Vec::new(),
                popularity: item.popularity,
                vote_average: item.vote_average,
                vote_count: item.vote_count as u64,
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
                genres: Vec::new(),
                popularity: item.popularity,
                vote_average: item.vote_average,
                vote_count: item.vote_count as u64,
                original_language: item.original_language,
                origin_country: Vec::new(),
            }),
            _ => UnifiedSearchResult::Person(UnifiedPerson {
                provider_id: format!("tmdb:{}", item.id),
                name: item.name.or(item.title).unwrap_or_default(),
                known_for_department: None,
                profile_url: resolve_poster(&item.poster_path), // multi uses poster_path
                popularity: item.popularity,
            }),
        }
    }
}
