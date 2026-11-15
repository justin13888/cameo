//! TMDB movie type → unified model conversions.

use super::{resolve_backdrop, resolve_date, resolve_logo, resolve_poster};
use crate::{
    generated::tmdb::types,
    unified::{genre::Genre, media_id::MediaId, models::*},
};

macro_rules! impl_movie_from {
    ($ty:ty) => {
        impl From<$ty> for UnifiedMovie {
            fn from(m: $ty) -> Self {
                UnifiedMovie {
                    provider_id: MediaId::tmdb(m.id.unwrap_or_default() as u64),
                    title: m.title.unwrap_or_default(),
                    original_title: m.original_title,
                    alternative_titles: Vec::new(),
                    overview: m.overview,
                    release_date: resolve_date(&m.release_date),
                    poster_url: resolve_poster(&m.poster_path),
                    backdrop_url: resolve_backdrop(&m.backdrop_path),
                    genres: m
                        .genre_ids
                        .iter()
                        .flatten()
                        .map(|&id| Genre::from_tmdb_id(id))
                        .collect(),
                    popularity: m.popularity,
                    vote_average: m.vote_average,
                    vote_count: m.vote_count.map(|c| c.max(0) as u64),
                    original_language: m.original_language,
                    adult: m.adult,
                    // TMDB list endpoints return neither runtime nor status.
                    runtime: None,
                    status: None,
                }
            }
        }
    };
}

impl_movie_from!(types::SearchMovieResponseResultsItem);
impl_movie_from!(types::TrendingMoviesResponseResultsItem);
impl_movie_from!(types::MoviePopularListResponseResultsItem);
impl_movie_from!(types::MovieTopRatedListResponseResultsItem);
impl_movie_from!(types::DiscoverMovieResponseResultsItem);
impl_movie_from!(types::MovieSimilarResponseResultsItem);

impl From<types::MovieDetailsResponseProductionCompaniesItem> for Company {
    fn from(c: types::MovieDetailsResponseProductionCompaniesItem) -> Self {
        Company {
            id: c.id.and_then(|id| u64::try_from(id).ok()),
            name: c.name.unwrap_or_default(),
            logo_url: resolve_logo(&c.logo_path),
            origin_country: c.origin_country,
        }
    }
}

impl From<types::MovieDetailsResponse> for UnifiedMovieDetails {
    fn from(m: types::MovieDetailsResponse) -> Self {
        let belongs_to_collection = m.belongs_to_collection.as_ref().map(parse_collection);
        let runtime = m.runtime.filter(|&v| v > 0).map(|v| v as u32);

        UnifiedMovieDetails {
            movie: UnifiedMovie {
                provider_id: MediaId::tmdb(m.id.unwrap_or_default() as u64),
                title: m.title.unwrap_or_default(),
                original_title: m.original_title,
                alternative_titles: Vec::new(),
                overview: m.overview,
                release_date: resolve_date(&m.release_date),
                poster_url: resolve_poster(&m.poster_path),
                backdrop_url: resolve_backdrop(&m.backdrop_path),
                genres: m
                    .genres
                    .iter()
                    .flatten()
                    .filter_map(|g| g.id.map(Genre::from_tmdb_id))
                    .collect(),
                popularity: m.popularity,
                vote_average: m.vote_average,
                vote_count: m.vote_count.map(|c| c.max(0) as u64),
                original_language: m.original_language,
                adult: m.adult,
                // The embedded summary mirrors the detail-level runtime/status.
                runtime,
                status: m.status.clone(),
            },
            tagline: m.tagline,
            runtime,
            budget: m.budget.filter(|&v| v > 0).map(|v| v as u64),
            revenue: m.revenue.filter(|&v| v > 0).map(|v| v as u64),
            status: m.status,
            homepage: m.homepage,
            imdb_id: m.imdb_id,
            production_companies: m
                .production_companies
                .into_iter()
                .flatten()
                .map(Company::from)
                .collect(),
            production_countries: m
                .production_countries
                .iter()
                .flatten()
                .filter_map(|c| c.name.clone())
                .collect(),
            spoken_languages: m
                .spoken_languages
                .iter()
                .flatten()
                .filter_map(|l| l.english_name.clone())
                .collect(),
            // Spec default was `true`; an absent/null flag stays unknown.
            video: m.video,
            belongs_to_collection,
        }
    }
}

