//! Conversions from TheTVDB v4 records to unified models.
//!
//! Names and overviews prefer the translation in the client's configured
//! language and fall back to the record's original-language values; the
//! original-language name becomes `original_title` / `original_name`.
//! TheTVDB's ISO 639-2 language codes are mapped to the ISO 639-1 codes the
//! unified models carry where a mapping is known, and dropped otherwise.

use crate::{
    providers::tvdb::response::{
        TvdbAlias, TvdbArtwork, TvdbEpisode, TvdbGenre, TvdbMovie, TvdbRemoteId, TvdbSearchResult,
        TvdbSeries, TvdbTranslations,
    },
    unified::{
        date::PartialDate,
        external_ids::ExternalIds,
        genre::Genre,
        media_id::MediaId,
        models::{
            AlternativeTitle, Company, Network, SeriesStatus, TitleKind, UnifiedEpisode,
            UnifiedMovie, UnifiedMovieDetails, UnifiedSearchResult, UnifiedSeason,
            UnifiedSeasonDetails, UnifiedTvShow, UnifiedTvShowDetails,
        },
    },
};

/// Series artwork type: poster.
const SERIES_POSTER: i64 = 2;
/// Series artwork type: background.
const SERIES_BACKGROUND: i64 = 3;
/// Movie artwork type: poster.
const MOVIE_POSTER: i64 = 14;
/// Movie artwork type: background.
const MOVIE_BACKGROUND: i64 = 15;

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

// ── Search ────────────────────────────────────────────────────────────────────

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

// ── Details ───────────────────────────────────────────────────────────────────

/// A series' details in `language`.
pub(crate) fn series_to_details(s: &TvdbSeries, language: &str) -> UnifiedTvShowDetails {
    let name = s.translations.name(language).unwrap_or(&s.name);
    let mut show = UnifiedTvShow::new(MediaId::tvdb(s.id), name)
        .with_alternative_titles(alternative_titles(
            &s.translations,
            &s.aliases,
            s.original_language.as_deref(),
        ))
        .with_genres(record_genres(&s.genres));
    if s.name != name {
        show = show.with_original_name(&s.name);
    }
    if let Some(v) = s
        .translations
        .overview(language)
        .or(non_blank(s.overview.as_deref()))
    {
        show = show.with_overview(v);
    }
    if let Some(v) = parse_date(s.first_aired.as_deref()) {
        show = show.with_first_air_date(v);
    }
    if let Some(v) = non_blank(s.image.as_deref()).or(best_artwork(&s.artworks, SERIES_POSTER)) {
        show = show.with_poster_url(v);
    }
    if let Some(v) = best_artwork(&s.artworks, SERIES_BACKGROUND) {
        show = show.with_backdrop_url(v);
    }
    if let Some(v) = s.score {
        show = show.with_popularity(v);
    }
    if let Some(v) = s.original_language.as_deref().and_then(iso639_1) {
        show = show.with_original_language(v);
    }
    if let Some(v) = non_blank(s.original_country.as_deref()) {
        show = show.with_origin_country(vec![v.to_string()]);
    }
    if let Some(v) = s.average_runtime.filter(|&r| r > 0) {
        show = show.with_episode_run_time(v);
    }
    let status = s
        .status
        .as_ref()
        .and_then(|st| non_blank(st.name.as_deref()))
        .map(series_status);
    if let Some(v) = status.clone() {
        show = show.with_status(v);
    }

    let seasons = default_order_seasons(s);
    let regular_seasons = seasons.iter().filter(|x| x.season_number > 0).count();
    let mut details = UnifiedTvShowDetails::new(show)
        .with_networks(networks(s))
        .with_seasons(seasons)
        .with_number_of_seasons(u32::try_from(regular_seasons).unwrap_or(u32::MAX));
    if let Some(v) = s.average_runtime.filter(|&r| r > 0) {
        details = details.with_episode_run_time(vec![v]);
    }
    if let Some(v) = parse_date(s.last_aired.as_deref()) {
        details = details.with_last_air_date(v);
    }
    if let Some(v) = status {
        details = details
            .with_in_production(matches!(
                v,
                SeriesStatus::ReturningSeries | SeriesStatus::Planned
            ))
            .with_status(v);
    }
    if let Some(v) = s.translations.name_translations.iter().find_map(|t| {
        (t.language == language)
            .then_some(t.tagline.as_deref())
            .flatten()
            .and_then(|t| non_blank(Some(t)))
    }) {
        details = details.with_tagline(v);
    }
    details
}

/// The original and latest networks, without duplicates.
fn networks(s: &TvdbSeries) -> Vec<Network> {
    let mut out: Vec<Network> = Vec::new();
    for company in [&s.original_network, &s.latest_network]
        .into_iter()
        .flatten()
    {
        if out.iter().any(|n| n.id == Some(company.id)) {
            continue;
        }
        let mut network = Network::new(&company.name).with_id(company.id);
        if let Some(c) = non_blank(company.country.as_deref()) {
            network = network.with_origin_country(c);
        }
        out.push(network);
    }
    out
}

