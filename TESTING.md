# Testing Guide

## Test Pyramid

| Layer | Network | Location | Purpose |
|---|---|---|---|
| Unit | None | `tests/unit/` | Pure logic: conversions, builders, config, pagination, genre mapping, cache TTL |
| Wiremock (TMDB) | Mocked | `tests/integration/tmdb_*.rs` | Deserialization of real-shaped TMDB responses, generated-client field coverage |
| Wiremock (AniList) | Mocked | `tests/integration/anilist_*.rs` | AniList GraphQL response parsing, score conversion, format mapping, list-result episodes/duration/status |
| Facade | Mocked | `tests/integration/facade_dispatch_test.rs`, `tests/integration/cache_integration_test.rs` | CameoClient dispatch, cache read/write, provider fallback, `NotConfigured` / `Unsupported` errors |
| Live (gated) | Real API | `tests/integration/tmdb_search_test.rs` etc. | Smoke tests — assert results are non-empty, don't assert specific values |

---

## Run Commands

Tests run under [cargo-nextest](https://nexte.st), which runs every test in its
own process; install it with `cargo install --locked cargo-nextest`. Its
settings live in `.config/nextest.toml`:

- **`default` profile**: every test except the `live_*` ones, so
  `cargo nextest run --all-features` passes without a `TMDB_API_TOKEN`. No
  retries, so a flaky test fails. A test still running after 2 minutes is
  killed.
- **`ci` profile**: the same, without stopping at the first failure. CI uses it.
- **`live` profile**: only the `live_*` tests, one at a time.

nextest does not run doctests: run them with `cargo test --doc` (`just doc-test`).
Plain `cargo test` still works (`just test-cargo`), but it has no live-test
filter, so `cargo test --all-features` runs the live tests and they panic
without a token.

```bash
# Default features (`full` = tmdb + anilist + tvdb + cache): all unit + wiremock tests
cargo nextest run

# Doctests
cargo test --doc --features full

# Every CI feature combination, tests and doctests (what the pre-push hook runs)
just test-all-features

# AniList only (no TMDB)
cargo nextest run --no-default-features --features anilist

# Every feature; live tests are still filtered out
cargo nextest run --all-features

# Live TMDB + AniList tests, one at a time (requires TMDB_API_TOKEN in env or .env)
just test-live
TMDB_API_TOKEN=xxx cargo nextest run --profile live --features full,live-tests
```

---

## Mutation Testing

[cargo-mutants](https://mutants.rs) changes the library's code one small edit at a
time (a `+` becomes `-`, a function returns `Default::default()`, a match arm is
deleted) and runs the tests against each edit. A mutant the tests still pass on
("missed") marks behaviour no test constrains. Install it with
`cargo install --locked cargo-mutants`; its settings live in
`.cargo/mutants.toml`, which excludes the generated client under `src/generated/`.

```bash
# Every mutant in one file or glob; the report lands in mutants.out/
just mutants 'src/providers/tmdb/**/*.rs'

# Only the mutants on lines changed since a base ref
just mutants-diff origin/master
```

CI (`.github/workflows/mutants.yml`) runs the in-diff form on every pull request
with a 30-minute limit and fails it when a mutant on a changed line is missed or
times out. When it fails, add a test that fails against that mutant, or, where
the mutant cannot change observable behaviour, say why in the pull request.

---

## Coverage Matrix

### TmdbClient

| Method | Unit | Wiremock | Facade | Live |
|---|---|---|---|---|
| `search_movies` | — | ✓ | ✓ (cache) | ✓ |
| `search_tv_shows` | — | ✓ | — | ✓ |
| `search_people` | — | ✓ | — | ✓ |
| `search_multi` | — | ✓ | ✓ (facade) | ✓ |
| `movie_details` | — | ✓ | ✓ (cache) | ✓ |
| `movie_details_with_append` | — | — | — | — |
| `tv_show_details` | — | ✓ | — | ✓ |
| `person_details` | — | ✓ | — | ✓ |
| `movie_credits` | — | ✓ | — | — |
| `tv_show_credits` | — | ✓ | — | — |
| `trending_movies` | — | ✓ | ✓ (facade) | ✓ |
| `trending_tv_shows` | — | ✓ | — | ✓ |
| `popular_movies` | — | ✓ | ✓ (facade) | — |
| `popular_tv_shows` | — | ✓ | — | — |
| `top_rated_movies` | — | ✓ | — | — |
| `top_rated_tv_shows` | — | ✓ | — | — |
| `movie_recommendations` | — | ✓ | — | — |
| `tv_recommendations` | — | ✓ | — | — |
| `similar_movies` | — | ✓ | — | — |
| `similar_tv_shows` | — | ✓ | — | — |
| `season_details` | — | ✓ | ✓ (facade) | — |
| `episode_details` | — | ✓ | — | — |
| `movie_watch_providers` | — | ✓ | ✓ (facade) | — |
| `tv_watch_providers` | — | ✓ | — | — |
| `movie_external_ids` | — | ✓ | — | — |
| `tv_external_ids` | — | ✓ | ✓ (facade) | — |
| `find_by_external_id` | — | ✓ | ✓ (facade) | — |
| `movie_genres` | — | ✓ | — | — |
| `tv_show_genres` | — | ✓ | — | — |
| `movie_images` | — | ✓ | — | — |
| `discover_movies` | — | ✓ | — | ✓ |
| `discover_tv` | — | ✓ | — | ✓ |

### AniListClient

| Method | Unit | Wiremock |
|---|---|---|
| `search_movies` | ✓ | ✓ |
| `search_tv_shows` | — | ✓ |
| `search_people` | — | ✓ |
| `search_multi` | — | — |
| `movie_details` | — | ✓ |
| `tv_show_details` | — | ✓ |
| `person_details` | — | ✓ |
| `trending_movies` | — | ✓ |
| `trending_tv_shows` | — | ✓ |
| `popular_movies` | — | ✓ |
| `popular_tv_shows` | — | ✓ |
| `top_rated_movies` | — | ✓ |
| `top_rated_tv_shows` | — | ✓ |
| `discover_anime` (`execute_movies` / `execute_tv`) | ✓ | ✓ |
| `movie_recommendations` | — | ✓ |
| `tv_recommendations` | — | ✓ |
| `external_ids` | — | ✓ |
| `find_by_mal_id` | — | ✓ |
| `media_credits` | — | ✓ |

AniList's `RecommendationProvider` impl (incl. `similar_*` → `Unsupported`) is covered in `anilist_recommendations_test.rs`.

### CameoClient Facade (capability trait methods)

| Trait | Method | Wiremock | Cache |
|---|---|---|---|
| `SearchProvider` | `search_movies` | ✓ (facade) | ✓ |
| `SearchProvider` | `search_tv_shows` | — | — |
| `SearchProvider` | `search_people` | — | — |
| `SearchProvider` | `search_multi` | ✓ (facade) | — |
| `DetailProvider` | `movie_details` | — | ✓ |
| `DetailProvider` | `tv_show_details` | — | — |
| `DetailProvider` | `person_details` | — | — |
| `DiscoveryProvider` | `trending_movies` | ✓ (facade) | — |
| `DiscoveryProvider` | `trending_tv_shows` | — | — |
| `DiscoveryProvider` | `popular_movies` | ✓ (facade) | — |
| `DiscoveryProvider` | `top_rated_movies` | — | — |
| `DiscoveryProvider` | `popular_tv_shows` | — | — |
| `DiscoveryProvider` | `top_rated_tv_shows` | — | — |
| `RecommendationProvider` | `movie_recommendations` | ✓ (facade, AniList) | — |
| `RecommendationProvider` | `tv_recommendations` | ✓ (facade, AniList) | — |
| `RecommendationProvider` | `similar_movies` | — | — |
| `RecommendationProvider` | `similar_tv_shows` | ✓ (facade, AniList `Unsupported`) | — |
| `SeasonProvider` | `season_details` | ✓ (facade) | — |
| `SeasonProvider` | `episode_details` | — | — |
| `WatchAvailabilityProvider` | `movie_watch_providers` | ✓ (facade) | — |
| `WatchAvailabilityProvider` | `tv_watch_providers` | — | — |
| `ExternalIdProvider` | `movie_external_ids` | ✓ (registry stub) | — (not cached) |
| `ExternalIdProvider` | `tv_external_ids` | ✓ (facade) | — (not cached) |
| `ExternalIdProvider` | `find_by_external_id` | ✓ (facade, fan-out/merge via registry stubs) | — (not cached) |
| `CreditsProvider` | `movie_credits` | ✓ (facade) | — (not cached) |
| `CreditsProvider` | `tv_show_credits` | ✓ (facade, TMDB and AniList) | — (not cached) |

### CameoClient Cache API

| Method | Test |
|---|---|
| `cached_movie` | ✓ |
| `cached_movie_details` | ✓ |
| `cached_tv_show` | — |
| `cached_tv_show_details` | — |
| `cached_person` | — |
| `cached_person_details` | — |
| `invalidate_cached` | ✓ |
| `clear_cache` | ✓ |

---

## Fixtures

All fixtures live in `tests/fixtures/` and are trimmed to 1–2 results for speed. Name convention: `{resource}_response.json`.

| Fixture | TMDB Endpoint |
|---|---|
| `search_movie_response.json` | `/3/search/movie` |
| `search_tv_response.json` | `/3/search/tv` |
| `search_person_response.json` | `/3/search/person` |
| `search_multi_response.json` | `/3/search/multi` |
| `movie_details_response.json` | `/3/movie/{id}` |
| `tv_details_response.json` | `/3/tv/{id}` |
| `person_details_response.json` | `/3/person/{id}` |
| `movie_recommendations_response.json` | `/3/movie/{id}/recommendations` |
| `tv_recommendations_response.json` | `/3/tv/{id}/recommendations` |
| `movie_watch_providers_response.json` | `/3/movie/{id}/watch/providers` |
| `tv_season_details_response.json` | `/3/tv/{id}/season/{n}` |
| `tv_popular_response.json` | `/3/tv/popular` |
| `tv_top_rated_response.json` | `/3/tv/top_rated` |
| `trending_movies_response.json` | `/3/trending/movie/week` |
| `popular_movies_response.json` | `/3/movie/popular` |
| `top_rated_movies_response.json` | `/3/movie/top_rated` |
| `movie_credits_response.json` | `/3/movie/{id}/credits` |
| `tv_credits_response.json` | `/3/tv/{id}/aggregate_credits` |
| `discover_movies_response.json` | `/3/discover/movie` |
| `similar_movies_response.json` | `/3/movie/{id}/similar` |
| `similar_tv_response.json` | `/3/tv/{id}/similar` |
| `movie_genres_response.json` | `/3/genre/movie/list` |
| `movie_images_response.json` | `/3/movie/{id}/images` |
| `tv_genres_response.json` | `/3/genre/tv/list` |
| `tv_episode_details_response.json` | `/3/tv/{id}/season/{n}/episode/{n}` |
| `tv_watch_providers_response.json` | `/3/tv/{id}/watch/providers` |
| `movie_external_ids_response.json` | `/3/movie/{id}/external_ids` |
| `tv_external_ids_response.json` | `/3/tv/{id}/external_ids` |
| `find_by_imdb_id_response.json` | `/3/find/{id}?external_source=imdb_id` |
| `find_by_tvdb_id_response.json` | `/3/find/{id}?external_source=tvdb_id` |
| `trending_tv_response.json` | `/3/trending/tv/week` |
| `discover_tv_response.json` | `/3/discover/tv` |

AniList detail tests (`anilist_details_test.rs`) use `include_str!` fixture files (`anilist_media_details_response.json`, `anilist_staff_details_response.json`). Search, discovery, discover-builder (`anilist_discover_test.rs`), recommendation (`anilist_recommendations_test.rs`), and external-id (`anilist_external_ids_test.rs`) tests use inline JSON constructed via `serde_json::json!` helpers — no separate fixture files for those.

---

## Guidelines for Adding Tests

1. **Every new public method** → wiremock test with fixture (or inline JSON for AniList).
2. **Conversion logic** (e.g. new `Into` impl, new genre variant) → `#[cfg(test)]` unit test beside the conversion in `src/unified/conversions/`, or in `tests/unit/genre_test.rs` for genre mapping.
3. **Facade dispatch** → one test per dispatch category (not per method). Test that the right provider is called and the result converts correctly.
4. **Live tests** → smoke-only. Assert results are non-empty; never assert specific titles or IDs that could change.
5. **Fixtures** → capture from real API, trim to 1–2 results, name as `{resource}_response.json`.
