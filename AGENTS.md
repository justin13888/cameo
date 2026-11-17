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
The module map and data flow live in `DEVELOPMENT.md` (Module Structure & Data Flow).

- There is no `build.rs`: the TMDB client in `src/generated/tmdb.rs` is vendored
  spargen output, regenerated from `openapi/tmdb-api.json` with `cargo xtask codegen`
  (`xtask/src/main.rs`). Never edit it by hand.
- `openapi/tmdb-api.json` is itself generated: `scripts/fetch-openapi.sh` downloads the
  upstream spec and applies `scripts/tmdb-spec-patch.jq`. Fix the spec in that patch,
  never in the JSON. Commit the spec and `src/generated/` together.
- Implementing a new provider: see `docs/providers.md`.
