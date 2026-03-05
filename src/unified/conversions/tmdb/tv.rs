//! TMDB TV type → unified model conversions.

use super::{resolve_backdrop, resolve_poster};
use crate::{
    generated::tmdb::types,
    providers::tmdb::image_url::{ImageUrl, StillSize},
    unified::{genre::Genre, models::*},
};

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
                    genres: t
                        .genre_ids
                        .iter()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: t.popularity,
                    vote_average: t.vote_average,
                    vote_count: t.vote_count as u64,
                    original_language: t.original_language,
                    origin_country: t.origin_country,
                    adult: t.adult,
                }
            }
        }
    };
}

impl_tv_from!(types::SearchTvResponseResultsItem);
impl_tv_from!(types::TrendingTvResponseResultsItem);
impl_tv_from!(types::TvSeriesRecommendationsResponseResultsItem);

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
            genres: t
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: t.popularity,
            vote_average: if t.vote_average != 0 {
                Some(t.vote_average as f64)
            } else {
                None
            },
            vote_count: t.vote_count as u64,
            original_language: t.original_language,
            origin_country: t.origin_country,
            adult: false,
        }
    }
}

// TvSeriesPopularListResponseResultsItem has vote_average: i64 and no adult field
impl From<types::TvSeriesPopularListResponseResultsItem> for UnifiedTvShow {
    fn from(t: types::TvSeriesPopularListResponseResultsItem) -> Self {
        UnifiedTvShow {
            provider_id: format!("tmdb:{}", t.id),
            name: t.name.unwrap_or_default(),
            original_name: t.original_name,
            overview: t.overview,
            first_air_date: t.first_air_date,
            poster_url: resolve_poster(&t.poster_path),
            backdrop_url: resolve_backdrop(&t.backdrop_path),
            genres: t
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: t.popularity,
            vote_average: if t.vote_average != 0 {
                Some(t.vote_average as f64)
            } else {
                None
            },
            vote_count: t.vote_count as u64,
            original_language: t.original_language,
            origin_country: t.origin_country,
            adult: false,
        }
    }
}

// TvSeriesTopRatedListResponseResultsItem has vote_average: Option<f64> and no adult field
impl From<types::TvSeriesTopRatedListResponseResultsItem> for UnifiedTvShow {
    fn from(t: types::TvSeriesTopRatedListResponseResultsItem) -> Self {
        UnifiedTvShow {
            provider_id: format!("tmdb:{}", t.id),
            name: t.name.unwrap_or_default(),
            original_name: t.original_name,
            overview: t.overview,
            first_air_date: t.first_air_date,
            poster_url: resolve_poster(&t.poster_path),
            backdrop_url: resolve_backdrop(&t.backdrop_path),
            genres: t
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: t.popularity,
            vote_average: t.vote_average,
            vote_count: t.vote_count as u64,
            original_language: t.original_language,
            origin_country: t.origin_country,
            adult: false,
        }
    }
}

// TvSeriesSimilarResponseResultsItem has vote_average: i64 (not Option<f64>)
impl From<types::TvSeriesSimilarResponseResultsItem> for UnifiedTvShow {
    fn from(t: types::TvSeriesSimilarResponseResultsItem) -> Self {
        UnifiedTvShow {
            provider_id: format!("tmdb:{}", t.id),
            name: t.name.unwrap_or_default(),
            original_name: t.original_name,
            overview: t.overview,
            first_air_date: t.first_air_date,
            poster_url: resolve_poster(&t.poster_path),
            backdrop_url: resolve_backdrop(&t.backdrop_path),
            genres: t
                .genre_ids
                .iter()
                .map(|&id| Genre::from_tmdb_id(id))
                .collect(),
            popularity: t.popularity,
            vote_average: if t.vote_average != 0 {
                Some(t.vote_average as f64)
            } else {
                None
            },
            vote_count: t.vote_count as u64,
            original_language: t.original_language,
            origin_country: t.origin_country,
            adult: t.adult,
        }
    }
}

impl From<types::TvSeasonDetailsResponse> for UnifiedSeasonDetails {
    fn from(t: types::TvSeasonDetailsResponse) -> Self {
        UnifiedSeasonDetails {
            show_id: String::new(), // filled in by the caller
            season_number: t.season_number as u32,
            name: t.name,
            overview: t.overview,
            air_date: t.air_date,
            poster_url: resolve_poster(&t.poster_path),
            episodes: t.episodes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<types::TvSeasonDetailsResponseEpisodesItem> for UnifiedEpisode {
    fn from(e: types::TvSeasonDetailsResponseEpisodesItem) -> Self {
        UnifiedEpisode {
            episode_number: e.episode_number as u32,
            name: e.name,
            overview: e.overview,
            air_date: e.air_date,
            runtime: if e.runtime > 0 {
                Some(e.runtime as u32)
            } else {
                None
            },
            still_url: e
                .still_path
                .as_deref()
                .map(|p| ImageUrl::still(p, StillSize::W300)),
            vote_average: e.vote_average,
        }
    }
}

impl From<types::TvEpisodeDetailsResponse> for UnifiedEpisode {
    fn from(e: types::TvEpisodeDetailsResponse) -> Self {
        UnifiedEpisode {
            episode_number: e.episode_number as u32,
            name: e.name,
            overview: e.overview,
            air_date: e.air_date,
            runtime: if e.runtime > 0 {
                Some(e.runtime as u32)
            } else {
                None
            },
            still_url: e
                .still_path
                .as_deref()
                .map(|p| ImageUrl::still(p, StillSize::W300)),
            vote_average: e.vote_average,
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
                first_air_date: t.first_air_date.clone(),
                poster_url: resolve_poster(&t.poster_path),
                backdrop_url: resolve_backdrop(&t.backdrop_path),
                genres: t
                    .genres
                    .iter()
                    .filter_map(|g| g.name.as_deref())
                    .map(Genre::from_name)
                    .collect(),
                popularity: t.popularity,
                vote_average: t.vote_average,
                vote_count: t.vote_count as u64,
                original_language: t.original_language,
                origin_country: t.origin_country,
                adult: t.adult,
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
            last_air_date: t.last_air_date,
            type_: t.type_,
            created_by: t.created_by.iter().filter_map(|c| c.name.clone()).collect(),
            episode_run_time: t.episode_run_time.iter().map(|&r| r as u32).collect(),
            spoken_languages: t
                .spoken_languages
                .iter()
                .filter_map(|l| l.english_name.clone())
                .collect(),
            production_countries: t
                .production_countries
                .iter()
                .filter_map(|c| c.name.clone())
                .collect(),
        }
    }
}
