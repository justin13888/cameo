//! TheTVDB v4 client against a mock server: login and token renewal, search,
//! extended records, paged default-order episodes, and error mapping.

use std::time::Duration;

use cameo::{
    RateLimit, RetryPolicy,
    core::error::ProviderError,
    providers::tvdb::{TvdbClient, TvdbConfig, TvdbError, TvdbSearchType},
    unified::{
        DetailProvider, ExternalIdProvider, Genre, MediaId, SearchProvider, SeasonProvider,
        SeriesStatus, UnifiedSearchResult,
    },
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

const SECRET: &str = "SECRETKEY123";

fn config(server: &MockServer) -> TvdbConfig {
    TvdbConfig::new(SECRET)
        .with_base_url(server.uri())
        .with_rate_limit(RateLimit::Unlimited)
        .with_retry(RetryPolicy::disabled())
}

fn ok(data: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "status": "success", "data": data }))
}

fn failure(status: u16, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(json!({
        "status": "failure", "message": message, "data": null
    }))
}

/// Mount a login answering `token`, expected `times` times.
async fn mount_login(server: &MockServer, token: &str, times: u64) {
    Mock::given(method("POST"))
        .and(path("/login"))
        .and(body_json(json!({ "apikey": SECRET })))
        .respond_with(ok(json!({ "token": token })))
        .expect(times)
        .mount(server)
        .await;
}

fn series_json() -> Value {
    json!({
        "id": 81189,
        "name": "Breaking Bad",
        "slug": "breaking-bad",
        "image": "https://artworks.thetvdb.com/banners/posters/81189-1.jpg",
        "firstAired": "2008-01-20",
        "lastAired": "2013-09-29",
        "nextAired": "",
        "score": 2_000_000.0,
        "status": { "id": 2, "name": "Ended" },
        "originalCountry": "usa",
        "originalLanguage": "eng",
        "averageRuntime": 47,
        "defaultSeasonType": 1,
        "overview": "A chemistry teacher turns to crime.",
        "year": "2008",
        "originalNetwork": { "id": 1, "name": "AMC", "country": "usa" },
        "latestNetwork": { "id": 1, "name": "AMC", "country": "usa" },
        "aliases": [{ "language": "eng", "name": "BB" }],
        "artworks": [
            { "id": 1, "image": "https://img/bg-low.jpg", "type": 3, "score": 1.0 },
            { "id": 2, "image": "https://img/bg-high.jpg", "type": 3, "score": 9.0 },
            { "id": 3, "image": "https://img/banner.jpg", "type": 1, "score": 99.0 }
        ],
        "genres": [
            { "id": 1, "name": "Drama", "slug": "drama" },
            { "id": 2, "name": "Crime", "slug": "crime" }
        ],
        "remoteIds": [
            { "id": "tt0903747", "type": 2, "sourceName": "IMDB" },
            { "id": "1396", "type": 12, "sourceName": "TheMovieDB.com" }
        ],
        "contentRatings": [
            { "id": 1, "name": "TV-MA", "country": "usa", "contentType": "", "fullname": null }
        ],
        "seasons": [
            { "id": 10, "number": 0, "type": { "id": 1, "name": "Aired Order", "type": "official" } },
            { "id": 11, "number": 1, "type": { "id": 1, "name": "Aired Order", "type": "official" },
              "image": "https://img/s1.jpg", "year": "2008" },
            { "id": 12, "number": 2, "type": { "id": 1, "name": "Aired Order", "type": "official" } },
            { "id": 13, "number": 1, "type": { "id": 2, "name": "DVD Order", "type": "dvd" } }
        ],
        "translations": {
            "nameTranslations": [
                { "language": "eng", "name": "Breaking Bad", "isPrimary": true, "tagline": "Remember my name" },
                { "language": "deu", "name": "Breaking Bad (DE)" }
            ],
            "overviewTranslations": [
                { "language": "eng", "overview": "Walter White cooks." }
            ],
            "alias": null
        }
    })
}

