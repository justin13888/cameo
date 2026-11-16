//! Conversions from TheTVDB v4 records to unified models.
//!
//! Names and overviews prefer the translation in the client's configured
//! language and fall back to the record's original-language values; the
//! original-language name becomes `original_title` / `original_name`.
//! TheTVDB's ISO 639-2 language codes are mapped to the ISO 639-1 codes the
//! unified models carry where a mapping is known, and dropped otherwise.

mod details;
mod episode;
mod search;

pub(crate) use details::{movie_to_details, series_to_details};
pub(crate) use episode::{episode_to_unified, season_details};
pub(crate) use search::{search_to_movie, search_to_result, search_to_tv_show};

use crate::{
    providers::tvdb::response::{
        TvdbAlias, TvdbArtwork, TvdbGenre, TvdbRemoteId, TvdbTranslations,
    },
    unified::{
        date::PartialDate,
        external_ids::ExternalIds,
        genre::Genre,
        media_id::MediaId,
        models::{AlternativeTitle, SeriesStatus, TitleKind},
    },
};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// The [`MediaId`] of a TheTVDB record, or `None` (logged at `warn`) for id
/// `0`, which names no record.
pub(crate) fn tvdb_media_id(id: u64) -> Option<MediaId> {
    let valid = (id != 0).then_some(id);
    if valid.is_none() {
        tracing::warn!(tvdb_id = id, "tvdb: skipping a record with an invalid id");
    }
    valid.map(MediaId::tvdb)
}

/// Parse a TheTVDB `YYYY[-MM[-DD]]` date; empty or malformed values are `None`.
fn parse_date(value: Option<&str>) -> Option<PartialDate> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse().ok())
}

/// The first non-blank value.
fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.trim().is_empty())
}

/// Map a TheTVDB ISO 639-2 language code to ISO 639-1, for the languages
/// TheTVDB carries most; `None` for any other code.
pub(crate) fn iso639_1(code: &str) -> Option<&'static str> {
    Some(match code {
        "ara" => "ar",
        "bul" => "bg",
        "cat" => "ca",
        "ces" => "cs",
        "dan" => "da",
        "deu" => "de",
        "ell" => "el",
        "eng" => "en",
        "fin" => "fi",
        "fra" => "fr",
        "heb" => "he",
        "hin" => "hi",
        "hrv" => "hr",
        "hun" => "hu",
        "ind" => "id",
        "ita" => "it",
        "jpn" => "ja",
        "kor" => "ko",
        "msa" => "ms",
        "nld" => "nl",
        "nor" => "no",
        "pol" => "pl",
        "por" | "pt" => "pt",
        "ron" => "ro",
        "rus" => "ru",
        "slk" => "sk",
        "slv" => "sl",
        "spa" => "es",
        "srp" => "sr",
        "swe" => "sv",
        "tha" => "th",
        "tur" => "tr",
        "ukr" => "uk",
        "vie" => "vi",
        "zho" => "zh",
        _ => return None,
    })
}

/// Map a TheTVDB series status name (case-insensitive).
pub(crate) fn series_status(name: &str) -> SeriesStatus {
    match name.to_ascii_lowercase().as_str() {
        "continuing" => SeriesStatus::ReturningSeries,
        "ended" => SeriesStatus::Ended,
        "upcoming" => SeriesStatus::Planned,
        _ => SeriesStatus::Other(name.to_string()),
    }
}

/// Unified genres from TheTVDB genre names.
fn genres<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Genre> {
    names.into_iter().map(Genre::from_name).collect()
}

fn record_genres(list: &[TvdbGenre]) -> Vec<Genre> {
    genres(list.iter().map(|g| g.name.as_str()))
}

/// The highest-scored artwork of `artwork_type`.
fn best_artwork(artworks: &[TvdbArtwork], artwork_type: i64) -> Option<&str> {
    artworks
        .iter()
        .filter(|a| a.artwork_type == artwork_type && !a.image.is_empty())
        .max_by(|a, b| a.score.unwrap_or(0.0).total_cmp(&b.score.unwrap_or(0.0)))
        .map(|a| a.image.as_str())
}

/// Every title variant: name translations (other than the headline `title`'s
/// language), then aliases.
fn alternative_titles(
    translations: &TvdbTranslations,
    aliases: &[TvdbAlias],
    original_language: Option<&str>,
) -> Vec<AlternativeTitle> {
    let mut titles = Vec::new();
    for t in &translations.name_translations {
        let Some(name) = non_blank(t.name.as_deref()) else {
            continue;
        };
        let kind = if t.is_alias == Some(true) {
            TitleKind::Synonym
        } else if Some(t.language.as_str()) == original_language {
            TitleKind::Native
        } else if t.language == "eng" {
            TitleKind::English
        } else {
            TitleKind::Other
        };
        titles.push(with_language(
            AlternativeTitle::new(name, kind),
            Some(&t.language),
        ));
    }
    for alias in aliases {
        if !alias.name.trim().is_empty() {
            titles.push(with_language(
                AlternativeTitle::new(&alias.name, TitleKind::Synonym),
                alias.language.as_deref(),
            ));
        }
    }
    for alias in &translations.alias {
        if !alias.trim().is_empty() {
            titles.push(AlternativeTitle::new(alias, TitleKind::Synonym));
        }
    }
    titles
}

fn with_language(title: AlternativeTitle, language: Option<&str>) -> AlternativeTitle {
    match language.and_then(iso639_1) {
        Some(code) => title.with_language(code),
        None => title,
    }
}

