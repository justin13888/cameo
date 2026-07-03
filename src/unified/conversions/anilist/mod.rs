//! Conversions from AniList GraphQL response types to unified models.

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
    use super::*;
    use crate::providers::anilist::response::{
        AniListCoverImage, AniListDate, AniListMedia, AniListStaffImage, AniListStaffName,
        AniListTitle,
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
    fn test_search_result_dispatch_by_format() {
        use crate::unified::models::UnifiedSearchResult;
        let movie_result = anilist_media_to_search_result(make_media(1, "MOVIE"));
        assert!(matches!(movie_result, Some(UnifiedSearchResult::Movie(_))));

        let tv_result = anilist_media_to_search_result(make_media(2, "TV"));
        assert!(matches!(tv_result, Some(UnifiedSearchResult::TvShow(_))));

        let ova_result = anilist_media_to_search_result(make_media(3, "OVA"));
        assert!(matches!(ova_result, Some(UnifiedSearchResult::TvShow(_))));
    }

    #[test]
    fn test_staff_to_person() {
        use crate::providers::anilist::response::AniListStaff;
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
    fn test_partial_date_preserves_precision() {
        use crate::unified::date::PartialDate;

        let full = AniListDate {
            year: Some(2022),
            month: Some(4),
            day: Some(1),
        };
        assert_eq!(resolve_date(&full), PartialDate::ymd(2022, 4, 1));

        let partial = AniListDate {
            year: Some(2022),
            month: Some(4),
            day: None,
        };
        assert_eq!(resolve_date(&partial), PartialDate::year_month(2022, 4));

        let year_only = AniListDate {
            year: Some(2022),
            month: None,
            day: None,
        };
        assert_eq!(resolve_date(&year_only), Some(PartialDate::from_year(2022)));

        let empty = AniListDate {
            year: None,
            month: None,
            day: None,
        };
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