fn movie_json() -> Value {
    json!({
        "id": 12,
        "name": "君の名は。",
        "image": "https://img/movie-poster.jpg",
        "year": "2016",
        "score": 50.0,
        "runtime": 106,
        "status": { "id": 5, "name": "Released" },
        "originalLanguage": "jpn",
        "originalCountry": "jpn",
        "budget": "0",
        "boxOffice": "380000000",
        "first_release": { "country": "jpn", "date": "2016-08-26" },
        "companies": {
            "studio": [{ "id": 7, "name": "CoMix Wave Films", "country": "jpn" }],
            "production": [{ "id": 7, "name": "CoMix Wave Films", "country": "jpn" }],
            "network": null, "distributor": null, "special_effects": null
        },
        "production_countries": [{ "id": 1, "country": "jpn", "name": "Japan" }],
        "spoken_languages": ["jpn"],
        "artworks": [{ "id": 1, "image": "https://img/movie-bg.jpg", "type": 15, "score": 3.0 }],
        "genres": [{ "id": 3, "name": "Animation" }, { "id": 4, "name": "Romance" }],
        "remoteIds": [{ "id": "tt5311514", "type": 2, "sourceName": "IMDB" }],
        "contentRatings": null,
        "translations": {
            "nameTranslations": [
                { "language": "eng", "name": "Your Name." },
                { "language": "jpn", "name": "君の名は。" }
            ],
            "overviewTranslations": [{ "language": "eng", "overview": "Two strangers swap bodies." }]
        }
    })
}

fn episode(id: u64, season: u32, number: u32, absolute: u32) -> Value {
    json!({
        "id": id, "seriesId": 81189, "seasonNumber": season, "number": number,
        "absoluteNumber": absolute, "name": format!("S{season}E{number}"),
        "aired": format!("2008-0{}-1{}", season.max(1), number), "runtime": 47,
        "isMovie": 0
    })
}

// ── Login and token renewal ───────────────────────────────────────────────────

#[tokio::test]
async fn logs_in_once_and_reuses_the_bearer_token() {
    let server = MockServer::start().await;
    mount_login(&server, "T1", 1).await;
    Mock::given(method("GET"))
        .and(path("/series/81189/extended"))
        .and(query_param("meta", "translations"))
        .and(query_param("short", "true"))
        .and(header("authorization", "Bearer T1"))
        .respond_with(ok(series_json()))
        .expect(2)
        .mount(&server)
        .await;

    let client = TvdbClient::new(config(&server)).unwrap();
    let series = client.series_extended(81189).await.unwrap();
    assert_eq!(series.name, "Breaking Bad");
    assert_eq!(series.content_ratings[0].name, "TV-MA");
    assert_eq!(series.remote_ids.len(), 2);
    // A clone shares the token: no second login.
    client.clone().series_extended(81189).await.unwrap();
}

#[tokio::test]
async fn concurrent_requests_share_one_login() {
    let server = MockServer::start().await;
    mount_login(&server, "T1", 1).await;
    Mock::given(method("GET"))
        .respond_with(ok(movie_json()))
        .expect(3)
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let (a, b, c) = tokio::join!(
        client.movie_extended(12),
        client.movie_extended(12),
        client.movie_extended(12)
    );
    assert!(a.is_ok() && b.is_ok() && c.is_ok());
}

#[tokio::test]
async fn sends_the_pin_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .and(body_json(json!({ "apikey": SECRET, "pin": "4321" })))
        .respond_with(ok(json!({ "token": "T1" })))
        .expect(1)
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server).with_pin("4321")).unwrap();
    client.login().await.unwrap();
}

