use std::sync::Arc;

#[cfg(feature = "cache")]
use super::cache::Cache;
use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheBackend, CacheTtlConfig, SqliteCache};
#[cfg(any(feature = "tmdb", feature = "anilist", feature = "tvdb"))]
use crate::core::error::ProviderError;
#[cfg(feature = "anilist")]
use crate::providers::anilist::AniListConfig;
#[cfg(feature = "tmdb")]
use crate::providers::tmdb::TmdbConfig;
#[cfg(feature = "tvdb")]
use crate::providers::tvdb::TvdbConfig;
use crate::unified::traits::{
    CreditsProvider, DetailProvider, DiscoveryProvider, ExternalIdProvider, RecommendationProvider,
    SearchProvider, SeasonProvider, WatchAvailabilityProvider,
};

/// Builder for constructing a [`CameoClient`].
///
/// Built-in providers are configured with [`with_tmdb`](Self::with_tmdb),
/// [`with_anilist`](Self::with_anilist) and `with_tvdb` (with the `tvdb`
/// feature). Out-of-crate providers register
/// themselves with the `register_*_provider` methods (one per capability trait
/// they implement) — adding a provider requires no facade changes.
#[derive(Default)]
#[must_use = "a CameoClientBuilder does nothing until .build() is called"]
pub struct CameoClientBuilder {
    #[cfg(feature = "tmdb")]
    tmdb_config: Option<TmdbConfig>,
    #[cfg(feature = "anilist")]
    anilist_config: Option<AniListConfig>,
    #[cfg(feature = "tvdb")]
    tvdb_config: Option<TvdbConfig>,

    ext_search: Vec<Arc<dyn SearchProvider>>,
    ext_detail: Vec<Arc<dyn DetailProvider>>,
    ext_discovery: Vec<Arc<dyn DiscoveryProvider>>,
    ext_recommendation: Vec<Arc<dyn RecommendationProvider>>,
    ext_season: Vec<Arc<dyn SeasonProvider>>,
    ext_watch: Vec<Arc<dyn WatchAvailabilityProvider>>,
    ext_external: Vec<Arc<dyn ExternalIdProvider>>,
    ext_credits: Vec<Arc<dyn CreditsProvider>>,

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

    /// Configure the TheTVDB provider, which answers search, details,
    /// seasons and external ids for `tvdb:*` ids.
    ///
    /// It registers after TMDB and AniList, so it answers a search only when
    /// selected by id (`search_*_with("tvdb", ..)`) or ranked first by
    /// [`with_priority`](Self::with_priority).
    #[cfg(feature = "tvdb")]
    pub fn with_tvdb(mut self, config: TvdbConfig) -> Self {
        self.tvdb_config = Some(config);
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
    pub fn register_watch_availability_provider(
        mut self,
        provider: Arc<dyn WatchAvailabilityProvider>,
    ) -> Self {
        self.ext_watch.push(provider);
        self
    }

    /// Register an out-of-crate external-id provider.
    pub fn register_external_id_provider(mut self, provider: Arc<dyn ExternalIdProvider>) -> Self {
        self.ext_external.push(provider);
        self
    }

    /// Register an out-of-crate credits (cast and crew) provider.
    pub fn register_credits_provider(mut self, provider: Arc<dyn CreditsProvider>) -> Self {
        self.ext_credits.push(provider);
        self
    }

    /// Enable caching with the default SQLite backend.
    ///
    /// The database lives in the platform cache directory (or the temporary
    /// directory when there is none). If it cannot be opened, an in-memory
    /// cache is used instead; if that fails too, caching stays disabled. Each
    /// fallback logs a `tracing` warning rather than failing the builder.
    #[cfg(feature = "cache")]
    pub fn with_cache(self) -> Self {
        let path = dirs::cache_dir()
            .map(|d| d.join("cameo").join("cache.db"))
            .unwrap_or_else(|| std::env::temp_dir().join("cameo_cache.db"));
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(error = %e, dir = %parent.display(), "cache directory creation failed");
        }
        let backend = SqliteCache::new(&path).or_else(|e| {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "cache database open failed; falling back to an in-memory cache"
            );
            SqliteCache::in_memory()
        });
        match backend {
            Ok(backend) => self.with_cache_backend(Arc::new(backend)),
            Err(e) => {
                tracing::warn!(error = %e, "in-memory cache creation failed; caching disabled");
                self
            }
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
            credits: Vec::new(),
            #[cfg(feature = "cache")]
            cache: None,
        };

        // Built-in providers register first (TMDB, AniList, then TheTVDB) so the default
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
            client.external.push(tmdb.clone());
            client.credits.push(tmdb);
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
            client.recommendation.push(anilist.clone());
            client.external.push(anilist.clone());
            client.credits.push(anilist);
        }
        #[cfg(feature = "tvdb")]
        if let Some(config) = self.tvdb_config {
            let tvdb = Arc::new(
                crate::providers::tvdb::TvdbClient::new(config).map_err(ProviderError::from)?,
            );
            client.search.push(tvdb.clone());
            client.detail.push(tvdb.clone());
            client.season.push(tvdb.clone());
            client.external.push(tvdb);
        }

        // Out-of-crate providers register last.
        client.search.extend(self.ext_search);
        client.detail.extend(self.ext_detail);
        client.discovery.extend(self.ext_discovery);
        client.recommendation.extend(self.ext_recommendation);
        client.season.extend(self.ext_season);
        client.watch.extend(self.ext_watch);
        client.external.extend(self.ext_external);
        client.credits.extend(self.ext_credits);

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
