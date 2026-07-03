# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/justin13888/cameo/releases/tag/v0.2.0) - 2026-07-03

### Added

- Schema versioning + provider-namespaced keys
- V3 api-key auth, config validation, image base URL
- Add constructors and builders to unified models
- Correct and complete unified model fields
- Introduce PartialDate for release/air dates
- Per-call provider selection, configurable priority, id-origin routing
- Rebuild the facade as a capability registry with dyn dispatch
- Rework provider traits and implement them on both providers
- Consolidate error surface — ProviderError, split NotConfigured/Unsupported
- Redesign pagination — Page<T> with authoritative has-next and Send streams
- Introduce typed MediaId replacing stringly-typed provider ids
- Retry transient failures with backoff honoring Retry-After
- Real per-provider token-bucket rate limiting
- Shared HTTP client construction with timeouts and User-Agent
- Body-before-status handling, structured errors, partial-data policy
- Decode structured error bodies with status, code, and Retry-After

### Fixed

- Harden the SQLite backend
- Remove conversion-layer hacks (show_id, fallbacks, hand parsing)
- Fix genre mapping defects

### Changed

- Remove flush_cache_writes; writes are read-your-writes
- Consolidate GraphQL queries, drop dead fields
- Deduplicate and type the discover builders
- Stop leaking generated types from the public API

### Documentation

- Add the provider implementation guide
- Graceful env handling + selection/capability demos
- Reconcile MSRV policy with the nightly-fmt exception
- Configure docs.rs, doc_cfg, and a cargo doc CI gate
- Add API stability policy
- Add community and security files
- Finalize and document the v1 CacheBackend contract
- Fix intra-doc link to TmdbClient in models module
- Add provider abstraction ADR (traits, IDs, pagination, errors, capabilities)


## [0.1.1](https://github.com/justin13888/cameo/releases/tag/v0.1.1) - 2026-05-22

### Fixed

- Pin typify >=0.6.2 and adapt TMDB conversions to bare f64 fields


## [0.1.0] - 2026-03-08

### Added

- `CameoClient` facade with six unified provider traits: `SearchProvider`, `DetailProvider`, `DiscoveryProvider`, `RecommendationProvider`, `SeasonProvider`, and `WatchProviderTrait`
- TMDB provider via progenitor-generated client covering all 152 API operations with a rate-limited async client (`TmdbClient`) and a `TmdbConfig` builder supporting concurrent request limiting, optional timeout, language, region, and adult content controls
- AniList provider (opt-in via `anilist` feature) with GraphQL-based search, detail, discovery, and staff queries; no authentication required; scores normalised to 0–10 scale
- Unified model types: `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, and their corresponding detail variants (`UnifiedMovieDetails`, `UnifiedTvShowDetails`, `UnifiedPersonDetails`)
- SQLite-backed caching layer (enabled via `cache` feature, on by default) with per-resource-type configurable TTLs (details 24 h, search 1 h, discovery 15 min, items 6 h), a `CacheBackend` trait for custom backends, and automatic expiry purging
- `PaginatedResponse` with `into_stream()` for lazy async streaming of paginated results
- Type-safe image URL resolution via `image_url` helpers with size enums: `PosterSize`, `BackdropSize`, `ProfileSize`, `StillSize`, `LogoSize`
- 33-genre taxonomy with `Genre::from_tmdb_id` and `Genre::from_anilist_genre` mappings, including AniList-specific genres (`Mecha`, `MahouShoujo`, `SliceOfLife`, `Sports`, `Supernatural`, `Ecchi`)
- `DiscoverMoviesBuilder` and `DiscoverTvBuilder` for type-safe discovery queries against TMDB
- `tracing` instrumentation on all provider methods for structured observability
- Feature flags: `tmdb`, `anilist`, `cache` (all on by default via `full`); `live-tests` gates tests that hit real APIs

[Unreleased]: https://github.com/justin13888/cameo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/justin13888/cameo/releases/tag/v0.1.0
