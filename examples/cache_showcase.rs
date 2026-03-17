//! Showcase the caching layer — the transparent read-through cache that
//! eliminates redundant API calls and keeps frequently-accessed data
//! available instantly.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example cache_showcase [query]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cameo::{
    CacheTtlConfig, CameoClient, SqliteCache,
    core::config::TimeWindow,
    providers::tmdb::TmdbConfig,
    unified::{DetailProvider, DiscoveryProvider, SearchProvider},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Inception");

    // ── 0. File-backed cache via .with_cache() ──────────────────────────────
    //
    // The simplest way to enable caching. A SQLite database is created under
    // the OS cache directory (e.g. ~/.cache/cameo/cache.db on Linux) and
    // survives across process restarts.

    println!("=== 0. File-backed cache (.with_cache()) ===\n");

    let file_client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token.clone()).with_language("en-US"))
        .with_cache()
        .build()?;

    let t0a = Instant::now();
    let fb_movies = file_client.search_movies(query, None).await?;
    let elapsed0a = t0a.elapsed();
    println!(
        "  Search \"{query}\": {} results ({:.0?})",
        fb_movies.total_results, elapsed0a
    );

    file_client.flush_cache_writes().await;

    let t0b = Instant::now();
    let _ = file_client.search_movies(query, None).await?;
    let elapsed0b = t0b.elapsed();
    println!("  Repeat  (cache hit): {:.0?}", elapsed0b);
    println!(
        "  Speedup: ~{:.0}x\n",
        elapsed0a.as_secs_f64() / elapsed0b.as_secs_f64().max(0.000_001)
    );

    // Clean up so repeated runs don't accumulate stale data.
    file_client.clear_cache().await;

    // ── 1. Setup ──────────────────────────────────────────────────────────────

    println!("=== 1. Setup: CameoClient with in-memory cache ===\n");

    let mut custom_ttl = CacheTtlConfig::default();
    custom_ttl.search = Duration::from_secs(300); // 5 min (default: 1 hour)
    custom_ttl.discovery = Duration::from_secs(120); // 2 min (default: 15 min)
    println!("  Custom TTLs:");
    println!("    search:    {}s", custom_ttl.search.as_secs());
    println!("    discovery: {}s", custom_ttl.discovery.as_secs());
    println!("    details:   {}s (default)", custom_ttl.details.as_secs());
    println!("    items:     {}s (default)", custom_ttl.items.as_secs());

    let cache_backend = SqliteCache::in_memory()?;

    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .with_cache_backend(Arc::new(cache_backend))
        .with_cache_ttl(custom_ttl)
        .build()?;

    println!("  Client ready.\n");

    // ── 2. Search with auto-cache ─────────────────────────────────────────────

    println!("=== 2. Search with auto-cache ===\n");

    let t1 = Instant::now();
    let movies = client.search_movies(query, None).await?;
    let elapsed1 = t1.elapsed();
    println!(
        "  First search for \"{query}\": {} results ({:.0?})",
        movies.total_results, elapsed1
    );
    for m in movies.results.iter().take(3) {
        let year = m
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        println!("    [{year}] {}  ({})", m.title, m.provider_id);
    }

    // Flush background cache writes before reading back
    client.flush_cache_writes().await;

    let t2 = Instant::now();
    let movies2 = client.search_movies(query, None).await?;
    let elapsed2 = t2.elapsed();
    println!(
        "\n  Second search (cache hit): {} results ({:.0?})",
        movies2.total_results, elapsed2
    );
    println!(
        "  Speedup: ~{:.0}x faster",
        elapsed1.as_secs_f64() / elapsed2.as_secs_f64().max(0.000_001)
    );

    // ── 3. Item-level cache from search ───────────────────────────────────────

    println!("\n\n=== 3. Item-level cache from search ===\n");

    if let Some(first_movie) = movies.results.first() {
        let pid = &first_movie.provider_id;
        let cached = client.cached_movie(pid).await;
        match cached {
            Some(m) => println!("  cached_movie(\"{pid}\"): found \"{}\"", m.title),
            None => println!("  cached_movie(\"{pid}\"): not found (unexpected!)"),
        }
    } else {
        println!("  No search results to test item cache.");
    }

    // ── 4. Details with auto-cache ────────────────────────────────────────────

    println!("\n\n=== 4. Details with auto-cache ===\n");

    let detail_id = 550; // Fight Club
    let provider_id = format!("tmdb:{detail_id}");

    let t3 = Instant::now();
    let details = client.movie_details(detail_id).await?;
    let elapsed3 = t3.elapsed();
    println!(
        "  movie_details({detail_id}): \"{}\" ({:.0?})",
        details.movie.title, elapsed3
    );
    println!(
        "    Runtime: {:?} min, Status: {:?}",
        details.runtime, details.status
    );

    client.flush_cache_writes().await;

    let t4 = Instant::now();
    let details2 = client.movie_details(detail_id).await?;
    let elapsed4 = t4.elapsed();
    println!(
        "\n  Second call (cache hit): \"{}\" ({:.0?})",
        details2.movie.title, elapsed4
    );

    // Explicit cache read — no API call at all
    let cached_details = client.cached_movie_details(&provider_id).await;
    match cached_details {
        Some(d) => println!(
            "  cached_movie_details(\"{provider_id}\"): found \"{}\"",
            d.movie.title
        ),
        None => println!("  cached_movie_details(\"{provider_id}\"): not found (unexpected!)"),
    }

    // ── 5. Cache invalidation ─────────────────────────────────────────────────

    println!("\n\n=== 5. Cache invalidation ===\n");

    let before = client.cached_movie_details(&provider_id).await;
    println!(
        "  Before invalidation: cached_movie_details(\"{provider_id}\"): {}",
        if before.is_some() {
            "found"
        } else {
            "not found"
        }
    );

    client.invalidate_cached(&provider_id).await;
    println!("  Called invalidate_cached(\"{provider_id}\")");

    let after = client.cached_movie_details(&provider_id).await;
    println!(
        "  After invalidation:  cached_movie_details(\"{provider_id}\"): {}",
        if after.is_some() {
            "found"
        } else {
            "not found"
        }
    );

    // ── 6. Discovery with cache ───────────────────────────────────────────────

    println!("\n\n=== 6. Discovery (trending) with cache ===\n");

    let t5 = Instant::now();
    let trending = client.trending_movies(TimeWindow::Day, None).await?;
    let elapsed5 = t5.elapsed();
    println!(
        "  trending_movies(Day): {} results ({:.0?})",
        trending.total_results, elapsed5
    );
    for m in trending.results.iter().take(3) {
        let rating = m
            .vote_average
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("    {} ★ {rating}", m.title);
    }

    client.flush_cache_writes().await;

    let t6 = Instant::now();
    let trending2 = client.trending_movies(TimeWindow::Day, None).await?;
    let elapsed6 = t6.elapsed();
    println!(
        "\n  Second call (cache hit): {} results ({:.0?})",
        trending2.total_results, elapsed6
    );

    // ── 7. Cache clearing ─────────────────────────────────────────────────────

    println!("\n\n=== 7. Cache clearing ===\n");

    if let Some(first_trending) = trending.results.first() {
        let pid = &first_trending.provider_id;
        let before = client.cached_movie(pid).await;
        println!(
            "  Before clear_cache(): cached_movie(\"{pid}\"): {}",
            if before.is_some() {
                "found"
            } else {
                "not found"
            }
        );

        client.clear_cache().await;
        println!("  Called clear_cache()");

        let after = client.cached_movie(pid).await;
        println!(
            "  After clear_cache():  cached_movie(\"{pid}\"): {}",
            if after.is_some() {
                "found"
            } else {
                "not found"
            }
        );
    } else {
        println!("  No trending results to demonstrate cache clearing.");
    }

    println!("\nDone!");
    Ok(())
}
