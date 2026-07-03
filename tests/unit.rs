#![allow(missing_docs)]
mod unit {
    #[cfg(feature = "anilist")]
    mod anilist_test;
    #[cfg(feature = "tmdb")]
    mod builder_test;
    #[cfg(feature = "cache")]
    mod cache_test;
    mod genre_test;
    #[cfg(feature = "tmdb")]
    mod image_url_test;
    mod model_builders_test;
    mod pagination_test;
}
