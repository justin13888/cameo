/// Conversion implementations from provider types to unified types.
pub mod conversions;
/// Multi-provider facade client.
pub mod facade;
/// Genre enum covering all known TMDB genres.
pub mod genre;
/// Unified data models.
pub mod models;
/// Provider traits.
pub mod traits;

pub use facade::{CameoClient, CameoClientBuilder, CameoClientError};
pub use genre::{Genre, UnknownGenre};
pub use models::*;
pub use traits::{DetailProvider, DiscoveryProvider, MediaProvider, SearchProvider};
