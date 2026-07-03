use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
    (server, client)
}

#[tokio::test]
async fn movie_credits_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/credits"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_credits_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let credits = client.movie_credits(550).await.unwrap();

    assert_eq!(credits.cast.len(), 1);
    assert_eq!(credits.cast[0].id, 819);
    assert_eq!(credits.cast[0].name, "Edward Norton");
    assert_eq!(credits.cast[0].character.as_deref(), Some("The Narrator"));
    assert_eq!(credits.crew.len(), 1);
    assert_eq!(credits.crew[0].name, "David Fincher");
    assert_eq!(credits.crew[0].job.as_deref(), Some("Director"));
}

#[tokio::test]
async fn tv_series_credits_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/aggregate_credits"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_credits_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let credits = client.tv_series_credits(1396).await.unwrap();

    assert_eq!(credits.cast.len(), 1);
    assert_eq!(credits.cast[0].id, 17419);
    assert_eq!(credits.cast[0].name, "Bryan Cranston");
    assert_eq!(credits.cast[0].character.as_deref(), Some("Walter White"));
    assert_eq!(credits.crew.len(), 1);
    assert_eq!(credits.crew[0].name, "Vince Gilligan");
}
