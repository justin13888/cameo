# cameo

Unified movie/TV show database SDK for Rust

## MANDATORY: Use td for Task Management

You must run td usage --new-session at conversation start (or after /clear) to see current work.
Use td usage -q for subsequent reads.

## Conventions

- Use strict Rust — avoid `unwrap()` in library code; prefer `?` and proper error types
- Write tests for all public API surface
- Use conventional commits (`type: description`)
- Keep functions small and focused
- Document all public items with `///` doc comments
- Errors should implement `std::error::Error` and be exposed in the crate's public API. `thiserror` crate may help.

## Architecture

This is a Rust library crate exposing a typed, ergonomic client for movie/TV metadata.
Actual structure:
- `src/lib.rs` — public API re-exports
- `src/core/` — pagination, error types, config (TimeWindow)
- `src/providers/tmdb/` — TmdbClient, TmdbConfig, builders, image_url
- `src/providers/anilist/` — AniListClient, AniListConfig, GraphQL queries
- `src/unified/` — traits, models, conversions, CameoClient facade
- `src/cache/` — CacheBackend trait, SqliteCache
- `src/generated/` — progenitor-generated TMDB client (build.rs)
- `build.rs` — progenitor code generation from openapi/tmdb-api.json
