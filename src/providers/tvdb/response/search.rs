//! `GET /search` results.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{TvdbRemoteId, null_default};

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
