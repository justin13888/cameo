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
            "      description(asHtml: false)\n",
            "      startDate { year month day }\n",
            "      coverImage { large extraLarge }\n",
            "      bannerImage\n",
            "      genres\n",
            "      popularity\n",
            "      averageScore\n",
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
mod tests {
    use super::*;

    const ALL: &[(&str, &str)] = &[
        ("SEARCH_ANIME", SEARCH_ANIME),
        ("LIST_TRENDING_ANIME", LIST_TRENDING_ANIME),
        ("LIST_POPULAR_ANIME", LIST_POPULAR_ANIME),
        ("LIST_TOP_SCORED_ANIME", LIST_TOP_SCORED_ANIME),
        ("DISCOVER_ANIME", DISCOVER_ANIME),
        ("MEDIA_RECOMMENDATIONS", MEDIA_RECOMMENDATIONS),
        ("MEDIA_DETAILS", MEDIA_DETAILS),
        ("SEARCH_STAFF", SEARCH_STAFF),
        ("STAFF_DETAILS", STAFF_DETAILS),
    ];

    const LIST_QUERIES: &[&str] = &[
        SEARCH_ANIME,
        LIST_TRENDING_ANIME,
        LIST_POPULAR_ANIME,
        LIST_TOP_SCORED_ANIME,
        DISCOVER_ANIME,
    ];

    /// The `media { … }` selection body of a list query.
    fn selection(q: &str) -> String {
        let start = q.find("media(").expect("media selection");
        let brace = q[start..].find(") {").expect("media body") + start + 2;
        let end = q[brace..].find("\n    }").expect("media body end") + brace;
        q[brace..end].to_string()
    }

    /// Every query is well-formed GraphQL: balanced braces/parens and a single
    /// top-level `query` operation.
    #[test]
    fn queries_are_well_formed() {
        for (name, q) in ALL {
            let braces = q.chars().filter(|&c| c == '{').count();
            let close = q.chars().filter(|&c| c == '}').count();
            assert_eq!(braces, close, "{name}: unbalanced braces");
            let parens = q.chars().filter(|&c| c == '(').count();
            let cparens = q.chars().filter(|&c| c == ')').count();
            assert_eq!(parens, cparens, "{name}: unbalanced parens");
            assert_eq!(q.matches("query ").count(), 1, "{name}: not one operation");
        }
    }

    /// The list queries share the exact same media field selection — the
    /// point of the shared macro.
    #[test]
    fn list_queries_share_one_media_selection() {
        let first = selection(LIST_QUERIES[0]);
        for q in &LIST_QUERIES[1..] {
            assert_eq!(selection(q), first, "list media selections diverged");
        }
        // The shared selection requests exactly the fields the conversions use.
        for field in ["id", "title", "startDate", "coverImage", "genres", "format"] {
            assert!(first.contains(field), "missing {field}");
        }
    }

    /// Recommended media are requested with the same selection as the list
    /// queries, so they convert identically.
    #[test]
    fn recommendations_reuse_the_list_media_selection() {
        let list = selection(LIST_QUERIES[0]);
        assert!(
            MEDIA_RECOMMENDATIONS.contains(&list),
            "recommendations selection diverged from the list selection"
        );
        assert!(MEDIA_RECOMMENDATIONS.contains("sort: [RATING_DESC]"));
        for dead in ["episodes", "duration", "status"] {
            assert!(!MEDIA_RECOMMENDATIONS.contains(dead), "dead field {dead}");
        }
    }

    /// The discover query passes every filter as a variable — no filter value
    /// is ever interpolated into the query text.
    #[test]
    fn discover_query_uses_variables_for_every_filter() {
        for (var, arg) in [
            ("$formatIn: [MediaFormat]", "format_in: $formatIn"),
            ("$genreIn: [String]", "genre_in: $genreIn"),
            ("$genreNotIn: [String]", "genre_not_in: $genreNotIn"),
            (
                "$countryOfOrigin: CountryCode",
                "countryOfOrigin: $countryOfOrigin",
            ),
            ("$season: MediaSeason", "season: $season"),
            ("$seasonYear: Int", "seasonYear: $seasonYear"),
            ("$status: MediaStatus", "status: $status"),
            ("$isAdult: Boolean", "isAdult: $isAdult"),
            ("$sort: [MediaSort]", "sort: $sort"),
        ] {
            assert!(DISCOVER_ANIME.contains(var), "missing variable {var}");
            assert!(DISCOVER_ANIME.contains(arg), "missing argument {arg}");
        }
        assert!(DISCOVER_ANIME.contains("type: ANIME"));
    }

    /// No query requests fields the conversions never read.
    #[test]
    fn no_dead_queried_fields() {
        for q in LIST_QUERIES {
            // Inspect only the field selection: `status` is a legitimate
            // *filter argument* of the discover query.
            let fields = selection(q);
            for dead in ["episodes", "duration", "status"] {
                assert!(
                    !fields.contains(dead),
                    "list query still requests dead field {dead}"
                );
            }
        }
        assert!(
            !MEDIA_DETAILS.contains("season"),
            "details still requests season*"
        );
        assert!(
            !SEARCH_STAFF.contains("languageV2"),
            "staff search requests languageV2"
        );
        // The `pageInfo` selection must not request the unused `perPage` field
        // (the `$perPage` pagination *variable* is still required, so we only
        // inspect the pageInfo block).
        for (name, q) in ALL {
            if let Some(start) = q.find("pageInfo {") {
                let end = q[start..].find('}').expect("pageInfo body") + start;
                assert!(
                    !q[start..end].contains("perPage"),
                    "{name}: pageInfo still selects perPage"
                );
            }
        }
    }
}
