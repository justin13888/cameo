//! TMDB offset-paginated list responses → [`Page`].

use crate::{
    core::pagination::Page, generated::tmdb::types, providers::tmdb::error::TmdbError,
    unified::conversions::native_id,
};

/// The highest page number TMDB serves for any list endpoint.
///
/// TMDB rejects `page` above this, and the client clamps requested pages to
/// `1..=MAX_PAGE`, so no page past it can ever be fetched.
pub(crate) const MAX_PAGE: u32 = 500;

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
                    self.id.as_option().copied()
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

/// A generated TMDB list response carrying offset-pagination fields
/// (`page`, `results`, `total_pages`, `total_results`).
pub(crate) trait TmdbPaged {
    /// The generated result item type.
    type Item: TmdbListItem;

    /// Convert the response into a [`Page`] of its raw items, as
    /// [`offset_page`] does with this response's fields.
    ///
    /// `requested` is the page number sent to TMDB (`None` when no page was
    /// sent).
    ///
    /// # Errors
    ///
    /// [`TmdbError::PageOutOfRange`] when a reported page or total does not
    /// fit a `u32`.
    fn into_page(self, requested: Option<i32>) -> Result<Page<Self::Item>, TmdbError>;
}

macro_rules! impl_tmdb_paged {
    ($($resp:ty => $item:ty),* $(,)?) => {
        $(
            impl TmdbPaged for $resp {
                type Item = $item;

                fn into_page(self, requested: Option<i32>) -> Result<Page<$item>, TmdbError> {
                    offset_page(
                        requested,
                        self.page.into_option(),
                        self.results.into_option(),
                        self.total_pages.into_option(),
                        self.total_results.into_option(),
                    )
                }
            }
        )*
    };
}

impl_tmdb_paged!(
    types::SearchMovieResponse => types::SearchMovieResponseResultsItem,
    types::SearchTvResponse => types::SearchTvResponseResultsItem,
    types::SearchPersonResponse => types::SearchPersonResponseResultsItem,
    types::SearchMultiResponse => types::SearchMultiResponseResultsItem,
    types::TrendingMoviesResponse => types::TrendingMoviesResponseResultsItem,
    types::TrendingTvResponse => types::TrendingTvResponseResultsItem,
    types::MoviePopularListResponse => types::MoviePopularListResponseResultsItem,
    types::MovieTopRatedListResponse => types::MovieTopRatedListResponseResultsItem,
    types::TvSeriesPopularListResponse => types::TvSeriesPopularListResponseResultsItem,
    types::TvSeriesTopRatedListResponse => types::TvSeriesTopRatedListResponseResultsItem,
    types::MovieSimilarResponse => types::MovieSimilarResponseResultsItem,
    types::TvSeriesRecommendationsResponse => types::TvSeriesRecommendationsResponseResultsItem,
    types::TvSeriesSimilarResponse => types::TvSeriesSimilarResponseResultsItem,
    types::DiscoverMovieResponse => types::DiscoverMovieResponseResultsItem,
    types::DiscoverTvResponse => types::DiscoverTvResponseResultsItem,
);

/// Build a [`Page`] from TMDB offset-pagination fields, tolerating `null`s,
/// and drop result items without a valid TMDB id (see [`TmdbListItem`]).
///
/// `requested` is the page number sent to TMDB (`None` when no page was sent).
/// A missing `page` falls back to `requested` (or `1`). A missing
/// `total_pages` or `total_results` is unknown and reads as `0`, as [`Page`]
/// documents, so `has_next` (`page < total_pages`) is `false`. A reported
/// `total_pages` above [`MAX_PAGE`] is capped at it, so the page the client
/// cannot request is never advertised as next; `total_results` is kept as
/// reported. Missing `results` read as an empty list. An item whose id is
/// missing, zero, or negative is dropped and logged at `warn`.
///
/// # Errors
///
/// [`TmdbError::PageOutOfRange`] when TMDB reports a `page`, `total_pages`, or
/// `total_results` that does not fit a `u32` (negative or too large).
fn offset_page<T: TmdbListItem>(
    requested: Option<i32>,
    page: Option<i64>,
    results: Option<Vec<T>>,
    total_pages: Option<i64>,
    total_results: Option<i64>,
) -> Result<Page<T>, TmdbError> {
    let requested = requested.and_then(|p| u32::try_from(p).ok()).unwrap_or(1);
    let page = page_field("page", page)?.unwrap_or(requested);
    let total_pages = page_field("total_pages", total_pages)?
        .unwrap_or(0)
        .min(MAX_PAGE);
    let total_results = page_field("total_results", total_results)?.unwrap_or(0);
    let results = results
        .unwrap_or_default()
        .into_iter()
        .filter(|item| {
            let valid = item.tmdb_id().and_then(native_id).is_some();
            if !valid {
                tracing::warn!(
                    tmdb_id = item.tmdb_id(),
                    "tmdb: skipping result without a valid TMDB id"
                );
            }
            valid
        })
        .collect();
    Ok(Page::offset(page, results, total_pages, total_results))
}

/// Mark `page` as the only page of its result set: no next page, and
/// `total_pages` of `1`. `total_results` is kept as reported.
///
/// For endpoints that ignore the requested page (TMDB trending), where any
/// `total_pages` TMDB reports could never be walked: requesting page 2 returns
/// page 1 again.
pub(crate) fn single_page<T>(mut page: Page<T>) -> Page<T> {
    page.has_next = false;
    page.next = None;
    page.total_pages = 1;
    page
}

