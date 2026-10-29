use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn search_movie_fixture() -> &'static str {
    include_str!("../fixtures/search_movie_response.json")
}

fn movie_details_fixture() -> &'static str {
    include_str!("../fixtures/movie_details_response.json")
}

/// Helper: set up a mock server and a `TmdbClient` pointing to it.
async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
    (server, client)
}

#[tokio::test]
async fn search_movie_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(query_param("query", "fight club"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let page = client.search_movies("fight club", None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);

    let movie = &page.results[0];
    assert!(movie.provider_id == "tmdb:550");
    assert_eq!(movie.title, "Fight Club");
    assert_eq!(movie.vote_count, Some(26280));
}

#[tokio::test]
async fn movie_details_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(movie_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let details = client.movie_details(550).await.unwrap();

    assert!(details.movie.provider_id == "tmdb:550");
    assert_eq!(details.movie.title, "Fight Club");
    assert_eq!(details.runtime, Some(139));
    assert_eq!(details.budget, Some(63000000));
    assert_eq!(details.imdb_id.as_deref(), Some("tt0137523"));
    assert_eq!(details.movie.genres.len(), 2);
    assert!(details.movie.genres.contains(&cameo::Genre::Drama));
    assert_eq!(details.video, Some(false));
    assert_eq!(details.movie.adult, Some(false));
}

/// Serve `body` as the `/3/movie/550` response and fetch it through the client.
async fn movie_details_with_body(body: serde_json::Value) -> cameo::UnifiedMovieDetails {
    let (server, client) = setup_mock_client().await;
    Mock::given(method("GET"))
        .and(path("/3/movie/550"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    client.movie_details(550).await.unwrap()
}

fn movie_details_json() -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str(movie_details_fixture()).unwrap() {
        serde_json::Value::Object(map) => map,
        other => panic!("fixture is not an object: {other:?}"),
    }
}

#[tokio::test]
async fn movie_details_accepts_null_video_and_adult() {
    let mut body = movie_details_json();
    body.insert("video".to_string(), serde_json::Value::Null);
    body.insert("adult".to_string(), serde_json::Value::Null);
    body.insert("vote_count".to_string(), serde_json::Value::Null);

    let details = movie_details_with_body(body.into()).await;

    assert_eq!(details.movie.title, "Fight Club");
    assert_eq!(details.video, None);
    assert_eq!(details.movie.adult, None);
    assert_eq!(details.movie.vote_count, Some(0));
}

#[tokio::test]
async fn movie_details_missing_video_and_adult_are_none() {
    let mut body = movie_details_json();
    body.remove("video");
    body.remove("adult");

    let details = movie_details_with_body(body.into()).await;

    // The spec's `default: true` must not turn absent flags into `true`.
    assert_eq!(details.video, None);
    assert_eq!(details.movie.adult, None);
}

#[tokio::test]
async fn movie_genres_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/genre/movie/list"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_genres_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let genres = client.movie_genres().await.unwrap();

    assert!(!genres.is_empty());
    assert!(genres.contains(&cameo::Genre::Action));
}

#[tokio::test]
async fn movie_images_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/images"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_images_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let images = client.movie_images(550).await.unwrap();

    assert_eq!(images.backdrops.len(), 1);
    assert!(
        images.backdrops[0]
            .url
            .as_deref()
            .unwrap()
            .ends_with("/hZkgoQYus5vegHoetLkCJzb17zJ.jpg")
    );
    assert_eq!(images.posters.len(), 1);
}

#[tokio::test]
async fn search_multi_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/search/multi"))
        .and(query_param("query", "fight club"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/search_multi_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let page = client.search_multi("fight club", None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 3);
    assert_eq!(page.results.len(), 3);
}

#[tokio::test]
async fn bearer_token_is_sent() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_movie_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    // Succeeds only if the Authorization header is present (wiremock verifies the matcher)
    let page = client.search_movies("test", None).await.unwrap();
    assert_eq!(page.page, 1);
}
