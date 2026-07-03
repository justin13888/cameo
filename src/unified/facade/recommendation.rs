use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::CacheKey;
use crate::{
    core::pagination::Page,
    unified::{
        media_id::MediaId,
        models::{UnifiedMovie, UnifiedTvShow},
    },
};

impl CameoClient {
    /// Get movie recommendations based on a movie.
    pub async fn movie_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("movie_recommendations_{id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .route(&self.recommendation, id)?
            .movie_recommendations(id, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_discovery_ttl())
                .await;
            self.cache_movie_items(&result).await;
        }
        Ok(result)
    }

    /// Get TV show recommendations based on a TV show.
    pub async fn tv_recommendations(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("tv_recommendations_{id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .route(&self.recommendation, id)?
            .tv_recommendations(id, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_discovery_ttl())
                .await;
            self.cache_tv_items(&result).await;
        }
        Ok(result)
    }

    /// Get movies similar to a given movie.
    pub async fn similar_movies(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("similar_movies_{id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .route(&self.recommendation, id)?
            .similar_movies(id, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_discovery_ttl())
                .await;
            self.cache_movie_items(&result).await;
        }
        Ok(result)
    }

    /// Get TV shows similar to a given TV show.
    pub async fn similar_tv_shows(
        &self,
        id: &MediaId,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("similar_tv_{id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .route(&self.recommendation, id)?
            .similar_tv_shows(id, page)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_discovery_ttl())
                .await;
            self.cache_tv_items(&result).await;
        }
        Ok(result)
    }
}
