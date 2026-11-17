# cameo

**Unified movie/TV show database SDK for Rust**

[![Crates.io](https://img.shields.io/crates/v/cameo.svg)](https://crates.io/crates/cameo)
[![Docs.rs](https://docs.rs/cameo/badge.svg)](https://docs.rs/cameo)
[![CI](https://github.com/justin13888/cameo/actions/workflows/ci.yml/badge.svg)](https://github.com/justin13888/cameo/actions)
[![License](https://img.shields.io/crates/l/cameo.svg)](LICENSE-MIT)

## Overview

cameo is an async Rust SDK for querying movie and TV show metadata. It wraps the [TMDB API](https://developer.themoviedb.org/), the [AniList GraphQL API](https://anilist.gitbook.io/anilist-apiv2-docs/) and the [TheTVDB v4 API](https://thetvdb.github.io/v4-api/) behind a unified, ergonomic interface with type-safe models, cross-database id mapping (IMDb, TVDB, MyAnimeList, Wikidata), transparent SQLite caching, and first-class pagination support.

## Installation

```toml
[dependencies]
cameo = "0.4"
```

### Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tmdb` | yes | TMDB provider support |
| `cache` | yes | SQLite caching layer (requires `rusqlite`) |
| `anilist` | yes | AniList GraphQL provider (anime; no API key required) |
| `tvdb` | yes | TheTVDB v4 provider (series, movies, default-order episodes with absolute numbers; API key required) |
| `live-tests` | no | Gates tests that hit the real provider APIs (TMDB and AniList) |

For a minimal install without caching: `cameo = { version = "0.4", default-features = false, features = ["tmdb"] }`

## Quick Start

```rust
use cameo::{CameoClient, TmdbConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(std::env::var("TMDB_API_TOKEN")?))
        .with_cache()
        .build()?;

    let results = client.search_movies("Inception", None).await?;
    for movie in &results.results {
        println!("{}: {}", movie.provider_id, movie.title);
    }
    Ok(())
}
```

## Examples

| Example | Covers | Run command |
|---------|--------|-------------|
| `facade_showcase` | Search, details, discovery, recommendations, seasons, watch providers | `TMDB_API_TOKEN=xxx cargo run --example facade_showcase -- "query"` |
| `tmdb_lowlevel` | Direct TmdbClient: pagination, images, discover builder, credits, trending, genres | `TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel -- "query"` |
| `anilist_showcase` | AniList search, details, discovery (no API key needed) | `cargo run --example anilist_showcase -- "query"` |
| `cache_showcase` | File-backed/in-memory cache, custom TTLs, invalidation, clearing | `TMDB_API_TOKEN=xxx cargo run --example cache_showcase` |
| `error_handling` | Error variants, status matching, recovery patterns | `cargo run --example error_handling` |

## Architecture

`CameoClient` is a facade over a registry of providers (TMDB, AniList and
TheTVDB built in, others pluggable via the capability traits), with an optional cache layer.
The module map and data flow are in [DEVELOPMENT.md](DEVELOPMENT.md#module-structure--data-flow);
the public API is documented on [docs.rs](https://docs.rs/cameo). To plug in
your own provider, see [docs/providers.md](docs/providers.md).

## Testing

See [TESTING.md](TESTING.md) for the full test guide, run commands, and coverage matrix.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the codegen-regeneration workflow and
the cache-version rule, and [SECURITY.md](SECURITY.md) for reporting
vulnerabilities and handling API tokens. Copy [`.env.example`](.env.example) to
`.env` to run the examples.

Prerequisites: `rustup`, `lefthook`, [`just`](https://github.com/casey/just) (command runner)

1. Clone the repo and run `just setup` to install hooks and build, then `just test` to verify your environment
2. Run `just` to see all available recipes
3. Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, etc.
4. All public API items must have `///` doc comments
5. No `unwrap()` in library code — use `?` and proper error types
6. Add tests for all public API surface (unit tests preferred; wiremock for network calls)

## Stability

See [docs/STABILITY.md](docs/STABILITY.md) for the API stability policy: what
the stable surface is, how `#[non_exhaustive]` is used, what counts as a
breaking change, and the deprecation cadence.

## MSRV

The minimum supported Rust version is the `rust-version` declared in [`Cargo.toml`](Cargo.toml), set by the `cache` feature's SQLite backend (`rusqlite`); the crate also relies on [let-chains](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0.html) (`if let A = x && let B = y`). While the primary user is [Beam](https://github.com/justin13888/beam), you may file any issues related to this.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