#[tokio::test]
async fn renews_the_token_once_on_401() {
    let server = MockServer::start().await;
    // The first login hands out a token TheTVDB later refuses.
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ok(json!({ "token": "STALE" })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    mount_login(&server, "FRESH", 1).await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer STALE"))
        .respond_with(failure(401, "Unauthorized"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/movies/12/extended"))
        .and(header("authorization", "Bearer FRESH"))
        .respond_with(ok(movie_json()))
        .expect(2)
        .mount(&server)
        .await;

    let client = TvdbClient::new(config(&server)).unwrap();
    assert_eq!(client.movie_extended(12).await.unwrap().id, 12);
    // The renewed token is kept for the next request.
    client.movie_extended(12).await.unwrap();
}

#[tokio::test]
async fn a_token_refused_after_renewal_is_an_auth_error() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 2).await;
    Mock::given(method("GET"))
        .respond_with(failure(401, "Unauthorized"))
        .expect(2)
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let err = client.tv_show_details(&MediaId::tvdb(1)).await.unwrap_err();
    assert_eq!(err, ProviderError::Auth("Unauthorized".into()));
}

#[tokio::test]
async fn a_rejected_api_key_is_an_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(failure(401, "InvalidAPIKey"))
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let err = client.login().await.unwrap_err();
    assert!(
        matches!(err, TvdbError::Status { status: 401, ref message, .. } if message == "InvalidAPIKey"),
        "{err:?}"
    );
    let err = client.movie_details(&MediaId::tvdb(12)).await.unwrap_err();
    assert_eq!(err, ProviderError::Auth("InvalidAPIKey".into()));
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn missing_record_is_not_found() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .respond_with(failure(404, "NotFoundException"))
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    assert!(matches!(
        client.series_extended(9).await,
        Err(TvdbError::NotFound)
    ));
    assert_eq!(
        client.movie_details(&MediaId::tvdb(9)).await.unwrap_err(),
        ProviderError::NotFound
    );
}

#[tokio::test]
async fn transient_errors_are_retried() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503).set_body_string("<html>down</html>"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ok(movie_json()))
        .expect(1)
        .mount(&server)
        .await;
    let retry = RetryPolicy::new(1, Duration::from_millis(1), Duration::from_millis(1));
    let client = TvdbClient::new(config(&server).with_retry(retry)).unwrap();
    assert_eq!(client.movie_extended(12).await.unwrap().id, 12);
}

#[tokio::test]
async fn non_tvdb_ids_are_rejected_without_a_request() {
    let server = MockServer::start().await;
    let client = TvdbClient::new(config(&server)).unwrap();
    assert!(matches!(
        client.tv_show_details(&MediaId::tmdb(1396)).await,
        Err(ProviderError::InvalidInput(_))
    ));
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
    );
}

#[tokio::test]
async fn errors_never_render_the_api_key() {
    let client = TvdbClient::new(
        TvdbConfig::new(SECRET)
            .with_pin("PIN-SECRET")
            .with_base_url("http://127.0.0.1:1")
            .with_retry(RetryPolicy::disabled()),
    )
    .unwrap();
    let err = client.login().await.unwrap_err();
    for rendered in [format!("{err}"), format!("{err:?}"), format!("{client:?}")] {
        assert!(!rendered.contains(SECRET), "{rendered}");
        assert!(!rendered.contains("PIN-SECRET"), "{rendered}");
    }
}

#[test]
fn new_rejects_invalid_config() {
    for cfg in [
        TvdbConfig::new(""),
        TvdbConfig::new("k").with_language("english"),
        TvdbConfig::new("k").with_base_url("::"),
    ] {
        assert!(matches!(
            TvdbClient::new(cfg),
            Err(TvdbError::InvalidConfig(_))
        ));
    }
}

// ── Search ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_pages_by_offset_and_reads_the_next_link() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("query", "breaking bad"))
        .and(query_param("type", "series"))
        .and(query_param("limit", "20"))
        .and(query_param("offset", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": [
                { "tvdb_id": "81189", "type": "series", "name": "Breaking Bad",
                  "first_air_time": "2008-01-20", "image_url": "https://img/p.jpg",
                  "primary_language": "eng", "status": "Ended", "genres": ["Drama"] },
                { "tvdb_id": "0", "type": "series", "name": "Invalid" }
            ],
            "links": { "prev": "p", "self": "s", "next": "n", "total_items": 45, "page_size": 20 }
        })))
        .mount(&server)
        .await;

    let client = TvdbClient::new(config(&server)).unwrap();
    let raw = client
        .search("breaking bad", Some(TvdbSearchType::Series), Some(2))
        .await
        .unwrap();
    assert_eq!(raw.results.len(), 2);
    assert_eq!((raw.page, raw.total_pages, raw.total_results), (2, 3, 45));
    assert!(raw.has_next);

    let shows = client
        .search_tv_shows("breaking bad", Some(2))
        .await
        .unwrap();
    assert_eq!(shows.results.len(), 1, "the invalid id is skipped");
    let show = &shows.results[0];
    assert_eq!(show.provider_id, MediaId::tvdb(81189));
    assert_eq!(show.status, Some(SeriesStatus::Ended));
    assert_eq!(show.original_language.as_deref(), Some("en"));
    assert_eq!(show.genres, vec![Genre::Drama]);
}

