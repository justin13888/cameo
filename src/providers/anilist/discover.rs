//! Filtered anime discovery for [`AniListClient`].
//!
//! [`AniListClient::discover_anime`] returns a [`DiscoverAnimeBuilder`]; set
//! filters with its chainable setters, then run the query with
//! [`execute_movies`](DiscoverAnimeBuilder::execute_movies) or
//! [`execute_tv`](DiscoverAnimeBuilder::execute_tv).
//!
//! Unlike TMDB — which has separate movie and TV discover endpoints with
//! different filter sets, and therefore two builders — AniList exposes one
//! media query whose filters are identical for films and series. A single
//! builder with two typed terminal methods avoids duplicating every setter,
//! while still returning the same unified page types as TMDB's
//! `DiscoverMoviesBuilder::execute` and `DiscoverTvBuilder::execute` (plain
//! names rather than links, since the `tmdb` feature may be disabled).

use serde_json::{Map, Value, json};

use super::{AniListClient, AniListError};
use crate::{
    core::pagination::Page,
    unified::models::{UnifiedMovie, UnifiedTvShow},
};

/// An AniList anime format (the GraphQL `MediaFormat` enum, anime values only).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimeFormat {
    /// A TV series (`TV`).
    Tv,
    /// A short TV series, usually under 15 minutes per episode (`TV_SHORT`).
    TvShort,
    /// A feature film (`MOVIE`).
    Movie,
    /// A special episode or bonus release (`SPECIAL`).
    Special,
    /// An original video animation (`OVA`).
    Ova,
    /// An original net animation (`ONA`).
    Ona,
    /// A short music video (`MUSIC`).
    Music,
}

impl AnimeFormat {
    /// The formats [`DiscoverAnimeBuilder::execute_movies`] requests when no
    /// format filter is set.
    pub(crate) const MOVIES: &'static [AnimeFormat] = &[AnimeFormat::Movie];

    /// The formats [`DiscoverAnimeBuilder::execute_tv`] requests when no format
    /// filter is set.
    pub(crate) const TV: &'static [AnimeFormat] = &[
        AnimeFormat::Tv,
        AnimeFormat::TvShort,
        AnimeFormat::Ona,
        AnimeFormat::Ova,
        AnimeFormat::Special,
    ];

    /// The AniList GraphQL enum value.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AnimeFormat::Tv => "TV",
            AnimeFormat::TvShort => "TV_SHORT",
            AnimeFormat::Movie => "MOVIE",
            AnimeFormat::Special => "SPECIAL",
            AnimeFormat::Ova => "OVA",
            AnimeFormat::Ona => "ONA",
            AnimeFormat::Music => "MUSIC",
        }
    }
}

/// An AniList broadcast season (the GraphQL `MediaSeason` enum).
///
/// This is the *airing* season a show premiered in (e.g. "Spring 2024"), not a
/// season of a TV series.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimeSeason {
    /// December to February (`WINTER`).
    Winter,
    /// March to May (`SPRING`).
    Spring,
    /// June to August (`SUMMER`).
    Summer,
    /// September to November (`FALL`).
    Fall,
}

impl AnimeSeason {
    /// The AniList GraphQL enum value.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AnimeSeason::Winter => "WINTER",
            AnimeSeason::Spring => "SPRING",
            AnimeSeason::Summer => "SUMMER",
            AnimeSeason::Fall => "FALL",
        }
    }
}

/// An AniList release status (the GraphQL `MediaStatus` enum).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimeStatus {
    /// Currently airing (`RELEASING`).
    Releasing,
    /// Finished airing (`FINISHED`).
    Finished,
    /// Announced but not yet airing (`NOT_YET_RELEASED`).
    NotYetReleased,
    /// Cancelled before completion (`CANCELLED`).
    Cancelled,
    /// Paused, expected to resume (`HIATUS`).
    Hiatus,
}

impl AnimeStatus {
    /// The AniList GraphQL enum value.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AnimeStatus::Releasing => "RELEASING",
            AnimeStatus::Finished => "FINISHED",
            AnimeStatus::NotYetReleased => "NOT_YET_RELEASED",
            AnimeStatus::Cancelled => "CANCELLED",
            AnimeStatus::Hiatus => "HIATUS",
        }
    }
}

/// Sort order for [`DiscoverAnimeBuilder`] (a subset of the GraphQL
/// `MediaSort` enum).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimeSort {
    /// Most popular first (`POPULARITY_DESC`). The default.
    #[default]
    PopularityDesc,
    /// Highest average score first (`SCORE_DESC`).
    ScoreDesc,
    /// Currently trending first (`TRENDING_DESC`).
    TrendingDesc,
    /// Most favourited first (`FAVOURITES_DESC`).
    FavouritesDesc,
    /// Newest start date first (`START_DATE_DESC`).
    StartDateDesc,
    /// Oldest start date first (`START_DATE`).
    StartDateAsc,
    /// Romaji title, alphabetical (`TITLE_ROMAJI`).
    TitleRomajiAsc,
}

