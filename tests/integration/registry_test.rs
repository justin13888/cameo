//! An out-of-crate-style provider registers with the facade using only the
//! public API — proving adding a provider requires zero facade edits (#30).

use std::sync::Arc;

use async_trait::async_trait;
use cameo::{
    Page,
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
    let detail = client.movie_details(1).await;
    assert!(
        matches!(detail, Err(CameoClientError::Unsupported)),
        "expected Unsupported, got {detail:?}"
    );
}
