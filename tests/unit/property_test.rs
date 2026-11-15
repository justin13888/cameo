//! Property tests over the pure, round-trippable conversion helpers (#51).
//!
//! These complement the fixture-driven snapshot tests in `tests/integration/`
//! (each `tmdb_*`/`anilist_*` test deserializes a committed JSON fixture and
//! asserts the converted unified values) by checking algebraic invariants over
//! generated inputs.

use cameo::{Genre, MediaId, NativeId, PartialDate, UnknownGenre};
use proptest::prelude::*;

proptest! {
    /// A fully-specified date survives Display -> FromStr unchanged.
    #[test]
    fn partial_date_full_round_trips(y in any::<i32>(), m in 1u8..=12, d in 1u8..=31) {
        let date = PartialDate::ymd(y, m, d).expect("valid ymd");
        prop_assert_eq!(date.to_string().parse::<PartialDate>().unwrap(), date);
    }

    /// Every precision, including negative years, survives a JSON round-trip.
    #[test]
    fn partial_date_json_round_trips(
        y in any::<i32>(),
        m in proptest::option::of(1u8..=12),
        d in 1u8..=31,
        with_day in any::<bool>(),
    ) {
        let day = m.and(with_day.then_some(d));
        let date = PartialDate::new(y, m, day).expect("valid components");
        let json = serde_json::to_string(&date).expect("serialize");
        prop_assert_eq!(serde_json::from_str::<PartialDate>(&json).expect("deserialize"), date);
    }

    /// A year-month date stays partial across the round-trip (no day padding).
    #[test]
    fn partial_date_year_month_round_trips(y in any::<i32>(), m in 1u8..=12) {
        let date = PartialDate::year_month(y, m).expect("valid ym");
        let back = date.to_string().parse::<PartialDate>().unwrap();
        prop_assert_eq!(back, date);
        prop_assert_eq!(back.day(), None);
    }

    /// A year-only date stays year-only.
    #[test]
    fn partial_date_year_only_round_trips(y in any::<i32>()) {
        let date = PartialDate::from_year(y);
        let back = date.to_string().parse::<PartialDate>().unwrap();
        prop_assert_eq!(back, date);
        prop_assert_eq!(back.month(), None);
    }

    /// A provider-qualified id survives Display -> parse unchanged.
    #[test]
    fn media_id_numeric_round_trips(n in any::<u64>()) {
        let id = MediaId::tmdb(n);
        prop_assert_eq!(MediaId::parse(&id.to_string()).unwrap(), id);
    }

    /// Genre mapping and display are total: no id panics, and an unknown id is
    /// never lost (it either maps to a known variant or is preserved in Other).
    #[test]
    fn genre_from_tmdb_id_is_total(id in any::<i64>()) {
        let genre = Genre::from_tmdb_id(id);
        let name = genre.name();
        prop_assert!(!name.is_empty());
        prop_assert_eq!(genre.to_string(), name.to_string());
        // Round-trip: converting the id again yields an equal genre.
        prop_assert_eq!(Genre::from_tmdb_id(id), genre);
    }

    /// An id maps either to `Other(TmdbId(id))` with the id preserved
    /// verbatim, or to a known genre whose own TMDB id is that id.
    #[test]
    fn genre_from_tmdb_id_preserves_or_recognises_the_id(id in prop_oneof![
        any::<i64>(),
        prop::sample::select(KNOWN_TMDB_GENRE_IDS.to_vec()),
    ]) {
        match Genre::from_tmdb_id(id) {
            Genre::Other(UnknownGenre::TmdbId(x)) => {
                prop_assert_eq!(x, id);
                prop_assert!(!KNOWN_TMDB_GENRE_IDS.contains(&id));
            }
            known => {
                prop_assert!(KNOWN_TMDB_GENRE_IDS.contains(&id), "{id} mapped to {known:?}");
                prop_assert_eq!(known.tmdb_id(), u64::try_from(id).ok());
            }
        }
    }

    /// A canonical media id survives a serde JSON round-trip and serializes as
    /// its `provider:native` string.
    #[test]
    fn media_id_serde_round_trips(id in canonical_media_id()) {
        let json = serde_json::to_string(&id).expect("serialize");
        prop_assert_eq!(&json, &format!("\"{id}\""));
        let back: MediaId = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(back, id);
    }

    /// Any parseable string displays back as itself, and re-parsing that
    /// rendering yields the same id: parsing never normalises a native part.
    #[test]
    fn media_id_parse_display_round_trips(s in "[a-z]{1,4}:[+0-9a-z:]{1,12}") {
        let id = MediaId::parse(&s).expect("provider and native are non-empty");
        prop_assert_eq!(id.to_string(), s.clone());
        prop_assert_eq!(MediaId::parse(&id.to_string()).unwrap(), id);
    }

    /// A media id's string form parses back to the same id for every
    /// canonical id, including text natives that look like a non-canonical
    /// number.
    #[test]
    fn media_id_canonical_display_parse_round_trips(id in canonical_media_id()) {
        prop_assert_eq!(MediaId::parse(&id.to_string()).unwrap(), id);
    }

    /// `id == s` holds exactly when `id.to_string() == s`. The small alphabet
    /// makes matches, sign and leading-zero near misses all common.
    #[test]
    fn media_id_eq_str_matches_display(
        id in small_media_id(),
        s in "[ab]{0,1}:{0,2}[+0-9a]{0,3}",
    ) {
        prop_assert_eq!(id == *s.as_str(), id.to_string() == s);
        prop_assert!(id == *id.to_string().as_str());
    }
}

