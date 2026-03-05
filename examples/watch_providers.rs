//! Look up streaming availability for movies and TV shows.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example watch_providers

use cameo::{CameoClient, providers::tmdb::TmdbConfig, unified::WatchProviderTrait};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .build()?;

    // Fight Club (550) watch providers
    println!("Watch providers for Fight Club (id=550)");
    println!("{}", "─".repeat(60));
    let providers = client.movie_watch_providers(550).await?;
    if let Some(us) = providers.results.get("US") {
        if !us.flatrate.is_empty() {
            println!("  Streaming:");
            for svc in &us.flatrate {
                println!("    - {}", svc.name);
            }
        }
        if !us.rent.is_empty() {
            println!("  Rent:");
            for svc in &us.rent {
                println!("    - {}", svc.name);
            }
        }
        if !us.buy.is_empty() {
            println!("  Buy:");
            for svc in &us.buy {
                println!("    - {}", svc.name);
            }
        }
    } else {
        println!("  No US providers found.");
    }

    // Breaking Bad (1396) TV watch providers
    println!("\nWatch providers for Breaking Bad (id=1396)");
    println!("{}", "─".repeat(60));
    let tv_providers = client.tv_watch_providers(1396).await?;
    if let Some(us) = tv_providers.results.get("US") {
        if !us.flatrate.is_empty() {
            println!("  Streaming:");
            for svc in &us.flatrate {
                println!("    - {}", svc.name);
            }
        }
    } else {
        println!("  No US providers found.");
    }

    Ok(())
}
