//! Pins every observable behaviour of [`AniListClient`] against a mock
//! GraphQL endpoint: the response decision table of the GraphQL transport,
//! the request each method sends, the page-metadata defaults, and the retry
//! loop.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use wiremock::{Mock, MockServer, Request, ResponseTemplate, matchers::method};

use super::AniListClient;
use crate::{
    core::{
        config::TimeWindow,
        pagination::{Page, PageToken},
    },
    providers::{
        anilist::{config::AniListConfig, convert::TV_FORMATS, error::AniListError, query},
        rate_limit::RateLimit,
        retry::{RetryPolicy, Retryable},
    },
    unified::{external_ids::ExternalMatch, media_id::MediaId, models::UnifiedSearchResult},
};

// ── Harness ────────────────────────────────────────────────────────────────

/// A retry policy with two retries and near-zero backoff.
const TWO_FAST_RETRIES: RetryPolicy =
    RetryPolicy::new(2, Duration::from_millis(1), Duration::from_millis(1));

/// A client for `server` with retrying and rate limiting disabled, after
/// applying `configure`.
fn client_with(
    server: &MockServer,
    configure: impl FnOnce(AniListConfig) -> AniListConfig,
) -> AniListClient {
    let config = AniListConfig::new_with_base_url(server.uri())
        .with_retry(RetryPolicy::disabled())
        .with_rate_limit(RateLimit::Unlimited);
    AniListClient::new(configure(config)).expect("valid config")
}

fn client_for(server: &MockServer) -> AniListClient {
    client_with(server, |c| c)
}

/// Start a server answering every POST with `template`, verifying on drop
/// that it was hit exactly `hits` times.
async fn serve(template: ResponseTemplate, hits: u64) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(template)
        .expect(hits)
        .mount(&server)
        .await;
    server
}

fn json_response(status: u16, body: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(body)
}

fn raw_response(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "text/plain")
}

async fn requests(server: &MockServer) -> Vec<Request> {
    server
        .received_requests()
        .await
        .expect("request recording is enabled")
}

fn body_of(request: &Request) -> Value {
    serde_json::from_slice(&request.body).expect("JSON request body")
}

/// The JSON body of the single request `server` received, after checking
/// the request's method and JSON headers.
async fn only_request(server: &MockServer) -> Value {
    let requests = requests(server).await;
    assert_eq!(requests.len(), 1, "expected exactly one request");
    let request = &requests[0];
    assert_eq!(request.method.as_str(), "POST");
    for header in ["content-type", "accept"] {
        assert_eq!(
            request.headers.get(header).and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "{header} header"
        );
    }
    body_of(request)
}

// ── Fixtures ───────────────────────────────────────────────────────────────

fn media(id: i32, format: &str) -> Value {
    json!({
        "id": id,
        "title": { "romaji": "Romaji", "english": "English", "native": "Native" },
        "format": format
    })
}

fn page_info() -> Value {
    json!({ "total": 41, "currentPage": 2, "lastPage": 3, "hasNextPage": true })
}

fn media_page(items: Vec<Value>) -> Value {
    json!({ "data": { "Page": { "pageInfo": page_info(), "media": items } } })
}

fn staff_page() -> Value {
    json!({ "data": { "Page": {
            "pageInfo": page_info(),
            "staff": [{ "id": 3, "name": { "full": "Person" } }]
        } } })
}

fn details() -> Value {
    json!({ "data": { "Media": media(5, "MOVIE") } })
}

fn staff_details() -> Value {
    json!({ "data": { "Staff": { "id": 3, "name": { "full": "Person" } } } })
}

fn ids(format: &str) -> Value {
    json!({ "data": { "Media": { "id": 5, "idMal": 9, "format": format } } })
}

fn recommendations(nodes: Vec<Value>) -> Value {
    json!({ "data": { "Media": { "recommendations": {
            "pageInfo": page_info(),
            "nodes": nodes
        } } } })
}

fn credits() -> Value {
    json!({ "data": { "Media": {
            "id": 5,
            "characters": { "pageInfo": { "hasNextPage": false }, "edges": [] },
            "staff": { "pageInfo": { "hasNextPage": false }, "edges": [] }
        } } })
}

fn with_errors(mut body: Value, errors: Value) -> Value {
    body["errors"] = errors;
    body
}

