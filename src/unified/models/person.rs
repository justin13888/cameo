use serde::{Deserialize, Serialize};

use crate::unified::{date::PartialDate, media_id::MediaId};

/// A person's gender.
///
/// Replaces the provider-specific magic numbers (TMDB's `0/1/2/3`) with a
/// typed, `#[non_exhaustive]` enum. An absent or unspecified gender is
/// represented by `Option::None` at the use site rather than a fabricated
/// value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    /// The provider explicitly reports the gender as unspecified.
    NotSpecified,
    /// Female.
    Female,
    /// Male.
    Male,
    /// Non-binary.
    NonBinary,
}

impl Gender {
    /// Map a TMDB numeric gender code (`1` female, `2` male, `3` non-binary).
    ///
    /// Returns `None` for `0` (TMDB's "not set") or any unknown code, so an
    /// unknown or unset gender is absent rather than fabricated.
    #[cfg(feature = "tmdb")]
    pub(crate) fn from_tmdb(code: i64) -> Option<Self> {
        match code {
            1 => Some(Gender::Female),
            2 => Some(Gender::Male),
            3 => Some(Gender::NonBinary),
            _ => None,
        }
    }

    /// Map an AniList gender string (`"Female"`, `"Male"`, `"Non-binary"`).
    ///
    /// Returns `None` for any other string, so an unknown gender is absent
    /// rather than fabricated, matching `Gender::from_tmdb`. An absent value
    /// is handled by the caller and also yields `None`.
    #[cfg(feature = "anilist")]
    pub(crate) fn from_anilist(value: &str) -> Option<Self> {
        match value {
            "Female" => Some(Gender::Female),
            "Male" => Some(Gender::Male),
            "Non-binary" => Some(Gender::NonBinary),
            _ => None,
        }
    }
}

#[cfg(all(test, any(feature = "tmdb", feature = "anilist")))]
mod gender_tests {
    use super::Gender;

    #[test]
    #[cfg(feature = "tmdb")]
    fn from_tmdb_maps_known_codes() {
        assert_eq!(Gender::from_tmdb(1), Some(Gender::Female));
        assert_eq!(Gender::from_tmdb(2), Some(Gender::Male));
        assert_eq!(Gender::from_tmdb(3), Some(Gender::NonBinary));
    }

    #[test]
    #[cfg(feature = "tmdb")]
    fn from_tmdb_unset_or_unknown_is_none() {
        assert_eq!(Gender::from_tmdb(0), None);
        assert_eq!(Gender::from_tmdb(4), None);
        assert_eq!(Gender::from_tmdb(-1), None);
    }

    #[test]
    #[cfg(feature = "anilist")]
    fn from_anilist_maps_known_strings() {
        assert_eq!(Gender::from_anilist("Female"), Some(Gender::Female));
        assert_eq!(Gender::from_anilist("Male"), Some(Gender::Male));
        assert_eq!(Gender::from_anilist("Non-binary"), Some(Gender::NonBinary));
    }

    #[test]
    #[cfg(feature = "anilist")]
    fn from_anilist_unknown_is_none() {
        assert_eq!(Gender::from_anilist("Unknown"), None);
        assert_eq!(Gender::from_anilist(""), None);
        assert_eq!(Gender::from_anilist("female"), None);
    }
}

/// A person in the unified model.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedPerson {
    /// Provider-qualified person id (e.g. `tmdb:287`).
    pub provider_id: MediaId,
    /// Person's name.
    pub name: String,
    /// Known-for department (e.g. "Acting", "Directing").
    pub known_for_department: Option<String>,
    /// Full profile image URL.
    pub profile_url: Option<String>,
    /// Popularity score.
    pub popularity: Option<f64>,
    /// Gender, if the provider reports one.
    pub gender: Option<Gender>,
    /// Whether this person is associated with adult content, if reported.
    pub adult: Option<bool>,
}

/// Detailed person information in the unified model.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedPersonDetails {
    /// Base person info.
    pub person: UnifiedPerson,
    /// Biography.
    pub biography: Option<String>,
    /// Birthday (may be partial, e.g. year-only from some providers).
    pub birthday: Option<PartialDate>,
    /// Death day (may be partial), if applicable.
    pub deathday: Option<PartialDate>,
    /// Place of birth.
    pub place_of_birth: Option<String>,
    /// IMDB ID.
    pub imdb_id: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Other names the person is known by.
    pub also_known_as: Vec<String>,
}

/// The cast and crew credited on a movie or TV show.
///
/// Returned by [`CameoClient::movie_credits`](crate::unified::CameoClient::movie_credits)
/// and [`CameoClient::tv_show_credits`](crate::unified::CameoClient::tv_show_credits).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedCredits {
    /// Provider-qualified id of the title these credits belong to (e.g. `tmdb:550`).
    pub provider_id: MediaId,
    /// Cast credits, in the provider's billing order.
    pub cast: Vec<UnifiedCastCredit>,
    /// Crew credits, in the provider's order.
    pub crew: Vec<UnifiedCrewCredit>,
}

/// One cast credit: a person playing (or voicing) a character.
///
/// A person credited for several characters appears once per character.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedCastCredit {
    /// Provider-qualified person id (e.g. `tmdb:287`, `anilist:staff:95061`),
    /// usable with [`person_details`](crate::unified::CameoClient::person_details).
    pub person_id: MediaId,
    /// The person's name.
    pub name: String,
    /// Full profile image URL of the person.
    pub profile_url: Option<String>,
    /// The character played or voiced, if credited.
    pub character: Option<String>,
    /// Billing order (`0` = top-billed), if the provider reports one.
    pub order: Option<u32>,
    /// The person's gender, if the provider reports it.
    pub gender: Option<Gender>,
}

/// One crew credit: a person in a production role.
///
/// A person credited for several jobs appears once per job.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedCrewCredit {
    /// Provider-qualified person id (e.g. `tmdb:7467`, `anilist:staff:95061`),
    /// usable with [`person_details`](crate::unified::CameoClient::person_details).
    pub person_id: MediaId,
    /// The person's name.
    pub name: String,
    /// Full profile image URL of the person.
    pub profile_url: Option<String>,
    /// The specific job (e.g. TMDB `"Director"`, AniList `"Key Animation (ep 5)"`).
    pub job: Option<String>,
    /// The department the job belongs to (e.g. `"Directing"`), if the
    /// provider reports one. AniList reports none.
    pub department: Option<String>,
    /// The person's gender, if the provider reports it.
    pub gender: Option<Gender>,
}
