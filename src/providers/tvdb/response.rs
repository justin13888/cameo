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

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

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

// ── Search ────────────────────────────────────────────────────────────────────

/// One `GET /search` result.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbSearchResult {
    /// The record's TheTVDB id, as a decimal string.
    #[serde(default)]
    pub tvdb_id: Option<String>,
    /// The record's kind (`"series"`, `"movie"`, `"person"`, `"company"`, ...).
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    /// Name in the record's original language.
    #[serde(default)]
    pub name: Option<String>,
    /// URL slug.
    #[serde(default)]
    pub slug: Option<String>,
    /// Year of first release (e.g. `"2008"`).
    #[serde(default)]
    pub year: Option<String>,
    /// First air or release date (`YYYY-MM-DD`).
    #[serde(default)]
    pub first_air_time: Option<String>,
    /// Full poster URL.
    #[serde(default)]
    pub image_url: Option<String>,
    /// Overview in the record's original language.
    #[serde(default)]
    pub overview: Option<String>,
    /// Overviews keyed by language (ISO 639-2).
    #[serde(default, deserialize_with = "null_default")]
    pub overviews: HashMap<String, String>,
    /// Names keyed by language (ISO 639-2).
    #[serde(default, deserialize_with = "null_default")]
    pub translations: HashMap<String, String>,
    /// The record's original language (ISO 639-2).
    #[serde(default)]
    pub primary_language: Option<String>,
    /// Aliases.
    #[serde(default, deserialize_with = "null_default")]
    pub aliases: Vec<String>,
    /// Country of origin (ISO 3166-1 alpha-3, lowercase).
    #[serde(default)]
    pub country: Option<String>,
    /// Genre names.
    #[serde(default, deserialize_with = "null_default")]
    pub genres: Vec<String>,
    /// Status name.
    #[serde(default)]
    pub status: Option<String>,
    /// Ids in other databases.
    #[serde(default, deserialize_with = "null_default")]
    pub remote_ids: Vec<TvdbRemoteId>,
}

// ── Series ────────────────────────────────────────────────────────────────────

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

/// A series, as `GET /series/{id}/extended?meta=translations&short=true`
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

// ── Movies ────────────────────────────────────────────────────────────────────

/// A release of a movie in one country.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbRelease {
    /// Country (ISO 3166-1 alpha-3, lowercase, or `"global"`).
    #[serde(default)]
    pub country: Option<String>,
    /// Release date (`YYYY-MM-DD`).
    #[serde(default)]
    pub date: Option<String>,
    /// Free-text detail (e.g. a festival name).
    #[serde(default)]
    pub detail: Option<String>,
}

/// The companies credited on a movie, by role.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbMovieCompanies {
    /// Studios.
    #[serde(default, deserialize_with = "null_default")]
    pub studio: Vec<TvdbCompany>,
    /// Networks.
    #[serde(default, deserialize_with = "null_default")]
    pub network: Vec<TvdbCompany>,
    /// Production companies.
    #[serde(default, deserialize_with = "null_default")]
    pub production: Vec<TvdbCompany>,
    /// Distributors.
    #[serde(default, deserialize_with = "null_default")]
    pub distributor: Vec<TvdbCompany>,
}

/// A production country of a movie.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvdbProductionCountry {
    /// Country code (ISO 3166-1 alpha-3, lowercase).
    #[serde(default)]
    pub country: Option<String>,
    /// Country name.
    pub name: String,
}

