use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::CacheKey;
#[cfg(all(feature = "cache", feature = "tmdb"))]
use crate::cache::MediaType;
use crate::{
    core::pagination::PaginatedResponse,
    unified::{
        models::{UnifiedMovie, UnifiedTvShow},
        traits::RecommendationProvider,
    },
};

impl RecommendationProvider for CameoClient {
    type Error = CameoClientError;

    async fn movie_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        let _ = (id, page);
        #[cfg(feature = "cache")]
        let endpoint = format!("recommendations:movie:{id}");

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
                tracing::debug!(id, page = page_num, "cache hit: movie_recommendations");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(id, page = page_num, "cache miss: movie_recommendations");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, page = ?page, "dispatching movie_recommendations to tmdb");
            let unified = client.movie_recommendations(id, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let discovery_key = CacheKey::Discovery {
                    endpoint,
                    page: page_num,
                };
                cache.set(discovery_key, &unified, cache.ttl.discovery);
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn tv_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        let _ = (id, page);
        #[cfg(feature = "cache")]
        let endpoint = format!("recommendations:tv:{id}");

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
                tracing::debug!(id, page = page_num, "cache hit: tv_recommendations");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(id, page = page_num, "cache miss: tv_recommendations");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, page = ?page, "dispatching tv_recommendations to tmdb");
            let page_resp = client.tv_recommendations(id, page).await?;
            let unified: PaginatedResponse<UnifiedTvShow> = PaginatedResponse {
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
                cache.set(discovery_key, &unified, cache.ttl.discovery);
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn similar_movies(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        let _ = (id, page);
        #[cfg(feature = "cache")]
        let endpoint = format!("similar:movie:{id}");

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
                tracing::debug!(id, page = page_num, "cache hit: similar_movies");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(id, page = page_num, "cache miss: similar_movies");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, page = ?page, "dispatching similar_movies to tmdb");
            let page_resp = client.similar_movies(id, page).await?;
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
                cache.set(discovery_key, &unified, cache.ttl.discovery);
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }

    async fn similar_tv_shows(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        let _ = (id, page);
        #[cfg(feature = "cache")]
        let endpoint = format!("similar:tv:{id}");

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
                tracing::debug!(id, page = page_num, "cache hit: similar_tv_shows");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(id, page = page_num, "cache miss: similar_tv_shows");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(id, page = ?page, "dispatching similar_tv_shows to tmdb");
            let page_resp = client.similar_tv_shows(id, page).await?;
            let unified: PaginatedResponse<UnifiedTvShow> = PaginatedResponse {
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
                cache.set(discovery_key, &unified, cache.ttl.discovery);
                for item in &unified.results {
                    let k = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(k, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }
}
