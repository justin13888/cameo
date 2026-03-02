# cameo

**Unified movie/TV show database SDK for Rust**

[![Crates.io](https://img.shields.io/crates/v/cameo.svg)](https://crates.io/crates/cameo)
[![Docs.rs](https://docs.rs/cameo/badge.svg)](https://docs.rs/cameo)
[![CI](https://github.com/justin13888/cameo/actions/workflows/ci.yml/badge.svg)](https://github.com/justin13888/cameo/actions)
[![License](https://img.shields.io/crates/l/cameo.svg)](LICENSE-MIT)

## Overview

cameo is an async Rust SDK for querying movie and TV show metadata. It wraps the [TMDB API](https://developer.themoviedb.org/) behind a unified, ergonomic interface with type-safe models, transparent SQLite caching, and first-class pagination support.

**Key capabilities:**

- Ergonomic async Rust API over TMDB (152 operations via progenitor code generation)
- Unified model types: `UnifiedMovie`, `UnifiedTvShow`, `UnifiedPerson`, and their detailed variants
- Transparent SQLite caching with per-category TTL control
- Paginated response helpers (`has_next_page`, `next_page`) and lazy streaming via `into_stream`
- Type-safe image URL resolution for poster, backdrop, profile, still, and logo sizes
- 28-genre taxonomy spanning movies and TV, with `Genre::from_tmdb_id` mapping

## Installation

```toml
[dependencies]
cameo = "0.1"
```

### Feature flags

| Feature | Default | Description |
|||-|
| `tmdb` | yes | TMDB provider support |
| `cache` | yes | SQLite caching layer (requires `rusqlite`) |
| `live-tests` | no | Gates tests that hit the real TMDB API |

**Minimal install (no cache):**

```toml
cameo = { version = "0.1", default-features = false, features = ["tmdb"] }
```

## Quick Start

```rust
use cameo::{CameoClient, TmdbConfig};
use cameo::unified::SearchProvider;

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

## Usage

### Searching

```rust
use cameo::unified::SearchProvider;

// Search for movies
let movies = client.search_movies("Dune", None).await?;

// Search for TV shows
let shows = client.search_tv_shows("Breaking Bad", None).await?;

// Search for people
let people = client.search_people("Christopher Nolan", None).await?;

// Multi-search across movies, TV, and people
let mixed = client.search_multi("Dune", None).await?;
```

**Matching on `UnifiedSearchResult`:**

```rust
use cameo::unified::models::UnifiedSearchResult;

for result in &mixed.results {
    match result {
        UnifiedSearchResult::Movie(m) => println!("Movie: {}", m.title),
        UnifiedSearchResult::TvShow(t) => println!("TV: {}", t.name),
        UnifiedSearchResult::Person(p) => println!("Person: {}", p.name),
    }
}
```

**Pagination:**

```rust
let page1 = client.search_movies("Dune", None).await?;

if page1.has_next_page() {
    let page2 = client.search_movies("Dune", page1.next_page()).await?;
}
```

### Getting Details

```rust
use cameo::unified::DetailProvider;

// Movie details by numeric TMDB ID
let movie = client.movie_details(550).await?;
println!("{} — {}", movie.movie.title, movie.tagline.as_deref().unwrap_or(""));
println!("Runtime: {} min", movie.runtime.unwrap_or(0));
println!("Budget: ${}", movie.budget.unwrap_or(0));

// TV show details
let show = client.tv_show_details(1396).await?;
println!("{} — {} seasons", show.show.name, show.number_of_seasons);

// Person details
let person = client.person_details(138).await?;
println!("{}", person.biography.as_deref().unwrap_or("No bio"));
```

### Discovery & Trending

```rust
use cameo::core::config::TimeWindow;
use cameo::unified::DiscoveryProvider;

// Trending movies (daily or weekly)
let daily = client.trending_movies(TimeWindow::Day, None).await?;
let weekly = client.trending_movies(TimeWindow::Week, None).await?;

// Trending TV shows
let tv = client.trending_tv_shows(TimeWindow::Week, None).await?;

// Popular and top-rated movies
let popular = client.popular_movies(None).await?;
let top_rated = client.top_rated_movies(None).await?;
```

### Pagination Streaming

Lazily fetch all pages as an async `Stream`:

```rust
use futures::StreamExt;
use cameo::core::pagination::into_stream;

let stream = into_stream(|page| {
    let client = client.clone();
    async move { client.search_movies("Batman", Some(page)).await }
});

tokio::pin!(stream);
while let Some(result) = stream.next().await {
    let movie = result?;
    println!("{}", movie.title);
}
```

### Genres

```rust
use cameo::unified::genre::Genre;

// Map a TMDB genre ID to the Genre enum
let genre = Genre::from_tmdb_id(28);
// Genre::Action

// Display name
println!("{}", genre.name()); // "Action"

// Unknown genres are wrapped
let unknown = Genre::from_tmdb_id(99999);
// Genre::Other(UnknownGenre::TmdbId(99999))
```



## TMDB Provider (Direct Access)

`CameoClient` covers the most common operations. For TMDB-specific features like credits, images, or advanced discovery queries, use `TmdbClient` directly.

```rust
use cameo::{TmdbClient, TmdbConfig};

let config = TmdbConfig::new(token)
    .with_language("en-US")
    .with_region("US")
    .with_include_adult(false)
    .with_rate_limit(40);

let client = TmdbClient::new(config)?;
```

**Extended TMDB calls:**

```rust
// Cast & crew
let credits = client.movie_credits(550).await?;
let agg_credits = client.tv_series_aggregate_credits(1396).await?;

// Images
let images = client.movie_images(550).await?;

// Discover with filters
use cameo::generated::tmdb::types::DiscoverMovieSortBy;
let results = client.discover_movies()
    .sort_by(DiscoverMovieSortBy::PopularityDesc)
    .primary_release_year(2024)
    .execute()
    .await?;
```

### Image URLs

```rust
use cameo::providers::tmdb::image_url::{ImageUrl, PosterSize, BackdropSize, ProfileSize};

let poster = ImageUrl::poster("/path.jpg", PosterSize::W500);
let backdrop = ImageUrl::backdrop("/path.jpg", BackdropSize::W1280);
let profile = ImageUrl::profile("/path.jpg", ProfileSize::W185);
```

**Available sizes:**

| Type | Variants |
|||
| `PosterSize` | `W92`, `W154`, `W185`, `W342`, `W500`, `W780`, `Original` |
| `BackdropSize` | `W300`, `W780`, `W1280`, `Original` |
| `ProfileSize` | `W45`, `W185`, `H632`, `Original` |
| `StillSize` | `W92`, `W185`, `W300`, `Original` |
| `LogoSize` | `W45`, `W92`, `W154`, `W185`, `W300`, `W500`, `Original` |



## Caching

### Default SQLite Cache

`.with_cache()` stores to the OS cache directory (e.g. `~/.cache/cameo/cache.db` on Linux), falling back to a temp file if that fails.

```rust
let client = CameoClient::builder()
    .with_tmdb(config)
    .with_cache()
    .build()?;
```

### Custom TTLs

```rust
use cameo::CacheTtlConfig;
use std::time::Duration;

let client = CameoClient::builder()
    .with_tmdb(config)
    .with_cache()
    .with_cache_ttl(CacheTtlConfig {
        details:   Duration::from_secs(86400),  // 24 h
        search:    Duration::from_secs(3600),   // 1 h
        discovery: Duration::from_secs(900),    // 15 min
        items:     Duration::from_secs(21600),  // 6 h
    })
    .build()?;
```

### Custom Cache Backend

Implement `CacheBackend` to plug in Redis, an in-memory store, or any other backend:

```rust
use cameo::CacheBackend;
use std::sync::Arc;

// impl CacheBackend for MyBackend { ... }

let client = CameoClient::builder()
    .with_tmdb(config)
    .with_cache_backend(Arc::new(MyBackend::new()))
    .build()?;
```

### Cache Lookup & Management

Results from any API call are automatically cached. You can also query the cache directly:

```rust
// Check cache without making an API call
if let Some(movie) = client.cached_movie("tmdb:550").await {
    println!("Cache hit: {}", movie.title);
}

// Full details from cache
let details = client.cached_movie_details("tmdb:550").await;
let show = client.cached_tv_show("tmdb:1396").await;
let show_details = client.cached_tv_show_details("tmdb:1396").await;
let person = client.cached_person("tmdb:138").await;
let person_details = client.cached_person_details("tmdb:138").await;

// Invalidate entries for a specific item
client.invalidate_cached("tmdb:550").await;

// Clear the entire cache
client.clear_cache().await;
```

> **Note:** Cache errors are non-fatal. They are logged internally but do not cause API calls to fail.

## Error Handling

All errors implement `std::error::Error` and work naturally with `?`.

```rust
use cameo::{CameoClientError, TmdbError};

match client.movie_details(999_999_999).await {
    Ok(details) => println!("{}", details.movie.title),
    Err(CameoClientError::Tmdb(TmdbError::Api(msg))) => {
        eprintln!("API error: {msg}");
    }
    Err(CameoClientError::Tmdb(TmdbError::RateLimitExceeded)) => {
        eprintln!("Rate limited — back off and retry");
    }
    Err(e) => eprintln!("Error: {e}"),
}
```

**Error type hierarchy:**

- `CameoClientError` — top-level facade errors
  - `NoProviders` — no providers configured
  - `Tmdb(TmdbError)` — from the TMDB provider
  - `Cache(CacheError)` — non-fatal cache errors
- `TmdbError` — TMDB-specific
  - `Http(reqwest::Error)` — network failure
  - `Api(String)` — non-2xx API response
  - `Deserialization(serde_json::Error)` — unexpected response shape
  - `RateLimitExceeded` — 429 from TMDB
  - `InvalidConfig(String)` — bad configuration
- `CacheError` — cache backend errors
  - `Serialization(serde_json::Error)`
  - `Backend(...)` — storage-level error

## Data Models Reference

| Type | Key fields |
||--|
| `UnifiedMovie` | `provider_id`, `title`, `overview`, `release_date`, `poster_url`, `backdrop_url`, `genres`, `vote_average`, `vote_count`, `popularity` |
| `UnifiedMovieDetails` | `movie: UnifiedMovie`, `tagline`, `runtime`, `budget`, `revenue`, `status`, `imdb_id`, `production_companies`, `belongs_to_collection` |
| `UnifiedTvShow` | `provider_id`, `name`, `overview`, `first_air_date`, `poster_url`, `backdrop_url`, `genres`, `vote_average`, `origin_country` |
| `UnifiedTvShowDetails` | `show: UnifiedTvShow`, `tagline`, `number_of_seasons`, `number_of_episodes`, `in_production`, `networks`, `created_by`, `last_air_date` |
| `UnifiedPerson` | `provider_id`, `name`, `known_for_department`, `profile_url`, `popularity`, `gender` |
| `UnifiedPersonDetails` | `person: UnifiedPerson`, `biography`, `birthday`, `deathday`, `place_of_birth`, `imdb_id`, `also_known_as` |
| `UnifiedSearchResult` | `Movie(UnifiedMovie)`, `TvShow(UnifiedTvShow)`, `Person(UnifiedPerson)` |

**`provider_id` format:** `"tmdb:{id}"` — e.g. `"tmdb:550"` for Fight Club. All image fields (`poster_url`, `backdrop_url`, `profile_url`) are fully resolved HTTPS URLs.

## Traits Reference

```rust
use cameo::unified::{SearchProvider, DetailProvider, DiscoveryProvider, MediaProvider};

// Accept any configured provider generically
async fn show_trending<P>(provider: &P) -> Result<(), P::Error>
where
    P: DiscoveryProvider,
{
    use cameo::core::config::TimeWindow;
    let movies = provider.trending_movies(TimeWindow::Week, None).await?;
    for m in &movies.results {
        println!("{}", m.title);
    }
    Ok(())
}
```

| Trait | Methods |
|-||
| `SearchProvider` | `search_movies`, `search_tv_shows`, `search_people`, `search_multi` |
| `DetailProvider` | `movie_details`, `tv_show_details`, `person_details` |
| `DiscoveryProvider` | `trending_movies`, `trending_tv_shows`, `popular_movies`, `top_rated_movies` |
| `MediaProvider` | Blanket — requires all three above |

## Architecture

```
CameoClient (unified facade)
├── SearchProvider / DetailProvider / DiscoveryProvider traits
├── Cache layer (optional — SqliteCache or custom CacheBackend)
└── TmdbClient
    └── Generated progenitor client (152 TMDB API operations)
```

| Module | Purpose |
|--||
| `cameo::unified` | `CameoClient` facade, traits, unified model types, `Genre` |
| `cameo::providers::tmdb` | `TmdbClient`, `TmdbConfig`, `ImageUrl`, discover builders |
| `cameo::cache` | `CacheBackend` trait, `SqliteCache`, `CacheTtlConfig` |
| `cameo::core` | `PaginatedResponse`, `into_stream`, `TimeWindow`, `CameoError` |
| `cameo::generated` | Progenitor-generated low-level TMDB client (do not use directly) |

## Examples

```bash
export TMDB_API_TOKEN=your_token_here

cargo run --example search_movies -- "Inception"
cargo run --example movie_details -- 550
cargo run --example trending
cargo run --example discover
cargo run --example unified_search -- "Breaking Bad"
```

## Testing

```bash
# Unit tests + wiremock integration tests (no network required)
cargo test

# Live API tests against the real TMDB API
TMDB_API_TOKEN=your_token cargo test --features live-tests
```

## Contributing

1. Clone the repo and run `cargo test` to verify your environment
2. Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, etc.
3. All public API items must have `///` doc comments
4. No `unwrap()` in library code — use `?` and proper error types
5. Add tests for all public API surface (unit tests preferred; wiremock for network calls)

## MSRV

The minimum supported Rust version is 1.93.1 because it is primarily used by [Beam](https://github.com/justin13888/beam) which uses relatively new syntax. We will not bump this version unless something major comes up. In theory, this crate does not use much modern features though besides Rust edition 2024.

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
