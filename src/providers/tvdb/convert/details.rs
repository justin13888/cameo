//! TheTVDB series and movie records → unified details conversions.

use super::{
    alternative_titles, best_artwork, external_ids, iso639_1, non_blank, parse_date, record_genres,
    series_status,
};
use crate::{
    providers::tvdb::response::{TvdbMovie, TvdbSeason, TvdbSeries},
    unified::{
        media_id::MediaId,
        models::{
            Company, Network, SeriesStatus, UnifiedMovie, UnifiedMovieDetails, UnifiedSeason,
            UnifiedTvShow, UnifiedTvShowDetails,
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
    let in_default = |season: &&TvdbSeason| {
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
