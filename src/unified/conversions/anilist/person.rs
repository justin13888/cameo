//! AniList staff → unified person conversions.

use crate::{
    providers::anilist::response::{AniListStaff, AniListStaffDetail},
    unified::models::{UnifiedPerson, UnifiedPersonDetails},
};

// ── AniListStaff → UnifiedPerson ──────────────────────────────────────────────

/// Convert an AniList staff member to a [`UnifiedPerson`].
pub fn staff_to_person(s: AniListStaff) -> UnifiedPerson {
    let department = s
        .primary_occupations
        .as_deref()
        .and_then(|occ| occ.first())
        .map(|occ| normalize_occupation(occ));

    UnifiedPerson {
        provider_id: format!("anilist:staff:{}", s.id),
        name: s
            .name
            .as_ref()
            .and_then(|n| n.full.clone())
            .unwrap_or_default(),
        known_for_department: department,
        profile_url: s.image.and_then(|i| i.large),
        popularity: None,
        gender: None,
        adult: false,
    }
}

/// Convert an AniList staff detail to [`UnifiedPersonDetails`].
pub fn staff_detail_to_person_details(s: AniListStaffDetail) -> UnifiedPersonDetails {
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

    UnifiedPersonDetails {
        person: UnifiedPerson {
            provider_id: format!("anilist:staff:{}", s.id),
            name: s
                .name
                .as_ref()
                .and_then(|n| n.full.clone())
                .unwrap_or_default(),
            known_for_department: department,
            profile_url: s.image.and_then(|i| i.large),
            popularity: None,
            gender: s.gender.as_deref().map(|g| match g {
                "Female" => 1,
                "Male" => 2,
                "Non-binary" => 3,
                _ => 0,
            }),
            adult: false,
        },
        biography: s.description,
        birthday: s.date_of_birth.as_ref().and_then(|d| d.to_date_string()),
        deathday: s.date_of_death.as_ref().and_then(|d| d.to_date_string()),
        place_of_birth: s.home_town,
        imdb_id: None,
        homepage: s.site_url,
        also_known_as,
    }
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

/// Map an AniList gender string to a numeric code.
fn map_gender(g: &str) -> u8 {
    match g {
        "Female" => 1,
        "Male" => 2,
        "Non-binary" => 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{map_gender, normalize_occupation};

    #[test]
    fn gender_mapping() {
        assert_eq!(map_gender("Female"), 1);
        assert_eq!(map_gender("Male"), 2);
        assert_eq!(map_gender("Non-binary"), 3);
        assert_eq!(map_gender("Unknown"), 0);
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
