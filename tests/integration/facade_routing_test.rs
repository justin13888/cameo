//! `CameoClient` registration order, priority, routing errors, and cache
//! behaviour, pinned against fake providers and a recording cache backend.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cameo::{
    Capability, CreditsProvider, DetailProvider, DiscoveryProvider, ExternalId, ExternalIdProvider,
    ExternalIds, ExternalMatch, ExternalSource, MediaId, NativeId, Page, Provider, ProviderError,
    RecommendationProvider, SearchProvider, SeasonProvider, TimeWindow, UnifiedCredits,
    UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
    UnifiedSearchResult, UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
    UnifiedWatchProviders, WatchAvailabilityProvider,
    unified::{CameoClient, CameoClientBuilder, CameoClientError},
};

const ALL: &[Capability] = &[
    Capability::Search,
    Capability::Details,
    Capability::Discovery,
    Capability::Recommendations,
    Capability::Seasons,
    Capability::WatchAvailability,
    Capability::ExternalIds,
    Capability::Credits,
];

/// A provider implementing every capability. Each call is logged as
/// `"{id}.{method}"`, and every result carries the provider's id so a test can
/// tell who answered.
struct Fake {
    id: &'static str,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    fn new(id: &'static str) -> Arc<Self> {
        Arc::new(Self {
            id,
            calls: Arc::default(),
        })
    }

    fn log(&self, method: &str) {
        self.calls
            .lock()
            .expect("call log")
            .push(format!("{}.{method}", self.id));
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("call log").clone()
    }

    fn own_id(&self, n: u64) -> MediaId {
        MediaId::new(self.id, NativeId::Num(n))
    }

    fn movies(&self, method: &str) -> Page<UnifiedMovie> {
        self.log(method);
        Page::offset(1, vec![UnifiedMovie::new(self.own_id(1), self.id)], 1, 1)
    }

    fn shows(&self, method: &str) -> Page<UnifiedTvShow> {
        self.log(method);
        Page::offset(1, vec![UnifiedTvShow::new(self.own_id(2), self.id)], 1, 1)
    }
}

impl Provider for Fake {
    fn id(&self) -> &str {
        self.id
    }
    fn capabilities(&self) -> &[Capability] {
        ALL
    }
}

#[async_trait]
impl SearchProvider for Fake {
    async fn search_movies(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("search_movies"))
    }
    async fn search_tv_shows(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("search_tv_shows"))
    }
    async fn search_people(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedPerson>, ProviderError> {
        self.log("search_people");
        Ok(Page::offset(
            1,
            vec![UnifiedPerson::new(self.own_id(3), self.id)],
            1,
            1,
        ))
    }
    async fn search_multi(
        &self,
        _q: &str,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedSearchResult>, ProviderError> {
        self.log("search_multi");
        Ok(Page::offset(
            1,
            vec![
                UnifiedSearchResult::movie(UnifiedMovie::new(self.own_id(1), self.id)),
                UnifiedSearchResult::tv_show(UnifiedTvShow::new(self.own_id(2), self.id)),
                UnifiedSearchResult::person(UnifiedPerson::new(self.own_id(3), self.id)),
            ],
            1,
            3,
        ))
    }
}

#[async_trait]
impl DetailProvider for Fake {
    async fn movie_details(&self, id: &MediaId) -> Result<UnifiedMovieDetails, ProviderError> {
        self.log("movie_details");
        Ok(UnifiedMovieDetails::new(UnifiedMovie::new(id.clone(), self.id)).with_tagline("fresh"))
    }
    async fn tv_show_details(&self, id: &MediaId) -> Result<UnifiedTvShowDetails, ProviderError> {
        self.log("tv_show_details");
        Ok(UnifiedTvShowDetails::new(UnifiedTvShow::new(
            id.clone(),
            self.id,
        )))
    }
    async fn person_details(&self, id: &MediaId) -> Result<UnifiedPersonDetails, ProviderError> {
        self.log("person_details");
        Ok(UnifiedPersonDetails::new(UnifiedPerson::new(
            id.clone(),
            self.id,
        )))
    }
}

#[async_trait]
impl DiscoveryProvider for Fake {
    async fn trending_movies(
        &self,
        _w: TimeWindow,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("trending_movies"))
    }
    async fn trending_tv_shows(
        &self,
        _w: TimeWindow,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("trending_tv_shows"))
    }
    async fn popular_movies(&self, _p: Option<u32>) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("popular_movies"))
    }
    async fn top_rated_movies(&self, _p: Option<u32>) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("top_rated_movies"))
    }
    async fn popular_tv_shows(
        &self,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("popular_tv_shows"))
    }
    async fn top_rated_tv_shows(
        &self,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("top_rated_tv_shows"))
    }
}

