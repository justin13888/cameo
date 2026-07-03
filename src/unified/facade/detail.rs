use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::{CacheKey, MediaType};
use crate::unified::{
    media_id::{MediaId, NativeId},
    models::{UnifiedMovieDetails, UnifiedPersonDetails, UnifiedTvShowDetails},
};

impl CameoClient {
    /// Build a [`MediaId`] tagged with the first detail provider's id, so the
    /// raw `i32` id is routed to that provider. (Per-call routing by id origin
    /// is added in a later step.)
    fn detail_media_id(&self, id: i32) -> Result<MediaId, CameoClientError> {
        let provider = self.require(&self.detail)?;
        Ok(MediaId::new(
            provider.id().to_string(),
            NativeId::Num(id.max(0) as u64),
        ))
    }

    /// Get detailed information about a movie.
    pub async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, CameoClientError> {
        let media_id = self.detail_media_id(id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: media_id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedMovieDetails>(&key).await {
            return Ok(hit);
        }
        let result = self.require(&self.detail)?.movie_details(&media_id).await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::Movie,
                    provider_id: result.movie.provider_id.to_string(),
                },
                &result.movie,
                self.cache_items_ttl(),
            )
            .await;
        }
        Ok(result)
    }

    /// Get detailed information about a TV show.
    pub async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, CameoClientError> {
        let media_id = self.detail_media_id(id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::TvShow,
            provider_id: media_id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedTvShowDetails>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.detail)?
            .tv_show_details(&media_id)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::TvShow,
                    provider_id: result.show.provider_id.to_string(),
                },
                &result.show,
                self.cache_items_ttl(),
            )
            .await;
        }
        Ok(result)
    }

    /// Get detailed information about a person.
    pub async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, CameoClientError> {
        let media_id = self.detail_media_id(id)?;
        #[cfg(feature = "cache")]
        let key = CacheKey::Detail {
            media_type: MediaType::Person,
            provider_id: media_id.to_string(),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = self.cache_get::<UnifiedPersonDetails>(&key).await {
            return Ok(hit);
        }
        let result = self
            .require(&self.detail)?
            .person_details(&media_id)
            .await?;
        #[cfg(feature = "cache")]
        {
            self.cache_put(key, &result, self.cache_details_ttl()).await;
            self.cache_put(
                CacheKey::Item {
                    media_type: MediaType::Person,
                    provider_id: result.person.provider_id.to_string(),
                },
                &result.person,
                self.cache_items_ttl(),
            )
            .await;
        }
        Ok(result)
    }
}
