use cameo::generated::tmdb::Client as GeneratedClient;
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

/// Helper: set up a mock server and a generated client pointing to it.
async fn setup_mock_client() -> (MockServer, GeneratedClient) {
    let server = MockServer::start().await;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_static("Bearer test-token"),
    );
    let http = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap();
    let client = GeneratedClient::new_with_client(&server.uri(), http);
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

    let resp = client
        .search_movie(None, None, None, None, "fight club", None, None)
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);

    let movie = &body.results[0];
    assert_eq!(movie.id, 550);
    assert_eq!(movie.title.as_deref(), Some("Fight Club"));
    assert_eq!(movie.vote_count, 26280);
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

    let resp = client.movie_details(550, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 550);
    assert_eq!(body.title.as_deref(), Some("Fight Club"));
    assert_eq!(body.runtime, 139);
    assert_eq!(body.budget, 63000000);
    assert_eq!(body.imdb_id.as_deref(), Some("tt0137523"));
    assert_eq!(body.genres.len(), 2);
    assert_eq!(body.genres[0].name.as_deref(), Some("Drama"));
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

    let resp = client.genre_movie_list(None).await.unwrap();
    let body = resp.into_inner();

    assert!(!body.genres.is_empty());
    let action = body.genres.iter().find(|g| g.id == 28);
    assert!(action.is_some());
    assert_eq!(action.unwrap().name.as_deref(), Some("Action"));
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

    let resp = client.movie_images(550, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 550);
    assert_eq!(body.backdrops.len(), 1);
    assert_eq!(
        body.backdrops[0].file_path.as_deref(),
        Some("/hZkgoQYus5vegHoetLkCJzb17zJ.jpg")
    );
    assert_eq!(body.posters.len(), 1);
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

    let resp = client
        .search_multi(None, None, None, "fight club")
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 3);
    assert_eq!(body.results.len(), 3);
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
    let resp = client
        .search_movie(None, None, None, None, "test", None, None)
        .await
        .unwrap();
    assert_eq!(resp.into_inner().page, 1);
}
