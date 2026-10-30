use serde::{Deserialize, Serialize};

use super::{date::PartialDate, genre::Genre, media_id::MediaId};

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
    /// unknown gender is absent rather than fabricated.
    pub fn from_tmdb(code: i64) -> Option<Self> {
        match code {
            1 => Some(Gender::Female),
            2 => Some(Gender::Male),
            3 => Some(Gender::NonBinary),
            _ => None,
        }
    }

    /// Map an AniList gender string (`"Female"`, `"Male"`, `"Non-binary"`).
    ///
    /// A recognized-but-other string maps to [`Gender::NotSpecified`]; only an
    /// absent value (handled by the caller) yields `None`.
    pub fn from_anilist(value: &str) -> Self {
        match value {
            "Female" => Gender::Female,
            "Male" => Gender::Male,
            "Non-binary" => Gender::NonBinary,
            _ => Gender::NotSpecified,
        }
    }
}

/// A collection (franchise) a movie belongs to.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    /// Provider collection id, if known.
    pub id: Option<u64>,
    /// Collection name.
    pub name: String,
    /// Full poster image URL.
    pub poster_url: Option<String>,
    /// Full backdrop image URL.
    pub backdrop_url: Option<String>,
}

/// A production company / studio.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Company {
    /// Provider company id, if known.
    pub id: Option<u64>,
    /// Company name.
    pub name: String,
    /// Full logo image URL.
    pub logo_url: Option<String>,
    /// ISO 3166-1 country of origin.
    pub origin_country: Option<String>,
}

/// A TV network that airs a show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Network {
    /// Provider network id, if known.
    pub id: Option<u64>,
    /// Network name.
    pub name: String,
    /// Full logo image URL.
    pub logo_url: Option<String>,
    /// ISO 3166-1 country of origin.
    pub origin_country: Option<String>,
}

/// A credited creator of a TV show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Creator {
    /// Provider person id, if known.
    pub id: Option<u64>,
    /// Creator name.
    pub name: String,
    /// Full profile image URL.
    pub profile_url: Option<String>,
}

/// A movie in the unified model.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedMovie {
    /// Provider-qualified media id (e.g. `tmdb:550`).
    pub provider_id: MediaId,
    /// Movie title.
    pub title: String,
    /// Original title in the original language.
    pub original_title: Option<String>,
    /// Plot overview / synopsis.
    pub overview: Option<String>,
    /// Release date (may be partial, e.g. year-only from some providers).
    pub release_date: Option<PartialDate>,
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
    /// Total vote count, if the provider reports one.
    pub vote_count: Option<u64>,
    /// Original language (ISO 639-1).
    pub original_language: Option<String>,
    /// Whether this is adult content, if the provider reports it.
    pub adult: Option<bool>,
}

/// Detailed movie information in the unified model.
#[non_exhaustive]
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
    pub production_companies: Vec<Company>,
    /// Production countries.
    pub production_countries: Vec<String>,
    /// Spoken languages.
    pub spoken_languages: Vec<String>,
    /// Whether the movie has a video entry, if the provider reports it.
    ///
    /// `None` means unknown: the provider did not report the flag (the key was
    /// absent or `null`) or has no such concept.
    pub video: Option<bool>,
    /// The collection (franchise) this movie belongs to, if any.
    pub belongs_to_collection: Option<Collection>,
}

/// A TV show in the unified model.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedTvShow {
    /// Provider-qualified media id (e.g. `tmdb:1396`).
    pub provider_id: MediaId,
    /// Show name.
    pub name: String,
    /// Original name in the original language.
    pub original_name: Option<String>,
    /// Plot overview / synopsis.
    pub overview: Option<String>,
    /// First air date (may be partial, e.g. year-only from some providers).
    pub first_air_date: Option<PartialDate>,
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
    /// Total vote count, if the provider reports one.
    pub vote_count: Option<u64>,
    /// Original language (ISO 639-1).
    pub original_language: Option<String>,
    /// Origin countries.
    pub origin_country: Vec<String>,
    /// Whether this is adult content, if the provider reports it.
    pub adult: Option<bool>,
}