impl AnimeSort {
    /// The AniList GraphQL enum value.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AnimeSort::PopularityDesc => "POPULARITY_DESC",
            AnimeSort::ScoreDesc => "SCORE_DESC",
            AnimeSort::TrendingDesc => "TRENDING_DESC",
            AnimeSort::FavouritesDesc => "FAVOURITES_DESC",
            AnimeSort::StartDateDesc => "START_DATE_DESC",
            AnimeSort::StartDateAsc => "START_DATE",
            AnimeSort::TitleRomajiAsc => "TITLE_ROMAJI",
        }
    }
}

/// Builder for discovering anime with filters.
///
/// Created by [`AniListClient::discover_anime`]. Every filter is optional;
/// unset filters are not sent. Results are sorted by
/// [`AnimeSort::PopularityDesc`] unless [`sort`](Self::sort) is set.
///
/// # Format precedence
///
/// - With **no** [`formats`](Self::formats) filter,
///   [`execute_movies`](Self::execute_movies) requests only `MOVIE` and
///   [`execute_tv`](Self::execute_tv) requests `TV`, `TV_SHORT`, `ONA`, `OVA`,
///   and `SPECIAL` — the same split as the rest of the AniList provider.
/// - With an explicit `formats` filter, the caller's formats are sent
///   **unchanged** by both terminal methods (the caller's choice wins), and
///   every returned entry is converted to the terminal's unified type.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), cameo::providers::anilist::AniListError> {
/// use cameo::providers::anilist::{
///     AniListClient, AniListConfig, AnimeSeason, AnimeSort, AnimeStatus,
/// };
///
/// let client = AniListClient::new(AniListConfig::new())?;
/// let shows = client
///     .discover_anime()
///     .genres(["Action", "Fantasy"])
///     .season(AnimeSeason::Spring, 2024)
///     .status(AnimeStatus::Finished)
///     .sort(AnimeSort::ScoreDesc)
///     .execute_tv()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[must_use = "a discover query does nothing until .execute_movies() or .execute_tv() is awaited"]
pub struct DiscoverAnimeBuilder<'a> {
    client: &'a AniListClient,
    genres: Vec<String>,
    excluded_genres: Vec<String>,
    country_of_origin: Option<String>,
    season: Option<AnimeSeason>,
    season_year: Option<i32>,
    formats: Vec<AnimeFormat>,
    status: Option<AnimeStatus>,
    is_adult: Option<bool>,
    sort: AnimeSort,
    page: Option<u32>,
}

impl<'a> DiscoverAnimeBuilder<'a> {
    pub(crate) fn new(client: &'a AniListClient) -> Self {
        Self {
            client,
            genres: Vec::new(),
            excluded_genres: Vec::new(),
            country_of_origin: None,
            season: None,
            season_year: None,
            formats: Vec::new(),
            status: None,
            is_adult: None,
            sort: AnimeSort::default(),
            page: None,
        }
    }

    /// Only include media with the given genres (AniList `genre_in`), e.g.
    /// `"Action"`, `"Slice of Life"`. Genre names are AniList's own,
    /// case-sensitive strings. Replaces any previously set genres.
    pub fn genres<I, S>(mut self, genres: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.genres = genres.into_iter().map(Into::into).collect();
        self
    }

    /// Exclude media with any of the given genres (AniList `genre_not_in`).
    /// Replaces any previously set excluded genres.
    pub fn exclude_genres<I, S>(mut self, genres: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.excluded_genres = genres.into_iter().map(Into::into).collect();
        self
    }

    /// Filter by country of origin, as an ISO 3166-1 alpha-2 code
    /// (e.g. `"JP"`, `"KR"`, `"CN"`). The code is upper-cased before sending.
    pub fn country_of_origin(mut self, code: impl Into<String>) -> Self {
        self.country_of_origin = Some(code.into().to_ascii_uppercase());
        self
    }

    /// Filter by broadcast season and year (e.g. Spring 2024).
    pub fn season(mut self, season: AnimeSeason, year: i32) -> Self {
        self.season = Some(season);
        self.season_year = Some(year);
        self
    }

    /// Filter by the year of the broadcast season only (AniList `seasonYear`),
    /// without restricting the season itself.
    pub fn season_year(mut self, year: i32) -> Self {
        self.season_year = Some(year);
        self
    }

    /// Restrict results to the given formats, overriding the default movie/TV
    /// format split (see [Format precedence](Self#format-precedence)). An empty
    /// slice clears the filter and restores the default.
    pub fn formats(mut self, formats: &[AnimeFormat]) -> Self {
        self.formats = formats.to_vec();
        self
    }

