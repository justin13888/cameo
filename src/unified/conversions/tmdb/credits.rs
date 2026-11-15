//! TMDB credits → unified credits conversions.

use crate::{
    providers::tmdb::models::{CastMember, CrewMember, TmdbCredits},
    unified::{
        media_id::MediaId,
        models::{UnifiedCastCredit, UnifiedCredits, UnifiedCrewCredit},
    },
};

/// Convert TMDB credits for the title `title` into [`UnifiedCredits`].
///
/// A credit whose person id is `0` (TMDB sent no usable id, which
/// [`TmdbCredits`] records as `0`) cannot name a person, so it is skipped and
/// logged at `warn`.
pub(crate) fn tmdb_credits_to_unified(title: MediaId, credits: TmdbCredits) -> UnifiedCredits {
    let cast = credits
        .cast
        .into_iter()
        .filter_map(|c| cast_credit(&title, c))
        .collect();
    let crew = credits
        .crew
        .into_iter()
        .filter_map(|c| crew_credit(&title, c))
        .collect();
    UnifiedCredits {
        provider_id: title,
        cast,
        crew,
    }
}

fn cast_credit(title: &MediaId, c: CastMember) -> Option<UnifiedCastCredit> {
    if c.id == 0 {
        tracing::warn!(%title, name = %c.name, "tmdb: skipping cast credit without a person id");
        return None;
    }
    Some(UnifiedCastCredit {
        person_id: MediaId::tmdb(c.id),
        name: c.name,
        profile_url: c.profile_url,
        character: c.character,
        order: Some(c.order),
        gender: c.gender,
    })
}

fn crew_credit(title: &MediaId, c: CrewMember) -> Option<UnifiedCrewCredit> {
    if c.id == 0 {
        tracing::warn!(%title, name = %c.name, "tmdb: skipping crew credit without a person id");
        return None;
    }
    Some(UnifiedCrewCredit {
        person_id: MediaId::tmdb(c.id),
        name: c.name,
        profile_url: c.profile_url,
        job: c.job,
        department: c.department,
        gender: c.gender,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::models::Gender;

    fn cast(id: u64, name: &str, order: u32) -> CastMember {
        CastMember {
            id,
            name: name.to_string(),
            character: Some("The Narrator".to_string()),
            order,
            known_for_department: Some("Acting".to_string()),
            gender: Some(Gender::Male),
            profile_url: Some("https://image.tmdb.org/t/p/h632/a.jpg".to_string()),
        }
    }

    fn crew(id: u64, name: &str) -> CrewMember {
        CrewMember {
            id,
            name: name.to_string(),
            job: Some("Director".to_string()),
            department: Some("Directing".to_string()),
            gender: None,
            profile_url: None,
        }
    }

    #[test]
    fn maps_cast_and_crew_fields() {
        let credits = TmdbCredits {
            cast: vec![cast(819, "Edward Norton", 0)],
            crew: vec![crew(7467, "David Fincher")],
        };

        let unified = tmdb_credits_to_unified(MediaId::tmdb(550), credits);

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
    fn skips_credits_without_a_person_id() {
        let credits = TmdbCredits {
            cast: vec![cast(0, "Nobody", 0), cast(819, "Edward Norton", 1)],
            crew: vec![crew(0, "Nobody"), crew(7467, "David Fincher")],
        };

        let unified = tmdb_credits_to_unified(MediaId::tmdb(550), credits);

        assert_eq!(unified.cast.len(), 1);
        assert_eq!(unified.cast[0].person_id, MediaId::tmdb(819));
        assert_eq!(unified.cast[0].order, Some(1));
        assert_eq!(unified.crew.len(), 1);
        assert_eq!(unified.crew[0].person_id, MediaId::tmdb(7467));
    }
}
