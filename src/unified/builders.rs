//! Constructors and builder-style setters for the unified models.
//!
//! Every `Unified*` model is `#[non_exhaustive]`, which forbids struct-literal
//! construction outside this crate. Out-of-crate providers implementing the
//! unified traits still need to *build* the values their impls return, so each
//! model exposes a `new(...)` constructor (taking the genuinely required
//! fields) plus chainable `with_*` setters for the optional ones.
//!
//! ```
//! use cameo::{MediaId, PartialDate, UnifiedMovie};
//!
//! let movie = UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
//!     .with_overview("A ticking-time-bomb insomniac...")
//!     .with_release_date(PartialDate::from_year(1999))
//!     .with_vote_average(8.4);
//! assert_eq!(movie.title, "Fight Club");
//! ```

use super::{date::PartialDate, genre::Genre, media_id::MediaId, models::*};

impl UnifiedMovie {
    /// Create a movie with the required identity fields; all other fields
    /// default to empty/absent and can be set with the `with_*` methods.
    pub fn new(provider_id: MediaId, title: impl Into<String>) -> Self {
        Self {
            provider_id,
            title: title.into(),
            original_title: None,
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
            video: false,
            belongs_to_collection: None,
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
    /// Set whether the movie has a video.
    pub fn with_video(mut self, value: bool) -> Self {
        self.video = value;
        self
    }
    /// Set the collection this movie belongs to.
    pub fn with_collection(mut self, value: Collection) -> Self {
        self.belongs_to_collection = Some(value);
        self
    }
}

impl UnifiedTvShow {
    /// Create a TV show with the required identity fields.
    pub fn new(provider_id: MediaId, name: impl Into<String>) -> Self {
        Self {
            provider_id,
            name: name.into(),
            original_name: None,
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
    pub fn with_status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
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
            in_production: false,
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
    /// Set whether the show is still in production.
    pub fn with_in_production(mut self, value: bool) -> Self {
        self.in_production = value;
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
    /// Set the show classification (e.g. "Scripted").
    pub fn with_type(mut self, value: impl Into<String>) -> Self {
        self.type_ = Some(value.into());
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

impl UnifiedWatchProviders {
    /// Create an empty watch-providers set for the given media id.
    pub fn new(provider_id: MediaId) -> Self {
        Self {
            provider_id,
            results: std::collections::HashMap::new(),
        }
    }

    /// Insert a per-country entry.
    pub fn with_country(
        mut self,
        country_code: impl Into<String>,
        entry: UnifiedWatchProviderEntry,
    ) -> Self {
        self.results.insert(country_code.into(), entry);
        self
    }
}

impl UnifiedWatchProviderEntry {
    /// Create an empty entry (equivalent to [`Default::default`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the flat-rate / subscription services.
    pub fn with_flatrate(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.flatrate = value;
        self
    }
    /// Set the rental services.
    pub fn with_rent(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.rent = value;
        self
    }
    /// Set the purchase services.
    pub fn with_buy(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.buy = value;
        self
    }
}

impl UnifiedStreamingService {
    /// Create a streaming service with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            logo_url: None,
        }
    }

    /// Set the logo image URL.
    pub fn with_logo_url(mut self, value: impl Into<String>) -> Self {
        self.logo_url = Some(value.into());
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
