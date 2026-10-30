//! External-crate-style construction and round-tripping of the external-id
//! types (`ExternalSource`, `ExternalId`, `ExternalIds`, `ExternalMatch`).

use cameo::{ExternalId, ExternalIds, ExternalMatch, ExternalSource, MatchKind, MediaId};

#[test]
fn external_id_constructors_set_source_and_id() {
    assert_eq!(ExternalId::imdb("tt0137523").source, ExternalSource::Imdb);
    assert_eq!(ExternalId::imdb("tt0137523").id, "tt0137523");
    assert_eq!(ExternalId::tvdb(81189).id, "81189");
    assert_eq!(ExternalId::tvdb(81189).as_u64(), Some(81189));
    assert_eq!(ExternalId::mal(5114).source, ExternalSource::Mal);
    assert_eq!(
        ExternalId::wikidata("Q190050").source,
        ExternalSource::Wikidata
    );
    assert_eq!(ExternalId::imdb("tt0137523").as_u64(), None);
    assert_eq!(
        ExternalId::new(ExternalSource::Mal, "5114"),
        ExternalId::mal(5114)
    );
}

#[test]
fn external_id_displays_as_source_colon_id() {
    assert_eq!(ExternalId::imdb("tt0137523").to_string(), "imdb:tt0137523");
    assert_eq!(ExternalId::mal(1).to_string(), "mal:1");
}

#[test]
fn external_source_round_trips_through_str() {
    for source in [
        ExternalSource::Imdb,
        ExternalSource::Tvdb,
        ExternalSource::Mal,
        ExternalSource::Wikidata,
    ] {
        assert_eq!(source.to_string().parse::<ExternalSource>(), Ok(source));
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, format!("\"{}\"", source.as_str()));
    }
    assert_eq!("IMDB".parse::<ExternalSource>(), Ok(ExternalSource::Imdb));
    assert!("netflix".parse::<ExternalSource>().is_err());
}

#[test]
fn external_ids_builders_and_get() {
    let ids = ExternalIds::new()
        .with_imdb("tt0903747")
        .with_tvdb(81189)
        .with_mal(1)
        .with_wikidata("Q1079")
        .with_tmdb(1396)
        .with_anilist(2);
    assert_eq!(ids.imdb.as_deref(), Some("tt0903747"));
    assert_eq!(ids.tmdb, Some(1396));
    assert_eq!(ids.anilist, Some(2));
    assert_eq!(ids.get(ExternalSource::Tvdb), Some(ExternalId::tvdb(81189)));
    assert_eq!(ids.get(ExternalSource::Mal), Some(ExternalId::mal(1)));
    assert_eq!(
        ids.get(ExternalSource::Wikidata),
        Some(ExternalId::wikidata("Q1079"))
    );
    assert_eq!(ExternalIds::new().get(ExternalSource::Imdb), None);
    assert_eq!(ExternalIds::new(), ExternalIds::default());
}

#[test]
fn external_match_constructors() {
    let m = ExternalMatch::movie(MediaId::tmdb(550));
    assert_eq!(m.kind, MatchKind::Movie);
    assert_eq!(m.id, "tmdb:550");
    assert_eq!(
        ExternalMatch::tv_show(MediaId::anilist(1)).kind,
        MatchKind::TvShow
    );
    assert_eq!(
        ExternalMatch::new(MediaId::tmdb(1), MatchKind::TvShow),
        ExternalMatch::tv_show(MediaId::tmdb(1))
    );
}
