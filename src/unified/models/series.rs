use serde::{Deserialize, Serialize};

/// The release status of a TV series, unified across providers.
///
/// Distinct from the discover filter
/// `providers::tmdb::builders::TvStatus`, which encodes TMDB's numeric
/// `with_status` codes; this type describes a show a provider returned.
/// A status no variant covers is kept verbatim in [`SeriesStatus::Other`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeriesStatus {
    /// Still airing or renewed (TMDB `"Returning Series"`, AniList `"RELEASING"`).
    ReturningSeries,
    /// Announced but not yet in production or released (TMDB `"Planned"`,
    /// AniList `"NOT_YET_RELEASED"`).
    Planned,
    /// In production (TMDB `"In Production"`).
    InProduction,
    /// Finished airing (TMDB `"Ended"`, AniList `"FINISHED"`).
    Ended,
    /// Cancelled (TMDB `"Canceled"`, AniList `"CANCELLED"`).
    Cancelled,
    /// Only a pilot exists (TMDB `"Pilot"`).
    Pilot,
    /// On hiatus (AniList `"HIATUS"`).
    Hiatus,
    /// A status not covered by the known variants, as the provider reported it.
    Other(String),
}

impl SeriesStatus {
    /// Map a TMDB TV `status` string (case-insensitive).
    #[cfg(feature = "tmdb")]
    pub(crate) fn from_tmdb(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "returning series" => SeriesStatus::ReturningSeries,
            "planned" => SeriesStatus::Planned,
            "in production" => SeriesStatus::InProduction,
            "ended" => SeriesStatus::Ended,
            "canceled" | "cancelled" => SeriesStatus::Cancelled,
            "pilot" => SeriesStatus::Pilot,
            _ => SeriesStatus::Other(value.to_string()),
        }
    }

    /// Map an AniList `MediaStatus` value (case-insensitive).
    #[cfg(feature = "anilist")]
    pub(crate) fn from_anilist(value: &str) -> Self {
        match value.to_ascii_uppercase().as_str() {
            "RELEASING" => SeriesStatus::ReturningSeries,
            "NOT_YET_RELEASED" => SeriesStatus::Planned,
            "FINISHED" => SeriesStatus::Ended,
            "CANCELLED" => SeriesStatus::Cancelled,
            "HIATUS" => SeriesStatus::Hiatus,
            _ => SeriesStatus::Other(value.to_string()),
        }
    }
}

/// The classification of a TV series, unified across providers.
///
/// Distinct from the discover filter
/// `providers::tmdb::builders::TvShowType`, which encodes TMDB's numeric
/// `with_type` codes; this type describes a show a provider returned.
/// A type no variant covers is kept verbatim in [`SeriesType::Other`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeriesType {
    /// Documentary.
    Documentary,
    /// News.
    News,
    /// Miniseries.
    Miniseries,
    /// Reality.
    Reality,
    /// Scripted.
    Scripted,
    /// Talk show.
    TalkShow,
    /// Video.
    Video,
    /// A type not covered by the known variants, as the provider reported it.
    Other(String),
}

impl SeriesType {
    /// Map a TMDB TV `type` string (case-insensitive).
    #[cfg(feature = "tmdb")]
    pub(crate) fn from_tmdb(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "documentary" => SeriesType::Documentary,
            "news" => SeriesType::News,
            "miniseries" => SeriesType::Miniseries,
            "reality" => SeriesType::Reality,
            "scripted" => SeriesType::Scripted,
            "talk show" => SeriesType::TalkShow,
            "video" => SeriesType::Video,
            _ => SeriesType::Other(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SeriesStatus;

    #[cfg(feature = "tmdb")]
    #[test]
    fn series_status_from_tmdb_maps_every_known_value() {
        for (raw, expected) in [
            ("Returning Series", SeriesStatus::ReturningSeries),
            ("Planned", SeriesStatus::Planned),
            ("In Production", SeriesStatus::InProduction),
            ("Ended", SeriesStatus::Ended),
            ("Canceled", SeriesStatus::Cancelled),
            ("Cancelled", SeriesStatus::Cancelled),
            ("Pilot", SeriesStatus::Pilot),
            ("ended", SeriesStatus::Ended),
        ] {
            assert_eq!(SeriesStatus::from_tmdb(raw), expected, "{raw}");
        }
        assert_eq!(
            SeriesStatus::from_tmdb("Rumored"),
            SeriesStatus::Other("Rumored".to_string())
        );
    }

    #[cfg(feature = "tmdb")]
    #[test]
    fn series_type_from_tmdb_maps_every_known_value() {
        use super::SeriesType;
        for (raw, expected) in [
            ("Documentary", SeriesType::Documentary),
            ("News", SeriesType::News),
            ("Miniseries", SeriesType::Miniseries),
            ("Reality", SeriesType::Reality),
            ("Scripted", SeriesType::Scripted),
            ("Talk Show", SeriesType::TalkShow),
            ("Video", SeriesType::Video),
        ] {
            assert_eq!(SeriesType::from_tmdb(raw), expected, "{raw}");
        }
        assert_eq!(
            SeriesType::from_tmdb("Anthology"),
            SeriesType::Other("Anthology".to_string())
        );
    }

    #[cfg(feature = "anilist")]
    #[test]
    fn series_status_from_anilist_maps_every_known_value() {
        for (raw, expected) in [
            ("RELEASING", SeriesStatus::ReturningSeries),
            ("NOT_YET_RELEASED", SeriesStatus::Planned),
            ("FINISHED", SeriesStatus::Ended),
            ("CANCELLED", SeriesStatus::Cancelled),
            ("HIATUS", SeriesStatus::Hiatus),
        ] {
            assert_eq!(SeriesStatus::from_anilist(raw), expected, "{raw}");
        }
        assert_eq!(
            SeriesStatus::from_anilist("UNKNOWN"),
            SeriesStatus::Other("UNKNOWN".to_string())
        );
    }

    #[test]
    fn series_status_round_trips_through_serde() {
        for status in [
            SeriesStatus::Ended,
            SeriesStatus::Other("Rumored".to_string()),
        ] {
            let json = serde_json::to_string(&status).expect("serializes");
            let back: SeriesStatus = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(back, status);
        }
    }
}
