//! Response caching for [`CameoClient`]: the cache handle, the private
//! read/write helpers the capability modules use, and the public
//! `cached_*` / invalidation API.

use std::{sync::Arc, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

use super::CameoClient;
use crate::{
    cache::{CacheBackend, CacheKey, CacheTtlConfig, MediaType},
    unified::{
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
        traits::Provider,
    },
};

/// The configured cache backend and the TTLs entries are written with.
#[derive(Clone)]
pub(super) struct Cache {
    backend: Arc<dyn CacheBackend>,
    ttl: CacheTtlConfig,
}

impl Cache {
    pub(super) fn new(backend: Arc<dyn CacheBackend>, ttl: CacheTtlConfig) -> Self {
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
    async fn set<T: Serialize>(&self, key: CacheKey, value: &T, ttl: Duration) {
        match serde_json::to_value(value) {
            Ok(v) => {
                if let Err(e) = self.backend.set(key, v, ttl).await {
                    tracing::warn!(error = %e, "cache write failed");
                }
            }
            Err(e) => tracing::warn!(error = %e, "cache serialization failed"),
        }
    }

    /// Remove one entry. Errors are logged, not surfaced.
    async fn invalidate(&self, key: &CacheKey) {
        if let Err(e) = self.backend.invalidate(key).await {
            tracing::warn!(error = %e, "cache invalidation failed");
        }
    }
}

/// The item-cache key for an entry of `media_type` with `provider_id`.
fn item_key(media_type: MediaType, provider_id: &str) -> CacheKey {
    CacheKey::Item {
        media_type,
        provider_id: provider_id.to_string(),
    }
}

/// The detail-cache key for an entry of `media_type` with `provider_id`.
fn detail_key(media_type: MediaType, provider_id: &str) -> CacheKey {
    CacheKey::Detail {
        media_type,
        provider_id: provider_id.to_string(),
    }
}

/// A summary model indexed in the item cache under its provider id.
pub(super) trait CacheItem: Serialize + Sync {
    /// The item-cache key this value is stored under.
    fn item_key(&self) -> CacheKey;
}

impl CacheItem for UnifiedMovie {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::Movie, &self.provider_id.to_string())
    }
}

impl CacheItem for UnifiedTvShow {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::TvShow, &self.provider_id.to_string())
    }
}

impl CacheItem for UnifiedPerson {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::Person, &self.provider_id.to_string())
    }
}

/// A mixed search result is keyed by the media type and id of the value it
/// wraps; the wrapper itself is what gets stored.
impl CacheItem for UnifiedSearchResult {
    fn item_key(&self) -> CacheKey {
        match self {
            UnifiedSearchResult::Movie(m) => m.item_key(),
            UnifiedSearchResult::TvShow(t) => t.item_key(),
            UnifiedSearchResult::Person(p) => p.item_key(),
        }
    }
}

// ── Private helpers used by the capability modules ───────────────────────────

impl CameoClient {
    /// Cache namespace for a capability: the id of the provider that serves it
    /// (the first in `list`), or empty when none is registered.
    pub(super) fn cache_ns<T: Provider + ?Sized>(list: &[Arc<T>]) -> String {
        list.first().map(|p| p.id().to_string()).unwrap_or_default()
    }

    /// Read and deserialize an entry; a miss, a backend error, or a value of the
    /// wrong shape is `None`.
    pub(super) async fn cache_get<T: DeserializeOwned>(&self, key: &CacheKey) -> Option<T> {
        match &self.cache {
            Some(cache) => cache.get(key).await,
            None => None,
        }
    }

    /// Write an entry when a cache is configured.
    pub(super) async fn cache_put<T: Serialize>(&self, key: CacheKey, value: &T, ttl: Duration) {
        if let Some(cache) = &self.cache {
            cache.set(key, value, ttl).await;
        }
    }

