//! TheTVDB v4 records, as the API returns them.
//!
//! These are hand-owned types covering the fields cameo reads: they are
//! returned by [`TvdbClient`](crate::providers::tvdb::TvdbClient)'s inherent methods so that
//! TheTVDB-only data (content ratings, every translation, artwork, and each
//! episode's absolute number) stays reachable, and they are converted to the
//! unified models by the provider-trait implementations.
//!
//! TheTVDB sends `null` for many empty lists; every list field here reads a
//! `null` as empty.

use serde::{Deserialize, Deserializer, Serialize};

mod episode;
mod movie;
mod search;
mod series;
#[cfg(test)]
mod tests;

pub use episode::TvdbEpisode;
pub use movie::{TvdbMovie, TvdbMovieCompanies, TvdbProductionCountry, TvdbRelease};
pub use search::TvdbSearchResult;
pub use series::{TvdbSeason, TvdbSeasonType, TvdbSeries};

/// Read a `null` (or absent) value as `T::default()`.
fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Read an amount TheTVDB sends either as a number or as a decimal string
/// (movie `budget` and `boxOffice` are strings); anything else is `None`.
fn lenient_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Number(n)) => n.as_u64(),
            Some(serde_json::Value::String(s)) => s.trim().parse().ok(),
            _ => None,
        },
    )
}

/// Read TheTVDB's `0`/`1` integer flags (and real booleans) as a `bool`.
fn int_flag<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<serde_json::Value>::deserialize(deserializer)? {
            Some(serde_json::Value::Bool(b)) => Some(b),
            Some(serde_json::Value::Number(n)) => n.as_i64().map(|v| v != 0),
            _ => None,
        },
    )
}

/// Read TheTVDB's absolute episode number, where `0` means "none".
fn absolute_number<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<u32>::deserialize(deserializer)?.filter(|&n| n != 0))
}

// ── Envelope ──────────────────────────────────────────────────────────────────

/// Every v4 response: `{ "status": ..., "data": ..., "links": ... }`.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    /// The payload; `null` on some failures.
    pub(crate) data: Option<T>,
    /// Paging links, on paged endpoints.
    #[serde(default)]
    pub(crate) links: Option<Links>,
}

/// Paging metadata of a v4 list response.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Links {
    /// URL of the next page, `null` on the last one.
    #[serde(default)]
    pub(crate) next: Option<String>,
    /// Total records across every page.
    #[serde(default)]
    pub(crate) total_items: Option<u64>,
    /// Records per page.
    #[serde(default)]
    pub(crate) page_size: Option<u64>,
}

/// The error body TheTVDB sends with a non-2xx status.
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorBody {
    #[serde(default)]
    pub(crate) message: Option<String>,
}

/// `data` of `POST /login`.
#[derive(Deserialize)]
pub(crate) struct LoginData {
    pub(crate) token: String,
}

/// `data` of `GET /series/{id}/episodes/{season-type}/{lang}`.
#[derive(Debug, Deserialize)]
pub(crate) struct EpisodesData {
    #[serde(default, deserialize_with = "null_default")]
    pub(crate) episodes: Vec<TvdbEpisode>,
}

// ── Shared records ────────────────────────────────────────────────────────────

/// An identifier of a record in another database (IMDb, TMDB, ...).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbRemoteId {
    /// The id in the other database's own format (e.g. `"tt0903747"`).
    pub id: String,
    /// TheTVDB's numeric source type.
    #[serde(rename = "type", default)]
    pub source_type: Option<i64>,
    /// The other database's name as TheTVDB spells it (e.g. `"IMDB"`,
    /// `"TheMovieDB.com"`).
    #[serde(rename = "sourceName", default)]
    pub source_name: Option<String>,
}

/// A genre.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbGenre {
    /// TheTVDB genre id.
    pub id: u64,
    /// Genre name (e.g. `"Drama"`).
    pub name: String,
    /// URL slug.
    #[serde(default)]
    pub slug: Option<String>,
}