/// External ids from TheTVDB remote ids, keyed by `sourceName`.
pub(crate) fn external_ids(remote_ids: &[TvdbRemoteId]) -> ExternalIds {
    let mut ids = ExternalIds::new();
    for remote in remote_ids {
        let source = remote
            .source_name
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let value = remote.id.trim();
        if value.is_empty() {
            continue;
        }
        if source == "imdb" {
            ids = ids.with_imdb(value);
        } else if source.starts_with("themoviedb") {
            if let Ok(n) = value.parse() {
                ids = ids.with_tmdb(n);
            }
        } else if source == "myanimelist" {
            if let Ok(n) = value.parse() {
                ids = ids.with_mal(n);
            }
        } else if source == "wikidata" {
            ids = ids.with_wikidata(value);
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::*;

    /// Deserialize a fixture from TheTVDB's wire JSON.
    pub(super) fn from_json<T: DeserializeOwned>(value: Value) -> T {
        serde_json::from_value(value).expect("fixture matches TheTVDB's response shape")
    }

    #[test]
    fn iso639_maps_known_codes_only() {
        let table = [
            ("ara", "ar"),
            ("bul", "bg"),
            ("cat", "ca"),
            ("ces", "cs"),
            ("dan", "da"),
            ("deu", "de"),
            ("ell", "el"),
            ("eng", "en"),
            ("fin", "fi"),
            ("fra", "fr"),
            ("heb", "he"),
            ("hin", "hi"),
            ("hrv", "hr"),
            ("hun", "hu"),
            ("ind", "id"),
            ("ita", "it"),
            ("jpn", "ja"),
            ("kor", "ko"),
            ("msa", "ms"),
            ("nld", "nl"),
            ("nor", "no"),
            ("pol", "pl"),
            ("por", "pt"),
            ("pt", "pt"),
            ("ron", "ro"),
            ("rus", "ru"),
            ("slk", "sk"),
            ("slv", "sl"),
            ("spa", "es"),
            ("srp", "sr"),
            ("swe", "sv"),
            ("tha", "th"),
            ("tur", "tr"),
            ("ukr", "uk"),
            ("vie", "vi"),
            ("zho", "zh"),
        ];
        for (code, expected) in table {
            assert_eq!(iso639_1(code), Some(expected), "{code}");
        }
        assert_eq!(iso639_1("xyz"), None);
    }

    /// Name translations are classed by alias flag, original language and
    /// English, in that order; blank names and aliases are skipped.
    #[test]
    fn alternative_titles_classify_each_variant() {
        let translations: TvdbTranslations = from_json(json!({
            "nameTranslations": [
                { "language": "jpn", "name": "進撃の巨人" },
                { "language": "eng", "name": "Attack on Titan" },
                { "language": "fra", "name": "L'Attaque des Titans" },
                { "language": "deu", "name": "AoT DE", "isAlias": true },
                { "language": "spa", "name": "  " }
            ],
            "alias": ["Shingeki", " "]
        }));
        let aliases: Vec<TvdbAlias> = from_json(json!([
            { "language": "eng", "name": "AoT" },
            { "name": " " }
        ]));

        assert_eq!(
            alternative_titles(&translations, &aliases, Some("jpn")),
            vec![
                AlternativeTitle::new("進撃の巨人", TitleKind::Native).with_language("ja"),
                AlternativeTitle::new("Attack on Titan", TitleKind::English).with_language("en"),
                AlternativeTitle::new("L'Attaque des Titans", TitleKind::Other).with_language("fr"),
                AlternativeTitle::new("AoT DE", TitleKind::Synonym).with_language("de"),
                AlternativeTitle::new("AoT", TitleKind::Synonym).with_language("en"),
                AlternativeTitle::new("Shingeki", TitleKind::Synonym),
            ]
        );
    }

    #[test]
    fn series_status_maps_every_known_value() {
        assert_eq!(series_status("Continuing"), SeriesStatus::ReturningSeries);
        assert_eq!(series_status("Ended"), SeriesStatus::Ended);
        assert_eq!(series_status("upcoming"), SeriesStatus::Planned);
        assert_eq!(
            series_status("Paused"),
            SeriesStatus::Other("Paused".into())
        );
    }

    #[test]
    fn external_ids_read_each_known_source() {
        let ids = external_ids(&from_json::<Vec<TvdbRemoteId>>(json!([
            { "id": "tt0903747", "type": 2, "sourceName": "IMDB" },
            { "id": "1396", "type": 12, "sourceName": "TheMovieDB.com" },
            { "id": "Q1079", "sourceName": "Wikidata" },
            { "id": "5114", "sourceName": "MyAnimeList" },
            { "id": "x", "sourceName": "Official Website" },
            { "id": "not-a-number", "sourceName": "TheMovieDB.com" }
        ])));
        assert_eq!(ids.imdb.as_deref(), Some("tt0903747"));
        assert_eq!(ids.tmdb, Some(1396));
        assert_eq!(ids.wikidata.as_deref(), Some("Q1079"));
        assert_eq!(ids.mal, Some(5114));
        assert_eq!(ids.tvdb, None);
    }

    #[test]
    fn invalid_record_id_is_skipped() {
        assert_eq!(tvdb_media_id(0), None);
        assert_eq!(tvdb_media_id(5), Some(MediaId::tvdb(5)));
        assert_eq!(tvdb_media_id(u64::MAX), Some(MediaId::tvdb(u64::MAX)));
    }
}
