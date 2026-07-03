//! Error handling patterns for the cameo SDK.
//!
//! The facade returns [`CameoClientError`], which distinguishes:
//! - `NotConfigured` — no provider configured
//! - `Unsupported` — a provider is configured but none supports the operation
//! - `Provider(ProviderError)` — a provider-level, provider-agnostic error
//!
//! Usage:
//!   cargo run --example error_handling           # build-time only
//!   TMDB_API_TOKEN=... cargo run --example error_handling   # all demos

use cameo::{CameoClient, ProviderError, TmdbConfig, unified::CameoClientError};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    demo_build_time_errors();
    demo_api_errors().await;
    demo_not_found().await;
    demo_propagation_styles().await;
}

// ── 1. Build-time errors (no network) ────────────────────────────────────────

/// Shows errors that occur before any network call is made.
fn demo_build_time_errors() {
    println!("=== Build-time errors ===");

    // Building with no providers configured → NotConfigured
    match CameoClient::builder().build() {
        Ok(_) => println!("  [unexpected] client built with no providers"),
        Err(CameoClientError::NotConfigured) => {
            println!("  [ok] NotConfigured: must configure at least one provider");
        }
        Err(e) => println!("  [unexpected] {e}"),
    }

    // An empty token is rejected at construction → InvalidInput.
    match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(""))
        .build()
    {
        Ok(_) => println!("  [unexpected] empty token accepted"),
        Err(CameoClientError::Provider(ProviderError::InvalidInput(msg))) => {
            println!("  [ok] InvalidInput: {msg}");
        }
        Err(e) => println!("  [unexpected] {e}"),
    }

    println!();
}

// ── 2. Provider API errors (network) ─────────────────────────────────────────

/// Demonstrates matching on provider-agnostic `ProviderError` variants.
async fn demo_api_errors() {
    println!("=== API errors (invalid token → auth failure) ===");

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
        Ok(results) => println!("  [unexpected] got {} results", results.total_results),
        Err(CameoClientError::Provider(ProviderError::Auth(msg))) => {
            println!("  [ok] authentication failed — check your token: {msg}");
        }
        Err(CameoClientError::Provider(ProviderError::RateLimited { retry_after })) => {
            println!("  [ok] rate limited — retry after {retry_after:?}");
        }
        Err(CameoClientError::Provider(ProviderError::Transport(msg))) => {
            println!("  transport error (no network?): {msg}");
        }
        Err(CameoClientError::Provider(ProviderError::Api { status, message })) => {
            println!("  [ok] HTTP {status}: {message}");
        }
        Err(e) => println!("  unexpected error variant: {e}"),
    }

    println!();
}

// ── 3. Not-found recovery (network, needs valid TMDB_API_TOKEN) ──────────────

/// Shows how to gracefully handle a not-found and provide a fallback.
async fn demo_not_found() {
    println!("=== Not-found recovery ===");

    let Ok(token) = std::env::var("TMDB_API_TOKEN") else {
        println!("  skipped — TMDB_API_TOKEN not set\n");
        return;
    };

    let client = match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("  build error: {e}\n");
            return;
        }
    };

    // TMDB ID 999_999_999 does not exist → NotFound
    match client.movie_details(999_999_999).await {
        Ok(details) => println!("  [unexpected] found: {}", details.movie.title),
        Err(CameoClientError::Provider(ProviderError::NotFound)) => {
            println!("  [ok] movie not found — returning default placeholder");
        }
        Err(e) => println!("  unexpected error: {e}"),
    }

    println!();
}

// ── 4. Propagation styles ─────────────────────────────────────────────────────

/// Shows `?` propagation vs explicit `match`, side by side.
async fn demo_propagation_styles() {
    println!("=== Propagation styles ===");

    match search_with_question_mark("Inception").await {
        Ok(count) => println!("  [?-style]     found {count} results for 'Inception'"),
        Err(e) => println!("  [?-style]     skipped or error: {e}"),
    }

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
    let Ok(token) = std::env::var("TMDB_API_TOKEN") else {
        println!("  [match-style] skipped — TMDB_API_TOKEN not set");
        return;
    };

    let client = match CameoClient::builder()
        .with_tmdb(TmdbConfig::new(token))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("  [match-style] build error: {e}");
            return;
        }
    };

    match client.search_movies(query, None).await {
        Ok(results) => println!(
            "  [match-style] found {} results for '{query}'",
            results.total_results
        ),
        Err(CameoClientError::Provider(ProviderError::Auth(_))) => {
            println!("  [match-style] auth failed — check TMDB_API_TOKEN");
        }
        Err(e) => println!("  [match-style] search failed: {e}"),
    }
}
