//! Fetch season and episode details for a TV show.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example season_details

use cameo::{CameoClient, providers::tmdb::TmdbConfig, unified::SeasonProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .build()?;

    // Game of Thrones (1399) Season 1
    println!("Season 1 of Game of Thrones (id=1399)");
    println!("{}", "─".repeat(60));
    let season = client.season_details(1399, 1).await?;
    println!("Name: {}", season.name.as_deref().unwrap_or("Unknown"));
    println!("Episodes: {}", season.episodes.len());
    println!();

    for ep in season.episodes.iter().take(5) {
        let runtime = ep
            .runtime
            .map(|r| format!("{r}m"))
            .unwrap_or_else(|| "?".into());
        let rating = ep
            .vote_average
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "N/A".into());
        println!(
            "  E{:02} - {}  ({})  ★ {rating}",
            ep.episode_number,
            ep.name.as_deref().unwrap_or("Unknown"),
            runtime
        );
    }

    // Fetch a single episode
    println!("\nEpisode 1x01 Details:");
    println!("{}", "─".repeat(60));
    let ep = client.episode_details(1399, 1, 1).await?;
    println!("Name: {}", ep.name.as_deref().unwrap_or("Unknown"));
    if let Some(overview) = &ep.overview {
        let short: String = overview.chars().take(120).collect();
        println!("Overview: {short}...");
    }

    Ok(())
}
