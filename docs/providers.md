# Implementing a provider

cameo's facade is a capability registry: a provider is any type that implements
one or more capability traits and is registered with `CameoClient::builder()`.
**Adding a provider requires no changes to cameo** — everything below uses only
the public API. See [ADR 0001](adr/0001-provider-abstraction.md) for the design
rationale; a complete, compiling example lives in
`tests/integration/registry_test.rs` (`StubProvider`).

## 1. Identity and capabilities

Every provider implements [`Provider`], which gives it a stable string id and
declares which capabilities it supports:

```rust
use cameo::{Capability, Provider};

struct TvdbProvider { /* http client, config, … */ }

impl Provider for TvdbProvider {
    fn id(&self) -> &str { "tvdb" }
    fn capabilities(&self) -> &[Capability] {
        &[Capability::Search, Capability::Details]
    }
}
```

The id is how callers select the provider (`search_movies_with("tvdb", …)`) and
how ids minted by this provider route back to it (`MediaId::new("tvdb", …)`).

## 2. The two-layer pattern

Mirror the built-in providers: keep an inherent low-level client that speaks the
upstream API and returns **stable, hand-owned types** (or unified models), then
adapt it to the capability traits. The trait impls should be thin — convert to
unified models and map errors. `TmdbClient` (inherent methods) +
`impl SearchProvider for TmdbClient` (`src/providers/tmdb/`) is the reference.

## 3. Config contract

Take configuration as a `Config` struct with a `new(...)` constructor and
`with_*` setters, validated at client construction with a clear error. Never
require callers to build a `#[non_exhaustive]` struct literal.

## 4. Implement the capability traits

Each capability is one `#[async_trait]` trait. Implement the ones you declared.
Because they use `#[async_trait]`, reuse cameo's re-export so you need no extra
dependency:

```rust
use std::sync::Arc;
use cameo::async_trait::async_trait;
use cameo::{MediaId, Page, SearchProvider, UnifiedMovie, UnifiedTvShow,
    UnifiedPerson, UnifiedSearchResult, core::error::ProviderError};

#[async_trait]
impl SearchProvider for TvdbProvider {
    async fn search_movies(&self, query: &str, page: Option<u32>)
        -> Result<Page<UnifiedMovie>, ProviderError> {
        // 1. call your low-level client
        // 2. convert results to UnifiedMovie (use the new()/with_* builders)
        // 3. map any error to ProviderError
        Ok(Page::with_has_next(1, Vec::new(), false, 0, 0))
    }
    // search_tv_shows / search_people / search_multi …
}
```

Build the unified values with the model builders (`UnifiedMovie::new(id, title)
.with_*(…)`) — the models are `#[non_exhaustive]`, so struct literals won't
compile outside cameo. Mint ids with `MediaId::new(self.id(), NativeId::…)`.

## 5. Error mapping

Return [`ProviderError`]: map not-found to `NotFound`, auth failures to `Auth`,
rate limits to `RateLimited { retry_after }`, unsupported operations to
`Unsupported`, and anything else to the appropriate variant. The facade
translates these into `CameoClientError` for callers.

## 6. Register with the facade

Register one `Arc` per capability trait you implement — no facade edits:

```rust
let client = CameoClient::builder()
    .register_search_provider(Arc::new(TvdbProvider::new(config)))
    .with_priority(["tvdb", "tmdb"]) // optional default order
    .build()?;

let results = client.search_movies("Dune", None).await?;      // default priority
let tvdb = client.search_movies_with("tvdb", "Dune", None).await?; // explicit
```

Detail/season/recommendation/watch calls route by the origin of the `MediaId`
you pass, so ids your provider minted come back to it automatically. A capability
no configured provider supports yields `CameoClientError::Unsupported`.

[`Provider`]: https://docs.rs/cameo/latest/cameo/trait.Provider.html
[`ProviderError`]: https://docs.rs/cameo/latest/cameo/enum.ProviderError.html
