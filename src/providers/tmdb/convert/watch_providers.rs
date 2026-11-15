//! TMDB watch-provider results → unified watch-provider entries.

use std::collections::HashMap;

use serde::Serialize;

use super::resolve_logo;
use crate::{
    providers::tmdb::error::TmdbError,
    unified::models::{UnifiedStreamingService, UnifiedWatchProviderEntry},
};

/// One streaming service under a watch-provider country entry.
#[derive(serde::Deserialize)]
struct WatchProviderService {
    provider_name: Option<String>,
    logo_path: Option<String>,
}

/// One country's watch-provider lists.
#[derive(serde::Deserialize, Default)]
struct WatchProviderCountry {
    #[serde(default)]
    flatrate: Vec<WatchProviderService>,
    #[serde(default)]
    rent: Vec<WatchProviderService>,
    #[serde(default)]
    buy: Vec<WatchProviderService>,
}

impl From<WatchProviderCountry> for UnifiedWatchProviderEntry {
    fn from(c: WatchProviderCountry) -> Self {
        UnifiedWatchProviderEntry {
            flatrate: unified_services(c.flatrate),
            rent: unified_services(c.rent),
            buy: unified_services(c.buy),
        }
    }
}

/// Map watch-provider services to unified services, dropping any without a name.
fn unified_services(list: Vec<WatchProviderService>) -> Vec<UnifiedStreamingService> {
    list.into_iter()
        .filter_map(|s| {
            let name = s.provider_name?;
            Some(UnifiedStreamingService {
                name,
                logo_url: resolve_logo(&s.logo_path),
            })
        })
        .collect()
}

/// Parse a watch-provider response's `results` (one entry per country) into
/// unified entries keyed by country code; absent `results` read as no
/// countries.
///
/// The generated `results` types hold one field per country the spec lists,
/// each with its own type, so they are read through their serialized
/// per-country object rather than field by field.
///
/// # Errors
///
/// [`TmdbError`] when `results` does not have the per-country shape.
pub(crate) fn parse_watch_provider_results<R: Serialize>(
    results: Option<R>,
) -> Result<HashMap<String, UnifiedWatchProviderEntry>, TmdbError> {
    let Some(results) = results else {
        return Ok(HashMap::new());
    };
    let by_country: HashMap<String, WatchProviderCountry> =
        serde_json::from_value(serde_json::to_value(results)?)?;
    Ok(by_country
        .into_iter()
        .map(|(country, entry)| (country, entry.into()))
        .collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::generated::tmdb::types;

    /// Countries map to unified entries; services without a name drop and a
    /// logo path becomes a full W92 logo URL.
    #[test]
    fn parses_each_country_and_drops_unnamed_services() {
        let raw: types::MovieWatchProvidersResponse = serde_json::from_value(json!({
            "id": 550,
            "results": {
                "US": {
                    "flatrate": [
                        { "provider_name": "Netflix", "logo_path": "/n.jpg" },
                        { "logo_path": "/nameless.jpg" }
                    ],
                    "rent": [{ "provider_name": "Apple TV" }]
                },
                "DE": { "buy": [{ "provider_name": "Amazon Video" }] }
            }
        }))
        .expect("watch providers deserialize");

        let results = parse_watch_provider_results(raw.results).expect("per-country shape");

        assert_eq!(results.len(), 2);
        let us = &results["US"];
        assert_eq!(us.flatrate.len(), 1);
        assert_eq!(us.flatrate[0].name, "Netflix");
        assert_eq!(
            us.flatrate[0].logo_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w92/n.jpg")
        );
        assert_eq!(us.rent[0].name, "Apple TV");
        assert_eq!(us.rent[0].logo_url, None);
        assert!(us.buy.is_empty());
        assert_eq!(results["DE"].buy[0].name, "Amazon Video");
    }

    /// A response without `results` has no countries.
    #[test]
    fn absent_results_have_no_countries() {
        let raw: types::TvSeriesWatchProvidersResponse =
            serde_json::from_value(json!({ "id": 1399 })).expect("deserialize");

        let results = parse_watch_provider_results(raw.results).expect("no results");

        assert!(results.is_empty());
    }
}
