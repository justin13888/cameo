# ADR 0001 — Provider abstraction for v1

Status: **accepted** (2026-07)

## Context

The extension seam is the crux of v1, and today it does not exist: the concrete
providers do **not** implement the unified traits — only `CameoClient` does — so
the traits are just the facade's interface, not an extension point. The traits
use bare `async fn` (not object-safe, non-`Send` futures), each carries an
associated `type Error`, unified models carry `provider_id: String` while detail
lookups take a raw `i32`, and pagination hardwires TMDB's page/total shape.

This ADR settles the interlocking decisions once so the implementation issues
(#26–#31, #36) can proceed against a fixed target.

## Decision 1 — Async trait form

Use **`#[async_trait::async_trait]`** for the provider traits.

- It makes the traits **object-safe**, which is required for a `dyn` provider
  registry (`Arc<dyn SearchProvider>`).
- It boxes the returned futures as `Pin<Box<dyn Future + Send>>`, giving us the
  `Send` bound needed to `tokio::spawn` provider calls on a multi-thread runtime.
- `async-trait` is already a dependency; RPITIT (`-> impl Future`) is not
  object-safe without manual boxing and does not yet support `dyn`.

The small per-call allocation is acceptable for an I/O-bound HTTP SDK.

## Decision 2 — Unified `ProviderError`

Replace the per-trait associated `type Error` with a single concrete
`ProviderError` enum returned by every provider trait method. A concrete error
type is required for `dyn` dispatch (associated types are not `dyn`-compatible).

```rust
#[non_exhaustive]
pub enum ProviderError {
    NotFound,
    Unsupported,                                   // capability not offered
    RateLimited { retry_after: Option<Duration> },
    Auth(String),
    Api { status: u16, message: String },
    Transport(String),
    Deserialization(String),
    InvalidInput(String),
    Other(String),
}
```

Each provider maps its native error (`TmdbError`, `AniListError`) into
`ProviderError` via `From`. `CameoError` is deleted (dead API); the facade error
gains distinct `NotConfigured` vs `Unsupported` variants (#28, #30).

## Decision 3 — Typed `MediaId`

Replace `provider_id: String` and the raw `i32` detail parameters with a typed
`MediaId` that preserves provider origin and round-trips the current
`"provider:id"` string format.

```rust
pub struct MediaId { provider: Cow<'static, str>, native: NativeId }
pub enum NativeId { Num(u64), Text(String) }   // Text preserves e.g. "staff:95061"
```

- `MediaId::parse("tmdb:550")` → provider `"tmdb"`, native `Num(550)`.
- `MediaId::parse("anilist:staff:95061")` splits on the **first** `:`, giving
  native `Text("staff:95061")` — AniList person IDs keep their namespace.
- `Display`/`FromStr` round-trip losslessly; `Num` rejects negatives by
  construction. Detail lookups take `&MediaId`; the facade routes by
  `id.provider()` (#31).

## Decision 4 — Pagination contract

Replace `PaginatedResponse` (page/total_pages/total_results) with a
provider-agnostic, cursor-capable `Page<T>` whose has-next is authoritative and
whose totals are optional.

```rust
pub struct Page<T> {
    pub page: u32,
    pub results: Vec<T>,
    pub has_next: bool,               // authoritative (AniList hasNextPage; TMDB page<total_pages)
    pub next: Option<PageToken>,      // how to fetch the next page
    pub total_pages: u32,             // best-effort (0 when unknown)
    pub total_results: u32,           // best-effort (0 when unknown)
}
pub enum PageToken { Page(u32), Cursor(String) }
```

Totals are best-effort `u32` (`0` when a provider does not report them) rather
than `Option`, for ergonomics; iteration relies on the authoritative `has_next`
/ `next`, never on the totals.

- `into_stream` gains `Send` bounds so the stream can cross `tokio::spawn`.
- AniList populates `has_next` from `hasNextPage` (not reconstructed from nullable
  totals); TMDB uses checked page conversion and enforces the 500-page cap.

## Decision 5 — Capability discovery

A provider advertises the capabilities it supports; the facade distinguishes
"no provider configured" from "configured providers don't support this".

```rust
pub enum Capability { Search, Details, Discovery, Recommendations, Seasons, WatchAvailability }
```

- The base `Provider` trait exposes `fn id(&self) -> &str` and
  `fn capabilities(&self) -> &[Capability]` (or a set).
- The registry indexes providers by capability. A call with **no** provider
  configured → `NotConfigured`; a call where providers exist but none supports
  the capability → `Unsupported`.

## Decision 6 — Two-layer provider pattern

Each provider is two layers:

1. a **native low-level client** (`TmdbClient`, `AniListClient`) exposing the raw
   provider surface with the provider's own error type; and
2. a **unified trait implementation** (`impl SearchProvider for TmdbClient`, …)
   returning unified models and `ProviderError`.

A common provider config contract is documented in `docs/providers.md` (#53).
External crates implement the unified traits on their own client and register it
with the facade using only the public API (#30, #36, #53).

## Decision 7 — Trait naming and `TimeWindow`

- Rename `WatchProviderTrait` → **`WatchAvailabilityProvider`** (the old name
  collided conceptually with "watch providers" the data).
- `TimeWindow` (day/week) **stays** a unified trait parameter. Although TMDB
  originated it, day/week trending windows are a reasonable cross-provider
  concept; providers that lack the notion (AniList) document that they ignore it.

## Consequences

- `Arc<dyn SearchProvider>` (etc.) is constructible from both concrete providers,
  enabling a registry-based facade (#30) and out-of-crate providers (#53).
- Boxing futures and `ProviderError` string payloads trade a little performance
  and type richness for `dyn`-compatibility — the right call for an HTTP SDK.
- Unified models need public constructors/builders (#36) because external
  providers must build them through the public API.

## Sign-off

Decisions 1–7 accepted as the target for #26 (MediaId), #27 (pagination),
#28 (errors), #29 (traits + impls), #30 (registry facade), #31 (selection/
routing), and #36 (model constructors).
