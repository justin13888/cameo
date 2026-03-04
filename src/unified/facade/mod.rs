#[cfg(feature = "cache")]
use std::sync::Arc;

#[cfg(feature = "cache")]
use serde::Serialize;
#[cfg(feature = "cache")]
use serde::de::DeserializeOwned;

#[cfg(feature = "cache")]
use crate::cache::{CacheBackend, CacheError, CacheKey, CacheTtlConfig, MediaType, SqliteCache};
#[cfg(feature = "anilist")]
use crate::providers::anilist::{AniListClient, AniListConfig};
#[cfg(feature = "tmdb")]
use crate::providers::tmdb::{TmdbClient, TmdbConfig};

mod detail;
mod discovery;
mod search;

/// Error type for the `CameoClient` facade.
#[derive(Debug, thiserror::Error)]
pub enum CameoClientError {
    /// No providers have been configured.
    #[error("no providers configured")]
    NoProviders,

    /// Error from the TMDB provider.
    #[cfg(feature = "tmdb")]
    #[error(transparent)]
    Tmdb(#[from] crate::providers::tmdb::TmdbError),

    /// Error from the AniList provider.
    #[cfg(feature = "anilist")]
    #[error(transparent)]
    AniList(#[from] crate::providers::anilist::AniListError),

    /// Cache error (non-fatal; logged but does not fail the request).
    #[cfg(feature = "cache")]
    #[error("cache error: {0}")]
    Cache(#[from] CacheError),
}

// ── Cache helper ─────────────────────────────────────────────────────────────

#[cfg(feature = "cache")]
struct Cache {
    backend: Arc<dyn CacheBackend>,
    ttl: CacheTtlConfig,
}

#[cfg(feature = "cache")]
impl Cache {
    async fn get<T: DeserializeOwned>(&self, key: &CacheKey) -> Option<T> {
        match self.backend.get(key).await {
            Ok(Some(v)) => serde_json::from_value(v).ok(),
            _ => None,
        }
    }

    async fn set<T: Serialize>(&self, key: CacheKey, value: &T, ttl: std::time::Duration) {
        if let Ok(v) = serde_json::to_value(value) {
            let _ = self.backend.set(key, v, ttl).await;
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Builder for constructing a [`CameoClient`].
#[derive(Default)]
pub struct CameoClientBuilder {
    #[cfg(feature = "tmdb")]
    tmdb_config: Option<TmdbConfig>,

    #[cfg(feature = "anilist")]
    anilist_config: Option<AniListConfig>,

    #[cfg(feature = "cache")]
    cache_backend: Option<Arc<dyn CacheBackend>>,

    #[cfg(feature = "cache")]
    cache_ttl: Option<CacheTtlConfig>,
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

    /// Enable caching with the default SQLite backend.
    ///
    /// The database is stored in the OS cache directory under
    /// `cameo/cache.db`, falling back to a temporary file.
    #[cfg(feature = "cache")]
    pub fn with_cache(self) -> Self {
        let path = dirs::cache_dir()
            .map(|d| d.join("cameo").join("cache.db"))
            .unwrap_or_else(|| std::env::temp_dir().join("cameo_cache.db"));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match SqliteCache::new(&path) {
            Ok(backend) => self.with_cache_backend(Arc::new(backend)),
            Err(_) => {
                // Fall back to in-memory if file-based creation fails.
                match SqliteCache::in_memory() {
                    Ok(backend) => self.with_cache_backend(Arc::new(backend)),
                    Err(_) => self,
                }
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
        #[cfg(feature = "tmdb")]
        let tmdb = self
            .tmdb_config
            .map(TmdbClient::new)
            .transpose()
            .map_err(CameoClientError::Tmdb)?;

        #[cfg(not(feature = "tmdb"))]
        let tmdb: Option<()> = None;

        #[cfg(feature = "anilist")]
        let anilist = self.anilist_config.map(AniListClient::new);

        #[cfg(not(feature = "anilist"))]
        let anilist: Option<()> = None;

        if tmdb.is_none() && anilist.is_none() {
            return Err(CameoClientError::NoProviders);
        }

        #[cfg(feature = "cache")]
        let cache = self.cache_backend.map(|backend| Cache {
            backend,
            ttl: self.cache_ttl.unwrap_or_default(),
        });

        Ok(CameoClient {
            #[cfg(feature = "tmdb")]
            tmdb,
            #[cfg(feature = "anilist")]
            anilist,
            #[cfg(feature = "cache")]
            cache,
        })
    }
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Multi-provider facade client.
///
/// Use [`CameoClientBuilder`] to construct one.
///
/// # Provider Priority
///
/// When multiple providers are configured, TMDB is used first (if configured).
/// AniList is used as a fallback when TMDB is not configured.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "tmdb")]
/// # {
/// use cameo::providers::tmdb::TmdbConfig;
/// use cameo::unified::{CameoClient, SearchProvider};
///
/// let client = CameoClient::builder()
///     .with_tmdb(TmdbConfig::new("your-token"))
///     .build()?;
///
/// let results = client.search_movies("Inception", None).await?;
/// # }
/// # Ok(())
/// # }
/// ```
pub struct CameoClient {
    #[cfg(feature = "tmdb")]
    pub(self) tmdb: Option<TmdbClient>,

    #[cfg(feature = "anilist")]
    pub(self) anilist: Option<AniListClient>,

    #[cfg(feature = "cache")]
    pub(self) cache: Option<Cache>,
}

impl CameoClient {
    /// Create a new builder.
    pub fn builder() -> CameoClientBuilder {
        CameoClientBuilder::default()
    }

    /// Access the underlying TMDB client, if configured.
    #[cfg(feature = "tmdb")]
    pub fn tmdb(&self) -> Option<&TmdbClient> {
        self.tmdb.as_ref()
    }

    /// Access the underlying AniList client, if configured.
    #[cfg(feature = "anilist")]
    pub fn anilist(&self) -> Option<&AniListClient> {
        self.anilist.as_ref()
    }

    // ── Explicit cache lookup API ─────────────────────────────────────────────

    /// Look up a movie from the cache by provider_id (e.g. `"tmdb:550"` or `"anilist:1"`).
    ///
    /// Checks the Item cache first (populated by search/discovery results),
    /// then falls back to extracting the base movie from the Detail cache.
    #[cfg(feature = "cache")]
    pub async fn cached_movie(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedMovie> {
        use crate::unified::models::UnifiedMovieDetails;
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::Movie,
            provider_id: provider_id.to_string(),
        };
        if let Some(m) = cache
            .get::<crate::unified::models::UnifiedMovie>(&item_key)
            .await
        {
            return Some(m);
        }
        let detail_key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: provider_id.to_string(),
        };
        cache
            .get::<UnifiedMovieDetails>(&detail_key)
            .await
            .map(|d| d.movie)
    }

    /// Look up full movie details from the cache by provider_id.
    ///
    /// Only available if [`CameoClient::movie_details`] was previously called
    /// for this provider_id.
    #[cfg(feature = "cache")]
    pub async fn cached_movie_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedMovieDetails> {
        let cache = self.cache.as_ref()?;
        cache
            .get(&CacheKey::Detail {
                media_type: MediaType::Movie,
                provider_id: provider_id.to_string(),
            })
            .await
    }

    /// Look up a TV show from the cache by provider_id.
    #[cfg(feature = "cache")]
    pub async fn cached_tv_show(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedTvShow> {
        use crate::unified::models::UnifiedTvShowDetails;
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::TvShow,
            provider_id: provider_id.to_string(),
        };
        if let Some(t) = cache
            .get::<crate::unified::models::UnifiedTvShow>(&item_key)
            .await
        {
            return Some(t);
        }
        let detail_key = CacheKey::Detail {
            media_type: MediaType::TvShow,
            provider_id: provider_id.to_string(),
        };
        cache
            .get::<UnifiedTvShowDetails>(&detail_key)
            .await
            .map(|d| d.show)
    }

    /// Look up full TV show details from the cache by provider_id.
    #[cfg(feature = "cache")]
    pub async fn cached_tv_show_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedTvShowDetails> {
        let cache = self.cache.as_ref()?;
        cache
            .get(&CacheKey::Detail {
                media_type: MediaType::TvShow,
                provider_id: provider_id.to_string(),
            })
            .await
    }

    /// Look up a person from the cache by provider_id.
    #[cfg(feature = "cache")]
    pub async fn cached_person(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedPerson> {
        use crate::unified::models::UnifiedPersonDetails;
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::Person,
            provider_id: provider_id.to_string(),
        };
        if let Some(p) = cache
            .get::<crate::unified::models::UnifiedPerson>(&item_key)
            .await
        {
            return Some(p);
        }
        let detail_key = CacheKey::Detail {
            media_type: MediaType::Person,
            provider_id: provider_id.to_string(),
        };
        cache
            .get::<UnifiedPersonDetails>(&detail_key)
            .await
            .map(|d| d.person)
    }

    /// Look up full person details from the cache by provider_id.
    #[cfg(feature = "cache")]
    pub async fn cached_person_details(
        &self,
        provider_id: &str,
    ) -> Option<crate::unified::models::UnifiedPersonDetails> {
        let cache = self.cache.as_ref()?;
        cache
            .get(&CacheKey::Detail {
                media_type: MediaType::Person,
                provider_id: provider_id.to_string(),
            })
            .await
    }

    /// Invalidate all cache entries for the given provider_id.
    #[cfg(feature = "cache")]
    pub async fn invalidate_cached(&self, provider_id: &str) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };
        for mt in [MediaType::Movie, MediaType::TvShow, MediaType::Person] {
            let pid = provider_id.to_string();
            let _ = cache
                .backend
                .invalidate(&CacheKey::Detail {
                    media_type: mt,
                    provider_id: pid.clone(),
                })
                .await;
            let _ = cache
                .backend
                .invalidate(&CacheKey::Item {
                    media_type: mt,
                    provider_id: pid,
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
