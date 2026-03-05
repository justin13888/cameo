//! Get movie and TV show recommendations and similar content.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example recommendations

use cameo::{CameoClient, providers::tmdb::TmdbConfig, unified::RecommendationProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .build()?;

    // Fight Club (550) recommendations
    println!("Recommendations for Fight Club (id=550)");
    println!("{}", "─".repeat(60));
    let recs = client.movie_recommendations(550, None).await?;
    for (i, movie) in recs.results.iter().take(10).enumerate() {
        let year = movie
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let rating = movie
            .vote_average
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "N/A".into());
        println!("  {:2}. [{year}] {}  ★ {rating}", i + 1, movie.title);
    }

    // Breaking Bad (1396) TV recommendations
    println!("\nRecommendations for Breaking Bad (id=1396)");
    println!("{}", "─".repeat(60));
    let tv_recs = client.tv_recommendations(1396, None).await?;
    for (i, show) in tv_recs.results.iter().take(10).enumerate() {
        let year = show
            .first_air_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        let rating = show
            .vote_average
            .map(|r| format!("{:.1}", r))
            .unwrap_or_else(|| "N/A".into());
        println!("  {:2}. [{year}] {}  ★ {rating}", i + 1, show.name);
    }

    // Similar movies to Fight Club
    println!("\nSimilar movies to Fight Club (id=550)");
    println!("{}", "─".repeat(60));
    let similar = client.similar_movies(550, None).await?;
    for (i, movie) in similar.results.iter().take(5).enumerate() {
        println!("  {:2}. {}", i + 1, movie.title);
    }

    Ok(())
}
