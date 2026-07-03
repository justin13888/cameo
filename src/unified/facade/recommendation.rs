use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::CacheKey;
use crate::{
    core::pagination::Page,
    unified::models::{UnifiedMovie, UnifiedTvShow},
};

impl CameoClient {
    /// Get movie recommendations based on a movie.
    pub async fn movie_recommendations(
        &self,
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        let media_id = self.media_id_for(&self.recommendation, id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("movie_recommendations_{media_id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.recommendation)?
            .movie_recommendations(&media_id, page)
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
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        let media_id = self.media_id_for(&self.recommendation, id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("tv_recommendations_{media_id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.recommendation)?
            .tv_recommendations(&media_id, page)
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
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        let media_id = self.media_id_for(&self.recommendation, id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("similar_movies_{media_id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.recommendation)?
            .similar_movies(&media_id, page)
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
        id: i32,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        let media_id = self.media_id_for(&self.recommendation, id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: format!("similar_tv_{media_id}"),
            page: page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.recommendation)?
            .similar_tv_shows(&media_id, page)
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
