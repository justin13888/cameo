# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/justin13888/cameo/releases/tag/v0.5.0) - 2026-11-22

### Added

- Bump spargen to 0.8.0 and replace chrono with jiff ([#188](https://github.com/justin13888/cameo/pull/188)) [**breaking**]
- CacheTtlConfig setters and typed details TTL helpers ([#132](https://github.com/justin13888/cameo/pull/132)) [**breaking**]
- Add CameoClientBuilder::with_cache_at for a caller-chosen cache path ([#185](https://github.com/justin13888/cameo/pull/185))
- Take u64 ids with checked conversion on TmdbClient methods ([#175](https://github.com/justin13888/cameo/pull/175)) [**breaking**]
- Fill unified details certifications from the release_dates / content_ratings append ([#169](https://github.com/justin13888/cameo/pull/169))
- Add a TheTVDB v4 provider ([#159](https://github.com/justin13888/cameo/pull/159))
- Validate AniList config and align provider base_url shapes ([#165](https://github.com/justin13888/cameo/pull/165)) [**breaking**]
- Add movie and TV series certification lookups ([#154](https://github.com/justin13888/cameo/pull/154))
- Cache movie and TV details for the age-aware TTL ([#152](https://github.com/justin13888/cameo/pull/152))
- Derive common traits consistently across sibling public types ([#140](https://github.com/justin13888/cameo/pull/140)) [**breaking**]

### Fixed

- Reject ids whose namespace does not match the requested entity ([#173](https://github.com/justin13888/cameo/pull/173)) [**breaking**]
- Route discover builders through the client's retry seam ([#171](https://github.com/justin13888/cameo/pull/171))
- Stop pages advertising pages past TMDB's 500-page limit ([#167](https://github.com/justin13888/cameo/pull/167)) [**breaking**]
- Parse MediaId native parts as numbers only when canonical ([#155](https://github.com/justin13888/cameo/pull/155)) [**breaking**]
- Index wrapped models in the item cache after search_multi ([#151](https://github.com/justin13888/cameo/pull/151))
- Skip invalid native ids and reject out-of-range page fields ([#145](https://github.com/justin13888/cameo/pull/145)) [**breaking**]
- Mark MediaIdParseError #[non_exhaustive] ([#146](https://github.com/justin13888/cameo/pull/146)) [**breaking**]
- Parse the leading sign of negative PartialDate years ([#143](https://github.com/justin13888/cameo/pull/143))
- Report omitted page totals as 0, as Page documents ([#141](https://github.com/justin13888/cameo/pull/141)) [**breaking**]
- Omit formats outside the movie and TV lists from mixed results ([#138](https://github.com/justin13888/cameo/pull/138))

### Changed

- Mark AniListConfig, CameoClientBuilder and CacheTtlConfig #[must_use] ([#179](https://github.com/justin13888/cameo/pull/179)) [**breaking**]
- Hide reqwest and serde_json errors behind TransportError and DecodeError ([#178](https://github.com/justin13888/cameo/pull/178)) [**breaking**]
- Hide reqwest and serde_json errors behind TransportError and DecodeError ([#177](https://github.com/justin13888/cameo/pull/177)) [**breaking**]
- Unify TmdbClient and AniListClient TV method names on the trait vocabulary ([#174](https://github.com/justin13888/cameo/pull/174)) [**breaking**]
- Remove the unused TmdbConfig image_base_url setting ([#172](https://github.com/justin13888/cameo/pull/172)) [**breaking**]
- Rename register_watch_provider to register_watch_availability_provider ([#170](https://github.com/justin13888/cameo/pull/170)) [**breaking**]
- Give every public item one canonical path by making leaf modules private ([#168](https://github.com/justin13888/cameo/pull/168)) [**breaking**]
- Split the eight baselined Rust sources below the file-length limit ([#166](https://github.com/justin13888/cameo/pull/166))
- Read watch-provider results as a per-country map ([#161](https://github.com/justin13888/cameo/pull/161))
- Split AniListClient into client/ modules and move conversions under providers/anilist/convert ([#162](https://github.com/justin13888/cameo/pull/162))
- Split TmdbClient by endpoint family and move conversions under providers/tmdb ([#158](https://github.com/justin13888/cameo/pull/158))
- Make Gender provider parsers crate-private and return None for unknown values ([#137](https://github.com/justin13888/cameo/pull/137)) [**breaking**]
- Share one retry loop and HTTP error helpers ([#157](https://github.com/justin13888/cameo/pull/157))
- Take &MediaId in CameoClient cache lookups ([#156](https://github.com/justin13888/cameo/pull/156)) [**breaking**]
- Split the CameoClient facade into error, builder and cache modules ([#149](https://github.com/justin13888/cameo/pull/149))
- Split model builders into per-family modules ([#142](https://github.com/justin13888/cameo/pull/142))

### Documentation

- Remove td task management instructions from agent guide
- Commit to permanent 0.x versioning ([#187](https://github.com/justin13888/cameo/pull/187))
- State errors and edge cases in doc comments and match them to their functions ([#180](https://github.com/justin13888/cameo/pull/180))
- Stop linking feature-gated builder methods from CameoClientBuilder docs ([#182](https://github.com/justin13888/cameo/pull/182))
- Correct public-API documentation drift ([#150](https://github.com/justin13888/cameo/pull/150))


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
