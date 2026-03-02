//! Fetch detailed information about a movie including credits.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example movie_details -- [movie_id]
//! Default movie_id: 550 (Fight Club)

use cameo::providers::tmdb::{
    TmdbClient, TmdbConfig,
    image_url::{BackdropSize, ImageUrl, PosterSize},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let movie_id: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(550);

    let client = TmdbClient::new(TmdbConfig::new(token).with_language("en-US"))?;

    println!("Fetching movie details for id: {movie_id}");
    println!("{}", "─".repeat(60));

    let movie = client.movie_details(movie_id).await?;
    let credits = client.movie_credits(movie_id).await?;

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

    println!("Title:    {title} ({year})");
    println!("Rating:   ★ {rating}/10 ({} votes)", movie.vote_count);
    println!("Runtime:  {} min", movie.runtime);

    if let Some(tagline) = &movie.tagline {
        if !tagline.is_empty() {
            println!("Tagline:  \"{tagline}\"");
        }
    }

    if !movie.genres.is_empty() {
        let genre_names: Vec<_> = movie
            .genres
            .iter()
            .filter_map(|g| g.name.as_deref())
            .collect();
        println!("Genres:   {}", genre_names.join(", "));
    }

    if movie.budget > 0 {
        println!("Budget:   ${}", format_money(movie.budget));
    }
    if movie.revenue > 0 {
        println!("Revenue:  ${}", format_money(movie.revenue));
    }

    if let Some(overview) = &movie.overview {
        println!("\nOverview:\n  {overview}");
    }

    if let Some(poster) = &movie.poster_path {
        println!("\nPoster:   {}", ImageUrl::poster(poster, PosterSize::W500));
    }
    if let Some(backdrop) = &movie.backdrop_path {
        println!(
            "Backdrop: {}",
            ImageUrl::backdrop(backdrop, BackdropSize::W1280)
        );
    }

    println!("\nTop Cast:");
    for actor in credits.cast.iter().take(10) {
        let name = actor.name.as_deref().unwrap_or("Unknown");
        let char = actor.character.as_deref().unwrap_or("—");
        println!("  {name} as {char}");
    }

    println!("\nTop Crew:");
    for member in credits
        .crew
        .iter()
        .filter(|c| {
            matches!(
                c.job.as_deref(),
                Some("Director") | Some("Producer") | Some("Screenplay")
            )
        })
        .take(5)
    {
        let name = member.name.as_deref().unwrap_or("Unknown");
        let job = member.job.as_deref().unwrap_or("—");
        println!("  {job}: {name}");
    }

    Ok(())
}

fn format_money(amount: i64) -> String {
    let s = amount.to_string();
    let chars: Vec<char> = s.chars().rev().collect();
    let with_commas: String = chars
        .chunks(3)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",");
    with_commas.chars().rev().collect()
}
