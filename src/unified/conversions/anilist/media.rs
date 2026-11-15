//! AniList media → unified model conversions.

use super::{media_id, resolve_date};
use crate::{
    providers::anilist::{
        client::{Kind, classify},
        response::{AniListMedia, AniListMediaDetail, AniListTitle},
    },
    unified::{
        genre::Genre,
        models::{
            AlternativeTitle, Company, SeriesStatus, TitleKind, UnifiedMovie, UnifiedMovieDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
    },
};

// ── Private helpers ───────────────────────────────────────────────────────────

/// Resolve the headline title: English if present, else romaji.
fn resolve_title(title: Option<&AniListTitle>) -> String {
    title
        .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
        .unwrap_or_default()
}

/// Collect every title variant AniList reports into [`AlternativeTitle`]s.
///
/// Order: English, romanized (romaji), native, then synonyms in AniList
/// order. Absent, empty, or whitespace-only strings are skipped.
///
/// - English carries language `"en"`.
/// - Romaji carries no language: it is a Latin-script transliteration whose
///   source language is not reliably known (it is *usually* Japanese).
/// - Native carries the language mapped from `countryOfOrigin` when the
///   mapping is known (`JP`→`ja`, `CN`/`TW`→`zh`, `KR`→`ko`), else none.
/// - Synonyms carry no language (AniList mixes languages in that list).
///
/// The same text under *different* kinds is kept (e.g. an English title equal
/// to the romaji one yields both an English and a Romanized entry), since the
/// kind itself is information. Exact duplicates are dropped: a synonym whose
/// text already appears as an earlier entry (a title variant or an earlier
/// synonym) is skipped.
fn alternative_titles(
    title: Option<&AniListTitle>,
    synonyms: Option<&[String]>,
    country_of_origin: Option<&str>,
) -> Vec<AlternativeTitle> {
    let native_lang = country_of_origin.and_then(country_to_language);

    let mut out: Vec<AlternativeTitle> = Vec::new();
    if let Some(t) = title {
        if let Some(english) = non_empty(&t.english) {
            out.push(AlternativeTitle::new(english, TitleKind::English).with_language("en"));
        }
        if let Some(romaji) = non_empty(&t.romaji) {
            out.push(AlternativeTitle::new(romaji, TitleKind::Romanized));
        }
        if let Some(native) = non_empty(&t.native) {
            let mut alt = AlternativeTitle::new(native, TitleKind::Native);
            alt.language = native_lang.map(String::from);
            out.push(alt);
        }
    }
    for synonym in synonyms.unwrap_or_default() {
        if synonym.trim().is_empty() || out.iter().any(|a| a.title == *synonym) {
            continue;
        }
        out.push(AlternativeTitle::new(synonym.as_str(), TitleKind::Synonym));
    }
    out
}

/// Borrow a string only if it is present and not empty/whitespace-only.
fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
}

/// Resolve the cover image URL (prefer extra-large).
fn resolve_cover(media: &AniListMedia) -> Option<String> {
    media
        .cover_image
        .as_ref()
        .and_then(|c| c.extra_large.clone().or_else(|| c.large.clone()))
}

fn resolve_cover_detail(media: &AniListMediaDetail) -> Option<String> {
    media
        .cover_image
        .as_ref()
        .and_then(|c| c.extra_large.clone().or_else(|| c.large.clone()))
}

/// Convert AniList `averageScore` (0–100) to a 0–10 vote average.
fn score_to_vote_average(score: Option<i32>) -> Option<f64> {
    score.map(|s| s as f64 / 10.0)
}

/// Convert an ISO 3166-1 alpha-2 country code to a rough ISO 639-1 language code.
///
/// This is a best-effort mapping for common anime origins. Returns `None` for
/// countries without a known mapping rather than leaking the country code into
/// a language field.
fn country_to_language(country: &str) -> Option<&'static str> {
    match country {
        "JP" => Some("ja"),
        "CN" => Some("zh"),
        "KR" => Some("ko"),
        "TW" => Some("zh"),
        _ => None,
    }
}

/// Clamp an AniList count/minutes field (`episodes`, `duration`) to `u32`.
///
/// Shared by the list and detail conversions so a summary and its details
/// always report the same runtime / episode count for the same media.
fn non_negative(value: Option<i32>) -> Option<u32> {
    value.map(|v| v.max(0) as u32)
}

fn map_genres(genres: &Option<Vec<String>>) -> Vec<Genre> {
    genres
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|g| Genre::from_anilist_genre(g))
        .collect()
}

// ── AniListMedia → UnifiedMovie ───────────────────────────────────────────────

