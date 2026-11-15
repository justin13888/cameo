//! Conversions from AniList GraphQL response types to unified models.

pub mod credits;
pub mod media;
pub mod person;

pub use media::*;
pub use person::*;

use crate::{providers::anilist::response::AniListDate, unified::date::PartialDate};

/// Convert an AniList fuzzy date into a [`PartialDate`], preserving exactly the
/// components AniList supplied (year-only and year-month dates stay partial).
///
/// Returns `None` when the year is absent or the components are out of range.
pub(super) fn resolve_date(date: &AniListDate) -> Option<PartialDate> {
    let year = date.year?;
    let month = date.month.and_then(|m| u8::try_from(m).ok());
    let day = date.day.and_then(|d| u8::try_from(d).ok());
    // A day without a month is inconsistent; drop the day rather than the date.
    let (month, day) = match (month, day) {
        (None, Some(_)) => (None, None),
        pair => pair,
    };
    PartialDate::new(year, month, day)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::*;
    use crate::providers::anilist::response::{AniListDate, AniListMedia, AniListStaff};

    /// Deserialize a fixture from AniList's wire JSON, so fixtures track the
    /// response schema rather than the response types' Rust fields.
    fn from_json<T: DeserializeOwned>(value: Value) -> T {
        serde_json::from_value(value).expect("fixture matches the AniList response shape")
    }

    /// A fully-populated list-media object as AniList returns it.
    fn media_json(id: i32, format: &str) -> Value {
        json!({
            "id": id,
            "title": {
                "romaji": "Romaji Title",
                "english": "English Title",
                "native": "ネイティブ"
            },
            "synonyms": ["Synonym"],
            "description": "A description.",
            "startDate": { "year": 2022, "month": 4, "day": 1 },
            "coverImage": {
                "large": "https://example.com/large.jpg",
                "extraLarge": "https://example.com/xl.jpg"
            },
            "bannerImage": "https://example.com/banner.jpg",
            "genres": ["Action", "Mecha"],
            "popularity": 100000,
            "averageScore": 78,
            "episodes": 12,
            "duration": 24,
            "status": "FINISHED",
            "format": format,
            "countryOfOrigin": "JP",
            "isAdult": false
        })
    }

    fn make_media(id: i32, format: &str) -> AniListMedia {
        from_json(media_json(id, format))
    }

    fn date(value: Value) -> AniListDate {
        from_json(value)
    }

    #[test]
    fn test_media_to_movie_basic_fields() {
        let movie = anilist_media_to_movie(make_media(42, "MOVIE"));
        assert_eq!(movie.provider_id, "anilist:42");
        assert_eq!(movie.title, "English Title");
        assert_eq!(movie.original_title.as_deref(), Some("ネイティブ"));
        assert_eq!(
            movie.release_date.map(|d| d.to_string()).as_deref(),
            Some("2022-04-01")
        );
        assert_eq!(
            movie.poster_url.as_deref(),
            Some("https://example.com/xl.jpg")
        );
        assert_eq!(movie.vote_average, Some(7.8));
        assert_eq!(movie.popularity, Some(100000.0));
        assert_eq!(movie.original_language.as_deref(), Some("ja"));
        assert_eq!(movie.adult, Some(false));
    }

    #[test]
    fn test_unmapped_country_yields_no_original_language() {
        let mut movie_media = media_json(7, "MOVIE");
        movie_media["countryOfOrigin"] = json!("US");
        let movie = anilist_media_to_movie(from_json(movie_media));
        assert_eq!(movie.original_language, None);

        let mut tv_media = media_json(8, "TV");
        tv_media["countryOfOrigin"] = json!("US");
        let tv = anilist_media_to_tv(from_json(tv_media));
        assert_eq!(tv.original_language, None);
        assert_eq!(tv.origin_country, vec!["US"]);
    }

    #[test]
    fn test_media_to_movie_genre_mapping() {
        use crate::unified::genre::Genre;
        let movie = anilist_media_to_movie(make_media(1, "MOVIE"));
        assert!(movie.genres.contains(&Genre::Action));
        assert!(movie.genres.contains(&Genre::Mecha));
    }

    #[test]
    fn test_media_to_tv_basic_fields() {
        let tv = anilist_media_to_tv(make_media(99, "TV"));
        assert_eq!(tv.provider_id, "anilist:99");
        assert_eq!(tv.name, "English Title");
        assert_eq!(
            tv.first_air_date.map(|d| d.to_string()).as_deref(),
            Some("2022-04-01")
        );
        assert_eq!(tv.origin_country, vec!["JP"]);
    }

    #[test]
    fn test_media_to_movie_runtime_and_status() {
        let movie = anilist_media_to_movie(make_media(1, "MOVIE"));
        assert_eq!(movie.runtime, Some(24));
        assert_eq!(movie.status.as_deref(), Some("FINISHED"));
    }

    #[test]
    fn test_media_to_tv_episodes_duration_and_status() {
        let tv = anilist_media_to_tv(make_media(2, "TV"));
        assert_eq!(tv.number_of_episodes, Some(12));
        assert_eq!(tv.episode_run_time, Some(24));
        assert_eq!(tv.status, Some(crate::unified::models::SeriesStatus::Ended));
    }

    #[test]
    fn test_media_list_fields_absent_or_negative() {
        let mut m = media_json(3, "TV");
        m["episodes"] = Value::Null;
        m["duration"] = json!(-5);
        m["status"] = Value::Null;
        let tv = anilist_media_to_tv(from_json(m));
        assert_eq!(tv.number_of_episodes, None);
        // Negative values clamp to 0, matching the detail conversion.
        assert_eq!(tv.episode_run_time, Some(0));
        assert_eq!(tv.status, None);
    }

    #[test]
    fn test_search_result_dispatch_by_format() {
        use crate::unified::models::UnifiedSearchResult;
        let movie_result = anilist_media_to_search_result(make_media(1, "MOVIE"));
        assert!(matches!(movie_result, Some(UnifiedSearchResult::Movie(_))));

        let tv_result = anilist_media_to_search_result(make_media(2, "TV"));
        assert!(matches!(tv_result, Some(UnifiedSearchResult::TvShow(_))));

        let ova_result = anilist_media_to_search_result(make_media(3, "OVA"));
        assert!(matches!(ova_result, Some(UnifiedSearchResult::TvShow(_))));

        let music_result = anilist_media_to_search_result(make_media(4, "MUSIC"));
        assert!(music_result.is_none());
    }

    #[test]
    fn test_staff_to_person() {
        let staff: AniListStaff = from_json(json!({
            "id": 123,
            "name": { "full": "Test Person", "native": "テスト", "alternative": [] },
            "image": { "large": "https://example.com/p.jpg" },
            "primaryOccupations": ["Voice Actor"]
        }));
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
    fn test_partial_date_preserves_precision() {
        use crate::unified::date::PartialDate;

        let full = date(json!({ "year": 2022, "month": 4, "day": 1 }));
        assert_eq!(resolve_date(&full), PartialDate::ymd(2022, 4, 1));

        let partial = date(json!({ "year": 2022, "month": 4, "day": null }));
        assert_eq!(resolve_date(&partial), PartialDate::year_month(2022, 4));

        let year_only = date(json!({ "year": 2022, "month": null, "day": null }));
        assert_eq!(resolve_date(&year_only), Some(PartialDate::from_year(2022)));

        let empty = date(json!({ "year": null, "month": null, "day": null }));
        assert_eq!(resolve_date(&empty), None);
    }

    #[test]
    fn test_from_anilist_genre() {
        use crate::unified::genre::Genre;
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