#[async_trait]
impl RecommendationProvider for Fake {
    async fn movie_recommendations(
        &self,
        _id: &MediaId,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("movie_recommendations"))
    }
    async fn tv_recommendations(
        &self,
        _id: &MediaId,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("tv_recommendations"))
    }
    async fn similar_movies(
        &self,
        _id: &MediaId,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedMovie>, ProviderError> {
        Ok(self.movies("similar_movies"))
    }
    async fn similar_tv_shows(
        &self,
        _id: &MediaId,
        _p: Option<u32>,
    ) -> Result<Page<UnifiedTvShow>, ProviderError> {
        Ok(self.shows("similar_tv_shows"))
    }
}

#[async_trait]
impl SeasonProvider for Fake {
    async fn season_details(
        &self,
        show_id: &MediaId,
        season_number: u32,
    ) -> Result<UnifiedSeasonDetails, ProviderError> {
        self.log("season_details");
        Ok(UnifiedSeasonDetails::new(show_id.clone(), season_number))
    }
    async fn episode_details(
        &self,
        _show_id: &MediaId,
        _season_number: u32,
        episode_number: u32,
    ) -> Result<UnifiedEpisode, ProviderError> {
        self.log("episode_details");
        Ok(UnifiedEpisode::new(episode_number))
    }
}

#[async_trait]
impl WatchAvailabilityProvider for Fake {
    async fn movie_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError> {
        self.log("movie_watch_providers");
        Ok(UnifiedWatchProviders::new(id.clone()))
    }
    async fn tv_watch_providers(
        &self,
        id: &MediaId,
    ) -> Result<UnifiedWatchProviders, ProviderError> {
        self.log("tv_watch_providers");
        Ok(UnifiedWatchProviders::new(id.clone()))
    }
}

#[async_trait]
impl ExternalIdProvider for Fake {
    fn external_sources(&self) -> &[ExternalSource] {
        &[ExternalSource::Imdb]
    }
    async fn movie_external_ids(&self, _id: &MediaId) -> Result<ExternalIds, ProviderError> {
        self.log("movie_external_ids");
        Ok(ExternalIds::new())
    }
    async fn tv_external_ids(&self, _id: &MediaId) -> Result<ExternalIds, ProviderError> {
        self.log("tv_external_ids");
        Ok(ExternalIds::new())
    }
    async fn find_by_external_id(
        &self,
        _id: &ExternalId,
    ) -> Result<Vec<ExternalMatch>, ProviderError> {
        self.log("find_by_external_id");
        Ok(vec![ExternalMatch::movie(self.own_id(1))])
    }
}

#[async_trait]
impl CreditsProvider for Fake {
    async fn movie_credits(&self, id: &MediaId) -> Result<UnifiedCredits, ProviderError> {
        self.log("movie_credits");
        Ok(UnifiedCredits::new(id.clone()))
    }
    async fn tv_show_credits(&self, id: &MediaId) -> Result<UnifiedCredits, ProviderError> {
        self.log("tv_show_credits");
        Ok(UnifiedCredits::new(id.clone()))
    }
}