/// A movie, as `GET /movies/{id}/extended?meta=translations&short=true`
/// returns it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvdbMovie {
    /// TheTVDB movie id.
    pub id: u64,
    /// Name in the original language.
    pub name: String,
    /// URL slug.
    #[serde(default)]
    pub slug: Option<String>,
    /// Full poster URL.
    #[serde(default)]
    pub image: Option<String>,
    /// Year of first release.
    #[serde(default)]
    pub year: Option<String>,
    /// TheTVDB popularity score.
    #[serde(default)]
    pub score: Option<f64>,
    /// Runtime in minutes.
    #[serde(default)]
    pub runtime: Option<u32>,
    /// Release status.
    #[serde(default)]
    pub status: Option<TvdbStatus>,
    /// Country of origin (ISO 3166-1 alpha-3, lowercase).
    #[serde(rename = "originalCountry", default)]
    pub original_country: Option<String>,
    /// Original language (ISO 639-2).
    #[serde(rename = "originalLanguage", default)]
    pub original_language: Option<String>,
    /// Production budget, in the currency TheTVDB records (usually USD).
    #[serde(default, deserialize_with = "lenient_u64")]
    pub budget: Option<u64>,
    /// Worldwide box office, in the currency TheTVDB records (usually USD).
    #[serde(rename = "boxOffice", default, deserialize_with = "lenient_u64")]
    pub box_office: Option<u64>,
    /// The earliest release.
    #[serde(default)]
    pub first_release: Option<TvdbRelease>,
    /// Companies by role.
    #[serde(default, deserialize_with = "null_default")]
    pub companies: TvdbMovieCompanies,
    /// Production countries.
    #[serde(default, deserialize_with = "null_default")]
    pub production_countries: Vec<TvdbProductionCountry>,
    /// Spoken languages (ISO 639-2).
    #[serde(default, deserialize_with = "null_default")]
    pub spoken_languages: Vec<String>,
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
    /// Name and overview translations.
    #[serde(default, deserialize_with = "null_default")]
    pub translations: TvdbTranslations,
}

// ── Episodes ──────────────────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn episode_reads_absolute_numbers_and_flags() {
        let ep: TvdbEpisode = serde_json::from_value(json!({
            "id": 1, "seriesId": 9, "seasonNumber": 2, "number": 3,
            "absoluteNumber": 15, "isMovie": 0, "name": "Ep"
        }))
        .expect("episode");
        assert_eq!(ep.absolute_number, Some(15));
        assert_eq!(ep.is_movie, Some(false));
        assert!(!ep.is_special());

        let special: TvdbEpisode = serde_json::from_value(json!({
            "id": 2, "seasonNumber": 0, "number": 1, "absoluteNumber": 0,
            "isMovie": 1
        }))
        .expect("special");
        assert_eq!(special.absolute_number, None);
        assert_eq!(special.is_movie, Some(true));
        assert!(special.is_special());
    }

    #[test]
    fn null_lists_and_string_amounts_read_cleanly() {
        let movie: TvdbMovie = serde_json::from_value(json!({
            "id": 1, "name": "M", "genres": null, "artworks": null,
            "remoteIds": null, "companies": null, "translations": null,
            "budget": "160000000", "boxOffice": 5, "spoken_languages": null
        }))
        .expect("movie");
        assert!(movie.genres.is_empty());
        assert_eq!(movie.budget, Some(160_000_000));
        assert_eq!(movie.box_office, Some(5));
        assert_eq!(movie.translations, TvdbTranslations::default());

        let odd: TvdbMovie =
            serde_json::from_value(json!({ "id": 1, "name": "M", "budget": "n/a" }))
                .expect("movie");
        assert_eq!(odd.budget, None);
    }

    #[test]
    fn translations_skip_aliases_and_blanks() {
        let t: TvdbTranslations = serde_json::from_value(json!({
            "nameTranslations": [
                { "language": "eng", "name": "Alias", "isAlias": true },
                { "language": "eng", "name": " " },
                { "language": "eng", "name": "Name" }
            ],
            "overviewTranslations": [{ "language": "jpn", "overview": "概要" }]
        }))
        .expect("translations");
        assert_eq!(t.name("eng"), Some("Name"));
        assert_eq!(t.name("jpn"), None);
        assert_eq!(t.overview("jpn"), Some("概要"));
        assert_eq!(t.overview("eng"), None);
    }
}
