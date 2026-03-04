//! Demonstrate the AniList provider for anime search and discovery.
//!
//! AniList requires no API token — it is freely accessible for public data.
//!
//! Usage:
//!   cargo run --example anilist_search --features anilist -- "query"
//!   cargo run --example anilist_search --no-default-features --features anilist -- "Your Name"

#[cfg(feature = "anilist")]
use cameo::{
    AniListClient, AniListConfig,
    core::config::TimeWindow,
    unified::{CameoClient, SearchProvider},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "anilist"))]
    {
        eprintln!("This example requires the `anilist` feature.");
        eprintln!("Run with: cargo run --example anilist_search --features anilist");
        std::process::exit(1);
    }

    #[cfg(feature = "anilist")]
    run().await
}

#[cfg(feature = "anilist")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Spirited Away");

    println!("AniList search demo");
    println!("{}", "═".repeat(60));

    // ── Low-level AniListClient ───────────────────────────────────────────────
    println!("\n[Low-level AniListClient]");
    let client = AniListClient::new(AniListConfig::new());

    // Search anime movies
    println!("\nAnime movies matching \"{query}\":");
    println!("{}", "─".repeat(60));
    let movies = client.search_movies(query, None).await?;
    println!(
        "Total: {} (showing first {})",
        movies.total_results,
        movies.results.len()
    );
    for m in movies.results.iter().take(5) {
        let year = m
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let score = m
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  🎬 [{year}] {}  ★ {score}  ({})", m.title, m.provider_id);
        if !m.genres.is_empty() {
            println!(
                "       {}",
                m.genres
                    .iter()
                    .map(|g| g.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    // Search anime series
    println!("\nAnime series matching \"{query}\":");
    println!("{}", "─".repeat(60));
    let series = client.search_tv_shows(query, None).await?;
    println!(
        "Total: {} (showing first {})",
        series.total_results,
        series.results.len()
    );
    for t in series.results.iter().take(5) {
        let year = t
            .first_air_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let score = t
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  📺 [{year}] {}  ★ {score}  ({})", t.name, t.provider_id);
        if !t.genres.is_empty() {
            println!(
                "       {}",
                t.genres
                    .iter()
                    .map(|g| g.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    // Search staff
    println!("\nStaff matching \"{query}\":");
    println!("{}", "─".repeat(60));
    let people = client.search_people(query, None).await?;
    println!(
        "Total: {} (showing first {})",
        people.total_results,
        people.results.len()
    );
    for p in people.results.iter().take(5) {
        let dept = p.known_for_department.as_deref().unwrap_or("Unknown");
        println!("  👤 {} ({dept})  ({})", p.name, p.provider_id);
    }

    // Trending anime movies
    println!("\nTrending anime movies:");
    println!("{}", "─".repeat(60));
    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    for m in trending.results.iter().take(5) {
        let score = m
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  🔥 {}  ★ {score}", m.title);
    }

    // Trending anime series
    println!("\nTrending anime series:");
    println!("{}", "─".repeat(60));
    let trending_tv = client.trending_tv(TimeWindow::Week, None).await?;
    for t in trending_tv.results.iter().take(5) {
        let score = t
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  🔥 {}  ★ {score}", t.name);
    }

    // ── CameoClient facade (AniList-only) ─────────────────────────────────────
    println!("\n[CameoClient facade (AniList-only)]");
    println!("{}", "─".repeat(60));
    let cameo = CameoClient::builder()
        .with_anilist(AniListConfig::new())
        .build()?;

    let results = cameo.search_movies(query, None).await?;
    println!(
        "Facade search for \"{query}\": {} total results",
        results.total_results
    );
    for m in results.results.iter().take(3) {
        println!("  → {} ({})", m.title, m.provider_id);
    }

    Ok(())
}