/// The seasons of the series' default episode order (TheTVDB's `official`
/// order when the default is not reported), by number, specials (`0`)
/// included.
fn default_order_seasons(s: &TvdbSeries) -> Vec<UnifiedSeason> {
    let in_default = |season: &&crate::providers::tvdb::response::TvdbSeason| {
        let ty = season.season_type.as_ref();
        match s.default_season_type {
            Some(default) => ty.and_then(|t| t.id) == Some(default),
            None => ty.and_then(|t| t.order.as_deref()) == Some("official"),
        }
    };
    let mut seasons: Vec<UnifiedSeason> = Vec::new();
    for season in s.seasons.iter().filter(in_default) {
        if seasons.iter().any(|x| x.season_number == season.number) {
            continue;
        }
        let mut unified = UnifiedSeason::new(season.number);
        if let Some(v) = parse_date(season.year.as_deref()) {
            unified = unified.with_air_date(v);
        }
        if let Some(v) = non_blank(season.image.as_deref()) {
            unified = unified.with_poster_url(v);
        }
        seasons.push(unified);
    }
    seasons.sort_by_key(|x| x.season_number);
    seasons
}

/// A movie's details in `language`.
pub(crate) fn movie_to_details(m: &TvdbMovie, language: &str) -> UnifiedMovieDetails {
    let title = m.translations.name(language).unwrap_or(&m.name);
    let mut movie = UnifiedMovie::new(MediaId::tvdb(m.id), title)
        .with_alternative_titles(alternative_titles(
            &m.translations,
            &m.aliases,
            m.original_language.as_deref(),
        ))
        .with_genres(record_genres(&m.genres));
    if m.name != title {
        movie = movie.with_original_title(&m.name);
    }
    if let Some(v) = m.translations.overview(language) {
        movie = movie.with_overview(v);
    }
    let release = m
        .first_release
        .as_ref()
        .and_then(|r| parse_date(r.date.as_deref()))
        .or_else(|| parse_date(m.year.as_deref()));
    if let Some(v) = release {
        movie = movie.with_release_date(v);
    }
    if let Some(v) = non_blank(m.image.as_deref()).or(best_artwork(&m.artworks, MOVIE_POSTER)) {
        movie = movie.with_poster_url(v);
    }
    if let Some(v) = best_artwork(&m.artworks, MOVIE_BACKGROUND) {
        movie = movie.with_backdrop_url(v);
    }
    if let Some(v) = m.score {
        movie = movie.with_popularity(v);
    }
    if let Some(v) = m.original_language.as_deref().and_then(iso639_1) {
        movie = movie.with_original_language(v);
    }
    let runtime = m.runtime.filter(|&r| r > 0);
    if let Some(v) = runtime {
        movie = movie.with_runtime(v);
    }
    let status = m.status.as_ref().and_then(|s| non_blank(s.name.as_deref()));
    if let Some(v) = status {
        movie = movie.with_status(v);
    }

    let companies = m
        .companies
        .production
        .iter()
        .chain(&m.companies.studio)
        .fold(Vec::<Company>::new(), |mut out, c| {
            if !out.iter().any(|x| x.id == Some(c.id)) {
                let mut company = Company::new(&c.name).with_id(c.id);
                if let Some(country) = non_blank(c.country.as_deref()) {
                    company = company.with_origin_country(country);
                }
                out.push(company);
            }
            out
        });
    let imdb = external_ids(&m.remote_ids).imdb;
    let mut details = UnifiedMovieDetails::new(movie)
        .with_production_companies(companies)
        .with_production_countries(
            m.production_countries
                .iter()
                .map(|c| c.name.clone())
                .collect(),
        )
        .with_spoken_languages(
            m.spoken_languages
                .iter()
                .map(|l| iso639_1(l).map_or_else(|| l.clone(), str::to_string))
                .collect(),
        );
    if let Some(v) = runtime {
        details = details.with_runtime(v);
    }
    if let Some(v) = status {
        details = details.with_status(v);
    }
    if let Some(v) = imdb {
        details = details.with_imdb_id(v);
    }
    if let Some(v) = m.budget.filter(|&b| b > 0) {
        details = details.with_budget(v);
    }
    if let Some(v) = m.box_office.filter(|&b| b > 0) {
        details = details.with_revenue(v);
    }
    if let Some(v) = m.translations.name_translations.iter().find_map(|t| {
        (t.language == language)
            .then_some(t.tagline.as_deref())
            .flatten()
            .and_then(|t| non_blank(Some(t)))
    }) {
        details = details.with_tagline(v);
    }
    details
}

// ── Episodes ──────────────────────────────────────────────────────────────────

