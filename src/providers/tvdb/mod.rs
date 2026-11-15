//! TheTVDB (v4) provider: series, movies, and episodes with absolute numbers.
//!
//! [`TvdbClient`] logs in with a project API key (and optional subscriber
//! PIN), keeps the bearer token, and renews it when TheTVDB answers `401`.

/// TheTVDB REST client.
pub mod client;
/// TheTVDB client configuration.
pub mod config;
/// TheTVDB-specific error types.
pub mod error;
/// Unified provider-trait implementations.
mod provider;
/// TheTVDB records, as the API returns them.
pub mod response;

pub use client::{TvdbClient, TvdbSearchType};
pub use config::TvdbConfig;
pub use error::TvdbError;
pub use response::{
    TvdbAlias, TvdbArtwork, TvdbCompany, TvdbContentRating, TvdbEpisode, TvdbGenre, TvdbMovie,
    TvdbMovieCompanies, TvdbProductionCountry, TvdbRelease, TvdbRemoteId, TvdbSearchResult,
    TvdbSeason, TvdbSeasonType, TvdbSeries, TvdbStatus, TvdbTranslation, TvdbTranslations,
};