/// Register `fake` for every capability.
fn register_all(builder: CameoClientBuilder, fake: &Arc<Fake>) -> CameoClientBuilder {
    builder
        .register_search_provider(fake.clone())
        .register_detail_provider(fake.clone())
        .register_discovery_provider(fake.clone())
        .register_recommendation_provider(fake.clone())
        .register_season_provider(fake.clone())
        .register_watch_availability_provider(fake.clone())
        .register_external_id_provider(fake.clone())
        .register_credits_provider(fake.clone())
}

/// The eight `register_*` methods, so a test can populate one capability list.
type Register = fn(CameoClientBuilder, Arc<Fake>) -> CameoClientBuilder;

const REGISTER_ONE: [(&str, Register); 8] = [
    ("search", |b, f| b.register_search_provider(f)),
    ("detail", |b, f| b.register_detail_provider(f)),
    ("discovery", |b, f| b.register_discovery_provider(f)),
    ("recommendation", |b, f| {
        b.register_recommendation_provider(f)
    }),
    ("season", |b, f| b.register_season_provider(f)),
    ("watch", |b, f| b.register_watch_availability_provider(f)),
    ("external", |b, f| b.register_external_id_provider(f)),
    ("credits", |b, f| b.register_credits_provider(f)),
];

// ── Registration order and priority ──────────────────────────────────────────

/// A partial priority moves the listed provider first and keeps the unlisted
/// ones in registration order, in every one of the eight capability lists.
#[test]
fn partial_priority_keeps_unlisted_providers_in_registration_order() {
    for (list, register) in REGISTER_ONE {
        let mut builder = CameoClient::builder();
        for id in ["a", "b", "c"] {
            builder = register(builder, Fake::new(id));
        }
        let default = builder.build().expect("a registered provider configures");
        assert_eq!(default.provider_ids(), ["a", "b", "c"], "{list} default");

        let mut builder = CameoClient::builder().with_priority(["c"]);
        for id in ["a", "b", "c"] {
            builder = register(builder, Fake::new(id));
        }
        let client = builder.build().expect("a registered provider configures");
        assert_eq!(client.provider_ids(), ["c", "a", "b"], "{list} prioritized");
    }
}

/// A priority naming an unregistered id changes nothing.
#[test]
fn priority_for_an_unknown_id_keeps_registration_order() {
    let client = register_all(
        register_all(
            CameoClient::builder().with_priority(["zzz"]),
            &Fake::new("a"),
        ),
        &Fake::new("b"),
    )
    .build()
    .expect("configured");
    assert_eq!(client.provider_ids(), ["a", "b"]);
}

/// `provider_ids` lists each id once, search providers first, then ids first
/// seen in the other lists; `Debug` shows the same list.
#[test]
fn provider_ids_are_distinct_search_first_and_shown_by_debug() {
    let a = Fake::new("a");
    let b = Fake::new("b");
    let client = CameoClient::builder()
        .register_detail_provider(b.clone())
        .register_credits_provider(b)
        .register_search_provider(a.clone())
        .register_detail_provider(a)
        .build()
        .expect("configured");
    assert_eq!(client.provider_ids(), ["a", "b"]);
    assert_eq!(
        format!("{client:?}"),
        r#"CameoClient { providers: ["a", "b"], .. }"#
    );
    assert_eq!(
        format!("{:?}", CameoClient::builder()),
        "CameoClientBuilder { .. }"
    );
}

/// Built-in providers register before out-of-crate ones, TMDB before
/// AniList, whatever order the builder methods were called in.
#[cfg(all(feature = "tmdb", feature = "anilist"))]
#[test]
fn built_ins_register_tmdb_then_anilist_then_external() {
    use cameo::{AniListConfig, TmdbConfig};
    let client = CameoClient::builder()
        .register_search_provider(Fake::new("ext"))
        .with_anilist(AniListConfig::new())
        .with_tmdb(TmdbConfig::new("token"))
        .build()
        .expect("configured");
    assert_eq!(client.provider_ids(), ["tmdb", "anilist", "ext"]);

    let reprioritized = CameoClient::builder()
        .register_search_provider(Fake::new("ext"))
        .with_anilist(AniListConfig::new())
        .with_tmdb(TmdbConfig::new("token"))
        .with_priority(["ext", "anilist"])
        .build()
        .expect("configured");
    assert_eq!(reprioritized.provider_ids(), ["ext", "anilist", "tmdb"]);
}

