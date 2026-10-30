use std::sync::Arc;

#[cfg(feature = "cache")]
use serde::Serialize;
#[cfg(feature = "cache")]
use serde::de::DeserializeOwned;

#[cfg(feature = "cache")]
use crate::cache::{CacheBackend, CacheKey, CacheTtlConfig, MediaType, SqliteCache};
#[cfg(feature = "anilist")]
use crate::providers::anilist::AniListConfig;
#[cfg(feature = "tmdb")]
use crate::providers::tmdb::TmdbConfig;
use crate::{
    core::error::ProviderError,
    unified::{
        media_id::MediaId,
        traits::{
            DetailProvider, DiscoveryProvider, ExternalIdProvider, Provider,
            RecommendationProvider, SearchProvider, SeasonProvider, WatchAvailabilityProvider,
        },
    },
};

mod detail;
mod discovery;
mod external_ids;
mod recommendation;
mod search;
mod season;
mod watch_providers;

/// Error type returned by [`CameoClient`] methods and [`CameoClientBuilder::build`].
///
/// Distinguishes three situations: [`NotConfigured`](CameoClientError::NotConfigured)
/// (no provider is configured), [`Unsupported`](CameoClientError::Unsupported)
/// (a provider is configured but none supports the requested capability), and a
/// provider-level [`Provider`](CameoClientError::Provider) error.
///
/// # Matching on variants
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "tmdb")]
/// # {
/// use cameo::providers::tmdb::TmdbConfig;
/// use cameo::unified::{CameoClient, CameoClientError};
/// use cameo::ProviderError;
///
/// let client = CameoClient::builder()
///     .with_tmdb(TmdbConfig::new("your-token"))
///     .build()?;
///
/// match client.search_movies("Inception", None).await {
///     Ok(results) => println!("{} results", results.total_results),
///     Err(CameoClientError::Provider(ProviderError::Auth(msg))) => {
///         eprintln!("authentication failed: {msg}");
///     }
///     Err(CameoClientError::Provider(ProviderError::NotFound)) => {
///         eprintln!("not found");
///     }
///     Err(CameoClientError::NotConfigured) => eprintln!("no providers configured"),
///     Err(CameoClientError::Unsupported) => eprintln!("no provider supports this"),
///     Err(e) => eprintln!("error: {e}"),
/// }
/// # }
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CameoClientError {
    /// No providers have been configured (or compiled in).
    #[error("no providers configured")]
    NotConfigured,

    /// A provider is configured, but none supports the requested capability.
    #[error("no configured provider supports this operation")]
    Unsupported,

    /// An error returned by a provider.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

// ── Cache helper ─────────────────────────────────────────────────────────────

#[cfg(feature = "cache")]
struct Cache {
    backend: Arc<dyn CacheBackend>,
    ttl: CacheTtlConfig,
}

#[cfg(feature = "cache")]
impl Cache {
    fn new(backend: Arc<dyn CacheBackend>, ttl: CacheTtlConfig) -> Self {
        Self { backend, ttl }
    }

    async fn get<T: DeserializeOwned>(&self, key: &CacheKey) -> Option<T> {
        match self.backend.get(key).await {
            Ok(Some(v)) => serde_json::from_value(v).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, "cache read failed");
                None
            }
        }
    }

    /// Write-through: serialize and await the write so a subsequent read sees it
    /// (read-your-writes). Errors are logged, not surfaced.
    async fn set<T: Serialize>(&self, key: CacheKey, value: &T, ttl: std::time::Duration) {
        match serde_json::to_value(value) {
            Ok(v) => {
                if let Err(e) = self.backend.set(key, v, ttl).await {
                    tracing::warn!(error = %e, "cache write failed");
                }
            }
            Err(e) => tracing::warn!(error = %e, "cache serialization failed"),
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Builder for constructing a [`CameoClient`].
///
/// Built-in providers are configured with [`with_tmdb`](Self::with_tmdb) /
/// [`with_anilist`](Self::with_anilist). Out-of-crate providers register
/// themselves with the `register_*_provider` methods (one per capability trait
/// they implement) — adding a provider requires no facade changes.
#[derive(Default)]
pub struct CameoClientBuilder {
    #[cfg(feature = "tmdb")]
    tmdb_config: Option<TmdbConfig>,
    #[cfg(feature = "anilist")]
    anilist_config: Option<AniListConfig>,

    ext_search: Vec<Arc<dyn SearchProvider>>,
    ext_detail: Vec<Arc<dyn DetailProvider>>,
    ext_discovery: Vec<Arc<dyn DiscoveryProvider>>,
    ext_recommendation: Vec<Arc<dyn RecommendationProvider>>,
    ext_season: Vec<Arc<dyn SeasonProvider>>,
    ext_watch: Vec<Arc<dyn WatchAvailabilityProvider>>,
    ext_external: Vec<Arc<dyn ExternalIdProvider>>,

    /// Provider priority order (by id); providers not listed keep their default
    /// (registration) order after the prioritized ones.
    priority: Vec<String>,

    #[cfg(feature = "cache")]
    cache_backend: Option<Arc<dyn CacheBackend>>,
    #[cfg(feature = "cache")]
    cache_ttl: Option<CacheTtlConfig>,
}

impl std::fmt::Debug for CameoClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CameoClientBuilder").finish_non_exhaustive()
    }
}