/// Convert TMDB's `belongs_to_collection` object into a [`Collection`].
fn parse_collection(v: &types::MovieDetailsResponseBelongsToCollection) -> Collection {
    Collection {
        id: v.id.and_then(|id| u64::try_from(id).ok()),
        name: v.name.clone().unwrap_or_default(),
        poster_url: resolve_poster(&v.poster_path),
        backdrop_url: resolve_backdrop(&v.backdrop_path),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generated::tmdb::types,
        unified::{
            genre::Genre,
            models::{UnifiedMovie, UnifiedMovieDetails},
        },
    };

    fn make_search_movie() -> types::SearchMovieResponseResultsItem {
        types::SearchMovieResponseResultsItem {
            id: Some(550),
            title: Some("Fight Club".to_string()),
            original_title: Some("Fight Club".to_string()),
            overview: Some("A ticking-time-bomb insomniac...".to_string()),
            release_date: Some("1999-10-15".to_string()),
            poster_path: Some("/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg".to_string()),
            backdrop_path: Some("/hZkgoQYus5vegHoetLkCJzb17zJ.jpg".to_string()),
            genre_ids: Some(vec![18, 53]),
            popularity: Some(73.433),
            vote_average: Some(8.4),
            vote_count: Some(26280),
            original_language: Some("en".to_string()),
            adult: Some(false),
            video: Some(false),
        }
    }

    #[test]
    fn search_movie_to_unified() {
        let unified: UnifiedMovie = make_search_movie().into();
        assert_eq!(unified.provider_id, "tmdb:550");
        // TMDB exposes no title variants beyond title/original_title.
        assert!(unified.alternative_titles.is_empty());
        assert_eq!(unified.title, "Fight Club");
        assert_eq!(
            unified.release_date.map(|d| d.to_string()).as_deref(),
            Some("1999-10-15")
        );
        assert_eq!(unified.vote_count, Some(26280));
        assert_eq!(unified.adult, Some(false));
        // Genres resolved from IDs (18=Drama, 53=Thriller).
        assert_eq!(unified.genres, vec![Genre::Drama, Genre::Thriller]);
        assert!(
            unified
                .poster_url
                .as_deref()
                .unwrap()
                .starts_with("https://image.tmdb.org/t/p/")
        );
    }

    #[test]
    fn search_movie_list_has_no_runtime_or_status() {
        // TMDB list endpoints return neither field.
        let unified: UnifiedMovie = make_search_movie().into();
        assert_eq!(unified.runtime, None);
        assert_eq!(unified.status, None);
    }

    /// TMDB details fill the embedded summary from the detail-level values.
    #[test]
    fn movie_details_embedded_movie_mirrors_detail_fields() {
        let raw: types::MovieDetailsResponse = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/movie_details_response.json"
        ))
        .expect("fixture deserializes");
        let details: UnifiedMovieDetails = raw.into();
        assert_eq!(details.runtime, Some(139));
        assert_eq!(details.movie.runtime, details.runtime);
        assert_eq!(details.status.as_deref(), Some("Released"));
        assert_eq!(details.movie.status, details.status);
    }

    /// Every detail field is mapped: image paths become full URLs, zero
    /// runtime/budget/revenue become `None`, id-less genres are dropped, and
    /// the loosely-typed collection object is parsed.
    #[test]
    fn movie_details_converts_every_field() {
        let raw: types::MovieDetailsResponse = serde_json::from_value(serde_json::json!({
            "id": 603,
            "title": "The Matrix",
            "original_title": "The Matrix",
            "overview": "A hacker learns the truth.",
            "release_date": "1999-03-31",
            "poster_path": "/poster.jpg",
            "backdrop_path": "/backdrop.jpg",
            "genres": [{ "id": 28, "name": "Action" }, { "name": "No id" }, { "id": 878 }],
            "popularity": 80.5,
            "vote_average": 8.2,
            "vote_count": -3,
            "original_language": "en",
            "adult": false,
            "tagline": "Welcome to the Real World.",
            "runtime": 0,
            "budget": 63000000,
            "revenue": 0,
            "status": "Released",
            "homepage": "https://www.warnerbros.com/movies/matrix",
            "imdb_id": "tt0133093",
            "production_companies": [
                { "id": 79, "name": "Village Roadshow", "logo_path": "/vr.png", "origin_country": "US" },
                { "id": -1 }
            ],
            "production_countries": [{ "iso_3166_1": "US", "name": "United States of America" }, {}],
            "spoken_languages": [{ "english_name": "English", "iso_639_1": "en" }, {}],
            "video": true,
            "belongs_to_collection": {
                "id": 2344,
                "name": "The Matrix Collection",
                "poster_path": "/coll.jpg",
                "backdrop_path": null
            }
        }))
        .expect("details deserialize");

        let details: UnifiedMovieDetails = raw.into();

        let movie = &details.movie;
        assert_eq!(movie.provider_id, "tmdb:603");
        assert_eq!(movie.title, "The Matrix");
        assert_eq!(movie.original_title.as_deref(), Some("The Matrix"));
        assert_eq!(
            movie.overview.as_deref(),
            Some("A hacker learns the truth.")
        );
        assert_eq!(
            movie.release_date.map(|d| d.to_string()).as_deref(),
            Some("1999-03-31")
        );
        assert_eq!(
            movie.poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/poster.jpg")
        );
        assert_eq!(
            movie.backdrop_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w780/backdrop.jpg")
        );
        assert_eq!(movie.genres, vec![Genre::Action, Genre::ScienceFiction]);
        assert_eq!(movie.popularity, Some(80.5));
        assert_eq!(movie.vote_average, Some(8.2));
        // A negative count is clamped to 0, not wrapped.
        assert_eq!(movie.vote_count, Some(0));
        assert_eq!(movie.original_language.as_deref(), Some("en"));
        assert_eq!(movie.adult, Some(false));
        assert!(movie.alternative_titles.is_empty());
        assert_eq!(movie.runtime, None);
        assert_eq!(movie.status.as_deref(), Some("Released"));

        assert_eq!(
            details.tagline.as_deref(),
            Some("Welcome to the Real World.")
        );
        assert_eq!(details.runtime, None);
        assert_eq!(details.budget, Some(63_000_000));
        assert_eq!(details.revenue, None);
        assert_eq!(details.status.as_deref(), Some("Released"));
        assert_eq!(
            details.homepage.as_deref(),
            Some("https://www.warnerbros.com/movies/matrix")
        );
        assert_eq!(details.imdb_id.as_deref(), Some("tt0133093"));
        assert_eq!(details.video, Some(true));

        let companies: Vec<_> = details
            .production_companies
            .iter()
            .map(|c| {
                (
                    c.id,
                    c.name.as_str(),
                    c.logo_url.as_deref(),
                    c.origin_country.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            companies,
            [
                (
                    Some(79),
                    "Village Roadshow",
                    Some("https://image.tmdb.org/t/p/w92/vr.png"),
                    Some("US")
                ),
                // A negative id is not a valid TMDB id.
                (None, "", None, None),
            ]
        );
        assert_eq!(details.production_countries, ["United States of America"]);
        assert_eq!(details.spoken_languages, ["English"]);

        let collection = details
            .belongs_to_collection
            .as_ref()
            .expect("collection parsed");
        assert_eq!(collection.id, Some(2344));
        assert_eq!(collection.name, "The Matrix Collection");
        assert_eq!(
            collection.poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/coll.jpg")
        );
        assert_eq!(collection.backdrop_url, None);
    }

    #[test]
    fn movie_details_without_collection_has_none() {
        let raw: types::MovieDetailsResponse =
            serde_json::from_value(serde_json::json!({ "id": 1, "belongs_to_collection": null }))
                .expect("details deserialize");
        let details: UnifiedMovieDetails = raw.into();
        assert!(details.belongs_to_collection.is_none());
        assert!(details.production_companies.is_empty());
        assert_eq!(details.movie.title, "");
    }

    #[test]
    fn search_movie_no_poster() {
        let mut raw = make_search_movie();
        raw.poster_path = None;
        raw.backdrop_path = None;
        let unified: UnifiedMovie = raw.into();
        assert!(unified.poster_url.is_none());
        assert!(unified.backdrop_url.is_none());
    }

    #[test]
    fn search_movie_empty_genre_ids() {
        let mut raw = make_search_movie();
        raw.genre_ids = Some(vec![]);
        let unified: UnifiedMovie = raw.into();
        assert!(unified.genres.is_empty());
    }

    #[test]
    fn search_movie_unknown_genre_id() {
        let mut raw = make_search_movie();
        raw.genre_ids = Some(vec![99999]);
        let unified: UnifiedMovie = raw.into();
        assert_eq!(
            unified.genres,
            vec![Genre::Other(crate::unified::genre::UnknownGenre::TmdbId(
                99999
            ),)]
        );
    }
}
