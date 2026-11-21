//! TMDB TV type → unified model conversions.

use super::{
    certifications, resolve_backdrop, resolve_date, resolve_logo, resolve_poster, resolve_still,
    tmdb_media_id,
};
use crate::{
    generated::tmdb::types,
    unified::{genre::Genre, media_id::MediaId, models::*},
};

/// TV summary types that carry an `adult` flag.
macro_rules! impl_tv_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedTvShow {
            fn from(t: $ty) -> Self {
                UnifiedTvShow {
                    provider_id: tmdb_media_id(t.id),
                    name: t.name.into_option().unwrap_or_default(),
                    original_name: t.original_name.into_option(),
                    alternative_titles: Vec::new(),
                    overview: t.overview.into_option(),
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .as_option()
                        .into_iter()
                        .flatten()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: t.popularity.into_option(),
                    vote_average: t.vote_average.into_option(),
                    vote_count: t.vote_count.into_option().map(|c| c.max(0) as u64),
                    original_language: t.original_language.into_option(),
                    origin_country: t.origin_country.into_option().unwrap_or_default(),
                    adult: t.adult.into_option(),
                    // TMDB list endpoints return no episode data or status.
                    number_of_episodes: None,
                    episode_run_time: None,
                    status: None,
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
                    provider_id: tmdb_media_id(t.id),
                    name: t.name.into_option().unwrap_or_default(),
                    original_name: t.original_name.into_option(),
                    alternative_titles: Vec::new(),
                    overview: t.overview.into_option(),
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .as_option()
                        .into_iter()
                        .flatten()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: t.popularity.into_option(),
                    vote_average: t.vote_average.into_option(),
                    vote_count: t.vote_count.into_option().map(|c| c.max(0) as u64),
                    original_language: t.original_language.into_option(),
                    origin_country: t.origin_country.into_option().unwrap_or_default(),
                    adult: None,
                    // TMDB list endpoints return no episode data or status.
                    number_of_episodes: None,
                    episode_run_time: None,
                    status: None,
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
            episode_number: e.episode_number.into_option().unwrap_or_default().max(0) as u32,
            name: e.name.into_option(),
            overview: e.overview.into_option(),
            air_date: resolve_date(&e.air_date),
            runtime: e.runtime.into_option().filter(|&v| v > 0).map(|v| v as u32),
            still_url: resolve_still(&e.still_path),
            vote_average: e.vote_average.into_option(),
        }
    }
}

impl From<types::TvEpisodeDetailsResponse> for UnifiedEpisode {
    fn from(e: types::TvEpisodeDetailsResponse) -> Self {
        UnifiedEpisode {
            episode_number: e.episode_number.into_option().unwrap_or_default().max(0) as u32,
            name: e.name.into_option(),
            overview: e.overview.into_option(),
            air_date: resolve_date(&e.air_date),
            runtime: e.runtime.into_option().filter(|&v| v > 0).map(|v| v as u32),
            still_url: resolve_still(&e.still_path),
            vote_average: e.vote_average.into_option(),
        }
    }
}

impl From<types::TvSeriesDetailsResponseProductionCompaniesItem> for Company {
    fn from(c: types::TvSeriesDetailsResponseProductionCompaniesItem) -> Self {
        Company {
            id: c.id.into_option().and_then(|id| u64::try_from(id).ok()),
            name: c.name.into_option().unwrap_or_default(),
            logo_url: resolve_logo(&c.logo_path),
            origin_country: c.origin_country.into_option(),
        }
    }
}

impl From<types::TvSeriesDetailsResponseNetworksItem> for Network {
    fn from(n: types::TvSeriesDetailsResponseNetworksItem) -> Self {
        Network {
            id: n.id.into_option().and_then(|id| u64::try_from(id).ok()),
            name: n.name.into_option().unwrap_or_default(),
            logo_url: resolve_logo(&n.logo_path),
            origin_country: n.origin_country.into_option(),
        }
    }
}

