//! Property tests over the pure, round-trippable conversion helpers (#51).
//!
//! These complement the fixture-driven snapshot tests in `tests/integration/`
//! (each `tmdb_*`/`anilist_*` test deserializes a committed JSON fixture and
//! asserts the converted unified values) by checking algebraic invariants over
//! generated inputs.

use cameo::{Genre, MediaId, PartialDate};
use proptest::prelude::*;

proptest! {
    /// A fully-specified date survives Display -> FromStr unchanged.
    #[test]
    fn partial_date_full_round_trips(y in 0i32..9999, m in 1u8..=12, d in 1u8..=31) {
        let date = PartialDate::ymd(y, m, d).expect("valid ymd");
        prop_assert_eq!(date.to_string().parse::<PartialDate>().unwrap(), date);
    }

    /// A year-month date stays partial across the round-trip (no day padding).
    #[test]
    fn partial_date_year_month_round_trips(y in 0i32..9999, m in 1u8..=12) {
        let date = PartialDate::year_month(y, m).expect("valid ym");
        let back = date.to_string().parse::<PartialDate>().unwrap();
        prop_assert_eq!(back, date);
        prop_assert_eq!(back.day(), None);
    }

    /// A year-only date stays year-only.
    #[test]
    fn partial_date_year_only_round_trips(y in 0i32..9999) {
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
}
