//! TMDB credits → unified credits conversions.
//!
//! These read TMDB's responses directly rather than [`TmdbCredits`], so that
//! a missing billing order stays `None` and every character and job a person
//! has on a TV series is kept.
//!
//! [`TmdbCredits`]: crate::providers::tmdb::models::TmdbCredits

use super::resolve_profile;
use crate::{
    generated::tmdb::types,
    unified::{
        media_id::MediaId,
        models::{Gender, UnifiedCastCredit, UnifiedCredits, UnifiedCrewCredit},
    },
};

/// The person fields every TMDB credit carries.
struct Person {
    id: Option<i64>,
    name: Option<String>,
    profile_path: Option<String>,
    gender: Option<i64>,
}

/// A person resolved to a usable unified id.
#[derive(Clone)]
struct Credited {
    person_id: MediaId,
    name: String,
    profile_url: Option<String>,
    gender: Option<Gender>,
}

impl Person {
    /// Resolve the person, or `None` (logged at `warn`) when TMDB sent no
    /// usable person id: missing, zero, or negative.
    fn resolve(self, title: &MediaId, kind: &str) -> Option<Credited> {
        let name = self.name.unwrap_or_default();
        let Some(id) = self
            .id
            .and_then(|id| u64::try_from(id).ok())
            .filter(|&id| id != 0)
        else {
            tracing::warn!(%title, %name, kind, "tmdb: skipping credit without a person id");
            return None;
        };
        Some(Credited {
            person_id: MediaId::tmdb(id),
            name,
            profile_url: resolve_profile(&self.profile_path),
            gender: self.gender.and_then(Gender::from_tmdb),
        })
    }
}

fn billing_order(order: Option<i64>) -> Option<u32> {
    order.and_then(|o| u32::try_from(o).ok())
}

fn cast_credit(p: Credited, character: Option<String>, order: Option<u32>) -> UnifiedCastCredit {
    UnifiedCastCredit {
        person_id: p.person_id,
        name: p.name,
        profile_url: p.profile_url,
        character,
        order,
        gender: p.gender,
    }
}

fn crew_credit(p: Credited, job: Option<String>, department: Option<String>) -> UnifiedCrewCredit {
    UnifiedCrewCredit {
        person_id: p.person_id,
        name: p.name,
        profile_url: p.profile_url,
        job,
        department,
        gender: p.gender,
    }
}

/// Convert `/movie/{id}/credits` for the title `title` into [`UnifiedCredits`].
///
/// TMDB already lists one entry per character and per job. A credit without
/// a usable person id cannot name a person, so it is skipped and logged.
pub(crate) fn tmdb_movie_credits_to_unified(
    title: MediaId,
    credits: types::MovieCreditsResponse,
) -> UnifiedCredits {
    let cast = credits
        .cast
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let person = Person {
                id: c.id,
                name: c.name,
                profile_path: c.profile_path,
                gender: c.gender,
            };
            let p = person.resolve(&title, "cast")?;
            Some(cast_credit(p, c.character, billing_order(c.order)))
        })
        .collect();
    let crew = credits
        .crew
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let person = Person {
                id: c.id,
                name: c.name,
                profile_path: c.profile_path,
                gender: c.gender,
            };
            let p = person.resolve(&title, "crew")?;
            Some(crew_credit(p, c.job, c.department))
        })
        .collect();
    UnifiedCredits {
        provider_id: title,
        cast,
        crew,
    }
}

