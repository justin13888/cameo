# cameo

**Unified movie/TV show database SDK for Rust**

[![Crates.io](https://img.shields.io/crates/v/cameo.svg)](https://crates.io/crates/cameo)
[![Docs.rs](https://docs.rs/cameo/badge.svg)](https://docs.rs/cameo)
[![CI](https://github.com/justin13888/cameo/actions/workflows/ci.yml/badge.svg)](https://github.com/justin13888/cameo/actions)
[![License](https://img.shields.io/crates/l/cameo.svg)](LICENSE-MIT)

## Overview

cameo is an async Rust SDK for querying movie and TV show metadata. It wraps the [TMDB API](https://developer.themoviedb.org/) and [AniList GraphQL API](https://anilist.gitbook.io/anilist-apiv2-docs/) behind a unified, ergonomic interface with type-safe models, transparent SQLite caching, and first-class pagination support.

## Installation

```toml
[dependencies]
cameo = "0.1"
```

### Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tmdb` | yes | TMDB provider support |
| `cache` | yes | SQLite caching layer (requires `rusqlite`) |
| `anilist` | yes | AniList GraphQL provider (anime; no API key required) |
| `live-tests` | no | Gates tests that hit the real TMDB API |

For a minimal install without caching: `cameo = { version = "0.1", default-features = false, features = ["tmdb"] }`

## Quick Start

```rust
use cameo::{CameoClient, TmdbConfig};
use cameo::SearchProvider;

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
| `tmdb_lowlevel` | Direct TmdbClient: pagination, images, discover builder, credits | `TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel -- "query"` |
| `anilist_showcase` | AniList search, details, discovery (no API key needed) | `cargo run --example anilist_showcase --features anilist -- "query"` |
| `cache_showcase` | File-backed/in-memory cache, custom TTLs, invalidation, clearing | `TMDB_API_TOKEN=xxx cargo run --example cache_showcase` |
| `error_handling` | Error variants, status matching, recovery patterns | `cargo run --example error_handling` |

## Architecture

```
CameoClient (unified facade)
├── SearchProvider / DetailProvider / DiscoveryProvider traits
├── Cache layer (optional — SqliteCache or custom CacheBackend)
├── TmdbClient
│   └── Generated progenitor client (152 TMDB API operations)
└── AniListClient
    └── GraphQL client (reqwest POST to graphql.anilist.co)
```

| Module | Purpose |
|--------|---------|
| `cameo::unified` | `CameoClient` facade, traits, unified model types, `Genre` |
| `cameo::providers::tmdb` | `TmdbClient`, `TmdbConfig`, `ImageUrl`, discover builders |
| `cameo::providers::anilist` | `AniListClient`, `AniListConfig`, `AniListError` |
| `cameo::cache` | `CacheBackend` trait, `SqliteCache`, `CacheTtlConfig` |
| `cameo::core` | `PaginatedResponse`, `into_stream`, `TimeWindow`, `CameoError` |
| `cameo::generated` | Progenitor-generated low-level TMDB client (do not use directly) |

## Testing

See [TESTING.md](TESTING.md) for the full test guide, run commands, and coverage matrix.

## Contributing

1. Clone the repo and run `cargo test` to verify your environment
2. Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, etc.
3. All public API items must have `///` doc comments
4. No `unwrap()` in library code — use `?` and proper error types
5. Add tests for all public API surface (unit tests preferred; wiremock for network calls)

## MSRV

The minimum supported Rust version is **1.88**. This crate uses [let-chains](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0.html) (`if let A = x && let B = y`) which were stabilized in Rust 1.88. While the primary user is [Beam](https://github.com/justin13888/beam), you may file any issues related to this.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
