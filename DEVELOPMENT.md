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
just test         # run tests with cargo-nextest (default features = `full`: tmdb + anilist + tvdb + cache)
just test-cargo   # the same with plain `cargo test` (no cargo-nextest needed; includes doctests)
just test-all     # run tests with `--features full` (same set, stated explicitly)
just test-all-features  # run tests across every CI feature combination
just doc-test     # run doctests (nextest does not run them)
just test-live    # run live tests one at a time (requires TMDB_API_TOKEN in .env)
just doc          # build and open docs
just build        # build with default features
just build-features anilist  # AniList only
just mutants 'src/unified/date.rs'  # mutation-test one file or glob (needs cargo-mutants)
just mutants-diff # mutation-test only the lines changed since origin/master
just mutants-shard 3 16  # mutation-test one shard of the whole tree, as the weekly CI run does

# Run examples directly (just recipes don't cover examples)
TMDB_API_TOKEN=xxx cargo run --example facade_showcase -- 'Inception'
TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel -- 'Inception'
cargo run --example anilist_showcase --features anilist -- 'Your Name'
```

## Notes

- Lefthook is configured with pre-commit (`fmt` → `clippy-fix` → `check-modified`) and pre-push hooks
- Tests run under [cargo-nextest](https://nexte.st) (`cargo install --locked cargo-nextest`), configured in `.config/nextest.toml`
- AniList rate limit: 90 req/min — the nextest `live` profile (`just test-live`) runs live tests one at a time
- See TESTING.md for detailed test documentation

---

## Module Structure & Data Flow

```
User call → CameoClient (src/unified/facade/)
                ├─ TmdbClient (src/providers/tmdb/)
                │       └─ Generated REST client (vendored src/generated/tmdb.rs; regen via `cargo xtask codegen`)
                ├─ AniListClient (src/providers/anilist/)
                └─ TvdbClient (src/providers/tvdb/)
                            │
                            ▼  conversions (src/providers/<name>/convert/)
                     Unified types (src/unified/models.rs, src/unified/models/)
                            │
                            ▼  cache (src/cache/)
                     CacheBackend → SqliteCache (SQLite via rusqlite)
```

| Path | Purpose |
|---|---|
| `src/generated/` | Vendored spargen output (`tmdb.rs`, runtime support code embedded), committed; regenerate with `cargo xtask codegen`, do not edit directly |
| `src/core/` | `ProviderError`, `TransportError`, `DecodeError`, `TimeWindow`, `Page`, `PageToken`, `into_stream` |
| `src/cache/` | `CacheBackend` trait and `CacheError` (`backend.rs`), `CacheKey` and `MediaType` (`key.rs`), `SqliteCache` (`sqlite.rs`, `sqlite/`), `CacheTtlConfig` |
| `src/providers/http.rs` | Shared HTTP client construction (timeouts, `User-Agent`) for the built-in providers |
| `src/providers/rate_limit.rs` | Per-provider request-rate limiting (`RateLimit`) |
| `src/providers/retry.rs` | Retry policy for transient provider failures (`RetryPolicy`) |
| `src/providers/tmdb/` | `TmdbClient` (`client/`, one module per endpoint family), `TmdbConfig`, `ImageUrl`, discover builders (`builders/`), `TmdbError` (`error.rs`), conversions to unified types (`convert/`), capability-trait impls (`provider.rs`) |
| `src/providers/anilist/` | `AniListClient` (`client/`: GraphQL transport in `graphql.rs`, endpoints by family), `AniListConfig`, `DiscoverAnimeBuilder`, `AniListError`, GraphQL queries and response types, conversions (`convert/`), capability-trait impls (`provider.rs`) |
| `src/providers/tvdb/` | `TvdbClient`, `TvdbConfig`, `TvdbError`, response types, conversions (`convert/`), capability-trait impls (`provider.rs`) |
| `src/unified/traits.rs` | `Provider` (id + capabilities), the capability traits `SearchProvider`, `DetailProvider`, `DiscoveryProvider`, `RecommendationProvider`, `SeasonProvider`, `WatchAvailabilityProvider`, `ExternalIdProvider`, `CreditsProvider`, and the `MediaProvider` bundle; all return `ProviderError` |
| `src/unified/models.rs`, `src/unified/models/` | `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, detail variants, `UnifiedSearchResult`, credits |
| `src/unified/builders.rs`, `src/unified/builders/` | Constructors and builder-style setters for the unified models |
| `src/unified/date.rs` | `PartialDate` for release/air dates |
| `src/unified/genre.rs` | `Genre` and its provider mappings |
| `src/unified/media_id.rs` | `MediaId`, `NativeId`: provider-qualified ids |
| `src/unified/external_ids.rs` | `ExternalIds`, `ExternalSource`, `ExternalMatch` (IMDb, TVDB, MyAnimeList, Wikidata) |
| `src/unified/conversions/` | Shared conversion helpers (`native_id`); each provider's own conversions live in its `convert/` |
| `src/unified/facade/` | `CameoClient` (`mod.rs`), `CameoClientBuilder` (`builder.rs`), `CameoClientError` (`error.rs`), the facade cache layer (`cache.rs`), and one dispatch module per capability |
| `xtask/` | Standalone codegen tool (`cargo xtask codegen`) with pinned deps + committed lockfile; regenerates `src/generated/tmdb.rs` from the spec |

