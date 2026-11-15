//! Movie records and their releases, companies, and production countries.

use serde::{Deserialize, Serialize};

use super::{
    TvdbAlias, TvdbArtwork, TvdbCompany, TvdbContentRating, TvdbGenre, TvdbRemoteId, TvdbStatus,
    TvdbTranslations, lenient_u64, null_default,
};

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

/// A movie, as `GET /movies/{id}/extended?meta=translations`
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
