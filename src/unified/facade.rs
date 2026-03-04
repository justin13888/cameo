#[cfg(feature = "cache")]
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(feature = "cache")]
use serde::Serialize;
#[cfg(feature = "cache")]
use serde::de::DeserializeOwned;

use super::{
    models::*,
    traits::{DetailProvider, DiscoveryProvider, SearchProvider},
};
#[cfg(feature = "cache")]
use crate::cache::{CacheBackend, CacheError, CacheKey, CacheTtlConfig, MediaType, SqliteCache};
use crate::core::{config::TimeWindow, pagination::PaginatedResponse};
#[cfg(feature = "anilist")]
use crate::providers::anilist::{AniListClient, AniListConfig};
#[cfg(feature = "tmdb")]
use crate::providers::tmdb::{TmdbClient, TmdbConfig};

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
    tmdb: Option<TmdbClient>,

    #[cfg(feature = "anilist")]
    anilist: Option<AniListClient>,

    #[cfg(feature = "cache")]
    cache: Option<Cache>,
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
    pub async fn cached_movie(&self, provider_id: &str) -> Option<UnifiedMovie> {
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::Movie,
            provider_id: provider_id.to_string(),
        };
        if let Some(m) = cache.get::<UnifiedMovie>(&item_key).await {
            return Some(m);
        }
        // Fall back: extract base movie from detail cache.
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
    pub async fn cached_movie_details(&self, provider_id: &str) -> Option<UnifiedMovieDetails> {
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
    pub async fn cached_tv_show(&self, provider_id: &str) -> Option<UnifiedTvShow> {
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::TvShow,
            provider_id: provider_id.to_string(),
        };
        if let Some(t) = cache.get::<UnifiedTvShow>(&item_key).await {
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
    pub async fn cached_tv_show_details(&self, provider_id: &str) -> Option<UnifiedTvShowDetails> {
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
    pub async fn cached_person(&self, provider_id: &str) -> Option<UnifiedPerson> {
        let cache = self.cache.as_ref()?;
        let item_key = CacheKey::Item {
            media_type: MediaType::Person,
            provider_id: provider_id.to_string(),
        };
        if let Some(p) = cache.get::<UnifiedPerson>(&item_key).await {
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
    pub async fn cached_person_details(&self, provider_id: &str) -> Option<UnifiedPersonDetails> {
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

// ── SearchProvider ────────────────────────────────────────────────────────────

#[async_trait]
impl SearchProvider for CameoClient {
    type Error = CameoClientError;

    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let search_key = CacheKey::Search {
                media_type: Some(MediaType::Movie),
                query: query.to_string(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedMovie>>(&search_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.search_movies(query, page).await?;
            let unified: PaginatedResponse<UnifiedMovie> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Movie),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.search_movies(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Movie),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let search_key = CacheKey::Search {
                media_type: Some(MediaType::TvShow),
                query: query.to_string(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedTvShow>>(&search_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.search_tv_shows(query, page).await?;
            let unified: PaginatedResponse<UnifiedTvShow> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::TvShow),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.search_tv_shows(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::TvShow),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedPerson>, CameoClientError> {
        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let search_key = CacheKey::Search {
                media_type: Some(MediaType::Person),
                query: query.to_string(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedPerson>>(&search_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.search_people(query, page).await?;
            let unified: PaginatedResponse<UnifiedPerson> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Person),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Person,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.search_people(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Person),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Person,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedSearchResult>, CameoClientError> {
        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let search_key = CacheKey::Search {
                media_type: None,
                query: query.to_string(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedSearchResult>>(&search_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.search_multi(query, page).await?;
            let unified = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: None,
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                // Index individual items by provider_id.
                for item in &unified.results {
                    match item {
                        UnifiedSearchResult::Movie(m) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Movie,
                                provider_id: m.provider_id.clone(),
                            };
                            cache.set(k, m, cache.ttl.items).await;
                        }
                        UnifiedSearchResult::TvShow(t) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::TvShow,
                                provider_id: t.provider_id.clone(),
                            };
                            cache.set(k, t, cache.ttl.items).await;
                        }
                        UnifiedSearchResult::Person(p) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Person,
                                provider_id: p.provider_id.clone(),
                            };
                            cache.set(k, p, cache.ttl.items).await;
                        }
                    }
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.search_multi(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: None,
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search).await;
                for item in &unified.results {
                    match item {
                        UnifiedSearchResult::Movie(m) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Movie,
                                provider_id: m.provider_id.clone(),
                            };
                            cache.set(k, m, cache.ttl.items).await;
                        }
                        UnifiedSearchResult::TvShow(t) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::TvShow,
                                provider_id: t.provider_id.clone(),
                            };
                            cache.set(k, t, cache.ttl.items).await;
                        }
                        UnifiedSearchResult::Person(p) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Person,
                                provider_id: p.provider_id.clone(),
                            };
                            cache.set(k, p, cache.ttl.items).await;
                        }
                    }
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }
}

// ── DetailProvider ────────────────────────────────────────────────────────────

#[async_trait]
impl DetailProvider for CameoClient {
    type Error = CameoClientError;

    async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedMovieDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details: UnifiedMovieDetails = client.movie_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.movie_details_ttl(
                    details.movie.release_date.as_deref(),
                    details.status.as_deref(),
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id,
                };
                cache.set(item_key, &details.movie, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedMovieDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details = client.movie_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.movie_details_ttl(
                    details.movie.release_date.as_deref(),
                    details.status.as_deref(),
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Movie,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id,
                };
                cache.set(item_key, &details.movie, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedTvShowDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details: UnifiedTvShowDetails = client.tv_series_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.tv_show_details_ttl(
                    details.show.first_air_date.as_deref(),
                    details.last_air_date.as_deref(),
                    details.status.as_deref(),
                    details.in_production,
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id,
                };
                cache.set(item_key, &details.show, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedTvShowDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details = client.tv_show_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_ttl = cache.ttl.tv_show_details_ttl(
                    details.show.first_air_date.as_deref(),
                    details.last_air_date.as_deref(),
                    details.status.as_deref(),
                    details.in_production,
                );
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::TvShow,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, detail_ttl).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id,
                };
                cache.set(item_key, &details.show, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            #[cfg(feature = "cache")]
            let provider_id = format!("tmdb:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedPersonDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details: UnifiedPersonDetails = client.person_details(id).await?.into();

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, cache.ttl.details).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id,
                };
                cache.set(item_key, &details.person, cache.ttl.items).await;
            }

            return Ok(details);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            #[cfg(feature = "cache")]
            let provider_id = format!("anilist:staff:{id}");

            #[cfg(feature = "cache")]
            {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                if let Some(cache) = self.cache.as_ref()
                    && let Some(cached) = cache.get::<UnifiedPersonDetails>(&detail_key).await
                {
                    return Ok(cached);
                }
            }

            let details = client.person_details(id).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let detail_key = CacheKey::Detail {
                    media_type: MediaType::Person,
                    provider_id: provider_id.clone(),
                };
                cache.set(detail_key, &details, cache.ttl.details).await;
                let item_key = CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id,
                };
                cache.set(item_key, &details.person, cache.ttl.items).await;
            }

            return Ok(details);
        }

        Err(CameoClientError::NoProviders)
    }
}

// ── DiscoveryProvider ─────────────────────────────────────────────────────────

#[async_trait]
impl DiscoveryProvider for CameoClient {
    type Error = CameoClientError;

    async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let endpoint = format!("trending_movies:{}", time_window_str(time_window));

        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let discovery_key = CacheKey::Discovery {
                endpoint: endpoint.clone(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedMovie>>(&discovery_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.trending_movies(time_window, page).await?;
            let unified: PaginatedResponse<UnifiedMovie> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint: endpoint.clone(),
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.trending_movies(time_window, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "cache")]
        let endpoint = format!("trending_tv:{}", time_window_str(time_window));

        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let discovery_key = CacheKey::Discovery {
                endpoint: endpoint.clone(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedTvShow>>(&discovery_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.trending_tv(time_window, page).await?;
            let unified: PaginatedResponse<UnifiedTvShow> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint: endpoint.clone(),
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.trending_tv(time_window, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let endpoint = "popular_movies".to_string();

        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let discovery_key = CacheKey::Discovery {
                endpoint: endpoint.clone(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedMovie>>(&discovery_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.popular_movies(page).await?;
            let unified: PaginatedResponse<UnifiedMovie> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.popular_movies(page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let endpoint = "top_rated_movies".to_string();

        #[cfg(feature = "cache")]
        let page_num = page.unwrap_or(1);

        #[cfg(feature = "cache")]
        {
            let discovery_key = CacheKey::Discovery {
                endpoint: endpoint.clone(),
                page: page_num,
            };
            if let Some(cache) = self.cache.as_ref()
                && let Some(cached) = cache
                    .get::<PaginatedResponse<UnifiedMovie>>(&discovery_key)
                    .await
            {
                return Ok(cached);
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            let page_resp = client.top_rated_movies(page).await?;
            let unified: PaginatedResponse<UnifiedMovie> = PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            };

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            let unified = client.top_rated_movies(page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache
                    .set(discovery_key, &unified, cache.ttl.discovery)
                    .await;
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items).await;
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }
}

#[cfg(feature = "cache")]
fn time_window_str(tw: TimeWindow) -> &'static str {
    match tw {
        TimeWindow::Day => "day",
        TimeWindow::Week => "week",
    }
}
