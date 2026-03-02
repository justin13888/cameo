//! Fetch trending movies and TV shows by day and week.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example trending

use cameo::{
    core::config::TimeWindow,
    providers::tmdb::{TmdbClient, TmdbConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let client = TmdbClient::new(TmdbConfig::new(token).with_language("en-US"))?;

    println!("Trending Movies — This Week");
    println!("{}", "─".repeat(60));

    let movies = client.trending_movies(TimeWindow::Week, None).await?;
    for (i, movie) in movies.results.iter().take(10).enumerate() {
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
        println!("  {:2}. [{year}] {title}  ★ {rating}", i + 1);
    }

    println!("\nTrending TV Shows — This Week");
    println!("{}", "─".repeat(60));

    let shows = client.trending_tv(TimeWindow::Week, None).await?;
    for (i, show) in shows.results.iter().take(10).enumerate() {
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
        println!("  {:2}. [{year}] {name}  ↑ {pop}", i + 1);
    }

    println!("\nTrending Movies — Today");
    println!("{}", "─".repeat(60));

    let today = client.trending_movies(TimeWindow::Day, None).await?;
    for (i, movie) in today.results.iter().take(5).enumerate() {
        let title = movie.title.as_deref().unwrap_or("Unknown");
        println!("  {:2}. {title}", i + 1);
    }

    Ok(())
}
