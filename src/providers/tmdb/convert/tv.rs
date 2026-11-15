//! TMDB TV type → unified model conversions.

use super::{resolve_backdrop, resolve_date, resolve_logo, resolve_poster, tmdb_media_id};
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
                    provider_id: tmdb_media_id(t.id),
                    name: t.name.unwrap_or_default(),
                    original_name: t.original_name,
                    alternative_titles: Vec::new(),
                    overview: t.overview,
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .iter()
                        .flatten()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: t.popularity,
                    vote_average: t.vote_average,
                    vote_count: t.vote_count.map(|c| c.max(0) as u64),
                    original_language: t.original_language,
                    origin_country: t.origin_country.unwrap_or_default(),
                    adult: t.adult,
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
                    name: t.name.unwrap_or_default(),
                    original_name: t.original_name,
                    alternative_titles: Vec::new(),
                    overview: t.overview,
                    first_air_date: resolve_date(&t.first_air_date),
                    poster_url: resolve_poster(&t.poster_path),
                    backdrop_url: resolve_backdrop(&t.backdrop_path),
                    genres: t
                        .genre_ids
                        .iter()
                        .flatten()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: t.popularity,
                    vote_average: t.vote_average,
                    vote_count: t.vote_count.map(|c| c.max(0) as u64),
                    original_language: t.original_language,
                    origin_country: t.origin_country.unwrap_or_default(),
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
            episode_number: e.episode_number.unwrap_or_default().max(0) as u32,
            name: e.name,
            overview: e.overview,
            air_date: resolve_date(&e.air_date),
            runtime: e.runtime.filter(|&v| v > 0).map(|v| v as u32),
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
            episode_number: e.episode_number.unwrap_or_default().max(0) as u32,
            name: e.name,
            overview: e.overview,
            air_date: resolve_date(&e.air_date),
            runtime: e.runtime.filter(|&v| v > 0).map(|v| v as u32),
            still_url: e
                .still_path
                .as_deref()
                .map(|p| ImageUrl::still(p, StillSize::W300)),
            vote_average: e.vote_average,
        }
    }
}

impl From<types::TvSeriesDetailsResponseProductionCompaniesItem> for Company {
    fn from(c: types::TvSeriesDetailsResponseProductionCompaniesItem) -> Self {
        Company {
            id: c.id.and_then(|id| u64::try_from(id).ok()),
            name: c.name.unwrap_or_default(),
            logo_url: resolve_logo(&c.logo_path),
            origin_country: c.origin_country,
        }
    }
}

impl From<types::TvSeriesDetailsResponseNetworksItem> for Network {
    fn from(n: types::TvSeriesDetailsResponseNetworksItem) -> Self {
        Network {
            id: n.id.and_then(|id| u64::try_from(id).ok()),
            name: n.name.unwrap_or_default(),
            logo_url: resolve_logo(&n.logo_path),
            origin_country: n.origin_country,
        }
    }
}

