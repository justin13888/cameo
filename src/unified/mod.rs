/// Constructors and builder-style setters for the unified models.
mod builders;
/// Conversion implementations from provider types to unified types.
pub(crate) mod conversions;
/// Partial calendar dates for release/air dates.
pub mod date;
/// Third-party database identifiers (IMDb, TVDB, MyAnimeList, Wikidata).
pub mod external_ids;
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

pub use date::{ParseDateError, PartialDate};
pub use external_ids::{
    ExternalId, ExternalIds, ExternalMatch, ExternalSource, MatchKind, UnknownExternalSource,
};
pub use facade::{CameoClient, CameoClientBuilder, CameoClientError};
pub use genre::{Genre, UnknownGenre};
pub use media_id::{MediaId, MediaIdParseError, NativeId};
pub use models::*;
pub use traits::{
    Capability, DetailProvider, DiscoveryProvider, ExternalIdProvider, MediaProvider, Provider,
    RecommendationProvider, SearchProvider, SeasonProvider, WatchAvailabilityProvider,
};
