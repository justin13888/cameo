//! Every unified-model `with_*` setter writes exactly its own field, and every
//! `new()` starts from the documented defaults.
//!
//! Each setter check builds the expected value by assigning the one field on a
//! clone of the base and compares whole structs, so a setter that also touched
//! another field (or wrote the wrong one) fails.

use cameo::{
    AlternativeTitle, Collection, Company, Creator, Gender, Genre, MediaId, Network, PartialDate,
    SeriesStatus, SeriesType, TitleKind, UnifiedCastCredit, UnifiedCredits, UnifiedCrewCredit,
    UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson, UnifiedPersonDetails,
    UnifiedSearchResult, UnifiedSeason, UnifiedSeasonDetails, UnifiedStreamingService,
    UnifiedTvShow, UnifiedTvShowDetails, UnifiedWatchProviderEntry, UnifiedWatchProviders,
};

/// `base.setter(args)` equals `base` with only `field` replaced by `value`.
macro_rules! assert_sets {
    ($base:expr, $setter:ident($($arg:expr),*) => $field:ident = $value:expr) => {{
        let base = $base;
        let mut expected = base.clone();
        expected.$field = $value;
        assert_eq!(base.$setter($($arg),*), expected, stringify!($setter));
    }};
}

fn date() -> PartialDate {
    PartialDate::ymd(1999, 10, 15).expect("valid date")
}

fn movie() -> UnifiedMovie {
    UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
}

fn show() -> UnifiedTvShow {
    UnifiedTvShow::new(MediaId::tmdb(1396), "Breaking Bad")
}

fn person() -> UnifiedPerson {
    UnifiedPerson::new(MediaId::tmdb(287), "Brad Pitt")
}

fn s(v: &str) -> String {
    v.to_string()
}

#[test]
fn unified_movie_defaults_and_setters() {
    let m = movie();
    assert_eq!(m.provider_id, MediaId::tmdb(550));
    assert_eq!(m.title, "Fight Club");
    assert_eq!(m.original_title, None);
    assert!(m.alternative_titles.is_empty());
    assert_eq!(m.overview, None);
    assert_eq!(m.release_date, None);
    assert_eq!(m.poster_url, None);
    assert_eq!(m.backdrop_url, None);
    assert!(m.genres.is_empty());
    assert_eq!(m.popularity, None);
    assert_eq!(m.vote_average, None);
    assert_eq!(m.vote_count, None);
    assert_eq!(m.original_language, None);
    assert_eq!(m.adult, None);
    assert_eq!(m.runtime, None);
    assert_eq!(m.status, None);

    let alt = vec![AlternativeTitle::new("FC", TitleKind::Synonym)];
    assert_sets!(movie(), with_original_title("O") => original_title = Some(s("O")));
    assert_sets!(movie(), with_alternative_titles(alt.clone()) => alternative_titles = alt.clone());
    assert_sets!(movie(), with_overview("ov") => overview = Some(s("ov")));
    assert_sets!(movie(), with_release_date(date()) => release_date = Some(date()));
    assert_sets!(movie(), with_poster_url("p") => poster_url = Some(s("p")));
    assert_sets!(movie(), with_backdrop_url("b") => backdrop_url = Some(s("b")));
    assert_sets!(movie(), with_genres(vec![Genre::Drama]) => genres = vec![Genre::Drama]);
    assert_sets!(movie(), with_popularity(1.5) => popularity = Some(1.5));
    assert_sets!(movie(), with_vote_average(8.4) => vote_average = Some(8.4));
    assert_sets!(movie(), with_vote_count(42) => vote_count = Some(42));
    assert_sets!(movie(), with_original_language("en") => original_language = Some(s("en")));
    assert_sets!(movie(), with_adult(true) => adult = Some(true));
    assert_sets!(movie(), with_runtime(139) => runtime = Some(139));
    assert_sets!(movie(), with_status("Released") => status = Some(s("Released")));
}

