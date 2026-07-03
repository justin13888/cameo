use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::{
    core::pagination::Page,
    unified::models::{UnifiedMovie, UnifiedPerson, UnifiedSearchResult, UnifiedTvShow},
};

impl CameoClient {
    /// Search for movies via the configured provider(s).
    pub async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Search {
            media_type: Some(MediaType::Movie),
            query: query.to_string(),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.search)?
            .search_movies(query, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_search_ttl()).await;
            for item in &result.results {
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
        Ok(result)
    }

    /// Search for TV shows via the configured provider(s).
    pub async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Search {
            media_type: Some(MediaType::TvShow),
            query: query.to_string(),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.search)?
            .search_tv_shows(query, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_search_ttl()).await;
            for item in &result.results {
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
        Ok(result)
    }

    /// Search for people via the configured provider(s).
    pub async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Search {
            media_type: Some(MediaType::Person),
            query: query.to_string(),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedPerson>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.search)?
            .search_people(query, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_search_ttl()).await;
            for item in &result.results {
                self.cache_put(
                    CacheKey::Item {
                        media_type: MediaType::Person,
                        provider_id: item.provider_id.to_string(),
                    },
                    item,
                    self.cache_items_ttl(),
                )
                .await;
            }
        }
        Ok(result)
    }

    /// Multi-search across movies, TV shows, and people.
    pub async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Search {
            media_type: None,
            query: query.to_string(),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedSearchResult>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.search)?
            .search_multi(query, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_search_ttl()).await;
            for item in &result.results {
                let (media_type, provider_id) = match item {
                    UnifiedSearchResult::Movie(m) => (MediaType::Movie, m.provider_id.to_string()),
                    UnifiedSearchResult::TvShow(t) => {
                        (MediaType::TvShow, t.provider_id.to_string())
                    }
                    UnifiedSearchResult::Person(p) => {
                        (MediaType::Person, p.provider_id.to_string())
                    }
                };
                self.cache_put(
                    CacheKey::Item {
                        media_type,
                        provider_id,
                    },
                    item,
                    self.cache_items_ttl(),
                )
                .await;
            }
        }
        Ok(result)
    }
}

/// Per-call provider selection: search with a specific provider by id (e.g.
/// `"anilist"`), overriding the default priority. These bypass the cache since
/// the result is provider-scoped.
impl CameoClient {
    /// Search for movies with a specific provider (e.g. `"anilist"`).
    pub async fn search_movies_with(
        &self,
        provider_id: &str,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        Ok(self
            .select(&self.search, provider_id)?
            .search_movies(query, page)
            .await?)
    }

    /// Search for TV shows with a specific provider.
    pub async fn search_tv_shows_with(
        &self,
        provider_id: &str,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        Ok(self
            .select(&self.search, provider_id)?
            .search_tv_shows(query, page)
            .await?)
    }

    /// Search for people with a specific provider.
    pub async fn search_people_with(
        &self,
        provider_id: &str,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, CameoClientError> {
        Ok(self
            .select(&self.search, provider_id)?
            .search_people(query, page)
            .await?)
    }

    /// Multi-search with a specific provider.
    pub async fn search_multi_with(
        &self,
        provider_id: &str,
        query: &str,
        page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, CameoClientError> {
        Ok(self
            .select(&self.search, provider_id)?
            .search_multi(query, page)
            .await?)
    }
}
