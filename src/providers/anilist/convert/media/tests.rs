use super::*;
use crate::providers::anilist::response::{AniListMedia, AniListMediaDetail};

fn title(english: Option<&str>, romaji: Option<&str>, native: Option<&str>) -> AniListTitle {
    AniListTitle {
        english: english.map(String::from),
        romaji: romaji.map(String::from),
        native: native.map(String::from),
    }
}

fn alt(text: &str, kind: TitleKind, language: Option<&str>) -> AlternativeTitle {
    let a = AlternativeTitle::new(text, kind);
    match language {
        Some(l) => a.with_language(l),
        None => a,
    }
}

fn make_detail(t: Option<AniListTitle>, synonyms: Option<Vec<String>>) -> AniListMediaDetail {
    AniListMediaDetail {
        id: 7,
        title: t,
        synonyms,
        description: None,
        start_date: None,
        end_date: None,
        cover_image: None,
        banner_image: None,
        genres: None,
        popularity: None,
        average_score: None,
        episodes: None,
        duration: None,
        status: None,
        format: Some("MOVIE".to_string()),
        country_of_origin: Some("JP".to_string()),
        is_adult: None,
        studios: None,
    }
}

fn make_media(id: i32, format: Option<&str>) -> AniListMedia {
    AniListMedia {
        id,
        title: None,
        synonyms: None,
        description: None,
        start_date: None,
        cover_image: None,
        banner_image: None,
        genres: None,
        popularity: None,
        average_score: None,
        episodes: None,
        duration: None,
        status: None,
        format: format.map(String::from),
        country_of_origin: None,
        is_adult: None,
    }
}

fn make_list_fields_detail(format: &str) -> AniListMediaDetail {
    AniListMediaDetail {
        id: 7,
        title: None,
        synonyms: None,
        description: None,
        start_date: None,
        end_date: None,
        cover_image: None,
        banner_image: None,
        genres: None,
        popularity: None,
        average_score: None,
        episodes: Some(26),
        duration: Some(23),
        status: Some("RELEASING".to_string()),
        format: Some(format.to_string()),
        country_of_origin: None,
        is_adult: None,
        studios: None,
    }
}

/// The summary and the detail conversions agree on the new list fields for
/// the same media, and the detail's embedded summary mirrors its own
/// top-level values.
#[test]
fn list_and_detail_agree_on_runtime_episodes_status() {
    let mut list = make_media(7, Some("TV"));
    list.episodes = Some(26);
    list.duration = Some(23);
    list.status = Some("RELEASING".to_string());
    let summary = anilist_media_to_tv(list).expect("valid id");
    let details =
        anilist_media_detail_to_tv_details(make_list_fields_detail("TV")).expect("valid id");
    assert_eq!(details.show.number_of_episodes, summary.number_of_episodes);
    assert_eq!(details.show.episode_run_time, summary.episode_run_time);
    assert_eq!(details.show.status, summary.status);
    assert_eq!(details.number_of_episodes, Some(26));
    assert_eq!(details.episode_run_time, vec![23]);
    assert_eq!(details.status, Some(SeriesStatus::ReturningSeries));

    let mut list = make_media(7, Some("MOVIE"));
    list.duration = Some(23);
    list.status = Some("RELEASING".to_string());
    let summary = anilist_media_to_movie(list).expect("valid id");
    let details =
        anilist_media_detail_to_movie_details(make_list_fields_detail("MOVIE")).expect("valid id");
    assert_eq!(details.movie.runtime, summary.runtime);
    assert_eq!(details.movie.status, summary.status);
    assert_eq!(details.runtime, Some(23));
    assert_eq!(details.status.as_deref(), Some("RELEASING"));
}

#[test]
fn country_to_language_maps_known_origins() {
    assert_eq!(country_to_language("JP"), Some("ja"));
    assert_eq!(country_to_language("CN"), Some("zh"));
    assert_eq!(country_to_language("KR"), Some("ko"));
    assert_eq!(country_to_language("TW"), Some("zh"));
}

#[test]
fn country_to_language_unmapped_country_is_none() {
    assert_eq!(country_to_language("US"), None);
}

#[test]
fn test_score_conversion() {
    assert_eq!(score_to_vote_average(Some(85)), Some(8.5));
    assert_eq!(score_to_vote_average(Some(100)), Some(10.0));
    assert_eq!(score_to_vote_average(None), None);
}

#[test]
fn search_result_movie_format() {
    let result = anilist_media_to_search_result(make_media(1, Some("MOVIE")));
    assert!(matches!(result, Some(UnifiedSearchResult::Movie(_))));
}

#[test]
fn search_result_tv_format() {
    let result = anilist_media_to_search_result(make_media(2, Some("TV")));
    assert!(matches!(result, Some(UnifiedSearchResult::TvShow(_))));
}

#[test]
fn search_result_every_tv_format_is_a_show() {
    for format in ["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"] {
        let result = anilist_media_to_search_result(make_media(3, Some(format)));
        assert!(
            matches!(result, Some(UnifiedSearchResult::TvShow(_))),
            "{format} should classify as a show"
        );
    }
}

#[test]
fn search_result_format_outside_movie_and_tv_lists_is_dropped() {
    // MUSIC is in neither formatIn list, so no movie or TV list returns it.
    assert!(anilist_media_to_search_result(make_media(5, Some("MUSIC"))).is_none());
    // An unrecognized future format is omitted, not coerced to a show.
    assert!(anilist_media_to_search_result(make_media(6, Some("HOLOGRAM"))).is_none());
}

