//! Builders for models shared across media kinds: [`AlternativeTitle`],
//! [`Certification`], [`Company`] and [`UnifiedSearchResult`].

use crate::unified::models::*;

impl AlternativeTitle {
    /// Create an alternative title of the given kind with no language.
    pub fn new(title: impl Into<String>, kind: TitleKind) -> Self {
        Self {
            title: title.into(),
            kind,
            language: None,
        }
    }

    /// Set the language (ISO 639-1).
    pub fn with_language(mut self, value: impl Into<String>) -> Self {
        self.language = Some(value.into());
        self
    }
}

impl Certification {
    /// Create a certification: `rating` as assigned in `country` (ISO 3166-1
    /// alpha-2).
    pub fn new(country: impl Into<String>, rating: impl Into<String>) -> Self {
        Self {
            country: country.into(),
            rating: rating.into(),
        }
    }
}

impl Company {
    /// Create a company with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            logo_url: None,
            origin_country: None,
        }
    }

    /// Set the company id.
    pub fn with_id(mut self, value: u64) -> Self {
        self.id = Some(value);
        self
    }
    /// Set the logo image URL.
    pub fn with_logo_url(mut self, value: impl Into<String>) -> Self {
        self.logo_url = Some(value.into());
        self
    }
    /// Set the origin country.
    pub fn with_origin_country(mut self, value: impl Into<String>) -> Self {
        self.origin_country = Some(value.into());
        self
    }
}

impl UnifiedSearchResult {
    /// Wrap a movie as a search result.
    pub fn movie(movie: UnifiedMovie) -> Self {
        UnifiedSearchResult::Movie(movie)
    }
    /// Wrap a TV show as a search result.
    pub fn tv_show(show: UnifiedTvShow) -> Self {
        UnifiedSearchResult::TvShow(show)
    }
    /// Wrap a person as a search result.
    pub fn person(person: UnifiedPerson) -> Self {
        UnifiedSearchResult::Person(person)
    }
}