fn graphql_errors(err: &AniListError) -> &[crate::providers::anilist::error::AniListGqlError] {
    match err {
        AniListError::GraphQL { errors, .. } => errors,
        other => panic!("expected AniListError::GraphQL, got {other:?}"),
    }
}

// ── graphql_attempt decision table ─────────────────────────────────────────

#[tokio::test]
async fn success_with_data_returns_the_root() {
    let server = serve(json_response(200, details()), 1).await;
    let movie = client_for(&server).movie_details(5).await.expect("ok");
    assert_eq!(movie.movie.provider_id, "anilist:5");
}

#[tokio::test]
async fn error_status_404_is_not_found_even_with_data() {
    let body = with_errors(
        details(),
        json!([{ "message": "Not Found.", "status": 404 }]),
    );
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn errors_with_data_return_the_partial_data() {
    for status in [400, 500] {
        let body = with_errors(
            details(),
            json!([{ "message": "partial", "status": status }]),
        );
        let server = serve(json_response(200, body), 1).await;
        // Retries are enabled: a usable root is returned on the first attempt.
        let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
        let movie = client.movie_details(5).await.expect("partial data");
        assert_eq!(movie.movie.provider_id, "anilist:5");
    }
}

#[tokio::test]
async fn errors_without_data_are_graphql_errors_with_retry_after() {
    let template = json_response(
        200,
        json!({ "errors": [{ "message": "bad query", "status": 400 }] }),
    )
    .insert_header("Retry-After", "7");
    let server = serve(template, 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    let errors = graphql_errors(&err);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "bad query");
    assert_eq!(errors[0].status, Some(400));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
}

#[tokio::test]
async fn errors_with_a_null_root_are_graphql_errors() {
    let body = json!({
        "data": { "Media": null },
        "errors": [{ "message": "failed", "status": 200 }]
    });
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, Some(200));
}

