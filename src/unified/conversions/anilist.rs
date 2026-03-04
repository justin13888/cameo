//! Conversions from AniList GraphQL response types to unified models.

use crate::{
    providers::anilist::response::{
        AniListMedia, AniListMediaDetail, AniListStaff, AniListStaffDetail,
    },
    unified::{
        genre::Genre,
        models::{
            UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
            UnifiedSearchResult, UnifiedTvShow, UnifiedTvShowDetails,
        },
    },
};

// ── Helpers ───────────────────────────────────────────────────────────────────

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
        runtime: m.duration.map(|d| d as u32),
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
    let episode_run_time = m.duration.map(|d| vec![d as u32]).unwrap_or_default();

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
            origin_country,
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
        production_countries: Vec::new(),
    }
}

// ── AniListStaff → UnifiedPerson ──────────────────────────────────────────────

/// Convert an AniList staff member to a [`UnifiedPerson`].
pub fn staff_to_person(s: AniListStaff) -> UnifiedPerson {
    let department = s
        .primary_occupations
        .as_deref()
        .and_then(|occ| occ.first())
        .map(|occ| normalize_occupation(occ));

    UnifiedPerson {
        provider_id: format!("anilist:staff:{}", s.id),
        name: s
            .name
            .as_ref()
            .and_then(|n| n.full.clone())
            .unwrap_or_default(),
        known_for_department: department,
        profile_url: s.image.and_then(|i| i.large),
        popularity: None,
        gender: None,
        adult: false,
    }
}

/// Convert an AniList staff detail to [`UnifiedPersonDetails`].
pub fn staff_detail_to_person_details(s: AniListStaffDetail) -> UnifiedPersonDetails {
    let department = s
        .primary_occupations
        .as_deref()
        .and_then(|occ| occ.first())
        .map(|occ| normalize_occupation(occ));

    let also_known_as = s
        .name
        .as_ref()
        .map(|n| {
            let mut names: Vec<String> = n.alternative.clone();
            if let Some(native) = &n.native {
                names.push(native.clone());
            }
            names
        })
        .unwrap_or_default();

    UnifiedPersonDetails {
        person: UnifiedPerson {
            provider_id: format!("anilist:staff:{}", s.id),
            name: s
                .name
                .as_ref()
                .and_then(|n| n.full.clone())
                .unwrap_or_default(),
            known_for_department: department,
            profile_url: s.image.and_then(|i| i.large),
            popularity: None,
            gender: None,
            adult: false,
        },
        biography: s.description,
        birthday: s.date_of_birth.as_ref().and_then(|d| d.to_date_string()),
        deathday: s.date_of_death.as_ref().and_then(|d| d.to_date_string()),
        place_of_birth: s.home_town,
        imdb_id: None,
        homepage: s.site_url,
        also_known_as,
    }
}

