#![cfg(feature = "tmdb")]

use cameo::{
    MediaId,
    core::config::TimeWindow,
    providers::tmdb::TmdbConfig,
    unified::{CameoClient, models::UnifiedSearchResult},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn setup(server: &MockServer) -> CameoClient {
    let config = TmdbConfig::new_with_base_url("test-token", server.uri());
    CameoClient::builder().with_tmdb(config).build().unwrap()
}

// ── TMDB dispatch ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn facade_trending_movies_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/trending/movie/week"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/trending_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let result = client
        .trending_movies(TimeWindow::Week, None)
        .await
        .unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Fight Club");
    assert_eq!(result.results[0].provider_id, "tmdb:550");
}

#[tokio::test]
async fn facade_popular_movies_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/popular"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/popular_movies_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let result = client.popular_movies(None).await.unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].title, "Interstellar");
    assert_eq!(result.results[0].provider_id, "tmdb:157336");
}

#[tokio::test]
async fn facade_search_multi_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/search/multi"))
        .and(query_param("query", "fight club"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/search_multi_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let result = client.search_multi("fight club", None).await.unwrap();

    assert_eq!(result.total_results, 3);
    assert_eq!(result.results.len(), 3);

    let movies: Vec<_> = result
        .results
        .iter()
        .filter(|r| matches!(r, UnifiedSearchResult::Movie(_)))
        .collect();
    let tv_shows: Vec<_> = result
        .results
        .iter()
        .filter(|r| matches!(r, UnifiedSearchResult::TvShow(_)))
        .collect();
    let people: Vec<_> = result
        .results
        .iter()
        .filter(|r| matches!(r, UnifiedSearchResult::Person(_)))
        .collect();

    assert_eq!(movies.len(), 1, "expected 1 movie result");
    assert_eq!(tv_shows.len(), 1, "expected 1 tv result");
    assert_eq!(people.len(), 1, "expected 1 person result");
}

#[tokio::test]
async fn facade_season_details_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1399/season/1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_season_details_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let season = client
        .season_details(&MediaId::tmdb(1399), 1)
        .await
        .unwrap();

    assert_eq!(season.season_number, 1);
    assert_eq!(season.name.as_deref(), Some("Season 1"));
    assert_eq!(season.episodes.len(), 1);
    assert_eq!(season.show_id, "tmdb:1399");
}

#[tokio::test]
async fn facade_movie_watch_providers_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/movie/550/watch/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/movie_watch_providers_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let providers = client
        .movie_watch_providers(&MediaId::tmdb(550))
        .await
        .unwrap();

    assert_eq!(providers.provider_id, "tmdb:550");
    assert!(providers.results.contains_key("US"));
    let us = &providers.results["US"];
    assert!(!us.flatrate.is_empty());
    assert_eq!(us.flatrate[0].name, "fuboTV");
}

#[tokio::test]
async fn facade_tv_external_ids_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/external_ids"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/tv_external_ids_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let ids = client.tv_external_ids(&MediaId::tmdb(1396)).await.unwrap();

    assert_eq!(ids.tmdb, Some(1396));
    assert_eq!(ids.tvdb, Some(81189));
}

#[tokio::test]
async fn facade_find_by_external_id_dispatches_to_tmdb() {
    let server = MockServer::start().await;
    let client = setup(&server).await;

    Mock::given(method("GET"))
        .and(path("/3/find/tt0137523"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../fixtures/find_by_imdb_id_response.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let matches = client
        .find_by_external_id(&cameo::ExternalId::imdb("tt0137523"))
        .await
        .unwrap();

    assert_eq!(
        matches,
        vec![cameo::ExternalMatch::movie(MediaId::tmdb(550))]
    );
}

// ── AniList fallback ──────────────────────────────────────────────────────────

#[cfg(feature = "anilist")]
mod anilist_tests {
    use cameo::{
        providers::anilist::AniListConfig,
        unified::{CameoClient, CameoClientError},
    };
    use serde_json::{Value, json};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    fn media_item(id: i32, title: &str, format: &str) -> Value {
        json!({
            "id": id,
            "title": { "romaji": title, "english": title, "native": title },
            "description": "Test anime.",
            "startDate": { "year": 2020, "month": 1, "day": 1 },
            "coverImage": {
                "large": "https://example.com/cover.jpg",
                "extraLarge": "https://example.com/cover_xl.jpg"
            },
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

    async fn setup_anilist_only(body: Value) -> (MockServer, CameoClient) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        let config = AniListConfig::new_with_base_url(server.uri());
        let client = CameoClient::builder().with_anilist(config).build().unwrap();
        (server, client)
    }

    #[tokio::test]
    async fn facade_anilist_fallback_search_movies() {
        let body = page_response(vec![media_item(1, "Your Name", "MOVIE")], 1);
        let (_server, client) = setup_anilist_only(body).await;

        let result = client.search_movies("Your Name", None).await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].title, "Your Name");
        assert!(result.results[0].provider_id.provider() == "anilist");
    }

    #[tokio::test]
    async fn facade_anilist_only_recommendations_returns_unsupported() {
        let body = page_response(vec![], 0);
        let (_server, client) = setup_anilist_only(body).await;

        let result = client
            .movie_recommendations(&cameo::MediaId::tmdb(1), None)
            .await;

        assert!(
            matches!(result, Err(CameoClientError::Unsupported)),
            "expected Unsupported, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn facade_anilist_only_season_details_returns_unsupported() {
        let body = page_response(vec![], 0);
        let (_server, client) = setup_anilist_only(body).await;

        //  is only implemented for TMDB
        let result = client.season_details(&cameo::MediaId::tmdb(1), 1).await;

        assert!(
            matches!(result, Err(CameoClientError::Unsupported)),
            "expected Unsupported, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn facade_anilist_only_find_by_imdb_id_returns_unsupported() {
        let body = page_response(vec![], 0);
        let (_server, client) = setup_anilist_only(body).await;
        let result = client
            .find_by_external_id(&cameo::ExternalId::imdb("tt0137523"))
            .await;

        assert!(
            matches!(result, Err(CameoClientError::Unsupported)),
            "expected Unsupported, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn facade_anilist_find_by_mal_id() {
        let body = json!({ "data": { "Media": { "id": 5114, "format": "TV" } } });
        let (_server, client) = setup_anilist_only(body).await;
        let result = client
            .find_by_external_id(&cameo::ExternalId::mal(5114))
            .await
            .unwrap();

        assert_eq!(
            result,
            vec![cameo::ExternalMatch::tv_show(cameo::MediaId::anilist(5114))]
        );
    }

    #[tokio::test]
    async fn facade_anilist_only_watch_providers_returns_unsupported() {
        let body = page_response(vec![], 0);
        let (_server, client) = setup_anilist_only(body).await;
        let result = client.movie_watch_providers(&cameo::MediaId::tmdb(1)).await;

        assert!(
            matches!(result, Err(CameoClientError::Unsupported)),
            "expected Unsupported, got: {result:?}"
        );
    }
}
