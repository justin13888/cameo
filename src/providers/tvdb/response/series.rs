//! Series records and their seasons.

use serde::{Deserialize, Serialize};

use super::{
    TvdbAlias, TvdbArtwork, TvdbCompany, TvdbContentRating, TvdbGenre, TvdbRemoteId, TvdbStatus,
    TvdbTranslations, null_default,
};

/// The kind of a season's episode order.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbSeasonType {
    /// TheTVDB season-type id (compare with
    /// [`TvdbSeries::default_season_type`]).
    #[serde(default)]
    pub id: Option<i64>,
    /// Display name (e.g. `"Aired Order"`).
    #[serde(default)]
    pub name: Option<String>,
    /// Machine name (`"official"`, `"dvd"`, `"absolute"`, `"alternate"`, ...).
    #[serde(rename = "type", default)]
    pub order: Option<String>,
}

/// A season of a series, in one of its episode orders.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbSeason {
    /// TheTVDB season id.
    pub id: u64,
    /// Season number; `0` holds the specials.
    pub number: u32,
    /// Which episode order this season belongs to.
    #[serde(rename = "type", default)]
    pub season_type: Option<TvdbSeasonType>,
    /// Full poster URL.
    #[serde(default)]
    pub image: Option<String>,
    /// Year the season first aired.
    #[serde(default)]
    pub year: Option<String>,
}

/// A series, as `GET /series/{id}/extended?meta=translations`
/// returns it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvdbSeries {
    /// TheTVDB series id.
    pub id: u64,
    /// Name in the original language.
    pub name: String,
    /// URL slug.
    #[serde(default)]
    pub slug: Option<String>,
    /// Full poster URL.
    #[serde(default)]
    pub image: Option<String>,
    /// Overview in the original language.
    #[serde(default)]
    pub overview: Option<String>,
    /// First air date (`YYYY-MM-DD`).
    #[serde(rename = "firstAired", default)]
    pub first_aired: Option<String>,
    /// Last air date (`YYYY-MM-DD`).
    #[serde(rename = "lastAired", default)]
    pub last_aired: Option<String>,
    /// Next air date (`YYYY-MM-DD`), while the series airs.
    #[serde(rename = "nextAired", default)]
    pub next_aired: Option<String>,
    /// Year of first release.
    #[serde(default)]
    pub year: Option<String>,
    /// TheTVDB popularity score.
    #[serde(default)]
    pub score: Option<f64>,
    /// Release status.
    #[serde(default)]
    pub status: Option<TvdbStatus>,
    /// Country of origin (ISO 3166-1 alpha-3, lowercase).
    #[serde(rename = "originalCountry", default)]
    pub original_country: Option<String>,
    /// Original language (ISO 639-2).
    #[serde(rename = "originalLanguage", default)]
    pub original_language: Option<String>,
    /// Typical episode runtime in minutes.
    #[serde(rename = "averageRuntime", default)]
    pub average_runtime: Option<u32>,
    /// The season-type id of the series' default episode order.
    #[serde(rename = "defaultSeasonType", default)]
    pub default_season_type: Option<i64>,
    /// The network the series first aired on.
    #[serde(rename = "originalNetwork", default)]
    pub original_network: Option<TvdbCompany>,
    /// The network the series airs on now.
    #[serde(rename = "latestNetwork", default)]
    pub latest_network: Option<TvdbCompany>,
    /// Aliases.
    #[serde(default, deserialize_with = "null_default")]
    pub aliases: Vec<TvdbAlias>,
    /// Artwork.
    #[serde(default, deserialize_with = "null_default")]
    pub artworks: Vec<TvdbArtwork>,
    /// Genres.
    #[serde(default, deserialize_with = "null_default")]
    pub genres: Vec<TvdbGenre>,
    /// Ids in other databases.
    #[serde(rename = "remoteIds", default, deserialize_with = "null_default")]
    pub remote_ids: Vec<TvdbRemoteId>,
    /// Content ratings.
    #[serde(rename = "contentRatings", default, deserialize_with = "null_default")]
    pub content_ratings: Vec<TvdbContentRating>,
    /// Seasons, in every episode order the series has.
    #[serde(default, deserialize_with = "null_default")]
    pub seasons: Vec<TvdbSeason>,
    /// Name and overview translations.
    #[serde(default, deserialize_with = "null_default")]
    pub translations: TvdbTranslations,
}
