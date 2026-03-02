use cameo::providers::tmdb::{TmdbClient, TmdbConfig};

#[cfg(feature = "live-tests")]
fn token() -> String {
    std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN must be set for live tests")
}

#[cfg(feature = "live-tests")]
fn client() -> TmdbClient {
    TmdbClient::new(
        TmdbConfig::new(token())
            .with_language("en-US")
            .with_include_adult(false),
    )
    .unwrap()
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_search_movies_returns_results() {
    let c = client();
    let resp = c.search_movies("Inception", None).await.unwrap();
    assert!(
        resp.total_results > 0,
        "Should find results for 'Inception'"
    );
    assert!(!resp.results.is_empty(), "First page should have results");

    let first = &resp.results[0];
    assert!(!first.title.as_deref().unwrap_or("").is_empty());
    assert!(first.id > 0);
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_search_tv_returns_results() {
    let c = client();
    let resp = c.search_tv_shows("Breaking Bad", None).await.unwrap();
    assert!(resp.total_results > 0);
    assert!(!resp.results.is_empty());

    let first = &resp.results[0];
    assert!(!first.name.as_deref().unwrap_or("").is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_search_people_returns_results() {
    let c = client();
    let resp = c.search_people("Tom Hanks", None).await.unwrap();
    assert!(resp.total_results > 0);

    let first = &resp.results[0];
    assert!(!first.name.as_deref().unwrap_or("").is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_search_multi_returns_mixed_results() {
    let c = client();
    let resp = c.search_multi("Star Wars", None).await.unwrap();
    assert!(resp.total_results > 0);
    assert!(!resp.results.is_empty());
}
