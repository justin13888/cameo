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
/// Stable, hand-owned TMDB result types (credits, images).
pub mod models;
/// Unified provider-trait implementations.
mod provider;

pub use client::TmdbClient;
pub use config::{TmdbAuth, TmdbConfig};
pub use error::TmdbError;
pub use image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize, StillSize};
pub use models::{CastMember, CrewMember, TmdbCredits, TmdbImage, TmdbImages};
