use super::{Genre, UnknownGenre};

#[test]
fn from_anilist_genre_common() {
    assert_eq!(Genre::from_anilist_genre("Action"), Genre::Action);
    assert_eq!(Genre::from_anilist_genre("adventure"), Genre::Adventure);
    assert_eq!(Genre::from_anilist_genre("Comedy"), Genre::Comedy);
    assert_eq!(Genre::from_anilist_genre("Drama"), Genre::Drama);
    assert_eq!(Genre::from_anilist_genre("Fantasy"), Genre::Fantasy);
    assert_eq!(Genre::from_anilist_genre("Horror"), Genre::Horror);
    assert_eq!(Genre::from_anilist_genre("Music"), Genre::Music);
    assert_eq!(Genre::from_anilist_genre("Mystery"), Genre::Mystery);
    assert_eq!(Genre::from_anilist_genre("Romance"), Genre::Romance);
    assert_eq!(Genre::from_anilist_genre("Sci-Fi"), Genre::ScienceFiction);
    assert_eq!(Genre::from_anilist_genre("Thriller"), Genre::Thriller);
}

#[test]
fn from_anilist_genre_anime_specific() {
    assert_eq!(Genre::from_anilist_genre("Mecha"), Genre::Mecha);
    assert_eq!(
        Genre::from_anilist_genre("Mahou Shoujo"),
        Genre::MahouShoujo
    );
    assert_eq!(
        Genre::from_anilist_genre("Slice of Life"),
        Genre::SliceOfLife
    );
    assert_eq!(Genre::from_anilist_genre("Sports"), Genre::Sports);
    assert_eq!(
        Genre::from_anilist_genre("Supernatural"),
        Genre::Supernatural
    );
    assert_eq!(Genre::from_anilist_genre("Ecchi"), Genre::Ecchi);
}

#[test]
fn from_anilist_genre_fallback() {
    assert_eq!(
        Genre::from_anilist_genre("Isekai"),
        Genre::Other(UnknownGenre::Named("isekai".to_string()))
    );
    assert_eq!(
        Genre::from_anilist_genre("Harem"),
        Genre::Other(UnknownGenre::Named("harem".to_string()))
    );
}

#[test]
fn from_tmdb_id_unknown_preserves_id() {
    assert_eq!(
        Genre::from_tmdb_id(99999),
        Genre::Other(UnknownGenre::TmdbId(99999))
    );
    assert_eq!(
        Genre::from_tmdb_id(0),
        Genre::Other(UnknownGenre::TmdbId(0))
    );
    // Negative ids are preserved verbatim, not collapsed to 0.
    assert_eq!(
        Genre::from_tmdb_id(-5),
        Genre::Other(UnknownGenre::TmdbId(-5))
    );
    assert_ne!(Genre::from_tmdb_id(-5), Genre::from_tmdb_id(0));
    // The id survives in the display name rather than becoming "Unknown".
    assert_eq!(Genre::from_tmdb_id(4242).name(), "TMDB genre 4242");
}

#[test]
fn tmdb_id_round_trips_every_known_tmdb_id() {
    let ids: [i64; 27] = [
        28, 12, 14, 36, 27, 10402, 10749, 878, 53, 10770, 10752, 10759, 10762, 10763, 10764, 10765,
        10766, 10767, 10768, 16, 35, 80, 99, 18, 10751, 9648, 37,
    ];
    for id in ids {
        let genre = Genre::from_tmdb_id(id);
        assert!(
            !matches!(genre, Genre::Other(_)),
            "{id} maps to a known variant"
        );
        assert_eq!(genre.tmdb_id(), Some(id as u64), "round trip of {id}");
    }
}

#[test]
fn tmdb_id_unknown_ids_mirror_from_tmdb_id() {
    assert_eq!(Genre::from_tmdb_id(99999).tmdb_id(), Some(99999));
    assert_eq!(Genre::from_tmdb_id(0).tmdb_id(), Some(0));
    // A negative id is not a valid TMDB id.
    assert_eq!(Genre::from_tmdb_id(-5).tmdb_id(), None);
}

#[test]
fn tmdb_id_is_none_without_a_tmdb_genre() {
    for genre in [
        Genre::Mecha,
        Genre::MahouShoujo,
        Genre::SliceOfLife,
        Genre::Sports,
        Genre::Supernatural,
        Genre::Ecchi,
        Genre::Other(UnknownGenre::Named("isekai".to_string())),
    ] {
        assert_eq!(genre.tmdb_id(), None, "{genre:?}");
    }
}

#[test]
fn name_anime_specific() {
    assert_eq!(Genre::Mecha.name(), "Mecha");
    assert_eq!(Genre::MahouShoujo.name(), "Mahou Shoujo");
    assert_eq!(Genre::SliceOfLife.name(), "Slice of Life");
    assert_eq!(Genre::Sports.name(), "Sports");
    assert_eq!(Genre::Supernatural.name(), "Supernatural");
    assert_eq!(Genre::Ecchi.name(), "Ecchi");
}
