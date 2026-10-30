//! Cross-database identifiers (IMDb, TVDB, MyAnimeList, Wikidata) and the
//! types returned by [`ExternalIdProvider`](crate::unified::ExternalIdProvider).
//!
//! An [`ExternalId`] names a title in a third-party database and is the input
//! to a find-by-external-id lookup; [`ExternalIds`] collects every known
//! identifier for one title; [`ExternalMatch`] is one lookup result, tagged
//! with whether it is a movie or a TV show (TMDB movie and TV ids share a
//! numeric space, so the bare [`MediaId`] is not enough to fetch it).
//!
//! ```
//! use cameo::{ExternalId, ExternalIds, ExternalSource};
//!
//! let id = ExternalId::imdb("tt0137523");
//! assert_eq!(id.source, ExternalSource::Imdb);
//! assert_eq!(id.to_string(), "imdb:tt0137523");
//!
//! let ids = ExternalIds::new().with_tmdb(550).with_imdb("tt0137523");
//! assert_eq!(ids.get(ExternalSource::Imdb), Some(id));
//! ```

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::media_id::MediaId;

/// A third-party database that cameo can map identifiers to and from.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSource {
    /// IMDb (`tt…` title ids).
    Imdb,
    /// TheTVDB (numeric series ids).
    Tvdb,
    /// MyAnimeList (numeric anime ids).
    Mal,
    /// Wikidata (`Q…` entity ids).
    Wikidata,
}

impl ExternalSource {
    /// The lowercase tag used by [`Display`](fmt::Display) and
    /// [`FromStr`]: `"imdb"`, `"tvdb"`, `"mal"`, or `"wikidata"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternalSource::Imdb => "imdb",
            ExternalSource::Tvdb => "tvdb",
            ExternalSource::Mal => "mal",
            ExternalSource::Wikidata => "wikidata",
        }
    }
}

impl fmt::Display for ExternalSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string is not a known [`ExternalSource`] tag.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown external source `{0}` (expected imdb, tvdb, mal, or wikidata)")]
pub struct UnknownExternalSource(pub String);

impl FromStr for ExternalSource {
    type Err = UnknownExternalSource;

    /// Parse a source tag case-insensitively (`"imdb"`, `"TVDB"`, …).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "imdb" => Ok(ExternalSource::Imdb),
            "tvdb" => Ok(ExternalSource::Tvdb),
            "mal" => Ok(ExternalSource::Mal),
            "wikidata" => Ok(ExternalSource::Wikidata),
            _ => Err(UnknownExternalSource(s.to_string())),
        }
    }
}

/// An identifier in a third-party database, e.g. IMDb `tt0137523`.
///
/// The id is kept as a string in the source's own format; numeric sources
/// (TVDB, MyAnimeList) store the decimal rendering, see [`as_u64`](Self::as_u64).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalId {
    /// The database the id belongs to.
    pub source: ExternalSource,
    /// The id in the source's own format.
    pub id: String,
}

impl ExternalId {
    /// An id in `source`.
    pub fn new(source: ExternalSource, id: impl Into<String>) -> Self {
        Self {
            source,
            id: id.into(),
        }
    }

    /// An IMDb title id, e.g. `ExternalId::imdb("tt0137523")`.
    pub fn imdb(id: impl Into<String>) -> Self {
        Self::new(ExternalSource::Imdb, id)
    }

    /// A TheTVDB series id.
    pub fn tvdb(id: u64) -> Self {
        Self::new(ExternalSource::Tvdb, id.to_string())
    }

    /// A MyAnimeList anime id.
    pub fn mal(id: u64) -> Self {
        Self::new(ExternalSource::Mal, id.to_string())
    }

    /// A Wikidata entity id, e.g. `ExternalId::wikidata("Q190050")`.
    pub fn wikidata(id: impl Into<String>) -> Self {
        Self::new(ExternalSource::Wikidata, id)
    }

    /// The id parsed as a `u64`, if it is numeric (TVDB and MyAnimeList ids).
    pub fn as_u64(&self) -> Option<u64> {
        self.id.parse().ok()
    }
}

impl fmt::Display for ExternalId {
    /// Renders as `"source:id"`, e.g. `imdb:tt0137523`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.id)
    }
}

/// Every identifier a provider knows for one title.
///
/// Each field is `None` when the provider does not know (or does not track)
/// that database. The answering provider fills in its own id too (`tmdb` for
/// TMDB, `anilist` for AniList), so the value is self-describing.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalIds {
    /// IMDb title id (`tt…`).
    pub imdb: Option<String>,
    /// TheTVDB series id.
    pub tvdb: Option<u64>,
    /// MyAnimeList anime id.
    pub mal: Option<u64>,
    /// Wikidata entity id (`Q…`).
    pub wikidata: Option<String>,
    /// TMDB id.
    pub tmdb: Option<u64>,
    /// AniList media id.
    pub anilist: Option<u64>,
}

impl ExternalIds {
    /// An empty set of ids; fill it with the `with_*` setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the IMDb title id.
    pub fn with_imdb(mut self, value: impl Into<String>) -> Self {
        self.imdb = Some(value.into());
        self
    }
    /// Set the TheTVDB series id.
    pub fn with_tvdb(mut self, value: u64) -> Self {
        self.tvdb = Some(value);
        self
    }
    /// Set the MyAnimeList anime id.
    pub fn with_mal(mut self, value: u64) -> Self {
        self.mal = Some(value);
        self
    }
    /// Set the Wikidata entity id.
    pub fn with_wikidata(mut self, value: impl Into<String>) -> Self {
        self.wikidata = Some(value.into());
        self
    }
    /// Set the TMDB id.
    pub fn with_tmdb(mut self, value: u64) -> Self {
        self.tmdb = Some(value);
        self
    }
    /// Set the AniList media id.
    pub fn with_anilist(mut self, value: u64) -> Self {
        self.anilist = Some(value);
        self
    }

    /// The id for `source` as an [`ExternalId`], if known.
    pub fn get(&self, source: ExternalSource) -> Option<ExternalId> {
        match source {
            ExternalSource::Imdb => self.imdb.clone().map(ExternalId::imdb),
            ExternalSource::Tvdb => self.tvdb.map(ExternalId::tvdb),
            ExternalSource::Mal => self.mal.map(ExternalId::mal),
            ExternalSource::Wikidata => self.wikidata.clone().map(ExternalId::wikidata),
        }
    }
}

/// Whether an [`ExternalMatch`] is a movie or a TV show.
///
/// Tells the caller which detail call to make next
/// ([`movie_details`](crate::unified::DetailProvider::movie_details) or
/// [`tv_show_details`](crate::unified::DetailProvider::tv_show_details)).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchKind {
    /// A movie.
    Movie,
    /// A TV show (for AniList: any non-movie anime format).
    TvShow,
}

/// One result of a find-by-external-id lookup.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalMatch {
    /// The matching title's provider id.
    pub id: MediaId,
    /// Whether the title is a movie or a TV show.
    pub kind: MatchKind,
}

impl ExternalMatch {
    /// A match of the given kind.
    pub fn new(id: MediaId, kind: MatchKind) -> Self {
        Self { id, kind }
    }

    /// A movie match.
    pub fn movie(id: MediaId) -> Self {
        Self::new(id, MatchKind::Movie)
    }

    /// A TV-show match.
    pub fn tv_show(id: MediaId) -> Self {
        Self::new(id, MatchKind::TvShow)
    }
}
