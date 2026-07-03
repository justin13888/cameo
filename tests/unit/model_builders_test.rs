//! External-crate-style construction of every unified model.
//!
//! This test uses only the public `cameo::` API — `new()` + `with_*` builders —
//! with **no** struct-literal field access, proving that an out-of-crate
//! provider can construct every `#[non_exhaustive]` unified model it must return
//! from the unified traits (#36).

use cameo::{
    Collection, Company, Creator, Gender, Genre, MediaId, Network, PartialDate, UnifiedEpisode,
    UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails, UnifiedSearchResult,
    UnifiedSeason, UnifiedSeasonDetails, UnifiedStreamingService, UnifiedTvShow,
    UnifiedTvShowDetails, UnifiedWatchProviderEntry, UnifiedWatchProviders,
};

#[test]
fn build_movie_and_details() {
    let movie = UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
        .with_original_title("Fight Club")
        .with_overview("An insomniac...")
        .with_release_date(PartialDate::from_year(1999))
        .with_poster_url("https://example.com/p.jpg")
        .with_backdrop_url("https://example.com/b.jpg")
        .with_genres(vec![Genre::Drama, Genre::Thriller])
        .with_popularity(73.4)
        .with_vote_average(8.4)
        .with_vote_count(26_000)
        .with_original_language("en")
        .with_adult(false);
    assert_eq!(movie.title, "Fight Club");
    assert_eq!(movie.release_date, PartialDate::from_year(1999).into());

    let details = UnifiedMovieDetails::new(movie)
        .with_tagline("Mischief. Mayhem. Soap.")
        .with_runtime(139)
        .with_budget(63_000_000)
        .with_revenue(100_853_753)
        .with_status("Released")
        .with_homepage("https://example.com")
        .with_imdb_id("tt0137523")
        .with_production_companies(vec![
            Company::new("Fox").with_id(25).with_origin_country("US"),
        ])
        .with_production_countries(vec!["United States".to_string()])
        .with_spoken_languages(vec!["English".to_string()])
        .with_video(false)
        .with_collection(
            Collection::new("Some Collection")
                .with_id(1)
                .with_poster_url("x"),
        );
    assert_eq!(details.runtime, Some(139));
    assert_eq!(details.production_companies[0].name, "Fox");
}

#[test]
fn build_tv_show_and_details() {
    let show = UnifiedTvShow::new(MediaId::tmdb(1396), "Breaking Bad")
        .with_original_name("Breaking Bad")
        .with_overview("A chemistry teacher...")
        .with_first_air_date(PartialDate::ymd(2008, 1, 20).unwrap())
        .with_poster_url("x")
        .with_backdrop_url("y")
        .with_genres(vec![Genre::Drama, Genre::Crime])
        .with_popularity(1.0)
        .with_vote_average(8.9)
        .with_vote_count(14_000)
        .with_original_language("en")
        .with_origin_country(vec!["US".to_string()])
        .with_adult(false);
    assert_eq!(show.name, "Breaking Bad");

    let details = UnifiedTvShowDetails::new(show)
        .with_tagline("All Hail the King")
        .with_number_of_seasons(5)
        .with_number_of_episodes(62)
        .with_in_production(false)
        .with_status("Ended")
        .with_homepage("https://example.com")
        .with_networks(vec![
            Network::new("AMC").with_id(174).with_origin_country("US"),
        ])
        .with_production_companies(vec![Company::new("Sony")])
        .with_last_air_date(PartialDate::ymd(2013, 9, 29).unwrap())
        .with_type("Scripted")
        .with_media_format("TV")
        .with_created_by(vec![Creator::new("Vince Gilligan").with_id(66633)])
        .with_episode_run_time(vec![47])
        .with_spoken_languages(vec!["English".to_string()])
        .with_production_countries(vec!["United States".to_string()])
        .with_seasons(vec![
            UnifiedSeason::new(1)
                .with_name("Season 1")
                .with_overview("...")
                .with_air_date(PartialDate::from_year(2008))
                .with_episode_count(7)
                .with_poster_url("s1.jpg"),
        ]);
    assert_eq!(details.number_of_seasons, Some(5));
    assert_eq!(details.networks[0].name, "AMC");
    assert_eq!(details.seasons[0].season_number, 1);
}

#[test]
fn build_person_and_details() {
    let person = UnifiedPerson::new(MediaId::tmdb(287), "Brad Pitt")
        .with_known_for_department("Acting")
        .with_profile_url("x")
        .with_popularity(10.0)
        .with_gender(Gender::Male)
        .with_adult(false);
    assert_eq!(person.gender, Some(Gender::Male));

    let details = UnifiedPersonDetails::new(person)
        .with_biography("An American actor...")
        .with_birthday(PartialDate::ymd(1963, 12, 18).unwrap())
        .with_deathday(PartialDate::from_year(2100))
        .with_place_of_birth("Shawnee, Oklahoma")
        .with_imdb_id("nm0000093")
        .with_homepage("https://example.com")
        .with_also_known_as(vec!["William Bradley Pitt".to_string()]);
    assert_eq!(details.birthday, PartialDate::ymd(1963, 12, 18));
}

#[test]
fn build_season_and_episode() {
    let episode = UnifiedEpisode::new(1)
        .with_name("Pilot")
        .with_overview("...")
        .with_air_date(PartialDate::ymd(2008, 1, 20).unwrap())
        .with_runtime(58)
        .with_still_url("still.jpg")
        .with_vote_average(8.2);
    assert_eq!(episode.episode_number, 1);

    let season = UnifiedSeasonDetails::new(MediaId::tmdb(1396), 1)
        .with_name("Season 1")
        .with_overview("...")
        .with_air_date(PartialDate::from_year(2008))
        .with_poster_url("s1.jpg")
        .with_episodes(vec![episode]);
    assert_eq!(season.episodes.len(), 1);
    assert_eq!(season.show_id, MediaId::tmdb(1396));
}

#[test]
fn build_watch_providers_and_search_results() {
    let netflix = UnifiedStreamingService::new("Netflix").with_logo_url("n.jpg");
    let entry = UnifiedWatchProviderEntry::new()
        .with_flatrate(vec![netflix])
        .with_rent(Vec::new())
        .with_buy(Vec::new());
    let providers = UnifiedWatchProviders::new(MediaId::tmdb(550)).with_country("US", entry);
    assert_eq!(providers.results["US"].flatrate[0].name, "Netflix");

    // Every UnifiedSearchResult variant is constructible via the public API.
    let m = UnifiedSearchResult::movie(UnifiedMovie::new(MediaId::tmdb(1), "M"));
    let t = UnifiedSearchResult::tv_show(UnifiedTvShow::new(MediaId::tmdb(2), "T"));
    let p = UnifiedSearchResult::person(UnifiedPerson::new(MediaId::tmdb(3), "P"));
    assert!(matches!(m, UnifiedSearchResult::Movie(_)));
    assert!(matches!(t, UnifiedSearchResult::TvShow(_)));
    assert!(matches!(p, UnifiedSearchResult::Person(_)));
}
