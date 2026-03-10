//! Error handling patterns for the cameo SDK.
//!
//! Demonstrates how to match on specific error variants returned by
//! `CameoClient` and the underlying provider clients, covering:
//!
//! - Build-time errors (no providers configured, invalid config)
//! - API errors (auth failures, 404 not found)
//! - Graceful recovery patterns
//! - `?` propagation vs explicit `match`
//!
//! Some demos require network access; see each function's comment.
//!
//! Usage:
//!   cargo run --example error_handling                     # build-time only
//!   TMDB_API_TOKEN=xxx cargo run --example error_handling  # all demos

use cameo::{
    CameoClient, TmdbConfig, TmdbError,
    unified::{CameoClientError, DetailProvider, SearchProvider},
};

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    demo_build_time_errors();
    demo_api_errors().await;
    demo_not_found().await;
    demo_propagation_styles().await;

    #[cfg(feature = "anilist")]
    demo_anilist_errors().await;
}

// ── 1. Build-time errors (no network) ────────────────────────────────────────

/// Shows errors that occur before any network call is made.
fn demo_build_time_errors() {
    println!("=== Build-time errors ===");

    // Building with no providers configured → NoProviders
    match CameoClient::builder().build() {
        Ok(_) => println!("  [unexpected] client built with no providers"),
        Err(CameoClientError::NoProviders) => {
            println!("  [ok] NoProviders: must configure at least one provider");
        }
        Err(e) => println!("  [unexpected] {e}"),
    }

    // An empty token is rejected immediately by TmdbConfig::new
    match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(""))
        .build()
    {
        Ok(_) => println!("  [unexpected] empty token accepted"),
        Err(CameoClientError::Tmdb(TmdbError::InvalidConfig(msg))) => {
            println!("  [ok] InvalidConfig: {msg}");
        }
        Err(e) => println!("  [unexpected] {e}"),
    }

    println!();
}

// ── 2. API errors — authentication failure (network) ─────────────────────────

/// Demonstrates how to match on `TmdbError::Api` status codes.
///
/// Uses a deliberately invalid token so no real credentials are needed.
async fn demo_api_errors() {
    println!("=== API errors (invalid token → 401) ===");

    let client = match CameoClient::builder()
        .with_tmdb(TmdbConfig::new("invalid-token"))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("  build failed unexpectedly: {e}");
            println!();
            return;
        }
    };

    // The error only surfaces on the first API call, not at build time.
    match client.search_movies("Inception", None).await {
        Ok(results) => {
            println!("  [unexpected] got {} results", results.total_results);
        }
        Err(CameoClientError::Tmdb(TmdbError::Api { status, message })) => match status {
            401 => println!("  [ok] 401 Unauthorized — check your API token: {message}"),
            403 => println!("  [ok] 403 Forbidden — token lacks required permissions"),
            404 => println!("  [ok] 404 Not Found — endpoint may have moved"),
            429 => println!("  [ok] 429 Rate Limited — back off and retry"),
            500..=599 => println!("  [ok] {status} Server Error — TMDB is having issues"),
            _ => println!("  [ok] HTTP {status}: {message}"),
        },
        Err(CameoClientError::Tmdb(TmdbError::Http(e))) => {
            // No network connectivity, DNS failure, TLS error, etc.
            println!("  transport error (no network?): {e}");
        }
        Err(e) => println!("  unexpected error variant: {e}"),
    }

    println!();
}

// ── 3. Not-found recovery (network, needs valid TMDB_API_TOKEN) ──────────────

/// Shows how to gracefully handle a 404 and provide a fallback value.
async fn demo_not_found() {
    println!("=== Not-found recovery ===");

    let token = match std::env::var("TMDB_API_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            println!("  skipped — TMDB_API_TOKEN not set");
            println!();
            return;
        }
    };

    let client = match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("  build error: {e}");
            println!();
            return;
        }
    };

    // TMDB ID 999_999_999 does not exist → 404
    match client.movie_details(999_999_999).await {
        Ok(details) => println!("  [unexpected] found: {}", details.movie.title),
        Err(CameoClientError::Tmdb(TmdbError::Api { status: 404, .. })) => {
            println!("  [ok] movie not found — returning default placeholder");
            // In real code you might return None, a cached stale value, etc.
        }
        Err(e) => println!("  unexpected error: {e}"),
    }

    println!();
}

// ── 4. Propagation styles ─────────────────────────────────────────────────────

/// Shows `?` propagation vs explicit `match`, side by side.
async fn demo_propagation_styles() {
    println!("=== Propagation styles ===");

    // Style A: `?` — propagate and let the caller decide
    match search_with_question_mark("Inception").await {
        Ok(count) => println!("  [?-style]     found {count} results for 'Inception'"),
        Err(e) => println!("  [?-style]     skipped or error: {e}"),
    }

    // Style B: explicit match — handle and recover inline
    search_with_match("Inception").await;

    println!();
}

/// Uses `?` to propagate errors upward. The caller decides how to handle them.
async fn search_with_question_mark(query: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let token = std::env::var("TMDB_API_TOKEN")?;
    let client = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token))
        .build()?;
    let results = client.search_movies(query, None).await?;
    Ok(results.total_results)
}

/// Uses explicit `match` to handle errors and recover gracefully.
async fn search_with_match(query: &str) {
    let token = match std::env::var("TMDB_API_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            println!("  [match-style] skipped — TMDB_API_TOKEN not set");
            return;
        }
    };

    let client = match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token))
        .build()
    {
        Ok(c) => c,
        Err(CameoClientError::NoProviders) => {
            println!("  [match-style] no providers configured");
            return;
        }
        Err(e) => {
            println!("  [match-style] build error: {e}");
            return;
        }
    };

    match client.search_movies(query, None).await {
        Ok(results) => {
            println!(
                "  [match-style] found {} results for '{query}'",
                results.total_results
            );
        }
        Err(CameoClientError::Tmdb(TmdbError::Api { status: 401, .. })) => {
            println!("  [match-style] auth failed — check TMDB_API_TOKEN");
        }
        Err(e) => {
            println!("  [match-style] search failed: {e}");
        }
    }
}

// ── 5. AniList errors (feature = "anilist", network, no auth needed) ──────────

/// Demonstrates `AniListError` variants using a non-existent staff ID.
#[cfg(feature = "anilist")]
async fn demo_anilist_errors() {
    use cameo::{AniListConfig, AniListError};

    println!("=== AniList errors ===");

    let client = match CameoClient::builder()
        .with_anilist(AniListConfig::new())
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("  build error: {e}");
            println!();
            return;
        }
    };

    // AniList staff ID 999_999_999 does not exist → NotFound
    match client.person_details(999_999_999).await {
        Ok(details) => println!("  [unexpected] found: {}", details.person.name),
        Err(CameoClientError::AniList(AniListError::NotFound)) => {
            println!("  [ok] AniListError::NotFound — staff ID does not exist");
        }
        Err(CameoClientError::AniList(AniListError::GraphQL(errors))) => {
            // GraphQL errors carry structured messages from the API.
            for e in &errors {
                println!("  GraphQL error: {}", e.message);
            }
        }
        Err(CameoClientError::AniList(AniListError::Http(e))) => {
            println!("  transport error: {e}");
        }
        Err(e) => println!("  unexpected error: {e}"),
    }

    println!();
}
