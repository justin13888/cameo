//! AniList characters and staff → unified credits conversions.

use crate::{
    providers::anilist::response::{AniListCreditStaff, CharacterEdge, StaffEdge},
    unified::{
        media_id::MediaId,
        models::{UnifiedCastCredit, UnifiedCredits, UnifiedCrewCredit},
    },
};

/// Convert an anime's character and staff edges, in AniList's order, into
/// [`UnifiedCredits`] for the title `title`.
///
/// - **Cast:** one credit per voice actor of each character, carrying the
///   character's name. A character's position among the characters is its
///   billing `order`, shared by all of its voice actors. A character with no
///   voice actor names no person and yields no credit.
/// - **Crew:** one credit per staff edge; the edge's `role` (e.g.
///   `"Director"`) is the `job`. AniList reports no department.
///
/// A person whose id is not a valid AniList id (negative) is skipped and
/// logged at `warn`. AniList reports no gender in this selection.
pub(crate) fn anilist_credits_to_unified(
    title: MediaId,
    characters: Vec<CharacterEdge>,
    staff: Vec<StaffEdge>,
) -> UnifiedCredits {
    let mut cast = Vec::new();
    for (order, edge) in characters.into_iter().enumerate() {
        let character = edge.node.and_then(|n| n.name).and_then(|n| n.full);
        let order = u32::try_from(order).ok();
        for actor in edge.voice_actors.into_iter().flatten().flatten() {
            let Some((person_id, name, profile_url)) = person(&title, actor) else {
                continue;
            };
            cast.push(UnifiedCastCredit {
                person_id,
                name,
                profile_url,
                character: character.clone(),
                order,
                gender: None,
            });
        }
    }

    let crew = staff
        .into_iter()
        .filter_map(|edge| {
            let (person_id, name, profile_url) = person(&title, edge.node?)?;
            Some(UnifiedCrewCredit {
                person_id,
                name,
                profile_url,
                job: edge.role,
                department: None,
                gender: None,
            })
        })
        .collect();

    UnifiedCredits {
        provider_id: title,
        cast,
        crew,
    }
}

/// The person id, name, and profile URL of a credited staff member, or `None`
/// (logged) when the id is not a valid AniList id.
fn person(title: &MediaId, s: AniListCreditStaff) -> Option<(MediaId, String, Option<String>)> {
    let Ok(id) = u64::try_from(s.id) else {
        tracing::warn!(%title, staff_id = s.id, "anilist: skipping credit with an invalid staff id");
        return None;
    };
    let name = s.name.and_then(|n| n.full).unwrap_or_default();
    Some((
        MediaId::anilist_staff(id),
        name,
        s.image.and_then(|i| i.large),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn staff(id: i32, name: &str) -> Value {
        json!({ "id": id, "name": { "full": name }, "image": { "large": format!("https://img/{id}.jpg") } })
    }

    fn character(name: Option<&str>, actors: Value) -> CharacterEdge {
        serde_json::from_value(json!({
            "node": { "name": { "full": name } },
            "voiceActors": actors,
        }))
        .expect("character edge")
    }

    fn staff_edge(role: Option<&str>, node: Value) -> StaffEdge {
        serde_json::from_value(json!({ "role": role, "node": node })).expect("staff edge")
    }

    #[test]
    fn one_cast_credit_per_voice_actor_with_the_character_order() {
        let characters = vec![
            character(
                Some("Edward Elric"),
                json!([staff(95061, "Romi Park"), null]),
            ),
            character(Some("Background Soldier"), json!([])),
            character(
                Some("Alphonse Elric"),
                json!([staff(95062, "Rie Kugimiya"), staff(95063, "Second Actor")]),
            ),
        ];

        let credits = anilist_credits_to_unified(MediaId::anilist(5114), characters, Vec::new());

        assert_eq!(credits.provider_id, MediaId::anilist(5114));
        assert_eq!(credits.cast.len(), 3);
        let first = &credits.cast[0];
        assert_eq!(first.person_id, MediaId::anilist_staff(95061));
        assert_eq!(first.name, "Romi Park");
        assert_eq!(first.character.as_deref(), Some("Edward Elric"));
        assert_eq!(first.order, Some(0));
        assert_eq!(first.profile_url.as_deref(), Some("https://img/95061.jpg"));
        assert_eq!(first.gender, None);
        // The voiceless character still takes billing position 1.
        assert_eq!(credits.cast[1].character.as_deref(), Some("Alphonse Elric"));
        assert_eq!(credits.cast[1].order, Some(2));
        assert_eq!(credits.cast[2].order, Some(2));
        assert_eq!(credits.cast[2].person_id, MediaId::anilist_staff(95063));
        assert!(credits.crew.is_empty());
    }

    #[test]
    fn staff_role_is_the_crew_job() {
        let staff_edges = vec![
            staff_edge(Some("Director"), staff(100, "Yasuhiro Irie")),
            staff_edge(None, staff(101, "No Role")),
        ];

        let credits = anilist_credits_to_unified(MediaId::anilist(5114), Vec::new(), staff_edges);

        assert_eq!(credits.crew.len(), 2);
        assert_eq!(credits.crew[0].person_id, MediaId::anilist_staff(100));
        assert_eq!(credits.crew[0].name, "Yasuhiro Irie");
        assert_eq!(credits.crew[0].job.as_deref(), Some("Director"));
        assert_eq!(credits.crew[0].department, None);
        assert_eq!(credits.crew[1].job, None);
    }

    #[test]
    fn skips_null_nodes_and_invalid_ids() {
        let characters = vec![
            character(None, json!([staff(-1, "Bad Id"), staff(7, "Good")])),
            character(Some("No Actors"), Value::Null),
        ];
        let staff_edges = vec![
            staff_edge(Some("Director"), Value::Null),
            staff_edge(Some("Music"), staff(-5, "Bad Id")),
        ];

        let credits = anilist_credits_to_unified(MediaId::anilist(1), characters, staff_edges);

        assert_eq!(credits.cast.len(), 1);
        assert_eq!(credits.cast[0].person_id, MediaId::anilist_staff(7));
        assert_eq!(credits.cast[0].character, None);
        assert!(credits.crew.is_empty());
    }
}
