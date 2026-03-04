//! GraphQL query string constants for the AniList API.
//!
//! All queries target the AniList GraphQL v2 schema.
//! Media fields are kept identical across queries for consistent deserialization.

// ── Shared field fragments (inlined into each query) ──────────────────────────
//
// AniList does not support named fragments across request boundaries, so we
// inline the same field list into every query constant.

/// Search anime with an optional `formatIn` filter.
///
/// Variables:
/// - `$query: String` — search string
/// - `$page: Int`
/// - `$perPage: Int`
/// - `$formatIn: [MediaFormat]` — e.g. `["MOVIE"]`, or `null` for any format
pub const SEARCH_ANIME: &str = r#"
query SearchAnime($query: String, $page: Int, $perPage: Int, $formatIn: [MediaFormat]) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      currentPage
      lastPage
      hasNextPage
      perPage
    }
    media(search: $query, type: ANIME, format_in: $formatIn, sort: [SEARCH_MATCH, POPULARITY_DESC]) {
      id
      title { romaji english native }
      description(asHtml: false)
      startDate { year month day }
      coverImage { large extraLarge }
      bannerImage
      genres
      popularity
      averageScore
      episodes
      duration
      status
      format
      countryOfOrigin
      isAdult
    }
  }
}
"#;

/// List trending anime (sorted by `TRENDING_DESC`) with an optional format filter.
///
/// Variables:
/// - `$page: Int`
/// - `$perPage: Int`
/// - `$formatIn: [MediaFormat]`
pub const LIST_TRENDING_ANIME: &str = r#"
query ListTrendingAnime($page: Int, $perPage: Int, $formatIn: [MediaFormat]) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      currentPage
      lastPage
      hasNextPage
      perPage
    }
    media(type: ANIME, format_in: $formatIn, sort: [TRENDING_DESC]) {
      id
      title { romaji english native }
      description(asHtml: false)
      startDate { year month day }
      coverImage { large extraLarge }
      bannerImage
      genres
      popularity
      averageScore
      episodes
      duration
      status
      format
      countryOfOrigin
      isAdult
    }
  }
}
"#;

/// List popular anime (sorted by `POPULARITY_DESC`) with an optional format filter.
///
/// Variables:
/// - `$page: Int`
/// - `$perPage: Int`
/// - `$formatIn: [MediaFormat]`
pub const LIST_POPULAR_ANIME: &str = r#"
query ListPopularAnime($page: Int, $perPage: Int, $formatIn: [MediaFormat]) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      currentPage
      lastPage
      hasNextPage
      perPage
    }
    media(type: ANIME, format_in: $formatIn, sort: [POPULARITY_DESC]) {
      id
      title { romaji english native }
      description(asHtml: false)
      startDate { year month day }
      coverImage { large extraLarge }
      bannerImage
      genres
      popularity
      averageScore
      episodes
      duration
      status
      format
      countryOfOrigin
      isAdult
    }
  }
}
"#;

/// List top-scored anime (sorted by `SCORE_DESC`) with an optional format filter.
///
/// Variables:
/// - `$page: Int`
/// - `$perPage: Int`
/// - `$formatIn: [MediaFormat]`
pub const LIST_TOP_SCORED_ANIME: &str = r#"
query ListTopScoredAnime($page: Int, $perPage: Int, $formatIn: [MediaFormat]) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      currentPage
      lastPage
      hasNextPage
      perPage
    }
    media(type: ANIME, format_in: $formatIn, sort: [SCORE_DESC]) {
      id
      title { romaji english native }
      description(asHtml: false)
      startDate { year month day }
      coverImage { large extraLarge }
      bannerImage
      genres
      popularity
      averageScore
      episodes
      duration
      status
      format
      countryOfOrigin
      isAdult
    }
  }
}
"#;

/// Fetch full details for a single anime by AniList ID.
///
/// Variables:
/// - `$id: Int!`
pub const MEDIA_DETAILS: &str = r#"
query MediaDetails($id: Int!) {
  Media(id: $id, type: ANIME) {
    id
    title { romaji english native }
    description(asHtml: false)
    startDate { year month day }
    endDate { year month day }
    coverImage { large extraLarge }
    bannerImage
    genres
    popularity
    averageScore
    episodes
    duration
    status
    format
    countryOfOrigin
    isAdult
    season
    seasonYear
    studios(isMain: true) {
      nodes { name }
    }
  }
}
"#;

/// Search staff (real people) by name.
///
/// Variables:
/// - `$query: String`
/// - `$page: Int`
/// - `$perPage: Int`
pub const SEARCH_STAFF: &str = r#"
query SearchStaff($query: String, $page: Int, $perPage: Int) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      currentPage
      lastPage
      hasNextPage
      perPage
    }
    staff(search: $query, sort: [SEARCH_MATCH, RELEVANCE]) {
      id
      name { full native }
      image { large }
      description
      primaryOccupations
      languageV2
    }
  }
}
"#;

/// Fetch full details for a single staff member by AniList ID.
///
/// Variables:
/// - `$id: Int!`
pub const STAFF_DETAILS: &str = r#"
query StaffDetails($id: Int!) {
  Staff(id: $id) {
    id
    name { full native alternative }
    image { large }
    description
    primaryOccupations
    gender
    dateOfBirth { year month day }
    dateOfDeath { year month day }
    homeTown
    siteUrl
    languageV2
  }
}
"#;