#[test]
fn unified_movie_details_defaults_and_setters() {
    let d = UnifiedMovieDetails::new(movie());
    assert_eq!(d.movie, movie());
    assert_eq!(d.tagline, None);
    assert_eq!(d.runtime, None);
    assert_eq!(d.budget, None);
    assert_eq!(d.revenue, None);
    assert_eq!(d.status, None);
    assert_eq!(d.homepage, None);
    assert_eq!(d.imdb_id, None);
    assert!(d.production_companies.is_empty());
    assert!(d.production_countries.is_empty());
    assert!(d.spoken_languages.is_empty());
    assert_eq!(d.video, None);
    assert_eq!(d.belongs_to_collection, None);

    let base = || UnifiedMovieDetails::new(movie());
    let companies = vec![Company::new("Fox")];
    let collection = Collection::new("Saga");
    assert_sets!(base(), with_tagline("t") => tagline = Some(s("t")));
    // The details-level runtime/status are their own fields, not the movie's.
    assert_sets!(base(), with_runtime(139) => runtime = Some(139));
    assert_sets!(base(), with_status("Released") => status = Some(s("Released")));
    assert_sets!(base(), with_budget(1) => budget = Some(1));
    assert_sets!(base(), with_revenue(2) => revenue = Some(2));
    assert_sets!(base(), with_homepage("h") => homepage = Some(s("h")));
    assert_sets!(base(), with_imdb_id("tt1") => imdb_id = Some(s("tt1")));
    assert_sets!(base(), with_production_companies(companies.clone()) => production_companies = companies.clone());
    assert_sets!(base(), with_production_countries(vec![s("US")]) => production_countries = vec![s("US")]);
    assert_sets!(base(), with_spoken_languages(vec![s("en")]) => spoken_languages = vec![s("en")]);
    assert_sets!(base(), with_video(true) => video = Some(true));
    assert_sets!(base(), with_collection(collection.clone()) => belongs_to_collection = Some(collection.clone()));
}

#[test]
fn unified_tv_show_defaults_and_setters() {
    let t = show();
    assert_eq!(t.provider_id, MediaId::tmdb(1396));
    assert_eq!(t.name, "Breaking Bad");
    assert_eq!(t.original_name, None);
    assert!(t.alternative_titles.is_empty());
    assert_eq!(t.overview, None);
    assert_eq!(t.first_air_date, None);
    assert_eq!(t.poster_url, None);
    assert_eq!(t.backdrop_url, None);
    assert!(t.genres.is_empty());
    assert_eq!(t.popularity, None);
    assert_eq!(t.vote_average, None);
    assert_eq!(t.vote_count, None);
    assert_eq!(t.original_language, None);
    assert!(t.origin_country.is_empty());
    assert_eq!(t.adult, None);
    assert_eq!(t.number_of_episodes, None);
    assert_eq!(t.episode_run_time, None);
    assert_eq!(t.status, None);

    let alt = vec![AlternativeTitle::new("BrBa", TitleKind::Synonym)];
    assert_sets!(show(), with_original_name("O") => original_name = Some(s("O")));
    assert_sets!(show(), with_alternative_titles(alt.clone()) => alternative_titles = alt.clone());
    assert_sets!(show(), with_overview("ov") => overview = Some(s("ov")));
    assert_sets!(show(), with_first_air_date(date()) => first_air_date = Some(date()));
    assert_sets!(show(), with_poster_url("p") => poster_url = Some(s("p")));
    assert_sets!(show(), with_backdrop_url("b") => backdrop_url = Some(s("b")));
    assert_sets!(show(), with_genres(vec![Genre::Crime]) => genres = vec![Genre::Crime]);
    assert_sets!(show(), with_popularity(1.5) => popularity = Some(1.5));
    assert_sets!(show(), with_vote_average(8.9) => vote_average = Some(8.9));
    assert_sets!(show(), with_vote_count(42) => vote_count = Some(42));
    assert_sets!(show(), with_original_language("en") => original_language = Some(s("en")));
    assert_sets!(show(), with_origin_country(vec![s("US")]) => origin_country = vec![s("US")]);
    assert_sets!(show(), with_adult(false) => adult = Some(false));
    assert_sets!(show(), with_number_of_episodes(62) => number_of_episodes = Some(62));
    assert_sets!(show(), with_episode_run_time(47) => episode_run_time = Some(47));
    assert_sets!(show(), with_status(SeriesStatus::Ended) => status = Some(SeriesStatus::Ended));
}

