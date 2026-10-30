use cameo::{
    ExternalId, ExternalIdProvider, ExternalMatch, MediaId, ProviderError,
    providers::anilist::{AniListClient, AniListConfig, AniListError},
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method},
};

async fn mock_client(variables: Value, body: Value) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": variables })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();
    (server, client)
}

fn not_found() -> Value {
    json!({
        "data": { "Media": null },
        "errors": [{ "message": "Not Found.", "status": 404 }]
    })
}

// ── external_ids ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn external_ids_returns_mal_and_anilist_ids() {
    let (_server, client) = mock_client(
        json!({ "id": 5114 }),
        json!({ "data": { "Media": { "id": 5114, "idMal": 5114 } } }),
    )
    .await;

    let ids = client.external_ids(5114).await.unwrap();

    assert_eq!(ids.anilist, Some(5114));
    assert_eq!(ids.mal, Some(5114));
    assert_eq!(ids.imdb, None);
    assert_eq!(ids.tmdb, None);
}

#[tokio::test]
async fn external_ids_without_mal_id() {
    let (_server, client) = mock_client(
        json!({ "id": 7 }),
        json!({ "data": { "Media": { "id": 7, "idMal": null } } }),
    )
    .await;

    let ids = client.external_ids(7).await.unwrap();

    assert_eq!(ids.anilist, Some(7));
    assert_eq!(ids.mal, None);
}

#[tokio::test]
async fn external_ids_not_found() {
    let (_server, client) = mock_client(json!({ "id": 999999 }), not_found()).await;

    let err = client.external_ids(999999).await.unwrap_err();

    assert!(matches!(err, AniListError::NotFound), "{err:?}");
}

#[tokio::test]
async fn trait_movie_and_tv_external_ids_share_one_query() {
    let (_server, client) = mock_client(
        json!({ "id": 21519 }),
        json!({ "data": { "Media": { "id": 21519, "idMal": 32281 } } }),
    )
    .await;

    let movie = client
        .movie_external_ids(&MediaId::anilist(21519))
        .await
        .unwrap();
    let tv = client
        .tv_external_ids(&MediaId::anilist(21519))
        .await
        .unwrap();

    assert_eq!(movie.mal, Some(32281));
    assert_eq!(movie, tv);
}

// ── find_by_mal_id / find_by_external_id ─────────────────────────────────────

#[tokio::test]
async fn find_by_mal_id_returns_tv_match() {
    let (_server, client) = mock_client(
        json!({ "idMal": 5114 }),
        json!({ "data": { "Media": { "id": 5114, "format": "TV" } } }),
    )
    .await;

    let found = client.find_by_mal_id(5114).await.unwrap();

    assert_eq!(found, Some(ExternalMatch::tv_show(MediaId::anilist(5114))));
}

#[tokio::test]
async fn find_by_external_mal_id_returns_movie_match() {
    let (_server, client) = mock_client(
        json!({ "idMal": 32281 }),
        json!({ "data": { "Media": { "id": 21519, "format": "MOVIE" } } }),
    )
    .await;

    let found = client
        .find_by_external_id(&ExternalId::mal(32281))
        .await
        .unwrap();

    assert_eq!(found, vec![ExternalMatch::movie(MediaId::anilist(21519))]);
}

#[tokio::test]
async fn find_by_unknown_mal_id_is_empty() {
    let (_server, client) = mock_client(json!({ "idMal": 1 }), not_found()).await;

    assert_eq!(client.find_by_mal_id(1).await.unwrap(), None);
    assert!(
        client
            .find_by_external_id(&ExternalId::mal(1))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn find_by_non_mal_source_is_unsupported() {
    let (server, client) = mock_client(json!({}), not_found()).await;

    let err = client
        .find_by_external_id(&ExternalId::imdb("tt0137523"))
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::Unsupported), "{err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn find_by_non_numeric_mal_id_is_invalid_input() {
    let (_server, client) = mock_client(json!({}), not_found()).await;

    let err = client
        .find_by_external_id(&ExternalId::new(cameo::ExternalSource::Mal, "abc"))
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::InvalidInput(_)), "{err:?}");
}
