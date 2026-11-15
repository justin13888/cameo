use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use cameo::core::pagination::{Page, PageToken, into_stream};
use futures::StreamExt;
use proptest::prelude::*;

fn make_page(page: u32, total: u32, items: Vec<u32>) -> Page<u32> {
    Page::offset(page, items, total, total * 2)
}

#[test]
fn has_next_page_mid() {
    let p = make_page(1, 3, vec![1]);
    assert!(p.has_next_page());
}

#[test]
fn has_next_page_last() {
    let p = make_page(3, 3, vec![1]);
    assert!(!p.has_next_page());
}

#[test]
fn next_page_mid() {
    let p = make_page(2, 5, vec![]);
    assert_eq!(p.next_page(), Some(3));
}

#[test]
fn next_page_last() {
    let p = make_page(5, 5, vec![]);
    assert_eq!(p.next_page(), None);
}

#[test]
fn single_page() {
    let p = make_page(1, 1, vec![10, 20]);
    assert!(!p.has_next_page());
    assert_eq!(p.next_page(), None);
    assert_eq!(p.results, vec![10, 20]);
}

#[tokio::test]
async fn stream_collects_all_pages() {
    let data: Vec<Vec<u32>> = vec![vec![1, 2], vec![3, 4], vec![5]];
    let total = data.len() as u32;
    let data = std::sync::Arc::new(data);

    let stream = into_stream(move |page| {
        let data = data.clone();
        async move {
            let idx = (page - 1) as usize;
            let items = data[idx].clone();
            Ok::<_, String>(Page::offset(page, items, total, 5))
        }
    });

    let collected: Vec<Result<u32, String>> = stream.collect().await;
    let values: Vec<u32> = collected.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn stream_single_page() {
    let stream = into_stream(|_page| async { Ok::<_, String>(Page::offset(1, vec![42u32], 1, 1)) });

    let values: Vec<u32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(values, vec![42]);
}

#[tokio::test]
async fn stream_propagates_error() {
    let stream = into_stream(|page| async move {
        if page == 1 {
            Ok::<_, String>(Page::offset(1, vec![1u32], 2, 2))
        } else {
            Err("fetch error".to_string())
        }
    });

    let results: Vec<Result<u32, String>> = stream.collect().await;
    assert_eq!(results[0], Ok(1));
    assert_eq!(results[1], Err("fetch error".to_string()));
}

/// The stream is `Send`, so it can be moved into a `tokio::spawn`ed task on a
/// multi-thread runtime (ADR 0001 / #27).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_is_send_and_spawnable() {
    let handle = tokio::spawn(async move {
        let stream = into_stream(|page| async move {
            if page <= 2 {
                Ok::<_, String>(Page::with_has_next(page, vec![page], page < 2, 0, 0))
            } else {
                Ok(Page::with_has_next(page, vec![], false, 0, 0))
            }
        });
        stream.map(|r| r.unwrap()).collect::<Vec<u32>>().await
    });

    assert_eq!(handle.await.unwrap(), vec![1, 2]);
}

/// Authoritative `has_next` drives iteration even when totals are `0`
/// (the AniList null-totals case).
#[tokio::test]
async fn stream_uses_authoritative_has_next_with_zero_totals() {
    let stream = into_stream(|page| async move {
        // Totals are 0 (unknown); has_next alone controls continuation.
        Ok::<_, String>(Page::with_has_next(page, vec![page], page < 3, 0, 0))
    });
    let values: Vec<u32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(values, vec![1, 2, 3]);
}

// ── Metadata preservation, cursor termination, error termination ─────────────

/// A page whose every metadata field is distinguishable.
fn metadata_page() -> Page<u32> {
    let mut page = Page::with_has_next(2, vec![1, 2, 3, 4], true, 9, 87);
    page.next = Some(PageToken::Page(3));
    page
}

#[test]
fn map_preserves_metadata() {
    let mapped = metadata_page().map(|n| n * 10);
    assert_eq!(mapped.results, vec![10, 20, 30, 40]);
    assert_eq!(
        (
            mapped.page,
            mapped.has_next,
            mapped.next,
            mapped.total_pages,
            mapped.total_results
        ),
        (2, true, Some(PageToken::Page(3)), 9, 87)
    );
}

#[test]
fn filter_map_preserves_metadata_and_server_totals() {
    let filtered = metadata_page().filter_map(|n| (n % 2 == 0).then_some(n));
    assert_eq!(filtered.results, vec![2, 4]);
    // Totals still describe the provider's full result set, not the filtered page.
    assert_eq!(
        (
            filtered.page,
            filtered.has_next,
            filtered.next,
            filtered.total_pages,
            filtered.total_results
        ),
        (2, true, Some(PageToken::Page(3)), 9, 87)
    );
}

#[test]
fn cursor_token_has_no_next_page_number() {
    let mut page = Page::with_has_next(1, vec![1u32], true, 0, 0);
    page.next = Some(PageToken::Cursor("opaque".to_string()));
    assert!(page.has_next_page());
    assert_eq!(page.next_page(), None);
}

/// `into_stream` only follows page numbers: a cursor-paginated page ends the
/// stream even though `has_next` is set.
#[tokio::test]
async fn stream_ends_at_a_cursor_token() {
    let calls = Arc::new(AtomicU32::new(0));
    let seen = calls.clone();
    let stream = into_stream(move |page| {
        let calls = calls.clone();
        async move {
            calls.fetch_add(1, Ordering::SeqCst);
            let mut p = Page::with_has_next(page, vec![page], true, 0, 0);
            p.next = Some(PageToken::Cursor("next".to_string()));
            Ok::<_, String>(p)
        }
    });
    let values: Vec<u32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(values, vec![1]);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
}

/// The stream yields the error and then ends: nothing follows a failed fetch.
#[tokio::test]
async fn stream_ends_after_the_first_error() {
    let stream = into_stream(|page| async move {
        if page == 1 {
            Ok::<_, String>(Page::offset(1, vec![1u32], 5, 5))
        } else {
            Err(format!("fetch error {page}"))
        }
    });

    let results: Vec<Result<u32, String>> = stream.collect().await;
    assert_eq!(results, vec![Ok(1), Err("fetch error 2".to_string())]);
}

proptest! {
    /// Streaming yields exactly the pages' items concatenated in page order.
    #[test]
    fn stream_equals_pages_concatenated(
        pages in prop::collection::vec(prop::collection::vec(any::<u32>(), 0..5), 1..6)
    ) {
        let expected: Vec<u32> = pages.concat();
        let total = pages.len() as u32;
        let pages = Arc::new(pages);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");
        let values: Vec<u32> = runtime.block_on(async move {
            let stream = into_stream(move |page| {
                let pages = pages.clone();
                async move {
                    let items = pages[(page - 1) as usize].clone();
                    Ok::<_, String>(Page::offset(page, items, total, 0))
                }
            });
            stream.map(|r| r.unwrap()).collect().await
        });
        prop_assert_eq!(values, expected);
    }
}
