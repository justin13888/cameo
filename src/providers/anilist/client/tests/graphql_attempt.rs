//! The `graphql_attempt` decision table.

use super::*;

#[tokio::test]
async fn success_with_data_returns_the_root() {
    let server = serve(json_response(200, details()), 1).await;
    let movie = client_for(&server).movie_details(5).await.expect("ok");
    assert_eq!(movie.movie.provider_id, "anilist:5");
}

#[tokio::test]
async fn error_status_404_is_not_found_even_with_data() {
    let body = with_errors(
        details(),
        json!([{ "message": "Not Found.", "status": 404 }]),
    );
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn errors_with_data_return_the_partial_data() {
    for status in [400, 500] {
        let body = with_errors(
            details(),
            json!([{ "message": "partial", "status": status }]),
        );
        let server = serve(json_response(200, body), 1).await;
        // Retries are enabled: a usable root is returned on the first attempt.
        let client = client_with(&server, |c| c.with_retry(TWO_FAST_RETRIES));
        let movie = client.movie_details(5).await.expect("partial data");
        assert_eq!(movie.movie.provider_id, "anilist:5");
    }
}

#[tokio::test]
async fn errors_without_data_are_graphql_errors_with_retry_after() {
    let template = json_response(
        200,
        json!({ "errors": [{ "message": "bad query", "status": 400 }] }),
    )
    .insert_header("Retry-After", "7");
    let server = serve(template, 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    let errors = graphql_errors(&err);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "bad query");
    assert_eq!(errors[0].status, Some(400));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(7)));
}

#[tokio::test]
async fn errors_with_a_null_root_are_graphql_errors() {
    let body = json!({
        "data": { "Media": null },
        "errors": [{ "message": "failed", "status": 200 }]
    });
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, Some(200));
}

#[tokio::test]
async fn statusless_errors_inherit_only_a_non_2xx_status() {
    let body = json!({ "errors": [{ "message": "no status" }] });

    let server = serve(json_response(400, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, Some(400));

    let server = serve(json_response(200, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert_eq!(graphql_errors(&err)[0].status, None);

    // An inherited 404 is not-found.
    let server = serve(json_response(404, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn an_empty_errors_array_is_treated_as_no_errors() {
    let server = serve(json_response(200, with_errors(details(), json!([]))), 1).await;
    let movie = client_for(&server).movie_details(5).await.expect("ok");
    assert_eq!(movie.movie.provider_id, "anilist:5");

    let body = json!({ "data": { "Media": null }, "errors": [] });
    let server = serve(json_response(200, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NotFound), "got {err:?}");

    let server = serve(json_response(200, json!({ "errors": [] })), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(matches!(err, AniListError::NoData), "got {err:?}");
}

#[tokio::test]
async fn a_non_json_error_response_is_a_status_error() {
    let template =
        raw_response(502, "  <html>Bad Gateway</html>\n").insert_header("Retry-After", "3");
    let server = serve(template, 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    match err {
        AniListError::Status {
            status,
            retry_after,
            message,
        } => {
            assert_eq!(status, 502);
            assert_eq!(retry_after, Some(Duration::from_secs(3)));
            assert_eq!(message, "<html>Bad Gateway</html>");
        }
        other => panic!("expected Status, got {other:?}"),
    }

    let server = serve(raw_response(503, ""), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(&err, AniListError::Status { status: 503, message, .. } if message == "empty response body"),
        "got {err:?}"
    );

    let long = "x".repeat(300);
    let server = serve(raw_response(500, &long), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    let expected = format!("{}…", "x".repeat(200));
    assert!(
        matches!(&err, AniListError::Status { status: 500, message, .. } if *message == expected),
        "got {err:?}"
    );
}

#[tokio::test]
async fn an_undecodable_success_response_is_a_deserialization_error() {
    let server = serve(raw_response(200, "not json"), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Deserialization(_)),
        "got {err:?}"
    );

    // Valid JSON of the wrong shape is a decode error too...
    let body = json!({ "data": { "Media": { "id": "five" } } });
    let server = serve(json_response(200, body.clone()), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Deserialization(_)),
        "got {err:?}"
    );

    // ...unless the status is an error, which is the better explanation.
    let server = serve(json_response(500, body), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 500, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn no_data_and_no_errors_on_an_error_status_is_a_status_error() {
    let server = serve(json_response(503, json!({})), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(&err, AniListError::Status { status: 503, message, .. } if message == "{}"),
        "got {err:?}"
    );

    let server = serve(json_response(500, json!({ "data": { "Media": null } })), 1).await;
    let err = client_for(&server).movie_details(5).await.unwrap_err();
    assert!(
        matches!(err, AniListError::Status { status: 500, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn no_data_and_no_errors_on_success_is_no_data() {
    for body in [json!({}), json!({ "data": null })] {
        // Even a details call reports NoData, not NotFound.
        let server = serve(json_response(200, body.clone()), 1).await;
        let err = client_for(&server).movie_details(5).await.unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");

        let server = serve(json_response(200, body), 1).await;
        let err = client_for(&server).popular_movies(None).await.unwrap_err();
        assert!(matches!(err, AniListError::NoData), "got {err:?}");
    }
}
