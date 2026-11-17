//! TheTVDB episodes → unified episode and season conversions.

use super::{non_blank, parse_date};
use crate::{
    providers::tvdb::response::TvdbEpisode,
    unified::{
        media_id::MediaId,
        models::{UnifiedEpisode, UnifiedSeasonDetails},
    },
};

/// An episode as a [`UnifiedEpisode`].
pub(crate) fn episode_to_unified(e: &TvdbEpisode) -> UnifiedEpisode {
    let mut episode = UnifiedEpisode::new(e.number);
    if let Some(v) = non_blank(e.name.as_deref()) {
        episode = episode.with_name(v);
    }
    if let Some(v) = non_blank(e.overview.as_deref()) {
        episode = episode.with_overview(v);
    }
    if let Some(v) = parse_date(e.aired.as_deref()) {
        episode = episode.with_air_date(v);
    }
    if let Some(v) = e.runtime.filter(|&r| r > 0) {
        episode = episode.with_runtime(v);
    }
    if let Some(v) = non_blank(e.image.as_deref()) {
        episode = episode.with_still_url(v);
    }
    episode
}

/// One season of a series built from its default-order `episodes`; `None`
/// when no episode is in `season_number`. The air date is the earliest
/// episode's.
pub(crate) fn season_details(
    show_id: MediaId,
    season_number: u32,
    episodes: &[TvdbEpisode],
) -> Option<UnifiedSeasonDetails> {
    let mut in_season: Vec<&TvdbEpisode> = episodes
        .iter()
        .filter(|e| e.season_number == season_number)
        .collect();
    if in_season.is_empty() {
        return None;
    }
    in_season.sort_by_key(|e| e.number);
    let mut season = UnifiedSeasonDetails::new(show_id, season_number)
        .with_episodes(in_season.iter().map(|e| episode_to_unified(e)).collect());
    if let Some(v) = in_season
        .iter()
        .find_map(|e| non_blank(e.season_name.as_deref()))
    {
        season = season.with_name(v);
    }
    if let Some(v) = in_season
        .iter()
        .filter_map(|e| parse_date(e.aired.as_deref()))
        .min()
    {
        season = season.with_air_date(v);
    }
    Some(season)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::providers::tvdb::convert::tests::from_json;

    #[test]
    fn season_details_keep_specials_and_order_episodes() {
        let episodes: Vec<TvdbEpisode> = from_json(json!([
            { "id": 3, "seasonNumber": 1, "number": 2, "absoluteNumber": 2, "aired": "2008-01-27" },
            { "id": 2, "seasonNumber": 1, "number": 1, "absoluteNumber": 1, "aired": "2008-01-20",
              "name": "Pilot", "runtime": 58, "image": "https://img/1.jpg" },
            { "id": 9, "seasonNumber": 0, "number": 1, "absoluteNumber": 0, "name": "Special",
              "runtime": 0 }
        ]));
        let season = season_details(MediaId::tvdb(81189), 1, &episodes).expect("season 1");
        let numbers: Vec<u32> = season.episodes.iter().map(|e| e.episode_number).collect();
        assert_eq!(numbers, vec![1, 2]);
        assert_eq!(
            season.air_date.map(|d| d.to_string()).as_deref(),
            Some("2008-01-20")
        );
        assert_eq!(season.episodes[0].name.as_deref(), Some("Pilot"));
        assert_eq!(season.episodes[0].runtime, Some(58));
        assert_eq!(
            season.episodes[0].still_url.as_deref(),
            Some("https://img/1.jpg")
        );

        let specials = season_details(MediaId::tvdb(81189), 0, &episodes).expect("specials");
        assert_eq!(specials.episodes.len(), 1);
        assert_eq!(
            specials.episodes[0].runtime, None,
            "a zero runtime is unknown"
        );
        assert!(season_details(MediaId::tvdb(81189), 7, &episodes).is_none());
    }
}