/// Convert `/tv/{id}/aggregate_credits` for the show `title` into
/// [`UnifiedCredits`].
///
/// TMDB lists each person once with all their roles and jobs; this emits one
/// cast credit per role and one crew credit per job, all sharing the person's
/// billing order and department. A person with no roles (or no jobs) still
/// yields one credit, with `character` (or `job`) `None`. A credit without a
/// usable person id is skipped and logged.
pub(crate) fn tmdb_tv_credits_to_unified(
    title: MediaId,
    credits: types::TvSeriesAggregateCreditsResponse,
) -> UnifiedCredits {
    let mut cast = Vec::new();
    for c in credits.cast.into_iter().flatten() {
        let person = Person {
            id: c.id,
            name: c.name,
            profile_path: c.profile_path,
            gender: c.gender,
        };
        let Some(p) = person.resolve(&title, "cast") else {
            continue;
        };
        let order = billing_order(c.order);
        let mut characters: Vec<Option<String>> =
            c.roles.into_iter().flatten().map(|r| r.character).collect();
        if characters.is_empty() {
            characters.push(None);
        }
        for character in characters {
            cast.push(cast_credit(p.clone(), character, order));
        }
    }
    let mut crew = Vec::new();
    for c in credits.crew.into_iter().flatten() {
        let person = Person {
            id: c.id,
            name: c.name,
            profile_path: c.profile_path,
            gender: c.gender,
        };
        let Some(p) = person.resolve(&title, "crew") else {
            continue;
        };
        let mut jobs: Vec<Option<String>> = c.jobs.into_iter().flatten().map(|j| j.job).collect();
        if jobs.is_empty() {
            jobs.push(None);
        }
        for job in jobs {
            crew.push(crew_credit(p.clone(), job, c.department.clone()));
        }
    }
    UnifiedCredits {
        provider_id: title,
        cast,
        crew,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie(json: serde_json::Value) -> types::MovieCreditsResponse {
        serde_json::from_value(json).expect("valid movie credits")
    }

    fn tv(json: serde_json::Value) -> types::TvSeriesAggregateCreditsResponse {
        serde_json::from_value(json).expect("valid aggregate credits")
    }

    #[test]
    fn maps_movie_cast_and_crew_fields() {
        let credits = movie(serde_json::json!({
            "id": 550,
            "cast": [{
                "id": 819, "name": "Edward Norton", "character": "The Narrator",
                "order": 0, "gender": 2, "profile_path": "/a.jpg"
            }],
            "crew": [{
                "id": 7467, "name": "David Fincher", "job": "Director",
                "department": "Directing"
            }]
        }));

        let unified = tmdb_movie_credits_to_unified(MediaId::tmdb(550), credits);

        assert_eq!(unified.provider_id, MediaId::tmdb(550));
        assert_eq!(unified.cast.len(), 1);
        let c = &unified.cast[0];
        assert_eq!(c.person_id, MediaId::tmdb(819));
        assert_eq!(c.name, "Edward Norton");
        assert_eq!(c.character.as_deref(), Some("The Narrator"));
        assert_eq!(c.order, Some(0));
        assert_eq!(c.gender, Some(Gender::Male));
        assert_eq!(
            c.profile_url.as_deref(),
            Some("https://image.tmdb.org/t/p/h632/a.jpg")
        );
        assert_eq!(unified.crew.len(), 1);
        let d = &unified.crew[0];
        assert_eq!(d.person_id, MediaId::tmdb(7467));
        assert_eq!(d.job.as_deref(), Some("Director"));
        assert_eq!(d.department.as_deref(), Some("Directing"));
        assert_eq!(d.gender, None);
        assert_eq!(d.profile_url, None);
    }

    #[test]
    fn missing_order_stays_none() {
        let movie_credits = movie(serde_json::json!({
            "cast": [{ "id": 819, "name": "Edward Norton" }],
            "crew": []
        }));
        let tv_credits = tv(serde_json::json!({
            "cast": [{ "id": 17419, "name": "Bryan Cranston", "roles": [] }],
            "crew": []
        }));

        let m = tmdb_movie_credits_to_unified(MediaId::tmdb(550), movie_credits);
        let t = tmdb_tv_credits_to_unified(MediaId::tmdb(1396), tv_credits);

        assert_eq!(m.cast[0].order, None);
        assert_eq!(t.cast[0].order, None);
    }

    #[test]
    fn skips_credits_without_a_usable_person_id() {
        let credits = movie(serde_json::json!({
            "cast": [
                { "id": 0, "name": "Zero" },
                { "name": "Missing" },
                { "id": -3, "name": "Negative" },
                { "id": 819, "name": "Edward Norton", "order": 1 }
            ],
            "crew": [
                { "id": 0, "name": "Zero" },
                { "id": 7467, "name": "David Fincher" }
            ]
        }));

        let unified = tmdb_movie_credits_to_unified(MediaId::tmdb(550), credits);

        assert_eq!(unified.cast.len(), 1);
        assert_eq!(unified.cast[0].person_id, MediaId::tmdb(819));
        assert_eq!(unified.cast[0].order, Some(1));
        assert_eq!(unified.crew.len(), 1);
        assert_eq!(unified.crew[0].person_id, MediaId::tmdb(7467));
    }

    #[test]
    fn tv_emits_one_credit_per_role_and_per_job() {
        let credits = tv(serde_json::json!({
            "cast": [{
                "id": 1, "name": "Tatiana Maslany", "order": 0, "gender": 1,
                "roles": [
                    { "character": "Sarah Manning", "episode_count": 50 },
                    { "character": "Cosima Niehaus", "episode_count": 45 }
                ]
            }],
            "crew": [{
                "id": 2, "name": "Graeme Manson", "department": "Writing",
                "jobs": [
                    { "job": "Writer", "episode_count": 10 },
                    { "job": "Creator", "episode_count": 50 }
                ]
            }]
        }));

        let unified = tmdb_tv_credits_to_unified(MediaId::tmdb(40711), credits);

        let characters: Vec<_> = unified
            .cast
            .iter()
            .map(|c| c.character.as_deref())
            .collect();
        assert_eq!(characters, [Some("Sarah Manning"), Some("Cosima Niehaus")]);
        assert!(unified.cast.iter().all(|c| c.person_id == MediaId::tmdb(1)
            && c.order == Some(0)
            && c.gender == Some(Gender::Female)));
        let jobs: Vec<_> = unified.crew.iter().map(|c| c.job.as_deref()).collect();
        assert_eq!(jobs, [Some("Writer"), Some("Creator")]);
        assert!(
            unified
                .crew
                .iter()
                .all(|c| c.person_id == MediaId::tmdb(2)
                    && c.department.as_deref() == Some("Writing"))
        );
    }

    #[test]
    fn tv_person_without_roles_or_jobs_is_credited_once() {
        let credits = tv(serde_json::json!({
            "cast": [{ "id": 1, "name": "A", "order": 3, "roles": [] }],
            "crew": [
                { "id": 2, "name": "B", "jobs": [] },
                { "id": 0, "name": "C", "jobs": [] }
            ]
        }));

        let unified = tmdb_tv_credits_to_unified(MediaId::tmdb(1), credits);

        assert_eq!(unified.cast.len(), 1);
        assert_eq!(unified.cast[0].character, None);
        assert_eq!(unified.cast[0].order, Some(3));
        assert_eq!(unified.crew.len(), 1);
        assert_eq!(unified.crew[0].job, None);
    }
}
