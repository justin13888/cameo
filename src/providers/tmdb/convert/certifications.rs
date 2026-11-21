//! TMDB release-date and content-rating responses → [`TmdbCertification`]
//! and the unified [`Certification`].

use crate::{
    generated::tmdb::types, providers::tmdb::models::TmdbCertification,
    unified::models::Certification,
};

impl From<TmdbCertification> for Certification {
    fn from(c: TmdbCertification) -> Self {
        Self {
            country: c.country,
            rating: c.certification,
        }
    }
}

impl TmdbCertification {
    /// Pair a country with a rating, trimming both and rejecting either when
    /// it is empty.
    fn from_parts(country: Option<String>, certification: Option<String>) -> Option<Self> {
        let country = non_blank(country)?;
        let certification = non_blank(certification)?;
        Some(Self {
            country,
            certification,
        })
    }
}

/// The trimmed value, or `None` when it is missing or blank.
fn non_blank(value: Option<String>) -> Option<String> {
    let trimmed = value?.trim().to_owned();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// TMDB's release type for a theatrical release (`3`), whose certification
/// is preferred over those of other release types in the same country.
const THEATRICAL_RELEASE: i64 = 3;

/// One certification per country from a movie's `/release_dates` response.
///
/// A country lists one entry per release (premiere, theatrical, digital, ...),
/// many with an empty certification. The theatrical release's certification
/// wins; otherwise the first non-empty one in TMDB's order. Countries with no
/// code or no non-empty certification are skipped. Countries keep TMDB's order.
pub(crate) fn movie_certifications(
    response: types::MovieReleaseDatesResponse,
) -> Vec<TmdbCertification> {
    movie_results(response.results.into_option())
}

/// [`movie_certifications`] over the `results` list, which the
/// `/release_dates` response and the details `release_dates` append share.
pub(crate) fn movie_results(
    results: Option<types::MovieReleaseDatesResponseResults>,
) -> Vec<TmdbCertification> {
    results
        .into_iter()
        .flatten()
        .filter_map(|country| {
            let mut releases: Vec<_> = country
                .release_dates
                .into_option()
                .into_iter()
                .flatten()
                .collect();
            // A stable sort keeps TMDB's order among non-theatrical releases.
            releases.sort_by_key(|r| r.r#type.as_option() != Some(&THEATRICAL_RELEASE));
            let certification = releases
                .into_iter()
                .find_map(|r| non_blank(r.certification.into_option()))?;
            TmdbCertification::from_parts(country.iso_3166_1.into_option(), Some(certification))
        })
        .collect()
}

/// One certification per country from a TV series' `/content_ratings`
/// response. Entries with no country code or an empty rating are skipped.
pub(crate) fn tv_series_certifications(
    response: types::TvSeriesContentRatingsResponse,
) -> Vec<TmdbCertification> {
    tv_series_results(response.results.into_option())
}

/// [`tv_series_certifications`] over the `results` list, which the
/// `/content_ratings` response and the details `content_ratings` append share.
pub(crate) fn tv_series_results(
    results: Option<types::TvSeriesContentRatingsResponseResults>,
) -> Vec<TmdbCertification> {
    results
        .into_iter()
        .flatten()
        .filter_map(|r| {
            TmdbCertification::from_parts(r.iso_3166_1.into_option(), r.rating.into_option())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A movie's certification per country prefers the theatrical release,
    /// falls back to the first non-empty one, and skips countries with no
    /// code or no certification.
    #[test]
    fn movie_certifications_pick_one_per_country() {
        let raw: types::MovieReleaseDatesResponse = serde_json::from_value(json!({
            "id": 603,
            "results": [
                { "iso_3166_1": "US", "release_dates": [
                    { "certification": "", "type": 1 },
                    { "certification": "NR", "type": 4 },
                    { "certification": " R ", "type": 3 }
                ] },
                { "iso_3166_1": "DE", "release_dates": [
                    { "certification": "", "type": 3 },
                    { "certification": "16", "type": 5 },
                    { "certification": "18", "type": 6 }
                ] },
                { "iso_3166_1": "FR", "release_dates": [{ "certification": "", "type": 3 }] },
                { "iso_3166_1": "GB" },
                { "release_dates": [{ "certification": "PG", "type": 3 }] },
                { "iso_3166_1": " ", "release_dates": [{ "certification": "PG", "type": 3 }] }
            ]
        }))
        .expect("release dates deserialize");

        let certs: Vec<_> = movie_certifications(raw)
            .into_iter()
            .map(|c| (c.country, c.certification))
            .collect();
        assert_eq!(
            certs,
            [
                ("US".to_owned(), "R".to_owned()),
                ("DE".to_owned(), "16".to_owned())
            ]
        );
    }

    /// TV content ratings map one-to-one, skipping blank entries; a missing
    /// `results` reads as no ratings.
    #[test]
    fn tv_series_certifications_skip_blank_entries() {
        let raw: types::TvSeriesContentRatingsResponse = serde_json::from_value(json!({
            "id": 1396,
            "results": [
                { "iso_3166_1": "US", "rating": "TV-MA", "descriptors": [] },
                { "iso_3166_1": "DE", "rating": "" },
                { "iso_3166_1": "BR", "rating": null },
                { "rating": "16" },
                { "iso_3166_1": "KR", "rating": "19" }
            ]
        }))
        .expect("content ratings deserialize");

        let certs: Vec<_> = tv_series_certifications(raw)
            .into_iter()
            .map(|c| (c.country, c.certification))
            .collect();
        assert_eq!(
            certs,
            [
                ("US".to_owned(), "TV-MA".to_owned()),
                ("KR".to_owned(), "19".to_owned())
            ]
        );

        let empty: types::TvSeriesContentRatingsResponse =
            serde_json::from_value(json!({ "id": 1 })).expect("deserialize");
        assert!(tv_series_certifications(empty).is_empty());
    }
}
