/// Core error type for direct provider use.
///
/// Wraps provider-specific errors so callers can handle them uniformly
/// when working with `TmdbClient` or `AniListClient` directly.
///
/// Most applications should use the [`crate::unified::CameoClient`] facade
/// instead, which returns [`crate::unified::CameoClientError`]. That type
/// adds `NoProviders` and `Cache` variants on top of provider errors and
/// is the recommended entry point for error handling.
///
/// # Matching on variants
///
/// ```no_run
/// # #[cfg(feature = "tmdb")]
/// # {
/// use cameo::core::error::CameoError;
/// use cameo::TmdbError;
///
/// fn handle(err: CameoError) {
///     match err {
///         CameoError::Tmdb(TmdbError::Api { status: 401, .. }) => {
///             eprintln!("authentication failed — check your token");
///         }
///         CameoError::Tmdb(TmdbError::Api { status, message }) => {
///             eprintln!("TMDB API error {status}: {message}");
///         }
///         CameoError::Tmdb(TmdbError::Http(e)) => {
///             eprintln!("network error: {e}");
///         }
///         other => eprintln!("other error: {other}"),
///     }
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum CameoError {
    /// Error from the TMDB provider.
    #[cfg(feature = "tmdb")]
    #[error(transparent)]
    Tmdb(#[from] crate::providers::tmdb::error::TmdbError),

    /// Error from the AniList provider.
    #[cfg(feature = "anilist")]
    #[error(transparent)]
    AniList(#[from] crate::providers::anilist::AniListError),

}
