//! Every `MovieSortBy` / `TvSortBy` variant puts its own TMDB wire string in
//! the discover request's `sort_by` parameter.

use cameo::providers::tmdb::{
    TmdbClient, TmdbConfig,
    builders::{MovieSortBy, TvSortBy},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const DISCOVER_MOVIES: &str = include_str!("../fixtures/discover_movies_response.json");
const DISCOVER_TV: &str = include_str!("../fixtures/discover_tv_response.json");

/// The `sort_by` value of every request `server` received, in arrival order.
async fn sent_sort_orders(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .expect("request recording is enabled")
        .iter()
        .map(|request| {
            request
                .url
                .query_pairs()
                .find(|(k, _)| k == "sort_by")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default()
        })
        .collect()
}

async fn client_serving(route: &str, fixture: &'static str) -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_raw(fixture, "application/json"))
        .mount(&server)
        .await;
    let client = TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri()))
        .expect("valid config");
    (server, client)
}

#[tokio::test]
async fn every_movie_sort_order_has_its_wire_string() {
    let cases = [
        (MovieSortBy::OriginalTitleAsc, "original_title.asc"),
        (MovieSortBy::OriginalTitleDesc, "original_title.desc"),
        (MovieSortBy::PopularityAsc, "popularity.asc"),
        (MovieSortBy::PopularityDesc, "popularity.desc"),
        (MovieSortBy::RevenueAsc, "revenue.asc"),
        (MovieSortBy::RevenueDesc, "revenue.desc"),
        (
            MovieSortBy::PrimaryReleaseDateAsc,
            "primary_release_date.asc",
        ),
        (
            MovieSortBy::PrimaryReleaseDateDesc,
            "primary_release_date.desc",
        ),
        (MovieSortBy::TitleAsc, "title.asc"),
        (MovieSortBy::TitleDesc, "title.desc"),
        (MovieSortBy::VoteAverageAsc, "vote_average.asc"),
        (MovieSortBy::VoteAverageDesc, "vote_average.desc"),
        (MovieSortBy::VoteCountAsc, "vote_count.asc"),
        (MovieSortBy::VoteCountDesc, "vote_count.desc"),
    ];
    let (server, client) = client_serving("/3/discover/movie", DISCOVER_MOVIES).await;
    for (sort, _) in cases {
        client
            .discover_movies()
            .sort_by(sort)
            .execute()
            .await
            .expect("discover succeeds");
    }
    let expected: Vec<String> = cases.iter().map(|(_, wire)| wire.to_string()).collect();
    assert_eq!(sent_sort_orders(&server).await, expected);
}

#[tokio::test]
async fn every_tv_sort_order_has_its_wire_string() {
    let cases = [
        (TvSortBy::FirstAirDateAsc, "first_air_date.asc"),
        (TvSortBy::FirstAirDateDesc, "first_air_date.desc"),
        (TvSortBy::NameAsc, "name.asc"),
        (TvSortBy::NameDesc, "name.desc"),
        (TvSortBy::OriginalNameAsc, "original_name.asc"),
        (TvSortBy::OriginalNameDesc, "original_name.desc"),
        (TvSortBy::PopularityAsc, "popularity.asc"),
        (TvSortBy::PopularityDesc, "popularity.desc"),
        (TvSortBy::VoteAverageAsc, "vote_average.asc"),
        (TvSortBy::VoteAverageDesc, "vote_average.desc"),
        (TvSortBy::VoteCountAsc, "vote_count.asc"),
        (TvSortBy::VoteCountDesc, "vote_count.desc"),
    ];
    let (server, client) = client_serving("/3/discover/tv", DISCOVER_TV).await;
    for (sort, _) in cases {
        client
            .discover_tv()
            .sort_by(sort)
            .execute()
            .await
            .expect("discover succeeds");
    }
    let expected: Vec<String> = cases.iter().map(|(_, wire)| wire.to_string()).collect();
    assert_eq!(sent_sort_orders(&server).await, expected);
}
