/// Constructors and builder-style setters for the unified models.
mod builders;
/// Conversion implementations from provider types to unified types.
pub(crate) mod conversions;
/// Partial calendar dates for release/air dates.
pub(crate) mod date;
/// Third-party database identifiers (IMDb, TVDB, MyAnimeList, Wikidata).
pub(crate) mod external_ids;
/// Multi-provider facade client.
pub(crate) mod facade;
/// Genre enum covering all known media genres.
pub(crate) mod genre;
/// Typed, provider-qualified media identifiers.
pub(crate) mod media_id;
/// Unified data models.
pub(crate) mod models;
/// Provider traits.
pub(crate) mod traits;

pub use date::{ParseDateError, PartialDate};
pub use external_ids::{
    ExternalId, ExternalIds, ExternalMatch, ExternalSource, MatchKind, UnknownExternalSource,
};
pub use facade::{CameoClient, CameoClientBuilder, CameoClientError};
pub use genre::{Genre, UnknownGenre};
pub use media_id::{MediaId, MediaIdParseError, NativeId};
pub use models::*;
pub use traits::{
    Capability, CreditsProvider, DetailProvider, DiscoveryProvider, ExternalIdProvider,
    MediaProvider, Provider, RecommendationProvider, SearchProvider, SeasonProvider,
    WatchAvailabilityProvider,
};

/// The discovery time window the trending methods take, re-exported from
/// [`crate::core`] so it sits beside the traits that use it.
pub use crate::core::TimeWindow;
