use cameo::{
    core::config::TimeWindow,
    providers::anilist::{AniListClient, AniListConfig},
};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

fn media_item(id: i32, title: &str, format: &str) -> Value {
    json!({
        "id": id,
        "title": { "romaji": title, "english": title, "native": title },
        "description": "Test anime.",
        "startDate": { "year": 2020, "month": 1, "day": 1 },
        "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
        "bannerImage": null,
        "genres": ["Action"],
        "popularity": 50000,
        "averageScore": 78,
        "episodes": 12,
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

async fn mock_client(body: Value) -> (MockServer, AniListClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let client = AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap();
    (server, client)
}

// ── trending_movies ───────────────────────────────────────────────────────────

#[tokio::test]
async fn trending_movies_day_returns_results() {
    let body = page_response(vec![media_item(1, "Trending Movie", "MOVIE")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.trending_movies(TimeWindow::Day, None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Trending Movie");
}

#[tokio::test]
async fn trending_movies_week_returns_results() {
    let body = page_response(vec![media_item(2, "Weekly Movie", "MOVIE")], 1);
    let (_server, client) = mock_client(body).await;

    // AniList ignores time_window; both variants should work identically
    let result = client
        .trending_movies(TimeWindow::Week, None)
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Weekly Movie");
}

// ── trending_tv ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn trending_tv_day_returns_results() {
    let body = page_response(vec![media_item(10, "Trending TV", "TV")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.trending_tv(TimeWindow::Day, None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Trending TV");
}

#[tokio::test]
async fn trending_tv_week_returns_results() {
    let body = page_response(vec![media_item(11, "Weekly TV", "TV")], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.trending_tv(TimeWindow::Week, None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Weekly TV");
}

// ── popular_movies ────────────────────────────────────────────────────────────

#[tokio::test]
async fn popular_movies_returns_results() {
    let body = page_response(
        vec![
            media_item(100, "Popular Movie A", "MOVIE"),
            media_item(101, "Popular Movie B", "MOVIE"),
        ],
        2,
    );
    let (_server, client) = mock_client(body).await;

    let result = client.popular_movies(None).await.unwrap();

    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[0].title, "Popular Movie A");
    assert_eq!(result.results[1].title, "Popular Movie B");
}

#[tokio::test]
async fn popular_movies_respects_pagination() {
    let body = page_response(vec![media_item(1, "Movie", "MOVIE")], 100);
    let (_server, client) = mock_client(body).await;

    let result = client.popular_movies(Some(2)).await.unwrap();

    // Total from the mock — pagination metadata is passed through
    assert_eq!(result.total_results, 100);
    assert_eq!(result.total_pages, 5);
}

// ── top_rated_movies ──────────────────────────────────────────────────────────

#[tokio::test]
async fn top_rated_movies_returns_results() {
    let mut item = media_item(200, "Top Rated Movie", "MOVIE");
    item["averageScore"] = json!(95);
    let body = page_response(vec![item], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.top_rated_movies(None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Top Rated Movie");
    assert_eq!(result.results[0].vote_average, Some(9.5));
}

// ── popular_tv_shows ──────────────────────────────────────────────────────────

#[tokio::test]
async fn popular_tv_shows_returns_results() {
    let body = page_response(
        vec![
            media_item(100, "Popular TV A", "TV"),
            media_item(101, "Popular TV B", "TV"),
        ],
        2,
    );
    let (_server, client) = mock_client(body).await;

    let result = client.popular_tv_shows(None).await.unwrap();

    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[0].name, "Popular TV A");
    assert_eq!(result.results[1].name, "Popular TV B");
}

// ── top_rated_tv_shows ────────────────────────────────────────────────────────

#[tokio::test]
async fn top_rated_tv_shows_returns_results() {
    let mut item = media_item(200, "Top Rated TV", "TV");
    item["averageScore"] = json!(90);
    let body = page_response(vec![item], 1);
    let (_server, client) = mock_client(body).await;

    let result = client.top_rated_tv_shows(None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Top Rated TV");
    assert_eq!(result.results[0].vote_average, Some(9.0));
}

// ── Live tests (require real AniList network access) ──────────────────────────
//
// Kept to one test to stay well within AniList's rate limit.

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_discovery_smoke() {
    let c = AniListClient::new(AniListConfig::new()).unwrap();

    // trending_tv — exercises trending path
    let trending = c.trending_tv(TimeWindow::Week, None).await.unwrap();
    assert!(!trending.results.is_empty());

    // top_rated_movies — exercises score-sorted path and confirms vote_average is set
    let top = c.top_rated_movies(None).await.unwrap();
    assert!(!top.results.is_empty());
    assert!(top.results[0].vote_average.is_some());
}
