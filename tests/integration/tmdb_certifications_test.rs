use cameo::providers::tmdb::{TmdbClient, TmdbConfig, TmdbError};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
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

    let certs = client.tv_series_certifications(1396).await.unwrap();

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
    assert!(client.tv_series_certifications(1).await.unwrap().is_empty());
}

/// A non-positive id is rejected before any request is sent.
#[tokio::test]
async fn certifications_reject_non_positive_ids() {
    let (server, client) = setup_mock_client().await;

    assert!(matches!(
        client.movie_certifications(0).await,
        Err(TmdbError::Request { .. })
    ));
    assert!(matches!(
        client.tv_series_certifications(-5).await,
        Err(TmdbError::Request { .. })
    ));
    let received = server.received_requests().await.unwrap_or_default();
    assert!(received.is_empty(), "no request may be sent: {received:?}");
}
