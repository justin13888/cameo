//! Builders for the person family: [`UnifiedPerson`], [`UnifiedPersonDetails`]
//! and the credits that link people to titles ([`UnifiedCredits`],
//! [`UnifiedCastCredit`], [`UnifiedCrewCredit`]).

use crate::unified::{date::PartialDate, media_id::MediaId, models::*};

impl UnifiedPerson {
    /// Create a person with the required identity fields.
    pub fn new(provider_id: MediaId, name: impl Into<String>) -> Self {
        Self {
            provider_id,
            name: name.into(),
            known_for_department: None,
            profile_url: None,
            popularity: None,
            gender: None,
            adult: None,
        }
    }

    /// Set the known-for department.
    pub fn with_known_for_department(mut self, value: impl Into<String>) -> Self {
        self.known_for_department = Some(value.into());
        self
    }
    /// Set the profile image URL.
    pub fn with_profile_url(mut self, value: impl Into<String>) -> Self {
        self.profile_url = Some(value.into());
        self
    }
    /// Set the popularity score.
    pub fn with_popularity(mut self, value: f64) -> Self {
        self.popularity = Some(value);
        self
    }
    /// Set the gender.
    pub fn with_gender(mut self, value: Gender) -> Self {
        self.gender = Some(value);
        self
    }
    /// Set whether the person is associated with adult content.
    pub fn with_adult(mut self, value: bool) -> Self {
        self.adult = Some(value);
        self
    }
}

impl UnifiedPersonDetails {
    /// Create person details wrapping a base [`UnifiedPerson`].
    pub fn new(person: UnifiedPerson) -> Self {
        Self {
            person,
            biography: None,
            birthday: None,
            deathday: None,
            place_of_birth: None,
            imdb_id: None,
            homepage: None,
            also_known_as: Vec::new(),
        }
    }

    /// Set the biography.
    pub fn with_biography(mut self, value: impl Into<String>) -> Self {
        self.biography = Some(value.into());
        self
    }
    /// Set the birthday.
    pub fn with_birthday(mut self, value: PartialDate) -> Self {
        self.birthday = Some(value);
        self
    }
    /// Set the death day.
    pub fn with_deathday(mut self, value: PartialDate) -> Self {
        self.deathday = Some(value);
        self
    }
    /// Set the place of birth.
    pub fn with_place_of_birth(mut self, value: impl Into<String>) -> Self {
        self.place_of_birth = Some(value.into());
        self
    }
    /// Set the IMDB id.
    pub fn with_imdb_id(mut self, value: impl Into<String>) -> Self {
        self.imdb_id = Some(value.into());
        self
    }
    /// Set the homepage URL.
    pub fn with_homepage(mut self, value: impl Into<String>) -> Self {
        self.homepage = Some(value.into());
        self
    }
    /// Set the other names the person is known by.
    pub fn with_also_known_as(mut self, value: Vec<String>) -> Self {
        self.also_known_as = value;
        self
    }
}

impl UnifiedCredits {
    /// Create an empty credits list for the title `provider_id`.
    pub fn new(provider_id: MediaId) -> Self {
        Self {
            provider_id,
            cast: Vec::new(),
            crew: Vec::new(),
        }
    }

    /// Set the cast credits.
    pub fn with_cast(mut self, value: Vec<UnifiedCastCredit>) -> Self {
        self.cast = value;
        self
    }
    /// Set the crew credits.
    pub fn with_crew(mut self, value: Vec<UnifiedCrewCredit>) -> Self {
        self.crew = value;
        self
    }
}

impl UnifiedCastCredit {
    /// Create a cast credit for the person `person_id`.
    pub fn new(person_id: MediaId, name: impl Into<String>) -> Self {
        Self {
            person_id,
            name: name.into(),
            profile_url: None,
            character: None,
            order: None,
            gender: None,
        }
    }

    /// Set the profile image URL.
    pub fn with_profile_url(mut self, value: impl Into<String>) -> Self {
        self.profile_url = Some(value.into());
        self
    }
    /// Set the character played or voiced.
    pub fn with_character(mut self, value: impl Into<String>) -> Self {
        self.character = Some(value.into());
        self
    }
    /// Set the billing order (`0` = top-billed).
    pub fn with_order(mut self, value: u32) -> Self {
        self.order = Some(value);
        self
    }
    /// Set the gender.
    pub fn with_gender(mut self, value: Gender) -> Self {
        self.gender = Some(value);
        self
    }
}

impl UnifiedCrewCredit {
    /// Create a crew credit for the person `person_id`.
    pub fn new(person_id: MediaId, name: impl Into<String>) -> Self {
        Self {
            person_id,
            name: name.into(),
            profile_url: None,
            job: None,
            department: None,
            gender: None,
        }
    }

    /// Set the profile image URL.
    pub fn with_profile_url(mut self, value: impl Into<String>) -> Self {
        self.profile_url = Some(value.into());
        self
    }
    /// Set the job.
    pub fn with_job(mut self, value: impl Into<String>) -> Self {
        self.job = Some(value.into());
        self
    }
    /// Set the department.
    pub fn with_department(mut self, value: impl Into<String>) -> Self {
        self.department = Some(value.into());
        self
    }
    /// Set the gender.
    pub fn with_gender(mut self, value: Gender) -> Self {
        self.gender = Some(value);
        self
    }
}