/// Convert an AniList media entry to a [`UnifiedMovie`], or `None` (logged at
/// `warn`) when its id is not a valid AniList id (zero or negative).
pub fn anilist_media_to_movie(m: AniListMedia) -> Option<UnifiedMovie> {
    Some(UnifiedMovie {
        provider_id: media_id(m.id)?,
        title: resolve_title(m.title.as_ref()),
        original_title: m.title.as_ref().and_then(|t| t.native.clone()),
        alternative_titles: alternative_titles(
            m.title.as_ref(),
            m.synonyms.as_deref(),
            m.country_of_origin.as_deref(),
        ),
        overview: m.description.clone(),
        release_date: m.start_date.as_ref().and_then(resolve_date),
        poster_url: resolve_cover(&m),
        backdrop_url: m.banner_image.clone(),
        genres: map_genres(&m.genres),
        popularity: m.popularity.map(|p| p as f64),
        vote_average: score_to_vote_average(m.average_score),
        // AniList exposes an average score but no vote count.
        vote_count: None,
        original_language: m
            .country_of_origin
            .as_deref()
            .and_then(country_to_language)
            .map(str::to_owned),
        adult: m.is_adult,
        // A movie's `duration` is its full runtime.
        runtime: non_negative(m.duration),
        // AniList's raw `MediaStatus`, exactly as the detail conversion reports it.
        status: m.status,
    })
}

// ── AniListMedia → UnifiedTvShow ──────────────────────────────────────────────

/// Convert an AniList media entry to a [`UnifiedTvShow`], or `None` (logged at
/// `warn`) when its id is not a valid AniList id (zero or negative).
pub fn anilist_media_to_tv(m: AniListMedia) -> Option<UnifiedTvShow> {
    let provider_id = media_id(m.id)?;
    let origin_country = m
        .country_of_origin
        .as_deref()
        .map(|c| vec![c.to_string()])
        .unwrap_or_default();
    let lang = m
        .country_of_origin
        .as_deref()
        .and_then(country_to_language)
        .map(str::to_owned);

    Some(UnifiedTvShow {
        provider_id,
        name: resolve_title(m.title.as_ref()),
        original_name: m.title.as_ref().and_then(|t| t.native.clone()),
        alternative_titles: alternative_titles(
            m.title.as_ref(),
            m.synonyms.as_deref(),
            m.country_of_origin.as_deref(),
        ),
        overview: m.description.clone(),
        first_air_date: m.start_date.as_ref().and_then(resolve_date),
        poster_url: resolve_cover(&m),
        backdrop_url: m.banner_image.clone(),
        genres: map_genres(&m.genres),
        popularity: m.popularity.map(|p| p as f64),
        vote_average: score_to_vote_average(m.average_score),
        // AniList exposes an average score but no vote count.
        vote_count: None,
        original_language: lang,
        origin_country,
        adult: m.is_adult,
        number_of_episodes: non_negative(m.episodes),
        episode_run_time: non_negative(m.duration),
        // AniList's `MediaStatus`, mapped exactly as the detail conversion maps it.
        status: m.status.as_deref().map(SeriesStatus::from_anilist),
    })
}

// ── AniListMedia → UnifiedSearchResult ───────────────────────────────────────

/// Dispatch an AniList media entry to a [`UnifiedSearchResult`] based on its
/// `format`.
///
/// Shared unknown-media policy: an entry whose `format` is absent is **omitted**
/// (`None`) rather than coerced to an arbitrary type — the same rule TMDB
/// applies to an unrecognized `media_type`. A present format is classified
/// exactly as the movie and TV list queries filter: `MOVIE` is a movie; `TV`,
/// `TV_SHORT`, `ONA`, `OVA`, and `SPECIAL` are shows; any other format (e.g.
/// `MUSIC`) is omitted too, since no movie or TV list would ever return it.
/// An entry whose id is not a valid AniList id is omitted (and logged).
pub fn anilist_media_to_search_result(m: AniListMedia) -> Option<UnifiedSearchResult> {
    match m.format.as_deref().and_then(classify)? {
        Kind::Movie => anilist_media_to_movie(m).map(UnifiedSearchResult::Movie),
        Kind::Tv => anilist_media_to_tv(m).map(UnifiedSearchResult::TvShow),
    }
}

// ── AniListMediaDetail → UnifiedMovieDetails ──────────────────────────────────

