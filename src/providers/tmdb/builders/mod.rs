/// Movie discovery builder.
pub mod discover_movies;
/// TV show discovery builder.
pub mod discover_tv;
/// Sort-order enums for the discover builders.
pub mod sort;

pub use discover_movies::DiscoverMoviesBuilder;
pub use discover_tv::DiscoverTvBuilder;
pub use sort::{MovieSortBy, TvSortBy};
