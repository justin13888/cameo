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

// Enable `#[doc(cfg(...))]` feature badges on docs.rs (nightly-only; inert on
// stable). docs.rs passes `--cfg docsrs` via the metadata below.
#![cfg_attr(docsrs, feature(doc_cfg))]

// cameo is useless without a provider to talk to, so require at least one. The
// default `full` feature enables both `tmdb` and `anilist`.
#[cfg(not(any(feature = "tmdb", feature = "anilist")))]
compile_error!(
    "cameo needs at least one provider feature: enable `tmdb` and/or `anilist` \
     (the default `full` feature enables both)."
);

/// Auto-generated low-level API client code (from progenitor).
///
/// Crate-internal: these progenitor/typify-generated types are an
/// implementation detail and are **not** part of cameo's public API. The
/// stable boundary is [`TmdbClient`](providers::tmdb::TmdbClient), which returns
/// unified models and hand-owned TMDB types.
#[cfg(feature = "tmdb")]
pub(crate) mod generated;

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
pub use core::error::ProviderError;
pub use core::{
    config::TimeWindow,
    pagination::{Page, PageToken, into_stream},
};

/// Re-exported so out-of-crate implementers of the `#[async_trait]` provider
/// and `CacheBackend` traits don't need their own
/// `async-trait` dependency: use `#[cameo::async_trait::async_trait]`.
pub use async_trait;
#[cfg(feature = "cache")]
pub use cache::{CacheBackend, CacheError, CacheKey, CacheTtlConfig, MediaType, SqliteCache};
#[cfg(feature = "anilist")]
pub use providers::anilist::error::{AniListErrorLocation, AniListGqlError};
#[cfg(feature = "anilist")]
pub use providers::anilist::{AniListClient, AniListConfig, AniListError};
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub use providers::rate_limit::RateLimit;
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub use providers::retry::RetryPolicy;
#[cfg(feature = "tmdb")]
pub use providers::tmdb::{TmdbAuth, TmdbClient, TmdbConfig, TmdbError};
pub use unified::{
    AlternativeTitle, CameoClient, CameoClientBuilder, CameoClientError, Capability, Collection,
    Company, Creator, DetailProvider, DiscoveryProvider, Gender, Genre, MediaId, MediaProvider,
    NativeId, Network, ParseDateError, PartialDate, Provider, RecommendationProvider,
    SearchProvider, SeasonProvider, TitleKind, UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails,
    UnifiedPerson, UnifiedPersonDetails, UnifiedSearchResult, UnifiedSeason, UnifiedSeasonDetails,
    UnifiedStreamingService, UnifiedTvShow, UnifiedTvShowDetails, UnifiedWatchProviderEntry,
    UnifiedWatchProviders, UnknownGenre, WatchAvailabilityProvider,
};