// ── No-provider errors ────────────────────────────────────────────────────────

/// With something registered, a capability nobody provides is `Unsupported`
/// whether it is resolved by first-in-list, id origin, or explicit selection.
#[tokio::test]
async fn missing_capability_is_unsupported_for_require_route_and_select() {
    let a = Fake::new("a");
    let client = CameoClient::builder()
        .register_detail_provider(a.clone())
        .build()
        .expect("configured");

    // require: no search/discovery provider.
    let r = client.search_movies("q", None).await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");
    let r = client.trending_movies(TimeWindow::Day, None).await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");
    // route: an id whose origin has no provider for the capability.
    let r = client.movie_details(&MediaId::tmdb(1)).await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");
    let r = client
        .season_details(&MediaId::new("a", NativeId::Num(1)), 1)
        .await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");
    // select: a provider id that is not registered for search.
    let r = client.search_movies_with("a", "q", None).await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");
    let r = client.search_movies_with("zzz", "q", None).await;
    assert!(matches!(r, Err(CameoClientError::Unsupported)), "{r:?}");

    // The routed call to the registered origin still works.
    client
        .movie_details(&MediaId::new("a", NativeId::Num(1)))
        .await
        .expect("routed to a");
    assert_eq!(a.calls(), ["a.movie_details"]);
}

/// An id-routed call reaches only the provider that minted the id, for every
/// id-routed capability.
#[tokio::test]
async fn id_routed_calls_reach_only_the_minting_provider() {
    let a = Fake::new("a");
    let b = Fake::new("b");
    let client = register_all(register_all(CameoClient::builder(), &a), &b)
        .build()
        .expect("configured");
    let id = MediaId::new("b", NativeId::Num(7));

    client.movie_details(&id).await.expect("details");
    client.tv_show_details(&id).await.expect("details");
    client.person_details(&id).await.expect("details");
    client.movie_recommendations(&id, None).await.expect("recs");
    client.similar_tv_shows(&id, None).await.expect("similar");
    client.season_details(&id, 1).await.expect("season");
    client.episode_details(&id, 1, 2).await.expect("episode");
    client.movie_watch_providers(&id).await.expect("watch");
    client.tv_external_ids(&id).await.expect("external");
    client.movie_credits(&id).await.expect("credits");

    assert!(a.calls().is_empty(), "a was called: {:?}", a.calls());
    assert_eq!(
        b.calls(),
        [
            "b.movie_details",
            "b.tv_show_details",
            "b.person_details",
            "b.movie_recommendations",
            "b.similar_tv_shows",
            "b.season_details",
            "b.episode_details",
            "b.movie_watch_providers",
            "b.tv_external_ids",
            "b.movie_credits",
        ]
    );
}

// ── Built-in providers: origin routing and id validation ──────────────────────

#[cfg(all(feature = "tmdb", feature = "anilist"))]
mod built_ins {
    use cameo::{AniListConfig, MediaId, TmdbConfig, unified::CameoClient};
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    /// With both built-ins configured, `anilist:` ids never reach TMDB:
    /// AniList answers what it supports, and the rest is `Unsupported`.
    #[tokio::test]
    async fn anilist_ids_never_reach_tmdb() {
        let tmdb = MockServer::start().await;
        let anilist = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": { "Media": { "recommendations": {
                    "pageInfo": { "total": 0, "currentPage": 1, "lastPage": 1, "hasNextPage": false },
                    "nodes": []
                } } }
            })))
            .mount(&anilist)
            .await;
        let client = CameoClient::builder()
            .with_tmdb(TmdbConfig::new_with_base_url("token", tmdb.uri()))
            .with_anilist(AniListConfig::new_with_base_url(anilist.uri()))
            .build()
            .expect("configured");
        let id = MediaId::anilist(1);

        let recs = client
            .movie_recommendations(&id, None)
            .await
            .expect("AniList answers anilist-id recommendations");
        assert!(recs.results.is_empty());
        let season = client.season_details(&id, 1).await;
        assert!(
            matches!(season, Err(cameo::CameoClientError::Unsupported)),
            "{season:?}"
        );
        let watch = client.movie_watch_providers(&id).await;
        assert!(
            matches!(watch, Err(cameo::CameoClientError::Unsupported)),
            "{watch:?}"
        );

        assert_eq!(tmdb.received_requests().await.expect("recording").len(), 0);
        assert_eq!(
            anilist.received_requests().await.expect("recording").len(),
            1
        );
    }
}

