//! Golden-JSON shape tests for the unified models.
//!
//! `docs/STABILITY.md` treats the serde shape of the unified models as stable,
//! and the SQLite cache persists exactly this shape. Each test serializes a
//! fully populated model, compares it with a committed golden file under
//! `tests/fixtures/golden/`, and deserializes the golden file back to the same
//! model. A shape change must update the golden file (and bump the cache
//! schema version) deliberately.

use cameo::{
    AlternativeTitle, Collection, Company, Creator, Gender, Genre, MediaId, Network, Page,
    PartialDate, SeriesStatus, SeriesType, TitleKind, UnifiedCastCredit, UnifiedCredits,
    UnifiedCrewCredit, UnifiedEpisode, UnifiedMovie, UnifiedMovieDetails, UnifiedPerson,
    UnifiedPersonDetails, UnifiedSearchResult, UnifiedSeason, UnifiedSeasonDetails,
    UnifiedStreamingService, UnifiedTvShow, UnifiedTvShowDetails, UnifiedWatchProviderEntry,
    UnifiedWatchProviders, UnknownGenre,
};
use serde::{Serialize, de::DeserializeOwned};

/// Serialize `model`, compare with `golden`, and deserialize `golden` back.
fn assert_golden<T>(model: &T, golden: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let expected: serde_json::Value = serde_json::from_str(golden).expect("golden is valid JSON");
    let actual = serde_json::to_value(model).expect("model serializes");
    assert_eq!(
        actual,
        expected,
        "serialized shape drifted from the golden file; actual:\n{}",
        serde_json::to_string_pretty(&actual).unwrap_or_default()
    );
    let back: T = serde_json::from_value(expected).expect("golden deserializes");
    assert_eq!(&back, model, "golden file does not round-trip to the model");
}

fn movie() -> UnifiedMovie {
    UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
        .with_original_title("Fight Club")
        .with_alternative_titles(vec![
            AlternativeTitle::new("Fight Club", TitleKind::English).with_language("en"),
            AlternativeTitle::new("FC", TitleKind::Synonym),
        ])
        .with_overview("An insomniac office worker...")
        .with_release_date(PartialDate::ymd(1999, 10, 15).expect("valid date"))
        .with_poster_url("https://image.tmdb.org/t/p/w500/poster.jpg")
        .with_backdrop_url("https://image.tmdb.org/t/p/w1280/backdrop.jpg")
        .with_genres(vec![
            Genre::Drama,
            Genre::Other(UnknownGenre::TmdbId(4242)),
            Genre::Other(UnknownGenre::Named("cult".to_string())),
        ])
        .with_popularity(73.25)
        .with_vote_average(8.5)
        .with_vote_count(26_280)
        .with_original_language("en")
        .with_adult(false)
        .with_runtime(139)
        .with_status("Released")
}

fn show() -> UnifiedTvShow {
    UnifiedTvShow::new(MediaId::anilist(5114), "Fullmetal Alchemist: Brotherhood")
        .with_original_name("Hagane no Renkinjutsushi")
        .with_alternative_titles(vec![AlternativeTitle::new(
            "鋼の錬金術師",
            TitleKind::Native,
        )])
        .with_overview("Two brothers...")
        .with_first_air_date(PartialDate::year_month(2009, 4).expect("valid date"))
        .with_poster_url("https://example.com/cover.jpg")
        .with_backdrop_url("https://example.com/banner.jpg")
        .with_genres(vec![Genre::Action, Genre::Mecha])
        .with_popularity(1500.5)
        .with_vote_average(9.0)
        .with_vote_count(250_000)
        .with_original_language("ja")
        .with_origin_country(vec!["JP".to_string()])
        .with_adult(false)
        .with_number_of_episodes(64)
        .with_episode_run_time(24)
        .with_status(SeriesStatus::Ended)
}

fn person() -> UnifiedPerson {
    UnifiedPerson::new(MediaId::anilist_staff(95061), "Romi Park")
        .with_known_for_department("Acting")
        .with_profile_url("https://example.com/romi.jpg")
        .with_popularity(12.5)
        .with_gender(Gender::Female)
        .with_adult(false)
}

#[test]
fn movie_details_shape_is_pinned() {
    let details = UnifiedMovieDetails::new(movie())
        .with_tagline("Mischief. Mayhem. Soap.")
        .with_runtime(139)
        .with_budget(63_000_000)
        .with_revenue(100_853_753)
        .with_status("Released")
        .with_homepage("https://example.com/fightclub")
        .with_imdb_id("tt0137523")
        .with_production_companies(vec![
            Company::new("Fox 2000 Pictures")
                .with_id(711)
                .with_logo_url("https://example.com/fox.png")
                .with_origin_country("US"),
        ])
        .with_production_countries(vec!["United States of America".to_string()])
        .with_spoken_languages(vec!["English".to_string()])
        .with_video(false)
        .with_collection(
            Collection::new("Fight Club Collection")
                .with_id(1)
                .with_poster_url("https://example.com/c-poster.jpg")
                .with_backdrop_url("https://example.com/c-backdrop.jpg"),
        );
    assert_golden(
        &details,
        include_str!("../fixtures/golden/unified_movie_details.json"),
    );
}

