//! AniList media → unified model conversions.

use crate::{
    providers::anilist::response::{AniListMedia, AniListMediaDetail},
    unified::{
        genre::Genre,
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedSearchResult, UnifiedTvShow,
            UnifiedTvShowDetails,
        },
    },
};

// ── Private helpers ───────────────────────────────────────────────────────────

/// Resolve the best English title from an AniList media entry.
fn resolve_title(media: &AniListMedia) -> String {
    media
        .title
        .as_ref()
        .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
        .unwrap_or_default()
}

/// Resolve the best English title from an AniList detail entry.
fn resolve_detail_title(media: &AniListMediaDetail) -> String {
    media
        .title
        .as_ref()
        .and_then(|t| t.english.clone().or_else(|| t.romaji.clone()))
        .unwrap_or_default()
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
/// This is a best-effort mapping for common anime origins.
fn country_to_language(country: &str) -> String {
    match country {
        "JP" => "ja",
        "CN" => "zh",
        "KR" => "ko",
        "TW" => "zh",
        _ => country,
    }
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
        provider_id: format!("anilist:{}", m.id),
        title: resolve_title(&m),
        original_title: m.title.as_ref().and_then(|t| t.native.clone()),
        overview: m.description.clone(),
        release_date: m.start_date.as_ref().and_then(|d| d.to_date_string()),
        poster_url: resolve_cover(&m),
        backdrop_url: m.banner_image.clone(),
        genres: map_genres(&m.genres),
        popularity: m.popularity.map(|p| p as f64),
        vote_average: score_to_vote_average(m.average_score),
        vote_count: 0,
        original_language: m.country_of_origin.as_deref().map(country_to_language),
        adult: m.is_adult.unwrap_or(false),
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
        provider_id: format!("anilist:{}", m.id),
        name: resolve_title(&m),
        original_name: m.title.as_ref().and_then(|t| t.native.clone()),
        overview: m.description.clone(),
        first_air_date: m.start_date.as_ref().and_then(|d| d.to_date_string()),
        poster_url: resolve_cover(&m),
        backdrop_url: m.banner_image.clone(),
        genres: map_genres(&m.genres),
        popularity: m.popularity.map(|p| p as f64),
        vote_average: score_to_vote_average(m.average_score),
        vote_count: 0,
        original_language: lang,
        origin_country,
        adult: m.is_adult.unwrap_or(false),
    }
}

// ── AniListMedia → UnifiedSearchResult ───────────────────────────────────────

/// Dispatch an AniList media entry to [`UnifiedSearchResult::Movie`] or
/// [`UnifiedSearchResult::TvShow`] based on its `format` field.
pub fn anilist_media_to_search_result(m: AniListMedia) -> UnifiedSearchResult {
    match m.format.as_deref() {
        Some("MOVIE") => UnifiedSearchResult::Movie(anilist_media_to_movie(m)),
        _ => UnifiedSearchResult::TvShow(anilist_media_to_tv(m)),
    }
}

// ── AniListMediaDetail → UnifiedMovieDetails ──────────────────────────────────

/// Convert an AniList media detail entry to [`UnifiedMovieDetails`].
pub fn anilist_media_detail_to_movie_details(m: AniListMediaDetail) -> UnifiedMovieDetails {
    let production_companies: Vec<String> = m
        .studios
        .as_ref()
        .map(|s| s.nodes.iter().filter_map(|n| n.name.clone()).collect())
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
            provider_id: format!("anilist:{}", m.id),
            title: resolve_detail_title(&m),
            original_title: m.title.as_ref().and_then(|t| t.native.clone()),
            overview: m.description.clone(),
            release_date: m.start_date.as_ref().and_then(|d| d.to_date_string()),
            poster_url: resolve_cover_detail(&m),
            backdrop_url: m.banner_image.clone(),
            genres,
            popularity: m.popularity.map(|p| p as f64),
            vote_average: score_to_vote_average(m.average_score),
            vote_count: 0,
            original_language: origin_lang,
            adult: m.is_adult.unwrap_or(false),
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
    let production_companies: Vec<String> = m
        .studios
        .as_ref()
        .map(|s| s.nodes.iter().filter_map(|n| n.name.clone()).collect())
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
            provider_id: format!("anilist:{}", m.id),
            name: resolve_detail_title(&m),
            original_name: m.title.as_ref().and_then(|t| t.native.clone()),
            overview: m.description.clone(),
            first_air_date: m.start_date.as_ref().and_then(|d| d.to_date_string()),
            poster_url: resolve_cover_detail(&m),
            backdrop_url: m.banner_image.clone(),
            genres,
            popularity: m.popularity.map(|p| p as f64),
            vote_average: score_to_vote_average(m.average_score),
            vote_count: 0,
            original_language: origin_lang,
            origin_country: origin_country.clone(),
            adult: m.is_adult.unwrap_or(false),
        },
        tagline: None,
        // AniList does not expose a season count for anime.
        number_of_seasons: 1,
        number_of_episodes: m.episodes.unwrap_or(0) as u32,
        in_production,
        status: m.status,
        homepage: None,
        networks: Vec::new(),
        production_companies,
        last_air_date: m.end_date.as_ref().and_then(|d| d.to_date_string()),
        type_: m.format,
        created_by: Vec::new(),
        episode_run_time,
        spoken_languages: Vec::new(),
        production_countries: origin_country,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::score_to_vote_average;

    #[test]
    fn test_score_conversion() {
        assert_eq!(score_to_vote_average(Some(85)), Some(8.5));
        assert_eq!(score_to_vote_average(Some(100)), Some(10.0));
        assert_eq!(score_to_vote_average(None), None);
    }
}