#[tokio::test]
async fn search_multi_keeps_series_and_movies_only() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ok(json!([
            { "tvdb_id": "1", "type": "series", "name": "Show" },
            { "tvdb_id": "2", "type": "movie", "name": "Film", "year": "1999" },
            { "tvdb_id": "3", "type": "person", "name": "Someone" },
            { "tvdb_id": "4", "type": "company", "name": "Studio" }
        ])))
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let page = client.search_multi("x", None).await.unwrap();
    assert!(!page.has_next);
    let kinds: Vec<&str> = page
        .results
        .iter()
        .map(|r| match r {
            UnifiedSearchResult::TvShow(_) => "tv",
            UnifiedSearchResult::Movie(_) => "movie",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, vec!["tv", "movie"]);
    assert_eq!(
        client.search_people("x", None).await.unwrap_err(),
        ProviderError::Unsupported
    );
}

// ── Details ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn series_details_convert_to_the_unified_model() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .and(path("/series/81189/extended"))
        .respond_with(ok(series_json()))
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let d = client.tv_show_details(&MediaId::tvdb(81189)).await.unwrap();

    assert_eq!(d.show.provider_id, "tvdb:81189");
    assert_eq!(d.show.name, "Breaking Bad");
    assert_eq!(d.show.original_name, None);
    assert_eq!(d.show.overview.as_deref(), Some("Walter White cooks."));
    assert_eq!(d.show.genres, vec![Genre::Drama, Genre::Crime]);
    assert_eq!(
        d.show.backdrop_url.as_deref(),
        Some("https://img/bg-high.jpg")
    );
    assert_eq!(
        d.show.poster_url.as_deref(),
        Some("https://artworks.thetvdb.com/banners/posters/81189-1.jpg")
    );
    assert_eq!(d.show.original_language.as_deref(), Some("en"));
    assert_eq!(d.show.episode_run_time, Some(47));
    assert_eq!(d.status, Some(SeriesStatus::Ended));
    assert_eq!(d.in_production, Some(false));
    assert_eq!(d.tagline.as_deref(), Some("Remember my name"));
    assert_eq!(
        d.last_air_date.map(|x| x.to_string()).as_deref(),
        Some("2013-09-29")
    );
    assert_eq!(
        d.networks.len(),
        1,
        "original and latest network deduplicated"
    );
    assert_eq!(d.networks[0].name, "AMC");
    let seasons: Vec<u32> = d.seasons.iter().map(|s| s.season_number).collect();
    assert_eq!(seasons, vec![0, 1, 2], "default order only, specials kept");
    assert_eq!(d.number_of_seasons, Some(2));
    assert!(
        d.show
            .alternative_titles
            .iter()
            .any(|t| t.title == "BB" && t.language.as_deref() == Some("en"))
    );

    let ids = client.tv_external_ids(&MediaId::tvdb(81189)).await.unwrap();
    assert_eq!(ids.imdb.as_deref(), Some("tt0903747"));
    assert_eq!(ids.tmdb, Some(1396));
    assert_eq!(ids.tvdb, Some(81189));
}

