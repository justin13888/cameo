//! TMDB credits responses → [`TmdbCredits`], the TMDB-native credits model
//! (the unified credits conversions are in [`super::credits`]).

use super::resolve_profile;
use crate::{
    generated::tmdb::types,
    providers::tmdb::models::{CastMember, CrewMember, TmdbCredits},
    unified::{conversions::native_id, models::Gender},
};

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
        id: credit_person_id(
            c.id.into_option(),
            c.name.as_option().map(String::as_str),
            "cast",
        )?,
        name: c.name.into_option().unwrap_or_default(),
        character: c.character.into_option(),
        order: billing_order(c.order.into_option()),
        known_for_department: c.known_for_department.into_option(),
        gender: c.gender.into_option().and_then(Gender::from_tmdb),
        profile_url: resolve_profile(&c.profile_path),
    })
}

/// A movie crew credit, or `None` when it names no valid person id.
fn movie_crew_member(c: types::MovieCreditsResponseCrewItem) -> Option<CrewMember> {
    Some(CrewMember {
        id: credit_person_id(
            c.id.into_option(),
            c.name.as_option().map(String::as_str),
            "crew",
        )?,
        name: c.name.into_option().unwrap_or_default(),
        job: c.job.into_option(),
        department: c.department.into_option(),
        gender: c.gender.into_option().and_then(Gender::from_tmdb),
        profile_url: resolve_profile(&c.profile_path),
    })
}

/// Credits whose person id is missing, zero, or negative are skipped (and
/// logged at `warn`): they name no person TMDB can look up.
impl From<types::MovieCreditsResponse> for TmdbCredits {
    fn from(c: types::MovieCreditsResponse) -> Self {
        TmdbCredits {
            cast: c
                .cast
                .into_option()
                .into_iter()
                .flatten()
                .filter_map(movie_cast_member)
                .collect(),
            crew: c
                .crew
                .into_option()
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
            .into_option()
            .into_iter()
            .flatten()
            .filter_map(|c| {
                Some(CastMember {
                    id: credit_person_id(
                        c.id.into_option(),
                        c.name.as_option().map(String::as_str),
                        "cast",
                    )?,
                    name: c.name.into_option().unwrap_or_default(),
                    character: c
                        .roles
                        .as_option()
                        .into_iter()
                        .flatten()
                        .next()
                        .and_then(|r| r.character.clone().into_option()),
                    order: billing_order(c.order.into_option()),
                    known_for_department: c.known_for_department.into_option(),
                    gender: c.gender.into_option().and_then(Gender::from_tmdb),
                    profile_url: resolve_profile(&c.profile_path),
                })
            })
            .collect();
        let crew = c
            .crew
            .into_option()
            .into_iter()
            .flatten()
            .filter_map(|c| {
                Some(CrewMember {
                    id: credit_person_id(
                        c.id.into_option(),
                        c.name.as_option().map(String::as_str),
                        "crew",
                    )?,
                    name: c.name.into_option().unwrap_or_default(),
                    job: c
                        .jobs
                        .as_option()
                        .into_iter()
                        .flatten()
                        .next()
                        .and_then(|j| j.job.clone().into_option()),
                    department: c.department.into_option(),
                    gender: c.gender.into_option().and_then(Gender::from_tmdb),
                    profile_url: resolve_profile(&c.profile_path),
                })
            })
            .collect();
        TmdbCredits { cast, crew }
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

    /// A credit's profile path becomes a full H632 profile URL.
    #[test]
    fn credit_profile_paths_resolve_to_h632_urls() {
        let raw: types::MovieCreditsResponse = serde_json::from_value(json!({
            "cast": [{ "id": 819, "name": "Edward Norton", "profile_path": "/en.jpg" }],
            "crew": [{ "id": 7467, "name": "David Fincher" }]
        }))
        .expect("credits deserialize");

        let credits = TmdbCredits::from(raw);

        assert_eq!(
            credits.cast[0].profile_url.as_deref(),
            Some("https://image.tmdb.org/t/p/h632/en.jpg")
        );
        assert_eq!(credits.crew[0].profile_url, None);
    }
}
