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

mod graphql_attempt;
mod page_mapping;
mod request_bodies;
mod retry_loop;

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
