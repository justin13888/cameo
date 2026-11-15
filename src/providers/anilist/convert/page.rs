//! Paging variables and the conversion of AniList pages into [`Page`]s.

use super::anilist_media_to_search_result;
use crate::{
    core::pagination::Page,
    providers::anilist::{
        error::AniListError,
        response::{
            AniListMedia, MediaPageData, MediaPageResponse, PageInfo, RecommendationConnection,
        },
    },
    unified::models::UnifiedSearchResult,
};

/// The `page` and `perPage` variables of a list request: `page` defaults to
/// `1`.
pub(crate) fn page_vars(page: Option<u32>, per_page: u32) -> (i64, i64) {
    (i64::from(page.unwrap_or(1)), i64::from(per_page))
}

/// Root-field extractor for queries returning a media `Page`.
pub(crate) fn media_page_root(resp: MediaPageResponse) -> Option<MediaPageData> {
    resp.page
}

/// Convert a media page with `convert`, dropping the entries it rejects.
///
/// # Errors
///
/// [`AniListError::NoData`] when the page is missing, and
/// [`AniListError::PageOutOfRange`] as [`page_from`] reports it.
pub(crate) fn media_page<U>(
    page: Option<MediaPageData>,
    convert: fn(AniListMedia) -> Option<U>,
) -> Result<Page<U>, AniListError> {
    let page = page.ok_or(AniListError::NoData)?;
    let results = page.media.into_iter().filter_map(convert).collect();
    page_from(&page.page_info, results)
}

/// Convert a recommendation connection to a page of classified media,
/// skipping null nodes, null recommended media, and entries whose format is
/// absent or that [`classify`](super::classify) places in neither list (e.g.
/// `MUSIC`). A missing connection is an empty first page whose totals are
/// unknown (`0`).
pub(crate) fn recommendations_page(
    conn: Option<RecommendationConnection>,
) -> Result<Page<UnifiedSearchResult>, AniListError> {
    let Some(conn) = conn else {
        return Ok(Page::with_has_next(1, Vec::new(), false, 0, 0));
    };
    let pi = &conn.page_info;
    let results = conn
        .nodes
        .into_iter()
        .flatten()
        .filter_map(|node| node.media_recommendation)
        .filter_map(anilist_media_to_search_result)
        .collect();
    page_from(pi, results)
}

/// Build a [`Page`] from AniList's `pageInfo`.
///
/// A missing `currentPage` reads as `1`, and a missing `lastPage` or `total`
/// as `0` (unknown), as [`Page`] documents.
///
/// # Errors
///
/// [`AniListError::PageOutOfRange`] when AniList reports a negative
/// `currentPage`, `lastPage`, or `total`.
pub(crate) fn page_from<T>(pi: &PageInfo, results: Vec<T>) -> Result<Page<T>, AniListError> {
    Ok(Page::with_has_next(
        page_field("currentPage", pi.current_page)?.unwrap_or(1),
        results,
        pi.has_next_page,
        page_field("lastPage", pi.last_page)?.unwrap_or(0),
        page_field("total", pi.total)?.unwrap_or(0),
    ))
}

/// A reported `pageInfo` field as a `u32`: `None` when AniList omitted it,
/// and [`AniListError::PageOutOfRange`] when it is negative.
fn page_field(field: &'static str, value: Option<i32>) -> Result<Option<u32>, AniListError> {
    value
        .map(|v| {
            u32::try_from(v).map_err(|_| AniListError::PageOutOfRange {
                field,
                value: i64::from(v),
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::providers::anilist::convert::{anilist_media_to_movie, anilist_media_to_tv};

    #[test]
    fn page_vars_default_to_the_first_page() {
        assert_eq!(page_vars(None, 20), (1, 20));
        assert_eq!(page_vars(Some(4), 7), (4, 7));
        assert_eq!(page_vars(Some(u32::MAX), 50), (i64::from(u32::MAX), 50));
    }

    fn reported_page_info(current: Option<i32>, last: Option<i32>, total: Option<i32>) -> PageInfo {
        serde_json::from_value(json!({
            "currentPage": current,
            "lastPage": last,
            "total": total,
            "hasNextPage": true,
        }))
        .expect("page info")
    }

    /// Omitted `pageInfo` fields read as `1`/`0`/`0`; reported ones pass
    /// through.
    #[test]
    fn page_from_defaults_omitted_fields() {
        let page = page_from(&reported_page_info(None, None, None), vec![1]).expect("in range");
        assert_eq!((page.page, page.total_pages, page.total_results), (1, 0, 0));
        let page =
            page_from(&reported_page_info(Some(2), Some(5), Some(90)), vec![1]).expect("in range");
        assert_eq!(
            (page.page, page.total_pages, page.total_results),
            (2, 5, 90)
        );
    }

    /// A negative page number or total is a typed error naming the field,
    /// never wrapped into a huge `u32`.
    #[test]
    fn page_from_rejects_negative_fields() {
        let cases = [
            ("currentPage", reported_page_info(Some(-1), None, None)),
            ("lastPage", reported_page_info(Some(1), Some(-1), None)),
            (
                "total",
                reported_page_info(Some(1), Some(1), Some(i32::MIN)),
            ),
        ];
        for (expected, pi) in cases {
            match page_from(&pi, Vec::<u8>::new()) {
                Err(AniListError::PageOutOfRange { field, value }) => {
                    assert_eq!(field, expected);
                    assert!(value < 0, "{value}");
                }
                other => panic!("expected PageOutOfRange for {expected}, got {other:?}"),
            }
        }
    }

    /// Media entries with an invalid id are dropped from a page.
    #[test]
    fn media_pages_skip_invalid_ids() {
        let page: MediaPageData = serde_json::from_value(json!({
            "pageInfo": { "currentPage": 1, "lastPage": 1, "total": 3, "hasNextPage": false },
            "media": [{ "id": 1 }, { "id": 0 }, { "id": -2 }],
        }))
        .expect("media page");
        let movies = media_page(Some(page), anilist_media_to_movie).expect("in range");
        let ids: Vec<_> = movies
            .results
            .iter()
            .map(|m| m.provider_id.to_string())
            .collect();
        assert_eq!(ids, ["anilist:1"]);
    }

    /// A missing media page is no data, whatever the conversion.
    #[test]
    fn a_missing_media_page_is_no_data() {
        let err = media_page(None, anilist_media_to_tv).unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");
    }
}
