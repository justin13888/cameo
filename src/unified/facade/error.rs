use crate::core::error::ProviderError;

/// Error type returned by [`CameoClient`](super::CameoClient) methods and
/// [`CameoClientBuilder::build`](super::CameoClientBuilder::build).
///
/// Distinguishes three situations: [`NotConfigured`](CameoClientError::NotConfigured)
/// (no provider is configured), [`Unsupported`](CameoClientError::Unsupported)
/// (a provider is configured but none supports the requested capability), and a
/// provider-level [`Provider`](CameoClientError::Provider) error.
///
/// # Matching on variants
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "tmdb")]
/// # {
/// use cameo::providers::tmdb::TmdbConfig;
/// use cameo::unified::{CameoClient, CameoClientError};
/// use cameo::ProviderError;
///
/// let client = CameoClient::builder()
///     .with_tmdb(TmdbConfig::new("your-token"))
///     .build()?;
///
/// match client.search_movies("Inception", None).await {
///     Ok(results) => println!("{} results", results.total_results),
///     Err(CameoClientError::Provider(ProviderError::Auth(msg))) => {
///         eprintln!("authentication failed: {msg}");
///     }
///     Err(CameoClientError::Provider(ProviderError::NotFound)) => {
///         eprintln!("not found");
///     }
///     Err(CameoClientError::NotConfigured) => eprintln!("no providers configured"),
///     Err(CameoClientError::Unsupported) => eprintln!("no provider supports this"),
///     Err(e) => eprintln!("error: {e}"),
/// }
/// # }
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CameoClientError {
    /// No providers have been configured (or compiled in).
    #[error("no providers configured")]
    NotConfigured,

    /// A provider is configured, but none supports the requested capability.
    #[error("no configured provider supports this operation")]
    Unsupported,

    /// An error returned by a provider.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}
