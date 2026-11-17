//! Stable, hand-owned TMDB result types exposed at the [`TmdbClient`] boundary.
//!
//! These types are owned by cameo (unlike the generated `types::*`), so a spec
//! update or generator bump cannot rename or reshape the public API. They cover
//! the TMDB-specific endpoints that have no cross-provider unified equivalent
//! (credits, images and certifications); everything else returns a unified
//! model.
//!
//! [`TmdbClient`]: crate::providers::tmdb::TmdbClient

use serde::{Deserialize, Serialize};

use crate::unified::models::Gender;

/// Cast and crew credits for a movie or TV show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmdbCredits {
    /// Billed cast, in billing order.
    pub cast: Vec<CastMember>,
    /// Crew members.
    pub crew: Vec<CrewMember>,
}

/// A cast member (an actor playing a character).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastMember {
    /// TMDB person id.
    pub id: u64,
    /// Person's name.
    pub name: String,
    /// Character played, if credited.
    pub character: Option<String>,
    /// Billing order (0 = top-billed), or `None` when TMDB reported none.
    pub order: Option<u32>,
    /// Department the person is best known for.
    pub known_for_department: Option<String>,
    /// Gender, if reported.
    pub gender: Option<Gender>,
    /// Full profile image URL.
    pub profile_url: Option<String>,
}

/// A crew member (a person in a production role).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrewMember {
    /// TMDB person id.
    pub id: u64,
    /// Person's name.
    pub name: String,
    /// Specific job (e.g. "Director").
    pub job: Option<String>,
    /// Department (e.g. "Directing").
    pub department: Option<String>,
    /// Gender, if reported.
    pub gender: Option<Gender>,
    /// Full profile image URL.
    pub profile_url: Option<String>,
}

/// Image assets (backdrops, posters, logos) for a movie or TV show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmdbImages {
    /// Backdrop images.
    pub backdrops: Vec<TmdbImage>,
    /// Poster images.
    pub posters: Vec<TmdbImage>,
    /// Logo images.
    pub logos: Vec<TmdbImage>,
}

/// A title's certification (content rating) in one country, such as `PG-13`
/// in the US or `TV-MA` for a US TV series.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TmdbCertification {
    /// ISO 3166-1 alpha-2 code of the country the rating applies in (TMDB's
    /// `iso_3166_1`), e.g. `"US"`.
    pub country: String,
    /// The rating that country's rating board assigned, e.g. `"R"`.
    pub certification: String,
}

/// A single image asset.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmdbImage {
    /// Full image URL (resolved to the CDN base at a sensible size).
    pub url: Option<String>,
    /// Native width in pixels, or `None` when TMDB reported none.
    pub width: Option<u32>,
    /// Native height in pixels, or `None` when TMDB reported none.
    pub height: Option<u32>,
    /// Aspect ratio (width / height), or `None` when TMDB reported none.
    pub aspect_ratio: Option<f64>,
    /// Average vote score, or `None` when TMDB reported none.
    pub vote_average: Option<f64>,
    /// Total vote count, or `None` when TMDB reported none.
    pub vote_count: Option<u64>,
    /// ISO 639-1 language code of any embedded text, if applicable.
    pub language: Option<String>,
}
