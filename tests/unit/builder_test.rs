/// Tests for discover builder construction and parameter passing.
/// These just verify that the builders can be created and chained without panicking.
/// (Network calls are not made in unit tests.)
use cameo::generated::tmdb::types::{DiscoverMovieSortBy, DiscoverTvSortBy};
use cameo::{
    providers::tmdb::{TmdbClient, TmdbConfig},
    unified::{CameoClient, CameoClientError},
};
use chrono::NaiveDate;

fn make_client() -> TmdbClient {
    TmdbClient::new(TmdbConfig::new("fake-token-for-testing")).unwrap()
}

#[test]
fn discover_movies_builder_chains() {
    let client = make_client();
    // Just verify that builder creation and method chaining compiles and works
    let _builder = client
        .discover_movies()
        .sort_by(DiscoverMovieSortBy::PopularityDesc)
        .with_genres("28,12")
        .without_genres("99")
        .primary_release_year(2024)
        .vote_average_gte(7.0)
        .vote_count_gte(100.0)
        .with_original_language("en")
        .with_runtime_gte(90)
        .with_runtime_lte(180)
        .page(1)
        .include_adult(false);
}

#[test]
fn discover_movies_builder_with_dates() {
    let client = make_client();
    let _builder = client
        .discover_movies()
        .primary_release_date_gte(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        .primary_release_date_lte(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
}

#[test]
fn discover_tv_builder_chains() {
    let client = make_client();
    let _builder = client
        .discover_tv()
        .sort_by(DiscoverTvSortBy::VoteAverageDesc)
        .with_genres("10765")
        .without_genres("99")
        .first_air_date_year(2024)
        .vote_average_gte(8.0)
        .vote_count_gte(500.0)
        .with_original_language("en")
        .with_runtime_gte(20)
        .with_runtime_lte(60)
        .page(1);
}

#[test]
fn discover_tv_builder_with_dates() {
    let client = make_client();
    let _builder = client
        .discover_tv()
        .first_air_date_gte(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        .first_air_date_lte(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
}

#[test]
fn tmdb_config_builder() {
    let config = TmdbConfig::new("my-token")
        .with_language("en-US")
        .with_region("US")
        .with_include_adult(false)
        .with_rate_limit(30);

    assert_eq!(config.api_token, "my-token");
    assert_eq!(config.language.as_deref(), Some("en-US"));
    assert_eq!(config.region.as_deref(), Some("US"));
    assert_eq!(config.include_adult, Some(false));
    assert_eq!(config.rate_limit, 30);
}

#[test]
fn tmdb_client_empty_token_fails() {
    let result = TmdbClient::new(TmdbConfig::new(""));
    assert!(result.is_err());
}

// ── CameoClientBuilder error cases ─────────────────────────────────────────

#[test]
fn cameo_builder_no_providers_fails() {
    let result = CameoClient::builder().build();
    assert!(
        matches!(result, Err(CameoClientError::NoProviders)),
        "expected NoProviders error"
    );
}

#[test]
fn cameo_builder_empty_tmdb_token_fails() {
    let result = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(""))
        .build();
    assert!(
        result.is_err(),
        "expected error for empty TMDB token, got Ok"
    );
}

#[test]
fn tmdb_config_rate_limit_timeout_roundtrips() {
    use std::time::Duration;

    let config = TmdbConfig::new("tok")
        .with_rate_limit(10)
        .with_rate_limit_timeout(Duration::from_secs(5));

    assert_eq!(config.rate_limit, 10);
    assert_eq!(config.rate_limit_timeout, Some(Duration::from_secs(5)));
}