#[test]
fn unified_tv_show_details_defaults_and_setters() {
    let d = UnifiedTvShowDetails::new(show());
    assert_eq!(d.show, show());
    assert_eq!(d.tagline, None);
    assert_eq!(d.number_of_seasons, None);
    assert_eq!(d.number_of_episodes, None);
    assert_eq!(d.in_production, None);
    assert_eq!(d.status, None);
    assert_eq!(d.homepage, None);
    assert!(d.networks.is_empty());
    assert!(d.production_companies.is_empty());
    assert_eq!(d.last_air_date, None);
    assert_eq!(d.type_, None);
    assert_eq!(d.media_format, None);
    assert!(d.created_by.is_empty());
    assert!(d.episode_run_time.is_empty());
    assert!(d.spoken_languages.is_empty());
    assert!(d.production_countries.is_empty());
    assert!(d.seasons.is_empty());

    let base = || UnifiedTvShowDetails::new(show());
    let networks = vec![Network::new("AMC")];
    let companies = vec![Company::new("Sony")];
    let creators = vec![Creator::new("Vince Gilligan")];
    let seasons = vec![UnifiedSeason::new(1)];
    assert_sets!(base(), with_tagline("t") => tagline = Some(s("t")));
    assert_sets!(base(), with_number_of_seasons(5) => number_of_seasons = Some(5));
    assert_sets!(base(), with_number_of_episodes(62) => number_of_episodes = Some(62));
    assert_sets!(base(), with_in_production(true) => in_production = Some(true));
    assert_sets!(base(), with_status(SeriesStatus::Hiatus) => status = Some(SeriesStatus::Hiatus));
    assert_sets!(base(), with_homepage("h") => homepage = Some(s("h")));
    assert_sets!(base(), with_networks(networks.clone()) => networks = networks.clone());
    assert_sets!(base(), with_production_companies(companies.clone()) => production_companies = companies.clone());
    assert_sets!(base(), with_last_air_date(date()) => last_air_date = Some(date()));
    assert_sets!(base(), with_type(SeriesType::Miniseries) => type_ = Some(SeriesType::Miniseries));
    assert_sets!(base(), with_media_format("TV") => media_format = Some(s("TV")));
    assert_sets!(base(), with_created_by(creators.clone()) => created_by = creators.clone());
    assert_sets!(base(), with_episode_run_time(vec![47, 50]) => episode_run_time = vec![47, 50]);
    assert_sets!(base(), with_spoken_languages(vec![s("en")]) => spoken_languages = vec![s("en")]);
    assert_sets!(base(), with_production_countries(vec![s("US")]) => production_countries = vec![s("US")]);
    assert_sets!(base(), with_seasons(seasons.clone()) => seasons = seasons.clone());
}

#[test]
fn unified_season_defaults_and_setters() {
    let season = UnifiedSeason::new(3);
    assert_eq!(season.season_number, 3);
    assert_eq!(season.name, None);
    assert_eq!(season.overview, None);
    assert_eq!(season.air_date, None);
    assert_eq!(season.episode_count, None);
    assert_eq!(season.poster_url, None);

    let base = || UnifiedSeason::new(3);
    assert_sets!(base(), with_name("n") => name = Some(s("n")));
    assert_sets!(base(), with_overview("o") => overview = Some(s("o")));
    assert_sets!(base(), with_air_date(date()) => air_date = Some(date()));
    assert_sets!(base(), with_episode_count(7) => episode_count = Some(7));
    assert_sets!(base(), with_poster_url("p") => poster_url = Some(s("p")));
}

#[test]
fn unified_person_defaults_and_setters() {
    let p = person();
    assert_eq!(p.provider_id, MediaId::tmdb(287));
    assert_eq!(p.name, "Brad Pitt");
    assert_eq!(p.known_for_department, None);
    assert_eq!(p.profile_url, None);
    assert_eq!(p.popularity, None);
    assert_eq!(p.gender, None);
    assert_eq!(p.adult, None);

    assert_sets!(person(), with_known_for_department("Acting") => known_for_department = Some(s("Acting")));
    assert_sets!(person(), with_profile_url("p") => profile_url = Some(s("p")));
    assert_sets!(person(), with_popularity(2.5) => popularity = Some(2.5));
    assert_sets!(person(), with_gender(Gender::Female) => gender = Some(Gender::Female));
    assert_sets!(person(), with_adult(false) => adult = Some(false));
}

#[test]
fn unified_person_details_defaults_and_setters() {
    let d = UnifiedPersonDetails::new(person());
    assert_eq!(d.person, person());
    assert_eq!(d.biography, None);
    assert_eq!(d.birthday, None);
    assert_eq!(d.deathday, None);
    assert_eq!(d.place_of_birth, None);
    assert_eq!(d.imdb_id, None);
    assert_eq!(d.homepage, None);
    assert!(d.also_known_as.is_empty());

    let base = || UnifiedPersonDetails::new(person());
    let born = PartialDate::from_year(1963);
    assert_sets!(base(), with_biography("b") => biography = Some(s("b")));
    assert_sets!(base(), with_birthday(born) => birthday = Some(born));
    assert_sets!(base(), with_deathday(date()) => deathday = Some(date()));
    assert_sets!(base(), with_place_of_birth("Shawnee") => place_of_birth = Some(s("Shawnee")));
    assert_sets!(base(), with_imdb_id("nm1") => imdb_id = Some(s("nm1")));
    assert_sets!(base(), with_homepage("h") => homepage = Some(s("h")));
    assert_sets!(base(), with_also_known_as(vec![s("B")]) => also_known_as = vec![s("B")]);
}

