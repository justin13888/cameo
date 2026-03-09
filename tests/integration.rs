mod integration {
    #[cfg(feature = "anilist")]
    mod anilist_details_test;
    #[cfg(feature = "anilist")]
    mod anilist_discovery_test;
    #[cfg(feature = "anilist")]
    mod anilist_search_test;
    #[cfg(feature = "cache")]
    mod cache_integration_test;
    #[cfg(feature = "tmdb")]
    mod facade_dispatch_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_credits_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_discovery_movies_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_discovery_tv_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_episode_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_error_scenarios_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_movies_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_people_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_recommendations_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_search_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_season_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_similar_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_tv_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_watch_providers_test;
}
