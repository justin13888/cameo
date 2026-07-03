//! Typed discover-query parameters and the shared setter core.
//!
//! The movie and TV discover builders share ~15 common filters; those setters
//! are defined **once** in the `common_discover_setters` macro and invoked from
//! builder. Provider ID filters take `&[u64]` slices (joined internally) and the
//! previously stringly-typed codes are replaced by typed enums so invalid
//! inputs are unrepresentable.

/// Join a slice of numeric TMDB ids into TMDB's comma-separated (AND) form.
pub(crate) fn join_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

/// A movie release type (TMDB `with_release_type`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovieReleaseType {
    /// Premiere.
    Premiere,
    /// Theatrical (limited).
    TheatricalLimited,
    /// Theatrical.
    Theatrical,
    /// Digital.
    Digital,
    /// Physical.
    Physical,
    /// TV.
    Tv,
}

impl MovieReleaseType {
    pub(crate) fn code(self) -> i32 {
        match self {
            MovieReleaseType::Premiere => 1,
            MovieReleaseType::TheatricalLimited => 2,
            MovieReleaseType::Theatrical => 3,
            MovieReleaseType::Digital => 4,
            MovieReleaseType::Physical => 5,
            MovieReleaseType::Tv => 6,
        }
    }
}

/// A watch-provider monetization type (TMDB `with_watch_monetization_types`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonetizationType {
    /// Flat-rate / subscription streaming.
    Flatrate,
    /// Free with an account.
    Free,
    /// Ad-supported.
    Ads,
    /// Rental.
    Rent,
    /// Purchase.
    Buy,
}

impl MonetizationType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MonetizationType::Flatrate => "flatrate",
            MonetizationType::Free => "free",
            MonetizationType::Ads => "ads",
            MonetizationType::Rent => "rent",
            MonetizationType::Buy => "buy",
        }
    }

    /// Join a slice into TMDB's pipe-separated (OR) form.
    pub(crate) fn join(kinds: &[MonetizationType]) -> String {
        kinds
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }
}

/// A TV show release status (TMDB `with_status`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TvStatus {
    /// Returning series.
    ReturningSeries,
    /// Planned.
    Planned,
    /// In production.
    InProduction,
    /// Ended.
    Ended,
    /// Cancelled.
    Cancelled,
    /// Pilot.
    Pilot,
}

impl TvStatus {
    pub(crate) fn code(self) -> &'static str {
        match self {
            TvStatus::ReturningSeries => "0",
            TvStatus::Planned => "1",
            TvStatus::InProduction => "2",
            TvStatus::Ended => "3",
            TvStatus::Cancelled => "4",
            TvStatus::Pilot => "5",
        }
    }
}

/// A TV show type (TMDB `with_type`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TvShowType {
    /// Documentary.
    Documentary,
    /// News.
    News,
    /// Miniseries.
    Miniseries,
    /// Reality.
    Reality,
    /// Scripted.
    Scripted,
    /// Talk show.
    TalkShow,
    /// Video.
    Video,
}

impl TvShowType {
    pub(crate) fn code(self) -> &'static str {
        match self {
            TvShowType::Documentary => "0",
            TvShowType::News => "1",
            TvShowType::Miniseries => "2",
            TvShowType::Reality => "3",
            TvShowType::Scripted => "4",
            TvShowType::TalkShow => "5",
            TvShowType::Video => "6",
        }
    }
}