#[test]
fn credits_defaults_and_setters() {
    let credits = UnifiedCredits::new(MediaId::tmdb(550));
    assert_eq!(credits.provider_id, MediaId::tmdb(550));
    assert!(credits.cast.is_empty());
    assert!(credits.crew.is_empty());

    let cast_member = || UnifiedCastCredit::new(MediaId::tmdb(819), "Edward Norton");
    let crew_member = || UnifiedCrewCredit::new(MediaId::tmdb(7467), "David Fincher");
    let cast = vec![cast_member()];
    let crew = vec![crew_member()];
    let base = || UnifiedCredits::new(MediaId::tmdb(550));
    assert_sets!(base(), with_cast(cast.clone()) => cast = cast.clone());
    assert_sets!(base(), with_crew(crew.clone()) => crew = crew.clone());

    let c = cast_member();
    assert_eq!(
        (c.person_id, c.name.as_str()),
        (MediaId::tmdb(819), "Edward Norton")
    );
    assert_sets!(cast_member(), with_profile_url("p") => profile_url = Some(s("p")));
    assert_sets!(cast_member(), with_character("Narrator") => character = Some(s("Narrator")));
    assert_sets!(cast_member(), with_order(0) => order = Some(0));
    assert_sets!(cast_member(), with_gender(Gender::Male) => gender = Some(Gender::Male));

    let c = crew_member();
    assert_eq!(
        (c.person_id, c.name.as_str()),
        (MediaId::tmdb(7467), "David Fincher")
    );
    assert_eq!(c.profile_url, None);
    assert_eq!(c.gender, None);
    assert_sets!(crew_member(), with_profile_url("p") => profile_url = Some(s("p")));
    assert_sets!(crew_member(), with_job("Director") => job = Some(s("Director")));
    assert_sets!(crew_member(), with_department("Directing") => department = Some(s("Directing")));
    assert_sets!(crew_member(), with_gender(Gender::NonBinary) => gender = Some(Gender::NonBinary));
}

#[test]
fn season_details_and_episode_defaults_and_setters() {
    let d = UnifiedSeasonDetails::new(MediaId::tmdb(1396), 2);
    assert_eq!(
        (d.show_id.clone(), d.season_number),
        (MediaId::tmdb(1396), 2)
    );
    assert_eq!(d.name, None);
    assert_eq!(d.overview, None);
    assert_eq!(d.air_date, None);
    assert_eq!(d.poster_url, None);
    assert!(d.episodes.is_empty());

    let base = || UnifiedSeasonDetails::new(MediaId::tmdb(1396), 2);
    let episodes = vec![UnifiedEpisode::new(1)];
    assert_sets!(base(), with_name("n") => name = Some(s("n")));
    assert_sets!(base(), with_overview("o") => overview = Some(s("o")));
    assert_sets!(base(), with_air_date(date()) => air_date = Some(date()));
    assert_sets!(base(), with_poster_url("p") => poster_url = Some(s("p")));
    assert_sets!(base(), with_episodes(episodes.clone()) => episodes = episodes.clone());

    let e = UnifiedEpisode::new(4);
    assert_eq!(e.episode_number, 4);
    assert_eq!(e.name, None);
    assert_eq!(e.overview, None);
    assert_eq!(e.air_date, None);
    assert_eq!(e.runtime, None);
    assert_eq!(e.still_url, None);
    assert_eq!(e.vote_average, None);

    let base = || UnifiedEpisode::new(4);
    assert_sets!(base(), with_name("n") => name = Some(s("n")));
    assert_sets!(base(), with_overview("o") => overview = Some(s("o")));
    assert_sets!(base(), with_air_date(date()) => air_date = Some(date()));
    assert_sets!(base(), with_runtime(58) => runtime = Some(58));
    assert_sets!(base(), with_still_url("st") => still_url = Some(s("st")));
    assert_sets!(base(), with_vote_average(8.2) => vote_average = Some(8.2));
}