/// The built-in providers reject foreign, non-numeric, and out-of-range ids
/// as `InvalidInput` before sending any request.
#[cfg(feature = "tmdb")]
#[tokio::test]
async fn tmdb_provider_rejects_invalid_ids_without_a_request() {
    use cameo::{TmdbClient, TmdbConfig};
    let server = wiremock::MockServer::start().await;
    let tmdb = TmdbClient::new(TmdbConfig::new_with_base_url("token", server.uri()))
        .expect("valid config");
    for id in [
        MediaId::anilist(1),
        MediaId::new("tmdb", NativeId::Text("abc".into())),
        MediaId::tmdb(u64::from(u32::MAX)),
    ] {
        let r = DetailProvider::movie_details(&tmdb, &id).await;
        assert!(
            matches!(r, Err(ProviderError::InvalidInput(_))),
            "{id}: {r:?}"
        );
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("recording")
            .is_empty()
    );
}

#[cfg(feature = "anilist")]
#[tokio::test]
async fn anilist_provider_rejects_invalid_ids_without_a_request() {
    use cameo::{AniListClient, AniListConfig};
    let server = wiremock::MockServer::start().await;
    let anilist =
        AniListClient::new(AniListConfig::new_with_base_url(server.uri())).expect("valid config");
    for id in [
        MediaId::tmdb(1),
        MediaId::new("anilist", NativeId::Text("abc".into())),
        MediaId::anilist(u64::from(u32::MAX)),
    ] {
        let r = DetailProvider::movie_details(&anilist, &id).await;
        assert!(
            matches!(r, Err(ProviderError::InvalidInput(_))),
            "{id}: {r:?}"
        );
        let r = CreditsProvider::movie_credits(&anilist, &id).await;
        assert!(
            matches!(r, Err(ProviderError::InvalidInput(_))),
            "{id}: {r:?}"
        );
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("recording")
            .is_empty()
    );
}

// ── Cache behaviour ──────────────────────────────────────────────────────────

#[cfg(feature = "cache")]
mod cache {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use async_trait::async_trait;
    use cameo::{
        CacheBackend, CacheError, CacheKey, CacheTtlConfig, MediaId, MediaType, NativeId,
        TimeWindow, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
        UnifiedTvShow, UnifiedTvShowDetails, unified::CameoClient,
    };

    use super::{Fake, register_all};

