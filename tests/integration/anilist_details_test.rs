use cameo::providers::anilist::{AniListClient, AniListConfig, AniListError};
use serde_json::json;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

async fn mock_client(body: serde_json::Value) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();
    (server, client)
}

// ── movie_details ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn movie_details_returns_correct_fields() {
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
    let (_server, client) = mock_client(body).await;

    let result = client.movie_details(1575).await.unwrap();

    assert_eq!(result.movie.provider_id, "anilist:1575");
    assert_eq!(result.movie.title, "Spirited Away");
    assert_eq!(result.runtime, Some(125));
    assert_eq!(result.production_companies, vec!["Studio Ghibli"]);
    assert!(!result.movie.adult);
}

#[tokio::test]
async fn movie_details_score_converted_to_ten_scale() {
    let body = json!({
        "data": {
            "Media": {
                "id": 1,
                "title": { "romaji": "Test", "english": "Test", "native": "Test" },
                "description": null,
                "startDate": { "year": 2020, "month": 1, "day": 1 },
                "endDate": null,
                "coverImage": { "large": null, "extraLarge": null },
                "bannerImage": null,
                "genres": [],
                "popularity": 0,
                "averageScore": 85,
                "episodes": 1,
                "duration": 90,
                "status": "FINISHED",
                "format": "MOVIE",
                "countryOfOrigin": "JP",
                "isAdult": false,
                "season": null,
                "seasonYear": null,
                "studios": { "nodes": [] }
            }
        }
    });
    let (_server, client) = mock_client(body).await;

    let result = client.movie_details(1).await.unwrap();

    assert_eq!(result.movie.vote_average, Some(8.5));
}

#[tokio::test]
async fn movie_details_graphql_error_propagates() {
    let body = json!({ "errors": [{ "message": "Not Found." }] });
    let (_server, client) = mock_client(body).await;

    let err = client.movie_details(9999).await.unwrap_err();

    assert!(matches!(err, AniListError::GraphQL(_)));
}

// ── tv_show_details ───────────────────────────────────────────────────────────

#[tokio::test]
async fn tv_show_details_returns_correct_fields() {
    let body = json!({
        "data": {
            "Media": {
                "id": 16498,
                "title": { "romaji": "Shingeki no Kyojin", "english": "Attack on Titan", "native": "進撃の巨人" },
                "description": "Humanity fights giants.",
                "startDate": { "year": 2013, "month": 4, "day": 7 },
                "endDate": { "year": 2013, "month": 9, "day": 29 },
                "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
                "bannerImage": null,
                "genres": ["Action", "Drama"],
                "popularity": 500000,
                "averageScore": 84,
                "episodes": 25,
                "duration": 24,
                "status": "FINISHED",
                "format": "TV",
                "countryOfOrigin": "JP",
                "isAdult": false,
                "season": "SPRING",
                "seasonYear": 2013,
                "studios": { "nodes": [{ "name": "Wit Studio" }] }
            }
        }
    });
    let (_server, client) = mock_client(body).await;

    let result = client.tv_show_details(16498).await.unwrap();

    assert_eq!(result.show.provider_id, "anilist:16498");
    assert_eq!(result.show.name, "Attack on Titan");
    assert_eq!(result.show.vote_average, Some(8.4));
}

#[tokio::test]
async fn tv_show_details_graphql_error_propagates() {
    let body = json!({ "errors": [{ "message": "Not Found." }] });
    let (_server, client) = mock_client(body).await;

    let err = client.tv_show_details(9999).await.unwrap_err();

    assert!(matches!(err, AniListError::GraphQL(_)));
}

// ── person_details ────────────────────────────────────────────────────────────

#[tokio::test]
async fn person_details_returns_correct_fields() {
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
    let (_server, client) = mock_client(body).await;

    let result = client.person_details(95061).await.unwrap();

    assert_eq!(result.person.provider_id, "anilist:staff:95061");
    assert_eq!(result.person.name, "Yuki Kaji");
    assert_eq!(result.birthday.as_deref(), Some("1986-09-03"));
}

#[tokio::test]
async fn person_details_graphql_error_propagates() {
    let body = json!({ "errors": [{ "message": "Not Found." }] });
    let (_server, client) = mock_client(body).await;

    let err = client.person_details(9999).await.unwrap_err();

    assert!(matches!(err, AniListError::GraphQL(_)));
}