/// One piece of artwork (poster, background, banner, ...).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvdbArtwork {
    /// TheTVDB artwork id.
    pub id: u64,
    /// Full image URL.
    pub image: String,
    /// Full thumbnail URL.
    #[serde(default)]
    pub thumbnail: Option<String>,
    /// Language of any text in the image (ISO 639-2), if it has any.
    #[serde(default)]
    pub language: Option<String>,
    /// TheTVDB artwork type (series: `1` banner, `2` poster, `3` background;
    /// movies: `14` poster, `15` background).
    #[serde(rename = "type")]
    pub artwork_type: i64,
    /// TheTVDB's ranking score; higher ranks first.
    #[serde(default)]
    pub score: Option<f64>,
    /// Width in pixels.
    #[serde(default)]
    pub width: Option<u32>,
    /// Height in pixels.
    #[serde(default)]
    pub height: Option<u32>,
}

/// A content (age) rating in one country.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbContentRating {
    /// TheTVDB rating id.
    pub id: u64,
    /// The rating (e.g. `"TV-MA"`).
    pub name: String,
    /// Country the rating applies in (ISO 3166-1 alpha-3, lowercase).
    #[serde(default)]
    pub country: Option<String>,
    /// What the rating means.
    #[serde(default)]
    pub description: Option<String>,
    /// The kind of content rated (e.g. `"episode"`, `"movie"`).
    #[serde(rename = "contentType", default)]
    pub content_type: Option<String>,
    /// The rating's long form.
    #[serde(rename = "fullname", default)]
    pub full_name: Option<String>,
}

/// A record's release status.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbStatus {
    /// TheTVDB status id.
    #[serde(default)]
    pub id: Option<i64>,
    /// Status name (series: `"Continuing"`, `"Ended"`, `"Upcoming"`; movies:
    /// `"Released"`, ...).
    #[serde(default)]
    pub name: Option<String>,
}

/// An alias (alternative name) in one language.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbAlias {
    /// Language (ISO 639-2).
    #[serde(default)]
    pub language: Option<String>,
    /// The alias.
    pub name: String,
}

/// One translation of a record's name or overview.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbTranslation {
    /// Language (ISO 639-2).
    pub language: String,
    /// The translated name (name translations only).
    #[serde(default)]
    pub name: Option<String>,
    /// The translated overview (overview translations only).
    #[serde(default)]
    pub overview: Option<String>,
    /// The translated tagline, when TheTVDB has one.
    #[serde(default)]
    pub tagline: Option<String>,
    /// Whether this is the record's primary translation.
    #[serde(rename = "isPrimary", default)]
    pub is_primary: Option<bool>,
    /// Whether this name is an alias rather than a translation.
    #[serde(rename = "isAlias", default)]
    pub is_alias: Option<bool>,
}

/// A record's translations, as `meta=translations` returns them.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbTranslations {
    /// Every name translation.
    #[serde(
        rename = "nameTranslations",
        default,
        deserialize_with = "null_default"
    )]
    pub name_translations: Vec<TvdbTranslation>,
    /// Every overview translation.
    #[serde(
        rename = "overviewTranslations",
        default,
        deserialize_with = "null_default"
    )]
    pub overview_translations: Vec<TvdbTranslation>,
    /// Aliases with no language.
    #[serde(default, deserialize_with = "null_default")]
    pub alias: Vec<String>,
}

impl TvdbTranslations {
    /// The name translation in `language` (ISO 639-2), if there is one.
    pub fn name(&self, language: &str) -> Option<&str> {
        self.name_translations
            .iter()
            .filter(|t| t.language == language && t.is_alias != Some(true))
            .find_map(|t| t.name.as_deref().filter(|n| !n.trim().is_empty()))
    }

    /// The overview translation in `language` (ISO 639-2), if there is one.
    pub fn overview(&self, language: &str) -> Option<&str> {
        self.overview_translations
            .iter()
            .filter(|t| t.language == language)
            .find_map(|t| t.overview.as_deref().filter(|o| !o.trim().is_empty()))
    }
}

/// A company (network, studio, production company, ...).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbCompany {
    /// TheTVDB company id.
    pub id: u64,
    /// Company name.
    pub name: String,
    /// Country (ISO 3166-1 alpha-3, lowercase).
    #[serde(default)]
    pub country: Option<String>,
}