/// An episode as a [`UnifiedEpisode`].
pub(crate) fn episode_to_unified(e: &TvdbEpisode) -> UnifiedEpisode {
    let mut episode = UnifiedEpisode::new(e.number);
    if let Some(v) = non_blank(e.name.as_deref()) {
        episode = episode.with_name(v);
    }
    if let Some(v) = non_blank(e.overview.as_deref()) {
        episode = episode.with_overview(v);
    }
    if let Some(v) = parse_date(e.aired.as_deref()) {
        episode = episode.with_air_date(v);
    }
    if let Some(v) = e.runtime.filter(|&r| r > 0) {
        episode = episode.with_runtime(v);
    }
    if let Some(v) = non_blank(e.image.as_deref()) {
        episode = episode.with_still_url(v);
    }
    episode
}

/// One season of a series built from its default-order `episodes`; `None`
/// when no episode is in `season_number`. The air date is the earliest
/// episode's.
pub(crate) fn season_details(
    show_id: MediaId,
    season_number: u32,
    episodes: &[TvdbEpisode],
) -> Option<UnifiedSeasonDetails> {
    let mut in_season: Vec<&TvdbEpisode> = episodes
        .iter()
        .filter(|e| e.season_number == season_number)
        .collect();
    if in_season.is_empty() {
        return None;
    }
    in_season.sort_by_key(|e| e.number);
    let mut season = UnifiedSeasonDetails::new(show_id, season_number)
        .with_episodes(in_season.iter().map(|e| episode_to_unified(e)).collect());
    if let Some(v) = in_season
        .iter()
        .find_map(|e| non_blank(e.season_name.as_deref()))
    {
        season = season.with_name(v);
    }
    if let Some(v) = in_season
        .iter()
        .filter_map(|e| parse_date(e.aired.as_deref()))
        .min()
    {
        season = season.with_air_date(v);
    }
    Some(season)
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    use super::*;

    fn from_json<T: DeserializeOwned>(value: Value) -> T {
        serde_json::from_value(value).expect("fixture matches TheTVDB's response shape")
    }

    #[test]
    fn iso639_maps_known_codes_only() {
        assert_eq!(iso639_1("eng"), Some("en"));
        assert_eq!(iso639_1("jpn"), Some("ja"));
        assert_eq!(iso639_1("xyz"), None);
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
            "genres": ["Anime", "Action"], "aliases": ["AoT"], "country": "jpn"
        }));
        let show = search_to_tv_show(&r, "eng").expect("valid");
        assert_eq!(show.provider_id, MediaId::tvdb(267440));
        assert_eq!(show.name, "Attack on Titan");
        assert_eq!(show.original_name.as_deref(), Some("進撃の巨人"));
        assert_eq!(show.overview.as_deref(), Some("Humans fight titans."));
        assert_eq!(show.original_language.as_deref(), Some("ja"));
        assert_eq!(show.status, Some(SeriesStatus::Ended));
        assert!(show.genres.contains(&Genre::Action));
        assert_eq!(
            show.first_air_date.map(|d| d.to_string()).as_deref(),
            Some("2013-04-07")
        );

        let jp = search_to_tv_show(&r, "jpn").expect("valid");
        assert_eq!(jp.name, "進撃の巨人");
        assert_eq!(jp.original_name, None);
        assert_eq!(jp.overview.as_deref(), Some("原文"));
    }

    #[test]
    fn season_details_keep_specials_and_order_episodes() {
        let episodes: Vec<TvdbEpisode> = from_json(json!([
            { "id": 3, "seasonNumber": 1, "number": 2, "absoluteNumber": 2, "aired": "2008-01-27" },
            { "id": 2, "seasonNumber": 1, "number": 1, "absoluteNumber": 1, "aired": "2008-01-20",
              "name": "Pilot", "runtime": 58, "image": "https://img/1.jpg" },
            { "id": 9, "seasonNumber": 0, "number": 1, "absoluteNumber": 0, "name": "Special" }
        ]));
        let season = season_details(MediaId::tvdb(81189), 1, &episodes).expect("season 1");
        let numbers: Vec<u32> = season.episodes.iter().map(|e| e.episode_number).collect();
        assert_eq!(numbers, vec![1, 2]);
        assert_eq!(
            season.air_date.map(|d| d.to_string()).as_deref(),
            Some("2008-01-20")
        );
        assert_eq!(season.episodes[0].name.as_deref(), Some("Pilot"));
        assert_eq!(season.episodes[0].runtime, Some(58));
        assert_eq!(
            season.episodes[0].still_url.as_deref(),
            Some("https://img/1.jpg")
        );

        let specials = season_details(MediaId::tvdb(81189), 0, &episodes).expect("specials");
        assert_eq!(specials.episodes.len(), 1);
        assert!(season_details(MediaId::tvdb(81189), 7, &episodes).is_none());
    }

    #[test]
    fn invalid_record_id_is_skipped() {
        assert_eq!(tvdb_media_id(0), None);
        assert_eq!(tvdb_media_id(5), Some(MediaId::tvdb(5)));
        assert_eq!(tvdb_media_id(u64::MAX), Some(MediaId::tvdb(u64::MAX)));
    }
}
