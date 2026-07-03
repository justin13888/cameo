/// Conversion implementations from provider types to unified types.
pub(crate) mod conversions;
/// Multi-provider facade client.
pub mod facade;
/// Genre enum covering all known media genres.
pub mod genre;
/// Typed, provider-qualified media identifiers.
pub mod media_id;
/// Unified data models.
pub mod models;
/// Provider traits.
pub mod traits;

pub use facade::{CameoClient, CameoClientBuilder, CameoClientError};
pub use genre::{Genre, UnknownGenre};
pub use media_id::{MediaId, MediaIdParseError, NativeId};
pub use models::*;
pub use traits::{
    Capability, DetailProvider, DiscoveryProvider, MediaProvider, Provider, RecommendationProvider,
    SearchProvider, SeasonProvider, WatchAvailabilityProvider,
};
