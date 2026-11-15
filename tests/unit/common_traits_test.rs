//! The common trait impls (Rust API Guidelines C-COMMON-TRAITS) sibling public
//! types share: `Clone`/`PartialEq` on configs and errors, `Clone` on the
//! facade, `Debug`/`Copy` on `ImageUrl`, and serde on the small public enums.

use std::time::Duration;

use cameo::{CameoClient, Capability, ProviderError};

/// Compile-time check that `T` implements the listed traits.
fn assert_traits<T: Clone + PartialEq + Eq + std::fmt::Debug>() {}

#[test]
fn cameo_client_is_clone_send_sync() {
    fn assert_clone_send_sync<T: Clone + Send + Sync>() {}
    assert_clone_send_sync::<CameoClient>();
}

#[test]
fn provider_error_is_clone_and_eq() {
    assert_traits::<ProviderError>();
    let err = ProviderError::RateLimited {
        retry_after: Some(Duration::from_secs(2)),
    };
    assert_eq!(err.clone(), err);
    assert_ne!(
        ProviderError::Api {
            status: 500,
            message: "a".into()
        },
        ProviderError::Api {
            status: 500,
            message: "b".into()
        }
    );
    assert_ne!(ProviderError::NotFound, ProviderError::Unsupported);
}

#[test]
fn capability_serializes_as_snake_case() {
    let json = serde_json::to_string(&Capability::WatchAvailability).expect("serialize");
    assert_eq!(json, r#""watch_availability""#);
    for cap in [
        Capability::Search,
        Capability::Details,
        Capability::Discovery,
        Capability::Recommendations,
        Capability::Seasons,
        Capability::WatchAvailability,
        Capability::ExternalIds,
        Capability::Credits,
    ] {
        let json = serde_json::to_string(&cap).expect("serialize");
        let back: Capability = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, cap);
    }
    assert_eq!(
        serde_json::from_str::<Capability>(r#""external_ids""#).expect("deserialize"),
        Capability::ExternalIds
    );
    assert!(serde_json::from_str::<Capability>(r#""Search""#).is_err());
}

#[cfg(feature = "cache")]
#[test]
fn media_type_serializes_as_cache_key_string() {
    use cameo::{CacheKey, MediaType};
    for (media_type, expected) in [
        (MediaType::Movie, "movie"),
        (MediaType::TvShow, "tv"),
        (MediaType::Person, "person"),
    ] {
        let json = serde_json::to_string(&media_type).expect("serialize");
        assert_eq!(json, format!("\"{expected}\""));
        let back: MediaType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, media_type);
        // The serde form matches the string the cache key embeds.
        let key = CacheKey::Detail {
            media_type,
            provider_id: "tmdb:1".into(),
        };
        assert_eq!(key.key_id(), format!("{expected}:tmdb:1"));
    }
}

#[cfg(feature = "cache")]
#[test]
fn cache_ttl_config_is_eq() {
    use cameo::CacheTtlConfig;
    assert_traits::<CacheTtlConfig>();
    let a = CacheTtlConfig::default();
    let mut b = a.clone();
    assert_eq!(a, b);
    b.search = Duration::from_secs(1);
    assert_ne!(a, b);
}

#[cfg(feature = "anilist")]
#[test]
fn anilist_config_is_eq() {
    use cameo::AniListConfig;
    assert_traits::<AniListConfig>();
    let a = AniListConfig::default();
    let mut b = a.clone();
    assert_eq!(a, b);
    b.per_page = 7;
    assert_ne!(a, b);
}

#[cfg(feature = "tmdb")]
#[test]
fn tmdb_config_is_eq() {
    use cameo::{TmdbAuth, TmdbConfig};
    assert_traits::<TmdbConfig>();
    assert_traits::<TmdbAuth>();
    assert_eq!(TmdbConfig::new("tok"), TmdbConfig::new("tok"));
    assert_ne!(TmdbConfig::new("tok"), TmdbConfig::new("other"));
    // Same credential, different scheme: not equal.
    assert_ne!(TmdbConfig::new("tok"), TmdbConfig::new_v3("tok"));
    assert_ne!(
        TmdbConfig::new("tok"),
        TmdbConfig::new("tok").with_language("fr-FR")
    );
}

#[cfg(feature = "tmdb")]
#[test]
fn image_url_has_common_traits() {
    use cameo::providers::tmdb::image_url::ImageUrl;
    fn assert_copy_hash<T: Copy + std::hash::Hash>() {}
    assert_traits::<ImageUrl>();
    assert_copy_hash::<ImageUrl>();
    assert_eq!(format!("{ImageUrl:?}"), "ImageUrl");
}

#[cfg(feature = "anilist")]
#[test]
fn cameo_client_clone_keeps_providers() {
    let client = CameoClient::builder()
        .with_anilist(cameo::AniListConfig::default())
        .build()
        .expect("build");
    let clone = client.clone();
    assert_eq!(clone.provider_ids(), client.provider_ids());
}

#[cfg(all(feature = "anilist", feature = "cache"))]
#[test]
fn cameo_client_with_cache_is_clone() {
    let client = CameoClient::builder()
        .with_anilist(cameo::AniListConfig::default())
        .with_cache_backend(std::sync::Arc::new(
            cameo::SqliteCache::in_memory().expect("in-memory cache"),
        ))
        .build()
        .expect("build");
    let clone = client.clone();
    assert_eq!(clone.provider_ids(), client.provider_ids());
}
