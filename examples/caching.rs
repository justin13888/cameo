//! Demonstrates every facet of the caching layer: file-backed and in-memory
//! backends, TTL customisation, explicit cache lookups, selective invalidation,
//! and full cache clearing.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example caching [query]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cameo::{
    CacheTtlConfig, CameoClient, SqliteCache,
    providers::tmdb::TmdbConfig,
    unified::{DetailProvider, SearchProvider},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Inception");

    // ── 1. File-backed cache via .with_cache() ──────────────────────────────
    //
    // The simplest way to enable caching. A SQLite database is created under
    // the OS cache directory (e.g. ~/.cache/cameo/cache.db on Linux) and
    // survives across process restarts.

    println!("=== 1. File-backed cache (.with_cache()) ===\n");

    let file_client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token.clone()).with_language("en-US"))
        .with_cache()
        .build()?;

    let t1 = Instant::now();
    let movies = file_client.search_movies(query, None).await?;
    let elapsed1 = t1.elapsed();
    println!(
        "  Search \"{query}\": {} results ({:.0?})",
        movies.total_results, elapsed1
    );

    file_client.flush_cache_writes().await;

    let t2 = Instant::now();
    let _ = file_client.search_movies(query, None).await?;
    let elapsed2 = t2.elapsed();
    println!("  Repeat  (cache hit): {:.0?}", elapsed2);
    println!(
        "  Speedup: ~{:.0}x\n",
        elapsed1.as_secs_f64() / elapsed2.as_secs_f64().max(0.000_001)
    );

    // Clean up so repeated runs don't accumulate stale data.
    file_client.clear_cache().await;

    // ── 2. In-memory cache via .with_cache_backend() ────────────────────────
    //
    // For tests, CLI tools, or short-lived processes you may prefer an
    // in-memory SQLite backend that disappears when the process exits.

    println!("=== 2. In-memory cache (.with_cache_backend()) ===\n");

    let mem_backend = Arc::new(SqliteCache::in_memory()?);

    let mem_client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token.clone()).with_language("en-US"))
        .with_cache_backend(mem_backend)
        .build()?;

    let t3 = Instant::now();
    let movies = mem_client.search_movies(query, None).await?;
    let elapsed3 = t3.elapsed();
    println!(
        "  Search \"{query}\": {} results ({:.0?})",
        movies.total_results, elapsed3
    );

    mem_client.flush_cache_writes().await;

    let t4 = Instant::now();
    let _ = mem_client.search_movies(query, None).await?;
    let elapsed4 = t4.elapsed();
    println!("  Repeat  (cache hit): {:.0?}", elapsed4);
    println!(
        "  Speedup: ~{:.0}x\n",
        elapsed3.as_secs_f64() / elapsed4.as_secs_f64().max(0.000_001)
    );

    // ── 3. Custom TTLs via .with_cache_ttl() ────────────────────────────────
    //
    // Override the default time-to-live for each cache category. Shorter TTLs
    // keep data fresher; longer TTLs reduce API traffic.

    println!("=== 3. Custom TTLs (.with_cache_ttl()) ===\n");

    let custom_ttl = CacheTtlConfig {
        search: Duration::from_secs(300),    // 5 min  (default: 1 hour)
        discovery: Duration::from_secs(120), // 2 min  (default: 15 min)
        details: Duration::from_secs(7200),  // 2 hour (default: 24 hours)
        items: Duration::from_secs(1800),    // 30 min (default: 6 hours)
        ..CacheTtlConfig::default()
    };

    println!("  search    = {}s", custom_ttl.search.as_secs());
    println!("  discovery = {}s", custom_ttl.discovery.as_secs());
    println!("  details   = {}s", custom_ttl.details.as_secs());
    println!("  items     = {}s", custom_ttl.items.as_secs());

    let ttl_client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .with_cache_backend(Arc::new(SqliteCache::in_memory()?))
        .with_cache_ttl(custom_ttl)
        .build()?;

    println!("  Client built with custom TTLs.\n");

    // ── 4. Explicit cache lookups ───────────────────────────────────────────
    //
    // After a search or details call, individual items are indexed in the
    // cache. You can read them back without any network request.

    println!("=== 4. Explicit cache lookups ===\n");

    // Populate cache via a search.
    let movies = ttl_client.search_movies(query, None).await?;
    ttl_client.flush_cache_writes().await;

    if let Some(first) = movies.results.first() {
        let pid = &first.provider_id;

        // cached_movie — returns the lightweight UnifiedMovie stored during search.
        match ttl_client.cached_movie(pid).await {
            Some(m) => println!("  cached_movie(\"{pid}\"): \"{}\"", m.title),
            None => println!("  cached_movie(\"{pid}\"): miss (unexpected)"),
        }

        // Fetch full details to populate the detail cache.
        let id: i32 = pid
            .strip_prefix("tmdb:")
            .and_then(|s| s.parse().ok())
            .expect("provider_id should be tmdb:<id>");
        let details = ttl_client.movie_details(id).await?;
        ttl_client.flush_cache_writes().await;

        // cached_movie_details — returns the full UnifiedMovieDetails.
        match ttl_client.cached_movie_details(pid).await {
            Some(d) => println!(
                "  cached_movie_details(\"{pid}\"): \"{}\" (runtime: {:?} min)",
                d.movie.title, d.runtime
            ),
            None => println!("  cached_movie_details(\"{pid}\"): miss (unexpected)"),
        }

        // Show that the data matches.
        println!(
            "  Live vs cached title match: {}",
            details.movie.title
                == ttl_client
                    .cached_movie_details(pid)
                    .await
                    .map(|d| d.movie.title)
                    .unwrap_or_default()
        );
    }

    // ── 5. Invalidation and clearing ────────────────────────────────────────
    //
    // invalidate_cached(provider_id) removes all cache entries (item + detail)
    // for a single entity. clear_cache() wipes everything.

    println!("\n=== 5. Invalidation and clearing ===\n");

    if let Some(first) = movies.results.first() {
        let pid = &first.provider_id;

        // Before invalidation.
        let before = ttl_client.cached_movie(pid).await.is_some();
        println!("  cached_movie(\"{pid}\") before invalidate: {before}");

        // Selective invalidation.
        ttl_client.invalidate_cached(pid).await;
        let after = ttl_client.cached_movie(pid).await.is_some();
        println!("  cached_movie(\"{pid}\") after  invalidate: {after}");

        // Re-populate for the clear_cache demo.
        let _ = ttl_client.search_movies(query, None).await?;
        ttl_client.flush_cache_writes().await;

        let repopulated = ttl_client.cached_movie(pid).await.is_some();
        println!("\n  Re-populated cache: cached_movie(\"{pid}\"): {repopulated}");

        // Full cache wipe.
        ttl_client.clear_cache().await;
        let cleared = ttl_client.cached_movie(pid).await.is_some();
        println!("  After clear_cache(): cached_movie(\"{pid}\"): {cleared}");
    }

    println!("\nDone!");
    Ok(())
}
