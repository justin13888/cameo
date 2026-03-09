//! Low-level TmdbClient usage — for when you need direct access to the raw TMDB API.
//!
//! Shows construction, search, details+credits, image URL building, the
//! discover builder API, trending, and genre lists.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example tmdb_lowlevel [query]

use cameo::{
    generated::tmdb::types::DiscoverMovieSortBy,
    providers::tmdb::{
        TmdbClient, TmdbConfig,
        image_url::{BackdropSize, ImageUrl, PosterSize},
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        let title = movie.title.as_deref().unwrap_or("Unknown");
        let year = movie
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let rating = movie
            .vote_average
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  [{year}] {title}  ★ {rating}  (id: {})", movie.id);
    }
    if resp.has_next_page() {
        println!("  {} more pages available.", resp.total_pages - resp.page);
    }

    // ── Movie details + credits ───────────────────────────────────────────────

    println!("\n=== movie_details + movie_credits: Fight Club (550) ===");
    let details = client.movie_details(550).await?;
    println!(
        "  Title:   {}",
        details.title.as_deref().unwrap_or("Unknown")
    );
    println!("  Runtime: {} min", details.runtime);
    println!("  Budget:  ${}", details.budget);

    // Image URL construction
    if let Some(poster_path) = &details.poster_path {
        let url = ImageUrl::poster(poster_path, PosterSize::W500);
        println!("  Poster:  {url}");
    }
    if let Some(backdrop_path) = &details.backdrop_path {
        let url = ImageUrl::backdrop(backdrop_path, BackdropSize::W780);
        println!("  Backdrop:{url}");
    }

    let credits = client.movie_credits(550).await?;
    println!("  Cast ({}):", credits.cast.len());
    for actor in credits.cast.iter().take(3) {
        let name = actor.name.as_deref().unwrap_or("Unknown");
        let character = actor.character.as_deref().unwrap_or("?");
        println!("    {name} as {character}");
    }
    let director = credits
        .crew
        .iter()
        .find(|c| c.job.as_deref() == Some("Director"));
    if let Some(dir) = director {
        println!("  Director: {}", dir.name.as_deref().unwrap_or("Unknown"));
    }

    // ── Discover builder ──────────────────────────────────────────────────────

    println!("\n=== discover_movies: drama + vote_count ≥ 1000 ===");
    let discover_resp = client
        .discover_movies()
        .sort_by(DiscoverMovieSortBy::VoteAverageDesc)
        .with_genres("18") // Drama
        .vote_count_gte(1000.0)
        .vote_average_gte(8.0)
        .with_original_language("en")
        .page(1)
        .execute()
        .await?;
    println!("  {} total matches", discover_resp.total_results);
    for movie in discover_resp.results.iter().take(5) {
        let title = movie.title.as_deref().unwrap_or("Unknown");
        let rating = movie
            .vote_average
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {title}  ★ {rating}");
    }

    // ── Trending ──────────────────────────────────────────────────────────────

    println!("\n=== trending_movies (week) ===");
    use cameo::core::config::TimeWindow;
    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    println!("  {} trending movies", trending.total_results);
    for m in trending.results.iter().take(5) {
        let title = m.title.as_deref().unwrap_or("Unknown");
        let pop = m
            .popularity
            .map(|p| format!("{p:.0}"))
            .unwrap_or_else(|| "?".into());
        println!("  {title}  ↑ {pop}");
    }

    // ── Genre list ────────────────────────────────────────────────────────────

    println!("\n=== movie_genres ===");
    let genres = client.movie_genres().await?;
    for g in genres.genres.iter().take(8) {
        let name = g.name.as_deref().unwrap_or("Unknown");
        println!("  {:5}  {name}", g.id);
    }

    Ok(())
}
