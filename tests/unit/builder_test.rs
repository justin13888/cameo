//! Discover builders: the exact query string each one puts on the wire, plus
//! TMDB config construction and client/facade construction errors.
use std::collections::BTreeMap;

use cameo::{
    Genre, ProviderError, UnknownGenre,
    providers::tmdb::{
        MonetizationType, MovieReleaseType, MovieSortBy, TmdbClient, TmdbConfig, TmdbError,
        TvShowType, TvSortBy, TvStatus,
    },
    unified::{CameoClient, CameoClientError},
};
use jiff::civil::Date;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// Serve `fixture` at `route` and build a client against the mock server,
/// letting `configure` adjust the config first.
async fn serve(
    route: &str,
    fixture: &'static str,
    configure: impl FnOnce(TmdbConfig) -> TmdbConfig,
) -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_raw(fixture, "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let config = configure(TmdbConfig::new_with_base_url("test-token", server.uri()));
    let client = TmdbClient::new(config).expect("valid config");
    (server, client)
}

/// The query string of the single request `server` received, as a sorted map.
async fn sent_query(server: &MockServer) -> BTreeMap<String, String> {
    let requests = server
        .received_requests()
        .await
        .expect("request recording is enabled");
    assert_eq!(requests.len(), 1, "expected exactly one request");
    requests[0]
        .url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

/// Build the expected query map from `(key, value)` pairs.
fn query(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

const DISCOVER_MOVIES: &str = include_str!("../fixtures/discover_movies_response.json");
const DISCOVER_TV: &str = include_str!("../fixtures/discover_tv_response.json");

fn date(y: i16, m: i8, d: i8) -> Date {
    Date::new(y, m, d).expect("valid date")
}

#[tokio::test]
async fn discover_movies_sends_every_filter_encoded_on_the_wire() {
    let (server, client) = serve("/3/discover/movie", DISCOVER_MOVIES, |c| c).await;

    let page = client
        .discover_movies()
        .sort_by(MovieSortBy::PopularityDesc)
        .with_genres(&[28, 12])
        .without_genres(&[99])
        .with_cast(&[287])
        .with_people(&[1, 2, 3])
        .with_release_type(MovieReleaseType::Theatrical)
        .with_watch_monetization_types(&[MonetizationType::Flatrate, MonetizationType::Rent])
        .primary_release_year(2024)
        .primary_release_date_gte(date(2024, 1, 1))
        .primary_release_date_lte(date(2024, 12, 31))
        .vote_average_gte(7.5)
        .vote_count_gte(100.0)
        .with_original_language("en")
        .with_runtime_gte(90)
        .with_runtime_lte(180)
        .include_adult(false)
        .page(9999)
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[
            ("include_adult", "false"),
            ("page", "500"),
            ("primary_release_date.gte", "2024-01-01"),
            ("primary_release_date.lte", "2024-12-31"),
            ("primary_release_year", "2024"),
            ("sort_by", "popularity.desc"),
            ("vote_average.gte", "7.5"),
            ("vote_count.gte", "100.0"),
            ("with_cast", "287"),
            ("with_genres", "28,12"),
            ("with_original_language", "en"),
            ("with_people", "1,2,3"),
            ("with_release_type", "3"),
            ("with_runtime.gte", "90"),
            ("with_runtime.lte", "180"),
            ("with_watch_monetization_types", "flatrate|rent"),
            ("without_genres", "99"),
        ])
    );
    assert_eq!(page.results[0].provider_id, "tmdb:550");
}

#[tokio::test]
async fn discover_movies_defaults_language_region_and_adult_from_config() {
    let (server, client) = serve("/3/discover/movie", DISCOVER_MOVIES, |c| {
        c.with_language("fr-FR")
            .with_region("FR")
            .with_include_adult(true)
    })
    .await;

    client
        .discover_movies()
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[
            ("include_adult", "true"),
            ("language", "fr-FR"),
            ("region", "FR"),
        ])
    );
}

#[tokio::test]
async fn discover_movies_with_no_filters_sends_no_query() {
    let (server, client) = serve("/3/discover/movie", DISCOVER_MOVIES, |c| c).await;

    client
        .discover_movies()
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(sent_query(&server).await, query(&[]));
}

#[tokio::test]
async fn discover_tv_sends_every_filter_encoded_on_the_wire() {
    let (server, client) = serve("/3/discover/tv", DISCOVER_TV, |c| c).await;

    client
        .discover_tv()
        .sort_by(TvSortBy::VoteAverageDesc)
        .with_genres(&[10765])
        .without_genres(&[99])
        .first_air_date_year(2024)
        .first_air_date_gte(date(2020, 1, 1))
        .first_air_date_lte(date(2024, 12, 31))
        .vote_average_gte(8.0)
        .vote_count_gte(500.0)
        .with_original_language("en")
        .with_runtime_gte(20)
        .with_runtime_lte(60)
        .with_status(TvStatus::ReturningSeries)
        .with_type(TvShowType::Scripted)
        .with_networks(213)
        .with_watch_monetization_types(&[MonetizationType::Free])
        .page(0)
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[
            ("first_air_date.gte", "2020-01-01"),
            ("first_air_date.lte", "2024-12-31"),
            ("first_air_date_year", "2024"),
            ("page", "1"),
            ("sort_by", "vote_average.desc"),
            ("vote_average.gte", "8.0"),
            ("vote_count.gte", "500.0"),
            ("with_genres", "10765"),
            ("with_networks", "213"),
            ("with_original_language", "en"),
            ("with_runtime.gte", "20"),
            ("with_runtime.lte", "60"),
            ("with_status", "0"),
            ("with_type", "4"),
            ("with_watch_monetization_types", "free"),
            ("without_genres", "99"),
        ])
    );
}