    /// Filter by release status.
    pub fn status(mut self, status: AnimeStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by the adult-content flag: `Some(false)` excludes adult media,
    /// `Some(true)` returns only adult media, `None` (the default) applies no
    /// filter.
    pub fn is_adult(mut self, is_adult: Option<bool>) -> Self {
        self.is_adult = is_adult;
        self
    }

    /// Set the sort order (default [`AnimeSort::PopularityDesc`]).
    pub fn sort(mut self, sort: AnimeSort) -> Self {
        self.sort = sort;
        self
    }

    /// Set the 1-indexed result page (values below 1 are clamped to 1).
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page.max(1));
        self
    }

    /// Run the query and convert the results to movies.
    ///
    /// Requests only the `MOVIE` format unless [`formats`](Self::formats) was
    /// set.
    ///
    /// # Errors
    ///
    /// Returns an [`AniListError`] on transport, GraphQL, or decoding failure.
    pub async fn execute_movies(self) -> Result<Page<UnifiedMovie>, AniListError> {
        let client = self.client;
        let vars = self.variables(AnimeFormat::MOVIES);
        client.discover_movies_page(vars).await
    }

    /// Run the query and convert the results to TV shows.
    ///
    /// Requests the `TV`, `TV_SHORT`, `ONA`, `OVA`, and `SPECIAL` formats unless
    /// [`formats`](Self::formats) was set.
    ///
    /// # Errors
    ///
    /// Returns an [`AniListError`] on transport, GraphQL, or decoding failure.
    pub async fn execute_tv(self) -> Result<Page<UnifiedTvShow>, AniListError> {
        let client = self.client;
        let vars = self.variables(AnimeFormat::TV);
        client.discover_tv_page(vars).await
    }

    /// Build the GraphQL variables object. Unset filters are omitted entirely
    /// (not sent as `null`) so AniList applies no filter for them.
    fn variables(&self, default_formats: &[AnimeFormat]) -> Value {
        let formats = if self.formats.is_empty() {
            default_formats
        } else {
            &self.formats
        };
        let mut vars = Map::new();
        vars.insert("page".into(), json!(self.page.unwrap_or(1)));
        vars.insert("perPage".into(), json!(self.client.config().per_page));
        vars.insert(
            "formatIn".into(),
            json!(formats.iter().map(|f| f.as_str()).collect::<Vec<_>>()),
        );
        vars.insert("sort".into(), json!([self.sort.as_str()]));
        if !self.genres.is_empty() {
            vars.insert("genreIn".into(), json!(self.genres));
        }
        if !self.excluded_genres.is_empty() {
            vars.insert("genreNotIn".into(), json!(self.excluded_genres));
        }
        if let Some(country) = &self.country_of_origin {
            vars.insert("countryOfOrigin".into(), json!(country));
        }
        if let Some(season) = self.season {
            vars.insert("season".into(), json!(season.as_str()));
        }
        if let Some(year) = self.season_year {
            vars.insert("seasonYear".into(), json!(year));
        }
        if let Some(status) = self.status {
            vars.insert("status".into(), json!(status.as_str()));
        }
        if let Some(is_adult) = self.is_adult {
            vars.insert("isAdult".into(), json!(is_adult));
        }
        Value::Object(vars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::anilist::AniListConfig;

    fn client() -> AniListClient {
        AniListClient::new(AniListConfig::new()).expect("client")
    }

    #[test]
    fn unset_filters_are_omitted() {
        let c = client();
        let vars = c.discover_anime().variables(AnimeFormat::TV);
        let obj = vars.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["formatIn", "page", "perPage", "sort"]);
        assert_eq!(vars["sort"], json!(["POPULARITY_DESC"]));
        assert_eq!(
            vars["formatIn"],
            json!(["TV", "TV_SHORT", "ONA", "OVA", "SPECIAL"])
        );
    }

    #[test]
    fn explicit_formats_override_defaults() {
        let c = client();
        let vars = c
            .discover_anime()
            .formats(&[AnimeFormat::Music])
            .variables(AnimeFormat::MOVIES);
        assert_eq!(vars["formatIn"], json!(["MUSIC"]));
        let cleared = c
            .discover_anime()
            .formats(&[AnimeFormat::Music])
            .formats(&[])
            .variables(AnimeFormat::MOVIES);
        assert_eq!(cleared["formatIn"], json!(["MOVIE"]));
    }

    #[test]
    fn enum_values_match_anilist_schema() {
        assert_eq!(AnimeSeason::Fall.as_str(), "FALL");
        assert_eq!(AnimeStatus::NotYetReleased.as_str(), "NOT_YET_RELEASED");
        assert_eq!(AnimeFormat::TvShort.as_str(), "TV_SHORT");
        assert_eq!(AnimeSort::StartDateAsc.as_str(), "START_DATE");
    }
}