/// Normalize an AniList primary occupation to a consistent department string.
fn normalize_occupation(occupation: &str) -> String {
    match occupation.to_lowercase().as_str() {
        "voice actor" | "voice actress" => "Voice Acting".to_string(),
        "director" => "Directing".to_string(),
        "music" | "composer" => "Music".to_string(),
        "animation director" | "animator" => "Animation".to_string(),
        "character design" => "Art".to_string(),
        _ => occupation.to_string(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::anilist::response::{
        AniListCoverImage, AniListDate, AniListStaffImage, AniListStaffName, AniListTitle,
    };

    fn make_media(id: i32, format: &str) -> AniListMedia {
        AniListMedia {
            id,
            title: Some(AniListTitle {
                romaji: Some("Romaji Title".to_string()),
                english: Some("English Title".to_string()),
                native: Some("ネイティブ".to_string()),
            }),
            description: Some("A description.".to_string()),
            start_date: Some(AniListDate {
                year: Some(2022),
                month: Some(4),
                day: Some(1),
            }),
            cover_image: Some(AniListCoverImage {
                large: Some("https://example.com/large.jpg".to_string()),
                extra_large: Some("https://example.com/xl.jpg".to_string()),
            }),
            banner_image: Some("https://example.com/banner.jpg".to_string()),
            genres: Some(vec!["Action".to_string(), "Mecha".to_string()]),
            popularity: Some(100000),
            average_score: Some(78),
            episodes: Some(24),
            duration: Some(23),
            status: Some("FINISHED".to_string()),
            format: Some(format.to_string()),
            country_of_origin: Some("JP".to_string()),
            is_adult: Some(false),
        }
    }

    #[test]
    fn test_media_to_movie_basic_fields() {
        let movie = anilist_media_to_movie(make_media(42, "MOVIE"));
        assert_eq!(movie.provider_id, "anilist:42");
        assert_eq!(movie.title, "English Title");
        assert_eq!(movie.original_title.as_deref(), Some("ネイティブ"));
        assert_eq!(movie.release_date.as_deref(), Some("2022-04-01"));
        assert_eq!(
            movie.poster_url.as_deref(),
            Some("https://example.com/xl.jpg")
        );
        assert_eq!(movie.vote_average, Some(7.8));
        assert_eq!(movie.popularity, Some(100000.0));
        assert_eq!(movie.original_language.as_deref(), Some("ja"));
        assert!(!movie.adult);
    }

    #[test]
    fn test_media_to_movie_genre_mapping() {
        let movie = anilist_media_to_movie(make_media(1, "MOVIE"));
        assert!(movie.genres.contains(&Genre::Action));
        assert!(movie.genres.contains(&Genre::Mecha));
    }

    #[test]
    fn test_media_to_tv_basic_fields() {
        let tv = anilist_media_to_tv(make_media(99, "TV"));
        assert_eq!(tv.provider_id, "anilist:99");
        assert_eq!(tv.name, "English Title");
        assert_eq!(tv.first_air_date.as_deref(), Some("2022-04-01"));
        assert_eq!(tv.origin_country, vec!["JP"]);
    }

    #[test]
    fn test_search_result_dispatch_by_format() {
        let movie_result = anilist_media_to_search_result(make_media(1, "MOVIE"));
        assert!(matches!(movie_result, UnifiedSearchResult::Movie(_)));

        let tv_result = anilist_media_to_search_result(make_media(2, "TV"));
        assert!(matches!(tv_result, UnifiedSearchResult::TvShow(_)));

        let ova_result = anilist_media_to_search_result(make_media(3, "OVA"));
        assert!(matches!(ova_result, UnifiedSearchResult::TvShow(_)));
    }

    #[test]
    fn test_score_conversion() {
        assert_eq!(score_to_vote_average(Some(85)), Some(8.5));
        assert_eq!(score_to_vote_average(Some(100)), Some(10.0));
        assert_eq!(score_to_vote_average(None), None);
    }

    #[test]
    fn test_staff_to_person() {
        let staff = AniListStaff {
            id: 123,
            name: Some(AniListStaffName {
                full: Some("Test Person".to_string()),
                native: Some("テスト".to_string()),
                alternative: vec![],
            }),
            image: Some(AniListStaffImage {
                large: Some("https://example.com/p.jpg".to_string()),
            }),
            description: None,
            primary_occupations: Some(vec!["Voice Actor".to_string()]),
            language: Some("Japanese".to_string()),
        };
        let person = staff_to_person(staff);
        assert_eq!(person.provider_id, "anilist:staff:123");
        assert_eq!(person.name, "Test Person");
        assert_eq!(person.known_for_department.as_deref(), Some("Voice Acting"));
        assert_eq!(
            person.profile_url.as_deref(),
            Some("https://example.com/p.jpg")
        );
    }

    #[test]
    fn test_date_formatting() {
        let full = AniListDate {
            year: Some(2022),
            month: Some(4),
            day: Some(1),
        };
        assert_eq!(full.to_date_string().as_deref(), Some("2022-04-01"));

        let partial = AniListDate {
            year: Some(2022),
            month: Some(4),
            day: None,
        };
        assert_eq!(partial.to_date_string().as_deref(), Some("2022-04"));

        let year_only = AniListDate {
            year: Some(2022),
            month: None,
            day: None,
        };
        assert_eq!(year_only.to_date_string().as_deref(), Some("2022"));

        let empty = AniListDate {
            year: None,
            month: None,
            day: None,
        };
        assert_eq!(empty.to_date_string(), None);
    }

    #[test]
    fn test_from_anilist_genre() {
        assert_eq!(Genre::from_anilist_genre("Mecha"), Genre::Mecha);
        assert_eq!(
            Genre::from_anilist_genre("Mahou Shoujo"),
            Genre::MahouShoujo
        );
        assert_eq!(
            Genre::from_anilist_genre("Slice of Life"),
            Genre::SliceOfLife
        );
        assert_eq!(Genre::from_anilist_genre("Sports"), Genre::Sports);
        assert_eq!(
            Genre::from_anilist_genre("Supernatural"),
            Genre::Supernatural
        );
        assert_eq!(Genre::from_anilist_genre("Ecchi"), Genre::Ecchi);
        assert_eq!(Genre::from_anilist_genre("Sci-Fi"), Genre::ScienceFiction);
        // Unknown genre falls through
        assert!(matches!(
            Genre::from_anilist_genre("Isekai"),
            crate::unified::genre::Genre::Other(_)
        ));
    }
}
