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
        let number_of_episodes = Some(t.number_of_episodes.max(0) as u32);
        let episode_run_time: Vec<u32> = t
            .episode_run_time
            .iter()
            .map(|&r| r.max(0) as u32)
            .collect();
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
                // The embedded summary mirrors the detail-level episode
                // data/status (first listed run time).
                number_of_episodes,
                episode_run_time: episode_run_time.first().copied(),
                status: t.status.clone(),
            },
            tagline: t.tagline,
            number_of_seasons: Some(t.number_of_seasons.max(0) as u32),
            number_of_episodes,
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
            episode_run_time,
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
        season_number: t.season_number.max(0) as u32,
        name: t.name,
        overview: t.overview,
        air_date: resolve_date(&t.air_date),
        poster_url: resolve_poster(&t.poster_path),
        episodes: t.episodes.into_iter().map(Into::into).collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generated::tmdb::types,
        unified::{
            genre::Genre,
            models::{UnifiedTvShow, UnifiedTvShowDetails},
        },
    };

    #[test]
    fn search_tv_to_unified() {
        let raw = types::SearchTvResponseResultsItem {
            id: 1396,
            name: Some("Breaking Bad".to_string()),
            original_name: Some("Breaking Bad".to_string()),
            overview: Some("A high school chemistry teacher...".to_string()),
            first_air_date: Some("2008-01-20".to_string()),
            poster_path: Some("/ggFHVNu6YYI5L9pCfOacjizRGt.jpg".to_string()),
            backdrop_path: Some("/tsRy63Mu5cu8etL1X7ZLyf7UP1M.jpg".to_string()),
            genre_ids: vec![18, 80],
            origin_country: vec!["US".to_string()],
            popularity: 282.9,
            vote_average: 9.5,
            vote_count: 14000,
            original_language: Some("en".to_string()),
            adult: false,
        };
        let unified: UnifiedTvShow = raw.into();
        assert_eq!(unified.provider_id, "tmdb:1396");
        assert_eq!(unified.name, "Breaking Bad");
        assert_eq!(
            unified.first_air_date.map(|d| d.to_string()).as_deref(),
            Some("2008-01-20")
        );
        assert_eq!(unified.origin_country, vec!["US"]);
        assert_eq!(unified.vote_count, Some(14000));
        assert_eq!(unified.adult, Some(false));
        // Genres from IDs (18=Drama, 80=Crime).
        assert_eq!(unified.genres, vec![Genre::Drama, Genre::Crime]);
        // TMDB list endpoints return no episode data or status.
        assert_eq!(unified.number_of_episodes, None);
        assert_eq!(unified.episode_run_time, None);
        assert_eq!(unified.status, None);
    }

    /// TMDB details fill the embedded summary from the detail-level values
    /// (first listed episode run time).
    #[test]
    fn tv_details_embedded_show_mirrors_detail_fields() {
        let raw: types::TvSeriesDetailsResponse = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/tv_details_response.json"
        ))
        .expect("fixture deserializes");
        let details: UnifiedTvShowDetails = raw.into();
        assert_eq!(details.episode_run_time, vec![45, 47]);
        assert_eq!(details.show.episode_run_time, Some(45));
        assert_eq!(details.show.number_of_episodes, details.number_of_episodes);
        assert_eq!(details.show.number_of_episodes, Some(62));
        assert_eq!(details.show.status.as_deref(), Some("Ended"));
        assert_eq!(details.show.status, details.status);
    }
}
