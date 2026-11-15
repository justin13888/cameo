use cameo::providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
    (server, client)
}

fn pairs(certs: Vec<cameo::providers::tmdb::TmdbCertification>) -> Vec<(String, String)> {
    certs
        .into_iter()
        .map(|c| (c.country, c.certification))
        .collect()
}

#[tokio::test]
async fn movie_certifications_reads_release_dates() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/603/release_dates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 603,
            "results": [
                { "iso_3166_1": "US", "release_dates": [
                    { "certification": "", "iso_639_1": "", "note": "",
                      "release_date": "1999-03-24T00:00:00.000Z", "type": 1, "descriptors": [] },
                    { "certification": "R", "iso_639_1": "", "note": "",
                      "release_date": "1999-03-31T00:00:00.000Z", "type": 3, "descriptors": [] }
                ] },
                { "iso_3166_1": "GB", "release_dates": [
                    { "certification": "15", "type": 3 }
                ] },
                { "iso_3166_1": "JP", "release_dates": [
                    { "certification": "", "type": 3 }
                ] }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let certs = client.movie_certifications(603).await.unwrap();

    assert_eq!(
        pairs(certs),
        [
            ("US".to_owned(), "R".to_owned()),
            ("GB".to_owned(), "15".to_owned())
        ]
    );
}

#[tokio::test]
async fn tv_series_certifications_reads_content_ratings() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396/content_ratings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 1396,
            "results": [
                { "descriptors": [], "iso_3166_1": "US", "rating": "TV-MA" },
                { "descriptors": [], "iso_3166_1": "DE", "rating": "16" },
                { "descriptors": [], "iso_3166_1": "FR", "rating": "" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let certs = client.tv_show_certifications(1396).await.unwrap();

    assert_eq!(
        pairs(certs),
        [
            ("US".to_owned(), "TV-MA".to_owned()),
            ("DE".to_owned(), "16".to_owned())
        ]
    );
}

/// A title TMDB has no ratings for reads as an empty list, not an error.
#[tokio::test]
async fn certifications_without_results_are_empty() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/1/release_dates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": 1 })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/3/tv/1/content_ratings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": 1, "results": [] })))
        .mount(&server)
        .await;

    assert!(client.movie_certifications(1).await.unwrap().is_empty());
    assert!(client.tv_show_certifications(1).await.unwrap().is_empty());
}

/// A zero id, or one above `i32::MAX`, is rejected before any request is sent.
#[tokio::test]
async fn certifications_reject_zero_and_out_of_range_ids() {
    let (server, client) = setup_mock_client().await;
    let too_big = u64::try_from(i32::MAX).expect("positive") + 1;

    for id in [0, too_big] {
        assert!(matches!(
            client.movie_certifications(id).await,
            Err(TmdbError::Request { .. })
        ));
        assert!(matches!(
            client.tv_show_certifications(id).await,
            Err(TmdbError::Request { .. })
        ));
    }
    let received = server.received_requests().await.unwrap_or_default();
    assert!(received.is_empty(), "no request may be sent: {received:?}");
}

fn unified_pairs(certs: Vec<cameo::Certification>) -> Vec<(String, String)> {
    certs.into_iter().map(|c| (c.country, c.rating)).collect()
}

/// The `release_dates` append fills the unified movie details'
/// certifications with the same per-country pick `movie_certifications` makes.
#[tokio::test]
async fn movie_details_with_release_dates_append_fills_certifications() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/603"))
        .and(query_param("append_to_response", "release_dates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 603,
            "title": "The Matrix",
            "release_dates": { "results": [
                { "iso_3166_1": "US", "release_dates": [
                    { "certification": "NR", "type": 4 },
                    { "certification": "R", "type": 3 }
                ] },
                { "iso_3166_1": "DE", "release_dates": [
                    { "certification": "", "type": 3 },
                    { "certification": "16", "type": 5 }
                ] },
                { "iso_3166_1": "JP", "release_dates": [{ "certification": "", "type": 3 }] }
            ] }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let details = client
        .movie_details_with_append(603, "release_dates")
        .await
        .unwrap();

    assert_eq!(
        unified_pairs(details.certifications),
        [
            ("US".to_owned(), "R".to_owned()),
            ("DE".to_owned(), "16".to_owned())
        ]
    );
}

/// `tv_show_details_with_append` sends the append, and the
/// `content_ratings` block fills the unified TV details' certifications.
#[tokio::test]
async fn tv_series_details_with_content_ratings_append_fills_certifications() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/tv/1396"))
        .and(query_param("append_to_response", "content_ratings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 1396,
            "name": "Breaking Bad",
            "content_ratings": { "results": [
                { "descriptors": [], "iso_3166_1": "US", "rating": "TV-MA" },
                { "descriptors": [], "iso_3166_1": "FR", "rating": "" },
                { "descriptors": [], "iso_3166_1": "DE", "rating": "16" }
            ] }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let details = client
        .tv_show_details_with_append(1396, "content_ratings")
        .await
        .unwrap();

    assert_eq!(details.show.provider_id, "tmdb:1396");
    assert_eq!(
        unified_pairs(details.certifications),
        [
            ("US".to_owned(), "TV-MA".to_owned()),
            ("DE".to_owned(), "16".to_owned())
        ]
    );
}

/// Without the append, or with a `null` block, the certifications are empty.
#[tokio::test]
async fn details_without_certification_append_have_no_certifications() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/movie/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": 1 })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/3/tv/1"))
        .and(query_param("append_to_response", "credits"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "id": 1, "content_ratings": null })),
        )
        .mount(&server)
        .await;

    let movie = client.movie_details(1).await.unwrap();
    assert!(movie.certifications.is_empty());
    let tv = client
        .tv_show_details_with_append(1, "credits")
        .await
        .unwrap();
    assert!(tv.certifications.is_empty());
}
