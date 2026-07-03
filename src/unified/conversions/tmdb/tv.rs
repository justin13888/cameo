//! TMDB TV type → unified model conversions.

use super::{resolve_backdrop, resolve_date, resolve_logo, resolve_poster};
use crate::{
    generated::tmdb::types,
    providers::tmdb::image_url::{ImageUrl, StillSize},
    unified::{genre::Genre, media_id::MediaId, models::*},
};

/// TV summary types that carry an `adult` flag.
macro_rules! impl_tv_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedTvShow {
            fn from(t: $ty) -> Self {
                UnifiedTvShow {
                    provider_id: MediaId::tmdb(t.id as u64),
                    name: t.name.unwrap_or_default(),
                    original_name: t.original_name,
                    overview: t.overview,
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .iter()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: Some(t.popularity),
                    vote_average: Some(t.vote_average),
                    vote_count: Some(t.vote_count.max(0) as u64),
                    original_language: t.original_language,
                    origin_country: t.origin_country,
                    adult: Some(t.adult),
                }
            }
        }
    };
}

impl_tv_from!(types::SearchTvResponseResultsItem);
impl_tv_from!(types::TrendingTvResponseResultsItem);
impl_tv_from!(types::TvSeriesRecommendationsResponseResultsItem);
impl_tv_from!(types::TvSeriesSimilarResponseResultsItem);

/// TV summary types that do **not** carry an `adult` flag; `adult` stays `None`
/// rather than being fabricated as `false`.
macro_rules! impl_tv_from_no_adult {
    ($ty:ty) => {
        impl From<$ty> for UnifiedTvShow {
            fn from(t: $ty) -> Self {
                UnifiedTvShow {
                    provider_id: MediaId::tmdb(t.id as u64),
                    name: t.name.unwrap_or_default(),
                    original_name: t.original_name,
                    overview: t.overview,
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .iter()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: Some(t.popularity),
                    vote_average: Some(t.vote_average),
                    vote_count: Some(t.vote_count.max(0) as u64),
                    original_language: t.original_language,
                    origin_country: t.origin_country,
                    adult: None,
                }
            }
        }
    };
}

impl_tv_from_no_adult!(types::DiscoverTvResponseResultsItem);
impl_tv_from_no_adult!(types::TvSeriesPopularListResponseResultsItem);
impl_tv_from_no_adult!(types::TvSeriesTopRatedListResponseResultsItem);

impl From<types::TvSeasonDetailsResponseEpisodesItem> for UnifiedEpisode {
    fn from(e: types::TvSeasonDetailsResponseEpisodesItem) -> Self {
        UnifiedEpisode {
            episode_number: e.episode_number.max(0) as u32,
            name: e.name,
            overview: e.overview,
            air_date: resolve_date(&e.air_date),
            runtime: if e.runtime > 0 {
                Some(e.runtime.max(0) as u32)
            } else {
                None
            },
            still_url: e
                .still_path
                .as_deref()
                .map(|p| ImageUrl::still(p, StillSize::W300)),
            vote_average: Some(e.vote_average),
        }
    }
}

impl From<types::TvEpisodeDetailsResponse> for UnifiedEpisode {
    fn from(e: types::TvEpisodeDetailsResponse) -> Self {
        UnifiedEpisode {
            episode_number: e.episode_number.max(0) as u32,
            name: e.name,
            overview: e.overview,
            air_date: resolve_date(&e.air_date),
            runtime: if e.runtime > 0 {
                Some(e.runtime.max(0) as u32)
            } else {
                None
            },
            still_url: e
                .still_path
                .as_deref()
                .map(|p| ImageUrl::still(p, StillSize::W300)),
            vote_average: Some(e.vote_average),
        }
    }
}

impl From<types::TvSeriesDetailsResponseProductionCompaniesItem> for Company {
    fn from(c: types::TvSeriesDetailsResponseProductionCompaniesItem) -> Self {
        Company {
            id: u64::try_from(c.id).ok(),
            name: c.name.unwrap_or_default(),
            logo_url: resolve_logo(&c.logo_path),
            origin_country: c.origin_country,
        }
    }
}

impl From<types::TvSeriesDetailsResponseNetworksItem> for Network {
    fn from(n: types::TvSeriesDetailsResponseNetworksItem) -> Self {
        Network {
            id: u64::try_from(n.id).ok(),
            name: n.name.unwrap_or_default(),
            logo_url: resolve_logo(&n.logo_path),
            origin_country: n.origin_country,
        }
    }
}

impl From<types::TvSeriesDetailsResponseCreatedByItem> for Creator {
    fn from(c: types::TvSeriesDetailsResponseCreatedByItem) -> Self {
        Creator {
            id: u64::try_from(c.id).ok(),
            name: c.name.unwrap_or_default(),
            profile_url: super::resolve_profile(&c.profile_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponseSeasonsItem> for UnifiedSeason {
    fn from(s: types::TvSeriesDetailsResponseSeasonsItem) -> Self {
        UnifiedSeason {
            season_number: s.season_number.max(0) as u32,
            name: s.name,
            overview: s.overview,
            air_date: resolve_date(&s.air_date),
            episode_count: Some(s.episode_count.max(0) as u32),
            poster_url: resolve_poster(&s.poster_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponse> for UnifiedTvShowDetails {
    fn from(t: types::TvSeriesDetailsResponse) -> Self {
        UnifiedTvShowDetails {
            show: UnifiedTvShow {
                provider_id: MediaId::tmdb(t.id as u64),
                name: t.name.unwrap_or_default(),
                original_name: t.original_name,
                overview: t.overview,
                first_air_date: resolve_date(&t.first_air_date),
                poster_url: resolve_poster(&t.poster_path),
                backdrop_url: resolve_backdrop(&t.backdrop_path),
                genres: t.genres.iter().map(|g| Genre::from_tmdb_id(g.id)).collect(),
                popularity: Some(t.popularity),
                vote_average: Some(t.vote_average),
                vote_count: Some(t.vote_count.max(0) as u64),
                original_language: t.original_language,
                origin_country: t.origin_country,
                adult: Some(t.adult),
            },
            tagline: t.tagline,
            number_of_seasons: Some(t.number_of_seasons.max(0) as u32),
            number_of_episodes: Some(t.number_of_episodes.max(0) as u32),
            in_production: t.in_production,
            status: t.status,
            homepage: t.homepage,
            networks: t.networks.into_iter().map(Network::from).collect(),
            production_companies: t
                .production_companies
                .into_iter()
                .map(Company::from)
                .collect(),
            last_air_date: resolve_date(&t.last_air_date),
            type_: t.type_,
            media_format: None,
            created_by: t.created_by.into_iter().map(Creator::from).collect(),
            episode_run_time: t
                .episode_run_time
                .iter()
                .map(|&r| r.max(0) as u32)
                .collect(),
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
            seasons: t.seasons.into_iter().map(UnifiedSeason::from).collect(),
        }
    }
}

impl From<types::TvSeasonDetailsResponse> for UnifiedSeasonDetails {
    fn from(t: types::TvSeasonDetailsResponse) -> Self {
        UnifiedSeasonDetails {
            show_id: MediaId::tmdb(0), // filled in by the caller
            season_number: t.season_number.max(0) as u32,
            name: t.name,
            overview: t.overview,
            air_date: resolve_date(&t.air_date),
            poster_url: resolve_poster(&t.poster_path),
            episodes: t.episodes.into_iter().map(Into::into).collect(),
        }
    }
}
