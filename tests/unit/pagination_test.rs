use cameo::core::pagination::{PaginatedResponse, into_stream};
use futures::StreamExt;

fn make_page(page: u32, total: u32, items: Vec<u32>) -> PaginatedResponse<u32> {
    PaginatedResponse {
        page,
        results: items,
        total_pages: total,
        total_results: total * 2,
    }
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
            Ok::<_, String>(PaginatedResponse {
                page,
                results: items,
                total_pages: total,
                total_results: 5,
            })
        }
    });

    let collected: Vec<Result<u32, String>> = stream.collect().await;
    let values: Vec<u32> = collected.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn stream_single_page() {
    let stream = into_stream(|_page| async {
        Ok::<_, String>(PaginatedResponse {
            page: 1,
            results: vec![42u32],
            total_pages: 1,
            total_results: 1,
        })
    });

    let values: Vec<u32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(values, vec![42]);
}

#[tokio::test]
async fn stream_propagates_error() {
    let stream = into_stream(|page| async move {
        if page == 1 {
            Ok::<_, String>(PaginatedResponse {
                page: 1,
                results: vec![1u32],
                total_pages: 2,
                total_results: 2,
            })
        } else {
            Err("fetch error".to_string())
        }
    });

    let results: Vec<Result<u32, String>> = stream.collect().await;
    assert_eq!(results[0], Ok(1));
    assert_eq!(results[1], Err("fetch error".to_string()));
}
