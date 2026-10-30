//! AniList media → unified model conversions.

use super::resolve_date;
use crate::{
    providers::anilist::response::{AniListMedia, AniListMediaDetail, AniListTitle},
    unified::{
        genre::Genre,
        media_id::MediaId,
        models::{
            AlternativeTitle, Company, TitleKind, UnifiedMovie, UnifiedMovieDetails,
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
    let native_lang = country_of_origin.and_then(known_country_language);

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

/// Map an ISO 3166-1 alpha-2 country code to an ISO 639-1 language code for
/// the common anime origins; `None` for any other country.
fn known_country_language(country: &str) -> Option<&'static str> {
    match country {
        "JP" => Some("ja"),
        "CN" | "TW" => Some("zh"),
        "KR" => Some("ko"),
        _ => None,
    }
}

/// Convert an ISO 3166-1 alpha-2 country code to a rough ISO 639-1 language code.
///
/// This is a best-effort mapping for common anime origins; an unknown country
/// code is passed through unchanged.
fn country_to_language(country: &str) -> String {
    known_country_language(country)
        .unwrap_or(country)
        .to_string()
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

/// Convert an AniList media entry to a [`UnifiedMovie`].
pub fn anilist_media_to_movie(m: AniListMedia) -> UnifiedMovie {
    UnifiedMovie {
        provider_id: MediaId::anilist(m.id as u64),
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
        original_language: m.country_of_origin.as_deref().map(country_to_language),
        adult: m.is_adult,
    }
}

// ── AniListMedia → UnifiedTvShow ──────────────────────────────────────────────

/// Convert an AniList media entry to a [`UnifiedTvShow`].
pub fn anilist_media_to_tv(m: AniListMedia) -> UnifiedTvShow {
    let origin_country = m
        .country_of_origin
        .as_deref()
        .map(|c| vec![c.to_string()])
        .unwrap_or_default();
    let lang = m.country_of_origin.as_deref().map(country_to_language);

    UnifiedTvShow {
        provider_id: MediaId::anilist(m.id as u64),
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
    }
}

// ── AniListMedia → UnifiedSearchResult ───────────────────────────────────────

/// Dispatch an AniList media entry to a [`UnifiedSearchResult`] based on its
/// `format`.
///
/// Shared unknown-media policy: an entry whose `format` is absent is **omitted**
/// (`None`) rather than coerced to an arbitrary type — the same rule TMDB
/// applies to an unrecognized `media_type`. Any present non-`MOVIE` AniList
/// format is a show.
pub fn anilist_media_to_search_result(m: AniListMedia) -> Option<UnifiedSearchResult> {
    match m.format.as_deref() {
        Some("MOVIE") => Some(UnifiedSearchResult::Movie(anilist_media_to_movie(m))),
        Some(_) => Some(UnifiedSearchResult::TvShow(anilist_media_to_tv(m))),
        None => None,
    }
}

// ── AniListMediaDetail → UnifiedMovieDetails ──────────────────────────────────

/// Convert an AniList media detail entry to [`UnifiedMovieDetails`].
pub fn anilist_media_detail_to_movie_details(m: AniListMediaDetail) -> UnifiedMovieDetails {
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

    let origin_lang = m.country_of_origin.as_deref().map(country_to_language);
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

    UnifiedMovieDetails {
        movie: UnifiedMovie {
            provider_id: MediaId::anilist(m.id as u64),
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
        },
        tagline: None,
        runtime: m.duration.map(|d| d.max(0) as u32),
        budget: None,
        revenue: None,
        status: m.status.clone(),
        homepage: None,
        imdb_id: None,
        production_companies,
        production_countries: origin_country,
        spoken_languages: Vec::new(),
        video: false,
        belongs_to_collection: None,
    }
}

// ── AniListMediaDetail → UnifiedTvShowDetails ─────────────────────────────────

/// Convert an AniList media detail entry to [`UnifiedTvShowDetails`].
pub fn anilist_media_detail_to_tv_details(m: AniListMediaDetail) -> UnifiedTvShowDetails {
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

    let origin_lang = m.country_of_origin.as_deref().map(country_to_language);
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

    let in_production = matches!(
        m.status.as_deref(),
        Some("RELEASING") | Some("NOT_YET_RELEASED")
    );
    let episode_run_time = m
        .duration
        .map(|d| vec![d.max(0) as u32])
        .unwrap_or_default();

    UnifiedTvShowDetails {
        show: UnifiedTvShow {
            provider_id: MediaId::anilist(m.id as u64),
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
        },
        tagline: None,
        // AniList models anime as a single continuous run, not TMDB-style seasons.
        number_of_seasons: None,
        number_of_episodes: m.episodes.map(|e| e.max(0) as u32),
        in_production,
        status: m.status,
        homepage: None,
        networks: Vec::new(),
        production_companies,
        last_air_date: m.end_date.as_ref().and_then(resolve_date),
        // AniList has no TMDB-style show classification; its format lives in `media_format`.
        type_: None,
        media_format: m.format,
        created_by: Vec::new(),
        episode_run_time,
        spoken_languages: Vec::new(),
        production_countries: origin_country,
        seasons: Vec::new(),
    }
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
            format: format.map(String::from),
            country_of_origin: None,
            is_adult: None,
        }
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
    fn search_result_other_present_format_is_a_show() {
        // Any present non-MOVIE format classifies as a show.
        let result = anilist_media_to_search_result(make_media(3, Some("OVA")));
        assert!(matches!(result, Some(UnifiedSearchResult::TvShow(_))));
    }

    #[test]
    fn search_result_absent_format_is_dropped() {
        // Shared unknown-media policy: no format -> omitted, not coerced.
        let result = anilist_media_to_search_result(make_media(4, None));
        assert!(result.is_none());
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
        let movie = anilist_media_to_movie(m);
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
        let show = anilist_media_to_tv(m);
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
        let show = anilist_media_to_tv(m);
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
        let movie = anilist_media_to_movie(m);
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
        let movie = anilist_media_to_movie(make_media(14, Some("MOVIE")));
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
        let movie = anilist_media_detail_to_movie_details(make());
        assert_eq!(movie.movie.title, "English");
        assert_eq!(movie.movie.alternative_titles, expected);
        let show = anilist_media_detail_to_tv_details(make());
        assert_eq!(show.show.name, "English");
        assert_eq!(show.show.alternative_titles, expected);
    }
}
