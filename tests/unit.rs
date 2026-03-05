mod unit {
    #[cfg(feature = "anilist")]
    mod anilist_test;
    #[cfg(feature = "tmdb")]
    mod builder_test;
    #[cfg(feature = "cache")]
    mod cache_test;
    #[cfg(feature = "tmdb")]
    mod conversion_test;
    mod genre_test;
    #[cfg(feature = "tmdb")]
    mod image_url_test;
    mod pagination_test;
}
