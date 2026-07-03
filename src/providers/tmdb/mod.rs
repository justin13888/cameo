/// Discover query builders.
pub mod builders;
/// High-level TMDB client.
pub mod client;
/// TMDB client configuration.
pub mod config;
/// TMDB-specific error types.
pub mod error;
/// Image URL construction helpers.
pub mod image_url;
/// Unified provider-trait implementations.
mod provider;

pub use client::TmdbClient;
pub use config::TmdbConfig;
pub use error::TmdbError;
pub use image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize, StillSize};