/// A reported pagination `field` as a `u32`: `None` when TMDB omitted it, and
/// [`TmdbError::PageOutOfRange`] when the value does not fit.
fn page_field(field: &'static str, value: Option<i64>) -> Result<Option<u32>, TmdbError> {
    value
        .map(|v| u32::try_from(v).map_err(|_| TmdbError::PageOutOfRange { field, value: v }))
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A list item with an optional id, standing in for a generated TMDB item.
    #[derive(Debug)]
    struct Item(Option<i64>);

    impl TmdbListItem for Item {
        fn tmdb_id(&self) -> Option<i64> {
            self.0
        }
    }

    /// Omitted totals are unknown and read as `0`, as `Page` documents, and
    /// leave no next page; an omitted page falls back to the requested one.
    #[test]
    fn offset_page_omitted_totals_read_as_zero() {
        let page = offset_page(Some(2), None, Some(vec![Item(Some(1))]), None, None)
            .expect("page in range");
        assert_eq!(page.page, 2);
        assert_eq!(page.total_pages, 0);
        assert_eq!(page.total_results, 0);
        assert!(!page.has_next);
        assert_eq!(page.next, None);
        assert_eq!(page.results.len(), 1);
    }

    /// Reported totals pass through and drive `has_next`; items without a
    /// valid id (missing, zero, or negative) drop.
    #[test]
    fn offset_page_reported_totals_pass_through() {
        let page = offset_page(
            None,
            Some(1),
            Some(vec![
                Item(Some(1)),
                Item(None),
                Item(Some(0)),
                Item(Some(-7)),
            ]),
            Some(3),
            Some(41),
        )
        .expect("page in range");
        assert_eq!(page.page, 1);
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.total_results, 41);
        assert!(page.has_next);
        assert_eq!(page.results.len(), 1);
    }

    /// A `total_pages` past TMDB's page limit is capped at it: page
    /// `MAX_PAGE` is the last page, and earlier pages still have a next one.
    /// `total_results` is kept as reported.
    #[test]
    fn offset_page_caps_total_pages_at_the_page_limit() {
        let last = offset_page::<Item>(None, Some(500), None, Some(48_211), Some(964_210))
            .expect("page in range");
        assert_eq!(last.total_pages, MAX_PAGE);
        assert_eq!(last.total_results, 964_210);
        assert!(!last.has_next);
        assert_eq!(last.next, None);

        let before =
            offset_page::<Item>(None, Some(499), None, Some(48_211), None).expect("page in range");
        assert_eq!(before.total_pages, MAX_PAGE);
        assert_eq!(before.next_page(), Some(500));
    }

    /// `single_page` leaves no next page whatever totals TMDB reported.
    #[test]
    fn single_page_reports_no_next_page() {
        let page =
            offset_page::<Item>(None, Some(1), None, Some(37), Some(740)).expect("page in range");
        assert!(page.has_next);

        let page = single_page(page);
        assert!(!page.has_next);
        assert_eq!(page.next, None);
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.total_results, 740);
    }

    /// A page number or total that does not fit a `u32` is a typed error
    /// naming the field and value, never a wrapped or clamped number.
    #[test]
    fn offset_page_out_of_range_fields_are_errors() {
        let too_big = i64::from(u32::MAX) + 1;
        let cases = [
            ("page", (Some(-1), None, None)),
            ("page", (Some(too_big), None, None)),
            ("total_pages", (Some(1), Some(-1), None)),
            ("total_pages", (Some(1), Some(too_big), None)),
            ("total_results", (Some(1), Some(1), Some(-1))),
            ("total_results", (Some(1), Some(1), Some(too_big))),
        ];
        for (expected, (page, total_pages, total_results)) in cases {
            let err = offset_page::<Item>(None, page, None, total_pages, total_results)
                .expect_err("out-of-range field");
            match err {
                TmdbError::PageOutOfRange { field, value } => {
                    assert_eq!(field, expected);
                    assert!(value < 0 || value == too_big, "{value}");
                }
                other => panic!("expected PageOutOfRange, got {other:?}"),
            }
        }
        // `u32::MAX` itself still fits.
        let max = i64::from(u32::MAX);
        let page = offset_page::<Item>(None, Some(max), None, Some(max), Some(max))
            .expect("u32::MAX fits");
        assert_eq!(page.page, u32::MAX);
        assert_eq!(page.total_results, u32::MAX);
    }

    /// `into_page` reads a generated response's own pagination fields and
    /// drops its id-less items, as `offset_page` does.
    #[test]
    fn into_page_reads_the_response_fields() {
        let raw: types::SearchMovieResponse = serde_json::from_value(json!({
            "page": 2,
            "results": [{ "id": 550 }, { "title": "No id" }],
            "total_pages": 5,
            "total_results": 90
        }))
        .expect("search response deserializes");

        let page = raw.into_page(Some(2)).expect("page in range");

        assert_eq!(page.page, 2);
        assert_eq!(page.total_pages, 5);
        assert_eq!(page.total_results, 90);
        assert!(page.has_next);
        let ids: Vec<_> = page
            .results
            .iter()
            .map(|m| m.id.as_option().copied())
            .collect();
        assert_eq!(ids, [Some(550)]);
    }

    /// A response with no pagination fields falls back to the requested page.
    #[test]
    fn into_page_falls_back_to_the_requested_page() {
        let raw: types::DiscoverTvResponse =
            serde_json::from_value(json!({})).expect("empty response deserializes");

        let page = raw.into_page(Some(3)).expect("page in range");

        assert_eq!(page.page, 3);
        assert!(page.results.is_empty());
        assert!(!page.has_next);
    }
}
