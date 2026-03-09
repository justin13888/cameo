//! Serde-deserializable types mirroring the AniList GraphQL schema.

use serde::Deserialize;

use super::error::AniListGqlError;

// ── GraphQL envelope ──────────────────────────────────────────────────────────

/// Top-level GraphQL response wrapper.
#[derive(Debug, Deserialize)]
pub(crate) struct GraphQlResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<AniListGqlError>>,
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// AniList pagination info returned by `Page.pageInfo`.
#[derive(Debug, Deserialize)]
pub struct PageInfo {
    /// Total items across all pages.
    pub total: Option<i32>,
    /// Current page number (1-indexed).
    #[serde(rename = "currentPage")]
    pub current_page: Option<i32>,
    /// Last available page number.
    #[serde(rename = "lastPage")]
    pub last_page: Option<i32>,
    /// Whether there is a next page.
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    /// Results per page.
    #[serde(rename = "perPage")]
    pub per_page: Option<i32>,
}

// ── Media types ───────────────────────────────────────────────────────────────

/// Localized title for an AniList media entry.
#[derive(Debug, Deserialize)]
pub struct AniListTitle {
    /// Title romanized (transliterated from Japanese).
    pub romaji: Option<String>,
    /// Official English title.
    pub english: Option<String>,
    /// Title in the original script (e.g. Japanese kanji).
    pub native: Option<String>,
}

