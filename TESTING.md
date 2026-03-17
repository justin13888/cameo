# Testing Guide

## Test Pyramid

| Layer | Network | Location | Purpose |
|---|---|---|---|
| Unit | None | `tests/unit/` | Pure logic: conversions, builders, config, pagination, genre mapping, cache TTL |
| Wiremock (TMDB) | Mocked | `tests/integration/tmdb_*.rs` | Deserialization of real-shaped TMDB responses, generated-client field coverage |
| Wiremock (AniList) | Mocked | `tests/integration/anilist_*.rs` | AniList GraphQL response parsing, score conversion, format mapping |
| Facade | Mocked | `tests/integration/facade_dispatch_test.rs`, `tests/integration/cache_integration_test.rs` | CameoClient dispatch, cache read/write, provider fallback, NoProviders errors |
| Live (gated) | Real API | `tests/integration/tmdb_search_test.rs` etc. | Smoke tests — assert results are non-empty, don't assert specific values |

---

## Run Commands

```bash
# Default features (tmdb + cache): all unit + wiremock tests
cargo test

# Include AniList wiremock tests
cargo test --features anilist

# AniList only (no TMDB)
cargo test --no-default-features --features anilist

# Live TMDB tests (requires API token)
TMDB_API_TOKEN=xxx cargo test --features live-tests

# With a .env file (set -a exports all vars to child processes)
set -a; source .env; set +a && cargo test --features live-tests

# Everything (TMDB + AniList live tests; sequential to avoid AniList rate limits)
set -a; source .env; set +a && cargo test --all-features -- --test-threads=1
```

---

## Coverage Matrix

### TmdbClient (29 methods)

| Method | Unit | Wiremock | Facade | Live |
|---|---|---|---|---|
| `search_movies` | — | ✓ | ✓ (cache) | ✓ |
| `search_tv_shows` | — | ✓ | — | ✓ |
| `search_people` | — | ✓ | — | ✓ |
| `search_multi` | — | ✓ | ✓ (facade) | ✓ |
| `movie_details` | — | ✓ | ✓ (cache) | ✓ |
| `movie_details_with_append` | — | — | — | — |
| `tv_series_details` | — | ✓ | — | ✓ |
| `person_details` | — | ✓ | — | ✓ |
| `movie_credits` | — | ✓ | — | — |
| `tv_series_credits` | — | ✓ | — | — |
| `trending_movies` | — | ✓ | ✓ (facade) | ✓ |
| `trending_tv` | — | ✓ | — | ✓ |
| `popular_movies` | — | ✓ | ✓ (facade) | — |
| `popular_tv_shows` | — | ✓ | — | — |
| `top_rated_movies` | — | ✓ | — | — |
| `top_rated_tv_shows` | — | ✓ | — | — |
| `movie_recommendations` | — | ✓ | — | — |
| `tv_recommendations` | — | ✓ | — | — |
| `similar_movies` | — | ✓ | — | — |
| `similar_tv_shows` | — | ✓ | — | — |
| `tv_season_details` | — | ✓ | ✓ (facade) | — |
| `tv_episode_details` | — | ✓ | — | — |
| `movie_watch_providers` | — | ✓ | ✓ (facade) | — |
| `tv_watch_providers` | — | ✓ | — | — |
| `movie_genres` | — | ✓ | — | — |
| `tv_genres` | — | ✓ | — | — |
| `movie_images` | — | ✓ | — | — |
| `discover_movies` | — | ✓ | — | ✓ |
| `discover_tv` | — | ✓ | — | ✓ |

### AniListClient (13 methods)

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
| `trending_tv` | — | ✓ |
| `popular_movies` | — | ✓ |
| `popular_tv_shows` | — | ✓ |
| `top_rated_movies` | — | ✓ |
| `top_rated_tv_shows` | — | ✓ |

### CameoClient Facade (22 trait methods)

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
| `RecommendationProvider` | `movie_recommendations` | — | — |
| `RecommendationProvider` | `tv_recommendations` | — | — |
| `RecommendationProvider` | `similar_movies` | — | — |
| `RecommendationProvider` | `similar_tv_shows` | — | — |
| `SeasonProvider` | `season_details` | ✓ (facade) | — |
| `SeasonProvider` | `episode_details` | — | — |
| `WatchProviderTrait` | `movie_watch_providers` | ✓ (facade) | — |
| `WatchProviderTrait` | `tv_watch_providers` | — | — |

### CameoClient Cache API (9 methods)

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
| `flush_cache_writes` | ✓ (used in all cache tests) |

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
| `trending_tv_response.json` | `/3/trending/tv/week` |
| `discover_tv_response.json` | `/3/discover/tv` |

AniList detail tests (`anilist_details_test.rs`) use `include_str!` fixture files (`anilist_media_details_response.json`, `anilist_staff_details_response.json`). Search and discovery tests use inline JSON constructed via `serde_json::json!` helpers — no separate fixture files for those.

---

## Guidelines for Adding Tests

1. **Every new public method** → wiremock test with fixture (or inline JSON for AniList).
2. **Conversion logic** (e.g. new `Into` impl, new genre variant) → unit test in `tests/unit/conversion_test.rs` or `tests/unit/genre_test.rs`.
3. **Facade dispatch** → one test per dispatch category (not per method). Test that the right provider is called and the result converts correctly.
4. **Live tests** → smoke-only. Assert results are non-empty; never assert specific titles or IDs that could change.
5. **Fixtures** → capture from real API, trim to 1–2 results, name as `{resource}_response.json`.

---

## Test Count Summary

| Layer | Count |
|---|---|
| Unit | 64 |
| Wiremock (TMDB) | 33 |
| Wiremock (AniList) | 25 |
| Facade / Cache | 16 |
| Live (TMDB) | 8 |
| **Total** | **~146** |
