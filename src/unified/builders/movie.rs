//! Builders for the movie family: [`UnifiedMovie`], [`UnifiedMovieDetails`]
//! and [`Collection`].

use crate::unified::{date::PartialDate, genre::Genre, media_id::MediaId, models::*};

impl UnifiedMovie {
    /// Create a movie with the required identity fields; all other fields
    /// default to empty/absent and can be set with the `with_*` methods.
    pub fn new(provider_id: MediaId, title: impl Into<String>) -> Self {
        Self {
            provider_id,
            title: title.into(),
            original_title: None,
            alternative_titles: Vec::new(),
            overview: None,
            release_date: None,
            poster_url: None,
            backdrop_url: None,
            genres: Vec::new(),
            popularity: None,
            vote_average: None,
            vote_count: None,
            original_language: None,
            adult: None,
            runtime: None,
            status: None,
        }
    }

    /// Set the original-language title.
    pub fn with_original_title(mut self, value: impl Into<String>) -> Self {
        self.original_title = Some(value.into());
        self
    }
    /// Set the alternative title variants.
    pub fn with_alternative_titles(mut self, value: Vec<AlternativeTitle>) -> Self {
        self.alternative_titles = value;
        self
    }
    /// Set the overview / synopsis.
    pub fn with_overview(mut self, value: impl Into<String>) -> Self {
        self.overview = Some(value.into());
        self
    }
    /// Set the release date.
    pub fn with_release_date(mut self, value: PartialDate) -> Self {
        self.release_date = Some(value);
        self
    }
    /// Set the poster image URL.
    pub fn with_poster_url(mut self, value: impl Into<String>) -> Self {
        self.poster_url = Some(value.into());
        self
    }
    /// Set the backdrop image URL.
    pub fn with_backdrop_url(mut self, value: impl Into<String>) -> Self {
        self.backdrop_url = Some(value.into());
        self
    }
    /// Set the genres.
    pub fn with_genres(mut self, value: Vec<Genre>) -> Self {
        self.genres = value;
        self
    }
    /// Set the popularity score.
    pub fn with_popularity(mut self, value: f64) -> Self {
        self.popularity = Some(value);
        self
    }
    /// Set the average vote score.
    pub fn with_vote_average(mut self, value: f64) -> Self {
        self.vote_average = Some(value);
        self
    }
    /// Set the vote count.
    pub fn with_vote_count(mut self, value: u64) -> Self {
        self.vote_count = Some(value);
        self
    }
    /// Set the original language (ISO 639-1).
    pub fn with_original_language(mut self, value: impl Into<String>) -> Self {
        self.original_language = Some(value.into());
        self
    }
    /// Set whether this is adult content.
    pub fn with_adult(mut self, value: bool) -> Self {
        self.adult = Some(value);
        self
    }
    /// Set the runtime in minutes.
    pub fn with_runtime(mut self, value: u32) -> Self {
        self.runtime = Some(value);
        self
    }
    /// Set the release status.
    pub fn with_status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
        self
    }
}

impl UnifiedMovieDetails {
    /// Create movie details wrapping a base [`UnifiedMovie`].
    pub fn new(movie: UnifiedMovie) -> Self {
        Self {
            movie,
            tagline: None,
            runtime: None,
            budget: None,
            revenue: None,
            status: None,
            homepage: None,
            imdb_id: None,
            production_companies: Vec::new(),
            production_countries: Vec::new(),
            spoken_languages: Vec::new(),
            video: None,
            belongs_to_collection: None,
            certifications: Vec::new(),
        }
    }

    /// Set the tagline.
    pub fn with_tagline(mut self, value: impl Into<String>) -> Self {
        self.tagline = Some(value.into());
        self
    }
    /// Set the runtime in minutes.
    pub fn with_runtime(mut self, value: u32) -> Self {
        self.runtime = Some(value);
        self
    }
    /// Set the production budget in USD.
    pub fn with_budget(mut self, value: u64) -> Self {
        self.budget = Some(value);
        self
    }
    /// Set the box-office revenue in USD.
    pub fn with_revenue(mut self, value: u64) -> Self {
        self.revenue = Some(value);
        self
    }
    /// Set the release status.
    pub fn with_status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
        self
    }
    /// Set the homepage URL.
    pub fn with_homepage(mut self, value: impl Into<String>) -> Self {
        self.homepage = Some(value.into());
        self
    }
    /// Set the IMDB id.
    pub fn with_imdb_id(mut self, value: impl Into<String>) -> Self {
        self.imdb_id = Some(value.into());
        self
    }
    /// Set the production companies.
    pub fn with_production_companies(mut self, value: Vec<Company>) -> Self {
        self.production_companies = value;
        self
    }
    /// Set the production countries.
    pub fn with_production_countries(mut self, value: Vec<String>) -> Self {
        self.production_countries = value;
        self
    }
    /// Set the spoken languages.
    pub fn with_spoken_languages(mut self, value: Vec<String>) -> Self {
        self.spoken_languages = value;
        self
    }
    /// Set whether the movie has a video entry (stored as `Some(value)`).
    pub fn with_video(mut self, value: bool) -> Self {
        self.video = Some(value);
        self
    }
    /// Set the collection this movie belongs to.
    pub fn with_collection(mut self, value: Collection) -> Self {
        self.belongs_to_collection = Some(value);
        self
    }
    /// Set the certifications (content ratings).
    pub fn with_certifications(mut self, value: Vec<Certification>) -> Self {
        self.certifications = value;
        self
    }
}

impl Collection {
    /// Create a collection with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            poster_url: None,
            backdrop_url: None,
        }
    }

    /// Set the collection id.
    pub fn with_id(mut self, value: u64) -> Self {
        self.id = Some(value);
        self
    }
    /// Set the poster image URL.
    pub fn with_poster_url(mut self, value: impl Into<String>) -> Self {
        self.poster_url = Some(value.into());
        self
    }
    /// Set the backdrop image URL.
    pub fn with_backdrop_url(mut self, value: impl Into<String>) -> Self {
        self.backdrop_url = Some(value.into());
        self
    }
}
