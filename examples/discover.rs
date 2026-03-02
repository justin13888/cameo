//! Discover movies with filters using the builder API.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example discover

use cameo::{
    generated::tmdb::types::DiscoverMovieSortBy,
    providers::tmdb::{TmdbClient, TmdbConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let client = TmdbClient::new(TmdbConfig::new(token).with_language("en-US"))?;

    println!("Discovering: top-rated action movies (vote_count ≥ 1000, released 2010–2024)");
    println!("{}", "─".repeat(60));

    let resp = client
        .discover_movies()
        .sort_by(DiscoverMovieSortBy::VoteAverageDesc)
        .with_genres("28") // Action
        .vote_average_gte(8.0)
        .vote_count_gte(1000.0)
        .primary_release_year(2024)
        .with_original_language("en")
        .page(1)
        .execute()
        .await?;

    println!(
        "Found {} results (showing first {})\n",
        resp.total_results,
        resp.results.len()
    );

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
            .unwrap_or_else(|| "N/A".into());
        println!("  [{year}] {title}  ★ {rating}  (id: {})", movie.id);
    }

    println!("\n---\nDiscovering: popular sci-fi TV shows");
    println!("{}", "─".repeat(60));

    use cameo::generated::tmdb::types::DiscoverTvSortBy;

    let tv_resp = client
        .discover_tv()
        .sort_by(DiscoverTvSortBy::PopularityDesc)
        .with_genres("10765") // Sci-Fi & Fantasy
        .with_original_language("en")
        .page(1)
        .execute()
        .await?;

    println!(
        "Found {} results (showing first {})\n",
        tv_resp.total_results,
        tv_resp.results.len()
    );

    for show in &tv_resp.results {
        let name = show.name.as_deref().unwrap_or("Unknown");
        let year = show
            .first_air_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let pop = show
            .popularity
            .map(|p| format!("{:.1}", p))
            .unwrap_or_else(|| "N/A".into());
        println!("  [{year}] {name}  ↑ {pop}  (id: {})", show.id);
    }

    Ok(())
}
