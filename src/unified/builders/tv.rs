//! Builders for the TV family: [`UnifiedTvShow`], [`UnifiedTvShowDetails`],
//! [`UnifiedSeason`], [`UnifiedSeasonDetails`], [`UnifiedEpisode`],
//! [`Network`] and [`Creator`].

use crate::unified::{date::PartialDate, genre::Genre, media_id::MediaId, models::*};

impl UnifiedTvShow {
    /// Create a TV show with the required identity fields.
    pub fn new(provider_id: MediaId, name: impl Into<String>) -> Self {
        Self {
            provider_id,
            name: name.into(),
            original_name: None,
            alternative_titles: Vec::new(),
            overview: None,
            first_air_date: None,
            poster_url: None,
            backdrop_url: None,
            genres: Vec::new(),
            popularity: None,
            vote_average: None,
            vote_count: None,
            original_language: None,
            origin_country: Vec::new(),
            adult: None,
            number_of_episodes: None,
            episode_run_time: None,
            status: None,
        }
    }

    /// Set the original-language name.
    pub fn with_original_name(mut self, value: impl Into<String>) -> Self {
        self.original_name = Some(value.into());
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
    /// Set the first air date.
    pub fn with_first_air_date(mut self, value: PartialDate) -> Self {
        self.first_air_date = Some(value);
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
    /// Set the origin countries.
    pub fn with_origin_country(mut self, value: Vec<String>) -> Self {
        self.origin_country = value;
        self
    }
    /// Set whether this is adult content.
    pub fn with_adult(mut self, value: bool) -> Self {
        self.adult = Some(value);
        self
    }
    /// Set the number of episodes.
    pub fn with_number_of_episodes(mut self, value: u32) -> Self {
        self.number_of_episodes = Some(value);
        self
    }
    /// Set the typical run time of one episode in minutes.
    pub fn with_episode_run_time(mut self, value: u32) -> Self {
        self.episode_run_time = Some(value);
        self
    }
    /// Set the release status.
    pub fn with_status(mut self, value: SeriesStatus) -> Self {
        self.status = Some(value);
        self
    }
}

impl UnifiedTvShowDetails {
    /// Create TV show details wrapping a base [`UnifiedTvShow`].
    pub fn new(show: UnifiedTvShow) -> Self {
        Self {
            show,
            tagline: None,
            number_of_seasons: None,
            number_of_episodes: None,
            in_production: None,
            status: None,
            homepage: None,
            networks: Vec::new(),
            production_companies: Vec::new(),
            last_air_date: None,
            type_: None,
            media_format: None,
            created_by: Vec::new(),
            episode_run_time: Vec::new(),
            spoken_languages: Vec::new(),
            production_countries: Vec::new(),
            seasons: Vec::new(),
            certifications: Vec::new(),
        }
    }

    /// Set the tagline.
    pub fn with_tagline(mut self, value: impl Into<String>) -> Self {
        self.tagline = Some(value.into());
        self
    }
    /// Set the number of seasons.
    pub fn with_number_of_seasons(mut self, value: u32) -> Self {
        self.number_of_seasons = Some(value);
        self
    }
    /// Set the number of episodes.
    pub fn with_number_of_episodes(mut self, value: u32) -> Self {
        self.number_of_episodes = Some(value);
        self
    }
    /// Set whether the show is still in production (stored as `Some(value)`).
    pub fn with_in_production(mut self, value: bool) -> Self {
        self.in_production = Some(value);
        self
    }
    /// Set the release status.
    pub fn with_status(mut self, value: SeriesStatus) -> Self {
        self.status = Some(value);
        self
    }
    /// Set the homepage URL.
    pub fn with_homepage(mut self, value: impl Into<String>) -> Self {
        self.homepage = Some(value.into());
        self
    }
    /// Set the networks.
    pub fn with_networks(mut self, value: Vec<Network>) -> Self {
        self.networks = value;
        self
    }
    /// Set the production companies.
    pub fn with_production_companies(mut self, value: Vec<Company>) -> Self {
        self.production_companies = value;
        self
    }
    /// Set the last air date.
    pub fn with_last_air_date(mut self, value: PartialDate) -> Self {
        self.last_air_date = Some(value);
        self
    }
    /// Set the show classification (e.g. [`SeriesType::Scripted`]).
    pub fn with_type(mut self, value: SeriesType) -> Self {
        self.type_ = Some(value);
        self
    }
    /// Set the provider media format (e.g. "TV", "OVA").
    pub fn with_media_format(mut self, value: impl Into<String>) -> Self {
        self.media_format = Some(value.into());
        self
    }
    /// Set the creators.
    pub fn with_created_by(mut self, value: Vec<Creator>) -> Self {
        self.created_by = value;
        self
    }
    /// Set the episode run times in minutes.
    pub fn with_episode_run_time(mut self, value: Vec<u32>) -> Self {
        self.episode_run_time = value;
        self
    }
    /// Set the spoken languages.
    pub fn with_spoken_languages(mut self, value: Vec<String>) -> Self {
        self.spoken_languages = value;
        self
    }
    /// Set the production countries.
    pub fn with_production_countries(mut self, value: Vec<String>) -> Self {
        self.production_countries = value;
        self
    }
    /// Set the seasons.
    pub fn with_seasons(mut self, value: Vec<UnifiedSeason>) -> Self {
        self.seasons = value;
        self
    }
    /// Set the certifications (content ratings).
    pub fn with_certifications(mut self, value: Vec<Certification>) -> Self {
        self.certifications = value;
        self
    }
}

impl UnifiedSeason {
    /// Create a season summary for the given season number.
    pub fn new(season_number: u32) -> Self {
        Self {
            season_number,
            name: None,
            overview: None,
            air_date: None,
            episode_count: None,
            poster_url: None,
        }
    }

    /// Set the season name.
    pub fn with_name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }
    /// Set the overview.
    pub fn with_overview(mut self, value: impl Into<String>) -> Self {
        self.overview = Some(value.into());
        self
    }
    /// Set the air date.
    pub fn with_air_date(mut self, value: PartialDate) -> Self {
        self.air_date = Some(value);
        self
    }
    /// Set the episode count.
    pub fn with_episode_count(mut self, value: u32) -> Self {
        self.episode_count = Some(value);
        self
    }
    /// Set the poster image URL.
    pub fn with_poster_url(mut self, value: impl Into<String>) -> Self {
        self.poster_url = Some(value.into());
        self
    }
}

impl UnifiedSeasonDetails {
    /// Create season details for the given show and season number.
    pub fn new(show_id: MediaId, season_number: u32) -> Self {
        Self {
            show_id,
            season_number,
            name: None,
            overview: None,
            air_date: None,
            poster_url: None,
            episodes: Vec::new(),
        }
    }

