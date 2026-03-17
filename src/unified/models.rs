use serde::{Deserialize, Serialize};

use super::genre::Genre;

/// A movie in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedMovie {
    /// Provider-qualified ID (e.g. `"tmdb:550"`).
    pub provider_id: String,
    /// Movie title.
    pub title: String,
    /// Original title in the original language.
    pub original_title: Option<String>,
    /// Plot overview / synopsis.
    pub overview: Option<String>,
    /// Release date as a string (YYYY-MM-DD).
    pub release_date: Option<String>,
    /// Full poster image URL (resolved from provider).
    pub poster_url: Option<String>,
    /// Full backdrop image URL (resolved from provider).
    pub backdrop_url: Option<String>,
    /// Genres.
    pub genres: Vec<Genre>,
    /// Popularity score.
    pub popularity: Option<f64>,
    /// Average vote score.
    pub vote_average: Option<f64>,
    /// Total vote count.
    pub vote_count: u64,
    /// Original language (ISO 639-1).
    pub original_language: Option<String>,
    /// Whether this is adult content.
    pub adult: bool,
}

/// Detailed movie information in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedMovieDetails {
    /// Base movie info.
    pub movie: UnifiedMovie,
    /// Tagline.
    pub tagline: Option<String>,
    /// Runtime in minutes.
    pub runtime: Option<u32>,
    /// Production budget in USD.
    pub budget: Option<u64>,
    /// Box office revenue in USD.
    pub revenue: Option<u64>,
    /// Release status (e.g. "Released", "In Production").
    pub status: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// IMDB ID (e.g. `"tt0137523"`).
    pub imdb_id: Option<String>,
    /// Production companies.
    pub production_companies: Vec<String>,
    /// Production countries.
    pub production_countries: Vec<String>,
    /// Spoken languages.
    pub spoken_languages: Vec<String>,
    /// Whether the movie has a video.
    pub video: bool,
    /// Name of the collection this movie belongs to, if any.
    pub belongs_to_collection: Option<String>,
}

/// A TV show in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedTvShow {
    /// Provider-qualified ID (e.g. `"tmdb:1396"`).
    pub provider_id: String,
    /// Show name.
    pub name: String,
    /// Original name in the original language.
    pub original_name: Option<String>,
    /// Plot overview / synopsis.
    pub overview: Option<String>,
    /// First air date as a string (YYYY-MM-DD).
    pub first_air_date: Option<String>,
    /// Full poster image URL.
    pub poster_url: Option<String>,
    /// Full backdrop image URL.
    pub backdrop_url: Option<String>,
    /// Genres.
    pub genres: Vec<Genre>,
    /// Popularity score.
    pub popularity: Option<f64>,
    /// Average vote score.
    pub vote_average: Option<f64>,
    /// Total vote count.
    pub vote_count: u64,
    /// Original language (ISO 639-1).
    pub original_language: Option<String>,
    /// Origin countries.
    pub origin_country: Vec<String>,
    /// Whether this is adult content.
    pub adult: bool,
}

/// Detailed TV show information in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedTvShowDetails {
    /// Base TV show info.
    pub show: UnifiedTvShow,
    /// Tagline.
    pub tagline: Option<String>,
    /// Number of seasons.
    pub number_of_seasons: u32,
    /// Number of episodes.
    pub number_of_episodes: u32,
    /// Whether the show is still in production.
    pub in_production: bool,
    /// Release status.
    pub status: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Networks that air the show.
    pub networks: Vec<String>,
    /// Production companies.
    pub production_companies: Vec<String>,
    /// Last air date as a string (YYYY-MM-DD).
    pub last_air_date: Option<String>,
    /// Type of show (e.g. "Scripted", "Reality").
    pub type_: Option<String>,
    /// Creators of the show.
    pub created_by: Vec<String>,
    /// Episode run times in minutes.
    pub episode_run_time: Vec<u32>,
    /// Spoken languages (English names).
    pub spoken_languages: Vec<String>,
    /// Production countries (ISO 3166-1 names).
    pub production_countries: Vec<String>,
}

/// A person in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedPerson {
    /// Provider-qualified ID (e.g. `"tmdb:287"`).
    pub provider_id: String,
    /// Person's name.
    pub name: String,
    /// Known-for department (e.g. "Acting", "Directing").
    pub known_for_department: Option<String>,
    /// Full profile image URL.
    pub profile_url: Option<String>,
    /// Popularity score.
    pub popularity: Option<f64>,
    /// Gender code (1=female, 2=male, 3=non-binary, 0=not specified).
    pub gender: Option<i32>,
    /// Whether this person is associated with adult content.
    pub adult: bool,
}

/// Detailed person information in the unified model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedPersonDetails {
    /// Base person info.
    pub person: UnifiedPerson,
    /// Biography.
    pub biography: Option<String>,
    /// Birthday (YYYY-MM-DD).
    pub birthday: Option<String>,
    /// Death day (YYYY-MM-DD), if applicable.
    pub deathday: Option<String>,
    /// Place of birth.
    pub place_of_birth: Option<String>,
    /// IMDB ID.
    pub imdb_id: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Other names the person is known by.
    pub also_known_as: Vec<String>,
}

/// A search result that can be a movie, TV show, or person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnifiedSearchResult {
    /// Movie result.
    Movie(UnifiedMovie),
    /// TV show result.
    TvShow(UnifiedTvShow),
    /// Person result.
    Person(UnifiedPerson),
}

/// Detailed season information for a TV show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedSeasonDetails {
    /// Provider-qualified show ID (e.g. `"tmdb:1396"`).
    pub show_id: String,
    /// Season number.
    pub season_number: u32,
    /// Season name.
    pub name: Option<String>,
    /// Overview / synopsis.
    pub overview: Option<String>,
    /// Air date as a string (YYYY-MM-DD).
    pub air_date: Option<String>,
    /// Full poster image URL.
    pub poster_url: Option<String>,
    /// Episodes in this season.
    pub episodes: Vec<UnifiedEpisode>,
}

/// A single episode in a TV show season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedEpisode {
    /// Episode number within the season.
    pub episode_number: u32,
    /// Episode name.
    pub name: Option<String>,
    /// Episode overview.
    pub overview: Option<String>,
    /// Air date as a string (YYYY-MM-DD).
    pub air_date: Option<String>,
    /// Runtime in minutes.
    pub runtime: Option<u32>,
    /// Full still image URL.
    pub still_url: Option<String>,
    /// Average vote score.
    pub vote_average: Option<f64>,
}

/// Streaming availability for a movie or TV show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedWatchProviders {
    /// Provider-qualified media ID (e.g. `"tmdb:550"`).
    pub provider_id: String,
    /// Per-country watch provider entries, keyed by ISO 3166-1 alpha-2 country code.
    pub results: std::collections::HashMap<String, UnifiedWatchProviderEntry>,
}

/// Streaming services available in a specific country.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UnifiedWatchProviderEntry {
    /// Services offering flat-rate / subscription streaming.
    pub flatrate: Vec<UnifiedStreamingService>,
    /// Services offering rental.
    pub rent: Vec<UnifiedStreamingService>,
    /// Services offering purchase.
    pub buy: Vec<UnifiedStreamingService>,
}

/// A single streaming service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedStreamingService {
    /// Service name (e.g. "Netflix").
    pub name: String,
    /// Full logo image URL.
    pub logo_url: Option<String>,
}