impl From<types::TvSeriesDetailsResponseCreatedByItem> for Creator {
    fn from(c: types::TvSeriesDetailsResponseCreatedByItem) -> Self {
        Creator {
            id: c.id.and_then(|id| u64::try_from(id).ok()),
            name: c.name.unwrap_or_default(),
            profile_url: super::resolve_profile(&c.profile_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponseSeasonsItem> for UnifiedSeason {
    fn from(s: types::TvSeriesDetailsResponseSeasonsItem) -> Self {
        UnifiedSeason {
            season_number: s.season_number.unwrap_or_default().max(0) as u32,
            name: s.name,
            overview: s.overview,
            air_date: resolve_date(&s.air_date),
            episode_count: s.episode_count.map(|n| n.max(0) as u32),
            poster_url: resolve_poster(&s.poster_path),
        }
    }
}

impl From<types::TvSeriesDetailsResponse> for UnifiedTvShowDetails {
    fn from(t: types::TvSeriesDetailsResponse) -> Self {
        let number_of_episodes = t.number_of_episodes.map(|n| n.max(0) as u32);
        let episode_run_time: Vec<u32> = t
            .episode_run_time
            .iter()
            .flatten()
            .map(|&r| r.max(0) as u32)
            .collect();
        let status = t.status.as_deref().map(SeriesStatus::from_tmdb);
        UnifiedTvShowDetails {
            show: UnifiedTvShow {
                provider_id: tmdb_media_id(t.id),
                name: t.name.unwrap_or_default(),
                original_name: t.original_name,
                alternative_titles: Vec::new(),
                overview: t.overview,
                first_air_date: resolve_date(&t.first_air_date),
                poster_url: resolve_poster(&t.poster_path),
                backdrop_url: resolve_backdrop(&t.backdrop_path),
                genres: t
                    .genres
                    .iter()
                    .flatten()
                    .filter_map(|g| g.id.map(Genre::from_tmdb_id))
                    .collect(),
                popularity: t.popularity,
                vote_average: t.vote_average,
                vote_count: t.vote_count.map(|c| c.max(0) as u64),
                original_language: t.original_language,
                origin_country: t.origin_country.unwrap_or_default(),
                adult: t.adult,
                // The embedded summary mirrors the detail-level episode
                // data/status (first listed run time).
                number_of_episodes,
                episode_run_time: episode_run_time.first().copied(),
                status: status.clone(),
            },
            tagline: t.tagline,
            number_of_seasons: t.number_of_seasons.map(|n| n.max(0) as u32),
            number_of_episodes,
            // Spec default was `true`; an absent/null flag stays unknown.
            in_production: t.in_production,
            status,
            homepage: t.homepage,
            networks: t
                .networks
                .into_iter()
                .flatten()
                .map(Network::from)
                .collect(),
            production_companies: t
                .production_companies
                .into_iter()
                .flatten()
                .map(Company::from)
                .collect(),
            last_air_date: resolve_date(&t.last_air_date),
            type_: t.r#type.as_deref().map(SeriesType::from_tmdb),
            media_format: None,
            created_by: t
                .created_by
                .into_iter()
                .flatten()
                .map(Creator::from)
                .collect(),
            episode_run_time,
            spoken_languages: t
                .spoken_languages
                .iter()
                .flatten()
                .filter_map(|l| l.english_name.clone())
                .collect(),
            production_countries: t
                .production_countries
                .iter()
                .flatten()
                .filter_map(|c| c.name.clone())
                .collect(),
            // A season with no number cannot be addressed; defaulting it to 0
            // would pass it off as the Specials season, so it is dropped.
            seasons: t
                .seasons
                .into_iter()
                .flatten()
                .filter(|s| s.season_number.is_some())
                .map(UnifiedSeason::from)
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
        season_number: t.season_number.unwrap_or_default().max(0) as u32,
        name: t.name,
        overview: t.overview,
        air_date: resolve_date(&t.air_date),
        poster_url: resolve_poster(&t.poster_path),
        // An episode with no number cannot be addressed; defaulting it to 0
        // would invent a phantom episode 0, so it is dropped.
        episodes: t
            .episodes
            .into_iter()
            .flatten()
            .filter(|e| e.episode_number.is_some())
            .map(Into::into)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generated::tmdb::types,
        unified::{
            genre::Genre,
            media_id::MediaId,
            models::{
                SeriesStatus, SeriesType, UnifiedEpisode, UnifiedTvShow, UnifiedTvShowDetails,
            },
        },
    };

    /// Every detail field is mapped: networks, companies and creators keep
    /// valid ids and get full image URLs, run times and counts are clamped,
    /// and seasons without a number are dropped.
    #[test]
    fn tv_details_converts_every_field() {
        let raw: types::TvSeriesDetailsResponse = serde_json::from_value(serde_json::json!({
            "id": 1399,
            "name": "Game of Thrones",
            "original_name": "Game of Thrones",
            "overview": "Seven noble families fight.",
            "first_air_date": "2011-04-17",
            "last_air_date": "2019-05-19",
            "poster_path": "/poster.jpg",
            "backdrop_path": "/backdrop.jpg",
            "genres": [{ "id": 10765 }, { "name": "No id" }, { "id": 18 }],
            "popularity": 400.1,
            "vote_average": 8.4,
            "vote_count": 21000,
            "original_language": "en",
            "origin_country": ["US"],
            "adult": false,
            "tagline": "Winter is coming.",
            "number_of_seasons": 8,
            "number_of_episodes": -1,
            "in_production": false,
            "status": "Ended",
            "type": "Scripted",
            "homepage": "https://www.hbo.com/game-of-thrones",
            "episode_run_time": [-5, 60],
            "networks": [{ "id": 49, "name": "HBO", "logo_path": "/hbo.png", "origin_country": "US" }],
            "production_companies": [{ "id": -2, "name": "Bighead" }],
            "created_by": [
                { "id": 9813, "name": "David Benioff", "profile_path": "/db.jpg" },
                { "name": "No id" }
            ],
            "spoken_languages": [{ "english_name": "English" }],
            "production_countries": [{ "name": "United Kingdom" }],
            "seasons": [
                { "season_number": 0, "name": "Specials", "episode_count": 3, "air_date": "2010-12-05", "poster_path": "/s0.jpg" },
                { "name": "No number" },
                { "season_number": 1, "name": "Season 1", "episode_count": -4 }
            ]
        }))
        .expect("details deserialize");

        let details: UnifiedTvShowDetails = raw.into();

        let show = &details.show;
        assert_eq!(show.provider_id, "tmdb:1399");
        assert_eq!(show.name, "Game of Thrones");
        assert_eq!(show.original_name.as_deref(), Some("Game of Thrones"));
        assert_eq!(
            show.overview.as_deref(),
            Some("Seven noble families fight.")
        );
        assert_eq!(
            show.first_air_date.map(|d| d.to_string()).as_deref(),
            Some("2011-04-17")
        );
        assert_eq!(
            show.poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/poster.jpg")
        );
        assert_eq!(
            show.backdrop_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w780/backdrop.jpg")
        );
        assert_eq!(show.genres, vec![Genre::SciFiFantasy, Genre::Drama]);
        assert_eq!(show.popularity, Some(400.1));
        assert_eq!(show.vote_average, Some(8.4));
        assert_eq!(show.vote_count, Some(21000));
        assert_eq!(show.original_language.as_deref(), Some("en"));
        assert_eq!(show.origin_country, ["US"]);
        assert_eq!(show.adult, Some(false));
        assert!(show.alternative_titles.is_empty());
        assert_eq!(show.number_of_episodes, Some(0));
        assert_eq!(show.episode_run_time, Some(0));
        assert_eq!(show.status, Some(SeriesStatus::Ended));

        assert_eq!(details.tagline.as_deref(), Some("Winter is coming."));
        assert_eq!(details.number_of_seasons, Some(8));
        assert_eq!(details.number_of_episodes, Some(0));
        assert_eq!(details.in_production, Some(false));
        assert_eq!(details.status, Some(SeriesStatus::Ended));
        assert_eq!(details.type_, Some(SeriesType::Scripted));
        assert_eq!(details.media_format, None);
        assert_eq!(
            details.homepage.as_deref(),
            Some("https://www.hbo.com/game-of-thrones")
        );
        assert_eq!(
            details.last_air_date.map(|d| d.to_string()).as_deref(),
            Some("2019-05-19")
        );
        assert_eq!(details.episode_run_time, [0, 60]);
        assert_eq!(details.spoken_languages, ["English"]);
        assert_eq!(details.production_countries, ["United Kingdom"]);

        let networks: Vec<_> = details
            .networks
            .iter()
            .map(|n| {
                (
                    n.id,
                    n.name.as_str(),
                    n.logo_url.as_deref(),
                    n.origin_country.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            networks,
            [(
                Some(49),
                "HBO",
                Some("https://image.tmdb.org/t/p/w92/hbo.png"),
                Some("US")
            )]
        );
        let companies: Vec<_> = details
            .production_companies
            .iter()
            .map(|c| (c.id, c.name.as_str()))
            .collect();
        assert_eq!(companies, [(None, "Bighead")]);
        let creators: Vec<_> = details
            .created_by
            .iter()
            .map(|c| (c.id, c.name.as_str(), c.profile_url.as_deref()))
            .collect();
        assert_eq!(
            creators,
            [
                (
                    Some(9813),
                    "David Benioff",
                    Some("https://image.tmdb.org/t/p/h632/db.jpg")
                ),
                (None, "No id", None),
            ]
        );

        let seasons: Vec<_> = details
            .seasons
            .iter()
            .map(|s| {
                (
                    s.season_number,
                    s.name.as_deref(),
                    s.episode_count,
                    s.air_date.map(|d| d.to_string()),
                    s.poster_url.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            seasons,
            [
                (
                    0,
                    Some("Specials"),
                    Some(3),
                    Some("2010-12-05".to_string()),
                    Some("https://image.tmdb.org/t/p/w500/s0.jpg")
                ),
                (1, Some("Season 1"), Some(0), None, None),
            ]
        );
    }

    /// The show id comes from the caller, and episodes without a number are
    /// dropped rather than becoming a phantom episode 0.
    #[test]
    fn season_details_converts_fields_and_drops_unnumbered_episodes() {
        let raw: types::TvSeasonDetailsResponse = serde_json::from_value(serde_json::json!({
            "season_number": 2,
            "name": "Season 2",
            "overview": "War comes.",
            "air_date": "2012-04-01",
            "poster_path": "/s2.jpg",
            "episodes": [
                {
                    "episode_number": 1,
                    "name": "The North Remembers",
                    "overview": "Tyrion arrives.",
                    "air_date": "2012-04-01",
                    "runtime": 53,
                    "still_path": "/e1.jpg",
                    "vote_average": 7.9
                },
                { "name": "No number" },
                { "episode_number": 2, "runtime": 0 }
            ]
        }))
        .expect("season deserializes");

        let season = super::season_details_from(raw, MediaId::tmdb(1399));

        assert_eq!(season.show_id, "tmdb:1399");
        assert_eq!(season.season_number, 2);
        assert_eq!(season.name.as_deref(), Some("Season 2"));
        assert_eq!(season.overview.as_deref(), Some("War comes."));
        assert_eq!(
            season.air_date.map(|d| d.to_string()).as_deref(),
            Some("2012-04-01")
        );
        assert_eq!(
            season.poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/s2.jpg")
        );
        let episodes: Vec<_> = season
            .episodes
            .iter()
            .map(|e| {
                (
                    e.episode_number,
                    e.name.as_deref(),
                    e.overview.as_deref(),
                    e.air_date.map(|d| d.to_string()),
                    e.runtime,
                    e.still_url.as_deref(),
                    e.vote_average,
                )
            })
            .collect();
        assert_eq!(
            episodes,
            [
                (
                    1,
                    Some("The North Remembers"),
                    Some("Tyrion arrives."),
                    Some("2012-04-01".to_string()),
                    Some(53),
                    Some("https://image.tmdb.org/t/p/w300/e1.jpg"),
                    Some(7.9)
                ),
                (2, None, None, None, None, None, None),
            ]
        );
    }

    #[test]
    fn episode_details_converts_every_field() {
        let raw: types::TvEpisodeDetailsResponse = serde_json::from_value(serde_json::json!({
            "episode_number": 9,
            "name": "Baelor",
            "overview": "Ned makes a choice.",
            "air_date": "2011-06-12",
            "runtime": -1,
            "still_path": "/baelor.jpg",
            "vote_average": 9.1
        }))
        .expect("episode deserializes");

        let episode: UnifiedEpisode = raw.into();

        assert_eq!(episode.episode_number, 9);
        assert_eq!(episode.name.as_deref(), Some("Baelor"));
        assert_eq!(episode.overview.as_deref(), Some("Ned makes a choice."));
        assert_eq!(
            episode.air_date.map(|d| d.to_string()).as_deref(),
            Some("2011-06-12")
        );
        // A non-positive runtime is unknown, not zero minutes.
        assert_eq!(episode.runtime, None);
        assert_eq!(
            episode.still_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w300/baelor.jpg")
        );
        assert_eq!(episode.vote_average, Some(9.1));
    }

    #[test]
    fn episode_details_without_number_defaults_to_zero() {
        let raw: types::TvEpisodeDetailsResponse =
            serde_json::from_value(serde_json::json!({})).expect("episode deserializes");
        let episode: UnifiedEpisode = raw.into();
        assert_eq!(episode.episode_number, 0);
        assert_eq!(episode.still_url, None);
    }

    #[test]
    fn search_tv_to_unified() {
        let raw = types::SearchTvResponseResultsItem {
            id: Some(1396),
            name: Some("Breaking Bad".to_string()),
            original_name: Some("Breaking Bad".to_string()),
            overview: Some("A high school chemistry teacher...".to_string()),
            first_air_date: Some("2008-01-20".to_string()),
            poster_path: Some("/ggFHVNu6YYI5L9pCfOacjizRGt.jpg".to_string()),
            backdrop_path: Some("/tsRy63Mu5cu8etL1X7ZLyf7UP1M.jpg".to_string()),
            genre_ids: Some(vec![18, 80]),
            origin_country: Some(vec!["US".to_string()]),
            popularity: Some(282.9),
            vote_average: Some(9.5),
            vote_count: Some(14000),
            original_language: Some("en".to_string()),
            adult: Some(false),
        };
        let unified: UnifiedTvShow = raw.into();
        assert_eq!(unified.provider_id, "tmdb:1396");
        // TMDB exposes no title variants beyond name/original_name.
        assert!(unified.alternative_titles.is_empty());
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
        assert_eq!(details.show.status, Some(SeriesStatus::Ended));
        assert_eq!(details.show.status, details.status);
    }
}