#[tokio::test]
async fn discover_movies_typed_genre_lists_send_tmdb_ids() {
    let (server, client) = serve("/3/discover/movie", DISCOVER_MOVIES, |c| c).await;

    client
        .discover_movies()
        // Mecha has no TMDB id and is skipped.
        .with_genre_list(&[Genre::Action, Genre::Mecha, Genre::ScienceFiction])
        .without_genre_list(&[Genre::Documentary, Genre::Other(UnknownGenre::TmdbId(4242))])
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[("with_genres", "28,878"), ("without_genres", "99,4242")])
    );
}

#[tokio::test]
async fn discover_tv_typed_genre_lists_send_tmdb_ids() {
    let (server, client) = serve("/3/discover/tv", DISCOVER_TV, |c| c).await;

    client
        .discover_tv()
        .with_genres(&[1])
        // The typed setter replaces the raw-id filter set before it.
        .with_genre_list(&[Genre::SciFiFantasy, Genre::Animation])
        .without_genre_list(&[Genre::Reality])
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[("with_genres", "10765,16"), ("without_genres", "10764")])
    );
}

#[tokio::test]
async fn discover_tv_defaults_language_and_adult_but_not_region_from_config() {
    let (server, client) = serve("/3/discover/tv", DISCOVER_TV, |c| {
        c.with_language("fr-FR")
            .with_region("FR")
            .with_include_adult(true)
    })
    .await;

    client
        .discover_tv()
        .execute()
        .await
        .expect("discover succeeds");

    assert_eq!(
        sent_query(&server).await,
        query(&[("include_adult", "true"), ("language", "fr-FR")])
    );
}

#[test]
fn tmdb_config_builder() {
    use std::time::Duration;

    use cameo::RateLimit;

    let rate = RateLimit::per(30, Duration::from_secs(10));
    let config = TmdbConfig::new("my-token")
        .with_language("en-US")
        .with_region("US")
        .with_include_adult(false)
        .with_rate_limit(rate);

    assert_eq!(config.auth.credential(), "my-token");
    assert_eq!(config.language.as_deref(), Some("en-US"));
    assert_eq!(config.region.as_deref(), Some("US"));
    assert_eq!(config.include_adult, Some(false));
    assert_eq!(config.rate_limit, rate);
}

#[test]
fn tmdb_client_empty_token_fails() {
    let result = TmdbClient::new(TmdbConfig::new(""));
    assert!(
        matches!(result, Err(TmdbError::InvalidConfig(_))),
        "expected InvalidConfig, got {result:?}"
    );
}

// ── CameoClientBuilder error cases ─────────────────────────────────────────

#[test]
fn cameo_builder_no_providers_fails() {
    let result = CameoClient::builder().build();
    assert!(
        matches!(result, Err(CameoClientError::NotConfigured)),
        "expected NotConfigured error"
    );
}

#[test]
fn cameo_builder_empty_tmdb_token_fails() {
    let result = CameoClient::builder()
        .with_tmdb(TmdbConfig::new(""))
        .build();
    assert!(
        matches!(
            result,
            Err(CameoClientError::Provider(ProviderError::InvalidInput(_)))
        ),
        "expected Provider(InvalidInput) for empty TMDB token, got {:?}",
        result.err()
    );
}

#[test]
fn tmdb_config_rate_limit_roundtrips() {
    use std::time::Duration;

    use cameo::RateLimit;

    let config = TmdbConfig::new("tok").with_rate_limit(RateLimit::Unlimited);
    assert_eq!(config.rate_limit, RateLimit::Unlimited);

    let limited = RateLimit::per(10, Duration::from_secs(1));
    let config = TmdbConfig::new("tok").with_rate_limit(limited);
    assert_eq!(config.rate_limit, limited);
}

#[test]
fn tmdb_client_zero_rate_limit_is_construction_error() {
    use std::time::Duration;

    use cameo::RateLimit;

    // A misconfigured limiter is rejected at construction, not a runtime deadlock.
    let result = TmdbClient::new(TmdbConfig::new("tok").with_rate_limit(RateLimit::Limited {
        max_requests: 0,
        per: Duration::from_secs(1),
    }));
    assert!(
        matches!(result, Err(TmdbError::InvalidConfig(_))),
        "expected zero rate limit to be rejected as InvalidConfig, got {result:?}"
    );
}
