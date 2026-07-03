use super::{CameoClient, CameoClientError};
#[cfg(feature = "cache")]
use crate::cache::CacheKey;
use crate::{
    core::{config::TimeWindow, pagination::Page},
    unified::models::{UnifiedMovie, UnifiedTvShow},
};

/// Dispatch a movie listing to the first discovery provider, with caching.
macro_rules! discover_movies {
    ($self:ident, $endpoint:expr, $page:expr, $provider:ident => $call:expr) => {{
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: $endpoint,
            page: $page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = $self.cache_get::<Page<UnifiedMovie>>(&key).await {
            return Ok(hit);
        }
        let $provider = $self.require(&$self.discovery)?;
        let result = $call.await?;
        #[cfg(feature = "cache")]
        {
            $self
                .cache_put(key, &result, $self.cache_discovery_ttl())
                .await;
            $self.cache_movie_items(&result).await;
        }
        Ok(result)
    }};
}

/// Dispatch a TV listing to the first discovery provider, with caching.
macro_rules! discover_tv {
    ($self:ident, $endpoint:expr, $page:expr, $provider:ident => $call:expr) => {{
        #[cfg(feature = "cache")]
        let key = CacheKey::Discovery {
            endpoint: $endpoint,
            page: $page.unwrap_or(1),
        };
        #[cfg(feature = "cache")]
        if let Some(hit) = $self.cache_get::<Page<UnifiedTvShow>>(&key).await {
            return Ok(hit);
        }
        let $provider = $self.require(&$self.discovery)?;
        let result = $call.await?;
        #[cfg(feature = "cache")]
        {
            $self
                .cache_put(key, &result, $self.cache_discovery_ttl())
                .await;
            $self.cache_tv_items(&result).await;
        }
        Ok(result)
    }};
}

impl CameoClient {
    /// Get trending movies.
    pub async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        discover_movies!(self, format!("trending_movies_{time_window}"), page,
            p => p.trending_movies(time_window, page))
    }

    /// Get trending TV shows.
    pub async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        discover_tv!(self, format!("trending_tv_{time_window}"), page,
            p => p.trending_tv_shows(time_window, page))
    }

    /// Get popular movies.
    pub async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        discover_movies!(self, "popular_movies".to_string(), page,
            p => p.popular_movies(page))
    }

    /// Get top-rated movies.
    pub async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, CameoClientError> {
        discover_movies!(self, "top_rated_movies".to_string(), page,
            p => p.top_rated_movies(page))
    }

    /// Get popular TV shows.
    pub async fn popular_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        discover_tv!(self, "popular_tv".to_string(), page,
            p => p.popular_tv_shows(page))
    }

    /// Get top-rated TV shows.
    pub async fn top_rated_tv_shows(
        &self,
        page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, CameoClientError> {
        discover_tv!(self, "top_rated_tv".to_string(), page,
            p => p.top_rated_tv_shows(page))
    }
}
