use cameo::providers::tmdb::{TmdbClient, TmdbConfig};
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

async fn setup_mock_client() -> (MockServer, TmdbClient) {
    let server = MockServer::start().await;
    let client =
        TmdbClient::new(TmdbConfig::new_with_base_url("test-token", server.uri())).unwrap();
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

    let page = client.search_people("brad pitt", None).await.unwrap();

    assert_eq!(page.page, 1);
    assert_eq!(page.total_results, 1);
    assert_eq!(page.results.len(), 1);

    let person = &page.results[0];
    assert!(person.provider_id == "tmdb:287");
    assert_eq!(person.name, "Brad Pitt");
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

    let details = client.person_details(287).await.unwrap();

    assert!(details.person.provider_id == "tmdb:287");
    assert_eq!(details.person.name, "Brad Pitt");
    assert_eq!(
        details.biography.as_deref().map(|b| !b.is_empty()),
        Some(true)
    );
    assert_eq!(
        details.birthday.map(|d| d.to_string()).as_deref(),
        Some("1963-12-18")
    );
    assert_eq!(details.imdb_id.as_deref(), Some("nm0000093"));
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
    let details = c.person_details(287).await.unwrap(); // Brad Pitt

    assert!(details.person.provider_id == "tmdb:287");
    assert_eq!(details.person.name, "Brad Pitt");
    assert!(
        details
            .biography
            .as_deref()
            .map(|b| !b.is_empty())
            .unwrap_or(false)
    );
}
