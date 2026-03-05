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
    mod tmdb_movies_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_people_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_search_test;
    #[cfg(feature = "tmdb")]
    mod tmdb_tv_test;
}
