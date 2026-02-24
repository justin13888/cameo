use async_trait::async_trait;

use crate::core::{config::TimeWindow, pagination::PaginatedResponse};

use super::{
    models::*,
    traits::{DetailProvider, DiscoveryProvider, SearchProvider},
};

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
}

/// Builder for constructing a [`CameoClient`].
#[derive(Default)]
pub struct CameoClientBuilder {
    #[cfg(feature = "tmdb")]
    tmdb_config: Option<TmdbConfig>,
}

impl CameoClientBuilder {
    /// Configure the TMDB provider.
    #[cfg(feature = "tmdb")]
    pub fn with_tmdb(mut self, config: TmdbConfig) -> Self {
        self.tmdb_config = Some(config);
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

        if tmdb.is_none() {
            return Err(CameoClientError::NoProviders);
        }

        Ok(CameoClient {
            #[cfg(feature = "tmdb")]
            tmdb,
        })
    }
}

/// Multi-provider facade client.
///
/// Use [`CameoClientBuilder`] to construct one.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use cameo::providers::tmdb::TmdbConfig;
/// use cameo::unified::{CameoClient, SearchProvider};
///
/// let client = CameoClient::builder()
///     .with_tmdb(TmdbConfig::new("your-token"))
///     .build()?;
///
/// let results = client.search_movies("Inception", None).await?;
/// # Ok(())
/// # }
/// ```
pub struct CameoClient {
    #[cfg(feature = "tmdb")]
    tmdb: Option<TmdbClient>,
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

    /// Returns the first available TMDB client or an error.
    #[cfg(feature = "tmdb")]
    fn tmdb_or_err(&self) -> Result<&TmdbClient, CameoClientError> {
        self.tmdb.as_ref().ok_or(CameoClientError::NoProviders)
    }
}

#[async_trait]
impl SearchProvider for CameoClient {
    type Error = CameoClientError;

    async fn search_movies(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.search_movies(query, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn search_tv_shows(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.search_tv_shows(query, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn search_people(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedPerson>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.search_people(query, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn search_multi(
        &self,
        query: &str,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedSearchResult>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.search_multi(query, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }
}

#[async_trait]
impl DetailProvider for CameoClient {
    type Error = CameoClientError;

    async fn movie_details(&self, id: i32) -> Result<UnifiedMovieDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            return Ok(client.movie_details(id).await?.into());
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn tv_show_details(&self, id: i32) -> Result<UnifiedTvShowDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            return Ok(client.tv_series_details(id).await?.into());
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn person_details(&self, id: i32) -> Result<UnifiedPersonDetails, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            return Ok(client.person_details(id).await?.into());
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }
}

#[async_trait]
impl DiscoveryProvider for CameoClient {
    type Error = CameoClientError;

    async fn trending_movies(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.trending_movies(time_window, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn trending_tv_shows(
        &self,
        time_window: TimeWindow,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedTvShow>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.trending_tv(time_window, page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn popular_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.popular_movies(page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }

    async fn top_rated_movies(
        &self,
        page: Option<u32>,
    ) -> Result<PaginatedResponse<UnifiedMovie>, CameoClientError> {
        #[cfg(feature = "tmdb")]
        {
            let client = self.tmdb_or_err()?;
            let page_resp = client.top_rated_movies(page).await?;
            return Ok(PaginatedResponse {
                page: page_resp.page,
                total_pages: page_resp.total_pages,
                total_results: page_resp.total_results,
                results: page_resp.results.into_iter().map(Into::into).collect(),
            });
        }
        #[allow(unreachable_code)]
        Err(CameoClientError::NoProviders)
    }
}