impl From<types::TvSeriesDetailsResponseCreatedByItem> for Creator {
    fn from(c: types::TvSeriesDetailsResponseCreatedByItem) -> Self {
        Creator {
            id: c.id.into_option().and_then(|id| u64::try_from(id).ok()),
            name: c.name.into_option().unwrap_or_default(),
            profile_url: super::resolve_profile(&c.profile_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponseSeasonsItem> for UnifiedSeason {
    fn from(s: types::TvSeriesDetailsResponseSeasonsItem) -> Self {
        UnifiedSeason {
            season_number: s.season_number.into_option().unwrap_or_default().max(0) as u32,
            name: s.name.into_option(),
            overview: s.overview.into_option(),
            air_date: resolve_date(&s.air_date),
            episode_count: s.episode_count.into_option().map(|n| n.max(0) as u32),
            poster_url: resolve_poster(&s.poster_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponse> for UnifiedTvShowDetails {
    fn from(t: types::TvSeriesDetailsResponse) -> Self {
        let number_of_episodes = t.number_of_episodes.into_option().map(|n| n.max(0) as u32);
        let episode_run_time: Vec<u32> = t
            .episode_run_time
            .as_option()
            .into_iter()
            .flatten()
            .map(|&r| r.max(0) as u32)
            .collect();
        let status = t
            .status
            .as_option()
            .map(String::as_str)
            .map(SeriesStatus::from_tmdb);
        UnifiedTvShowDetails {
            show: UnifiedTvShow {
                provider_id: tmdb_media_id(t.id),
                name: t.name.into_option().unwrap_or_default(),
                original_name: t.original_name.into_option(),
                alternative_titles: Vec::new(),
                overview: t.overview.into_option(),
                first_air_date: resolve_date(&t.first_air_date),
                poster_url: resolve_poster(&t.poster_path),
                backdrop_url: resolve_backdrop(&t.backdrop_path),
                genres: t
                    .genres
                    .as_option()
                    .into_iter()
                    .flatten()
                    .filter_map(|g| g.id.into_option().map(Genre::from_tmdb_id))
                    .collect(),
                popularity: t.popularity.into_option(),
                vote_average: t.vote_average.into_option(),
                vote_count: t.vote_count.into_option().map(|c| c.max(0) as u64),
                original_language: t.original_language.into_option(),
                origin_country: t.origin_country.into_option().unwrap_or_default(),
                adult: t.adult.into_option(),
                // The embedded summary mirrors the detail-level episode
                // data/status (first listed run time).
                number_of_episodes,
                episode_run_time: episode_run_time.first().copied(),
                status: status.clone(),
            },
            tagline: t.tagline.into_option(),
            number_of_seasons: t.number_of_seasons.into_option().map(|n| n.max(0) as u32),
            number_of_episodes,
            // Spec default was `true`; an absent/null flag stays unknown.
            in_production: t.in_production.into_option(),
            status,
            homepage: t.homepage.into_option(),
            networks: t
                .networks
                .into_option()
                .into_iter()
                .flatten()
                .map(Network::from)
                .collect(),
            production_companies: t
                .production_companies
                .into_option()
                .into_iter()
                .flatten()
                .map(Company::from)
                .collect(),
            last_air_date: resolve_date(&t.last_air_date),
            type_: t
                .r#type
                .as_option()
                .map(String::as_str)
                .map(SeriesType::from_tmdb),
            media_format: None,
            created_by: t
                .created_by
                .into_option()
                .into_iter()
                .flatten()
                .map(Creator::from)
                .collect(),
            episode_run_time,
            spoken_languages: t
                .spoken_languages
                .as_option()
                .into_iter()
                .flatten()
                .filter_map(|l| l.english_name.clone().into_option())
                .collect(),
            production_countries: t
                .production_countries
                .as_option()
                .into_iter()
                .flatten()
                .filter_map(|c| c.name.clone().into_option())
                .collect(),
            // A season with no number cannot be addressed; defaulting it to 0
            // would pass it off as the Specials season, so it is dropped.
            seasons: t
                .seasons
                .into_option()
                .into_iter()
                .flatten()
                .filter(|s| s.season_number.into_option().is_some())
                .map(UnifiedSeason::from)
                .collect(),
            // Present only when the `content_ratings` append was requested.
            certifications: certifications::tv_series_results(
                t.content_ratings
                    .into_option()
                    .and_then(|block| block.results.into_option()),
            )
            .into_iter()
            .map(Certification::from)
            .collect(),
        }
    }
}

/// Build a [`UnifiedSeasonDetails`] from a TMDB season response.
///
/// The TMDB season endpoint response does not carry its parent show id, so it
/// is supplied by the caller and the value is valid by construction — there is
/// no placeholder to patch after the fact.
pub(crate) fn season_details_from(
    t: types::TvSeasonDetailsResponse,
    show_id: MediaId,
) -> UnifiedSeasonDetails {
    UnifiedSeasonDetails {
        show_id,
        season_number: t.season_number.into_option().unwrap_or_default().max(0) as u32,
        name: t.name.into_option(),
        overview: t.overview.into_option(),
        air_date: resolve_date(&t.air_date),
        poster_url: resolve_poster(&t.poster_path),
        // An episode with no number cannot be addressed; defaulting it to 0
        // would invent a phantom episode 0, so it is dropped.
        episodes: t
            .episodes
            .into_option()
            .into_iter()
            .flatten()
            .filter(|e| e.episode_number.into_option().is_some())
            .map(Into::into)
            .collect(),
    }
}

#[cfg(test)]
mod tests;
