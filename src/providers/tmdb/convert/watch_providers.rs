//! TMDB watch-provider results → unified watch-provider entries.

use std::collections::{BTreeMap, HashMap};

use super::resolve_logo;
use crate::{
    generated::tmdb::types::{WatchProviderCountry, WatchProviderService},
    unified::models::{UnifiedStreamingService, UnifiedWatchProviderEntry},
};

impl From<WatchProviderCountry> for UnifiedWatchProviderEntry {
    fn from(c: WatchProviderCountry) -> Self {
        UnifiedWatchProviderEntry {
            flatrate: unified_services(c.flatrate.into_option()),
            rent: unified_services(c.rent.into_option()),
            buy: unified_services(c.buy.into_option()),
        }
    }
}

/// Map watch-provider services to unified services, dropping any without a
/// name; an absent list reads as empty.
fn unified_services(list: Option<Vec<WatchProviderService>>) -> Vec<UnifiedStreamingService> {
    list.unwrap_or_default()
        .into_iter()
        .filter_map(|s| {
            let name = s.provider_name.into_option()?;
            Some(UnifiedStreamingService {
                name,
                logo_url: resolve_logo(&s.logo_path),
            })
        })
        .collect()
}

/// Convert a watch-provider response's per-country `results` map into
/// unified entries keyed by country code; absent `results` read as no
/// countries.
///
/// Every generated `*WatchProvidersResponseResults` type is a map of
/// [`WatchProviderCountry`] keyed by ISO 3166-1 country code, so callers pass
/// its `additional` field.
pub(crate) fn parse_watch_provider_results(
    results: Option<BTreeMap<String, WatchProviderCountry>>,
) -> HashMap<String, UnifiedWatchProviderEntry> {
    results
        .unwrap_or_default()
        .into_iter()
        .map(|(country, entry)| (country, entry.into()))
        .collect()
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
                "DE": { "buy": [{ "provider_name": "Amazon Video" }], "rent": null }
            }
        }))
        .expect("watch providers deserialize");

        let results = parse_watch_provider_results(raw.results.into_option().map(|r| r.additional));

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
        assert!(results["DE"].rent.is_empty());
    }

    /// A country code the bundled spec never listed still reaches the
    /// unified results, for movies and TV series alike.
    #[test]
    fn keeps_countries_the_spec_does_not_list() {
        let body = json!({
            "id": 1,
            "results": { "ZZ": { "flatrate": [{ "provider_name": "Local" }] } }
        });
        let movie: types::MovieWatchProvidersResponse =
            serde_json::from_value(body.clone()).expect("movie deserialize");
        let tv: types::TvSeriesWatchProvidersResponse =
            serde_json::from_value(body).expect("tv deserialize");

        for results in [
            parse_watch_provider_results(movie.results.into_option().map(|r| r.additional)),
            parse_watch_provider_results(tv.results.into_option().map(|r| r.additional)),
        ] {
            assert_eq!(results.len(), 1);
            assert_eq!(results["ZZ"].flatrate[0].name, "Local");
        }
    }

    /// A response without `results`, or with `results: null`, has no
    /// countries.
    #[test]
    fn absent_results_have_no_countries() {
        for body in [
            json!({ "id": 1399 }),
            json!({ "id": 1399, "results": null }),
        ] {
            let raw: types::TvSeriesWatchProvidersResponse =
                serde_json::from_value(body).expect("deserialize");

            let results =
                parse_watch_provider_results(raw.results.into_option().map(|r| r.additional));

            assert!(results.is_empty());
        }
    }
}