impl CameoClientBuilder {
    /// Configure the TMDB provider.
    #[cfg(feature = "tmdb")]
    pub fn with_tmdb(mut self, config: TmdbConfig) -> Self {
        self.tmdb_config = Some(config);
        self
    }

    /// Configure the AniList provider (no authentication required).
    #[cfg(feature = "anilist")]
    pub fn with_anilist(mut self, config: AniListConfig) -> Self {
        self.anilist_config = Some(config);
        self
    }

    /// Set the default provider priority by id (e.g. `["anilist", "tmdb"]`).
    ///
    /// Providers whose id is not listed keep their registration order after the
    /// prioritized ones. Per-call `*_with` methods override this.
    pub fn with_priority<I, S>(mut self, order: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.priority = order.into_iter().map(Into::into).collect();
        self
    }

    /// Register an out-of-crate search provider.
    pub fn register_search_provider(mut self, provider: Arc<dyn SearchProvider>) -> Self {
        self.ext_search.push(provider);
        self
    }

    /// Register an out-of-crate detail provider.
    pub fn register_detail_provider(mut self, provider: Arc<dyn DetailProvider>) -> Self {
        self.ext_detail.push(provider);
        self
    }

    /// Register an out-of-crate discovery provider.
    pub fn register_discovery_provider(mut self, provider: Arc<dyn DiscoveryProvider>) -> Self {
        self.ext_discovery.push(provider);
        self
    }

    /// Register an out-of-crate recommendation provider.
    pub fn register_recommendation_provider(
        mut self,
        provider: Arc<dyn RecommendationProvider>,
    ) -> Self {
        self.ext_recommendation.push(provider);
        self
    }

    /// Register an out-of-crate season provider.
    pub fn register_season_provider(mut self, provider: Arc<dyn SeasonProvider>) -> Self {
        self.ext_season.push(provider);
        self
    }

    /// Register an out-of-crate watch-availability provider.
    pub fn register_watch_provider(mut self, provider: Arc<dyn WatchAvailabilityProvider>) -> Self {
        self.ext_watch.push(provider);
        self
    }

    /// Register an out-of-crate external-id provider.
    pub fn register_external_id_provider(mut self, provider: Arc<dyn ExternalIdProvider>) -> Self {
        self.ext_external.push(provider);
        self
    }

    /// Enable caching with the default SQLite backend.
    #[cfg(feature = "cache")]
    pub fn with_cache(self) -> Self {
        let path = dirs::cache_dir()
            .map(|d| d.join("cameo").join("cache.db"))
            .unwrap_or_else(|| std::env::temp_dir().join("cameo_cache.db"));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match SqliteCache::new(&path).or_else(|_| SqliteCache::in_memory()) {
            Ok(backend) => self.with_cache_backend(Arc::new(backend)),
            Err(_) => self,
        }
    }

    /// Enable caching with a custom backend.
    #[cfg(feature = "cache")]
    pub fn with_cache_backend(mut self, backend: Arc<dyn CacheBackend>) -> Self {
        self.cache_backend = Some(backend);
        self
    }