For feature flags, see README.md. For test commands, see TESTING.md.

---

## Regenerating the TMDB OpenAPI Spec

Prerequisites: `curl`, `jq`.

```bash
./scripts/fetch-openapi.sh
```

What it does:
1. Downloads the official TMDB spec (OpenAPI 3.1) from `developer.themoviedb.org`. It stays 3.1: spargen reads 3.1 natively and rejects 3.0, so there is no down-convert step
2. Applies `scripts/tmdb-spec-patch.jq` — a set of deterministic normalizations (see that file for per-fix rationale):
   - renames the TV-season `_id` field to `mongo_id` (avoids a duplicate `id` field in the generated code)
   - coerces integer `vote_average` schemas to `number`
   - coerces string-typed boolean defaults (`"default": "false"`) to real booleans
   - strips `example`/`examples` payloads (ignored by codegen, ~75% of the raw spec size)
   - drops `default` from object property schemas (operation parameter defaults are kept)
   - marks every optional response property nullable (`"type": [T, "null"]`), so it generates as `Option<T>` and accepts TMDB's explicit `null`s — arrays included
   - hoists each operation's `200` body into `components/schemas/<OperationId>Response` and each enum parameter into `<OperationId><Param>`, so generated types get stable names (`MovieDetailsResponse`, `TrendingMoviesTimeWindow`) instead of hashed ones
   - points the empty movie-recommendations response at the similar-movies schema
   - declares the v4 token as an `http` bearer scheme (`v4_bearer`) and adds the v3 key as an `api_key` query scheme (`v3_api_key`); either satisfies every operation
   - declares the `append_to_response` certification blocks on the details schemas: `release_dates` on `MovieDetailsResponse` and `content_ratings` on `TvSeriesDetailsResponse`
3. Writes the result to `openapi/tmdb-api.json`

The pipeline is fully deterministic and requires **no manual post-editing**: given the same upstream spec, re-running the script reproduces `openapi/tmdb-api.json` byte-for-byte.

### Regenerating the vendored client

