use cameo::{
    core::config::TimeWindow,
    providers::anilist::{AniListClient, AniListConfig, AniListError},
    unified::models::UnifiedSearchResult,
};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

fn mock_media_item(id: i32, title: &str, format: &str) -> Value {
    json!({
        "id": id,
        "title": { "romaji": title, "english": title, "native": title },
        "description": "A test anime.",
        "startDate": { "year": 2020, "month": 1, "day": 1 },
        "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
        "bannerImage": null,
        "genres": ["Action", "Drama"],
        "popularity": 50000,
        "averageScore": 85,
        "episodes": 12,
        "duration": 24,
        "status": "FINISHED",
        "format": format,
        "countryOfOrigin": "JP",
        "isAdult": false
    })
}

fn mock_page_response(media: Vec<Value>, total: i32) -> Value {
    json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": total,
                    "currentPage": 1,
                    "lastPage": (total + 19) / 20,
                    "hasNextPage": total > 20,
                    "perPage": 20
                },
                "media": media
            }
        }
    })
}

fn mock_staff_item(id: i32, name: &str) -> Value {
    json!({
        "id": id,
        "name": { "full": name, "native": name },
        "image": { "large": "https://example.com/profile.jpg" },
        "description": "A voice actor.",
        "primaryOccupations": ["Voice Actor"],
        "languageV2": "Japanese"
    })
}

async fn setup_server_with_response(body: Value) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let config = AniListConfig::new_with_base_url(server.uri());
    let client = AniListClient::new(config).unwrap();
    (server, client)
}

#[tokio::test]
async fn test_search_movies_returns_unified_movies() {
    let media = vec![mock_media_item(1535, "Your Name", "MOVIE")];
    let body = mock_page_response(media, 1);
    let (_server, client) = setup_server_with_response(body).await;

    let result = client.search_movies("Your Name", None).await.unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Your Name");
    assert_eq!(result.results[0].provider_id, "anilist:1535");
    assert_eq!(result.results[0].vote_average, Some(8.5));
    assert!(!result.results[0].adult);
    assert_eq!(result.page, 1);
}

#[tokio::test]
async fn test_search_tv_shows_returns_unified_tv() {
    let media = vec![mock_media_item(11757, "Sword Art Online", "TV")];
    let body = mock_page_response(media, 1);
    let (_server, client) = setup_server_with_response(body).await;

    let result = client
        .search_tv_shows("Sword Art Online", None)
        .await
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Sword Art Online");
    assert_eq!(result.results[0].provider_id, "anilist:11757");
}

#[tokio::test]
async fn test_search_people_returns_unified_persons() {
    let body = json!({
        "data": {
            "Page": {
                "pageInfo": { "total": 1, "currentPage": 1, "lastPage": 1, "hasNextPage": false, "perPage": 20 },
                "staff": [mock_staff_item(95061, "Yuki Kaji")]
            }
        }
    });
    let (_server, client) = setup_server_with_response(body).await;

    let result = client.search_people("Yuki Kaji", None).await.unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Yuki Kaji");
    assert_eq!(result.results[0].provider_id, "anilist:staff:95061");
    assert_eq!(
        result.results[0].known_for_department.as_deref(),
        Some("Voice Acting")
    );
}

#[tokio::test]
async fn test_search_multi_returns_movies_and_tv() {
    let media = vec![
        mock_media_item(1, "Movie A", "MOVIE"),
        mock_media_item(2, "Series B", "TV"),
    ];
    let body = mock_page_response(media, 2);
    let (_server, client) = setup_server_with_response(body).await;

    let result = client.search_multi("test", None).await.unwrap();
    assert_eq!(result.results.len(), 2);
    assert!(matches!(result.results[0], UnifiedSearchResult::Movie(_)));
    assert!(matches!(result.results[1], UnifiedSearchResult::TvShow(_)));
}

#[tokio::test]
async fn test_movie_details_returns_details() {
    let body = json!({
        "data": {
            "Media": {
                "id": 1575,
                "title": { "romaji": "Spirited Away", "english": "Spirited Away", "native": "千と千尋の神隠し" },
                "description": "A girl in a spirit world.",
                "startDate": { "year": 2001, "month": 7, "day": 20 },
                "endDate": { "year": 2001, "month": 7, "day": 20 },
                "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
                "bannerImage": null,
                "genres": ["Adventure", "Fantasy"],
                "popularity": 450000,
                "averageScore": 92,
                "episodes": 1,
                "duration": 125,
                "status": "FINISHED",
                "format": "MOVIE",
                "countryOfOrigin": "JP",
                "isAdult": false,
                "season": null,
                "seasonYear": null,
                "studios": { "nodes": [{ "name": "Studio Ghibli" }] }
            }
        }
    });
    let (_server, client) = setup_server_with_response(body).await;

    let result = client.movie_details(1575).await.unwrap();
    assert_eq!(result.movie.title, "Spirited Away");
    assert_eq!(result.movie.provider_id, "anilist:1575");
    assert_eq!(result.production_companies, vec!["Studio Ghibli"]);
    assert_eq!(result.runtime, Some(125));
}

#[tokio::test]
async fn test_person_details_returns_staff_details() {
    let body = json!({
        "data": {
            "Staff": {
                "id": 95061,
                "name": { "full": "Yuki Kaji", "native": "梶裕貴", "alternative": [] },
                "image": { "large": "https://example.com/profile.jpg" },
                "description": "A voice actor.",
                "primaryOccupations": ["Voice Actor"],
                "gender": "Male",
                "dateOfBirth": { "year": 1986, "month": 9, "day": 3 },
                "dateOfDeath": null,
                "homeTown": "Tokyo, Japan",
                "siteUrl": "https://anilist.co/staff/95061",
                "languageV2": "Japanese"
            }
        }
    });
    let (_server, client) = setup_server_with_response(body).await;

    let result = client.person_details(95061).await.unwrap();
    assert_eq!(result.person.name, "Yuki Kaji");
    assert_eq!(result.person.provider_id, "anilist:staff:95061");
    assert_eq!(result.birthday.as_deref(), Some("1986-09-03"));
}

#[tokio::test]
async fn test_graphql_errors_propagate() {
    let body = json!({
        "errors": [{ "message": "Not Found." }]
    });
    let (_server, client) = setup_server_with_response(body).await;

    let err = client.search_movies("anything", None).await.unwrap_err();
    assert!(matches!(err, AniListError::GraphQL(_)));
}

#[tokio::test]
async fn test_trending_movies_ignores_time_window() {
    let media = vec![mock_media_item(1, "Trending Movie", "MOVIE")];
    let body = mock_page_response(media, 1);
    let (_server, client) = setup_server_with_response(body).await;

    // Both time windows should work — AniList ignores the window.
    let day = client.trending_movies(TimeWindow::Day, None).await.unwrap();
    assert_eq!(day.results.len(), 1);
}
