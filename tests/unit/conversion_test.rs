use cameo::generated::tmdb::types;
use cameo::unified::models::*;

fn make_search_movie() -> types::SearchMovieResponseResultsItem {
    types::SearchMovieResponseResultsItem {
        id: 550,
        title: Some("Fight Club".to_string()),
        original_title: Some("Fight Club".to_string()),
        overview: Some("A ticking-time-bomb insomniac...".to_string()),
        release_date: Some("1999-10-15".to_string()),
        poster_path: Some("/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg".to_string()),
        backdrop_path: Some("/hZkgoQYus5vegHoetLkCJzb17zJ.jpg".to_string()),
        genre_ids: vec![18, 53],
        popularity: Some(73.433),
        vote_average: Some(8.4),
        vote_count: 26280,
        original_language: Some("en".to_string()),
        adult: false,
        video: false,
    }
}

#[test]
fn search_movie_to_unified() {
    let raw = make_search_movie();
    let unified: UnifiedMovie = raw.into();

    assert_eq!(unified.provider_id, "tmdb:550");
    assert_eq!(unified.title, "Fight Club");
    assert_eq!(unified.original_title.as_deref(), Some("Fight Club"));
    assert_eq!(unified.release_date.as_deref(), Some("1999-10-15"));
    assert_eq!(unified.vote_count, 26280);
    assert!((unified.vote_average.unwrap() - 8.4).abs() < 0.01);
    assert_eq!(unified.original_language.as_deref(), Some("en"));
    assert!(!unified.adult);
    // Genres are empty for list results (only IDs are returned)
    assert!(unified.genres.is_empty());
    // Image URLs should be resolved
    assert!(unified.poster_url.as_deref().unwrap().starts_with("https://image.tmdb.org/t/p/"));
    assert!(unified.backdrop_url.as_deref().unwrap().starts_with("https://image.tmdb.org/t/p/"));
}

#[test]
fn search_movie_no_poster() {
    let mut raw = make_search_movie();
    raw.poster_path = None;
    raw.backdrop_path = None;
    let unified: UnifiedMovie = raw.into();
    assert!(unified.poster_url.is_none());
    assert!(unified.backdrop_url.is_none());
}

fn make_search_tv() -> types::SearchTvResponseResultsItem {
    types::SearchTvResponseResultsItem {
        id: 1396,
        name: Some("Breaking Bad".to_string()),
        original_name: Some("Breaking Bad".to_string()),
        overview: Some("A high school chemistry teacher...".to_string()),
        first_air_date: Some("2008-01-20".to_string()),
        poster_path: Some("/ggFHVNu6YYI5L9pCfOacjizRGt.jpg".to_string()),
        backdrop_path: Some("/tsRy63Mu5cu8etL1X7ZLyf7UP1M.jpg".to_string()),
        genre_ids: vec![18, 80],
        origin_country: vec!["US".to_string()],
        popularity: Some(282.9),
        vote_average: Some(9.5),
        vote_count: 14000,
        original_language: Some("en".to_string()),
        adult: false,
    }
}

#[test]
fn search_tv_to_unified() {
    let raw = make_search_tv();
    let unified: UnifiedTvShow = raw.into();

    assert_eq!(unified.provider_id, "tmdb:1396");
    assert_eq!(unified.name, "Breaking Bad");
    assert_eq!(unified.first_air_date.as_deref(), Some("2008-01-20"));
    assert_eq!(unified.origin_country, vec!["US"]);
    assert_eq!(unified.vote_count, 14000);
    assert!(unified.poster_url.is_some());
}

fn make_search_person() -> types::SearchPersonResponseResultsItem {
    types::SearchPersonResponseResultsItem {
        id: 287,
        name: Some("Brad Pitt".to_string()),
        original_name: None,
        known_for_department: Some("Acting".to_string()),
        profile_path: Some("/kU3B75TyRiCgE270EyZnHjfivoq.jpg".to_string()),
        popularity: Some(25.3),
        adult: false,
        gender: 2,
        known_for: vec![],
    }
}

#[test]
fn search_person_to_unified() {
    let raw = make_search_person();
    let unified: UnifiedPerson = raw.into();

    assert_eq!(unified.provider_id, "tmdb:287");
    assert_eq!(unified.name, "Brad Pitt");
    assert_eq!(unified.known_for_department.as_deref(), Some("Acting"));
    assert!(unified.profile_url.is_some());
    assert!(unified.profile_url.unwrap().starts_with("https://image.tmdb.org/t/p/"));
}

#[test]
fn search_multi_movie_type() {
    let item = types::SearchMultiResponseResultsItem {
        id: 550,
        media_type: Some("movie".to_string()),
        title: Some("Fight Club".to_string()),
        name: None,
        original_title: Some("Fight Club".to_string()),
        original_name: None,
        overview: Some("overview".to_string()),
        release_date: Some("1999-10-15".to_string()),
        poster_path: Some("/poster.jpg".to_string()),
        backdrop_path: None,
        genre_ids: vec![],
        popularity: Some(73.4),
        vote_average: Some(8.4),
        vote_count: 26000,
        original_language: Some("en".to_string()),
        adult: false,
        video: false,
    };

    let result: UnifiedSearchResult = item.into();
    match result {
        UnifiedSearchResult::Movie(m) => {
            assert_eq!(m.provider_id, "tmdb:550");
            assert_eq!(m.title, "Fight Club");
        }
        other => panic!("expected Movie, got {:?}", other),
    }
}

#[test]
fn search_multi_tv_type() {
    let item = types::SearchMultiResponseResultsItem {
        id: 1396,
        media_type: Some("tv".to_string()),
        title: None,
        name: Some("Breaking Bad".to_string()),
        original_title: None,
        original_name: Some("Breaking Bad".to_string()),
        overview: None,
        release_date: None,
        poster_path: None,
        backdrop_path: None,
        genre_ids: vec![],
        popularity: None,
        vote_average: None,
        vote_count: 0,
        original_language: None,
        adult: false,
        video: false,
    };

    let result: UnifiedSearchResult = item.into();
    match result {
        UnifiedSearchResult::TvShow(t) => {
            assert_eq!(t.provider_id, "tmdb:1396");
            assert_eq!(t.name, "Breaking Bad");
        }
        other => panic!("expected TvShow, got {:?}", other),
    }
}
