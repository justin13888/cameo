//! GraphQL query string constants for the AniList API (GraphQL v2 schema).
//!
//! AniList does not support named fragments across request boundaries, so the
//! four list queries (search / trending / popular / top-scored) share a single
//! media selection defined **once** in [`anime_list_query`]; each query differs
//! only in its operation name, variable list, and `media(...)` arguments. Only
//! fields that are actually converted are requested — no dead queried fields.

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
            "      id\n",
            "      title { romaji english native }\n",
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
        ("MEDIA_DETAILS", MEDIA_DETAILS),
        ("SEARCH_STAFF", SEARCH_STAFF),
        ("STAFF_DETAILS", STAFF_DETAILS),
    ];

    const LIST_QUERIES: &[&str] = &[
        SEARCH_ANIME,
        LIST_TRENDING_ANIME,
        LIST_POPULAR_ANIME,
        LIST_TOP_SCORED_ANIME,
    ];

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

    /// The four list queries share the exact same media field selection — the
    /// point of the shared macro.
    #[test]
    fn list_queries_share_one_media_selection() {
        let selection = |q: &str| {
            let start = q.find("media(").expect("media selection");
            let brace = q[start..].find('{').expect("media body") + start;
            let end = q[brace..].find("\n  }").expect("media body end") + brace;
            q[brace..end].to_string()
        };
        let first = selection(LIST_QUERIES[0]);
        for q in &LIST_QUERIES[1..] {
            assert_eq!(selection(q), first, "list media selections diverged");
        }
        // The shared selection requests exactly the fields the conversions use.
        for field in [
            "id",
            "title",
            "startDate",
            "coverImage",
            "genres",
            "episodes",
            "duration",
            "status",
            "format",
        ] {
            assert!(first.contains(field), "missing {field}");
        }
    }

    /// No query requests fields the conversions never read.
    #[test]
    fn no_dead_queried_fields() {
        // The list selection requests exactly the fields `AniListMedia`
        // deserializes and the list conversions map — `episodes`, `duration`
        // and `status` are now live (#67), while detail-only fields
        // (`endDate`, `studios`) stay out of list queries.
        for q in LIST_QUERIES {
            for dead in ["endDate", "studios", "season"] {
                assert!(!q.contains(dead), "list query requests dead field {dead}");
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
