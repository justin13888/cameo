use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::{
    core::pagination::PaginatedResponse,
    unified::{
        models::{UnifiedMovie, UnifiedPerson, UnifiedSearchResult, UnifiedTvShow},
        traits::SearchProvider,
    },
};

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
                tracing::debug!(query, page = page_num, "cache hit: search_movies");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(query, page = page_num, "cache miss: search_movies");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(query, page = ?page, "dispatching search_movies to tmdb");
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
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(query, page = ?page, "dispatching search_movies to anilist");
            let unified = client.search_movies(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Movie),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Movie,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
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
                tracing::debug!(query, page = page_num, "cache hit: search_tv_shows");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(query, page = page_num, "cache miss: search_tv_shows");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(query, page = ?page, "dispatching search_tv_shows to tmdb");
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
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(query, page = ?page, "dispatching search_tv_shows to anilist");
            let unified = client.search_tv_shows(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::TvShow),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::TvShow,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
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
                tracing::debug!(query, page = page_num, "cache hit: search_people");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(query, page = page_num, "cache miss: search_people");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(query, page = ?page, "dispatching search_people to tmdb");
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
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Person,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(query, page = ?page, "dispatching search_people to anilist");
            let unified = client.search_people(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: Some(MediaType::Person),
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    let item_key = CacheKey::Item {
                        media_type: MediaType::Person,
                        provider_id: item.provider_id.clone(),
                    };
                    cache.set(item_key, item, cache.ttl.items);
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
                tracing::debug!(query, page = page_num, "cache hit: search_multi");
                return Ok(cached);
            }
            if self.cache.is_some() {
                tracing::debug!(query, page = page_num, "cache miss: search_multi");
            }
        }

        #[cfg(feature = "tmdb")]
        if let Some(client) = &self.tmdb {
            tracing::debug!(query, page = ?page, "dispatching search_multi to tmdb");
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
                cache.set(search_key, &unified, cache.ttl.search);
                // Index individual items by provider_id.
                for item in &unified.results {
                    match item {
                        UnifiedSearchResult::Movie(m) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Movie,
                                provider_id: m.provider_id.clone(),
                            };
                            cache.set(k, m, cache.ttl.items);
                        }
                        UnifiedSearchResult::TvShow(t) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::TvShow,
                                provider_id: t.provider_id.clone(),
                            };
                            cache.set(k, t, cache.ttl.items);
                        }
                        UnifiedSearchResult::Person(p) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Person,
                                provider_id: p.provider_id.clone(),
                            };
                            cache.set(k, p, cache.ttl.items);
                        }
                    }
                }
            }

            return Ok(unified);
        }

        #[cfg(feature = "anilist")]
        if let Some(client) = &self.anilist {
            tracing::debug!(query, page = ?page, "dispatching search_multi to anilist");
            let unified = client.search_multi(query, page).await?;

            #[cfg(feature = "cache")]
            if let Some(cache) = self.cache.as_ref() {
                let search_key = CacheKey::Search {
                    media_type: None,
                    query: query.to_string(),
                    page: page_num,
                };
                cache.set(search_key, &unified, cache.ttl.search);
                for item in &unified.results {
                    match item {
                        UnifiedSearchResult::Movie(m) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Movie,
                                provider_id: m.provider_id.clone(),
                            };
                            cache.set(k, m, cache.ttl.items);
                        }
                        UnifiedSearchResult::TvShow(t) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::TvShow,
                                provider_id: t.provider_id.clone(),
                            };
                            cache.set(k, t, cache.ttl.items);
                        }
                        UnifiedSearchResult::Person(p) => {
                            let k = CacheKey::Item {
                                media_type: MediaType::Person,
                                provider_id: p.provider_id.clone(),
                            };
                            cache.set(k, p, cache.ttl.items);
                        }
                    }
                }
            }

            return Ok(unified);
        }

        Err(CameoClientError::NoProviders)
    }
}
