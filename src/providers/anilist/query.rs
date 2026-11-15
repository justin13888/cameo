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
        ("MEDIA_EXTERNAL_IDS", MEDIA_EXTERNAL_IDS),
        ("MEDIA_BY_MAL_ID", MEDIA_BY_MAL_ID),
        ("MEDIA_CREDITS", MEDIA_CREDITS),
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

    /// The body of the first `{ … }` block opening at or after `from`,
    /// found by brace matching (independent of indentation and line breaks).
    fn block_after(q: &str, from: usize) -> &str {
        let open = q[from..].find('{').expect("a selection block") + from;
        let mut depth = 0usize;
        for (i, c) in q[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &q[open + 1..open + i];
                    }
                }
                _ => {}
            }
        }
        panic!("unterminated selection block");
    }

    /// The body of the selection block following the first `marker`.
    fn block_of<'q>(q: &'q str, marker: &str) -> &'q str {
        let at = q
            .find(marker)
            .unwrap_or_else(|| panic!("no {marker} in query"));
        block_after(q, at)
    }

    /// The `media { … }` selection body of a list query.
    fn selection(q: &str) -> &str {
        block_of(q, "media(")
    }

    /// Selection text with whitespace normalized, so two selections compare
    /// equal whatever their layout.
    fn normalized(sel: &str) -> String {
        sel.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The field names a selection requests, at every nesting level, in
    /// order. Argument lists (`(…)`) are skipped, so variables and filter
    /// arguments are never mistaken for selected fields.
    fn field_tokens(sel: &str) -> Vec<String> {
        fn flush(cur: &mut String, out: &mut Vec<String>) {
            if !cur.is_empty() {
                out.push(std::mem::take(cur));
            }
        }
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut args_depth = 0usize;
        for c in sel.chars() {
            match c {
                '(' => {
                    flush(&mut cur, &mut out);
                    args_depth += 1;
                }
                ')' => args_depth = args_depth.saturating_sub(1),
                _ if args_depth > 0 => {}
                c if c.is_ascii_alphanumeric() || c == '_' => cur.push(c),
                _ => flush(&mut cur, &mut out),
            }
        }
        flush(&mut cur, &mut out);
        out
    }

    /// The field names selected anywhere in a whole query operation.
    fn query_fields(q: &str) -> Vec<String> {
        field_tokens(block_after(q, 0))
    }

    fn has_field(fields: &[String], name: &str) -> bool {
        fields.iter().any(|f| f == name)
    }

    /// `season`-family fields (`season`, `seasonYear`, `seasonInt`, …).
    fn has_season_field(fields: &[String]) -> bool {
        fields.iter().any(|f| f.starts_with("season"))
    }

    #[test]
    fn field_tokens_skip_arguments_and_ignore_layout() {
        let sel = "id\n  title { romaji english }\n description(asHtml: false) staff(sort: [ROLE]) { id }";
        assert_eq!(
            field_tokens(sel),
            [
                "id",
                "title",
                "romaji",
                "english",
                "description",
                "staff",
                "id"
            ]
        );
        let compact = "media(season: $season) { id seasonYear }";
        assert_eq!(
            field_tokens(block_of(compact, "media(")),
            ["id", "seasonYear"]
        );
        assert_eq!(normalized(" a  {\n b }\n"), "a { b }");
        // The absence checks below are not vacuous: selected fields are seen.
        assert!(has_field(&query_fields(MEDIA_EXTERNAL_IDS), "idMal"));
        assert!(has_field(&query_fields(MEDIA_DETAILS), "endDate"));
        assert!(has_season_field(&field_tokens("seasonYear")));
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
        let first = normalized(selection(LIST_QUERIES[0]));
        for q in &LIST_QUERIES[1..] {
            assert_eq!(
                normalized(selection(q)),
                first,
                "list media selections diverged"
            );
        }
        // The shared selection requests exactly the fields the conversions use.
        let fields = field_tokens(selection(LIST_QUERIES[0]));
        for field in [
            "id",
            "title",
            "synonyms",
            "startDate",
            "coverImage",
            "genres",
            "episodes",
            "duration",
            "status",
            "format",
        ] {
            assert!(has_field(&fields, field), "missing {field}");
        }
    }

    /// Every media query requests all title variants plus `synonyms`, which
    /// the conversions map into `alternative_titles`.
    #[test]
    fn media_queries_request_all_title_variants() {
        for q in LIST_QUERIES
            .iter()
            .chain([&MEDIA_RECOMMENDATIONS, &MEDIA_DETAILS])
        {
            assert!(q.contains("title { romaji english native }"));
            assert!(q.contains("synonyms"), "media query must request synonyms");
        }
    }

    /// Recommended media are requested with the same selection as the list
    /// queries, so they convert identically.
    #[test]
    fn recommendations_reuse_the_list_media_selection() {
        let list = normalized(selection(LIST_QUERIES[0]));
        assert_eq!(
            normalized(block_of(MEDIA_RECOMMENDATIONS, "mediaRecommendation")),
            list,
            "recommendations selection diverged from the list selection"
        );
        assert!(MEDIA_RECOMMENDATIONS.contains("sort: [RATING_DESC]"));
        let fields = query_fields(MEDIA_RECOMMENDATIONS);
        for dead in ["endDate", "studios"] {
            assert!(!has_field(&fields, dead), "dead field {dead}");
        }
        assert!(!has_season_field(&fields), "dead field season*");
    }

    /// The credits query pages characters and staff independently and selects
    /// exactly the fields the credits conversion reads.
    #[test]
    fn credits_query_pages_each_connection_and_selects_voice_actors() {
        assert!(MEDIA_CREDITS.contains("characters(page: $characterPage, perPage: $perPage"));
        assert!(MEDIA_CREDITS.contains("staff(page: $staffPage, perPage: $perPage"));
        assert_eq!(MEDIA_CREDITS.matches("pageInfo { hasNextPage }").count(), 2);
        assert!(MEDIA_CREDITS.contains("voiceActors(language: JAPANESE"));
        assert!(MEDIA_CREDITS.contains("role"));
        assert_eq!(MEDIA_CREDITS.matches("image { large }").count(), 2);
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
        // The list selection requests exactly the fields `AniListMedia`
        // deserializes and the list conversions map — `episodes`, `duration`
        // and `status` are now live (#67), while detail-only fields
        // (`endDate`, `studios`) stay out of list queries.
        for q in LIST_QUERIES {
            // Inspect only selected fields: `season` is a legitimate *filter
            // argument* of the discover query, which `field_tokens` skips.
            let fields = field_tokens(selection(q));
            for dead in ["endDate", "studios"] {
                assert!(
                    !has_field(&fields, dead),
                    "list query requests dead field {dead}"
                );
            }
            assert!(
                !has_season_field(&fields),
                "list query requests dead field season*"
            );
        }
        assert!(
            !has_season_field(&query_fields(MEDIA_DETAILS)),
            "details still requests season*"
        );
        // `idMal` is only read by the external-id lookup, never by the list or
        // detail conversions (`MEDIA_BY_MAL_ID` takes it as an argument only).
        for q in LIST_QUERIES
            .iter()
            .chain([&MEDIA_DETAILS, &MEDIA_BY_MAL_ID])
        {
            assert!(
                !has_field(&query_fields(q), "idMal"),
                "query selects unused idMal"
            );
        }
        assert!(
            !has_field(&query_fields(SEARCH_STAFF), "languageV2"),
            "staff search requests languageV2"
        );
        // The `pageInfo` selection must not request the unused `perPage` field
        // (the `$perPage` pagination *variable* is still required, so we only
        // inspect the pageInfo blocks).
        for (name, q) in ALL {
            for (at, _) in q.match_indices("pageInfo") {
                assert!(
                    !has_field(&field_tokens(block_after(q, at)), "perPage"),
                    "{name}: pageInfo still selects perPage"
                );
            }
        }
    }
}
