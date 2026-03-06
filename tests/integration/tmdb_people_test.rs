use cameo::generated::tmdb::Client as GeneratedClient;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn search_person_fixture() -> &'static str {
    include_str!("../fixtures/search_person_response.json")
}

fn person_details_fixture() -> &'static str {
    include_str!("../fixtures/person_details_response.json")
}

async fn setup_mock_client() -> (MockServer, GeneratedClient) {
    let server = MockServer::start().await;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_static("Bearer test-token"),
    );
    let http = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap();
    let client = GeneratedClient::new_with_client(&server.uri(), http);
    (server, client)
}

#[tokio::test]
async fn search_person_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/search/person"))
        .and(query_param("query", "brad pitt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(search_person_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let resp = client
        .search_person(None, None, None, "brad pitt")
        .await
        .unwrap();
    let body = resp.into_inner();

    assert_eq!(body.page, 1);
    assert_eq!(body.total_results, 1);
    assert_eq!(body.results.len(), 1);

    let person = &body.results[0];
    assert_eq!(person.id, 287);
    assert_eq!(person.name.as_deref(), Some("Brad Pitt"));
    assert_eq!(person.known_for_department.as_deref(), Some("Acting"));
}

#[tokio::test]
async fn person_details_deserializes_response() {
    let (server, client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path("/3/person/287"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(person_details_fixture(), "application/json"),
        )
        .mount(&server)
        .await;

    let resp = client.person_details(287, None, None).await.unwrap();
    let body = resp.into_inner();

    assert_eq!(body.id, 287);
    assert_eq!(body.name.as_deref(), Some("Brad Pitt"));
    assert_eq!(body.biography.as_deref().map(|b| !b.is_empty()), Some(true));
    assert_eq!(body.birthday.as_deref(), Some("1963-12-18"));
    assert_eq!(body.imdb_id.as_deref(), Some("nm0000093"));
}

#[cfg(feature = "live-tests")]
fn client() -> TmdbClient {
    let token = std::env::var("TMDB_API_TOKEN").expect("TMDB_API_TOKEN must be set for live tests");
    TmdbClient::new(TmdbConfig::new(token).with_language("en-US")).unwrap()
}

#[cfg(feature = "live-tests")]
#[tokio::test]
async fn live_person_details_brad_pitt() {
    let c = client();
    let person = c.person_details(287).await.unwrap(); // Brad Pitt

    assert_eq!(person.id, 287);
    assert_eq!(person.name.as_deref(), Some("Brad Pitt"));
    assert!(
        person
            .biography
            .as_deref()
            .map(|b| !b.is_empty())
            .unwrap_or(false)
    );
}
