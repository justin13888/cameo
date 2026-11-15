//! AniList media → unified model conversions.

use super::{Kind, classify, media_id, resolve_date};
use crate::{
    providers::anilist::response::{AniListMedia, AniListMediaDetail, AniListTitle},
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
mod tests;
