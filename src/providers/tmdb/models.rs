//! Stable, hand-owned TMDB result types exposed at the [`TmdbClient`] boundary.
//!
//! These types are owned by cameo (unlike the generated `types::*`), so a spec
//! update or generator bump cannot rename or reshape the public API. They cover
//! the TMDB-specific endpoints that have no cross-provider unified equivalent
//! (credits and images); everything else returns a unified model.
//!
//! [`TmdbClient`]: crate::providers::tmdb::TmdbClient

use serde::{Deserialize, Serialize};

use super::image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize};
use crate::{
    generated::tmdb::types,
    unified::{conversions::native_id, models::Gender},
};

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

/// The person id of a TMDB credit, or `None` (logged at `warn`) when TMDB sent
/// no valid one: missing, zero, or negative. Such a credit is skipped.
fn credit_person_id(id: Option<i64>, name: Option<&str>, kind: &str) -> Option<u64> {
    let valid = id.and_then(native_id);
    if valid.is_none() {
        tracing::warn!(
            person_id = id,
            name,
            kind,
            "tmdb: skipping credit without a valid person id"
        );
    }
    valid
}

/// A billing order TMDB reported, or `None` when it sent none or a negative one.
fn billing_order(order: Option<i64>) -> Option<u32> {
    order.and_then(|o| u32::try_from(o).ok())
}

/// A movie cast credit, or `None` when it names no valid person id.
fn movie_cast_member(c: types::MovieCreditsResponseCastItem) -> Option<CastMember> {
    Some(CastMember {
        id: credit_person_id(c.id, c.name.as_deref(), "cast")?,
        name: c.name.unwrap_or_default(),
        character: c.character,
        order: billing_order(c.order),
        known_for_department: c.known_for_department,
        gender: c.gender.and_then(Gender::from_tmdb),
        profile_url: c
            .profile_path
            .as_deref()
            .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
    })
}

/// A movie crew credit, or `None` when it names no valid person id.
fn movie_crew_member(c: types::MovieCreditsResponseCrewItem) -> Option<CrewMember> {
    Some(CrewMember {
        id: credit_person_id(c.id, c.name.as_deref(), "crew")?,
        name: c.name.unwrap_or_default(),
        job: c.job,
        department: c.department,
        gender: c.gender.and_then(Gender::from_tmdb),
        profile_url: c
            .profile_path
            .as_deref()
            .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
    })
}

/// Credits whose person id is missing, zero, or negative are skipped (and
/// logged at `warn`): they name no person TMDB can look up.
impl From<types::MovieCreditsResponse> for TmdbCredits {
    fn from(c: types::MovieCreditsResponse) -> Self {
        TmdbCredits {
            cast: c
                .cast
                .into_iter()
                .flatten()
                .filter_map(movie_cast_member)
                .collect(),
            crew: c
                .crew
                .into_iter()
                .flatten()
                .filter_map(movie_crew_member)
                .collect(),
        }
    }
}

