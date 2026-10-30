use cameo::providers::anilist::{
    AniListClient, AniListConfig, AnimeFormat, AnimeSeason, AnimeSort, AnimeStatus,
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{body_partial_json, body_string_contains, method},
};

fn media_item(id: i32, title: &str, format: &str) -> Value {
    json!({
        "id": id,
        "title": { "romaji": title, "english": title, "native": title },
        "description": "Test anime.",
        "startDate": { "year": 2024, "month": 4, "day": 1 },
        "coverImage": { "large": "https://example.com/cover.jpg", "extraLarge": "https://example.com/cover_xl.jpg" },
        "bannerImage": null,
        "genres": ["Action", "Fantasy"],
        "popularity": 50000,
        "averageScore": 82,
        "format": format,
        "countryOfOrigin": "JP",
        "isAdult": false
    })
}

fn page_response(media: Vec<Value>, current: i32, last: i32, total: i32, next: bool) -> Value {
    json!({
        "data": {
            "Page": {
                "pageInfo": {
                    "total": total,
                    "currentPage": current,
                    "lastPage": last,
                    "hasNextPage": next
                },
                "media": media
            }
        }
    })
}

fn client_for(server: &MockServer) -> AniListClient {
    AniListClient::new(AniListConfig::new_with_base_url(server.uri())).unwrap()
}

#[tokio::test]
async fn discover_tv_sends_every_filter_as_a_variable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("query DiscoverAnime"))
        .and(body_partial_json(json!({
            "variables": {
                "page": 2,
                "genreIn": ["Action", "Fantasy"],
                "genreNotIn": ["Ecchi"],
                "countryOfOrigin": "JP",
                "season": "SPRING",
                "seasonYear": 2024,
                "status": "FINISHED",
                "isAdult": false,
                "sort": ["SCORE_DESC"],
                "formatIn": ["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"]
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![media_item(1, "Spring Show", "TV")],
            2,
            2,
            21,
            false,
        )))
        .expect(1)
        .mount(&server)
        .await;

    let result = client_for(&server)
        .discover_anime()
        .genres(["Action", "Fantasy"])
        .exclude_genres(["Ecchi"])
        .country_of_origin("jp")
        .season(AnimeSeason::Spring, 2024)
        .status(AnimeStatus::Finished)
        .is_adult(Some(false))
        .sort(AnimeSort::ScoreDesc)
        .page(2)
        .execute_tv()
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].name, "Spring Show");
    assert_eq!(result.results[0].provider_id.to_string(), "anilist:1");
}

#[tokio::test]
async fn discover_omits_unset_filters_and_defaults_sort() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(|req: &Request| {
            let body: Value = match serde_json::from_slice(&req.body) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let Some(vars) = body["variables"].as_object() else {
                return false;
            };
            let mut keys: Vec<&str> = vars.keys().map(String::as_str).collect();
            keys.sort_unstable();
            keys == ["formatIn", "page", "perPage", "sort"]
                && vars["sort"] == json!(["POPULARITY_DESC"])
                && vars["page"] == json!(1)
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![],
            1,
            1,
            0,
            false,
        )))
        .expect(1)
        .mount(&server)
        .await;

    let result = client_for(&server)
        .discover_anime()
        .execute_tv()
        .await
        .unwrap();

    assert!(result.results.is_empty());
}

#[tokio::test]
async fn discover_maps_page_info() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![
                media_item(10, "Show A", "TV"),
                media_item(11, "Show B", "ONA"),
            ],
            3,
            5,
            90,
            true,
        )))
        .mount(&server)
        .await;

    let result = client_for(&server)
        .discover_anime()
        .page(3)
        .execute_tv()
        .await
        .unwrap();

    assert_eq!(result.page, 3);
    assert_eq!(result.total_pages, 5);
    assert_eq!(result.total_results, 90);
    assert!(result.has_next_page());
    assert_eq!(result.results.len(), 2);
}

#[tokio::test]
async fn discover_movies_defaults_to_movie_format() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            json!({ "variables": { "formatIn": ["MOVIE"] } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![media_item(20, "Anime Film", "MOVIE")],
            1,
            1,
            1,
            false,
        )))
        .expect(1)
        .mount(&server)
        .await;

    let result = client_for(&server)
        .discover_anime()
        .genres(["Drama"])
        .execute_movies()
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Anime Film");
    assert_eq!(result.results[0].vote_average, Some(8.2));
}

#[tokio::test]
async fn discover_explicit_formats_override_terminal_default() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(
            json!({ "variables": { "formatIn": ["OVA", "SPECIAL"] } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![media_item(30, "An OVA", "OVA")],
            1,
            1,
            1,
            false,
        )))
        .expect(1)
        .mount(&server)
        .await;

    // The caller's formats win even for execute_movies; entries are converted
    // to the terminal's unified type regardless of their format.
    let result = client_for(&server)
        .discover_anime()
        .formats(&[AnimeFormat::Ova, AnimeFormat::Special])
        .execute_movies()
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "An OVA");
}

#[tokio::test]
async fn discover_season_year_alone_is_sent_without_season() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(|req: &Request| {
            serde_json::from_slice::<Value>(&req.body)
                .map(|b| {
                    b["variables"]["seasonYear"] == json!(2023)
                        && b["variables"].get("season").is_none()
                })
                .unwrap_or(false)
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(page_response(
            vec![],
            1,
            1,
            0,
            false,
        )))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .discover_anime()
        .season_year(2023)
        .execute_tv()
        .await
        .unwrap();
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_discover_smoke() {
    let c = AniListClient::new(AniListConfig::new()).unwrap();
    let shows = c
        .discover_anime()
        .genres(["Action"])
        .country_of_origin("JP")
        .is_adult(Some(false))
        .execute_tv()
        .await
        .unwrap();
    assert!(!shows.results.is_empty());
}
