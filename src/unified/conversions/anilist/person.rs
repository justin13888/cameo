//! AniList staff → unified person conversions.

use super::staff_id;
use crate::{
    providers::anilist::response::{AniListStaff, AniListStaffDetail},
    unified::models::{Gender, UnifiedPerson, UnifiedPersonDetails},
};

// ── AniListStaff → UnifiedPerson ──────────────────────────────────────────────

/// Convert an AniList staff member to a [`UnifiedPerson`], or `None` (logged at
/// `warn`) when its id is not a valid AniList staff id (zero or negative).
pub fn staff_to_person(s: AniListStaff) -> Option<UnifiedPerson> {
    let provider_id = staff_id(s.id)?;
    let department = s
        .primary_occupations
        .as_deref()
        .and_then(|occ| occ.first())
        .map(|occ| normalize_occupation(occ));

    Some(UnifiedPerson {
        provider_id,
        name: s
            .name
            .as_ref()
            .and_then(|n| n.full.clone())
            .unwrap_or_default(),
        known_for_department: department,
        profile_url: s.image.and_then(|i| i.large),
        popularity: None,
        gender: None,
        adult: None,
    })
}

/// Convert an AniList staff detail to [`UnifiedPersonDetails`], or `None`
/// (logged at `warn`) when its id is not a valid AniList staff id.
pub fn staff_detail_to_person_details(s: AniListStaffDetail) -> Option<UnifiedPersonDetails> {
    let provider_id = staff_id(s.id)?;
    let department = s
        .primary_occupations
        .as_deref()
        .and_then(|occ| occ.first())
        .map(|occ| normalize_occupation(occ));

    let also_known_as = s
        .name
        .as_ref()
        .map(|n| {
            let mut names: Vec<String> = n.alternative.clone();
            if let Some(native) = &n.native {
                names.push(native.clone());
            }
            names
        })
        .unwrap_or_default();

    Some(UnifiedPersonDetails {
        person: UnifiedPerson {
            provider_id,
            name: s
                .name
                .as_ref()
                .and_then(|n| n.full.clone())
                .unwrap_or_default(),
            known_for_department: department,
            profile_url: s.image.and_then(|i| i.large),
            popularity: None,
            gender: s.gender.as_deref().map(Gender::from_anilist),
            adult: None,
        },
        biography: s.description,
        birthday: s.date_of_birth.as_ref().and_then(super::resolve_date),
        deathday: s.date_of_death.as_ref().and_then(super::resolve_date),
        place_of_birth: s.home_town,
        imdb_id: None,
        homepage: s.site_url,
        also_known_as,
    })
}

/// Normalize an AniList primary occupation to a consistent department string.
pub(crate) fn normalize_occupation(occupation: &str) -> String {
    match occupation.to_lowercase().as_str() {
        "voice actor" | "voice actress" => "Voice Acting".to_string(),
        "director" => "Directing".to_string(),
        "music" | "composer" => "Music".to_string(),
        "animation director" | "animator" => "Animation".to_string(),
        "character design" => "Art".to_string(),
        _ => occupation.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_occupation;

    #[test]
    fn gender_mapping() {
        use crate::unified::models::Gender;
        assert_eq!(Gender::from_anilist("Female"), Gender::Female);
        assert_eq!(Gender::from_anilist("Male"), Gender::Male);
        assert_eq!(Gender::from_anilist("Non-binary"), Gender::NonBinary);
        assert_eq!(Gender::from_anilist("Unknown"), Gender::NotSpecified);
    }

    #[test]
    fn normalize_occupation_known() {
        assert_eq!(normalize_occupation("Voice Actor"), "Voice Acting");
        assert_eq!(normalize_occupation("voice actress"), "Voice Acting");
        assert_eq!(normalize_occupation("Director"), "Directing");
        assert_eq!(normalize_occupation("music"), "Music");
        assert_eq!(normalize_occupation("Composer"), "Music");
    }

    #[test]
    fn normalize_occupation_unknown_passthrough() {
        assert_eq!(normalize_occupation("Storyboard"), "Storyboard");
        assert_eq!(normalize_occupation("Producer"), "Producer");
    }
}