The generated [spargen](https://github.com/getkono/spargen) client is **vendored** at `src/generated/tmdb.rs` and committed — a normal `cargo build` performs **no** code generation. After changing `openapi/tmdb-api.json`, regenerate the client with the dedicated tool:

```bash
cargo xtask codegen
```

This reads `openapi/tmdb-api.json`, runs spargen (pinned in `xtask/Cargo.toml`), and rewrites `src/generated/tmdb.rs`; generation either succeeds or fails with a diagnostic naming the spec construct and its JSON Pointer. No post-generation patch is applied: v3 `api_key` and v4 bearer auth are security schemes the generated client attaches itself (`TmdbClient` registers the configured one with `with_v4_bearer` or `with_v3_api_key`), and the generated client's `Debug` redacts the credential. spargen embeds its runtime in the output, so no spargen crate is a runtime dependency; the `tmdb` feature's `bytes`, `http`, `http-body`, `secrecy` and `url` dependencies (and the `jiff`/`reqwest`/`serde`/`serde_json` floors) are what that runtime needs; the `tokio` floor (1.53.3) follows spargen's, which is past RUSTSEC-2026-0386. `cargo xtask codegen` selects spargen's `jiff` date-time backend, so the generated `Date` wraps a `jiff::civil::Date`, the same type cameo's public API uses. The `xtask/` crate has its own committed `Cargo.lock`, so the output is reproducible — a CI freshness check (`.github/workflows/codegen-freshness.yml`) regenerates and fails the PR if the committed `src/generated/` is stale. Commit both `openapi/tmdb-api.json` and `src/generated/tmdb.rs` together.

---

## Adding a New Provider

A provider implements `Provider` plus whichever capability traits it supports and
is registered with `CameoClient::builder().register_*_provider(...)`; this needs
no changes to cameo. [docs/providers.md](docs/providers.md) is the guide for that
(traits, `ProviderError` mapping, registration, priority).

Only a provider that ships **inside** this crate as a built-in (like TMDB,
AniList and TheTVDB) needs the in-crate steps below, on top of that guide:

1. Add a feature flag in `Cargo.toml` (`<name> = [...]`, deps optional) and gate
   `src/providers/<name>/` on it.
2. Add `From<<Name>Error> for ProviderError` in `src/providers/<name>/error.rs`.
3. Add conversions to the unified models in `src/providers/<name>/convert/`.
4. Add `with_<name>(config)` to `CameoClientBuilder` (`src/unified/facade/builder.rs`)
   and, in `build()`, push the client into each capability list it implements.
   Built-ins register before out-of-crate providers; callers reorder with
   `with_priority`.
5. Add feature-gated tests in `tests/integration/` and any `[[example]]` with
   `required-features = ["<name>"]`.

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

cameo stays on `0.x` by design and never releases 1.0 (see
[docs/STABILITY.md](docs/STABILITY.md)). release-plz derives the next version
from commit types:

| Commit type | Bump |
|---|---|
| `fix:` | patch (0.y.z) |
| `feat:` | patch (0.y.z) |
| Any `!` breaking change | minor (0.y.0) |

Pull requests never change `version` in `Cargo.toml`; only the Release PR does.
CI's *SemVer compatibility* job therefore takes its release type from the commits
merged since the last `v*` tag plus the pull request's title: a breaking marker
(`type!:` or a `BREAKING CHANGE:` footer) in any of them allows breaking API
changes (a `0.(y+1)` release), and otherwise only additive changes pass (a
`0.y.(z+1)` release). A pull request that breaks the
public API must say so in its title, e.g. `feat!: ...`.

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
| TV season details `mongo_id` field | TMDB's `_id` (MongoDB ObjectId) is renamed to `mongo_id` by `scripts/tmdb-spec-patch.jq` to avoid a collision with the `id` field |
| `trending_movies` / `trending_tv_shows` | No `page` parameter — these endpoints do not support pagination in the TMDB spec |
| Response fields are `Option<T>` | `scripts/tmdb-spec-patch.jq` drops the spec's property `default`s (`0`, `true`) and marks every optional response property nullable, so each field — scalar or array (`genre_ids`, `results`, `genres` are `Option<Vec<T>>`) — reads a missing key and an explicit `null` alike as `None`. spargen decodes a non-nullable optional field strictly and would reject the `null`. Conversions keep unified `Option` fields `None` when TMDB omits or nulls a value, read a missing list as empty, `TmdbClient` fills a missing details id from the requested id, and list/search items without an id are dropped |
| Parameter types are aliases | spargen names every inline parameter schema by a hash (`Language80018710 = String`); they are plain type aliases, so `TmdbClient` passes `String`/`i32`/`bool` values directly |
| Watch-provider `results` | `scripts/tmdb-spec-patch.jq` declares `results` on every `*WatchProvidersResponse` as a map of one shared `WatchProviderCountry` schema keyed by country code, instead of the spec's one property (and type) per listed country, so `*WatchProvidersResponseResults` is a flattened `BTreeMap<String, WatchProviderCountry>` (`additional`) and countries the spec omits are kept |
| Details `release_dates` / `content_ratings` | `scripts/tmdb-spec-patch.jq` declares the block `append_to_response=release_dates` (movie) and `append_to_response=content_ratings` (TV) embed, as an optional, nullable reference to `MovieReleaseDatesResponse` / `TvSeriesContentRatingsResponse`; the spec omits both keys, so the block was dropped on deserialization. spargen generates a sibling struct (`MovieDetailsResponseReleaseDates`, `TvSeriesDetailsResponseContentRatings`) with the same fields and item types, and the field is `None` unless the append was requested |
| `vote_average` on TV list types | `scripts/tmdb-spec-patch.jq` coerces the integer-typed `vote_average` schemas to `number`; affects `DiscoverTvResponseResultsItem`, `TvSeriesPopularListResponseResultsItem`, `TvSeriesSimilarResponseResultsItem`, and similar |
