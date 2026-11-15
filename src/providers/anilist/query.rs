//! GraphQL query string constants for the AniList API (GraphQL v2 schema).
//!
//! AniList does not support named fragments across request boundaries, so every
//! query returning list-shaped media (search / trending / popular / top-scored /
//! discover / recommendations) shares a single media selection defined **once**
//! in [`anime_media_selection`]; the paginated list queries are built by
//! [`anime_list_query`] and differ only in their operation name, variable list,
//! and `media(...)` arguments. Only fields that are actually converted are
//! requested — no dead queried fields.

/// The shared media field selection (the body of a `media { … }` block) used
/// by every query whose results are converted with
/// `anilist_media_to_movie` / `anilist_media_to_tv`.
macro_rules! anime_media_selection {
    () => {
        concat!(
            "      id\n",
            "      title { romaji english native }\n",
            "      synonyms\n",
            "      description(asHtml: false)\n",
            "      startDate { year month day }\n",
            "      coverImage { large extraLarge }\n",
            "      bannerImage\n",
            "      genres\n",
            "      popularity\n",
            "      averageScore\n",
            "      episodes\n",
            "      duration\n",
            "      status\n",
            "      format\n",
            "      countryOfOrigin\n",
            "      isAdult\n",
        )
    };
}

/// Build a paginated ANIME list query with the shared media selection.
///
/// The `pageInfo` and `media { … }` field selections are written exactly once
/// here; each query const below supplies its operation name, variables, and
/// `media(...)` arguments.
macro_rules! anime_list_query {
    ($op:literal, $vars:literal, $media_args:literal) => {
        concat!(
            "query ",
            $op,
            $vars,
            " {\n",
            "  Page(page: $page, perPage: $perPage) {\n",
            "    pageInfo { total currentPage lastPage hasNextPage }\n",
            "    media(",
            $media_args,
            ") {\n",
            anime_media_selection!(),
            "    }\n",
            "  }\n",
            "}\n"
        )
    };
}

/// Search anime with an optional `formatIn` filter.
pub const SEARCH_ANIME: &str = anime_list_query!(
    "SearchAnime",
    "($query: String, $page: Int, $perPage: Int, $formatIn: [MediaFormat])",
    "search: $query, type: ANIME, format_in: $formatIn, sort: [SEARCH_MATCH, POPULARITY_DESC]"
);

/// List trending anime (`TRENDING_DESC`) with an optional format filter.
pub const LIST_TRENDING_ANIME: &str = anime_list_query!(
    "ListTrendingAnime",
    "($page: Int, $perPage: Int, $formatIn: [MediaFormat])",
    "type: ANIME, format_in: $formatIn, sort: [TRENDING_DESC]"
);

/// List popular anime (`POPULARITY_DESC`) with an optional format filter.
pub const LIST_POPULAR_ANIME: &str = anime_list_query!(
    "ListPopularAnime",
    "($page: Int, $perPage: Int, $formatIn: [MediaFormat])",
    "type: ANIME, format_in: $formatIn, sort: [POPULARITY_DESC]"
);

/// List top-scored anime (`SCORE_DESC`) with an optional format filter.
pub const LIST_TOP_SCORED_ANIME: &str = anime_list_query!(
    "ListTopScoredAnime",
    "($page: Int, $perPage: Int, $formatIn: [MediaFormat])",
    "type: ANIME, format_in: $formatIn, sort: [SCORE_DESC]"
);

/// Discover anime with the filters of
/// [`DiscoverAnimeBuilder`](super::discover::DiscoverAnimeBuilder).
///
/// Every filter is a GraphQL variable (user input is never interpolated into
/// the query text). Unset filters are omitted from the variables object, which
/// AniList treats as "no filter".
pub const DISCOVER_ANIME: &str = anime_list_query!(
    "DiscoverAnime",
    "($page: Int, $perPage: Int, $formatIn: [MediaFormat], $genreIn: [String], \
     $genreNotIn: [String], $countryOfOrigin: CountryCode, $season: MediaSeason, \
     $seasonYear: Int, $status: MediaStatus, $isAdult: Boolean, $sort: [MediaSort])",
    "type: ANIME, format_in: $formatIn, genre_in: $genreIn, genre_not_in: $genreNotIn, \
     countryOfOrigin: $countryOfOrigin, season: $season, seasonYear: $seasonYear, \
     status: $status, isAdult: $isAdult, sort: $sort"
);

