//! Characterization tests for `TmdbClient`: each test pins one observable
//! behaviour of the client (what it sends, how it retries and paces, how it
//! shapes responses) so a refactor that changes it fails here.
//!
//! These pin current behaviour, including behaviour other issues intend to
//! change (e.g. discover builders bypassing the retry seam); such an issue
//! updates the assertion here as part of its change.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use cameo::{
    RateLimit, RetryPolicy, TimeWindow, UnifiedSearchResult,
    providers::tmdb::{TmdbClient, TmdbConfig, TmdbError},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const SEARCH_MOVIE: &str = include_str!("../fixtures/search_movie_response.json");
const EMPTY_PAGE: &str = r#"{"page":1,"results":[],"total_pages":1,"total_results":0}"#;
const SERVER_ERROR: &str = r#"{"status_code":11,"status_message":"Internal error."}"#;

/// Two retries with negligible backoff, so retry tests stay fast.
const TWO_RETRIES: RetryPolicy =
    RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(2));

/// Mount a `GET route` mock answering `template`, expecting `expected` hits.
async fn mount(server: &MockServer, route: &str, template: ResponseTemplate, expected: u64) {
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(template)
        .expect(expected)
        .mount(server)
        .await;
}

fn json(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.to_owned(), "application/json")
}

/// A client against `server`, with `configure` applied to the config.
fn client(server: &MockServer, configure: impl FnOnce(TmdbConfig) -> TmdbConfig) -> TmdbClient {
    TmdbClient::new(configure(TmdbConfig::new_with_base_url(
        "test-token",
        server.uri(),
    )))
    .expect("valid config")
}

/// The query string of every request `server` received, in order, as maps.
async fn sent_queries(server: &MockServer) -> Vec<BTreeMap<String, String>> {
    server
        .received_requests()
        .await
        .expect("request recording is enabled")
        .iter()
        .map(|r| {
            r.url
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
        .collect()
}

fn query(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

// ── Query parameters ──────────────────────────────────────────────────────────

#[tokio::test]
async fn page_is_clamped_to_tmdb_range_and_omitted_when_unset() {
    let server = MockServer::start().await;
    mount(&server, "/3/search/movie", json(SEARCH_MOVIE), 4).await;
    let client = client(&server, |c| c);

    for page in [Some(0), Some(9999), Some(7), None] {
        client.search_movies("q", page).await.unwrap();
    }

    assert_eq!(
        sent_queries(&server).await,
        vec![
            query(&[("page", "1"), ("query", "q")]),
            query(&[("page", "500"), ("query", "q")]),
            query(&[("page", "7"), ("query", "q")]),
            query(&[("query", "q")]),
        ]
    );
}

#[tokio::test]
async fn config_language_region_and_adult_are_forwarded_per_endpoint() {
    let server = MockServer::start().await;
    for route in [
        "/3/search/movie",
        "/3/search/tv",
        "/3/search/person",
        "/3/search/multi",
        "/3/movie/popular",
        "/3/movie/top_rated",
        "/3/tv/popular",
        "/3/tv/top_rated",
    ] {
        mount(&server, route, json(EMPTY_PAGE), 1).await;
    }
    let client = client(&server, |c| {
        c.with_language("de-DE")
            .with_region("DE")
            .with_include_adult(true)
    });

    client.search_movies("q", Some(2)).await.unwrap();
    client.search_tv_shows("q", Some(2)).await.unwrap();
    client.search_people("q", Some(2)).await.unwrap();
    client.search_multi("q", Some(2)).await.unwrap();
    client.popular_movies(Some(2)).await.unwrap();
    client.top_rated_movies(Some(2)).await.unwrap();
    client.popular_tv_shows(Some(2)).await.unwrap();
    client.top_rated_tv_shows(Some(2)).await.unwrap();

    let search_with_region = query(&[
        ("include_adult", "true"),
        ("language", "de-DE"),
        ("page", "2"),
        ("query", "q"),
        ("region", "DE"),
    ]);
    let search = query(&[
        ("include_adult", "true"),
        ("language", "de-DE"),
        ("page", "2"),
        ("query", "q"),
    ]);
    let list_with_region = query(&[("language", "de-DE"), ("page", "2"), ("region", "DE")]);
    let list = query(&[("language", "de-DE"), ("page", "2")]);
    assert_eq!(
        sent_queries(&server).await,
        vec![
            search_with_region,
            search.clone(),
            search.clone(),
            search,
            list_with_region.clone(),
            list_with_region,
            list.clone(),
            list,
        ]
    );
}

#[tokio::test]
async fn movie_details_with_append_sends_append_to_response() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/3/movie/550",
        json(include_str!("../fixtures/movie_details_response.json")),
        1,
    )
    .await;
    let client = client(&server, |c| c.with_language("en-US"));

    let details = client
        .movie_details_with_append(550, "credits,images")
        .await
        .unwrap();

    assert_eq!(details.movie.provider_id, "tmdb:550");
    assert_eq!(
        sent_queries(&server).await,
        vec![query(&[
            ("append_to_response", "credits,images"),
            ("language", "en-US"),
        ])]
    );
}

// ── Trending ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn trending_day_hits_day_window_and_sends_no_page() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/3/trending/movie/day",
        json(include_str!("../fixtures/trending_movies_response.json")),
        1,
    )
    .await;
    mount(
        &server,
        "/3/trending/tv/day",
        json(include_str!("../fixtures/trending_tv_response.json")),
        1,
    )
    .await;
    let client = client(&server, |c| c);

    // The page argument is ignored: TMDB trending is not paginated.
    let movies = client
        .trending_movies(TimeWindow::Day, Some(3))
        .await
        .unwrap();
    let shows = client.trending_tv(TimeWindow::Day, Some(3)).await.unwrap();

    assert_eq!(movies.page, 1);
    assert_eq!(shows.page, 1);
    assert_eq!(sent_queries(&server).await, vec![query(&[]), query(&[])]);
}

