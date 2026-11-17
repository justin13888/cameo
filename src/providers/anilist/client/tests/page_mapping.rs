//! Page mapping.

use super::*;

fn assert_passed_through<T>(page: &Page<T>) {
    assert_eq!(page.page, 2);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.total_results, 41);
    assert!(page.has_next);
    assert_eq!(page.next, Some(PageToken::Page(3)));
}

fn assert_defaults<T>(page: &Page<T>) {
    assert_eq!(page.page, 1);
    // Omitted totals are unknown, which `Page` documents as `0`.
    assert_eq!(page.total_pages, 0);
    assert_eq!(page.total_results, 0);
    assert!(!page.has_next);
    assert_eq!(page.next, None);
}

#[tokio::test]
async fn page_metadata_is_passed_through() {
    let server = serve(json_response(200, media_page(vec![media(1, "MOVIE")])), 3).await;
    let client = client_for(&server);
    let movies = client.popular_movies(None).await.expect("ok");
    assert_passed_through(&movies);
    assert_eq!(movies.results.len(), 1);
    assert_passed_through(&client.popular_tv_shows(None).await.expect("ok"));
    assert_passed_through(&client.search_multi("x", None).await.expect("ok"));

    let node = json!({ "mediaRecommendation": media(7, "MOVIE") });
    let server = serve(json_response(200, recommendations(vec![node])), 1).await;
    let recs = client_for(&server)
        .movie_recommendations(5, None)
        .await
        .expect("ok");
    assert_passed_through(&recs);
    assert_eq!(recs.results[0].provider_id, "anilist:7");

    let server = serve(json_response(200, staff_page()), 1).await;
    let people = client_for(&server)
        .search_people("x", None)
        .await
        .expect("ok");
    assert_passed_through(&people);
    assert_eq!(people.results[0].provider_id, "anilist:staff:3");
}

#[tokio::test]
async fn missing_page_metadata_takes_defaults() {
    let media = json!({ "data": { "Page": { "pageInfo": {} } } });
    let server = serve(json_response(200, media), 3).await;
    let client = client_for(&server);
    let movies = client.top_rated_movies(None).await.expect("ok");
    assert_defaults(&movies);
    assert!(movies.results.is_empty());
    assert_defaults(&client.search_tv_shows("x", None).await.expect("ok"));
    assert_defaults(&client.search_multi("x", None).await.expect("ok"));

    let staff = json!({ "data": { "Page": { "pageInfo": {} } } });
    let server = serve(json_response(200, staff), 1).await;
    assert_defaults(
        &client_for(&server)
            .search_people("x", None)
            .await
            .expect("ok"),
    );

    let recs = json!({ "data": { "Media": { "recommendations": { "pageInfo": {} } } } });
    let server = serve(json_response(200, recs), 1).await;
    assert_defaults(
        &client_for(&server)
            .tv_recommendations(5, None)
            .await
            .expect("ok"),
    );

    // A missing recommendation connection is an empty first page.
    let none = json!({ "data": { "Media": { "recommendations": null } } });
    let server = serve(json_response(200, none), 1).await;
    let page = client_for(&server)
        .movie_recommendations(5, None)
        .await
        .expect("ok");
    assert_defaults(&page);
    assert!(page.results.is_empty());
}

#[tokio::test]
async fn a_missing_page_is_no_data() {
    for body in [json!({ "data": {} }), json!({ "data": { "Page": null } })] {
        let server = serve(json_response(200, body), 4).await;
        let client = client_for(&server);
        let errors = [
            client.popular_movies(None).await.map(drop).unwrap_err(),
            client
                .search_tv_shows("x", None)
                .await
                .map(drop)
                .unwrap_err(),
            client.search_multi("x", None).await.map(drop).unwrap_err(),
            client.search_people("x", None).await.map(drop).unwrap_err(),
        ];
        for err in errors {
            assert!(matches!(err, AniListError::NoData), "got {err:?}");
        }
    }
}