/// Detailed TV show information in the unified model.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedTvShowDetails {
    /// Base TV show info.
    pub show: UnifiedTvShow,
    /// Tagline.
    pub tagline: Option<String>,
    /// Number of seasons, if the provider reports one.
    pub number_of_seasons: Option<u32>,
    /// Number of episodes, if the provider reports one.
    pub number_of_episodes: Option<u32>,
    /// Whether the show is still in production, if known.
    ///
    /// `None` means unknown: the provider did not report the flag (the key was
    /// absent or `null`) or reported no status to derive it from.
    pub in_production: Option<bool>,
    /// Release status.
    pub status: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Networks that air the show.
    pub networks: Vec<Network>,
    /// Production companies.
    pub production_companies: Vec<Company>,
    /// Last air date (may be partial, e.g. year-only from some providers).
    pub last_air_date: Option<PartialDate>,
    /// Show classification (e.g. "Scripted", "Reality", "Documentary").
    pub type_: Option<String>,
    /// Provider media format (e.g. AniList's "TV", "OVA", "MOVIE").
    pub media_format: Option<String>,
    /// Creators of the show.
    pub created_by: Vec<Creator>,
    /// Episode run times in minutes.
    pub episode_run_time: Vec<u32>,
    /// Spoken languages (English names).
    pub spoken_languages: Vec<String>,
    /// Production countries (ISO 3166-1 names).
    pub production_countries: Vec<String>,
    /// The show's seasons.
    pub seasons: Vec<UnifiedSeason>,
}

/// Summary information about a single season of a TV show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedSeason {
    /// Season number.
    pub season_number: u32,
    /// Season name.
    pub name: Option<String>,
    /// Overview / synopsis.
    pub overview: Option<String>,
    /// Air date (may be partial, e.g. year-only from some providers).
    pub air_date: Option<PartialDate>,
    /// Number of episodes in the season, if known.
    pub episode_count: Option<u32>,
    /// Full poster image URL.
    pub poster_url: Option<String>,
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

/// A search result that can be a movie, TV show, or person.
#[non_exhaustive]
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
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedSeasonDetails {
    /// Provider-qualified show id (e.g. `tmdb:1396`).
    pub show_id: MediaId,
    /// Season number.
    pub season_number: u32,
    /// Season name.
    pub name: Option<String>,
    /// Overview / synopsis.
    pub overview: Option<String>,
    /// Air date (may be partial, e.g. year-only from some providers).
    pub air_date: Option<PartialDate>,
    /// Full poster image URL.
    pub poster_url: Option<String>,
    /// Episodes in this season.
    pub episodes: Vec<UnifiedEpisode>,
}

/// A single episode in a TV show season.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedEpisode {
    /// Episode number within the season.
    pub episode_number: u32,
    /// Episode name.
    pub name: Option<String>,
    /// Episode overview.
    pub overview: Option<String>,
    /// Air date (may be partial, e.g. year-only from some providers).
    pub air_date: Option<PartialDate>,
    /// Runtime in minutes.
    pub runtime: Option<u32>,
    /// Full still image URL.
    pub still_url: Option<String>,
    /// Average vote score.
    pub vote_average: Option<f64>,
}

/// Streaming availability for a movie or TV show.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedWatchProviders {
    /// Provider-qualified media id (e.g. `tmdb:550`).
    pub provider_id: MediaId,
    /// Per-country watch provider entries, keyed by ISO 3166-1 alpha-2 country code.
    pub results: std::collections::HashMap<String, UnifiedWatchProviderEntry>,
}

/// Streaming services available in a specific country.
#[non_exhaustive]
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
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedStreamingService {
    /// Service name (e.g. "Netflix").
    pub name: String,
    /// Full logo image URL.
    pub logo_url: Option<String>,
}
