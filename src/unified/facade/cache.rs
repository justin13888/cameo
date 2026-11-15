//! Response caching for [`CameoClient`]: the cache handle, the private
//! read/write helpers the capability modules use, and the public
//! `cached_*` / invalidation API.

use std::{sync::Arc, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

use super::CameoClient;
use crate::{
    cache::{CacheBackend, CacheKey, CacheTtlConfig, MediaType},
    unified::{
        media_id::MediaId,
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails, UnifiedTvShow,
            UnifiedTvShowDetails,
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

/// The item-cache key for an entry of `media_type` with `id`.
fn item_key(media_type: MediaType, id: &MediaId) -> CacheKey {
    CacheKey::Item {
        media_type,
        provider_id: id.to_string(),
    }
}

/// The detail-cache key for an entry of `media_type` with `id`.
fn detail_key(media_type: MediaType, id: &MediaId) -> CacheKey {
    CacheKey::Detail {
        media_type,
        provider_id: id.to_string(),
    }
}

/// A summary model indexed in the item cache under its provider id.
pub(super) trait CacheItem: Serialize + Sync {
    /// The item-cache key this value is stored under.
    fn item_key(&self) -> CacheKey;
}

impl CacheItem for UnifiedMovie {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::Movie, &self.provider_id)
    }
}

impl CacheItem for UnifiedTvShow {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::TvShow, &self.provider_id)
    }
}

impl CacheItem for UnifiedPerson {
    fn item_key(&self) -> CacheKey {
        item_key(MediaType::Person, &self.provider_id)
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

    /// Look up a summary model by id: the item entry if present,
    /// otherwise the summary `project`ed out of the detail entry.
    async fn cached_with_detail_fallback<I, D>(
        &self,
        media_type: MediaType,
        id: &MediaId,
        project: fn(D) -> I,
    ) -> Option<I>
    where
        I: DeserializeOwned,
        D: DeserializeOwned,
    {
        if let Some(item) = self.cache_get(&item_key(media_type, id)).await {
            return Some(item);
        }
        self.cache_get(&detail_key(media_type, id))
            .await
            .map(project)
    }
}

// ── Explicit cache lookup API ────────────────────────────────────────────────

impl CameoClient {
    /// Look up a movie from the cache by id (e.g. `MediaId::tmdb(550)`).
    pub async fn cached_movie(&self, id: &MediaId) -> Option<UnifiedMovie> {
        self.cached_with_detail_fallback(MediaType::Movie, id, |d: UnifiedMovieDetails| d.movie)
            .await
    }

    /// Look up full movie details from the cache by id.
    pub async fn cached_movie_details(&self, id: &MediaId) -> Option<UnifiedMovieDetails> {
        self.cache_get(&detail_key(MediaType::Movie, id)).await
    }

    /// Look up a TV show from the cache by id.
    pub async fn cached_tv_show(&self, id: &MediaId) -> Option<UnifiedTvShow> {
        self.cached_with_detail_fallback(MediaType::TvShow, id, |d: UnifiedTvShowDetails| d.show)
            .await
    }

    /// Look up full TV show details from the cache by id.
    pub async fn cached_tv_show_details(&self, id: &MediaId) -> Option<UnifiedTvShowDetails> {
        self.cache_get(&detail_key(MediaType::TvShow, id)).await
    }

    /// Look up a person from the cache by id.
    pub async fn cached_person(&self, id: &MediaId) -> Option<UnifiedPerson> {
        self.cached_with_detail_fallback(MediaType::Person, id, |d: UnifiedPersonDetails| d.person)
            .await
    }

    /// Look up full person details from the cache by id.
    pub async fn cached_person_details(&self, id: &MediaId) -> Option<UnifiedPersonDetails> {
        self.cache_get(&detail_key(MediaType::Person, id)).await
    }

    /// Invalidate all cache entries for `id`.
    ///
    /// Backend failures are logged as `tracing` warnings, not surfaced.
    pub async fn invalidate_cached(&self, id: &MediaId) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };
        for mt in [MediaType::Movie, MediaType::TvShow, MediaType::Person] {
            cache.invalidate(&detail_key(mt, id)).await;
            cache.invalidate(&item_key(mt, id)).await;
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
