//! TheTVDB (v4) provider: series, movies, and episodes with absolute numbers.
//!
//! [`TvdbClient`](crate::providers::tvdb::TvdbClient) logs in with a project
//! API key (and optional subscriber PIN), keeps the bearer token, and renews
//! it when TheTVDB answers `401`.

/// TheTVDB REST client.
pub(crate) mod client;
/// TheTVDB client configuration.
pub(crate) mod config;
/// Conversions from TheTVDB records to unified models.
mod convert;
/// TheTVDB-specific error types.
pub(crate) mod error;
/// Unified provider-trait implementations.
mod provider;
/// TheTVDB records, as the API returns them.
pub(crate) mod response;

pub use client::{TvdbClient, TvdbSearchType};
pub use config::TvdbConfig;
pub use error::TvdbError;
pub use response::{
    TvdbAlias, TvdbArtwork, TvdbCompany, TvdbContentRating, TvdbEpisode, TvdbGenre, TvdbMovie,
    TvdbMovieCompanies, TvdbProductionCountry, TvdbRelease, TvdbRemoteId, TvdbSearchResult,
    TvdbSeason, TvdbSeasonType, TvdbSeries, TvdbStatus, TvdbTranslation, TvdbTranslations,
};
