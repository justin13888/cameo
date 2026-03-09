//! Showcase the CameoClient facade — the primary way to use cameo.
//!
//! Covers all trait methods: search, details, discovery, recommendations,
//! seasons, and watch providers. This example doubles as an end-to-end smoke
//! test for the unified interface.
//!
//! Usage: TMDB_API_TOKEN=xxx cargo run --example facade_showcase [query]

use cameo::{
    CameoClient,
    core::config::TimeWindow,
    providers::tmdb::TmdbConfig,
    unified::{
        DetailProvider, DiscoveryProvider, RecommendationProvider, SearchProvider, SeasonProvider,
        WatchProviderTrait, models::UnifiedSearchResult,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    let query = args.get(1).map(String::as_str).unwrap_or("Dune");

    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token).with_language("en-US"))
        .build()?;

    // ── Search ────────────────────────────────────────────────────────────────

    println!("=== search_movies: \"{query}\" ===");
    let movies = client.search_movies(query, None).await?;
    println!("  {} total results", movies.total_results);
    for m in movies.results.iter().take(3) {
        let year = m
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("????");
        println!("  [{year}] {}  ({})", m.title, m.provider_id);
    }

    println!("\n=== search_multi: \"{query}\" ===");
    let multi = client.search_multi(query, None).await?;
    println!("  {} total results", multi.total_results);
    for r in multi.results.iter().take(5) {
        match r {
            UnifiedSearchResult::Movie(m) => println!("  Movie: {}", m.title),
            UnifiedSearchResult::TvShow(t) => println!("  TV:    {}", t.name),
            UnifiedSearchResult::Person(p) => println!("  Person:{}", p.name),
        }
    }

    // ── Details ───────────────────────────────────────────────────────────────

    println!("\n=== movie_details: Fight Club (550) ===");
    let md = client.movie_details(550).await?;
    println!("  Title:    {}", md.movie.title);
    println!("  Runtime:  {:?} min", md.runtime);
    println!(
        "  Genres:   {}",
        md.movie
            .genres
            .iter()
            .map(|g| g.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!("\n=== tv_show_details: Breaking Bad (1396) ===");
    let tv = client.tv_show_details(1396).await?;
    println!("  Name:     {}", tv.show.name);
    println!("  Seasons:  {:?}", tv.number_of_seasons);
    println!("  Status:   {:?}", tv.status);

    println!("\n=== person_details: Brad Pitt (287) ===");
    let person = client.person_details(287).await?;
    println!("  Name:     {}", person.person.name);
    println!("  Dept:     {:?}", person.person.known_for_department);

    // ── Discovery ─────────────────────────────────────────────────────────────

    println!("\n=== trending_movies (week) ===");
    let trending = client.trending_movies(TimeWindow::Week, None).await?;
    for m in trending.results.iter().take(3) {
        let rating = m
            .vote_average
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {} ★ {rating}", m.title);
    }

    println!("\n=== popular_tv_shows ===");
    let pop_tv = client.popular_tv_shows(None).await?;
    for t in pop_tv.results.iter().take(3) {
        println!("  {}", t.name);
    }

    println!("\n=== top_rated_movies ===");
    let top = client.top_rated_movies(None).await?;
    for m in top.results.iter().take(3) {
        let rating = m
            .vote_average
            .map(|r| format!("{r:.1}"))
            .unwrap_or_else(|| "N/A".into());
        println!("  {} ★ {rating}", m.title);
    }

    // ── Recommendations ───────────────────────────────────────────────────────

    println!("\n=== movie_recommendations: Fight Club (550) ===");
    let recs = client.movie_recommendations(550, None).await?;
    println!("  {} recommendations", recs.total_results);
    for m in recs.results.iter().take(3) {
        println!("  {}", m.title);
    }

    println!("\n=== similar_movies: Fight Club (550) ===");
    let sim = client.similar_movies(550, None).await?;
    for m in sim.results.iter().take(3) {
        println!("  {}", m.title);
    }

    // ── Seasons ───────────────────────────────────────────────────────────────

    println!("\n=== season_details: Game of Thrones s1 (1399) ===");
    let season = client.season_details(1399, 1).await?;
    println!(
        "  {} — {} episodes",
        season.name.as_deref().unwrap_or("?"),
        season.episodes.len()
    );
    for ep in season.episodes.iter().take(3) {
        println!(
            "  E{:02} {}",
            ep.episode_number,
            ep.name.as_deref().unwrap_or("Unknown")
        );
    }

    println!("\n=== episode_details: GoT s1e1 ===");
    let ep = client.episode_details(1399, 1, 1).await?;
    println!("  {}", ep.name.as_deref().unwrap_or("Unknown"));

    // ── Watch Providers ───────────────────────────────────────────────────────

    println!("\n=== movie_watch_providers: Fight Club (550) ===");
    let wp = client.movie_watch_providers(550).await?;
    if let Some(us) = wp.results.get("US") {
        for svc in us.flatrate.iter().take(3) {
            println!("  Stream: {}", svc.name);
        }
        for svc in us.rent.iter().take(3) {
            println!("  Rent:   {}", svc.name);
        }
    } else {
        println!("  No US providers.");
    }

    Ok(())
}
