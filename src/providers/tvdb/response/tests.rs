//! Deserialization tests for TheTVDB records against wire-shaped JSON.

use serde_json::json;

use super::*;

#[test]
fn episode_reads_absolute_numbers_and_flags() {
    let ep: TvdbEpisode = serde_json::from_value(json!({
        "id": 1, "seriesId": 9, "seasonNumber": 2, "number": 3,
        "absoluteNumber": 15, "isMovie": 0, "name": "Ep"
    }))
    .expect("episode");
    assert_eq!(ep.absolute_number, Some(15));
    assert_eq!(ep.is_movie, Some(false));
    assert!(!ep.is_special());

    let special: TvdbEpisode = serde_json::from_value(json!({
        "id": 2, "seasonNumber": 0, "number": 1, "absoluteNumber": 0,
        "isMovie": 1
    }))
    .expect("special");
    assert_eq!(special.absolute_number, None);
    assert_eq!(special.is_movie, Some(true));
    assert!(special.is_special());
}

#[test]
fn null_lists_and_string_amounts_read_cleanly() {
    let movie: TvdbMovie = serde_json::from_value(json!({
        "id": 1, "name": "M", "genres": null, "artworks": null,
        "remoteIds": null, "companies": null, "translations": null,
        "budget": "160000000", "boxOffice": 5, "spoken_languages": null
    }))
    .expect("movie");
    assert!(movie.genres.is_empty());
    assert_eq!(movie.budget, Some(160_000_000));
    assert_eq!(movie.box_office, Some(5));
    assert_eq!(movie.translations, TvdbTranslations::default());

    let odd: TvdbMovie =
        serde_json::from_value(json!({ "id": 1, "name": "M", "budget": "n/a" })).expect("movie");
    assert_eq!(odd.budget, None);
}

#[test]
fn translations_skip_aliases_and_blanks() {
    let t: TvdbTranslations = serde_json::from_value(json!({
        "nameTranslations": [
            { "language": "eng", "name": "Alias", "isAlias": true },
            { "language": "eng", "name": " " },
            { "language": "eng", "name": "Name" }
        ],
        "overviewTranslations": [{ "language": "jpn", "overview": "概要" }]
    }))
    .expect("translations");
    assert_eq!(t.name("eng"), Some("Name"));
    assert_eq!(t.name("jpn"), None);
    assert_eq!(t.overview("jpn"), Some("概要"));
    assert_eq!(t.overview("eng"), None);
}
