# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
