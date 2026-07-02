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
                │       └─ Generated REST client (vendored src/generated/tmdb.rs; regen via `cargo xtask codegen`)
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
| `src/generated/` | Vendored progenitor output (`tmdb.rs`), committed; regenerate with `cargo xtask codegen`, do not edit directly |
| `src/core/` | `CameoError`, `TimeWindow`, `PaginatedResponse`, `into_stream` |
| `src/cache/` | `CacheBackend` trait, `SqliteCache`, `CacheTtlConfig`, `CacheKey`, `MediaType` |
| `src/providers/tmdb/` | `TmdbClient` (rate-limited), `TmdbConfig`, `ImageUrl`, discover builders, `TmdbError` |
| `src/providers/anilist/` | `AniListClient` (GraphQL/reqwest), `AniListConfig`, `AniListError` |
| `src/unified/traits.rs` | Six provider traits: `SearchProvider`, `DetailProvider`, `DiscoveryProvider`, `RecommendationProvider`, `SeasonProvider`, `WatchProviderTrait` |
| `src/unified/models.rs` | `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, detail variants, `UnifiedSearchResult` |
| `src/unified/conversions/` | `Into` impls from provider types to unified types (tmdb/, anilist/ submodules) |
| `src/unified/facade/` | `CameoClient`, `CameoClientBuilder`, `CameoClientError`; dispatch across providers |
| `xtask/` | Standalone codegen tool (`cargo xtask codegen`) with pinned deps + committed lockfile; regenerates `src/generated/tmdb.rs` from the spec |

For feature flags, see README.md. For test commands, see TESTING.md.

---

## Regenerating the TMDB OpenAPI Spec

Prerequisites: `curl`, `jq`, `npx` (Node.js).

```bash
./scripts/fetch-openapi.sh
```

What it does:
1. Downloads the official TMDB spec from `developer.themoviedb.org`
2. Downgrades from OpenAPI 3.1.0 → 3.0.3 using a **pinned** `@apiture/openapi-down-convert` (progenitor requires 3.0.x); the version is pinned in the script so its output cannot drift with upstream releases
3. Applies `scripts/tmdb-spec-patch.jq` — a set of deterministic normalizations (see that file for per-fix rationale):
   - renames the TV-season `_id` field to `mongo_id` (avoids a duplicate `id` field in the generated code)
   - coerces integer `vote_average` schemas to `number`
   - coerces string-typed boolean defaults (`"default": "false"`) to real booleans
   - strips `example`/`examples` payloads (ignored by codegen, ~75% of the raw spec size)
4. Writes the result to `openapi/tmdb-api.json`

The pipeline is fully deterministic and requires **no manual post-editing**: given the same upstream spec, re-running the script reproduces `openapi/tmdb-api.json` byte-for-byte.

### Regenerating the vendored client

The generated progenitor client is **vendored** at `src/generated/tmdb.rs` and committed — a normal `cargo build` performs **no** code generation. After changing `openapi/tmdb-api.json`, regenerate the client with the dedicated tool:

```bash
cargo xtask codegen
```

This reads `openapi/tmdb-api.json`, runs progenitor, and rewrites `src/generated/tmdb.rs`. The `xtask/` crate has its own committed `Cargo.lock` with pinned codegen dependencies, so the output is reproducible — a CI freshness check (`.github/workflows/codegen-freshness.yml`) regenerates and fails the PR if the committed `src/generated/` is stale. Commit both `openapi/tmdb-api.json` and `src/generated/tmdb.rs` together.

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

## Releasing

Releases are fully automated via [release-plz](https://release-plz.dev/) — no manual steps on dev machines.

### How it works

1. Push conventional commits to `master` → `release-plz` creates or updates a **Release PR** with a version bump and generated changelog entry.
2. Review and merge the Release PR → `release-plz` publishes to crates.io, creates the git tag, and creates a GitHub Release.

Publishing authenticates via [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing): the workflow exchanges a GitHub OIDC token (`rust-lang/crates-io-auth-action`) for a short-lived crates.io token. No long-lived API token is stored in the repo.

### Prerequisites (one-time repo setup)

- **Actions PR write permission** — under *Settings → Actions → General*, set "Workflow permissions" to "Read and write permissions" and enable "Allow GitHub Actions to create and approve pull requests".
- **Trusted Publisher** — Trusted Publishing can only be configured *after* a crate exists on crates.io, so the **first** release must be bootstrapped with a token:
  1. Create a crates.io API token and add it as the repo secret `CARGO_REGISTRY_TOKEN`.
  2. Let `release-plz` publish the first version (`v0.1.0`).
  3. On crates.io → crate `cameo` → *Settings → Trusted Publishing*, add a publisher: owner `justin13888`, repo `cameo`, workflow `release-plz.yml`.
  4. **Delete the `CARGO_REGISTRY_TOKEN` secret.** All subsequent releases use OIDC with no stored secret.

### Day-to-day workflow

Write conventional commits (`feat:`, `fix:`, `refactor:`, `docs:`, etc.) — that's it.
When you're ready to ship, merge the Release PR that release-plz keeps open.

### Versioning

release-plz derives the next version from commit types:

| Commit type | Bump |
|---|---|
| `fix:` | patch (0.1.x) |
| `feat:` | minor (0.x.0) |
| Any `!` breaking change | major (x.0.0) |

### Skipping a release

Close the Release PR. release-plz will reopen it on the next push to `master` with accumulated changes.

### Emergency / manual publish

Steady-state releases need no token — only the one-time bootstrap (above) or an emergency manual publish does. For an emergency publish:

```bash
cargo publish
```

Set a temporary `CARGO_REGISTRY_TOKEN` in your environment and ensure tests pass first (`just publish-check`).

### Local changelog preview

```bash
just changelog   # requires: cargo install release-plz
```

---

## Known Generated-Type Quirks

These apply to types in `src/generated/` (auto-generated from `openapi/tmdb-api.json`):

| Type / Field | Quirk |
|---|---|
| `PersonDetailsResponse.deathday` | `Option<serde_json::Value>` — TMDB returns null or a date string; parse as string if `Some` |
| `PersonDetailsResponse.homepage` | `Option<serde_json::Value>` — same pattern as `deathday` |
| TV season details `mongo_id` field | TMDB's `_id` (MongoDB ObjectId) is renamed to `mongo_id` by `scripts/tmdb-spec-patch.jq` to avoid a progenitor collision with the `id` field |
| `trending_movies` / `trending_tv` | No `page` parameter — these endpoints do not support pagination in the TMDB spec |
| `vote_average` on TV list types | `scripts/tmdb-spec-patch.jq` coerces the integer-typed `vote_average` schemas to `number`; affects `DiscoverTvResponseResultsItem`, `TvSeriesPopularListResponseResultsItem`, `TvSeriesSimilarResponseResultsItem`, and similar |