#[test]
fn tv_show_details_shape_is_pinned() {
    let details = UnifiedTvShowDetails::new(show())
        .with_tagline("A heart made fullmetal")
        .with_number_of_seasons(1)
        .with_number_of_episodes(64)
        .with_in_production(false)
        .with_status(SeriesStatus::Other("Rumored".to_string()))
        .with_homepage("https://example.com/fmab")
        .with_networks(vec![
            Network::new("MBS")
                .with_id(94)
                .with_logo_url("https://example.com/mbs.png")
                .with_origin_country("JP"),
        ])
        .with_production_companies(vec![Company::new("Bones")])
        .with_last_air_date(PartialDate::from_year(2010))
        .with_type(SeriesType::Scripted)
        .with_media_format("TV")
        .with_created_by(vec![
            Creator::new("Hiromu Arakawa")
                .with_id(7)
                .with_profile_url("https://example.com/arakawa.jpg"),
        ])
        .with_episode_run_time(vec![24, 25])
        .with_spoken_languages(vec!["Japanese".to_string()])
        .with_production_countries(vec!["Japan".to_string()])
        .with_seasons(vec![
            UnifiedSeason::new(1)
                .with_name("Season 1")
                .with_overview("The first season.")
                .with_air_date(PartialDate::ymd(2009, 4, 5).expect("valid date"))
                .with_episode_count(64)
                .with_poster_url("https://example.com/s1.jpg"),
        ]);
    assert_golden(
        &details,
        include_str!("../fixtures/golden/unified_tv_show_details.json"),
    );
}

#[test]
fn person_details_shape_is_pinned() {
    let details = UnifiedPersonDetails::new(person())
        .with_biography("A Japanese voice actress.")
        .with_birthday(PartialDate::ymd(1972, 1, 22).expect("valid date"))
        .with_deathday(PartialDate::from_year(2100))
        .with_place_of_birth("Tokyo")
        .with_imdb_id("nm0662272")
        .with_homepage("https://example.com/romi")
        .with_also_known_as(vec!["朴璐美".to_string()]);
    assert_golden(
        &details,
        include_str!("../fixtures/golden/unified_person_details.json"),
    );
}

#[test]
fn search_page_shape_is_pinned() {
    let mut page = Page::offset(
        2,
        vec![
            UnifiedSearchResult::movie(UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")),
            UnifiedSearchResult::tv_show(UnifiedTvShow::new(MediaId::tmdb(1396), "Breaking Bad")),
            UnifiedSearchResult::person(UnifiedPerson::new(MediaId::tmdb(287), "Brad Pitt")),
        ],
        5,
        93,
    );
    page.next = Some(cameo::PageToken::Page(3));
    assert_golden(
        &page,
        include_str!("../fixtures/golden/search_multi_page.json"),
    );
}

#[test]
fn credits_shape_is_pinned() {
    let credits = UnifiedCredits::new(MediaId::tmdb(550))
        .with_cast(vec![
            UnifiedCastCredit::new(MediaId::tmdb(819), "Edward Norton")
                .with_profile_url("https://example.com/norton.jpg")
                .with_character("The Narrator")
                .with_order(0)
                .with_gender(Gender::Male),
        ])
        .with_crew(vec![
            UnifiedCrewCredit::new(MediaId::tmdb(7467), "David Fincher")
                .with_profile_url("https://example.com/fincher.jpg")
                .with_job("Director")
                .with_department("Directing")
                .with_gender(Gender::NonBinary),
        ]);
    assert_golden(
        &credits,
        include_str!("../fixtures/golden/unified_credits.json"),
    );
}

#[test]
fn season_details_and_watch_providers_shape_is_pinned() {
    let season = UnifiedSeasonDetails::new(MediaId::tmdb(1396), 1)
        .with_name("Season 1")
        .with_overview("High school chemistry teacher...")
        .with_air_date(PartialDate::ymd(2008, 1, 20).expect("valid date"))
        .with_poster_url("https://example.com/s1.jpg")
        .with_episodes(vec![
            UnifiedEpisode::new(1)
                .with_name("Pilot")
                .with_overview("Walter White...")
                .with_air_date(PartialDate::ymd(2008, 1, 20).expect("valid date"))
                .with_runtime(58)
                .with_still_url("https://example.com/still.jpg")
                .with_vote_average(8.25),
        ]);
    assert_golden(
        &season,
        include_str!("../fixtures/golden/unified_season_details.json"),
    );

    let providers = UnifiedWatchProviders::new(MediaId::tmdb(550)).with_country(
        "US",
        UnifiedWatchProviderEntry::new()
            .with_flatrate(vec![
                UnifiedStreamingService::new("Netflix").with_logo_url("https://example.com/n.png"),
            ])
            .with_rent(vec![UnifiedStreamingService::new("Apple TV")])
            .with_buy(Vec::new()),
    );
    assert_golden(
        &providers,
        include_str!("../fixtures/golden/unified_watch_providers.json"),
    );
}
