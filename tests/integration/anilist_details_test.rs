use cameo::{
    providers::anilist::{AniListClient, AniListConfig, AniListError},
    unified::genre::Genre,
};
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

async fn mock_client_str(body: &'static str) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();
    (server, client)
}

// ── movie_details ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn movie_details_returns_correct_fields() {
    let (_server, client) = mock_client_str(include_str!(
        "../fixtures/anilist_media_details_response.json"
    ))
    .await;

    let result = client.movie_details(1535).await.unwrap();

    // Provider ID and title
    assert_eq!(result.movie.provider_id, "anilist:1535");
    assert_eq!(result.movie.title, "Your Name.");
    // Overview / description
    assert!(
        result
            .movie
            .overview
            .as_deref()
            .unwrap_or("")
            .contains("strangers")
    );
    // Release date
    assert_eq!(result.movie.release_date.as_deref(), Some("2016-08-26"));
    // Poster URL from coverImage.extraLarge
    assert!(result.movie.poster_url.is_some());
    // vote_average (86 / 10)
    assert_eq!(result.movie.vote_average, Some(8.6));
    // Runtime
    assert_eq!(result.runtime, Some(106));
    // Production companies
    assert_eq!(result.production_companies, vec!["CoMix Wave Films"]);
    // Adult flag
    assert!(!result.movie.adult);
    // Original language from countryOfOrigin "JP"
    assert_eq!(result.movie.original_language.as_deref(), Some("ja"));
    // Genres: Drama, Romance, Supernatural
    assert!(result.movie.genres.contains(&Genre::Drama));
    assert!(result.movie.genres.contains(&Genre::Romance));
    assert!(result.movie.genres.contains(&Genre::Supernatural));
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

    assert!(matches!(err, AniListError::NotFound));
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

    assert!(matches!(err, AniListError::NotFound));
}

// ── person_details ────────────────────────────────────────────────────────────

#[tokio::test]
async fn person_details_returns_correct_fields() {
    let (_server, client) = mock_client_str(include_str!(
        "../fixtures/anilist_staff_details_response.json"
    ))
    .await;

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

    assert!(matches!(err, AniListError::NotFound));
}

// ── Live tests (require real AniList network access) ──────────────────────────
//
// Kept to one test to stay well within AniList's rate limit.

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_details_smoke() {
    let c = AniListClient::new(AniListConfig::new()).unwrap();
    let delay = tokio::time::Duration::from_millis(750);

    // movie_details: AniList ID 1535 = "Your Name."
    let movie = c.movie_details(1535).await.unwrap();
    assert_eq!(movie.movie.provider_id, "anilist:1535");
    assert!(!movie.movie.title.is_empty());
    assert!(movie.movie.vote_average.is_some());

    tokio::time::sleep(delay).await;

    // tv_show_details: AniList ID 16498 = Attack on Titan
    let tv = c.tv_show_details(16498).await.unwrap();
    assert_eq!(tv.show.provider_id, "anilist:16498");
    assert!(!tv.show.name.is_empty());

    tokio::time::sleep(delay).await;

    // person_details: AniList staff ID 95061 = Yuki Kaji
    let person = c.person_details(95061).await.unwrap();
    assert_eq!(person.person.provider_id, "anilist:staff:95061");
    assert!(!person.person.name.is_empty());
}