    /// Set the season name.
    pub fn with_name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }
    /// Set the overview.
    pub fn with_overview(mut self, value: impl Into<String>) -> Self {
        self.overview = Some(value.into());
        self
    }
    /// Set the air date.
    pub fn with_air_date(mut self, value: PartialDate) -> Self {
        self.air_date = Some(value);
        self
    }
    /// Set the poster image URL.
    pub fn with_poster_url(mut self, value: impl Into<String>) -> Self {
        self.poster_url = Some(value.into());
        self
    }
    /// Set the episodes.
    pub fn with_episodes(mut self, value: Vec<UnifiedEpisode>) -> Self {
        self.episodes = value;
        self
    }
}

impl UnifiedEpisode {
    /// Create an episode for the given episode number.
    pub fn new(episode_number: u32) -> Self {
        Self {
            episode_number,
            name: None,
            overview: None,
            air_date: None,
            runtime: None,
            still_url: None,
            vote_average: None,
        }
    }

    /// Set the episode name.
    pub fn with_name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }
    /// Set the overview.
    pub fn with_overview(mut self, value: impl Into<String>) -> Self {
        self.overview = Some(value.into());
        self
    }
    /// Set the air date.
    pub fn with_air_date(mut self, value: PartialDate) -> Self {
        self.air_date = Some(value);
        self
    }
    /// Set the runtime in minutes.
    pub fn with_runtime(mut self, value: u32) -> Self {
        self.runtime = Some(value);
        self
    }
    /// Set the still image URL.
    pub fn with_still_url(mut self, value: impl Into<String>) -> Self {
        self.still_url = Some(value.into());
        self
    }
    /// Set the average vote score.
    pub fn with_vote_average(mut self, value: f64) -> Self {
        self.vote_average = Some(value);
        self
    }
}

impl Network {
    /// Create a network with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            logo_url: None,
            origin_country: None,
        }
    }

    /// Set the network id.
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

impl Creator {
    /// Create a creator with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            profile_url: None,
        }
    }

    /// Set the creator id.
    pub fn with_id(mut self, value: u64) -> Self {
        self.id = Some(value);
        self
    }
    /// Set the profile image URL.
    pub fn with_profile_url(mut self, value: impl Into<String>) -> Self {
        self.profile_url = Some(value.into());
        self
    }
}
