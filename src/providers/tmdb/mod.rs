//! TMDB (The Movie Database) provider.
//!
//! The leaf modules are private; every public item is re-exported here, which
//! is its canonical path (the most common ones are also at the crate root).

/// Discover query builders.
pub(crate) mod builders;
/// High-level TMDB client.
pub(crate) mod client;
/// TMDB client configuration.
pub(crate) mod config;
/// Conversions from generated TMDB response types to cameo's own types.
mod convert;
/// TMDB-specific error types.
pub(crate) mod error;
/// Image URL construction helpers.
pub(crate) mod image_url;
/// Stable, hand-owned TMDB result types (credits, images, certifications).
pub(crate) mod models;
/// Unified provider-trait implementations.
mod provider;

pub use builders::{
    DiscoverMoviesBuilder, DiscoverTvBuilder, MonetizationType, MovieReleaseType, MovieSortBy,
    TvShowType, TvSortBy, TvStatus,
};
pub use client::TmdbClient;
pub use config::{TmdbAuth, TmdbConfig};
pub use error::{RequestErrorKind, TmdbError, TransportError};
pub use image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize, StillSize};
pub use models::{CastMember, CrewMember, TmdbCertification, TmdbCredits, TmdbImage, TmdbImages};
