//! Both TMDB auth modes (v4 bearer, v3 api_key) against wiremock, plus config
//! validation (#39).

use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn search_fixture() -> &'static str {
    include_str!("../fixtures/search_movie_response.json")
}

#[tokio::test]
async fn v4_bearer_sends_authorization_header() {
    let server = MockServer::start().await;
    let client = TmdbClient::new(TmdbConfig::new_with_base_url("v4-token", server.uri())).unwrap();

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(header("authorization", "Bearer v4-token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(search_fixture(), "application/json"))
        .mount(&server)
        .await;

    // Succeeds only if the Bearer header matched.
    let page = client.search_movies("x", None).await.unwrap();
    assert_eq!(page.page, 1);
}

#[tokio::test]
async fn v3_api_key_sends_query_param() {
    let server = MockServer::start().await;
    let client = TmdbClient::new(TmdbConfig::new_v3("v3-key").with_base_url(server.uri())).unwrap();

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(query_param("api_key", "v3-key"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(search_fixture(), "application/json"))
        .mount(&server)
        .await;

    // Succeeds only if the `?api_key=v3-key` query param was injected on the
    // request by the pre-hook.
    let page = client.search_movies("x", None).await.unwrap();
    assert_eq!(page.page, 1);
}

#[test]
fn bad_config_is_rejected_with_clear_errors() {
    // Empty credential.
    assert!(TmdbClient::new(TmdbConfig::new("")).is_err());

    // Malformed locale.
    let err = TmdbClient::new(TmdbConfig::new("t").with_language("english")).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("language"), "{err}");

    // Malformed region.
    let err = TmdbClient::new(TmdbConfig::new("t").with_region("usa")).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("region"), "{err}");

    // A well-formed config is accepted.
    assert!(
        TmdbClient::new(
            TmdbConfig::new("t")
                .with_language("en-US")
                .with_region("US")
        )
        .is_ok()
    );
}
