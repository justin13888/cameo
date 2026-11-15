use cameo::{
    CameoClient, CameoClientError, CreditsProvider, MediaId, ProviderError,
    providers::anilist::{AniListClient, AniListConfig, AniListError},
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, method},
};

fn staff(id: i32, name: &str) -> Value {
    json!({ "id": id, "name": { "full": name }, "image": { "large": format!("https://img/{id}.jpg") } })
}

fn credits_page(
    characters: Vec<Value>,
    more_characters: bool,
    staff: Vec<Value>,
    more_staff: bool,
) -> Value {
    json!({
        "data": {
            "Media": {
                "id": 5114,
                "characters": { "pageInfo": { "hasNextPage": more_characters }, "edges": characters },
                "staff": { "pageInfo": { "hasNextPage": more_staff }, "edges": staff },
            }
        }
    })
}

async fn mount_page(server: &MockServer, page: u32, body: Value) {
    Mock::given(method("POST"))
        .and(body_partial_json(
            json!({ "variables": { "id": 5114, "characterPage": page, "staffPage": page } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

fn client(server: &MockServer) -> AniListClient {
    AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap()
}

#[tokio::test]
async fn media_credits_maps_characters_and_staff() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        1,
        credits_page(
            vec![json!({
                "node": { "name": { "full": "Edward Elric" } },
                "voiceActors": [staff(95061, "Romi Park")],
            })],
            false,
            vec![json!({ "role": "Director", "node": staff(100, "Yasuhiro Irie") })],
            false,
        ),
    )
    .await;

    let credits = client(&server).media_credits(5114).await.unwrap();

    assert_eq!(credits.provider_id, MediaId::anilist(5114));
    assert_eq!(credits.cast.len(), 1);
    assert_eq!(credits.cast[0].person_id, MediaId::anilist_staff(95061));
    assert_eq!(credits.cast[0].name, "Romi Park");
    assert_eq!(credits.cast[0].character.as_deref(), Some("Edward Elric"));
    assert_eq!(credits.cast[0].order, Some(0));
    assert_eq!(
        credits.cast[0].profile_url.as_deref(),
        Some("https://img/95061.jpg")
    );
    assert_eq!(credits.crew.len(), 1);
    assert_eq!(credits.crew[0].person_id, MediaId::anilist_staff(100));
    assert_eq!(credits.crew[0].job.as_deref(), Some("Director"));
}

#[tokio::test]
async fn media_credits_walks_every_page_of_each_connection() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        1,
        credits_page(
            vec![json!({ "node": { "name": { "full": "A" } }, "voiceActors": [staff(1, "Actor A")] })],
            true,
            vec![json!({ "role": "Director", "node": staff(10, "Crew 1") })],
            false,
        ),
    )
    .await;
    // Staff is exhausted after page 1: page 2's staff edges must be ignored.
    mount_page(
        &server,
        2,
        credits_page(
            vec![json!({ "node": { "name": { "full": "B" } }, "voiceActors": [staff(2, "Actor B")] })],
            false,
            vec![json!({ "role": "Ignored", "node": staff(99, "Ignored") })],
            false,
        ),
    )
    .await;

    let credits = client(&server).media_credits(5114).await.unwrap();

    let cast: Vec<_> = credits
        .cast
        .iter()
        .map(|c| (c.name.as_str(), c.order))
        .collect();
    assert_eq!(cast, vec![("Actor A", Some(0)), ("Actor B", Some(1))]);
    let crew: Vec<_> = credits.crew.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(crew, vec!["Crew 1"]);
}

#[tokio::test]
async fn media_credits_not_found() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        1,
        json!({
            "data": { "Media": null },
            "errors": [{ "message": "Not Found.", "status": 404 }]
        }),
    )
    .await;

    let err = client(&server).media_credits(5114).await.unwrap_err();

    assert!(matches!(err, AniListError::NotFound), "{err:?}");
}

#[tokio::test]
async fn media_credits_negative_id_is_not_found_without_a_request() {
    let server = MockServer::start().await;

    let err = client(&server).media_credits(-1).await.unwrap_err();

    assert!(matches!(err, AniListError::NotFound), "{err:?}");
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
    );
}

#[tokio::test]
async fn trait_movie_and_tv_credits_share_one_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "id": 5114 } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(credits_page(
            vec![],
            false,
            vec![json!({ "role": "Director", "node": staff(100, "Yasuhiro Irie") })],
            false,
        )))
        .expect(2)
        .mount(&server)
        .await;
    let client = client(&server);

    let movie = CreditsProvider::movie_credits(&client, &MediaId::anilist(5114))
        .await
        .unwrap();
    let tv = CreditsProvider::tv_show_credits(&client, &MediaId::anilist(5114))
        .await
        .unwrap();

    assert_eq!(movie, tv);
    assert_eq!(movie.crew[0].person_id, MediaId::anilist_staff(100));
}

#[tokio::test]
async fn trait_credits_rejects_a_tmdb_id() {
    let server = MockServer::start().await;

    let err = CreditsProvider::movie_credits(&client(&server), &MediaId::tmdb(550))
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::InvalidInput(_)), "{err:?}");
}

#[tokio::test]
async fn facade_routes_anilist_credits_to_anilist() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        1,
        credits_page(
            vec![json!({
                "node": { "name": { "full": "Edward Elric" } },
                "voiceActors": [staff(95061, "Romi Park")],
            })],
            false,
            vec![],
            false,
        ),
    )
    .await;
    let facade = CameoClient::builder()
        .with_anilist(AniListConfig::new_with_base_url(server.uri()))
        .build()
        .unwrap();

    let credits = facade
        .tv_show_credits(&MediaId::anilist(5114))
        .await
        .unwrap();
    assert_eq!(credits.cast[0].person_id, MediaId::anilist_staff(95061));

    let tmdb = facade.movie_credits(&MediaId::tmdb(550)).await;
    assert!(
        matches!(tmdb, Err(CameoClientError::Unsupported)),
        "expected Unsupported, got: {tmdb:?}"
    );
}