/// Convert an AniList media detail entry to [`UnifiedMovieDetails`], or `None`
/// (logged at `warn`) when its id is not a valid AniList id.
pub fn anilist_media_detail_to_movie_details(m: AniListMediaDetail) -> Option<UnifiedMovieDetails> {
    let provider_id = media_id(m.id)?;
    let production_companies: Vec<Company> = m
        .studios
        .as_ref()
        .map(|s| {
            s.nodes
                .iter()
                .filter_map(|n| n.name.clone())
                .map(|name| Company {
                    id: None,
                    name,
                    logo_url: None,
                    origin_country: None,
                })
                .collect()
        })
        .unwrap_or_default();

    let origin_lang = m
        .country_of_origin
        .as_deref()
        .and_then(country_to_language)
        .map(str::to_owned);
    let origin_country = m
        .country_of_origin
        .as_deref()
        .map(|c| vec![c.to_string()])
        .unwrap_or_default();

    let genres: Vec<Genre> = m
        .genres
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|g| Genre::from_anilist_genre(g))
        .collect();
    let runtime = non_negative(m.duration);

    Some(UnifiedMovieDetails {
        movie: UnifiedMovie {
            provider_id,
            title: resolve_title(m.title.as_ref()),
            original_title: m.title.as_ref().and_then(|t| t.native.clone()),
            alternative_titles: alternative_titles(
                m.title.as_ref(),
                m.synonyms.as_deref(),
                m.country_of_origin.as_deref(),
            ),
            overview: m.description.clone(),
            release_date: m.start_date.as_ref().and_then(resolve_date),
            poster_url: resolve_cover_detail(&m),
            backdrop_url: m.banner_image.clone(),
            genres,
            popularity: m.popularity.map(|p| p as f64),
            vote_average: score_to_vote_average(m.average_score),
            vote_count: None,
            original_language: origin_lang,
            adult: m.is_adult,
            // The embedded summary mirrors the detail-level runtime/status.
            runtime,
            status: m.status.clone(),
        },
        tagline: None,
        runtime,
        budget: None,
        revenue: None,
        status: m.status,
        homepage: None,
        imdb_id: None,
        production_companies,
        production_countries: origin_country,
        spoken_languages: Vec::new(),
        // AniList has no "video entry" concept.
        video: None,
        belongs_to_collection: None,
    })
}

// ── AniListMediaDetail → UnifiedTvShowDetails ─────────────────────────────────

/// Convert an AniList media detail entry to [`UnifiedTvShowDetails`], or
/// `None` (logged at `warn`) when its id is not a valid AniList id.
pub fn anilist_media_detail_to_tv_details(m: AniListMediaDetail) -> Option<UnifiedTvShowDetails> {
    let provider_id = media_id(m.id)?;
    let production_companies: Vec<Company> = m
        .studios
        .as_ref()
        .map(|s| {
            s.nodes
                .iter()
                .filter_map(|n| n.name.clone())
                .map(|name| Company {
                    id: None,
                    name,
                    logo_url: None,
                    origin_country: None,
                })
                .collect()
        })
        .unwrap_or_default();

    let origin_lang = m
        .country_of_origin
        .as_deref()
        .and_then(country_to_language)
        .map(str::to_owned);
    let origin_country = m
        .country_of_origin
        .as_deref()
        .map(|c| vec![c.to_string()])
        .unwrap_or_default();

    let genres: Vec<Genre> = m
        .genres
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|g| Genre::from_anilist_genre(g))
        .collect();

    // Unknown status -> unknown production state, rather than `false`.
    let in_production = m
        .status
        .as_deref()
        .map(|s| matches!(s, "RELEASING" | "NOT_YET_RELEASED"));
    let status = m.status.as_deref().map(SeriesStatus::from_anilist);
    let run_time = non_negative(m.duration);
    let number_of_episodes = non_negative(m.episodes);

    Some(UnifiedTvShowDetails {
        show: UnifiedTvShow {
            provider_id,
            name: resolve_title(m.title.as_ref()),
            original_name: m.title.as_ref().and_then(|t| t.native.clone()),
            alternative_titles: alternative_titles(
                m.title.as_ref(),
                m.synonyms.as_deref(),
                m.country_of_origin.as_deref(),
            ),
            overview: m.description.clone(),
            first_air_date: m.start_date.as_ref().and_then(resolve_date),
            poster_url: resolve_cover_detail(&m),
            backdrop_url: m.banner_image.clone(),
            genres,
            popularity: m.popularity.map(|p| p as f64),
            vote_average: score_to_vote_average(m.average_score),
            vote_count: None,
            original_language: origin_lang,
            origin_country: origin_country.clone(),
            adult: m.is_adult,
            // The embedded summary mirrors the detail-level episode data/status.
            number_of_episodes,
            episode_run_time: run_time,
            status: status.clone(),
        },
        tagline: None,
        // AniList models anime as a single continuous run, not TMDB-style seasons.
        number_of_seasons: None,
        number_of_episodes,
        in_production,
        status,
        homepage: None,
        networks: Vec::new(),
        production_companies,
        last_air_date: m.end_date.as_ref().and_then(resolve_date),
        // AniList has no TMDB-style show classification; its format lives in `media_format`.
        type_: None,
        media_format: m.format,
        created_by: Vec::new(),
        episode_run_time: run_time.into_iter().collect(),
        spoken_languages: Vec::new(),
        production_countries: origin_country,
        seasons: Vec::new(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        let details = anilist_media_detail_to_movie_details(make_list_fields_detail("MOVIE"))
            .expect("valid id");
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
}