#[test]
fn watch_provider_defaults_and_setters() {
    let w = UnifiedWatchProviders::new(MediaId::tmdb(550));
    assert_eq!(w.provider_id, MediaId::tmdb(550));
    assert!(w.results.is_empty());

    let entry = UnifiedWatchProviderEntry::new();
    assert_eq!(entry, UnifiedWatchProviderEntry::default());
    assert!(entry.flatrate.is_empty() && entry.rent.is_empty() && entry.buy.is_empty());

    let services = vec![UnifiedStreamingService::new("Netflix")];
    let base = UnifiedWatchProviderEntry::new;
    assert_sets!(base(), with_flatrate(services.clone()) => flatrate = services.clone());
    assert_sets!(base(), with_rent(services.clone()) => rent = services.clone());
    assert_sets!(base(), with_buy(services.clone()) => buy = services.clone());

    let svc = UnifiedStreamingService::new("Netflix");
    assert_eq!(
        (svc.name.as_str(), svc.logo_url.as_deref()),
        ("Netflix", None)
    );
    assert_sets!(UnifiedStreamingService::new("Netflix"), with_logo_url("l") => logo_url = Some(s("l")));
}

/// `with_country` inserts per country, and a repeated country code keeps the
/// last entry written.
#[test]
fn with_country_is_last_write_wins() {
    let first =
        UnifiedWatchProviderEntry::new().with_flatrate(vec![UnifiedStreamingService::new("A")]);
    let second = UnifiedWatchProviderEntry::new().with_buy(vec![UnifiedStreamingService::new("B")]);
    let providers = UnifiedWatchProviders::new(MediaId::tmdb(550))
        .with_country("US", first.clone())
        .with_country("GB", first.clone())
        .with_country("US", second.clone());
    assert_eq!(providers.results.len(), 2);
    assert_eq!(providers.results["US"], second);
    assert_eq!(providers.results["GB"], first);
}

#[test]
fn small_value_types_defaults_and_setters() {
    let c = Collection::new("Saga");
    assert_eq!(
        (
            c.id,
            c.name.as_str(),
            c.poster_url.clone(),
            c.backdrop_url.clone()
        ),
        (None, "Saga", None, None)
    );
    assert_sets!(Collection::new("Saga"), with_id(10) => id = Some(10));
    assert_sets!(Collection::new("Saga"), with_poster_url("p") => poster_url = Some(s("p")));
    assert_sets!(Collection::new("Saga"), with_backdrop_url("b") => backdrop_url = Some(s("b")));

    let a = AlternativeTitle::new("Title", TitleKind::Native);
    assert_eq!(
        (a.title.as_str(), a.kind, a.language.clone()),
        ("Title", TitleKind::Native, None)
    );
    assert_sets!(AlternativeTitle::new("Title", TitleKind::Native), with_language("ja") => language = Some(s("ja")));

    let co = Company::new("Fox");
    assert_eq!(
        (
            co.id,
            co.name.as_str(),
            co.logo_url.clone(),
            co.origin_country.clone()
        ),
        (None, "Fox", None, None)
    );
    assert_sets!(Company::new("Fox"), with_id(25) => id = Some(25));
    assert_sets!(Company::new("Fox"), with_logo_url("l") => logo_url = Some(s("l")));
    assert_sets!(Company::new("Fox"), with_origin_country("US") => origin_country = Some(s("US")));

    let n = Network::new("AMC");
    assert_eq!(
        (
            n.id,
            n.name.as_str(),
            n.logo_url.clone(),
            n.origin_country.clone()
        ),
        (None, "AMC", None, None)
    );
    assert_sets!(Network::new("AMC"), with_id(174) => id = Some(174));
    assert_sets!(Network::new("AMC"), with_logo_url("l") => logo_url = Some(s("l")));
    assert_sets!(Network::new("AMC"), with_origin_country("US") => origin_country = Some(s("US")));

    let cr = Creator::new("Vince");
    assert_eq!(
        (cr.id, cr.name.as_str(), cr.profile_url.clone()),
        (None, "Vince", None)
    );
    assert_sets!(Creator::new("Vince"), with_id(66633) => id = Some(66633));
    assert_sets!(Creator::new("Vince"), with_profile_url("p") => profile_url = Some(s("p")));
}

/// Each `UnifiedSearchResult` constructor wraps its argument unchanged in the
/// matching variant.
#[test]
fn search_result_constructors_wrap_unchanged() {
    let m = movie().with_overview("x");
    let t = show().with_overview("y");
    let p = person().with_popularity(1.0);
    assert_eq!(
        UnifiedSearchResult::movie(m.clone()),
        UnifiedSearchResult::Movie(m)
    );
    assert_eq!(
        UnifiedSearchResult::tv_show(t.clone()),
        UnifiedSearchResult::TvShow(t)
    );
    assert_eq!(
        UnifiedSearchResult::person(p.clone()),
        UnifiedSearchResult::Person(p)
    );
}
