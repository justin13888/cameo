//! Episode records.

use serde::{Deserialize, Serialize};

use super::{absolute_number, int_flag};

/// An episode, in the order it was listed in.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvdbEpisode {
    /// TheTVDB episode id.
    pub id: u64,
    /// The series the episode belongs to.
    #[serde(rename = "seriesId", default)]
    pub series_id: Option<u64>,
    /// Season number in the listed order; `0` holds the specials.
    #[serde(rename = "seasonNumber")]
    pub season_number: u32,
    /// Episode number within its season.
    pub number: u32,
    /// Episode number counted across the whole series, ignoring seasons — what
    /// maps an absolutely numbered file (typical of anime) onto its season and
    /// episode. `None` when TheTVDB has none (it sends `0`, as for most
    /// specials).
    #[serde(
        rename = "absoluteNumber",
        default,
        deserialize_with = "absolute_number"
    )]
    pub absolute_number: Option<u32>,
    /// Name, in the requested language when TheTVDB has a translation.
    #[serde(default)]
    pub name: Option<String>,
    /// Overview, in the requested language when TheTVDB has a translation.
    #[serde(default)]
    pub overview: Option<String>,
    /// Air date (`YYYY-MM-DD`).
    #[serde(default)]
    pub aired: Option<String>,
    /// Runtime in minutes.
    #[serde(default)]
    pub runtime: Option<u32>,
    /// Full still-image URL.
    #[serde(default)]
    pub image: Option<String>,
    /// Whether the episode is a movie (e.g. a feature-length special).
    #[serde(rename = "isMovie", default, deserialize_with = "int_flag")]
    pub is_movie: Option<bool>,
    /// The name of the episode's season, if it has one.
    #[serde(rename = "seasonName", default)]
    pub season_name: Option<String>,
    /// For a special: the season it airs before.
    #[serde(rename = "airsBeforeSeason", default)]
    pub airs_before_season: Option<u32>,
    /// For a special: the episode it airs before.
    #[serde(rename = "airsBeforeEpisode", default)]
    pub airs_before_episode: Option<u32>,
    /// For a special: the season it airs after.
    #[serde(rename = "airsAfterSeason", default)]
    pub airs_after_season: Option<u32>,
    /// Marks a finale (`"season"`, `"midseason"`, `"series"`).
    #[serde(rename = "finaleType", default)]
    pub finale_type: Option<String>,
}

impl TvdbEpisode {
    /// Whether this episode is a special (season `0`).
    pub fn is_special(&self) -> bool {
        self.season_number == 0
    }
}
