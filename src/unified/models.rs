/// A movie in the unified model.
#[derive(Debug, Clone)]
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
    /// Genre names.
    pub genres: Vec<String>,
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
#[derive(Debug, Clone)]
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
}

/// A TV show in the unified model.
#[derive(Debug, Clone)]
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
    /// Genre names.
    pub genres: Vec<String>,
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
}

/// Detailed TV show information in the unified model.
#[derive(Debug, Clone)]
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
}

/// A person in the unified model.
#[derive(Debug, Clone)]
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
}

/// Detailed person information in the unified model.
#[derive(Debug, Clone)]
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
}

/// A search result that can be a movie, TV show, or person.
#[derive(Debug, Clone)]
pub enum UnifiedSearchResult {
    /// Movie result.
    Movie(UnifiedMovie),
    /// TV show result.
    TvShow(UnifiedTvShow),
    /// Person result.
    Person(UnifiedPerson),
}