#[tokio::test]
async fn statusless_errors_inherit_only_a_non_2xx_status() {
    let body = json!({ "errors": [{ "message": "no status" }] });

    let server = serve(json_response(400, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, Some(400));

    let server = serve(json_response(200, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, None);

    // An inherited 404 is not-found.
    let server = serve(json_response(404, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn an_empty_errors_array_is_treated_as_no_errors() {
    let server = serve(json_response(200, with_errors(details(), json!([]))), 1).await;
    let movie = client_for(&server).movie_details(5).await.expect("ok");
    assert_eq!(movie.movie.provider_id, "anilist:5");

    let body = json!({ "data": { "Media": null }, "errors": [] });
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");

    let server = serve(json_response(200, json!({ "errors": [] })), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NoData), "got {err:?}");
}

#[tokio::test]
async fn a_non_json_error_response_is_a_status_error() {
    let template =
        raw_response(502, "  <html>Bad Gateway</html>\n").insert_header("Retry-After", "3");
    let server = serve(template, 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    match err {
        AniListError::Status {
            status,
            retry_after,
            message,
        } => {
            assert_eq!(status, 502);
            assert_eq!(retry_after, Some(Duration::from_secs(3)));
            assert_eq!(message, "<html>Bad Gateway</html>");
        }
        other => panic!("expected Status, got {other:?}"),
    }

    let server = serve(raw_response(503, ""), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(&err, AniListError::Status { status: 503, message, .. } if message == "empty response body"),
        "got {err:?}"
    );

    let long = "x".repeat(300);
    let server = serve(raw_response(500, &long), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    let expected = format!("{}…", "x".repeat(200));
    assert!(
        matches!(&err, AniListError::Status { status: 500, message, .. } if *message == expected),
        "got {err:?}"
    );
}

#[tokio::test]
async fn an_undecodable_success_response_is_a_deserialization_error() {
    let server = serve(raw_response(200, "not json"), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Deserialization(_)),
        "got {err:?}"
    );

    // Valid JSON of the wrong shape is a decode error too...
    let body = json!({ "data": { "Media": { "id": "five" } } });
    let server = serve(json_response(200, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Deserialization(_)),
        "got {err:?}"
    );

    // ...unless the status is an error, which is the better explanation.
    let server = serve(json_response(500, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 500, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn no_data_and_no_errors_on_an_error_status_is_a_status_error() {
    let server = serve(json_response(503, json!({})), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(&err, AniListError::Status { status: 503, message, .. } if message == "{}"),
        "got {err:?}"
    );

    let server = serve(json_response(500, json!({ "data": { "Media": null } })), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 500, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn no_data_and_no_errors_on_success_is_no_data() {
    for body in [json!({}), json!({ "data": null })] {
        // Even a details call reports NoData, not NotFound.
        let server = serve(json_response(200, body.clone()), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");

        let server = serve(json_response(200, body), 1).await;
        let err = client_for(&server).popular_movies(None).await.unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");
    }
}

// ── Request bodies ─────────────────────────────────────────────────────────

/// The formats a TV-typed list request filters on.
fn tv_formats() -> Value {
    json!(["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"])
}

/// Run `$call` against a server answering `$response`, with `perPage` 7,
/// and assert the one request sent carried `$query` and exactly `$vars`.
macro_rules! assert_sends {
    ($response:expr, |$client:ident| $call:expr, $query:expr, $vars:expr $(,)?) => {{
        let server = serve(json_response(200, $response), 1).await;
        let $client = client_with(&server, |c| c.with_per_page(7));
        $call.await.expect("call succeeds");
        let body = only_request(&server).await;
        assert_eq!(body["query"], $query, "query text");
        assert_eq!(body["variables"], $vars, "variables");
        assert_eq!(
            body.as_object().map(|o| o.len()),
            Some(2),
            "body holds only query and variables"
        );
    }};
}

#[tokio::test]
async fn search_requests_filter_formats_by_media_type() {
    assert_sends!(
        media_page(vec![]),
        |c| c.search_movies("naruto", None),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 1, "perPage": 7, "formatIn": ["MOVIE"] }),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.search_tv_shows("naruto", Some(3)),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 3, "perPage": 7, "formatIn": tv_formats() }),
    );
}

#[tokio::test]
async fn search_multi_and_people_omit_the_format_filter() {
    assert_sends!(
        media_page(vec![]),
        |c| c.search_multi("naruto", Some(2)),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 2, "perPage": 7 }),
    );
    assert_sends!(
        staff_page(),
        |c| c.search_people("kaji", None),
        query::SEARCH_STAFF,
        json!({ "query": "kaji", "page": 1, "perPage": 7 }),
    );
}

#[tokio::test]
async fn list_requests_send_page_per_page_and_formats() {
    let movies = json!({ "page": 1, "perPage": 7, "formatIn": ["MOVIE"] });
    let tv = json!({ "page": 4, "perPage": 7, "formatIn": tv_formats() });
    assert_sends!(
        media_page(vec![]),
        |c| c.trending_movies(TimeWindow::Week, None),
        query::LIST_TRENDING_ANIME,
        movies.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.trending_tv(TimeWindow::Day, Some(4)),
        query::LIST_TRENDING_ANIME,
        tv.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.popular_movies(None),
        query::LIST_POPULAR_ANIME,
        movies.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.popular_tv_shows(Some(4)),
        query::LIST_POPULAR_ANIME,
        tv.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.top_rated_movies(None),
        query::LIST_TOP_SCORED_ANIME,
        movies,
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.top_rated_tv_shows(Some(4)),
        query::LIST_TOP_SCORED_ANIME,
        tv,
    );
}

#[tokio::test]
async fn per_page_defaults_to_20() {
    let server = serve(json_response(200, media_page(vec![])), 1).await;
    client_for(&server)
        .popular_movies(None)
        .await
        .expect("call succeeds");
    let body = only_request(&server).await;
    assert_eq!(body["variables"]["perPage"], 20);
    assert_eq!(body["variables"]["page"], 1);
}

#[tokio::test]
async fn id_requests_send_only_the_id() {
    assert_sends!(
        details(),
        |c| c.movie_details(5),
        query::MEDIA_DETAILS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        details(),
        |c| c.tv_show_details(5),
        query::MEDIA_DETAILS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        staff_details(),
        |c| c.person_details(3),
        query::STAFF_DETAILS,
        json!({ "id": 3 }),
    );
    assert_sends!(
        ids("TV"),
        |c| c.external_ids(5),
        query::MEDIA_EXTERNAL_IDS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        ids("TV"),
        |c| c.find_by_mal_id(9),
        query::MEDIA_BY_MAL_ID,
        json!({ "idMal": 9 }),
    );
}

#[tokio::test]
async fn recommendation_and_credit_requests_send_their_paging() {
    let vars = json!({ "id": 5, "page": 1, "perPage": 7 });
    assert_sends!(
        recommendations(vec![]),
        |c| c.movie_recommendations(5, None),
        query::MEDIA_RECOMMENDATIONS,
        vars.clone(),
    );
    assert_sends!(
        recommendations(vec![]),
        |c| c.tv_recommendations(5, Some(1)),
        query::MEDIA_RECOMMENDATIONS,
        vars,
    );
    // Credits page with their own fixed size, not the configured one.
    assert_sends!(
        credits(),
        |c| c.media_credits(5),
        query::MEDIA_CREDITS,
        json!({ "id": 5, "characterPage": 1, "staffPage": 1, "perPage": 25 }),
    );
}

#[tokio::test]
async fn a_negative_credits_id_is_not_found_without_a_request() {
    let server = serve(json_response(200, credits()), 0).await;
    let err = client_for(&server).media_credits(-1).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn a_zero_credits_id_is_not_found_without_a_request() {
    let server = serve(json_response(200, credits()), 0).await;
    let err = client_for(&server).media_credits(0).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

// ── Page mapping ───────────────────────────────────────────────────────────

fn assert_passed_through<T>(page: &Page<T>) {
    assert_eq!(page.page, 2);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.total_results, 41);
    assert!(page.has_next);
    assert_eq!(page.next, Some(PageToken::Page(3)));
}

fn assert_defaults<T>(page: &Page<T>) {
    assert_eq!(page.page, 1);
    // Omitted totals are unknown, which `Page` documents as `0`.
    assert_eq!(page.total_pages, 0);
    assert_eq!(page.total_results, 0);
    assert!(!page.has_next);
    assert_eq!(page.next, None);
}

#[tokio::test]
async fn page_metadata_is_passed_through() {
    let server = serve(json_response(200, media_page(vec![media(1, "MOVIE")])), 3).await;
    let client = client_for(&server);
    let movies = client.popular_movies(None).await.expect("ok");
    assert_passed_through(&movies);
    assert_eq!(movies.results.len(), 1);
    assert_passed_through(&client.popular_tv_shows(None).await.expect("ok"));
    assert_passed_through(&client.search_multi("x", None).await.expect("ok"));

    let node = json!({ "mediaRecommendation": media(7, "MOVIE") });
    let server = serve(json_response(200, recommendations(vec![node])), 1).await;
    let recs = client_for(&server)
        .movie_recommendations(5, None)
        .await
        .expect("ok");
    assert_passed_through(&recs);
    assert_eq!(recs.results[0].provider_id, "anilist:7");

    let server = serve(json_response(200, staff_page()), 1).await;
    let people = client_for(&server)
        .search_people("x", None)
        .await
        .expect("ok");
    assert_passed_through(&people);
    assert_eq!(people.results[0].provider_id, "anilist:staff:3");
}

#[tokio::test]
async fn missing_page_metadata_takes_defaults() {
    let media = json!({ "data": { "Page": { "pageInfo": {} } } });
    let server = serve(json_response(200, media), 3).await;
    let client = client_for(&server);
    let movies = client.top_rated_movies(None).await.expect("ok");
    assert_defaults(&movies);
    assert!(movies.results.is_empty());
    assert_defaults(&client.search_tv_shows("x", None).await.expect("ok"));
    assert_defaults(&client.search_multi("x", None).await.expect("ok"));

    let staff = json!({ "data": { "Page": { "pageInfo": {} } } });
    let server = serve(json_response(200, staff), 1).await;
    assert_defaults(
        &client_for(&server)
            .search_people("x", None)
            .await
            .expect("ok"),
    );

    let recs = json!({ "data": { "Media": { "recommendations": { "pageInfo": {} } } } });
    let server = serve(json_response(200, recs), 1).await;
    assert_defaults(
        &client_for(&server)
            .tv_recommendations(5, None)
            .await
            .expect("ok"),
    );

    // A missing recommendation connection is an empty first page.
    let none = json!({ "data": { "Media": { "recommendations": null } } });
    let server = serve(json_response(200, none), 1).await;
    let page = client_for(&server)
        .movie_recommendations(5, None)
        .await
        .expect("ok");
    assert_defaults(&page);
    assert!(page.results.is_empty());
}

#[tokio::test]
async fn a_missing_page_is_no_data() {
    for body in [json!({ "data": {} }), json!({ "data": { "Page": null } })] {
        let server = serve(json_response(200, body), 4).await;
        let client = client_for(&server);
        let errors = [
            client.popular_movies(None).await.map(drop).unwrap_err(),
            client
                .search_tv_shows("x", None)
                .await
                .map(drop)
                .unwrap_err(),
            client.search_multi("x", None).await.map(drop).unwrap_err(),
            client.search_people("x", None).await.map(drop).unwrap_err(),
        ];
        for err in errors {
            assert!(matches!(err, AniListError::NoData), "got {err:?}");
        }
    }
}

#[tokio::test]
async fn a_null_root_on_single_item_lookups_is_not_found() {
    let media = json!({ "data": { "Media": null } });
    let server = serve(json_response(200, media.clone()), 5).await;
    let client = client_for(&server);
    let errors = [
        client.movie_details(5).await.map(drop).unwrap_err(),
        client.tv_show_details(5).await.map(drop).unwrap_err(),
        client.external_ids(5).await.map(drop).unwrap_err(),
        client
            .movie_recommendations(5, None)
            .await
            .map(drop)
            .unwrap_err(),
        client.media_credits(5).await.map(drop).unwrap_err(),
    ];
    for err in errors {
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    let server = serve(json_response(200, json!({ "data": { "Staff": null } })), 1).await;
    let err = client_for(&server).person_details(3).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");

    // A MAL lookup reports absence as an empty match instead.
    let server = serve(json_response(200, media), 1).await;
    assert_eq!(
        client_for(&server).find_by_mal_id(9).await.expect("ok"),
        None
    );
}

#[tokio::test]
async fn search_multi_classifies_by_format_and_drops_formatless_media() {
    let mut formatless = media(3, "TV");
    formatless["format"] = Value::Null;
    let body = media_page(vec![media(1, "MOVIE"), media(2, "OVA"), formatless]);
    let server = serve(json_response(200, body), 1).await;
    let page = client_for(&server)
        .search_multi("x", None)
        .await
        .expect("ok");
    assert_eq!(page.results.len(), 2);
    assert!(matches!(page.results[0], UnifiedSearchResult::Movie(_)));
    assert!(matches!(page.results[1], UnifiedSearchResult::TvShow(_)));
}

#[tokio::test]
async fn external_ids_map_mal_and_anilist_ids() {
    let server = serve(json_response(200, ids("TV")), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.anilist, Some(5));
    assert_eq!(ids.mal, Some(9));
    assert_eq!(ids.imdb, None);

    let body = json!({ "data": { "Media": { "id": 5, "idMal": -1 } } });
    let server = serve(json_response(200, body), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.mal, None, "a negative MAL id is dropped");

    let body = json!({ "data": { "Media": { "id": 0, "idMal": 9 } } });
    let server = serve(json_response(200, body), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.anilist, None, "a reported AniList id of 0 is dropped");
    assert_eq!(ids.mal, Some(9));
}

#[tokio::test]
async fn find_by_mal_id_drops_a_media_id_of_zero() {
    let body = json!({ "data": { "Media": { "id": 0, "format": "MOVIE" } } });
    let server = serve(json_response(200, body), 1).await;
    let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
    assert_eq!(found, None);
}

#[tokio::test]
async fn find_by_mal_id_classifies_movie_and_series() {
    let movie = MediaId::anilist(5);
    let server = serve(json_response(200, ids("MOVIE")), 1).await;
    let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
    assert_eq!(found, Some(ExternalMatch::movie(movie.clone())));

    for format in TV_FORMATS {
        let body = json!({ "data": { "Media": { "id": 5, "format": format.as_str() } } });
        let server = serve(json_response(200, body), 1).await;
        let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
        assert_eq!(found, Some(ExternalMatch::tv_show(movie.clone())));
    }

    // A format `classify` places in neither list is not a match.
    for format in [json!("MUSIC"), json!("NOT_A_FORMAT"), Value::Null] {
        let body = json!({ "data": { "Media": { "id": 5, "format": format } } });
        let server = serve(json_response(200, body), 1).await;
        let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
        assert_eq!(found, None, "format {format}");
    }

    let not_found = json!({ "errors": [{ "message": "Not Found.", "status": 404 }] });
    let server = serve(json_response(404, not_found), 1).await;
    assert_eq!(
        client_for(&server).find_by_mal_id(9).await.expect("ok"),
        None
    );
}

// ── Retry loop ─────────────────────────────────────────────────────────────

/// Call `movie_details` against a server answering `template`, with two
/// fast retries, asserting the server is hit exactly `attempts` times.
async fn attempts_for(template: ResponseTemplate, attempts: u64) -> AniListError {
    let server = serve(template, attempts).await;
    let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
    let err = client.movie_details(5).await.unwrap_err();
    drop(server); // verifies the attempt count
    err
}

#[tokio::test]
async fn rate_limit_and_server_errors_are_retried_until_the_policy_gives_up() {
    let rate_limited = json_response(
        429,
        json!({ "errors": [{ "message": "Too Many Requests.", "status": 429 }] }),
    )
    .insert_header("Retry-After", "0");
    let err = attempts_for(rate_limited, 3).await;
    assert_eq!(graphql_errors(&err)[0].status, Some(429));

    let err = attempts_for(raw_response(503, "unavailable"), 3).await;
    assert!(
        matches!(err, AniListError::Status { status: 503, .. }),
        "got {err:?}"
    );

    let gql_5xx = json_response(
        200,
        json!({ "data": { "Media": null }, "errors": [{ "message": "boom", "status": 502 }] }),
    );
    let err = attempts_for(gql_5xx, 3).await;
    assert_eq!(graphql_errors(&err)[0].status, Some(502));
}

#[tokio::test]
async fn non_transient_errors_are_not_retried() {
    let bad_request = json_response(
        200,
        json!({ "errors": [{ "message": "x", "status": 400 }] }),
    );
    assert!(matches!(
        attempts_for(bad_request, 1).await,
        AniListError::GraphQL { .. }
    ));
    let not_found = json_response(404, json!({ "errors": [{ "message": "Not Found." }] }));
    assert!(matches!(
        attempts_for(not_found, 1).await,
        AniListError::NotFound
    ));
    assert!(matches!(
        attempts_for(raw_response(200, "not json"), 1).await,
        AniListError::Deserialization(_)
    ));
    assert!(matches!(
        attempts_for(raw_response(404, "gone"), 1).await,
        AniListError::Status { status: 404, .. }
    ));
}

#[tokio::test]
async fn a_disabled_retry_policy_makes_one_attempt() {
    let server = serve(raw_response(503, "unavailable"), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 503, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn a_retry_recovers_with_the_next_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(raw_response(503, "unavailable"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(json_response(200, details()))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
    let movie = client.movie_details(5).await.expect("recovers");
    assert_eq!(movie.movie.provider_id, "anilist:5");
    // Every attempt sends the same request.
    let sent = requests(&server).await;
    assert_eq!(body_of(&sent[0]), body_of(&sent[1]));
}

#[tokio::test]
async fn every_attempt_acquires_the_rate_limiter() {
    const WINDOW: Duration = Duration::from_millis(300);
    let server = serve(raw_response(503, "unavailable"), 3).await;
    // One request per window: the first attempt spends the only token, so
    // each of the two retries waits for a refill.
    let client = client_with(&server, |c| {
        c.with_retry(TWO_FAST_RETRIES)
            .with_rate_limit(RateLimit::per(1, WINDOW))
    });
    let started = Instant::now();
    let err = client.movie_details(5).await.unwrap_err();
    let elapsed = started.elapsed();
    assert!(matches!(err, AniListError::Status { status: 503, .. }));
    // Backoff is ~1ms per retry, so only the limiter explains the wait.
    assert!(
        elapsed >= WINDOW * 2 - Duration::from_millis(20),
        "three attempts took only {elapsed:?}"
    );
}

/// A details response with an invalid id is converted under the
/// requested id instead of a wrapped one.
#[tokio::test]
async fn movie_details_with_invalid_reported_id_uses_requested_id() {
    let server = serve(
        ResponseTemplate::new(200).set_body_json(json!({
            "data": { "Media": { "id": -1, "format": "MOVIE" } }
        })),
        1,
    )
    .await;
    let client = client_with(&server, |c| c);
    let movie = client.movie_details(5).await.expect("details");
    assert_eq!(movie.movie.provider_id, "anilist:5");
}
