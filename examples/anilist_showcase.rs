//! Showcase the AniList provider for anime search, details, and discovery.
//!
//! AniList requires no API token — all data is freely accessible.
//!
//! Usage:
//!   cargo run --example anilist_showcase --features anilist -- "Your Name"
//!   cargo run --example anilist_showcase --no-default-features --features anilist

#[cfg(feature = "anilist")]
use cameo::{
    AniListClient, AniListConfig,
    core::config::TimeWindow,
    unified::{CameoClient, SearchProvider, models::UnifiedSearchResult},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    #[cfg(not(feature = "anilist"))]
    {
        eprintln!("This example requires the `anilist` feature.");
        eprintln!("Run with: cargo run --example anilist_showcase --features anilist");
        std::process::exit(1);
    }

    #[cfg(feature = "anilist")]
    run().await
}

#[cfg(feature = "anilist")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Spirited Away");

    let client = AniListClient::new(AniListConfig::new())?;

    // ── Search ────────────────────────────────────────────────────────────────

    println!("=== search_movies: \"{query}\" ===");
    let movies = client.search_movies(query, None).await?;
    println!("  {} total results", movies.total_results);
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
        println!("  [{year}] {}  ★ {score}  ({})", m.title, m.provider_id);
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

    println!("\n=== search_tv_shows: \"{query}\" ===");
    let series = client.search_tv_shows(query, None).await?;
    println!("  {} total results", series.total_results);
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
        println!("  [{year}] {}  ★ {score}  ({})", t.name, t.provider_id);
    }

    println!("\n=== search_people: \"{query}\" ===");
    let people = client.search_people(query, None).await?;
    println!("  {} total results", people.total_results);
    for p in people.results.iter().take(3) {
        let dept = p.known_for_department.as_deref().unwrap_or("Unknown");
        println!("  {} ({dept})  ({})", p.name, p.provider_id);
    }

    // ── Details ───────────────────────────────────────────────────────────────

    if let Some(movie) = movies.results.first() {
        // Extract numeric ID from "anilist:{id}"
        if let Some(id_str) = movie.provider_id.strip_prefix("anilist:")
            && let Ok(id) = id_str.parse::<i32>()
        {
            println!("\n=== movie_details: {} (id={id}) ===", movie.title);
            let md = client.movie_details(id).await?;
            println!("  Title:   {}", md.movie.title);
            println!(
                "  Genres:  {}",
                md.movie
                    .genres
                    .iter()
                    .map(|g| g.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if let Some(overview) = &md.movie.overview {
                let short: String = overview.chars().take(120).collect();
                println!("  Overview:{short}...");
            }
        }
    }

    // ── Discovery ─────────────────────────────────────────────────────────────

    println!("\n=== trending_movies (week) ===");
    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    println!("  {} trending anime movies", trending.total_results);
    for m in trending.results.iter().take(5) {
        let score = m
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {}  ★ {score}", m.title);
    }

    println!("\n=== trending_tv (week) ===");
    let trending_tv = client.trending_tv(TimeWindow::Week, None).await?;
    println!("  {} trending anime series", trending_tv.total_results);
    for t in trending_tv.results.iter().take(5) {
        let score = t
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {}  ★ {score}", t.name);
    }

    println!("\n=== popular_tv_shows ===");
    let pop_tv = client.popular_tv_shows(None).await?;
    for t in pop_tv.results.iter().take(5) {
        println!("  {}", t.name);
    }

    println!("\n=== top_rated_tv_shows ===");
    let top_tv = client.top_rated_tv_shows(None).await?;
    for t in top_tv.results.iter().take(5) {
        let score = t
            .vote_average
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {}  ★ {score}", t.name);
    }

    // ── CameoClient facade (AniList-only) ─────────────────────────────────────

    println!("\n=== CameoClient facade (AniList-only) ===");
    let cameo = CameoClient::builder()
        .with_anilist(AniListConfig::new())
        .build()?;

    let results = cameo.search_movies(query, None).await?;
    println!(
        "  Facade search for \"{query}\": {} total results",
        results.total_results
    );
    for m in results.results.iter().take(3) {
        println!("  → {} ({})", m.title, m.provider_id);
    }

    // Also demonstrate search_multi via facade
    let multi = cameo.search_multi(query, None).await?;
    println!("\n  search_multi: {} total", multi.total_results);
    for r in multi.results.iter().take(3) {
        match r {
            UnifiedSearchResult::Movie(m) => println!("    Movie: {}", m.title),
            UnifiedSearchResult::TvShow(t) => println!("    TV:    {}", t.name),
            UnifiedSearchResult::Person(p) => println!("    Person:{}", p.name),
            _ => {}
        }
    }

    Ok(())
}
