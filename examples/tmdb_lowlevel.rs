//! Low-level TmdbClient usage.
//!
//! The low-level [`TmdbClient`] returns the same unified models as the facade,
//! so you get a stable, provider-neutral surface without going through
//! `CameoClient`. Shows construction, search, details + credits, the discover
//! builder API, trending, and genre lists.
//!
//! Usage: cargo run --example tmdb_lowlevel [query]
//!   (reads TMDB_API_TOKEN from .env or the environment)

use cameo::providers::tmdb::{TmdbClient, TmdbConfig, builders::MovieSortBy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Inception");
    let page: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    let client = TmdbClient::new(TmdbConfig::new(token).with_language("en-US"))?;

    // ── Paginated search ──────────────────────────────────────────────────────

    println!("=== search_movies: \"{query}\" (page {page}) ===");
    let resp = client.search_movies(query, Some(page)).await?;
    println!(
        "  {} results (page {}/{}, {} total)",
        resp.results.len(),
        resp.page,
        resp.total_pages,
        resp.total_results
    );
    for movie in resp.results.iter().take(5) {
        let year = movie
            .release_date
            .map(|d| d.year().to_string())
            .unwrap_or_else(|| "????".to_string());
        let rating = movie
            .vote_average
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "?".to_string());
        println!(
            "  [{year}] {}  ★ {rating}  ({})",
            movie.title, movie.provider_id
        );
    }
    if resp.has_next {
        println!("  {} more pages available.", resp.total_pages - resp.page);
    }

    // ── Movie details + credits ───────────────────────────────────────────────

    println!("\n=== movie_details + movie_credits: Fight Club (550) ===");
    let details = client.movie_details(550).await?;
    println!("  Title:   {}", details.movie.title);
    if let Some(runtime) = details.runtime {
        println!("  Runtime: {runtime} min");
    }
    if let Some(budget) = details.budget {
        println!("  Budget:  ${budget}");
    }
    if let Some(poster) = &details.movie.poster_url {
        println!("  Poster:  {poster}");
    }

    let credits = client.movie_credits(550).await?;
    println!("  Cast ({}):", credits.cast.len());
    for actor in credits.cast.iter().take(3) {
        let character = actor.character.as_deref().unwrap_or("?");
        println!("    {} as {character}", actor.name);
    }
    if let Some(dir) = credits
        .crew
        .iter()
        .find(|c| c.job.as_deref() == Some("Director"))
    {
        println!("  Director: {}", dir.name);
    }

    // ── Discover builder ──────────────────────────────────────────────────────

    println!("\n=== discover_movies: drama + vote_count ≥ 1000 ===");
    let discover_resp = client
        .discover_movies()
        .sort_by(MovieSortBy::VoteAverageDesc)
        .with_genres(&[18]) // Drama
        .vote_count_gte(1000.0)
        .vote_average_gte(8.0)
        .with_original_language("en")
        .page(1)
        .execute()
        .await?;
    println!("  {} total matches", discover_resp.total_results);
    for movie in discover_resp.results.iter().take(5) {
        let rating = movie
            .vote_average
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "?".to_string());
        println!("  {}  ★ {rating}", movie.title);
    }

    // ── Trending ──────────────────────────────────────────────────────────────

    println!("\n=== trending_movies (week) ===");
    use cameo::core::config::TimeWindow;
    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    println!("  {} trending movies", trending.total_results);
    for m in trending.results.iter().take(5) {
        let pop = m
            .popularity
            .map(|p| format!("{p:.0}"))
            .unwrap_or_else(|| "?".to_string());
        println!("  {}  ↑ {pop}", m.title);
    }

    // ── Genre list ────────────────────────────────────────────────────────────

    println!("\n=== movie_genres ===");
    let genres = client.movie_genres().await?;
    for g in genres.iter().take(8) {
        println!("  {g}");
    }

    Ok(())
}
