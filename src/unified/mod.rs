/// Conversion implementations from provider types to unified types.
pub mod conversions;
/// Multi-provider facade client.
pub mod facade;
/// Unified data models.
pub mod models;
/// Provider traits.
pub mod traits;

pub use facade::{CameoClient, CameoClientBuilder};
pub use models::*;
pub use traits::{DetailProvider, DiscoveryProvider, MediaProvider, SearchProvider};
