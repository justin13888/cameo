//! An out-of-crate-style provider registers with the facade using only the
//! public API — proving adding a provider requires zero facade edits (#30).

use std::sync::Arc;

use async_trait::async_trait;
use cameo::{
    MediaId, Page,
    core::error::ProviderError,
    unified::{
        CameoClient, CameoClientError, Capability, Provider, SearchProvider,
        models::{UnifiedMovie, UnifiedPerson, UnifiedSearchResult, UnifiedTvShow},
    },
};

/// A trivial provider implemented entirely against cameo's public API.
struct StubProvider;

impl Provider for StubProvider {
    fn id(&self) -> &str {
        "stub"
    }
    fn capabilities(&self) -> &[Capability] {
        &[Capability::Search]
    }
}

#[async_trait]
impl SearchProvider for StubProvider {
    async fn search_movies(
        &self,
        _query: &str,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, 0))
    }
    async fn search_tv_shows(
        &self,
        _query: &str,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, 0))
    }
    async fn search_people(
        &self,
        _query: &str,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, 0))
    }
    async fn search_multi(
        &self,
        _query: &str,
        _page: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, 0))
    }
}

#[tokio::test]
async fn external_provider_registers_and_dispatches() {
    let client = CameoClient::builder()
        .register_search_provider(Arc::new(StubProvider))
        .build()
        .expect("a registered provider is a configured provider");

    // Search dispatches to the stub.
    let result = client.search_movies("anything", None).await;
    assert!(result.is_ok(), "stub search should succeed: {result:?}");
    assert_eq!(result.unwrap().results.len(), 0);

    // The stub does not support details → Unsupported (not NotConfigured).
    let detail = client.movie_details(&MediaId::tmdb(1)).await;
    assert!(
        matches!(detail, Err(CameoClientError::Unsupported)),
        "expected Unsupported, got {detail:?}"
    );
}

// ── Per-call selection, priority, and id-origin routing (#31) ─────────────────

/// A provider that marks its search result (via `total_results`) and its detail
/// result (via an `Other` error carrying its id), so tests can tell which
/// provider handled a call.
struct MarkerProvider {
    id: &'static str,
    marker: u32,
}

impl Provider for MarkerProvider {
    fn id(&self) -> &str {
        self.id
    }
    fn capabilities(&self) -> &[Capability] {
        &[Capability::Search, Capability::Details]
    }
}

#[async_trait]
impl SearchProvider for MarkerProvider {
    async fn search_movies(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, self.marker))
    }
    async fn search_tv_shows(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, self.marker))
    }
    async fn search_people(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, self.marker))
    }
    async fn search_multi(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        Ok(Page::with_has_next(1, Vec::new(), false, 0, self.marker))
    }
}

#[async_trait]
impl cameo::unified::DetailProvider for MarkerProvider {
    async fn movie_details(
        &self,
        _id: &MediaId,
    ) -> Result<cameo::UnifiedMovieDetails, ProviderError> {
        Err(ProviderError::Other(self.id.to_string()))
    }
    async fn tv_show_details(
        &self,
        _id: &MediaId,
    ) -> Result<cameo::UnifiedTvShowDetails, ProviderError> {
        Err(ProviderError::Other(self.id.to_string()))
    }
    async fn person_details(
        &self,
        _id: &MediaId,
    ) -> Result<cameo::UnifiedPersonDetails, ProviderError> {
        Err(ProviderError::Other(self.id.to_string()))
    }
}

fn dual_client(priority: Option<Vec<&str>>) -> CameoClient {
    let alpha = Arc::new(MarkerProvider {
        id: "alpha",
        marker: 1,
    });
    let beta = Arc::new(MarkerProvider {
        id: "beta",
        marker: 2,
    });
    let mut builder = CameoClient::builder()
        .register_search_provider(alpha.clone())
        .register_detail_provider(alpha)
        .register_search_provider(beta.clone())
        .register_detail_provider(beta);
    if let Some(order) = priority {
        builder = builder.with_priority(order);
    }
    builder.build().unwrap()
}

/// Per-call selection reaches the non-default provider (the "AniList reachable"
/// property): with alpha first by default, `search_movies_with("beta", …)` still
/// reaches beta.
#[tokio::test]
async fn per_call_selection_reaches_non_default_provider() {
    let client = dual_client(None);

    // Default priority → alpha.
    let default = client.search_movies("x", None).await.unwrap();
    assert_eq!(default.total_results, 1, "default should be alpha");

    // Explicit selection → beta.
    let beta = client.search_movies_with("beta", "x", None).await.unwrap();
    assert_eq!(
        beta.total_results, 2,
        "explicit selection should reach beta"
    );
}

/// Configurable default priority.
#[tokio::test]
async fn configurable_default_priority() {
    let client = dual_client(Some(vec!["beta", "alpha"]));
    let default = client.search_movies("x", None).await.unwrap();
    assert_eq!(
        default.total_results, 2,
        "beta prioritized → default is beta"
    );
}

/// Detail lookups route to the provider that minted the id.
#[tokio::test]
async fn detail_lookup_routes_by_id_origin() {
    use cameo::{MediaId, NativeId};
    let client = dual_client(None);

    match client
        .movie_details(&MediaId::new("alpha", NativeId::Num(1)))
        .await
    {
        Err(CameoClientError::Provider(ProviderError::Other(id))) => assert_eq!(id, "alpha"),
        other => panic!("alpha-origin id should route to alpha, got {other:?}"),
    }

    match client
        .movie_details(&MediaId::new("beta", NativeId::Num(1)))
        .await
    {
        Err(CameoClientError::Provider(ProviderError::Other(id))) => assert_eq!(id, "beta"),
        other => panic!("beta-origin id should route to beta, got {other:?}"),
    }

    // An id from an unregistered provider → Unsupported.
    let c = client
        .movie_details(&MediaId::new("gamma", NativeId::Num(1)))
        .await;
    assert!(
        matches!(c, Err(CameoClientError::Unsupported)),
        "unknown-origin id should be Unsupported, got {c:?}"
    );
}