/// Fetch a page of community recommendations for an anime, highest-rated
/// first. Each recommended media uses the shared list selection.
///
/// Variables: `$id: Int!`, `$page: Int`, `$perPage: Int`.
pub const MEDIA_RECOMMENDATIONS: &str = concat!(
    "query MediaRecommendations($id: Int!, $page: Int, $perPage: Int) {\n",
    "  Media(id: $id, type: ANIME) {\n",
    "    recommendations(page: $page, perPage: $perPage, sort: [RATING_DESC]) {\n",
    "      pageInfo { total currentPage lastPage hasNextPage }\n",
    "      nodes {\n",
    "        mediaRecommendation {\n",
    anime_media_selection!(),
    "        }\n",
    "      }\n",
    "    }\n",
    "  }\n",
    "}\n"
);

/// Fetch full details for a single anime by AniList ID.
///
/// Variables: `$id: Int!`.
pub const MEDIA_DETAILS: &str = r#"
query MediaDetails($id: Int!) {
  Media(id: $id, type: ANIME) {
    id
    title { romaji english native }
    synonyms
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
    studios(isMain: true) {
      nodes { name }
    }
  }
}
"#;

/// Fetch only the ids of a single anime by AniList ID — a cheap query for
/// external-id lookups that skips the full detail selection.
///
/// Variables: `$id: Int!`.
pub const MEDIA_EXTERNAL_IDS: &str = r#"
query MediaExternalIds($id: Int!) {
  Media(id: $id, type: ANIME) {
    id
    idMal
  }
}
"#;

/// Find a single anime by its MyAnimeList ID, returning its AniList id and
/// format (to tell movies from series).
///
/// Variables: `$idMal: Int!`.
pub const MEDIA_BY_MAL_ID: &str = r#"
query MediaByMalId($idMal: Int!) {
  Media(idMal: $idMal, type: ANIME) {
    id
    format
  }
}
"#;

/// Fetch one page of an anime's characters (with their Japanese voice actors)
/// and one page of its staff, by AniList ID.
///
/// Both connections are paged by their own variable so a caller can walk each
/// until its `hasNextPage` is false. Characters sort main roles first
/// (`ROLE`), then by relevance; staff sort by relevance.
///
/// Variables: `$id: Int!`, `$characterPage: Int`, `$staffPage: Int`,
/// `$perPage: Int`.
pub const MEDIA_CREDITS: &str = r#"
query MediaCredits($id: Int!, $characterPage: Int, $staffPage: Int, $perPage: Int) {
  Media(id: $id, type: ANIME) {
    id
    characters(page: $characterPage, perPage: $perPage, sort: [ROLE, RELEVANCE, ID]) {
      pageInfo { hasNextPage }
      edges {
        node { name { full } }
        voiceActors(language: JAPANESE, sort: [RELEVANCE, ID]) {
          id
          name { full }
          image { large }
        }
      }
    }
    staff(page: $staffPage, perPage: $perPage, sort: [RELEVANCE, ID]) {
      pageInfo { hasNextPage }
      edges {
        role
        node {
          id
          name { full }
          image { large }
        }
      }
    }
  }
}
"#;

/// Search staff (real people) by name.
///
/// Variables: `$query: String`, `$page: Int`, `$perPage: Int`.
pub const SEARCH_STAFF: &str = r#"
query SearchStaff($query: String, $page: Int, $perPage: Int) {
  Page(page: $page, perPage: $perPage) {
    pageInfo { total currentPage lastPage hasNextPage }
    staff(search: $query, sort: [SEARCH_MATCH]) {
      id
      name { full native }
      image { large }
      primaryOccupations
    }
  }
}
"#;

/// Fetch full details for a single staff member by AniList ID.
///
/// Variables: `$id: Int!`.
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
  }
}
"#;

#[cfg(test)]
mod tests;
