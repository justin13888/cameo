use cameo::providers::tmdb::{TmdbClient, TmdbConfig};

#[cfg(feature = "live-tests")]
fn client() -> TmdbClient {
    let token = std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN must be set for live tests");
    TmdbClient::new(TmdbConfig::new(token).with_language("en-US")).unwrap()
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_tv_series_details_breaking_bad() {
    let c = client();
    let details = c.tv_series_details(1396).await.unwrap(); // Breaking Bad

    assert_eq!(details.id, 1396);
    assert_eq!(details.name.as_deref(), Some("Breaking Bad"));
    assert!(details.number_of_seasons > 0);
    assert!(!details.genres.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_trending_tv_returns_page() {
    use cameo::core::config::TimeWindow;
    let c = client();
    let resp = c.trending_tv(TimeWindow::Week, None).await.unwrap();
    assert!(!resp.results.is_empty());
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_discover_tv_with_filters() {
    use cameo::generated::tmdb::types::DiscoverTvSortBy;

    let c = client();
    let resp = c
        .discover_tv()
        .sort_by(DiscoverTvSortBy::PopularityDesc)
        .with_original_language("en")
        .execute()
        .await
        .unwrap();

    assert!(!resp.results.is_empty());
}
