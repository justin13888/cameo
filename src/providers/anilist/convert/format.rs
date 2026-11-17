//! The AniList anime formats each unified media type covers.

use serde_json::Value;

use crate::providers::anilist::discover::AnimeFormat;

/// AniList formats for anime that map to the "movie" media type.
pub(crate) const MOVIE_FORMATS: &[AnimeFormat] = AnimeFormat::MOVIES;

/// AniList formats for anime that map to the "TV show" media type.
pub(crate) const TV_FORMATS: &[AnimeFormat] = AnimeFormat::TV;

/// The unified media type an AniList anime format maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A movie: a format in [`MOVIE_FORMATS`].
    Movie,
    /// A TV show: a format in [`TV_FORMATS`].
    Tv,
}

/// Classify an AniList `format` value (e.g. `"MOVIE"`, `"TV"`).
///
/// The single source of truth shared by the `formatIn` lists the movie and TV
/// queries send ([`MOVIE_FORMATS`], [`TV_FORMATS`]) and the conversions that
/// sort mixed results: a format in neither list (e.g. `MUSIC`) or an unknown
/// value is `None`, so no result set reports it as a movie or a show.
pub(crate) fn classify(format: &str) -> Option<Kind> {
    let in_list = |formats: &[AnimeFormat]| formats.iter().any(|f| f.as_str() == format);
    if in_list(MOVIE_FORMATS) {
        Some(Kind::Movie)
    } else if in_list(TV_FORMATS) {
        Some(Kind::Tv)
    } else {
        None
    }
}

/// The `formatIn` variable for `formats`: a JSON array of their AniList names.
pub(crate) fn format_in_value(formats: &[AnimeFormat]) -> Value {
    Value::Array(
        formats
            .iter()
            .map(|f| Value::String(f.as_str().to_string()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn classify_matches_the_format_in_lists() {
        for f in MOVIE_FORMATS {
            assert_eq!(classify(f.as_str()), Some(Kind::Movie), "{f:?}");
        }
        for f in TV_FORMATS {
            assert_eq!(classify(f.as_str()), Some(Kind::Tv), "{f:?}");
        }
    }

    #[test]
    fn classify_formats_outside_both_lists_are_none() {
        assert_eq!(classify(AnimeFormat::Music.as_str()), None);
        assert_eq!(classify("UNKNOWN"), None);
        assert_eq!(classify(""), None);
    }

    #[test]
    fn format_in_value_lists_the_anilist_names_in_order() {
        assert_eq!(format_in_value(MOVIE_FORMATS), json!(["MOVIE"]));
        assert_eq!(
            format_in_value(TV_FORMATS),
            json!(["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"])
        );
        assert_eq!(format_in_value(&[]), json!([]));
    }
}