/// Aggregate TV credits collapse a person's multiple episode roles into a
/// single billed entry; take the highest-billed role for the character/job.
/// Credits without a valid person id are skipped, as for movie credits.
impl From<types::TvSeriesAggregateCreditsResponse> for TmdbCredits {
    fn from(c: types::TvSeriesAggregateCreditsResponse) -> Self {
        let cast = c
            .cast
            .into_iter()
            .flatten()
            .filter_map(|c| {
                Some(CastMember {
                    id: credit_person_id(c.id, c.name.as_deref(), "cast")?,
                    name: c.name.unwrap_or_default(),
                    character: c
                        .roles
                        .iter()
                        .flatten()
                        .next()
                        .and_then(|r| r.character.clone()),
                    order: billing_order(c.order),
                    known_for_department: c.known_for_department,
                    gender: c.gender.and_then(Gender::from_tmdb),
                    profile_url: c
                        .profile_path
                        .as_deref()
                        .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
                })
            })
            .collect();
        let crew = c
            .crew
            .into_iter()
            .flatten()
            .filter_map(|c| {
                Some(CrewMember {
                    id: credit_person_id(c.id, c.name.as_deref(), "crew")?,
                    name: c.name.unwrap_or_default(),
                    job: c.jobs.iter().flatten().next().and_then(|j| j.job.clone()),
                    department: c.department,
                    gender: c.gender.and_then(Gender::from_tmdb),
                    profile_url: c
                        .profile_path
                        .as_deref()
                        .map(|p| ImageUrl::profile(p, ProfileSize::H632)),
                })
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
                .flatten()
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
                .flatten()
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
                .flatten()
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

/// Build a [`TmdbImage`]; an absent/`null` numeric field (or a negative size
/// or count) is `None`, never a fabricated `0`.
#[allow(clippy::too_many_arguments)]
fn image_from(
    file_path: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    aspect_ratio: Option<f64>,
    vote_average: Option<f64>,
    vote_count: Option<i64>,
    kind: ImageKind,
) -> TmdbImage {
    let url = file_path.as_deref().map(|p| match kind {
        ImageKind::Backdrop => ImageUrl::backdrop(p, BackdropSize::W780),
        ImageKind::Poster => ImageUrl::poster(p, PosterSize::W500),
        ImageKind::Logo => ImageUrl::logo(p, LogoSize::W92),
    });
    TmdbImage {
        url,
        width: width.and_then(|w| u32::try_from(w).ok()),
        height: height.and_then(|h| u32::try_from(h).ok()),
        aspect_ratio,
        vote_average,
        vote_count: vote_count.and_then(|c| u64::try_from(c).ok()),
        language: None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Movie credits drop people without a valid id (missing, zero, or
    /// negative) instead of reporting them as person `0`, and keep an
    /// unreported billing order as `None`.
    #[test]
    fn movie_credits_skip_invalid_person_ids() {
        let raw: types::MovieCreditsResponse = serde_json::from_value(json!({
            "cast": [
                { "id": 819, "name": "Edward Norton", "order": 0 },
                { "id": 287, "name": "Brad Pitt" },
                { "name": "No id", "order": 2 },
                { "id": 0, "name": "Zero id", "order": 3 },
                { "id": -4, "name": "Negative id", "order": 4 }
            ],
            "crew": [
                { "id": 7467, "name": "David Fincher", "job": "Director" },
                { "id": -1, "name": "Negative id" },
                { "name": "No id" }
            ]
        }))
        .expect("credits deserialize");

        let credits = TmdbCredits::from(raw);

        let cast: Vec<_> = credits
            .cast
            .iter()
            .map(|c| (c.id, c.name.as_str(), c.order))
            .collect();
        assert_eq!(
            cast,
            [(819, "Edward Norton", Some(0)), (287, "Brad Pitt", None)]
        );
        let crew: Vec<_> = credits
            .crew
            .iter()
            .map(|c| (c.id, c.name.as_str()))
            .collect();
        assert_eq!(crew, [(7467, "David Fincher")]);
    }

    /// Aggregate TV credits apply the same id and billing-order rules.
    #[test]
    fn aggregate_credits_skip_invalid_person_ids() {
        let raw: types::TvSeriesAggregateCreditsResponse = serde_json::from_value(json!({
            "cast": [
                { "id": 17419, "name": "Bryan Cranston", "order": 0,
                  "roles": [{ "character": "Walter White" }] },
                { "id": 84497, "name": "Aaron Paul", "order": -1 },
                { "id": 0, "name": "Zero id", "order": 2 }
            ],
            "crew": [
                { "id": 66633, "name": "Vince Gilligan", "jobs": [{ "job": "Creator" }] },
                { "id": -9, "name": "Negative id" }
            ]
        }))
        .expect("aggregate credits deserialize");

        let credits = TmdbCredits::from(raw);

        let cast: Vec<_> = credits
            .cast
            .iter()
            .map(|c| (c.id, c.character.as_deref(), c.order))
            .collect();
        assert_eq!(
            cast,
            [(17419, Some("Walter White"), Some(0)), (84497, None, None)]
        );
        let crew: Vec<_> = credits
            .crew
            .iter()
            .map(|c| (c.id, c.job.as_deref()))
            .collect();
        assert_eq!(crew, [(66633, Some("Creator"))]);
    }

    /// Image fields TMDB omits (or reports negative) are `None`, not `0`;
    /// reported values pass through.
    #[test]
    fn image_numeric_fields_are_honestly_optional() {
        let raw: types::MovieImagesResponse = serde_json::from_value(json!({
            "backdrops": [
                { "file_path": "/b.jpg", "width": 1920, "height": 1080,
                  "aspect_ratio": 1.778, "vote_average": 5.3, "vote_count": 12 }
            ],
            "posters": [{ "file_path": "/p.jpg" }],
            "logos": [{ "file_path": "/l.png", "width": -1, "height": -1, "vote_count": -1 }]
        }))
        .expect("images deserialize");

        let images = TmdbImages::from(raw);

        let backdrop = &images.backdrops[0];
        assert_eq!(backdrop.width, Some(1920));
        assert_eq!(backdrop.height, Some(1080));
        assert_eq!(backdrop.aspect_ratio, Some(1.778));
        assert_eq!(backdrop.vote_average, Some(5.3));
        assert_eq!(backdrop.vote_count, Some(12));
        for image in [&images.posters[0], &images.logos[0]] {
            assert_eq!(image.width, None);
            assert_eq!(image.height, None);
            assert_eq!(image.vote_count, None);
        }
        assert_eq!(images.posters[0].aspect_ratio, None);
        assert_eq!(images.posters[0].vote_average, None);
    }
}
