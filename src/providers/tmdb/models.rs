//! Stable, hand-owned TMDB result types exposed at the [`TmdbClient`] boundary.
//!
//! These types are owned by cameo (unlike the generated `types::*`), so a spec
//! update or generator bump cannot rename or reshape the public API. They cover
//! the TMDB-specific endpoints that have no cross-provider unified equivalent
//! (credits and images); everything else returns a unified model.
//!
//! [`TmdbClient`]: super::TmdbClient

use serde::{Deserialize, Serialize};

use super::image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize};
use crate::{generated::tmdb::types, unified::models::Gender};

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
    /// Billing order (0 = top-billed).
    pub order: u32,
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

/// A single image asset.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TmdbImage {
    /// Full image URL (resolved to the CDN base at a sensible size).
    pub url: Option<String>,
    /// Native width in pixels.
    pub width: u32,
    /// Native height in pixels.
    pub height: u32,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f64,
    /// Average vote score.
    pub vote_average: f64,
    /// Total vote count.
    pub vote_count: u64,
    /// ISO 639-1 language code of any embedded text, if applicable.
    pub language: Option<String>,
}

impl From<types::MovieCreditsResponseCastItem> for CastMember {
    fn from(c: types::MovieCreditsResponseCastItem) -> Self {
        CastMember {
            id: u64::try_from(c.id).unwrap_or(0),
            name: c.name.unwrap_or_default(),
            character: c.character,
            order: u32::try_from(c.order).unwrap_or(0),
            known_for_department: c.known_for_department,
            gender: Gender::from_tmdb(c.gender),
            profile_url: c
                .profile_path
                .as_deref()
                .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
        }
    }
}

impl From<types::MovieCreditsResponseCrewItem> for CrewMember {
    fn from(c: types::MovieCreditsResponseCrewItem) -> Self {
        CrewMember {
            id: u64::try_from(c.id).unwrap_or(0),
            name: c.name.unwrap_or_default(),
            job: c.job,
            department: c.department,
            gender: Gender::from_tmdb(c.gender),
            profile_url: c
                .profile_path
                .as_deref()
                .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
        }
    }
}

impl From<types::MovieCreditsResponse> for TmdbCredits {
    fn from(c: types::MovieCreditsResponse) -> Self {
        TmdbCredits {
            cast: c.cast.into_iter().map(CastMember::from).collect(),
            crew: c.crew.into_iter().map(CrewMember::from).collect(),
        }
    }
}

/// Aggregate TV credits collapse a person's multiple episode roles into a
/// single billed entry; take the highest-billed role for the character/job.
impl From<types::TvSeriesAggregateCreditsResponse> for TmdbCredits {
    fn from(c: types::TvSeriesAggregateCreditsResponse) -> Self {
        let cast = c
            .cast
            .into_iter()
            .map(|c| CastMember {
                id: u64::try_from(c.id).unwrap_or(0),
                name: c.name.unwrap_or_default(),
                character: c.roles.first().and_then(|r| r.character.clone()),
                order: u32::try_from(c.order).unwrap_or(0),
                known_for_department: c.known_for_department,
                gender: Gender::from_tmdb(c.gender),
                profile_url: c
                    .profile_path
                    .as_deref()
                    .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
            })
            .collect();
        let crew = c
            .crew
            .into_iter()
            .map(|c| CrewMember {
                id: u64::try_from(c.id).unwrap_or(0),
                name: c.name.unwrap_or_default(),
                job: c.jobs.first().and_then(|j| j.job.clone()),
                department: c.department,
                gender: Gender::from_tmdb(c.gender),
                profile_url: c
                    .profile_path
                    .as_deref()
                    .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
            })
            .collect();
        TmdbCredits { cast, crew }
    }
}

impl From<types::MovieImagesResponse> for TmdbImages {
    fn from(i: types::MovieImagesResponse) -> Self {
        TmdbImages {
            backdrops: i
                .backdrops
                .into_iter()
                .map(|b| {
                    image_from(
                        b.file_path,
                        b.width,
                        b.height,
                        b.aspect_ratio,
                        b.vote_average,
                        b.vote_count,
                        ImageKind::Backdrop,
                    )
                })
                .collect(),
            posters: i
                .posters
                .into_iter()
                .map(|p| {
                    image_from(
                        p.file_path,
                        p.width,
                        p.height,
                        p.aspect_ratio,
                        p.vote_average,
                        p.vote_count,
                        ImageKind::Poster,
                    )
                })
                .collect(),
            logos: i
                .logos
                .into_iter()
                .map(|l| {
                    image_from(
                        l.file_path,
                        l.width,
                        l.height,
                        l.aspect_ratio,
                        l.vote_average,
                        l.vote_count,
                        ImageKind::Logo,
                    )
                })
                .collect(),
        }
    }
}

enum ImageKind {
    Backdrop,
    Poster,
    Logo,
}

#[allow(clippy::too_many_arguments)]
fn image_from(
    file_path: Option<String>,
    width: i64,
    height: i64,
    aspect_ratio: f64,
    vote_average: f64,
    vote_count: i64,
    kind: ImageKind,
) -> TmdbImage {
    let url = file_path.as_deref().map(|p| match kind {
        ImageKind::Backdrop => ImageUrl::backdrop(p, BackdropSize::W780),
        ImageKind::Poster => ImageUrl::poster(p, PosterSize::W500),
        ImageKind::Logo => ImageUrl::logo(p, LogoSize::W92),
    });
    TmdbImage {
        url,
        width: u32::try_from(width).unwrap_or(0),
        height: u32::try_from(height).unwrap_or(0),
        aspect_ratio,
        vote_average,
        vote_count: u64::try_from(vote_count).unwrap_or(0),
        language: None,
    }
}
