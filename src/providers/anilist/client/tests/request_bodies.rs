//! Request bodies.

use super::*;

/// The formats a TV-typed list request filters on.
fn tv_formats() -> Value {
    json!(["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"])
}

/// Run `$call` against a server answering `$response`, with `perPage` 7,
/// and assert the one request sent carried `$query` and exactly `$vars`.
macro_rules! assert_sends {
    ($response:expr, |$client:ident| $call:expr, $query:expr, $vars:expr $(,)?) => {{
        let server = serve(json_response(200, $response), 1).await;
        let $client = client_with(&server, |c| c.with_per_page(7));
        $call.await.expect("call succeeds");
        let body = only_request(&server).await;
        assert_eq!(body["query"], $query, "query text");
        assert_eq!(body["variables"], $vars, "variables");
        assert_eq!(
            body.as_object().map(|o| o.len()),
            Some(2),
            "body holds only query and variables"
        );
    }};
}

#[tokio::test]
async fn search_requests_filter_formats_by_media_type() {
    assert_sends!(
        media_page(vec![]),
        |c| c.search_movies("naruto", None),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 1, "perPage": 7, "formatIn": ["MOVIE"] }),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.search_tv_shows("naruto", Some(3)),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 3, "perPage": 7, "formatIn": tv_formats() }),
    );
}

#[tokio::test]
async fn search_multi_and_people_omit_the_format_filter() {
    assert_sends!(
        media_page(vec![]),
        |c| c.search_multi("naruto", Some(2)),
        query::SEARCH_ANIME,
        json!({ "query": "naruto", "page": 2, "perPage": 7 }),
    );
    assert_sends!(
        staff_page(),
        |c| c.search_people("kaji", None),
        query::SEARCH_STAFF,
        json!({ "query": "kaji", "page": 1, "perPage": 7 }),
    );
}

#[tokio::test]
async fn list_requests_send_page_per_page_and_formats() {
    let movies = json!({ "page": 1, "perPage": 7, "formatIn": ["MOVIE"] });
    let tv = json!({ "page": 4, "perPage": 7, "formatIn": tv_formats() });
    assert_sends!(
        media_page(vec![]),
        |c| c.trending_movies(TimeWindow::Week, None),
        query::LIST_TRENDING_ANIME,
        movies.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.trending_tv_shows(TimeWindow::Day, Some(4)),
        query::LIST_TRENDING_ANIME,
        tv.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.popular_movies(None),
        query::LIST_POPULAR_ANIME,
        movies.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.popular_tv_shows(Some(4)),
        query::LIST_POPULAR_ANIME,
        tv.clone(),
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.top_rated_movies(None),
        query::LIST_TOP_SCORED_ANIME,
        movies,
    );
    assert_sends!(
        media_page(vec![]),
        |c| c.top_rated_tv_shows(Some(4)),
        query::LIST_TOP_SCORED_ANIME,
        tv,
    );
}

#[tokio::test]
async fn per_page_defaults_to_20() {
    let server = serve(json_response(200, media_page(vec![])), 1).await;
    client_for(&server)
        .popular_movies(None)
        .await
        .expect("call succeeds");
    let body = only_request(&server).await;
    assert_eq!(body["variables"]["perPage"], 20);
    assert_eq!(body["variables"]["page"], 1);
}

#[tokio::test]
async fn id_requests_send_only_the_id() {
    assert_sends!(
        details(),
        |c| c.movie_details(5),
        query::MEDIA_DETAILS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        details(),
        |c| c.tv_show_details(5),
        query::MEDIA_DETAILS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        staff_details(),
        |c| c.person_details(3),
        query::STAFF_DETAILS,
        json!({ "id": 3 }),
    );
    assert_sends!(
        ids("TV"),
        |c| c.external_ids(5),
        query::MEDIA_EXTERNAL_IDS,
        json!({ "id": 5 }),
    );
    assert_sends!(
        ids("TV"),
        |c| c.find_by_mal_id(9),
        query::MEDIA_BY_MAL_ID,
        json!({ "idMal": 9 }),
    );
}

#[tokio::test]
async fn recommendation_and_credit_requests_send_their_paging() {
    let vars = json!({ "id": 5, "page": 1, "perPage": 7 });
    assert_sends!(
        recommendations(vec![]),
        |c| c.movie_recommendations(5, None),
        query::MEDIA_RECOMMENDATIONS,
        vars.clone(),
    );
    assert_sends!(
        recommendations(vec![]),
        |c| c.tv_recommendations(5, Some(1)),
        query::MEDIA_RECOMMENDATIONS,
        vars,
    );
    // Credits page with their own fixed size, not the configured one.
    assert_sends!(
        credits(),
        |c| c.media_credits(5),
        query::MEDIA_CREDITS,
        json!({ "id": 5, "characterPage": 1, "staffPage": 1, "perPage": 25 }),
    );
}

#[tokio::test]
async fn a_negative_credits_id_is_not_found_without_a_request() {
    let server = serve(json_response(200, credits()), 0).await;
    let err = client_for(&server).media_credits(-1).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn a_zero_credits_id_is_not_found_without_a_request() {
    let server = serve(json_response(200, credits()), 0).await;
    let err = client_for(&server).media_credits(0).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}