/// Generate the ~15 filter setters shared by both discover builders.
///
/// Invoked inside each builder's `impl` block so the shared parameters are
/// defined exactly once. Every field it references exists (with the same name
/// and type) on both builders.
macro_rules! common_discover_setters {
    () => {
        /// Filter by genre IDs (AND semantics — items must match all).
        pub fn with_genres(mut self, genre_ids: &[u64]) -> Self {
            self.with_genres = Some($crate::providers::tmdb::builders::params::join_ids(
                genre_ids,
            ));
            self
        }

        /// Exclude genre IDs.
        pub fn without_genres(mut self, genre_ids: &[u64]) -> Self {
            self.without_genres = Some($crate::providers::tmdb::builders::params::join_ids(
                genre_ids,
            ));
            self
        }

        /// Filter by keyword IDs.
        pub fn with_keywords(mut self, keyword_ids: &[u64]) -> Self {
            self.with_keywords = Some($crate::providers::tmdb::builders::params::join_ids(
                keyword_ids,
            ));
            self
        }

        /// Exclude keyword IDs.
        pub fn without_keywords(mut self, keyword_ids: &[u64]) -> Self {
            self.without_keywords = Some($crate::providers::tmdb::builders::params::join_ids(
                keyword_ids,
            ));
            self
        }

        /// Filter by company IDs.
        pub fn with_companies(mut self, company_ids: &[u64]) -> Self {
            self.with_companies = Some($crate::providers::tmdb::builders::params::join_ids(
                company_ids,
            ));
            self
        }

        /// Exclude company IDs.
        pub fn without_companies(mut self, company_ids: &[u64]) -> Self {
            self.without_companies = Some($crate::providers::tmdb::builders::params::join_ids(
                company_ids,
            ));
            self
        }

        /// Filter by watch-provider IDs (requires [`Self::watch_region`]).
        pub fn with_watch_providers(mut self, provider_ids: &[u64]) -> Self {
            self.with_watch_providers = Some($crate::providers::tmdb::builders::params::join_ids(
                provider_ids,
            ));
            self
        }

        /// Exclude watch-provider IDs.
        pub fn without_watch_providers(mut self, provider_ids: &[u64]) -> Self {
            self.without_watch_providers = Some(
                $crate::providers::tmdb::builders::params::join_ids(provider_ids),
            );
            self
        }

        /// Filter by watch-provider monetization types.
        pub fn with_watch_monetization_types(
            mut self,
            kinds: &[$crate::providers::tmdb::builders::params::MonetizationType],
        ) -> Self {
            self.with_watch_monetization_types =
                Some($crate::providers::tmdb::builders::params::MonetizationType::join(kinds));
            self
        }

        /// Filter by watch region (ISO 3166-1 alpha-2 country code).
        pub fn watch_region(mut self, region: impl Into<String>) -> Self {
            self.watch_region = Some(region.into());
            self
        }

        /// Filter by original language (ISO 639-1).
        pub fn with_original_language(mut self, lang: impl Into<String>) -> Self {
            self.with_original_language = Some(lang.into());
            self
        }

        /// Minimum vote average.
        pub fn vote_average_gte(mut self, min: f32) -> Self {
            self.vote_average_gte = Some(min);
            self
        }

        /// Maximum vote average.
        pub fn vote_average_lte(mut self, max: f32) -> Self {
            self.vote_average_lte = Some(max);
            self
        }

        /// Minimum vote count.
        pub fn vote_count_gte(mut self, min: f32) -> Self {
            self.vote_count_gte = Some(min);
            self
        }

        /// Maximum vote count.
        pub fn vote_count_lte(mut self, max: f32) -> Self {
            self.vote_count_lte = Some(max);
            self
        }

        /// Minimum runtime in minutes.
        pub fn with_runtime_gte(mut self, min: u32) -> Self {
            self.with_runtime_gte = Some(min.min(i32::MAX as u32) as i32);
            self
        }

        /// Maximum runtime in minutes.
        pub fn with_runtime_lte(mut self, max: u32) -> Self {
            self.with_runtime_lte = Some(max.min(i32::MAX as u32) as i32);
            self
        }

        /// Include adult content.
        pub fn include_adult(mut self, include: bool) -> Self {
            self.include_adult = Some(include);
            self
        }

        /// Set the result page (clamped to TMDB's `1..=500` range).
        pub fn page(mut self, page: u32) -> Self {
            self.page = Some(page.clamp(1, 500) as i32);
            self
        }
    };
}

pub(crate) use common_discover_setters;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_ids_is_comma_separated() {
        assert_eq!(join_ids(&[28, 12, 878]), "28,12,878");
        assert_eq!(join_ids(&[550]), "550");
        assert_eq!(join_ids(&[]), "");
    }

    #[test]
    fn monetization_joins_with_pipe() {
        assert_eq!(
            MonetizationType::join(&[MonetizationType::Flatrate, MonetizationType::Rent]),
            "flatrate|rent"
        );
        assert_eq!(MonetizationType::join(&[MonetizationType::Free]), "free");
    }

    #[test]
    fn typed_codes_match_tmdb() {
        assert_eq!(MovieReleaseType::Theatrical.code(), 3);
        assert_eq!(MovieReleaseType::Tv.code(), 6);
        assert_eq!(TvStatus::ReturningSeries.code(), "0");
        assert_eq!(TvStatus::Pilot.code(), "5");
        assert_eq!(TvShowType::Scripted.code(), "4");
        assert_eq!(TvShowType::Video.code(), "6");
    }
}
