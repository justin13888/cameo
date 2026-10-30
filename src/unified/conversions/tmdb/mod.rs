//! Conversions from TMDB generated types to unified models.
//!
//! Generated TMDB response fields are `Option<T>`, since TMDB may omit them or
//! send an explicit `null`. The conversions keep that optionality honest:
//!
//! - Unified `Option` fields (counts, scores, flags such as `adult`, `video`,
//!   `in_production`, nested company/network/creator ids) are `None` when TMDB
//!   omits the field or sends `null`; a value TMDB actually sends (including
//!   `0`) is kept. Boolean flags whose spec default is `true` therefore never
//!   read as `true` when unknown.
//! - Non-`Option` unified numbers (episode/season numbers) fall back to `0`.
//! - Genre entries without an id are dropped.
//! - A missing top-level id: the `From` impls fall back to `tmdb:0`, but
//!   [`TmdbClient`](crate::providers::tmdb::TmdbClient) never relies on that —
//!   details requests fill in the requested id before converting, and list or
//!   search items without an id are dropped (see [`TmdbListItem`]).

pub mod movie;
pub mod person;
pub mod tv;

use crate::{
    generated::tmdb::types,
    providers::tmdb::image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize},
    unified::date::PartialDate,
};

/// A TMDB list/search result item whose TMDB id may be absent.
///
/// An item without an id cannot be fetched again or cached under a stable key
/// (it would collide with every other id-less item), so the client drops such
/// items from result pages.
pub(crate) trait TmdbListItem {
    /// The item's TMDB id, if TMDB reported one.
    fn tmdb_id(&self) -> Option<i64>;
}

macro_rules! impl_tmdb_list_item {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TmdbListItem for $ty {
                fn tmdb_id(&self) -> Option<i64> {
                    self.id
                }
            }
        )*
    };
}

impl_tmdb_list_item!(
    types::SearchMovieResponseResultsItem,
    types::TrendingMoviesResponseResultsItem,
    types::MoviePopularListResponseResultsItem,
    types::MovieTopRatedListResponseResultsItem,
    types::DiscoverMovieResponseResultsItem,
    types::MovieSimilarResponseResultsItem,
    types::SearchTvResponseResultsItem,
    types::TrendingTvResponseResultsItem,
    types::TvSeriesRecommendationsResponseResultsItem,
    types::TvSeriesSimilarResponseResultsItem,
    types::DiscoverTvResponseResultsItem,
    types::TvSeriesPopularListResponseResultsItem,
    types::TvSeriesTopRatedListResponseResultsItem,
    types::SearchPersonResponseResultsItem,
    types::SearchMultiResponseResultsItem,
);

pub(super) fn resolve_logo(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::logo(p, LogoSize::W92))
}

/// Parse a TMDB date string (`YYYY-MM-DD`, occasionally empty or malformed)
/// into a [`PartialDate`], dropping empty/invalid values to `None`.
pub(super) fn resolve_date(raw: &Option<String>) -> Option<PartialDate> {
    raw.as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}

pub(super) fn resolve_poster(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::poster(p, PosterSize::W500))
}

pub(super) fn resolve_backdrop(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::backdrop(p, BackdropSize::W780))
}

pub(super) fn resolve_profile(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::profile(p, ProfileSize::H632))
}
