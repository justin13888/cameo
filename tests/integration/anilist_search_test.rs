use cameo::{
    providers::anilist::{AniListClient, AniListConfig, AniListError},
    unified::models::UnifiedSearchResult,
};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn media_item(id: i32, title: &str, format: &str) -> Value {
    json!({
        "id": id,
        "title": { "romaji": title, "english": title, "native": title },
        "description": "A test anime.",
        "startDate": { "year": 2016, "month": 4, "day": 1 },
        "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
        "bannerImage": null,
        "genres": ["Action", "Drama"],
        "popularity": 100000,
        "averageScore": 80,
        "episodes": 24,
        "duration": 24,
        "status": "FINISHED",
        "format": format,
        "countryOfOrigin": "JP",
        "isAdult": false
    })
}

fn page_response(media: Vec<Value>, total: i32) -> Value {
    json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": total,
                    "currentPage": 1,
                    "lastPage": std::cmp::max(1, (total + 19) / 20),
                    "hasNextPage": total > 20,
                    "perPage": 20
                },
                "media": media
            }
        }
    })
}

fn staff_page_response(staff: Vec<Value>, total: i32) -> Value {
    json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": total,
                    "currentPage": 1,
                    "lastPage": 1,
                    "hasNextPage": false,
                    "perPage": 20
                },
                "staff": staff
            }
        }
    })
}

fn staff_item(id: i32, name: &str) -> Value {
    json!({
        "id": id,
        "name": { "full": name, "native": name },
        "image": { "large": "https://example.com/profile.jpg" },
        "description": "A voice actor.",
        "primaryOccupations": ["Voice Actor"],
        "languageV2": "Japanese"
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

// ── search_movies ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_movies_returns_movie_results() {
    let body = page_response(vec![media_item(1535, "Your Name", "MOVIE")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.search_movies("Your Name", None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].provider_id, "anilist:1535");
    assert_eq!(result.results[0].title, "Your Name");
    assert_eq!(result.page, 1);
    assert_eq!(result.total_results, 1);
}

#[tokio::test]
async fn search_movies_empty_results() {
    let body = page_response(vec![], 0);
    let (_server, client) = mock_client(body).await;

    let result = client.search_movies("nonexistent", None).await.unwrap();

    assert_eq!(result.results.len(), 0);
    assert_eq!(result.total_results, 0);
}

#[tokio::test]
async fn search_movies_maps_score_correctly() {
    let mut item = media_item(1, "Test", "MOVIE");
    item["averageScore"] = json!(92);
    let body = page_response(vec![item], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.search_movies("Test", None).await.unwrap();

    assert_eq!(result.results[0].vote_average, Some(9.2));
}

// ── search_tv_shows ───────────────────────────────────────────────────────────

#[tokio::test]
async fn search_tv_shows_returns_tv_results() {
    let body = page_response(vec![media_item(11757, "Sword Art Online", "TV")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client
        .search_tv_shows("Sword Art Online", None)
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].provider_id, "anilist:11757");
    assert_eq!(result.results[0].name, "Sword Art Online");
}

#[tokio::test]
async fn search_tv_shows_ova_format() {
    let body = page_response(vec![media_item(42, "Test OVA", "OVA")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.search_tv_shows("Test OVA", None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Test OVA");
}

// ── search_people ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_people_returns_staff_results() {
    let body = staff_page_response(vec![staff_item(95061, "Yuki Kaji")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.search_people("Yuki Kaji", None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].provider_id, "anilist:staff:95061");
    assert_eq!(result.results[0].name, "Yuki Kaji");
    assert_eq!(
        result.results[0].known_for_department.as_deref(),
        Some("Voice Acting")
    );
}

#[tokio::test]
async fn search_people_empty_results() {
    let body = staff_page_response(vec![], 0);
    let (_server, client) = mock_client(body).await;

    let result = client.search_people("nobody", None).await.unwrap();

    assert_eq!(result.results.len(), 0);
}

// ── search_multi ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_multi_returns_mixed_types() {
    let body = page_response(
        vec![
            media_item(1, "Movie A", "MOVIE"),
            media_item(2, "Series B", "TV"),
        ],
        2,
    );
    let (_server, client) = mock_client(body).await;

    let result = client.search_multi("test", None).await.unwrap();

    assert_eq!(result.results.len(), 2);
    assert!(matches!(result.results[0], UnifiedSearchResult::Movie(_)));
    assert!(matches!(result.results[1], UnifiedSearchResult::TvShow(_)));
}

#[tokio::test]
async fn search_multi_graphql_error_propagates() {
    let body = json!({ "errors": [{ "message": "Internal Server Error.", "status": 500 }] });
    let (_server, client) = mock_client(body).await;

    let err = client.search_multi("test", None).await.unwrap_err();

    assert!(matches!(err, AniListError::GraphQL { .. }));
}

#[tokio::test]
async fn rate_limit_error_exposes_retry_after() {
    // AniList signals rate limiting via a GraphQL error plus a Retry-After header.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "12")
                .set_body_json(json!({
                    "errors": [{ "message": "Too Many Requests.", "status": 429 }]
                })),
        )
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();

    let err = client.search_multi("test", None).await.unwrap_err();

    match err {
        AniListError::GraphQL {
            retry_after,
            errors,
        } => {
            assert_eq!(retry_after, Some(std::time::Duration::from_secs(12)));
            assert_eq!(errors[0].status, Some(429));
        }
        other => panic!("expected GraphQL error with retry_after, got {other:?}"),
    }
}

#[tokio::test]
async fn partial_data_with_errors_is_returned() {
    // A 200 carrying both data and (non-fatal) errors returns the usable data.
    let body = json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": 1, "currentPage": 1, "lastPage": 1,
                    "hasNextPage": false, "perPage": 20
                },
                "media": [ media_item(1, "Partial Anime", "TV") ]
            }
        },
        "errors": [{ "message": "Some field failed to resolve.", "status": 200 }]
    });
    let (_server, client) = mock_client(body).await;

    let result = client.search_tv_shows("test", None).await.unwrap();
    assert_eq!(result.results.len(), 1);
}

// ── Live tests (require real AniList network access) ──────────────────────────
//
// Kept to one test per domain to stay well within AniList's rate limit.

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_search_smoke() {
    let c = AniListClient::new(AniListConfig::new()).unwrap();
    let delay = tokio::time::Duration::from_millis(750);

    // search_movies
    let movies = c.search_movies("Your Name", None).await.unwrap();
    assert!(movies.total_results > 0);
    assert!(!movies.results[0].title.is_empty());
    assert!(
        movies.results[0]
            .provider_id
            .to_string()
            .starts_with("anilist:")
    );

    tokio::time::sleep(delay).await;

    // search_tv_shows
    let tv = c.search_tv_shows("Attack on Titan", None).await.unwrap();
    assert!(tv.total_results > 0);
    assert!(!tv.results[0].name.is_empty());

    tokio::time::sleep(delay).await;

    // search_people
    let people = c.search_people("Yuki Kaji", None).await.unwrap();
    assert!(people.total_results > 0);
    assert!(
        people.results[0]
            .provider_id
            .to_string()
            .starts_with("anilist:staff:")
    );

    tokio::time::sleep(delay).await;

    // search_multi
    let multi = c.search_multi("Spirited Away", None).await.unwrap();
    assert!(multi.total_results > 0);
}
