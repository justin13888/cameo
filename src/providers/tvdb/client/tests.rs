//! Unit tests for [`TvdbClient`]'s paging and body-truncation helpers, its
//! search types, config validation, and credential redaction.

use super::*;

#[test]
fn paged_reads_next_and_totals() {
    let links = Links {
        next: Some("https://next".into()),
        total_items: Some(1001),
        page_size: Some(500),
    };
    let page = paged(1, vec![1, 2], &links);
    assert!(page.has_next);
    assert_eq!(page.next_page(), Some(2));
    assert_eq!((page.total_pages, page.total_results), (3, 1001));

    let last = paged(3, vec![3], &Links::default());
    assert!(!last.has_next);
    assert_eq!((last.total_pages, last.total_results), (0, 0));

    let huge = Links {
        next: None,
        total_items: Some(u64::MAX),
        page_size: Some(1),
    };
    let page = paged(1, Vec::<u8>::new(), &huge);
    assert_eq!((page.total_pages, page.total_results), (u32::MAX, u32::MAX));
}

#[test]
fn truncate_body_names_empty_and_cuts_long_bodies() {
    assert_eq!(truncate_body("  "), "empty response body");
    assert_eq!(truncate_body(" x "), "x");
    assert_eq!(
        truncate_body(&"é".repeat(201)),
        format!("{}…", "é".repeat(200))
    );
}

#[test]
fn search_type_values_are_tvdbs() {
    assert_eq!(TvdbSearchType::Series.as_str(), "series");
    assert_eq!(TvdbSearchType::Movie.as_str(), "movie");
    assert_eq!(TvdbSearchType::Person.as_str(), "person");
    assert_eq!(TvdbSearchType::Company.as_str(), "company");
}

#[test]
fn new_rejects_invalid_config() {
    let err = TvdbClient::new(TvdbConfig::new("")).unwrap_err();
    assert!(matches!(err, TvdbError::InvalidConfig(_)), "{err:?}");
}

#[test]
fn debug_omits_credentials() {
    let client =
        TvdbClient::new(TvdbConfig::new("SECRETKEY").with_pin("SECRETPIN")).expect("valid config");
    let rendered = format!("{client:?}");
    assert!(!rendered.contains("SECRETKEY") && !rendered.contains("SECRETPIN"));
}
