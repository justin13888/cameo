# Development with Cameo

Prerequisites: `rustup`, `lefthook`, [`just`](https://github.com/casey/just) (command runner)

## Setup

```bash
just setup  # installs lefthook hooks and runs cargo build
```

## Common Commands

Run `just` to list all available recipes. Key ones:

```bash
just fmt          # format all code
just clippy       # run clippy lints
just test         # run tests (default features)
just test-all     # run tests including AniList
just test-live    # run live tests (requires TMDB_API_TOKEN in .env)
just doc          # build and open docs
just build        # build with default features
just build-features anilist  # AniList only

# Run examples directly (just recipes don't cover examples)
TMDB_API_TOKEN=xxx cargo run --example facade_showcase -- 'Inception'
TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel -- 'Inception'
cargo run --example anilist_showcase --features anilist -- 'Your Name'
```

## Notes

- Lefthook is configured with pre-commit (`fmt` → `clippy-fix` → `check-modified`) and pre-push hooks
- AniList rate limit: 90 req/min — run live tests with `--test-threads=1`
- See TESTING.md for detailed test documentation

---

## Module Structure & Data Flow

```
User call → CameoClient (src/unified/facade/)
                ├─ TmdbClient (src/providers/tmdb/)
                │       └─ Generated REST client (src/generated/ via build.rs + openapi/tmdb-api.json)
                └─ AniListClient (src/providers/anilist/)
                            │
                            ▼  conversions (src/unified/conversions/)
                     Unified types (src/unified/models.rs)
                            │
                            ▼  cache (src/cache/)
                     CacheBackend → SqliteCache (SQLite via rusqlite)
```

| Path | Purpose |
|---|---|
| `src/generated/` | Includes `OUT_DIR/tmdb_generated.rs` (progenitor output); do not edit directly |
| `src/core/` | `CameoError`, `TimeWindow`, `PaginatedResponse`, `into_stream` |
| `src/cache/` | `CacheBackend` trait, `SqliteCache`, `CacheTtlConfig`, `CacheKey`, `MediaType` |
| `src/providers/tmdb/` | `TmdbClient` (rate-limited), `TmdbConfig`, `ImageUrl`, discover builders, `TmdbError` |
| `src/providers/anilist/` | `AniListClient` (GraphQL/reqwest), `AniListConfig`, `AniListError` |
| `src/unified/traits.rs` | Six provider traits: `SearchProvider`, `DetailProvider`, `DiscoveryProvider`, `RecommendationProvider`, `SeasonProvider`, `WatchProviderTrait` |
| `src/unified/models.rs` | `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, detail variants, `UnifiedSearchResult` |
| `src/unified/conversions/` | `Into` impls from provider types to unified types (tmdb/, anilist/ submodules) |
| `src/unified/facade/` | `CameoClient`, `CameoClientBuilder`, `CameoClientError`; dispatch across providers |
| `build.rs` | Reads `openapi/tmdb-api.json`, calls progenitor, writes formatted code to `OUT_DIR/tmdb_generated.rs` |

For feature flags, see README.md. For test commands, see TESTING.md.

---

## Regenerating the TMDB OpenAPI Spec

Prerequisites: `curl`, `jq`, `npx` (Node.js).

```bash
./scripts/fetch-openapi.sh
```

What it does:
1. Downloads the official TMDB spec from `developer.themoviedb.org`
2. Downgrades from OpenAPI 3.1.0 → 3.0.3 using `@apiture/openapi-down-convert` (progenitor requires 3.0.x)
3. Writes to `openapi/tmdb-api.json`

**Manual fix required after regeneration:** In the TV season details schema, rename the field `_id` to `mongo_id`. This avoids a duplicate `id` field error in the progenitor-generated code.

`build.rs` watches `openapi/tmdb-api.json` and regenerates automatically on `cargo build`.

---

## Adding a New Provider

1. Create `src/providers/<name>/` with:
   - `mod.rs` — re-export submodules
   - `client.rs` — `<Name>Client` struct with `new(config)` constructor
   - `config.rs` — `<Name>Config` struct
   - `error.rs` — `<Name>Error` implementing `std::error::Error` (use `thiserror`)

2. Implement traits from `src/unified/traits.rs` on `<Name>Client`:
   - Required: `SearchProvider`, `DetailProvider`, `DiscoveryProvider`
   - Optional: `RecommendationProvider`, `SeasonProvider`, `WatchProviderTrait`
   - Each trait has an associated `Error` type — use `<Name>Error`

3. Add conversions in `src/unified/conversions/<name>/`:
   - `Into<UnifiedMovie>`, `Into<UnifiedTvShow>`, `Into<UnifiedPerson>` for provider response types

4. Add feature flag in `Cargo.toml`:
   ```toml
   [features]
   <name> = []  # add any required deps as optional
   ```

5. Wire the builder in `src/unified/facade/mod.rs`:
   - Add `#[cfg(feature = "<name>")] <name>_config: Option<<Name>Config>` to `CameoClientBuilder`
   - Add `pub fn with_<name>(mut self, config: <Name>Config) -> Self` method
   - Add `#[cfg(feature = "<name>")] <name>: Option<<Name>Client>` to `CameoClient`
   - Add a `<Name>` variant to `CameoClientError`

6. Plumb the client through each facade dispatch module (`search.rs`, `detail.rs`, `discovery.rs`, etc.):
   - Provider priority: TMDB first, then AniList, then new provider (or make priority configurable)

7. Add `#[cfg(feature = "<name>")]`-gated tests in `tests/integration/` and a `[[example]]` entry with `required-features = ["<name>"]`

---

## Known Generated-Type Quirks

These apply to types in `src/generated/` (auto-generated from `openapi/tmdb-api.json`):

| Type / Field | Quirk |
|---|---|
| `PersonDetailsResponse.deathday` | `Option<serde_json::Value>` — TMDB returns null or a date string; parse as string if `Some` |
| `PersonDetailsResponse.homepage` | `Option<serde_json::Value>` — same pattern as `deathday` |
| TV season details `mongo_id` field | TMDB's `_id` (MongoDB ObjectId) was renamed to `mongo_id` in the spec to avoid progenitor collision with the `id` field |
| `trending_movies` / `trending_tv` | No `page` parameter — these endpoints do not support pagination in the TMDB spec |
| `vote_average` on TV list types | The spec was patched to `Option<f64>` (was `i64`); affects `DiscoverTvResponseResultsItem`, `TvSeriesPopularListResponseResultsItem`, `TvSeriesSimilarResponseResultsItem`, and similar |
