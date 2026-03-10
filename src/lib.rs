//! # cameo
//!
//! Unified movie/TV show database SDK for Rust.
//!
//! ## Quick Start
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "tmdb")]
//! # {
//! use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
//! use cameo::unified::{CameoClient, SearchProvider};
//!
//! // Low-level TMDB client
//! let client = TmdbClient::new(TmdbConfig::new("your-tmdb-token"))?;
//! let results = client.search_movies("Inception", None).await?;
//!
//! // High-level unified facade
//! let cameo = CameoClient::builder()
//!     .with_tmdb(TmdbConfig::new("your-tmdb-token"))
//!     .build()?;
//! let movies = cameo.search_movies("Dune", None).await?;
//! # }
//! # Ok(())
//! # }
//! ```
//!
//! ## AniList (anime/manga)
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(feature = "anilist")]
//! # {
//! use cameo::providers::anilist::{AniListClient, AniListConfig};
//! use cameo::unified::{CameoClient, SearchProvider};
//!
//! // Low-level AniList client (no auth required)
//! let client = AniListClient::new(AniListConfig::new())?;
//! let results = client.search_movies("Your Name", None).await?;
//!
//! // High-level unified facade (AniList only)
//! let cameo = CameoClient::builder()
//!     .with_anilist(AniListConfig::new())
//!     .build()?;
//! let anime = cameo.search_tv_shows("Attack on Titan", None).await?;
//! # }
//! # Ok(())
//! # }
//! ```

/// Auto-generated low-level API client code (from progenitor).
pub mod generated;

/// Caching layer for transparent API response caching.
#[cfg(feature = "cache")]
pub mod cache;

/// Shared core types: pagination, configuration, errors.
pub mod core;

/// Provider implementations.
pub mod providers;

/// Unified cross-provider types, traits, and facade client.
pub mod unified;

/// Re-export the most common types.
pub use core::error::CameoError;
pub use core::{config::TimeWindow, pagination::PaginatedResponse};

#[cfg(feature = "cache")]
pub use cache::{CacheBackend, CacheError, CacheTtlConfig, SqliteCache};
#[cfg(feature = "anilist")]
pub use providers::anilist::error::AniListGqlError;
#[cfg(feature = "anilist")]
pub use providers::anilist::{AniListClient, AniListConfig, AniListError};
#[cfg(feature = "tmdb")]
pub use providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
pub use unified::{
    CameoClient, CameoClientBuilder, CameoClientError, DetailProvider, DiscoveryProvider, Genre,
    MediaProvider, RecommendationProvider, SearchProvider, SeasonProvider, UnifiedEpisode,
    UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails, UnifiedSearchResult,
    UnifiedSeasonDetails, UnifiedStreamingService, UnifiedTvShow, UnifiedTvShowDetails,
    UnifiedWatchProviderEntry, UnifiedWatchProviders, UnknownGenre, WatchProviderTrait,
};
