//! Stable, hand-owned sort-order enums for the discover builders.
//!
//! These wrap the generated `DiscoverMovieSortBy` / `DiscoverTvSortBy` enums so
//! the public discover API does not depend on generated names.

use crate::generated::tmdb::types::{DiscoverMovieSortBy, DiscoverTvSortBy};

/// Sort order for [`DiscoverMoviesBuilder`](super::DiscoverMoviesBuilder).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovieSortBy {
    /// Original title, ascending.
    OriginalTitleAsc,
    /// Original title, descending.
    OriginalTitleDesc,
    /// Popularity, ascending.
    PopularityAsc,
    /// Popularity, descending.
    PopularityDesc,
    /// Revenue, ascending.
    RevenueAsc,
    /// Revenue, descending.
    RevenueDesc,
    /// Primary release date, ascending.
    PrimaryReleaseDateAsc,
    /// Primary release date, descending.
    PrimaryReleaseDateDesc,
    /// Title, ascending.
    TitleAsc,
    /// Title, descending.
    TitleDesc,
    /// Average vote, ascending.
    VoteAverageAsc,
    /// Average vote, descending.
    VoteAverageDesc,
    /// Vote count, ascending.
    VoteCountAsc,
    /// Vote count, descending.
    VoteCountDesc,
}

impl MovieSortBy {
    pub(crate) fn to_generated(self) -> DiscoverMovieSortBy {
        match self {
            MovieSortBy::OriginalTitleAsc => DiscoverMovieSortBy::OriginalTitleAsc,
            MovieSortBy::OriginalTitleDesc => DiscoverMovieSortBy::OriginalTitleDesc,
            MovieSortBy::PopularityAsc => DiscoverMovieSortBy::PopularityAsc,
            MovieSortBy::PopularityDesc => DiscoverMovieSortBy::PopularityDesc,
            MovieSortBy::RevenueAsc => DiscoverMovieSortBy::RevenueAsc,
            MovieSortBy::RevenueDesc => DiscoverMovieSortBy::RevenueDesc,
            MovieSortBy::PrimaryReleaseDateAsc => DiscoverMovieSortBy::PrimaryReleaseDateAsc,
            MovieSortBy::PrimaryReleaseDateDesc => DiscoverMovieSortBy::PrimaryReleaseDateDesc,
            MovieSortBy::TitleAsc => DiscoverMovieSortBy::TitleAsc,
            MovieSortBy::TitleDesc => DiscoverMovieSortBy::TitleDesc,
            MovieSortBy::VoteAverageAsc => DiscoverMovieSortBy::VoteAverageAsc,
            MovieSortBy::VoteAverageDesc => DiscoverMovieSortBy::VoteAverageDesc,
            MovieSortBy::VoteCountAsc => DiscoverMovieSortBy::VoteCountAsc,
            MovieSortBy::VoteCountDesc => DiscoverMovieSortBy::VoteCountDesc,
        }
    }
}

/// Sort order for [`DiscoverTvBuilder`](super::DiscoverTvBuilder).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TvSortBy {
    /// First air date, ascending.
    FirstAirDateAsc,
    /// First air date, descending.
    FirstAirDateDesc,
    /// Name, ascending.
    NameAsc,
    /// Name, descending.
    NameDesc,
    /// Original name, ascending.
    OriginalNameAsc,
    /// Original name, descending.
    OriginalNameDesc,
    /// Popularity, ascending.
    PopularityAsc,
    /// Popularity, descending.
    PopularityDesc,
    /// Average vote, ascending.
    VoteAverageAsc,
    /// Average vote, descending.
    VoteAverageDesc,
    /// Vote count, ascending.
    VoteCountAsc,
    /// Vote count, descending.
    VoteCountDesc,
}

impl TvSortBy {
    pub(crate) fn to_generated(self) -> DiscoverTvSortBy {
        match self {
            TvSortBy::FirstAirDateAsc => DiscoverTvSortBy::FirstAirDateAsc,
            TvSortBy::FirstAirDateDesc => DiscoverTvSortBy::FirstAirDateDesc,
            TvSortBy::NameAsc => DiscoverTvSortBy::NameAsc,
            TvSortBy::NameDesc => DiscoverTvSortBy::NameDesc,
            TvSortBy::OriginalNameAsc => DiscoverTvSortBy::OriginalNameAsc,
            TvSortBy::OriginalNameDesc => DiscoverTvSortBy::OriginalNameDesc,
            TvSortBy::PopularityAsc => DiscoverTvSortBy::PopularityAsc,
            TvSortBy::PopularityDesc => DiscoverTvSortBy::PopularityDesc,
            TvSortBy::VoteAverageAsc => DiscoverTvSortBy::VoteAverageAsc,
            TvSortBy::VoteAverageDesc => DiscoverTvSortBy::VoteAverageDesc,
            TvSortBy::VoteCountAsc => DiscoverTvSortBy::VoteCountAsc,
            TvSortBy::VoteCountDesc => DiscoverTvSortBy::VoteCountDesc,
        }
    }
}
