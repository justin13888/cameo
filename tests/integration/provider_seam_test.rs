//! The v1 extension seam: concrete providers are usable as `dyn` trait objects,
//! carry capability metadata, and are `Send + Sync` (spawnable).

use std::sync::Arc;

use cameo::unified::{Capability, SearchProvider};

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

#[cfg(feature = "tmdb")]
#[test]
fn tmdb_client_is_a_dyn_provider() {
    use cameo::{
        providers::tmdb::{TmdbClient, TmdbConfig},
        unified::{DetailProvider, DiscoveryProvider},
    };

    let client = TmdbClient::new(TmdbConfig::new("token")).unwrap();

    // Constructible as each capability trait object.
    let search: Arc<dyn SearchProvider> = Arc::new(client.clone());
    let _detail: Arc<dyn DetailProvider> = Arc::new(client.clone());
    let _discovery: Arc<dyn DiscoveryProvider> = Arc::new(client);

    assert_eq!(search.id(), "tmdb");
    assert!(search.supports(Capability::Search));
    assert!(search.supports(Capability::WatchAvailability));

    // The trait objects are Send + Sync, so they can cross `tokio::spawn`.
    assert_send_sync::<Arc<dyn SearchProvider>>();
    assert_send_sync::<Arc<dyn DetailProvider>>();
}

#[cfg(feature = "anilist")]
#[test]
fn anilist_client_is_a_dyn_provider() {
    use cameo::providers::anilist::{AniListClient, AniListConfig};

    let client = AniListClient::new(AniListConfig::new()).unwrap();
    let search: Arc<dyn SearchProvider> = Arc::new(client);

    assert_eq!(search.id(), "anilist");
    assert!(search.supports(Capability::Search));
    assert!(search.supports(Capability::Discovery));
    // AniList does not implement recommendations / seasons / watch availability.
    assert!(!search.supports(Capability::WatchAvailability));
    assert!(!search.supports(Capability::Seasons));

    assert_send_sync::<Arc<dyn SearchProvider>>();
}

/// A heterogeneous registry of `dyn` providers is constructible from both.
#[cfg(all(feature = "tmdb", feature = "anilist"))]
#[test]
fn heterogeneous_dyn_registry() {
    use cameo::providers::{
        anilist::{AniListClient, AniListConfig},
        tmdb::{TmdbClient, TmdbConfig},
    };

    let providers: Vec<Arc<dyn SearchProvider>> = vec![
        Arc::new(TmdbClient::new(TmdbConfig::new("token")).unwrap()),
        Arc::new(AniListClient::new(AniListConfig::new()).unwrap()),
    ];
    let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();
    assert_eq!(ids, vec!["tmdb", "anilist"]);
}
