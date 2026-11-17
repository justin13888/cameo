//! TheTVDB search results → unified model conversions.

use super::{genres, iso639_1, non_blank, parse_date, series_status, tvdb_media_id};
use crate::{
    providers::tvdb::response::TvdbSearchResult,
    unified::{
        date::PartialDate,
        media_id::MediaId,
        models::{AlternativeTitle, TitleKind, UnifiedMovie, UnifiedSearchResult, UnifiedTvShow},
    },
};

/// The headline name, original name, and overview of a search result in
/// `language`.
struct SearchText<'a> {
    name: &'a str,
    original: Option<&'a str>,
    overview: Option<&'a str>,
}

fn search_text<'a>(r: &'a TvdbSearchResult, language: &str) -> Option<SearchText<'a>> {
    let original = non_blank(r.name.as_deref());
    let translated = non_blank(r.translations.get(language).map(String::as_str));
    let name = translated.or(original)?;
    let overview = non_blank(r.overviews.get(language).map(String::as_str))
        .or(non_blank(r.overview.as_deref()));
    Some(SearchText {
        name,
        original: original.filter(|o| *o != name),
        overview,
    })
}

/// The id of a search result, or `None` (logged at `warn`) when it is not a
/// canonical decimal id.
fn search_result_id(r: &TvdbSearchResult) -> Option<MediaId> {
    let raw = r.tvdb_id.as_deref().unwrap_or_default();
    // Only the canonical decimal rendering counts (no sign, no leading zero).
    let canonical = raw.parse::<u64>().ok().filter(|n| n.to_string() == raw);
    match canonical {
        Some(n) => tvdb_media_id(n),
        None => {
            tracing::warn!(
                tvdb_id = raw,
                "tvdb: skipping a search result with an invalid id"
            );
            None
        }
    }
}

fn search_date(r: &TvdbSearchResult) -> Option<PartialDate> {
    parse_date(r.first_air_time.as_deref()).or_else(|| parse_date(r.year.as_deref()))
}

fn search_aliases(r: &TvdbSearchResult) -> Vec<AlternativeTitle> {
    r.aliases
        .iter()
        .filter(|a| !a.trim().is_empty())
        .map(|a| AlternativeTitle::new(a, TitleKind::Synonym))
        .collect()
}

/// A movie search result as a [`UnifiedMovie`]; `None` when it has no valid
/// id or no name.
pub(crate) fn search_to_movie(r: &TvdbSearchResult, language: &str) -> Option<UnifiedMovie> {
    let id = search_result_id(r)?;
    let text = search_text(r, language)?;
    let mut movie = UnifiedMovie::new(id, text.name)
        .with_alternative_titles(search_aliases(r))
        .with_genres(genres(r.genres.iter().map(String::as_str)));
    if let Some(v) = text.original {
        movie = movie.with_original_title(v);
    }
    if let Some(v) = text.overview {
        movie = movie.with_overview(v);
    }
    if let Some(v) = search_date(r) {
        movie = movie.with_release_date(v);
    }
    if let Some(v) = non_blank(r.image_url.as_deref()) {
        movie = movie.with_poster_url(v);
    }
    if let Some(v) = r.primary_language.as_deref().and_then(iso639_1) {
        movie = movie.with_original_language(v);
    }
    if let Some(v) = non_blank(r.status.as_deref()) {
        movie = movie.with_status(v);
    }
    Some(movie)
}

/// A series search result as a [`UnifiedTvShow`]; `None` when it has no
/// valid id or no name.
pub(crate) fn search_to_tv_show(r: &TvdbSearchResult, language: &str) -> Option<UnifiedTvShow> {
    let id = search_result_id(r)?;
    let text = search_text(r, language)?;
    let mut show = UnifiedTvShow::new(id, text.name)
        .with_alternative_titles(search_aliases(r))
        .with_genres(genres(r.genres.iter().map(String::as_str)));
    if let Some(v) = text.original {
        show = show.with_original_name(v);
    }
    if let Some(v) = text.overview {
        show = show.with_overview(v);
    }
    if let Some(v) = search_date(r) {
        show = show.with_first_air_date(v);
    }
    if let Some(v) = non_blank(r.image_url.as_deref()) {
        show = show.with_poster_url(v);
    }
    if let Some(v) = r.primary_language.as_deref().and_then(iso639_1) {
        show = show.with_original_language(v);
    }
    if let Some(v) = non_blank(r.country.as_deref()) {
        show = show.with_origin_country(vec![v.to_string()]);
    }
    if let Some(v) = non_blank(r.status.as_deref()) {
        show = show.with_status(series_status(v));
    }
    Some(show)
}

/// A multi-search result: series become TV shows and movies become movies;
/// every other kind (people, companies, ...) is omitted.
pub(crate) fn search_to_result(
    r: &TvdbSearchResult,
    language: &str,
) -> Option<UnifiedSearchResult> {
    match r.kind.as_deref() {
        Some("series") => search_to_tv_show(r, language).map(UnifiedSearchResult::TvShow),
        Some("movie") => search_to_movie(r, language).map(UnifiedSearchResult::Movie),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        providers::tvdb::convert::tests::from_json,
        unified::{genre::Genre, models::SeriesStatus},
    };

    #[test]
    fn search_results_skip_invalid_ids_and_unknown_kinds() {
        let result = |id: &str, kind: &str| {
            from_json::<TvdbSearchResult>(json!({
                "tvdb_id": id, "type": kind, "name": "Name"
            }))
        };
        assert!(search_to_result(&result("81189", "series"), "eng").is_some());
        assert!(search_to_result(&result("81189", "movie"), "eng").is_some());
        assert!(search_to_result(&result("81189", "person"), "eng").is_none());
        assert!(search_to_result(&result("0", "series"), "eng").is_none());
        assert!(search_to_result(&result("0081189", "series"), "eng").is_none());
        assert!(search_to_result(&result("", "movie"), "eng").is_none());
    }

    #[test]
    fn search_prefers_the_translation_in_the_language() {
        let r: TvdbSearchResult = from_json(json!({
            "tvdb_id": "267440", "type": "series", "name": "進撃の巨人",
            "translations": { "eng": "Attack on Titan" },
            "overviews": { "eng": "Humans fight titans." },
            "overview": "原文", "primary_language": "jpn",
            "first_air_time": "2013-04-07", "status": "Ended",
            "genres": ["Anime", "Action"], "aliases": ["AoT", " "], "country": "jpn"
        }));
        let show = search_to_tv_show(&r, "eng").expect("valid");
        assert_eq!(show.provider_id, MediaId::tvdb(267440));
        assert_eq!(show.name, "Attack on Titan");
        assert_eq!(show.original_name.as_deref(), Some("進撃の巨人"));
        assert_eq!(show.overview.as_deref(), Some("Humans fight titans."));
        assert_eq!(show.original_language.as_deref(), Some("ja"));
        assert_eq!(show.status, Some(SeriesStatus::Ended));
        assert!(show.genres.contains(&Genre::Action));
        // Blank aliases are skipped.
        assert_eq!(
            show.alternative_titles,
            vec![AlternativeTitle::new("AoT", TitleKind::Synonym)]
        );
        assert_eq!(
            show.first_air_date.map(|d| d.to_string()).as_deref(),
            Some("2013-04-07")
        );

        let jp = search_to_tv_show(&r, "jpn").expect("valid");
        assert_eq!(jp.name, "進撃の巨人");
        assert_eq!(jp.original_name, None);
        assert_eq!(jp.overview.as_deref(), Some("原文"));
    }
}
