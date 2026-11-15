use cameo::{
    MediaId,
    core::error::ProviderError,
    providers::anilist::{AniListClient, AniListConfig, AniListError},
    unified::RecommendationProvider,
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, body_string_contains, method},
};

fn media_item(id: i32, title: &str, format: Option<&str>) -> Value {
    json!({
        "id": id,
        "title": { "romaji": title, "english": title, "native": title },
        "description": "Recommended anime.",
        "startDate": { "year": 2016, "month": 8, "day": 26 },
        "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
        "bannerImage": null,
        "genres": ["Romance", "Drama"],
        "popularity": 400000,
        "averageScore": 85,
        "format": format,
        "countryOfOrigin": "JP",
        "isAdult": false
    })
}

fn node(media: Option<Value>) -> Value {
    json!({ "mediaRecommendation": media })
}

fn recommendations_response(
    nodes: Vec<Value>,
    current: i32,
    last: i32,
    total: i32,
    next: bool,
) -> Value {
    json!({
        "data": {
            "Media": {
                "recommendations": {
                    "pageInfo": {
                        "total": total,
                        "currentPage": current,
                        "lastPage": last,
                        "hasNextPage": next
                    },
                    "nodes": nodes
                }
            }
        }
    })
}

async fn mock_client(body: Value) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();
    (server, client)
}

#[tokio::test]
async fn movie_recommendations_sends_id_and_page_and_keeps_movies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("query MediaRecommendations"))
        .and(body_string_contains("RATING_DESC"))
        .and(body_partial_json(
            json!({ "variables": { "id": 21519, "page": 2 } }),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(recommendations_response(
                vec![
                    node(Some(media_item(1, "Weathering With You", Some("MOVIE")))),
                    node(Some(media_item(2, "Toradora!", Some("TV")))),
                ],
                2,
                4,
                70,
                true,
            )),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();

    let result = client.movie_recommendations(21519, Some(2)).await.unwrap();

    // Non-movie recommendations are filtered out; pagination metadata is
    // AniList's (unfiltered) metadata.
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Weathering With You");
    assert_eq!(result.results[0].provider_id.to_string(), "anilist:1");
    assert_eq!(result.page, 2);
    assert_eq!(result.total_pages, 4);
    assert_eq!(result.total_results, 70);
    assert!(result.has_next_page());
}

#[tokio::test]
async fn tv_recommendations_keeps_series_formats() {
    let body = recommendations_response(
        vec![
            node(Some(media_item(1, "A Film", Some("MOVIE")))),
            node(Some(media_item(2, "A Series", Some("TV")))),
            node(Some(media_item(3, "An OVA", Some("OVA")))),
            node(Some(media_item(4, "Unknown Format", None))),
        ],
        1,
        1,
        4,
        false,
    );
    let (_server, client) = mock_client(body).await;

    let result = client.tv_recommendations(1, None).await.unwrap();

    let names: Vec<&str> = result.results.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["A Series", "An OVA"]);
    assert!(!result.has_next_page());
}

#[tokio::test]
async fn null_recommendation_nodes_are_skipped() {
    let body = recommendations_response(
        vec![
            Value::Null,
            node(None),
            node(Some(media_item(5, "Kept", Some("TV")))),
        ],
        1,
        1,
        3,
        false,
    );
    let (_server, client) = mock_client(body).await;

    let result = client.tv_recommendations(1, None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Kept");
}

#[tokio::test]
async fn recommendations_missing_media_is_not_found() {
    // AniList reports an unknown id as a GraphQL error with status 404.
    let body = json!({
        "data": { "Media": null },
        "errors": [{ "message": "Not Found.", "status": 404 }]
    });
    let (_server, client) = mock_client(body).await;

    let err = client
        .movie_recommendations(999_999_999, None)
        .await
        .unwrap_err();

    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn recommendations_null_media_without_errors_is_not_found() {
    let (_server, client) = mock_client(json!({ "data": { "Media": null } })).await;

    let err = client.tv_recommendations(1, None).await.unwrap_err();

    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn trait_recommendations_route_through_provider() {
    let body = recommendations_response(
        vec![node(Some(media_item(9, "Trait Movie", Some("MOVIE"))))],
        1,
        1,
        1,
        false,
    );
    let (_server, client) = mock_client(body).await;

    let result = RecommendationProvider::movie_recommendations(&client, &MediaId::anilist(1), None)
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Trait Movie");
}

#[tokio::test]
async fn similar_titles_are_unsupported() {
    let (server, client) = mock_client(json!({})).await;

    let movies = RecommendationProvider::similar_movies(&client, &MediaId::anilist(1), None).await;
    let shows = RecommendationProvider::similar_tv_shows(&client, &MediaId::anilist(1), None).await;

    assert!(
        matches!(movies, Err(ProviderError::Unsupported)),
        "got {movies:?}"
    );
    assert!(
        matches!(shows, Err(ProviderError::Unsupported)),
        "got {shows:?}"
    );
    // No request is sent for an unsupported operation.
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
    );
}

#[tokio::test]
async fn trait_recommendations_reject_foreign_ids() {
    let (_server, client) = mock_client(json!({})).await;

    let result = RecommendationProvider::tv_recommendations(&client, &MediaId::tmdb(1), None).await;

    assert!(
        matches!(result, Err(ProviderError::InvalidInput(_))),
        "got {result:?}"
    );
}
