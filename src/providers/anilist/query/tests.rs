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
    let sel =
        "id\n  title { romaji english }\n description(asHtml: false) staff(sort: [ROLE]) { id }";
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