// ── Response shaping ──────────────────────────────────────────────────────────

#[tokio::test]
async fn movie_recommendations_without_totals_or_results_defaults_from_requested_page() {
    let server = MockServer::start().await;
    mount(&server, "/3/movie/550/recommendations", json("{}"), 2).await;
    let client = client(&server, |c| c);

    let requested = client.movie_recommendations(550, Some(4)).await.unwrap();
    assert_eq!(requested.page, 4);
    assert_eq!(requested.total_pages, 4);
    assert_eq!(requested.total_results, 0);
    assert!(requested.results.is_empty());

    let unrequested = client.movie_recommendations(550, None).await.unwrap();
    assert_eq!(unrequested.page, 1);
    assert_eq!(unrequested.total_pages, 1);
    assert_eq!(unrequested.total_results, 0);
    assert!(unrequested.results.is_empty());
}

#[tokio::test]
async fn search_multi_drops_unknown_media_types_and_keeps_totals() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "page": 2,
        "results": [
            { "id": 550, "media_type": "movie", "title": "Fight Club" },
            { "id": 10, "media_type": "collection", "name": "Some Collection" },
            { "id": 1396, "media_type": "tv", "name": "Breaking Bad" },
            { "id": 11, "name": "No media type" },
            { "id": 287, "media_type": "person", "name": "Brad Pitt" }
        ],
        "total_pages": 3,
        "total_results": 42
    });
    mount(
        &server,
        "/3/search/multi",
        ResponseTemplate::new(200).set_body_json(body),
        1,
    )
    .await;
    let client = client(&server, |c| c);

    let page = client.search_multi("q", Some(2)).await.unwrap();

    assert_eq!(page.page, 2);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.total_results, 42);
    let ids: Vec<String> = page
        .results
        .iter()
        .map(|r| match r {
            UnifiedSearchResult::Movie(m) => format!("movie {}", m.provider_id),
            UnifiedSearchResult::TvShow(s) => format!("tv {}", s.provider_id),
            UnifiedSearchResult::Person(p) => format!("person {}", p.provider_id),
            other => panic!("unexpected result {other:?}"),
        })
        .collect();
    assert_eq!(ids, ["movie tmdb:550", "tv tmdb:1396", "person tmdb:287"]);
}

#[tokio::test]
async fn watch_providers_with_null_results_is_an_empty_map() {
    let server = MockServer::start().await;
    let body = r#"{"id":550,"results":null}"#;
    mount(&server, "/3/movie/550/watch/providers", json(body), 1).await;
    mount(&server, "/3/tv/1396/watch/providers", json(body), 1).await;
    let client = client(&server, |c| c);

    let movie = client.movie_watch_providers(550).await.unwrap();
    let tv = client.tv_watch_providers(1396).await.unwrap();

    assert_eq!(movie.provider_id, "tmdb:550");
    assert!(movie.results.is_empty());
    assert_eq!(tv.provider_id, "tmdb:1396");
    assert!(tv.results.is_empty());
}

