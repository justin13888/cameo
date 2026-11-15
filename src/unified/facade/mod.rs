use std::sync::Arc;

use crate::unified::{
    media_id::MediaId,
    traits::{
        CreditsProvider, DetailProvider, DiscoveryProvider, ExternalIdProvider, Provider,
        RecommendationProvider, SearchProvider, SeasonProvider, WatchAvailabilityProvider,
    },
};

mod builder;
#[cfg(feature = "cache")]
mod cache;
mod credits;
mod detail;
mod discovery;
mod error;
mod external_ids;
mod recommendation;
mod search;
mod season;
mod watch_providers;

pub use builder::CameoClientBuilder;
pub use error::CameoClientError;

// ── Client ────────────────────────────────────────────────────────────────────

/// Multi-provider facade client backed by a capability registry.
///
/// Each capability is dispatched to the first provider that supports it (built-in
/// providers first, then registered ones). A call with no provider configured
/// returns [`CameoClientError::NotConfigured`]; a call no configured provider
/// supports returns [`CameoClientError::Unsupported`].
///
/// # Multi-provider semantics (v1)
///
/// Results are **not merged** across providers. A search/discovery call is
/// answered by a single provider — the highest-priority one supporting the
/// capability (configurable via [`CameoClientBuilder::with_priority`], overridable
/// per-call via the `*_with` methods). Detail/recommendation/season/watch,
/// credits, and external-id lookups are **routed by [`MediaId`] origin**: an `anilist:*` id
/// is always answered by AniList and never sent to TMDB. The one exception is
/// [`find_by_external_id`](CameoClient::find_by_external_id), which has no
/// origin to route by: it queries every provider accepting the id's source and
/// merges the matches in priority order. Otherwise, cross-provider result
/// merging or de-duplication is out of scope for v1.
///
/// Cloning is cheap: every provider and the cache backend are held behind an
/// [`Arc`], so a clone shares them (including rate limiters and the cache)
/// with the original.
#[derive(Clone)]
pub struct CameoClient {
    search: Vec<Arc<dyn SearchProvider>>,
    detail: Vec<Arc<dyn DetailProvider>>,
    discovery: Vec<Arc<dyn DiscoveryProvider>>,
    recommendation: Vec<Arc<dyn RecommendationProvider>>,
    season: Vec<Arc<dyn SeasonProvider>>,
    watch: Vec<Arc<dyn WatchAvailabilityProvider>>,
    external: Vec<Arc<dyn ExternalIdProvider>>,
    credits: Vec<Arc<dyn CreditsProvider>>,
    #[cfg(feature = "cache")]
    cache: Option<cache::Cache>,
}

impl std::fmt::Debug for CameoClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameoClient")
            .field("providers", &self.provider_ids())
            .finish_non_exhaustive()
    }
}

impl CameoClient {
    /// Create a new builder.
    pub fn builder() -> CameoClientBuilder {
        CameoClientBuilder::default()
    }

    /// The distinct provider ids registered, in priority order.
    pub fn provider_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        for p in &self.search {
            if !ids.contains(&p.id()) {
                ids.push(p.id());
            }
        }
        for id in [
            self.detail.iter().map(|p| p.id()).collect::<Vec<_>>(),
            self.discovery.iter().map(|p| p.id()).collect(),
            self.recommendation.iter().map(|p| p.id()).collect(),
            self.season.iter().map(|p| p.id()).collect(),
            self.watch.iter().map(|p| p.id()).collect(),
            self.external.iter().map(|p| p.id()).collect(),
            self.credits.iter().map(|p| p.id()).collect(),
        ]
        .concat()
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    /// Reorder every capability registry by the configured provider priority.
    /// Stable so providers of equal rank keep their registration order.
    fn apply_priority(&mut self, priority: &[String]) {
        if priority.is_empty() {
            return;
        }
        let rank = |id: &str| priority.iter().position(|p| p == id).unwrap_or(usize::MAX);
        self.search.sort_by_key(|p| rank(p.id()));
        self.detail.sort_by_key(|p| rank(p.id()));
        self.discovery.sort_by_key(|p| rank(p.id()));
        self.recommendation.sort_by_key(|p| rank(p.id()));
        self.season.sort_by_key(|p| rank(p.id()));
        self.watch.sort_by_key(|p| rank(p.id()));
        self.external.sort_by_key(|p| rank(p.id()));
        self.credits.sort_by_key(|p| rank(p.id()));
    }

    /// Whether any provider is registered for any capability.
    fn any_provider(&self) -> bool {
        !self.search.is_empty()
            || !self.detail.is_empty()
            || !self.discovery.is_empty()
            || !self.recommendation.is_empty()
            || !self.season.is_empty()
            || !self.watch.is_empty()
            || !self.external.is_empty()
            || !self.credits.is_empty()
    }

    /// The "no provider" error for a capability with no eligible provider:
    /// [`Unsupported`](CameoClientError::Unsupported) if anything is
    /// registered, else [`NotConfigured`](CameoClientError::NotConfigured).
    fn no_provider_error(&self) -> CameoClientError {
        if self.any_provider() {
            CameoClientError::Unsupported
        } else {
            CameoClientError::NotConfigured
        }
    }

    /// Resolve the first provider in `list`, or the right "no provider" error.
    fn require<'a, T: ?Sized>(&self, list: &'a [Arc<T>]) -> Result<&'a Arc<T>, CameoClientError> {
        list.first().ok_or_else(|| self.no_provider_error())
    }

    /// Route to the provider in `list` matching a [`MediaId`]'s origin.
    ///
    /// A lookup is answered by the provider that minted the id (e.g. an
    /// `anilist:*` id is never sent to TMDB). Returns [`Unsupported`] when no
    /// registered provider for this capability matches the id's origin, or
    /// [`NotConfigured`] when nothing is registered at all.
    ///
    /// [`Unsupported`]: CameoClientError::Unsupported
    /// [`NotConfigured`]: CameoClientError::NotConfigured
    fn route<'a, T: Provider + ?Sized>(
        &self,
        list: &'a [Arc<T>],
        id: &MediaId,
    ) -> Result<&'a Arc<T>, CameoClientError> {
        self.select(list, id.provider())
    }

    /// Select a provider in `list` by its id (per-call provider override), or
    /// the right "no provider" error.
    fn select<'a, T: Provider + ?Sized>(
        &self,
        list: &'a [Arc<T>],
        provider_id: &str,
    ) -> Result<&'a Arc<T>, CameoClientError> {
        list.iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| self.no_provider_error())
    }
}
