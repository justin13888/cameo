use cameo::providers::tmdb::{TmdbClient, TmdbConfig};

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
    assert!(person.biography.as_deref().map(|b| !b.is_empty()).unwrap_or(false));
}