    /// The TTL `pick` selects from the configured TTLs (zero with no cache).
    pub(super) fn cache_ttl(&self, pick: impl Fn(&CacheTtlConfig) -> Duration) -> Duration {
        self.cache
            .as_ref()
            .map(|c| pick(&c.ttl))
            .unwrap_or_default()
    }

    /// Index one summary model in the item cache.
    pub(super) async fn cache_item<T: CacheItem>(&self, item: &T) {
        self.cache_put(item.item_key(), item, self.cache_ttl(|t| t.items))
            .await;
    }

    /// Index every summary model in `items` in the item cache.
    pub(super) async fn cache_items<T: CacheItem>(&self, items: &[T]) {
        for item in items {
            self.cache_item(item).await;
        }
    }

    /// Look up a summary model by provider id: the item entry if present,
    /// otherwise the summary `project`ed out of the detail entry.
    async fn cached_with_detail_fallback<I, D>(
        &self,
        media_type: MediaType,
        provider_id: &str,
        project: fn(D) -> I,
    ) -> Option<I>
    where
        I: DeserializeOwned,
        D: DeserializeOwned,
    {
        if let Some(item) = self.cache_get(&item_key(media_type, provider_id)).await {
            return Some(item);
        }
        self.cache_get(&detail_key(media_type, provider_id))
            .await
            .map(project)
    }
}

// ── Explicit cache lookup API ────────────────────────────────────────────────

impl CameoClient {
    /// Look up a movie from the cache by provider id (e.g. `"tmdb:550"`).
    pub async fn cached_movie(&self, provider_id: &str) -> Option<UnifiedMovie> {
        self.cached_with_detail_fallback(MediaType::Movie, provider_id, |d: UnifiedMovieDetails| {
            d.movie
        })
        .await
    }

    /// Look up full movie details from the cache by provider id.
    pub async fn cached_movie_details(&self, provider_id: &str) -> Option<UnifiedMovieDetails> {
        self.cache_get(&detail_key(MediaType::Movie, provider_id))
            .await
    }

    /// Look up a TV show from the cache by provider id.
    pub async fn cached_tv_show(&self, provider_id: &str) -> Option<UnifiedTvShow> {
        self.cached_with_detail_fallback(
            MediaType::TvShow,
            provider_id,
            |d: UnifiedTvShowDetails| d.show,
        )
        .await
    }

    /// Look up full TV show details from the cache by provider id.
    pub async fn cached_tv_show_details(&self, provider_id: &str) -> Option<UnifiedTvShowDetails> {
        self.cache_get(&detail_key(MediaType::TvShow, provider_id))
            .await
    }

    /// Look up a person from the cache by provider id.
    pub async fn cached_person(&self, provider_id: &str) -> Option<UnifiedPerson> {
        self.cached_with_detail_fallback(
            MediaType::Person,
            provider_id,
            |d: UnifiedPersonDetails| d.person,
        )
        .await
    }

    /// Look up full person details from the cache by provider id.
    pub async fn cached_person_details(&self, provider_id: &str) -> Option<UnifiedPersonDetails> {
        self.cache_get(&detail_key(MediaType::Person, provider_id))
            .await
    }

    /// Invalidate all cache entries for the given provider id.
    ///
    /// Backend failures are logged as `tracing` warnings, not surfaced.
    pub async fn invalidate_cached(&self, provider_id: &str) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };
        for mt in [MediaType::Movie, MediaType::TvShow, MediaType::Person] {
            cache.invalidate(&detail_key(mt, provider_id)).await;
            cache.invalidate(&item_key(mt, provider_id)).await;
        }
    }

    /// Clear all entries from the cache.
    ///
    /// Backend failures are logged as a `tracing` warning, not surfaced.
    pub async fn clear_cache(&self) {
        if let Some(cache) = self.cache.as_ref()
            && let Err(e) = cache.backend.clear().await
        {
            tracing::warn!(error = %e, "cache clear failed");
        }
    }
}
