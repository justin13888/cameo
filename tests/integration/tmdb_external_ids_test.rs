use cameo::{
    ExternalId, ExternalMatch, MediaId,
    providers::tmdb::{TmdbClient, TmdbConfig, TmdbError},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
    (server, client)
}

#[tokio::test]
async fn movie_external_ids_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_external_ids_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let ids = client.movie_external_ids(550).await.unwrap();

    assert_eq!(ids.tmdb, Some(550));
    assert_eq!(ids.imdb.as_deref(), Some("tt0137523"));
    assert_eq!(ids.wikidata.as_deref(), Some("Q190050"));
    assert_eq!(ids.tvdb, None);
    assert_eq!(ids.mal, None);
    assert_eq!(ids.anilist, None);
}

#[tokio::test]
async fn tv_external_ids_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_external_ids_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let ids = client.tv_external_ids(1396).await.unwrap();

    assert_eq!(ids.tmdb, Some(1396));
    assert_eq!(ids.imdb.as_deref(), Some("tt0903747"));
    assert_eq!(ids.tvdb, Some(81189));
    assert_eq!(ids.wikidata.as_deref(), Some("Q1079"));
}

/// TMDB sends `null` for unknown numeric ids and `""` for unknown string ids;
/// both come back as `None`.
#[tokio::test]
async fn tv_external_ids_accepts_null_and_empty_ids() {
    let (server, client) = setup_mock_client().await;

    let mut body: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/tv_external_ids_response.json")).unwrap();
    body["tvdb_id"] = serde_json::Value::Null;
    body["imdb_id"] = serde_json::Value::String(String::new());
    body["wikidata_id"] = serde_json::Value::Null;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let ids = client.tv_external_ids(1396).await.unwrap();

    assert_eq!(ids.tmdb, Some(1396));
    assert_eq!(ids.tvdb, None);
    assert_eq!(ids.imdb, None);
    assert_eq!(ids.wikidata, None);
}

#[tokio::test]
async fn find_by_imdb_id_returns_movie_match() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/find/tt0137523"))
        .and(query_param("external_source", "imdb_id"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/find_by_imdb_id_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let matches = client
        .find_by_external_id(&ExternalId::imdb("tt0137523"))
        .await
        .unwrap();

    assert_eq!(matches, vec![ExternalMatch::movie(MediaId::tmdb(550))]);
}

#[tokio::test]
async fn find_by_tvdb_id_returns_tv_match() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/find/81189"))
        .and(query_param("external_source", "tvdb_id"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/find_by_tvdb_id_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let matches = client
        .find_by_external_id(&ExternalId::tvdb(81189))
        .await
        .unwrap();

    assert_eq!(matches, vec![ExternalMatch::tv_show(MediaId::tmdb(1396))]);
}

/// TV results without a usable numeric `id` are skipped, not fatal.
#[tokio::test]
async fn find_skips_malformed_tv_results() {
    let (server, client) = setup_mock_client().await;

    let mut body: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/find_by_tvdb_id_response.json")).unwrap();
    let tv = body["tv_results"].as_array_mut().unwrap();
    tv.push(serde_json::json!({ "name": "no id" }));
    tv.push(serde_json::json!({ "id": "not-a-number" }));

    Mock::given(method("GET"))
        .and(path("/3/find/81189"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let matches = client
        .find_by_external_id(&ExternalId::tvdb(81189))
        .await
        .unwrap();

    assert_eq!(matches, vec![ExternalMatch::tv_show(MediaId::tmdb(1396))]);
}

#[tokio::test]
async fn find_by_wikidata_id_with_no_results_is_empty() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/find/Q0"))
        .and(query_param("external_source", "wikidata_id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "movie_results": [],
            "person_results": [],
            "tv_results": [],
            "tv_episode_results": [],
            "tv_season_results": []
        })))
        .mount(&server)
        .await;

    let matches = client
        .find_by_external_id(&ExternalId::wikidata("Q0"))
        .await
        .unwrap();

    assert!(matches.is_empty());
}

#[tokio::test]
async fn find_by_mal_id_is_unsupported_without_a_request() {
    let (server, client) = setup_mock_client().await;

    let err = client
        .find_by_external_id(&ExternalId::mal(5114))
        .await
        .unwrap_err();

    assert!(matches!(err, TmdbError::UnsupportedSource(_)), "{err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}