/// AniList fuzzy date (partial; fields may be absent).
#[derive(Debug, Deserialize)]
pub struct AniListDate {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

impl AniListDate {
    /// Format as `YYYY-MM-DD`, `YYYY-MM`, or `YYYY` depending on available fields.
    pub fn to_date_string(&self) -> Option<String> {
        match (self.year, self.month, self.day) {
            (Some(y), Some(m), Some(d)) => Some(format!("{y:04}-{m:02}-{d:02}")),
            (Some(y), Some(m), None) => Some(format!("{y:04}-{m:02}")),
            (Some(y), None, _) => Some(format!("{y:04}")),
            _ => None,
        }
    }
}

/// Cover image URLs for a media entry.
#[derive(Debug, Deserialize)]
pub struct AniListCoverImage {
    /// Large cover image URL.
    pub large: Option<String>,
    /// Extra-large cover image URL (highest quality).
    #[serde(rename = "extraLarge")]
    pub extra_large: Option<String>,
}

/// An AniList studio.
#[derive(Debug, Deserialize)]
pub struct AniListStudio {
    pub name: Option<String>,
}

/// A list of studios with their nodes.
#[derive(Debug, Deserialize)]
pub struct AniListStudios {
    pub nodes: Vec<AniListStudio>,
}

/// Core AniList media entry (anime) — used in list/search results.
#[derive(Debug, Deserialize)]
pub struct AniListMedia {
    pub id: i32,
    pub title: Option<AniListTitle>,
    /// Synopsis (HTML stripped if `asHtml: false` in query).
    pub description: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<AniListDate>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<AniListCoverImage>,
    #[serde(rename = "bannerImage")]
    pub banner_image: Option<String>,
    pub genres: Option<Vec<String>>,
    pub popularity: Option<i32>,
    /// Score from 0–100.
    #[serde(rename = "averageScore")]
    pub average_score: Option<i32>,
    /// Number of episodes (TV) or null (movies).
    pub episodes: Option<i32>,
    /// Episode duration in minutes.
    pub duration: Option<i32>,
    /// Release status: `FINISHED`, `RELEASING`, `NOT_YET_RELEASED`, `CANCELLED`.
    pub status: Option<String>,
    /// Media format: `TV`, `TV_SHORT`, `MOVIE`, `OVA`, `ONA`, `SPECIAL`, etc.
    pub format: Option<String>,
    /// ISO 3166-1 alpha-2 country of origin.
    #[serde(rename = "countryOfOrigin")]
    pub country_of_origin: Option<String>,
    #[serde(rename = "isAdult")]
    pub is_adult: Option<bool>,
}

/// Extended AniList media entry — used in detail queries (includes studios, etc.).
#[derive(Debug, Deserialize)]
pub struct AniListMediaDetail {
    pub id: i32,
    pub title: Option<AniListTitle>,
    pub description: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<AniListDate>,
    #[serde(rename = "endDate")]
    pub end_date: Option<AniListDate>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<AniListCoverImage>,
    #[serde(rename = "bannerImage")]
    pub banner_image: Option<String>,
    pub genres: Option<Vec<String>>,
    pub popularity: Option<i32>,
    #[serde(rename = "averageScore")]
    pub average_score: Option<i32>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    pub status: Option<String>,
    pub format: Option<String>,
    #[serde(rename = "countryOfOrigin")]
    pub country_of_origin: Option<String>,
    #[serde(rename = "isAdult")]
    pub is_adult: Option<bool>,
    /// Main production studios.
    pub studios: Option<AniListStudios>,
    /// Broadcast season (WINTER, SPRING, SUMMER, FALL).
    pub season: Option<String>,
    #[serde(rename = "seasonYear")]
    pub season_year: Option<i32>,
    /// Number of episodes in the series (TV-specific).
    pub episodes_count: Option<i32>,
}

// ── Page response wrappers ────────────────────────────────────────────────────

/// `data` object for media list queries.
#[derive(Debug, Deserialize)]
pub(crate) struct MediaPageResponse {
    /// `Page` may be null in AniList error responses (e.g. rate-limited).
    #[serde(rename = "Page")]
    pub page: Option<MediaPageData>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaPageData {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    #[serde(default)]
    pub media: Vec<AniListMedia>,
}

/// `data` object for single-media detail queries.
#[derive(Debug, Deserialize)]
pub(crate) struct MediaDetailResponse {
    #[serde(rename = "Media")]
    pub media: AniListMediaDetail,
}

// ── Staff types ───────────────────────────────────────────────────────────────

/// Name object for an AniList staff member.
#[derive(Debug, Deserialize)]
pub struct AniListStaffName {
    pub full: Option<String>,
    pub native: Option<String>,
    #[serde(default)]
    pub alternative: Vec<String>,
}

/// Image object for a staff member.
#[derive(Debug, Deserialize)]
pub struct AniListStaffImage {
    pub large: Option<String>,
}

/// AniList staff member (real person — voice actor, director, etc.).
#[derive(Debug, Deserialize)]
pub struct AniListStaff {
    pub id: i32,
    pub name: Option<AniListStaffName>,
    pub image: Option<AniListStaffImage>,
    pub description: Option<String>,
    /// Primary occupations (e.g. `["Voice Actor", "Director"]`).
    #[serde(rename = "primaryOccupations")]
    pub primary_occupations: Option<Vec<String>>,
    /// Native language of the staff member (e.g. `"Japanese"`).
    #[serde(rename = "languageV2")]
    pub language: Option<String>,
}

/// Extended staff info returned by detail queries.
#[derive(Debug, Deserialize)]
pub struct AniListStaffDetail {
    pub id: i32,
    pub name: Option<AniListStaffName>,
    pub image: Option<AniListStaffImage>,
    pub description: Option<String>,
    #[serde(rename = "primaryOccupations")]
    pub primary_occupations: Option<Vec<String>>,
    pub gender: Option<String>,
    #[serde(rename = "dateOfBirth")]
    pub date_of_birth: Option<AniListDate>,
    #[serde(rename = "dateOfDeath")]
    pub date_of_death: Option<AniListDate>,
    #[serde(rename = "homeTown")]
    pub home_town: Option<String>,
    #[serde(rename = "siteUrl")]
    pub site_url: Option<String>,
    #[serde(rename = "languageV2")]
    pub language: Option<String>,
}

/// `data` object for staff list queries.
#[derive(Debug, Deserialize)]
pub(crate) struct StaffPageResponse {
    /// `Page` may be null in AniList error responses (e.g. rate-limited).
    #[serde(rename = "Page")]
    pub page: Option<StaffPageData>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StaffPageData {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    #[serde(default)]
    pub staff: Vec<AniListStaff>,
}

/// `data` object for single-staff detail queries.
#[derive(Debug, Deserialize)]
pub(crate) struct StaffDetailResponse {
    #[serde(rename = "Staff")]
    pub staff: AniListStaffDetail,
}