    /// Customize cache TTLs.
    #[cfg(feature = "cache")]
    pub fn with_cache_ttl(mut self, ttl: CacheTtlConfig) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    /// Build the `CameoClient`.
    pub fn build(self) -> Result<CameoClient, CameoClientError> {
        let mut client = CameoClient {
            search: Vec::new(),
            detail: Vec::new(),
            discovery: Vec::new(),
            recommendation: Vec::new(),
            season: Vec::new(),
            watch: Vec::new(),
            external: Vec::new(),
            #[cfg(feature = "cache")]
            cache: None,
        };

        // Built-in providers register first (TMDB, then AniList) so the default
        // priority is preserved.
        #[cfg(feature = "tmdb")]
        if let Some(config) = self.tmdb_config {
            let tmdb = Arc::new(
                crate::providers::tmdb::TmdbClient::new(config).map_err(ProviderError::from)?,
            );
            client.search.push(tmdb.clone());
            client.detail.push(tmdb.clone());
            client.discovery.push(tmdb.clone());
            client.recommendation.push(tmdb.clone());
            client.season.push(tmdb.clone());
            client.watch.push(tmdb.clone());
            client.external.push(tmdb);
        }
        #[cfg(feature = "anilist")]
        if let Some(config) = self.anilist_config {
            let anilist = Arc::new(
                crate::providers::anilist::AniListClient::new(config)
                    .map_err(ProviderError::from)?,
            );
            client.search.push(anilist.clone());
            client.detail.push(anilist.clone());
            client.discovery.push(anilist.clone());
            client.external.push(anilist);
        }

        // Out-of-crate providers register last.
        client.search.extend(self.ext_search);
        client.detail.extend(self.ext_detail);
        client.discovery.extend(self.ext_discovery);
        client.recommendation.extend(self.ext_recommendation);
        client.season.extend(self.ext_season);
        client.watch.extend(self.ext_watch);
        client.external.extend(self.ext_external);

        client.apply_priority(&self.priority);

        if !client.any_provider() {
            return Err(CameoClientError::NotConfigured);
        }

        #[cfg(feature = "cache")]
        {
            client.cache = self
                .cache_backend
                .map(|backend| Cache::new(backend, self.cache_ttl.unwrap_or_default()));
        }

        Ok(client)
    }
}

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
/// per-call via the `*_with` methods). Detail/recommendation/season/watch and
/// external-id lookups are **routed by [`MediaId`] origin**: an `anilist:*` id
/// is always answered by AniList and never sent to TMDB. The one exception is
/// [`find_by_external_id`](CameoClient::find_by_external_id), which has no
/// origin to route by: it queries every provider accepting the id's source and
/// merges the matches in priority order. Otherwise, cross-provider result
/// merging or de-duplication is out of scope for v1.
pub struct CameoClient {
    search: Vec<Arc<dyn SearchProvider>>,
    detail: Vec<Arc<dyn DetailProvider>>,
    discovery: Vec<Arc<dyn DiscoveryProvider>>,
    recommendation: Vec<Arc<dyn RecommendationProvider>>,
    season: Vec<Arc<dyn SeasonProvider>>,
    watch: Vec<Arc<dyn WatchAvailabilityProvider>>,
    external: Vec<Arc<dyn ExternalIdProvider>>,
    #[cfg(feature = "cache")]
    cache: Option<Cache>,
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
        match list.first() {
            Some(provider) => Ok(provider),
            None if self.any_provider() => Err(CameoClientError::Unsupported),
            None => Err(CameoClientError::NotConfigured),
        }
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
        if let Some(provider) = list.iter().find(|p| p.id() == id.provider()) {
            return Ok(provider);
        }
        if !self.any_provider() {
            Err(CameoClientError::NotConfigured)
        } else {
            Err(CameoClientError::Unsupported)
        }
    }

    /// Select a provider in `list` by its id (per-call provider override), or
    /// the right "no provider" error.
    fn select<'a, T: Provider + ?Sized>(
        &self,
        list: &'a [Arc<T>],
        provider_id: &str,
    ) -> Result<&'a Arc<T>, CameoClientError> {
        if let Some(provider) = list.iter().find(|p| p.id() == provider_id) {
            return Ok(provider);
        }
        if !self.any_provider() {
            Err(CameoClientError::NotConfigured)
        } else {
            Err(CameoClientError::Unsupported)
        }
    }

    // ── Cache helpers ─────────────────────────────────────────────────────────

    /// Id of the provider that serves search requests (for cache namespacing).
    #[cfg(feature = "cache")]
    fn search_ns(&self) -> String {
        self.require(&self.search)
            .map(|p| p.id().to_string())
            .unwrap_or_default()
    }

    /// Id of the provider that serves discovery requests (for cache namespacing).
    #[cfg(feature = "cache")]
    fn discovery_ns(&self) -> String {
        self.require(&self.discovery)
            .map(|p| p.id().to_string())
            .unwrap_or_default()
    }

    #[cfg(feature = "cache")]
    async fn cache_get<T: DeserializeOwned>(&self, key: &CacheKey) -> Option<T> {
        match &self.cache {
            Some(cache) => cache.get(key).await,
            None => None,
        }
    }

    #[cfg(feature = "cache")]
    async fn cache_put<T: Serialize>(&self, key: CacheKey, value: &T, ttl: std::time::Duration) {
        if let Some(cache) = &self.cache {
            cache.set(key, value, ttl).await;
        }
    }

    #[cfg(feature = "cache")]
    fn cache_search_ttl(&self) -> std::time::Duration {
        self.cache
            .as_ref()
            .map(|c| c.ttl.search)
            .unwrap_or_default()
    }

    #[cfg(feature = "cache")]
    fn cache_items_ttl(&self) -> std::time::Duration {
        self.cache.as_ref().map(|c| c.ttl.items).unwrap_or_default()
    }

    #[cfg(feature = "cache")]
    fn cache_details_ttl(&self) -> std::time::Duration {
        self.cache
            .as_ref()
            .map(|c| c.ttl.details)
            .unwrap_or_default()
    }

    #[cfg(feature = "cache")]
    fn cache_discovery_ttl(&self) -> std::time::Duration {
        self.cache
            .as_ref()
            .map(|c| c.ttl.discovery)
            .unwrap_or_default()
    }

    /// Index each movie in a page by its provider id (item cache).
    #[cfg(feature = "cache")]
    async fn cache_movie_items(
        &self,
        page: &crate::core::pagination::Page<crate::unified::models::UnifiedMovie>,
    ) {
        for item in &page.results {
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id: item.provider_id.to_string(),
                },
                item,
                self.cache_items_ttl(),
            )
            .await;
        }
    }

    /// Index each TV show in a page by its provider id (item cache).
    #[cfg(feature = "cache")]
    async fn cache_tv_items(
        &self,
        page: &crate::core::pagination::Page<crate::unified::models::UnifiedTvShow>,
    ) {
        for item in &page.results {
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id: item.provider_id.to_string(),
                },
                item,
                self.cache_items_ttl(),
            )
            .await;
        }
    }

    // ── Explicit cache lookup API ─────────────────────────────────────────────

    /// Look up a movie from the cache by provider id (e.g. `"tmdb:550"`).
    #[cfg(feature = "cache")]
    pub async fn cached_movie(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedMovie> {
        use crate::unified::models::{UnifiedMovie, UnifiedMovieDetails};
        if let Some(m) = self
            .cache_get::<UnifiedMovie>(&CacheKey::Item {
                media_type: MediaType::Movie,
                provider_id: provider_id.to_string(),
            })
            .await
        {
            return Some(m);
        }
        self.cache_get::<UnifiedMovieDetails>(&CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: provider_id.to_string(),
        })
        .await
        .map(|d| d.movie)
    }

    /// Look up full movie details from the cache by provider id.
    #[cfg(feature = "cache")]
    pub async fn cached_movie_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedMovieDetails> {
        self.cache_get(&CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: provider_id.to_string(),
        })
        .await
    }

    /// Look up a TV show from the cache by provider id.
    #[cfg(feature = "cache")]
    pub async fn cached_tv_show(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedTvShow> {
        use crate::unified::models::{UnifiedTvShow, UnifiedTvShowDetails};
        if let Some(t) = self
            .cache_get::<UnifiedTvShow>(&CacheKey::Item {
                media_type: MediaType::TvShow,
                provider_id: provider_id.to_string(),
            })
            .await
        {
            return Some(t);
        }
        self.cache_get::<UnifiedTvShowDetails>(&CacheKey::Detail {
            media_type: MediaType::TvShow,
            provider_id: provider_id.to_string(),
        })
        .await
        .map(|d| d.show)
    }

    /// Look up full TV show details from the cache by provider id.
    #[cfg(feature = "cache")]
    pub async fn cached_tv_show_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedTvShowDetails> {
        self.cache_get(&CacheKey::Detail {
            media_type: MediaType::TvShow,
            provider_id: provider_id.to_string(),
        })
        .await
    }

    /// Look up a person from the cache by provider id.
    #[cfg(feature = "cache")]
    pub async fn cached_person(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedPerson> {
        use crate::unified::models::{UnifiedPerson, UnifiedPersonDetails};
        if let Some(p) = self
            .cache_get::<UnifiedPerson>(&CacheKey::Item {
                media_type: MediaType::Person,
                provider_id: provider_id.to_string(),
            })
            .await
        {
            return Some(p);
        }
        self.cache_get::<UnifiedPersonDetails>(&CacheKey::Detail {
            media_type: MediaType::Person,
            provider_id: provider_id.to_string(),
        })
        .await
        .map(|d| d.person)
    }

    /// Look up full person details from the cache by provider id.
    #[cfg(feature = "cache")]
    pub async fn cached_person_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedPersonDetails> {
        self.cache_get(&CacheKey::Detail {
            media_type: MediaType::Person,
            provider_id: provider_id.to_string(),
        })
        .await
    }

    /// Invalidate all cache entries for the given provider id.
    #[cfg(feature = "cache")]
    pub async fn invalidate_cached(&self, provider_id: &str) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };
        for mt in [MediaType::Movie, MediaType::TvShow, MediaType::Person] {
            let _ = cache
                .backend
                .invalidate(&CacheKey::Detail {
                    media_type: mt,
                    provider_id: provider_id.to_string(),
                })
                .await;
            let _ = cache
                .backend
                .invalidate(&CacheKey::Item {
                    media_type: mt,
                    provider_id: provider_id.to_string(),
                })
                .await;
        }
    }

    /// Clear all entries from the cache.
    #[cfg(feature = "cache")]
    pub async fn clear_cache(&self) {
        if let Some(cache) = self.cache.as_ref() {
            let _ = cache.backend.clear().await;
        }
    }
}
