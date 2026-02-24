//! Search for movies by title and display paginated results.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example search_movies -- "query" [page]

use cameo::providers::tmdb::{TmdbClient, TmdbConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("TMDB_API_TOKEN")
        .expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Inception");
    let page: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    let client = TmdbClient::new(TmdbConfig::new(token).with_language("en-US"))?;

    println!("Searching for: \"{query}\" (page {page})");
    println!("{}", "─".repeat(60));

    let resp = client.search_movies(query, Some(page)).await?;

    println!(
        "Found {} results (page {}/{}, {} total)",
        resp.results.len(),
        resp.page,
        resp.total_pages,
        resp.total_results
    );
    println!();

    for movie in &resp.results {
        let title = movie.title.as_deref().unwrap_or("Unknown");
        let year = movie
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let rating = movie
            .vote_average
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "N/A".to_string());
        let popularity = movie
            .popularity
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "N/A".to_string());

        println!("  [{year}] {title}  ★ {rating}  ↑ {popularity}  (id: {})", movie.id);
        if let Some(overview) = &movie.overview {
            let truncated: String = overview.chars().take(100).collect();
            let suffix = if overview.len() > 100 { "…" } else { "" };
            println!("         {truncated}{suffix}");
        }
    }

    if resp.has_next_page() {
        println!(
            "\n{} more pages available. Run with page {}.",
            resp.total_pages - resp.page,
            resp.page + 1
        );
    }

    Ok(())
}
