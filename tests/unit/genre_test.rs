use cameo::unified::genre::{Genre, UnknownGenre};

// ── from_tmdb_id ──

#[test]
fn known_tmdb_ids_round_trip() {
    let cases: &[(i64, Genre)] = &[
        (28, Genre::Action),
        (12, Genre::Adventure),
        (16, Genre::Animation),
        (35, Genre::Comedy),
        (80, Genre::Crime),
        (99, Genre::Documentary),
        (18, Genre::Drama),
        (10751, Genre::Family),
        (14, Genre::Fantasy),
        (36, Genre::History),
        (27, Genre::Horror),
        (10402, Genre::Music),
        (9648, Genre::Mystery),
        (10749, Genre::Romance),
        (878, Genre::ScienceFiction),
        (53, Genre::Thriller),
        (10770, Genre::TvMovie),
        (10752, Genre::War),
        (37, Genre::Western),
        (10759, Genre::ActionAdventure),
        (10762, Genre::Kids),
        (10763, Genre::News),
        (10764, Genre::Reality),
        (10765, Genre::SciFiFantasy),
        (10766, Genre::Soap),
        (10767, Genre::Talk),
        (10768, Genre::WarPolitics),
    ];
    for (id, expected) in cases {
        assert_eq!(Genre::from_tmdb_id(*id), *expected, "id={id}");
    }
}

#[test]
fn unknown_tmdb_id_becomes_other() {
    let g = Genre::from_tmdb_id(99999);
    assert_eq!(g, Genre::Other(UnknownGenre::TmdbId(99999)));
}

// ── from_name ──

#[test]
fn known_names_round_trip() {
    let cases: &[(&str, Genre)] = &[
        ("Action", Genre::Action),
        ("Adventure", Genre::Adventure),
        ("Animation", Genre::Animation),
        ("Comedy", Genre::Comedy),
        ("Crime", Genre::Crime),
        ("Documentary", Genre::Documentary),
        ("Drama", Genre::Drama),
        ("Family", Genre::Family),
        ("Fantasy", Genre::Fantasy),
        ("History", Genre::History),
        ("Horror", Genre::Horror),
        ("Music", Genre::Music),
        ("Mystery", Genre::Mystery),
        ("Romance", Genre::Romance),
        ("Science Fiction", Genre::ScienceFiction),
        ("Thriller", Genre::Thriller),
        ("TV Movie", Genre::TvMovie),
        ("War", Genre::War),
        ("Western", Genre::Western),
        ("Action & Adventure", Genre::ActionAdventure),
        ("Kids", Genre::Kids),
        ("News", Genre::News),
        ("Reality", Genre::Reality),
        ("Sci-Fi & Fantasy", Genre::SciFiFantasy),
        ("Soap", Genre::Soap),
        ("Talk", Genre::Talk),
        ("War & Politics", Genre::WarPolitics),
    ];
    for (name, expected) in cases {
        assert_eq!(Genre::from_name(name), *expected, "name={name}");
    }
}

#[test]
fn from_name_is_case_insensitive() {
    assert_eq!(Genre::from_name("DRAMA"), Genre::Drama);
    assert_eq!(Genre::from_name("science fiction"), Genre::ScienceFiction);
    assert_eq!(Genre::from_name("Action & Adventure"), Genre::ActionAdventure);
}

#[test]
fn unknown_name_becomes_other() {
    let g = Genre::from_name("Superhero");
    assert_eq!(g, Genre::Other(UnknownGenre::Named("superhero".to_string())));
}

// ── name() / Display ──

#[test]
fn genre_name_display() {
    assert_eq!(Genre::Drama.name(), "Drama");
    assert_eq!(Genre::ScienceFiction.name(), "Science Fiction");
    assert_eq!(Genre::ActionAdventure.name(), "Action & Adventure");
    assert_eq!(Genre::SciFiFantasy.name(), "Sci-Fi & Fantasy");
    assert_eq!(Genre::TvMovie.name(), "TV Movie");
    assert_eq!(Genre::WarPolitics.name(), "War & Politics");
}

#[test]
fn unknown_named_genre_display() {
    let g = Genre::Other(UnknownGenre::Named("Superhero".to_string()));
    assert_eq!(g.name(), "Superhero");
    assert_eq!(g.to_string(), "Superhero");
}

#[test]
fn unknown_tmdb_id_genre_display() {
    let g = Genre::Other(UnknownGenre::TmdbId(99999));
    assert_eq!(g.name(), "Unknown");
    assert_eq!(g.to_string(), "Unknown");
}

#[test]
fn genre_serde_round_trip() {
    let genres = vec![Genre::Drama, Genre::ScienceFiction, Genre::Other(UnknownGenre::TmdbId(12345))];
    let json = serde_json::to_string(&genres).unwrap();
    let parsed: Vec<Genre> = serde_json::from_str(&json).unwrap();
    assert_eq!(genres, parsed);
}