#[test]
fn search_result_absent_format_is_dropped() {
    // Shared unknown-media policy: no format -> omitted, not coerced.
    let result = anilist_media_to_search_result(make_media(4, None));
    assert!(result.is_none());
}

/// A zero or negative id is not an AniList id: every conversion skips the
/// record rather than wrap the id into a huge `u64`.
#[test]
fn invalid_media_id_skips_the_record() {
    for id in [0, -1, i32::MIN] {
        assert!(anilist_media_to_movie(make_media(id, Some("MOVIE"))).is_none());
        assert!(anilist_media_to_tv(make_media(id, Some("TV"))).is_none());
        for format in ["MOVIE", "TV"] {
            assert!(anilist_media_to_search_result(make_media(id, Some(format))).is_none());
        }
        let detail = || AniListMediaDetail {
            id,
            ..make_detail(None, None)
        };
        assert!(anilist_media_detail_to_movie_details(detail()).is_none());
        assert!(anilist_media_detail_to_tv_details(detail()).is_none());
    }
    let movie = anilist_media_to_movie(make_media(1, Some("MOVIE"))).expect("valid id");
    assert_eq!(movie.provider_id, "anilist:1");
}

#[test]
fn romaji_is_kept_when_english_exists() {
    let mut m = make_media(10, Some("MOVIE"));
    m.title = Some(title(
        Some("Your Name."),
        Some("Kimi no Na wa."),
        Some("君の名は。"),
    ));
    m.country_of_origin = Some("JP".to_string());
    let movie = anilist_media_to_movie(m).expect("valid id");
    assert_eq!(movie.title, "Your Name.");
    assert_eq!(movie.original_title.as_deref(), Some("君の名は。"));
    assert_eq!(
        movie.alternative_titles,
        vec![
            alt("Your Name.", TitleKind::English, Some("en")),
            alt("Kimi no Na wa.", TitleKind::Romanized, None),
            alt("君の名は。", TitleKind::Native, Some("ja")),
        ]
    );
}

#[test]
fn romaji_only_falls_back_for_title_and_is_listed() {
    let mut m = make_media(11, Some("TV"));
    m.title = Some(title(None, Some("Shingeki no Kyojin"), None));
    let show = anilist_media_to_tv(m).expect("valid id");
    assert_eq!(show.name, "Shingeki no Kyojin");
    assert_eq!(show.original_name, None);
    assert_eq!(
        show.alternative_titles,
        vec![alt("Shingeki no Kyojin", TitleKind::Romanized, None)]
    );
}

#[test]
fn synonyms_are_mapped_after_title_variants() {
    let mut m = make_media(12, Some("TV"));
    m.title = Some(title(Some("Attack on Titan"), None, Some("進撃の巨人")));
    m.synonyms = Some(vec!["AoT".to_string(), "SnK".to_string()]);
    // Unknown origin country: native title carries no language.
    m.country_of_origin = Some("US".to_string());
    let show = anilist_media_to_tv(m).expect("valid id");
    assert_eq!(
        show.alternative_titles,
        vec![
            alt("Attack on Titan", TitleKind::English, Some("en")),
            alt("進撃の巨人", TitleKind::Native, None),
            alt("AoT", TitleKind::Synonym, None),
            alt("SnK", TitleKind::Synonym, None),
        ]
    );
}

#[test]
fn null_and_empty_variants_are_skipped_and_duplicates_dropped() {
    let mut m = make_media(13, Some("MOVIE"));
    m.title = Some(title(Some(""), Some("Same"), None));
    m.synonyms = Some(vec![
        "  ".to_string(),
        "Same".to_string(),
        "Alias".to_string(),
        "Alias".to_string(),
    ]);
    let movie = anilist_media_to_movie(m).expect("valid id");
    assert_eq!(
        movie.alternative_titles,
        vec![
            alt("Same", TitleKind::Romanized, None),
            alt("Alias", TitleKind::Synonym, None),
        ]
    );
}

#[test]
fn identical_text_under_different_kinds_is_kept() {
    let titles = alternative_titles(Some(&title(Some("Mob"), Some("Mob"), None)), None, None);
    assert_eq!(
        titles,
        vec![
            alt("Mob", TitleKind::English, Some("en")),
            alt("Mob", TitleKind::Romanized, None),
        ]
    );
}

#[test]
fn absent_title_and_synonyms_yield_no_alternatives() {
    let movie = anilist_media_to_movie(make_media(14, Some("MOVIE"))).expect("valid id");
    assert!(movie.alternative_titles.is_empty());
}

#[test]
fn detail_conversions_carry_alternative_titles() {
    let expected = vec![
        alt("English", TitleKind::English, Some("en")),
        alt("Romaji", TitleKind::Romanized, None),
        alt("ネイティブ", TitleKind::Native, Some("ja")),
        alt("Alias", TitleKind::Synonym, None),
    ];
    let make = || {
        make_detail(
            Some(title(Some("English"), Some("Romaji"), Some("ネイティブ"))),
            Some(vec!["Alias".to_string()]),
        )
    };
    let movie = anilist_media_detail_to_movie_details(make()).expect("valid id");
    assert_eq!(movie.movie.title, "English");
    assert_eq!(movie.movie.alternative_titles, expected);
    let show = anilist_media_detail_to_tv_details(make()).expect("valid id");
    assert_eq!(show.show.name, "English");
    assert_eq!(show.show.alternative_titles, expected);
}
