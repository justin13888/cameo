//! Demonstrate the unified CameoClient interface for cross-provider search.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example unified_search -- "query"

use cameo::{
    core::config::TimeWindow,
    providers::tmdb::TmdbConfig,
    unified::{CameoClient, DiscoveryProvider, SearchProvider, models::UnifiedSearchResult},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Dune");

    // Build the multi-provider facade (currently only TMDB)
    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .build()?;

    // ── Multi-search ──
    println!("Multi-search for: \"{query}\"");
    println!("{}", "─".repeat(60));

    let results = client.search_multi(query, None).await?;
    println!(
        "Found {} total results (showing first {})\n",
        results.total_results,
        results.results.len()
    );

    for result in results.results.iter().take(10) {
        match result {
            UnifiedSearchResult::Movie(m) => {
                let year = m
                    .release_date
                    .as_deref()
                    .and_then(|d| d.get(..4))
                    .unwrap_or("????");
                let rating = m
                    .vote_average
                    .map(|r| format!("{:.1}", r))
                    .unwrap_or_else(|| "N/A".into());
                println!(
                    "  🎬 [{}] {}  ★ {}  ({})",
                    year, m.title, rating, m.provider_id
                );
            }
            UnifiedSearchResult::TvShow(t) => {
                let year = t
                    .first_air_date
                    .as_deref()
                    .and_then(|d| d.get(..4))
                    .unwrap_or("????");
                let rating = t
                    .vote_average
                    .map(|r| format!("{:.1}", r))
                    .unwrap_or_else(|| "N/A".into());
                println!(
                    "  📺 [{}] {}  ★ {}  ({})",
                    year, t.name, rating, t.provider_id
                );
            }
            UnifiedSearchResult::Person(p) => {
                let dept = p.known_for_department.as_deref().unwrap_or("Unknown");
                println!("  👤 {} ({})  ({})", p.name, dept, p.provider_id);
            }
        }
    }

    // ── Unified trending ──
    println!("\nTrending Movies (unified, via TMDB):");
    println!("{}", "─".repeat(60));

    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    for movie in trending.results.iter().take(5) {
        let rating = movie
            .vote_average
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "N/A".into());
        println!("  {} ★ {}", movie.title, rating);
    }

    Ok(())
}