#[tokio::test]
async fn watch_providers_drop_nameless_services_and_use_w92_logos() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "id": 550,
        "results": {
            "US": {
                "link": "https://www.themoviedb.org/movie/550/watch",
                "flatrate": [
                    { "provider_name": "Netflix", "logo_path": "/netflix.jpg" },
                    { "logo_path": "/nameless.jpg" }
                ],
                "rent": [ { "provider_name": "Apple TV" } ]
            }
        }
    });
    mount(
        &server,
        "/3/movie/550/watch/providers",
        ResponseTemplate::new(200).set_body_json(body),
        1,
    )
    .await;
    let client = client(&server, |c| c);

    let providers = client.movie_watch_providers(550).await.unwrap();

    assert_eq!(providers.results.len(), 1);
    let us = &providers.results["US"];
    let services = |list: &[cameo::UnifiedStreamingService]| -> Vec<(String, Option<String>)> {
        list.iter()
            .map(|s| (s.name.clone(), s.logo_url.clone()))
            .collect()
    };
    assert_eq!(
        services(&us.flatrate),
        [(
            "Netflix".to_string(),
            Some("https://image.tmdb.org/t/p/w92/netflix.jpg".to_string())
        )]
    );
    assert_eq!(services(&us.rent), [("Apple TV".to_string(), None)]);
    assert!(us.buy.is_empty());
}

#[tokio::test]
async fn season_details_take_show_id_and_season_number_from_the_request() {
    let server = MockServer::start().await;
    let mut body: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/tv_season_details_response.json")).unwrap();
    // The body's own season number is absent: the requested one is used.
    body.as_object_mut()
        .expect("fixture is an object")
        .remove("season_number");
    mount(
        &server,
        "/3/tv/42/season/3",
        ResponseTemplate::new(200).set_body_json(body),
        1,
    )
    .await;
    let client = client(&server, |c| c);

    let season = client.tv_season_details(42, 3).await.unwrap();

    assert_eq!(season.show_id, "tmdb:42");
    assert_eq!(season.season_number, 3);
}

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Each retry re-acquires the limiter: three attempts under a one-request-per-
/// 300ms budget take at least 600ms even though the backoff is ~1ms.
#[tokio::test]
async fn every_retry_attempt_acquires_the_rate_limiter() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/3/search/movie",
        ResponseTemplate::new(500).set_body_raw(SERVER_ERROR, "application/json"),
        3,
    )
    .await;
    let client = client(&server, |c| {
        c.with_retry(TWO_RETRIES)
            .with_rate_limit(RateLimit::per(1, Duration::from_millis(300)))
    });

    let started = Instant::now();
    let err = client.search_movies("q", None).await.unwrap_err();
    let elapsed = started.elapsed();

    assert!(matches!(err, TmdbError::Api { status: 500, .. }), "{err:?}");
    assert!(
        elapsed >= Duration::from_millis(600),
        "3 paced attempts should take >= 600ms, took {elapsed:?}"
    );
}

#[tokio::test]
async fn clones_share_one_rate_limit_budget() {
    let server = MockServer::start().await;
    mount(&server, "/3/search/movie", json(EMPTY_PAGE), 2).await;
    let original = client(&server, |c| {
        c.with_rate_limit(RateLimit::per(1, Duration::from_millis(400)))
    });
    let clone = original.clone();

    let started = Instant::now();
    original.search_movies("q", None).await.unwrap();
    clone.search_movies("q", None).await.unwrap();
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_millis(400),
        "the clone should wait for the original's spent token, took {elapsed:?}"
    );
}

// ── Discover builders (outside the retry seam) ────────────────────────────────

/// Discover builders call the generated client directly rather than through
/// the retry seam: a 5xx is attempted once, and the generated client's error
/// conversion decodes TMDB's `{status_code, status_message}` error body.
#[tokio::test]
async fn discover_movies_is_not_retried_and_decodes_the_error_body() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/3/discover/movie",
        ResponseTemplate::new(500).set_body_raw(SERVER_ERROR, "application/json"),
        1,
    )
    .await;
    let client = client(&server, |c| c.with_retry(TWO_RETRIES));

    let err = client.discover_movies().execute().await.unwrap_err();

    match err {
        TmdbError::Api {
            status,
            status_code,
            message,
            retry_after,
        } => {
            assert_eq!(status, 500);
            assert_eq!(status_code, Some(11));
            assert_eq!(message, "Internal error.");
            assert_eq!(retry_after, None);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

/// See [`discover_movies_is_not_retried_and_decodes_the_error_body`].
#[tokio::test]
async fn discover_tv_is_not_retried_and_decodes_the_error_body() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/3/discover/tv",
        ResponseTemplate::new(429)
            .insert_header("Retry-After", "0")
            .set_body_raw(
                r#"{"status_code":25,"status_message":"limit"}"#,
                "application/json",
            ),
        1,
    )
    .await;
    let client = client(&server, |c| c.with_retry(TWO_RETRIES));

    let err = client.discover_tv().execute().await.unwrap_err();

    match err {
        TmdbError::Api {
            status,
            status_code,
            message,
            retry_after,
        } => {
            assert_eq!(status, 429);
            assert_eq!(status_code, Some(25));
            assert_eq!(message, "limit");
            assert_eq!(retry_after, Some(Duration::ZERO));
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}