/// Media ids over a deliberately tiny alphabet, so generated strings often
/// equal or nearly equal their rendering.
fn small_media_id() -> impl Strategy<Value = MediaId> {
    prop_oneof![
        ("[ab]", 0u64..12).prop_map(|(provider, n)| MediaId::new(provider, NativeId::Num(n))),
        ("[ab]", "[+0-9a:]{1,3}")
            .prop_map(|(provider, text)| MediaId::new(provider, NativeId::Text(text))),
    ]
}

/// Every TMDB genre id `Genre::from_tmdb_id` recognises.
const KNOWN_TMDB_GENRE_IDS: [i64; 27] = [
    28, 12, 14, 36, 27, 10402, 10749, 878, 53, 10770, 10752, 10759, 10762, 10763, 10764, 10765,
    10766, 10767, 10768, 16, 35, 80, 99, 18, 10751, 9648, 37,
];

/// Media ids whose string form parses back to the same id: numeric natives,
/// AniList staff ids, text natives that do not read as a number, and text
/// natives that read as a non-canonical number (a sign or a leading zero).
fn canonical_media_id() -> impl Strategy<Value = MediaId> {
    prop_oneof![
        ("[a-z]{1,8}", "(\\+[0-9]{1,6}|0[0-9]{1,6})")
            .prop_map(|(provider, text)| MediaId::new(provider, NativeId::Text(text))),
        any::<u64>().prop_map(MediaId::tmdb),
        any::<u64>().prop_map(MediaId::anilist),
        any::<u64>().prop_map(MediaId::anilist_staff),
        ("[a-z]{1,8}", any::<u64>())
            .prop_map(|(provider, n)| MediaId::new(provider, NativeId::Num(n))),
        ("[a-z]{1,8}", "[a-z][a-z0-9:_-]{0,12}")
            .prop_map(|(provider, text)| MediaId::new(provider, NativeId::Text(text))),
    ]
}

#[cfg(feature = "cache")]
mod cache_keys {
    use cameo::cache::{CacheKey, MediaType};
    use proptest::prelude::*;

    fn media_type() -> impl Strategy<Value = Option<MediaType>> {
        prop_oneof![
            Just(None),
            Just(Some(MediaType::Movie)),
            Just(Some(MediaType::TvShow)),
            Just(Some(MediaType::Person)),
        ]
    }

    fn search(provider: &str, media_type: Option<MediaType>, query: &str, page: u32) -> CacheKey {
        CacheKey::Search {
            provider: provider.to_string(),
            media_type,
            query: query.to_string(),
            page,
        }
    }

    /// The query component of a search key id (`provider:type:query:page`).
    fn query_part(key: &CacheKey) -> String {
        let id = key.key_id();
        let parts: Vec<&str> = id.split(':').collect();
        parts[2..parts.len() - 1].join(":")
    }

    proptest! {
        /// Normalizing a query twice changes nothing: re-keying with the
        /// already-normalized query yields the same key id.
        #[test]
        fn search_query_normalization_is_idempotent(
            provider in "[a-z]{1,8}",
            media_type in media_type(),
            query in "[ \tA-Za-z0-9ÀÉÎõü]{0,16}",
            page in any::<u32>(),
        ) {
            let key = search(&provider, media_type, &query, page);
            let normalized = query_part(&key);
            prop_assert_eq!(&normalized, &query.trim().to_lowercase());
            prop_assert_eq!(search(&provider, media_type, &normalized, page).key_id(), key.key_id());
        }

        /// For colon-free components, two search keys share an id exactly when
        /// their provider, media type, normalized query and page all match.
        #[test]
        fn search_key_id_is_injective_over_its_fields(
            a in ("[a-c]{1,2}", media_type(), "[ a-cA-C]{0,3}", 0u32..3),
            b in ("[a-c]{1,2}", media_type(), "[ a-cA-C]{0,3}", 0u32..3),
        ) {
            let fields_equal = a.0 == b.0
                && a.1 == b.1
                && a.2.trim().to_lowercase() == b.2.trim().to_lowercase()
                && a.3 == b.3;
            let ka = search(&a.0, a.1, &a.2, a.3);
            let kb = search(&b.0, b.1, &b.2, b.3);
            prop_assert_eq!(ka.key_id() == kb.key_id(), fields_equal);
        }

        /// Detail and item keys for the same media share a key id but are
        /// told apart by their key type; distinct media get distinct ids.
        #[test]
        fn detail_and_item_keys_are_distinct_by_type_and_id(
            media_type in prop_oneof![Just(MediaType::Movie), Just(MediaType::TvShow), Just(MediaType::Person)],
            a in "[a-z]{1,6}:[0-9]{1,4}",
            b in "[a-z]{1,6}:[0-9]{1,4}",
        ) {
            let detail = CacheKey::Detail { media_type, provider_id: a.clone() };
            let item = CacheKey::Item { media_type, provider_id: a.clone() };
            prop_assert_eq!(detail.key_id(), item.key_id());
            prop_assert_ne!(detail.key_type(), item.key_type());
            let other = CacheKey::Detail { media_type, provider_id: b.clone() };
            prop_assert_eq!(detail.key_id() == other.key_id(), a == b);
        }
    }
}
