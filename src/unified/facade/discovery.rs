use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::{
    core::{config::TimeWindow, pagination::PaginatedResponse},
    unified::{
        models::{UnifiedMovie, UnifiedTvShow},
        traits::DiscoveryProvider,
    },
};

impl DiscoveryProvider for CameoClient {
    type Error = CameoClientError;

    async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        let _ = (time_window, page);
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
                tracing::debug!(page = page_num, "cache hit: trending_movies");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: trending_movies");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching trending_movies to tmdb");
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching trending_movies to anilist");
            let unified = client.trending_movies(time_window, page).await?;

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

    async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        let _ = (time_window, page);
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
                tracing::debug!(page = page_num, "cache hit: trending_tv_shows");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: trending_tv_shows");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching trending_tv_shows to tmdb");
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching trending_tv_shows to anilist");
            let unified = client.trending_tv(time_window, page).await?;

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

    async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        let _ = page;
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
                tracing::debug!(page = page_num, "cache hit: popular_movies");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: popular_movies");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching popular_movies to tmdb");
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching popular_movies to anilist");
            let unified = client.popular_movies(page).await?;

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

    async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        let _ = page;
        #[cfg(feature = "cache")]
        let endpoint = "popular_tv_shows".to_string();

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
                tracing::debug!(page = page_num, "cache hit: popular_tv_shows");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: popular_tv_shows");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching popular_tv_shows to tmdb");
            let page_resp = client.popular_tv_shows(page).await?;
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching popular_tv_shows to anilist");
            let unified = client.popular_tv_shows(page).await?;

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

    async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        let _ = page;
        #[cfg(feature = "cache")]
        let endpoint = "top_rated_tv_shows".to_string();

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
                tracing::debug!(page = page_num, "cache hit: top_rated_tv_shows");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: top_rated_tv_shows");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching top_rated_tv_shows to tmdb");
            let page_resp = client.top_rated_tv_shows(page).await?;
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching top_rated_tv_shows to anilist");
            let unified = client.top_rated_tv_shows(page).await?;

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

    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        let _ = page;
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
                tracing::debug!(page = page_num, "cache hit: top_rated_movies");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(page = page_num, "cache miss: top_rated_movies");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(page = ?page, "dispatching top_rated_movies to tmdb");
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

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(page = ?page, "dispatching top_rated_movies to anilist");
            let unified = client.top_rated_movies(page).await?;

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
}

#[cfg(feature = "cache")]
fn time_window_str(tw: TimeWindow) -> &'static str {
    match tw {
        TimeWindow::Day => "day",
        TimeWindow::Week => "week",
    }
}
