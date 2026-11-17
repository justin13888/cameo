//! `CameoClientBuilder::with_tvdb`: TheTVDB registers for search, details,
//! seasons and external ids, and `tvdb:*` ids route to it.

use cameo::{
    RateLimit, RetryPolicy,
    providers::tvdb::TvdbConfig,
    unified::{CameoClient, CameoClientError, MediaId},
};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

fn ok(data: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "status": "success", "data": data }))
}

async fn tvdb_only(server: &MockServer) -> CameoClient {
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ok(json!({ "token": "T" })))
        .mount(server)
        .await;
    CameoClient::builder()
        .with_tvdb(
            TvdbConfig::new("key")
                .with_base_url(server.uri())
                .with_rate_limit(RateLimit::Unlimited)
                .with_retry(RetryPolicy::disabled()),
        )
        .build()
        .unwrap()
}

#[tokio::test]
async fn registers_as_the_tvdb_provider() {
    let server = MockServer::start().await;
    let client = tvdb_only(&server).await;
    assert_eq!(client.provider_ids(), vec!["tvdb"]);
}

#[tokio::test]
async fn routes_tvdb_ids_for_details_and_seasons() {
    let server = MockServer::start().await;
    let client = tvdb_only(&server).await;
    Mock::given(method("GET"))
        .and(path("/series/81189/extended"))
        .respond_with(ok(json!({ "id": 81189, "name": "Breaking Bad" })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/series/81189/episodes/default/eng"))
        .respond_with(ok(json!({ "episodes": [
            { "id": 1, "seasonNumber": 0, "number": 1, "absoluteNumber": 0, "name": "Special" }
        ] })))
        .mount(&server)
        .await;

    let id = MediaId::tvdb(81189);
    let details = client.tv_show_details(&id).await.unwrap();
    assert_eq!(details.show.name, "Breaking Bad");
    let specials = client.season_details(&id, 0).await.unwrap();
    assert_eq!(specials.episodes[0].name.as_deref(), Some("Special"));
}

#[tokio::test]
async fn searches_through_the_facade() {
    let server = MockServer::start().await;
    let client = tvdb_only(&server).await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ok(
            json!([{ "tvdb_id": "81189", "type": "series", "name": "Breaking Bad" }]),
        ))
        .mount(&server)
        .await;
    let page = client
        .search_tv_shows_with("tvdb", "breaking", None)
        .await
        .unwrap();
    assert_eq!(page.results[0].provider_id, MediaId::tvdb(81189));
}

#[tokio::test]
async fn capabilities_tvdb_lacks_are_unsupported() {
    let server = MockServer::start().await;
    let client = tvdb_only(&server).await;
    let id = MediaId::tvdb(81189);
    assert!(matches!(
        client.tv_show_credits(&id).await,
        Err(CameoClientError::Unsupported)
    ));
    assert!(matches!(
        client.popular_movies(None).await,
        Err(CameoClientError::Unsupported)
    ));
}

#[test]
fn an_invalid_config_fails_the_build() {
    let err = CameoClient::builder()
        .with_tvdb(TvdbConfig::new(""))
        .build()
        .unwrap_err();
    assert!(
        matches!(
            err,
            CameoClientError::Provider(cameo::ProviderError::InvalidInput(_))
        ),
        "{err:?}"
    );
}