#[tokio::test]
async fn a_null_root_on_single_item_lookups_is_not_found() {
    let media = json!({ "data": { "Media": null } });
    let server = serve(json_response(200, media.clone()), 5).await;
    let client = client_for(&server);
    let errors = [
        client.movie_details(5).await.map(drop).unwrap_err(),
        client.tv_show_details(5).await.map(drop).unwrap_err(),
        client.external_ids(5).await.map(drop).unwrap_err(),
        client
            .movie_recommendations(5, None)
            .await
            .map(drop)
            .unwrap_err(),
        client.media_credits(5).await.map(drop).unwrap_err(),
    ];
    for err in errors {
        assert!(matches!(err, AniListError::NotFound), "got {err:?}");
    }

    let server = serve(json_response(200, json!({ "data": { "Staff": null } })), 1).await;
    let err = client_for(&server).person_details(3).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");

    // A MAL lookup reports absence as an empty match instead.
    let server = serve(json_response(200, media), 1).await;
    assert_eq!(
        client_for(&server).find_by_mal_id(9).await.expect("ok"),
        None
    );
}

#[tokio::test]
async fn search_multi_classifies_by_format_and_drops_formatless_media() {
    let mut formatless = media(3, "TV");
    formatless["format"] = Value::Null;
    let body = media_page(vec![media(1, "MOVIE"), media(2, "OVA"), formatless]);
    let server = serve(json_response(200, body), 1).await;
    let page = client_for(&server)
        .search_multi("x", None)
        .await
        .expect("ok");
    assert_eq!(page.results.len(), 2);
    assert!(matches!(page.results[0], UnifiedSearchResult::Movie(_)));
    assert!(matches!(page.results[1], UnifiedSearchResult::TvShow(_)));
}

#[tokio::test]
async fn external_ids_map_mal_and_anilist_ids() {
    let server = serve(json_response(200, ids("TV")), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.anilist, Some(5));
    assert_eq!(ids.mal, Some(9));
    assert_eq!(ids.imdb, None);

    let body = json!({ "data": { "Media": { "id": 5, "idMal": -1 } } });
    let server = serve(json_response(200, body), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.mal, None, "a negative MAL id is dropped");

    let body = json!({ "data": { "Media": { "id": 0, "idMal": 9 } } });
    let server = serve(json_response(200, body), 1).await;
    let ids = client_for(&server).external_ids(5).await.expect("ok");
    assert_eq!(ids.anilist, None, "a reported AniList id of 0 is dropped");
    assert_eq!(ids.mal, Some(9));
}

#[tokio::test]
async fn find_by_mal_id_drops_a_media_id_of_zero() {
    let body = json!({ "data": { "Media": { "id": 0, "format": "MOVIE" } } });
    let server = serve(json_response(200, body), 1).await;
    let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
    assert_eq!(found, None);
}

#[tokio::test]
async fn find_by_mal_id_classifies_movie_and_series() {
    let movie = MediaId::anilist(5);
    let server = serve(json_response(200, ids("MOVIE")), 1).await;
    let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
    assert_eq!(found, Some(ExternalMatch::movie(movie.clone())));

    for format in TV_FORMATS {
        let body = json!({ "data": { "Media": { "id": 5, "format": format.as_str() } } });
        let server = serve(json_response(200, body), 1).await;
        let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
        assert_eq!(found, Some(ExternalMatch::tv_show(movie.clone())));
    }

    // A format `classify` places in neither list is not a match.
    for format in [json!("MUSIC"), json!("NOT_A_FORMAT"), Value::Null] {
        let body = json!({ "data": { "Media": { "id": 5, "format": format } } });
        let server = serve(json_response(200, body), 1).await;
        let found = client_for(&server).find_by_mal_id(9).await.expect("ok");
        assert_eq!(found, None, "format {format}");
    }

    let not_found = json!({ "errors": [{ "message": "Not Found.", "status": 404 }] });
    let server = serve(json_response(404, not_found), 1).await;
    assert_eq!(
        client_for(&server).find_by_mal_id(9).await.expect("ok"),
        None
    );
}