    #[derive(Debug)]
    struct Broken;
    impl std::fmt::Display for Broken {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("broken backend")
        }
    }
    impl std::error::Error for Broken {}

    /// An in-memory backend recording every write with its TTL; it can be
    /// told to fail reads or writes.
    #[derive(Default)]
    struct Recording {
        entries: Mutex<HashMap<CacheKey, serde_json::Value>>,
        writes: Mutex<Vec<(CacheKey, Duration)>>,
        fail_get: bool,
        fail_set: bool,
    }

    impl Recording {
        fn writes(&self) -> Vec<(CacheKey, Duration)> {
            self.writes.lock().expect("writes").clone()
        }
        fn put(&self, key: CacheKey, value: serde_json::Value) {
            self.entries.lock().expect("entries").insert(key, value);
        }
        fn value(&self, key: &CacheKey) -> Option<serde_json::Value> {
            self.entries.lock().expect("entries").get(key).cloned()
        }
    }

    #[async_trait]
    impl CacheBackend for Recording {
        async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError> {
            if self.fail_get {
                return Err(CacheError::backend(Broken));
            }
            Ok(self.value(key))
        }
        async fn set(
            &self,
            key: CacheKey,
            value: serde_json::Value,
            ttl: Duration,
        ) -> Result<(), CacheError> {
            self.writes.lock().expect("writes").push((key.clone(), ttl));
            if self.fail_set {
                return Err(CacheError::backend(Broken));
            }
            self.put(key, value);
            Ok(())
        }
        async fn invalidate(&self, key: &CacheKey) -> Result<(), CacheError> {
            self.entries.lock().expect("entries").remove(key);
            Ok(())
        }
        async fn clear(&self) -> Result<(), CacheError> {
            self.entries.lock().expect("entries").clear();
            Ok(())
        }
    }

    fn ttls() -> CacheTtlConfig {
        let mut ttl = CacheTtlConfig::default();
        ttl.details = Duration::from_secs(11);
        ttl.search = Duration::from_secs(22);
        ttl.discovery = Duration::from_secs(33);
        ttl.items = Duration::from_secs(44);
        ttl
    }

    fn client(fake: &Arc<Fake>, backend: Arc<Recording>) -> CameoClient {
        register_all(CameoClient::builder(), fake)
            .with_cache_backend(backend)
            .with_cache_ttl(ttls())
            .build()
            .expect("configured")
    }

    fn a_id(n: u64) -> MediaId {
        MediaId::new("a", NativeId::Num(n))
    }

    fn item(media_type: MediaType, id: &str) -> CacheKey {
        CacheKey::Item {
            media_type,
            provider_id: id.to_string(),
        }
    }

    fn detail(media_type: MediaType, id: &str) -> CacheKey {
        CacheKey::Detail {
            media_type,
            provider_id: id.to_string(),
        }
    }

    /// Each cache category is written with its own configured TTL, and list
    /// results also index each item with the item TTL.
    #[tokio::test]
    async fn each_category_is_written_with_its_own_ttl() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let client = client(&fake, backend.clone());

        client
            .search_movies("  Dune ", Some(2))
            .await
            .expect("search");
        client.movie_details(&a_id(9)).await.expect("details");
        client
            .trending_movies(TimeWindow::Week, None)
            .await
            .expect("trending");
        client
            .movie_recommendations(&a_id(9), None)
            .await
            .expect("recommendations");

        let s = Duration::from_secs;
        let search_key = CacheKey::Search {
            provider: "a".to_string(),
            media_type: Some(MediaType::Movie),
            query: "  Dune ".to_string(),
            page: 2,
        };
        let writes = backend.writes();
        let expected = vec![
            (search_key, s(22)),
            (item(MediaType::Movie, "a:1"), s(44)),
            (detail(MediaType::Movie, "a:9"), s(11)),
            (item(MediaType::Movie, "a:9"), s(44)),
            (
                CacheKey::Discovery {
                    provider: "a".to_string(),
                    endpoint: "trending_movies_week".to_string(),
                    page: 1,
                },
                s(33),
            ),
            (item(MediaType::Movie, "a:1"), s(44)),
            (
                CacheKey::Discovery {
                    provider: "a".to_string(),
                    endpoint: "movie_recommendations_a:9".to_string(),
                    page: 1,
                },
                s(33),
            ),
            (item(MediaType::Movie, "a:1"), s(44)),
        ];
        assert_eq!(writes, expected);
    }

    /// A hit is served from the cache without calling the provider.
    #[tokio::test]
    async fn a_hit_skips_the_provider() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let client = client(&fake, backend);

        let first = client.movie_details(&a_id(9)).await.expect("details");
        let second = client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(first, second);
        assert_eq!(fake.calls(), ["a.movie_details"]);
    }

    /// A failing read is a miss: the provider answers and the call succeeds.
    #[tokio::test]
    async fn a_failing_read_is_a_silent_miss() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording {
            fail_get: true,
            ..Recording::default()
        });
        let client = client(&fake, backend.clone());

        client.movie_details(&a_id(9)).await.expect("details");
        client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(fake.calls(), ["a.movie_details", "a.movie_details"]);
        assert_eq!(client.cached_movie_details(&a_id(9)).await, None);
    }

    /// A failing write is swallowed: the call still returns the provider's
    /// result.
    #[tokio::test]
    async fn a_failing_write_is_silent() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording {
            fail_set: true,
            ..Recording::default()
        });
        let client = client(&fake, backend.clone());

        let page = client.search_movies("q", None).await.expect("search");
        assert_eq!(page.results[0].title, "a");
        assert_eq!(backend.writes().len(), 2, "search page and its one item");
        assert_eq!(client.cached_movie(&a_id(1)).await, None);
    }

    /// A stored value of the wrong shape is a miss, and the fresh result
    /// overwrites it.
    #[tokio::test]
    async fn an_undeserializable_entry_is_a_miss_and_is_overwritten() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let key = detail(MediaType::Movie, "a:9");
        backend.put(key.clone(), serde_json::json!({ "not": "a movie" }));
        let client = client(&fake, backend.clone());

        let details = client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(fake.calls(), ["a.movie_details"]);
        assert_eq!(
            backend.value(&key),
            Some(serde_json::to_value(&details).expect("serializes"))
        );
    }

    /// `cached_*` prefers the item entry, falls back to the detail entry's
    /// embedded model, and is `None` when neither is present.
    #[tokio::test]
    async fn cached_lookups_fall_back_from_item_to_detail() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let client = client(&fake, backend.clone());
        fn json<T: serde::Serialize>(value: &T) -> serde_json::Value {
            serde_json::to_value(value).expect("serializes")
        }

        let movie_details = UnifiedMovieDetails::new(UnifiedMovie::new(a_id(1), "from detail"));
        let show_details = UnifiedTvShowDetails::new(UnifiedTvShow::new(a_id(2), "from detail"));
        let person_details = UnifiedPersonDetails::new(UnifiedPerson::new(a_id(3), "from detail"));
        backend.put(detail(MediaType::Movie, "a:1"), json(&movie_details));
        backend.put(detail(MediaType::TvShow, "a:2"), json(&show_details));
        backend.put(detail(MediaType::Person, "a:3"), json(&person_details));

        assert_eq!(
            client.cached_movie(&a_id(1)).await,
            Some(movie_details.movie)
        );
        assert_eq!(
            client.cached_tv_show(&a_id(2)).await,
            Some(show_details.show)
        );
        assert_eq!(
            client.cached_person(&a_id(3)).await,
            Some(person_details.person)
        );

        let movie_item = UnifiedMovie::new(a_id(1), "from item");
        let show_item = UnifiedTvShow::new(a_id(2), "from item");
        let person_item = UnifiedPerson::new(a_id(3), "from item");
        backend.put(item(MediaType::Movie, "a:1"), json(&movie_item));
        backend.put(item(MediaType::TvShow, "a:2"), json(&show_item));
        backend.put(item(MediaType::Person, "a:3"), json(&person_item));

        assert_eq!(client.cached_movie(&a_id(1)).await, Some(movie_item));
        assert_eq!(client.cached_tv_show(&a_id(2)).await, Some(show_item));
        assert_eq!(client.cached_person(&a_id(3)).await, Some(person_item));

        assert_eq!(client.cached_movie(&a_id(404)).await, None);
        assert_eq!(client.cached_tv_show_details(&a_id(404)).await, None);
        assert_eq!(client.cached_person_details(&a_id(404)).await, None);
        assert!(fake.calls().is_empty(), "lookups never call a provider");
    }

    /// `search_multi` indexes the model each result wraps, so `cached_*`
    /// returns it, and it does not clobber an item an earlier typed search
    /// stored.
    #[tokio::test]
    async fn search_multi_indexes_the_wrapped_models() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let client = client(&fake, backend.clone());

        client.search_movies("q", None).await.expect("search");
        let typed = client.cached_movie(&a_id(1)).await;
        assert_eq!(typed, Some(UnifiedMovie::new(a_id(1), "a")));

        client.search_multi("q", None).await.expect("search");

        assert_eq!(client.cached_movie(&a_id(1)).await, typed);
        assert_eq!(
            client.cached_tv_show(&a_id(2)).await,
            Some(UnifiedTvShow::new(a_id(2), "a"))
        );
        assert_eq!(
            client.cached_person(&a_id(3)).await,
            Some(UnifiedPerson::new(a_id(3), "a"))
        );
        let ttl = Duration::from_secs(44);
        let multi_writes = backend.writes().split_off(2);
        assert_eq!(
            multi_writes[1..],
            [
                (item(MediaType::Movie, "a:1"), ttl),
                (item(MediaType::TvShow, "a:2"), ttl),
                (item(MediaType::Person, "a:3"), ttl),
            ]
        );
    }

    /// Per-call `*_with` searches neither read nor write the cache.
    #[tokio::test]
    async fn search_with_bypasses_the_cache() {
        let fake = Fake::new("a");
        let backend = Arc::new(Recording::default());
        let client = client(&fake, backend.clone());

        client.search_movies("q", None).await.expect("search");
        client
            .search_movies_with("a", "q", None)
            .await
            .expect("search");
        client
            .search_tv_shows_with("a", "q", None)
            .await
            .expect("search");
        client
            .search_people_with("a", "q", None)
            .await
            .expect("search");
        client
            .search_multi_with("a", "q", None)
            .await
            .expect("search");

        assert_eq!(
            fake.calls(),
            [
                "a.search_movies",
                "a.search_movies",
                "a.search_tv_shows",
                "a.search_people",
                "a.search_multi",
            ],
            "the cached search page does not answer search_movies_with"
        );
        assert_eq!(
            backend.writes().len(),
            2,
            "only the default search wrote (page + item)"
        );
    }

    /// A per-test directory under the temporary directory, removed on drop.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("cameo-facade-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("temp dir");
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `with_cache_at` creates missing parent directories, opens the SQLite
    /// database there, and keeps the providers registered before it.
    #[tokio::test]
    async fn with_cache_at_opens_the_database_at_the_path() {
        let dir = TempDir::new("with-cache-at");
        let path = dir.0.join("nested").join("cache.db");
        let fake = Fake::new("a");
        let client = register_all(CameoClient::builder(), &fake)
            .with_cache_at(&path)
            .build()
            .expect("providers registered before with_cache_at are kept");

        assert!(path.is_file(), "database created at {}", path.display());
        client.movie_details(&a_id(9)).await.expect("details");
        client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(fake.calls(), ["a.movie_details"]);
    }

    /// When the database cannot be opened, an in-memory cache answers instead.
    #[tokio::test]
    async fn with_cache_at_falls_back_to_memory_when_the_path_cannot_open() {
        let dir = TempDir::new("with-cache-at-fallback");
        let blocker = dir.0.join("not-a-dir");
        std::fs::write(&blocker, b"").expect("blocker file");
        let path = blocker.join("cache.db");
        let fake = Fake::new("a");
        let client = register_all(CameoClient::builder(), &fake)
            .with_cache_at(&path)
            .build()
            .expect("configured");

        assert!(!path.exists());
        client.movie_details(&a_id(9)).await.expect("details");
        client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(fake.calls(), ["a.movie_details"], "served from memory");
    }

    /// Without a backend nothing is cached and every lookup is `None`.
    #[tokio::test]
    async fn no_backend_means_no_caching() {
        let fake = Fake::new("a");
        let client = register_all(CameoClient::builder(), &fake)
            .build()
            .expect("configured");
        client.movie_details(&a_id(9)).await.expect("details");
        client.movie_details(&a_id(9)).await.expect("details");
        assert_eq!(fake.calls(), ["a.movie_details", "a.movie_details"]);
        assert_eq!(client.cached_movie(&a_id(9)).await, None);
        client.invalidate_cached(&a_id(9)).await;
        client.clear_cache().await;
    }
}
