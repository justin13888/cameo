# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - YYYY-MM-DD

### Added

- `CameoClient` facade with `SearchProvider`, `DetailProvider`, and `DiscoveryProvider` traits for a unified provider interface
- TMDB provider via progenitor-generated client covering 152 API operations
- Unified model types: `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, and their corresponding detail variants
- SQLite-backed caching layer (enabled via `cache` feature) with configurable TTLs per resource type and a `CacheBackend` trait for custom backends
- `PaginatedResponse` with `into_stream()` for lazy streaming of paginated results
- Type-safe image URL resolution with size enums: `PosterSize`, `BackdropSize`, `ProfileSize`, `StillSize`, `LogoSize`
- 28-genre taxonomy with `Genre::from_tmdb_id` mapping for normalised genre identifiers
- `TmdbConfig` builder supporting rate limiting, language, region, and adult content controls

[Unreleased]: https://github.com/justin13888/cameo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/justin13888/cameo/releases/tag/v0.1.0
