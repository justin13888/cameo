//! # cameo
//!
//! Unified movie/TV show database SDK for Rust.
//!
//! ## Quick Start
//!
//! ```no_run
//! use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
//! use cameo::unified::{CameoClient, SearchProvider};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Low-level TMDB client
//! let client = TmdbClient::new(TmdbConfig::new("your-tmdb-token"))?;
//! let results = client.search_movies("Inception", None).await?;
//!
//! // High-level unified facade
//! let cameo = CameoClient::builder()
//!     .with_tmdb(TmdbConfig::new("your-tmdb-token"))
//!     .build()?;
//! let movies = cameo.search_movies("Dune", None).await?;
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
pub use core::pagination::PaginatedResponse;

#[cfg(feature = "cache")]
pub use cache::{CacheBackend, CacheError, CacheTtlConfig, SqliteCache};
#[cfg(feature = "tmdb")]
pub use providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
pub use unified::{CameoClient, CameoClientBuilder};