#[tokio::test]
async fn movie_details_use_the_configured_language() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .and(path("/movies/12/extended"))
        .respond_with(ok(movie_json()))
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let d = client.movie_details(&MediaId::tvdb(12)).await.unwrap();

    assert_eq!(d.movie.title, "Your Name.");
    assert_eq!(d.movie.original_title.as_deref(), Some("君の名は。"));
    assert_eq!(
        d.movie.overview.as_deref(),
        Some("Two strangers swap bodies.")
    );
    assert_eq!(
        d.movie.release_date.map(|x| x.to_string()).as_deref(),
        Some("2016-08-26")
    );
    assert_eq!(d.movie.original_language.as_deref(), Some("ja"));
    assert_eq!(
        d.movie.backdrop_url.as_deref(),
        Some("https://img/movie-bg.jpg")
    );
    assert_eq!(d.runtime, Some(106));
    assert_eq!(d.status.as_deref(), Some("Released"));
    assert_eq!(d.imdb_id.as_deref(), Some("tt5311514"));
    assert_eq!(d.budget, None, "a zero budget is unknown");
    assert_eq!(d.revenue, Some(380_000_000));
    assert_eq!(d.production_companies.len(), 1);
    assert_eq!(d.production_countries, vec!["Japan".to_string()]);
    assert_eq!(d.spoken_languages, vec!["ja".to_string()]);
    assert!(d.movie.genres.contains(&Genre::Romance));

    let ids = client.movie_external_ids(&MediaId::tvdb(12)).await.unwrap();
    assert_eq!(ids.imdb.as_deref(), Some("tt5311514"));
    assert_eq!(ids.tvdb, None, "a movie id is not a series id");
}

// ── Episodes ──────────────────────────────────────────────────────────────────

/// Mount two pages of default-order episodes in English: page 0 links to
/// page 1, which is the last.
async fn mount_episodes(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/series/81189/episodes/default/eng"))
        .and(query_param("page", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": { "series": { "id": 81189 }, "episodes": [
                episode(100, 0, 1, 0),
                episode(101, 1, 1, 1),
                episode(102, 1, 2, 2)
            ] },
            "links": { "next": "page=1", "total_items": 4, "page_size": 3 }
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/series/81189/episodes/default/eng"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "success",
            "data": { "episodes": [episode(103, 2, 1, 3)] },
            "links": { "next": null, "total_items": 4, "page_size": 3 }
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn episodes_page_from_one_and_carry_absolute_numbers() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    mount_episodes(&server).await;
    let client = TvdbClient::new(config(&server)).unwrap();

    let first = client.series_episodes(81189, None).await.unwrap();
    assert_eq!(
        (first.page, first.total_pages, first.total_results),
        (1, 2, 4)
    );
    assert_eq!(first.next_page(), Some(2));
    assert!(first.results[0].is_special());
    assert_eq!(first.results[0].absolute_number, None);

    let all = client.all_series_episodes(81189).await.unwrap();
    let numbered: Vec<(u32, u32, Option<u32>)> = all
        .iter()
        .map(|e| (e.season_number, e.number, e.absolute_number))
        .collect();
    assert_eq!(
        numbered,
        vec![
            (0, 1, None),
            (1, 1, Some(1)),
            (1, 2, Some(2)),
            (2, 1, Some(3))
        ]
    );
}

#[tokio::test]
async fn seasons_come_from_the_default_order() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    mount_episodes(&server).await;
    let client = TvdbClient::new(config(&server)).unwrap();
    let show = MediaId::tvdb(81189);

    let specials = client.season_details(&show, 0).await.unwrap();
    assert_eq!(specials.episodes.len(), 1);
    assert_eq!(specials.show_id, show);

    let season2 = client.season_details(&show, 2).await.unwrap();
    assert_eq!(season2.episodes[0].name.as_deref(), Some("S2E1"));

    let ep = client.episode_details(&show, 1, 2).await.unwrap();
    assert_eq!(ep.episode_number, 2);
    assert_eq!(ep.runtime, Some(47));

    assert_eq!(
        client.season_details(&show, 9).await.unwrap_err(),
        ProviderError::NotFound
    );
    assert_eq!(
        client.episode_details(&show, 1, 9).await.unwrap_err(),
        ProviderError::NotFound
    );
}

#[tokio::test]
async fn episodes_use_the_configured_language() {
    let server = MockServer::start().await;
    mount_login(&server, "T", 1).await;
    Mock::given(method("GET"))
        .and(path("/series/5/episodes/default/jpn"))
        .respond_with(ok(json!({ "episodes": null })))
        .expect(1)
        .mount(&server)
        .await;
    let client = TvdbClient::new(config(&server).with_language("jpn")).unwrap();
    let page = client.series_episodes(5, Some(1)).await.unwrap();
    assert!(page.results.is_empty());
    assert!(!page.has_next);
}
