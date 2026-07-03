//! Partial calendar dates for release/air dates that may be incompletely known.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// A calendar date that may be only partially specified.
///
/// Providers do not always expose a full `YYYY-MM-DD` date: AniList in
/// particular emits fuzzy dates such as `"2022"` (year only) or `"2022-04"`
/// (year and month). `PartialDate` preserves exactly the components a provider
/// supplied — no more, no less — so a year-only date round-trips as a year-only
/// date rather than being silently padded to January 1st.
///
/// # Invariants
///
/// - A `month`, when present, is in `1..=12`.
/// - A `day`, when present, is in `1..=31` **and** implies a `month`.
///
/// These are enforced by every constructor, so a `PartialDate` is always
/// internally consistent.
///
/// # Text form
///
/// [`Display`](fmt::Display) and [`FromStr`] use `YYYY`, `YYYY-MM`, or
/// `YYYY-MM-DD` depending on how much is known, and serde serializes to that
/// same string — so any `PartialDate` round-trips losslessly through text and
/// JSON.
///
/// # Examples
///
/// ```
/// use cameo::PartialDate;
///
/// let full: PartialDate = "2022-04-01".parse().unwrap();
/// assert_eq!(full.year(), 2022);
/// assert_eq!(full.month(), Some(4));
/// assert_eq!(full.day(), Some(1));
///
/// let year_only: PartialDate = "2022".parse().unwrap();
/// assert_eq!(year_only.month(), None);
/// assert_eq!(year_only.to_string(), "2022");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartialDate {
    year: i32,
    month: Option<u8>,
    day: Option<u8>,
}

impl PartialDate {
    /// A year-only date (e.g. `2022`).
    pub const fn from_year(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    /// A year-and-month date. Returns `None` if `month` is not in `1..=12`.
    pub const fn year_month(year: i32, month: u8) -> Option<Self> {
        if 1 <= month && month <= 12 {
            Some(Self {
                year,
                month: Some(month),
                day: None,
            })
        } else {
            None
        }
    }

    /// A full year-month-day date. Returns `None` if `month` is not in `1..=12`
    /// or `day` is not in `1..=31`.
    pub const fn ymd(year: i32, month: u8, day: u8) -> Option<Self> {
        if 1 <= month && month <= 12 && 1 <= day && day <= 31 {
            Some(Self {
                year,
                month: Some(month),
                day: Some(day),
            })
        } else {
            None
        }
    }

    /// Construct from optional components.
    ///
    /// A `day` requires a `month`; passing a `day` without a `month` returns
    /// `None`, as do out-of-range components.
    pub const fn new(year: i32, month: Option<u8>, day: Option<u8>) -> Option<Self> {
        match (month, day) {
            (None, None) => Some(Self::from_year(year)),
            (Some(m), None) => Self::year_month(year, m),
            (Some(m), Some(d)) => Self::ymd(year, m, d),
            (None, Some(_)) => None,
        }
    }

    /// The year component.
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// The month component (`1..=12`), if known.
    pub const fn month(&self) -> Option<u8> {
        self.month
    }

    /// The day component (`1..=31`), if known.
    pub const fn day(&self) -> Option<u8> {
        self.day
    }

    /// Whether the date is fully specified (year, month, and day all present).
    pub const fn is_complete(&self) -> bool {
        self.day.is_some()
    }

    /// Convert to a [`chrono::NaiveDate`] when fully specified and a real
    /// calendar date; returns `None` for partial dates or impossible days
    /// (e.g. February 30th).
    pub fn to_naive_date(&self) -> Option<chrono::NaiveDate> {
        let month = self.month?;
        let day = self.day?;
        chrono::NaiveDate::from_ymd_opt(self.year, month as u32, day as u32)
    }
}

impl From<chrono::NaiveDate> for PartialDate {
    fn from(date: chrono::NaiveDate) -> Self {
        use chrono::Datelike;
        Self {
            year: date.year(),
            month: Some(date.month() as u8),
            day: Some(date.day() as u8),
        }
    }
}

impl fmt::Display for PartialDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.month, self.day) {
            (Some(m), Some(d)) => write!(f, "{:04}-{m:02}-{d:02}", self.year),
            (Some(m), None) => write!(f, "{:04}-{m:02}", self.year),
            _ => write!(f, "{:04}", self.year),
        }
    }
}

/// Error returned when a string cannot be parsed as a [`PartialDate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDateError {
    input: String,
}

impl fmt::Display for ParseDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid partial date {:?} (expected YYYY, YYYY-MM, or YYYY-MM-DD)",
            self.input
        )
    }
}

impl std::error::Error for ParseDateError {}

impl FromStr for PartialDate {
    type Err = ParseDateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseDateError {
            input: s.to_string(),
        };
        let mut parts = s.split('-');
        let year = parts.next().ok_or_else(err)?.parse().map_err(|_| err())?;
        let month = parts
            .next()
            .map(|m| m.parse::<u8>())
            .transpose()
            .map_err(|_| err())?;
        let day = parts
            .next()
            .map(|d| d.parse::<u8>())
            .transpose()
            .map_err(|_| err())?;
        if parts.next().is_some() {
            return Err(err());
        }
        PartialDate::new(year, month, day).ok_or_else(err)
    }
}

impl Serialize for PartialDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PartialDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_enforce_invariants() {
        assert_eq!(PartialDate::from_year(2022).to_string(), "2022");
        assert!(PartialDate::year_month(2022, 4).is_some());
        assert!(PartialDate::year_month(2022, 0).is_none());
        assert!(PartialDate::year_month(2022, 13).is_none());
        assert!(PartialDate::ymd(2022, 4, 1).is_some());
        assert!(PartialDate::ymd(2022, 4, 0).is_none());
        assert!(PartialDate::ymd(2022, 4, 32).is_none());
        // A day without a month is inconsistent.
        assert!(PartialDate::new(2022, None, Some(1)).is_none());
    }

    #[test]
    fn display_matches_precision() {
        assert_eq!(
            PartialDate::ymd(2022, 4, 1).unwrap().to_string(),
            "2022-04-01"
        );
        assert_eq!(
            PartialDate::year_month(2022, 4).unwrap().to_string(),
            "2022-04"
        );
        assert_eq!(PartialDate::from_year(2022).to_string(), "2022");
    }

    #[test]
    fn parse_round_trips_all_precisions() {
        for s in ["2022", "2022-04", "2022-04-01"] {
            let parsed: PartialDate = s.parse().unwrap();
            assert_eq!(parsed.to_string(), s, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn parse_rejects_garbage() {
        for s in [
            "",
            "abc",
            "2022-13",
            "2022-00-00",
            "2022-04-99",
            "2022-04-01-01",
            "-2022",
        ] {
            assert!(s.parse::<PartialDate>().is_err(), "should reject {s:?}");
        }
    }

    #[test]
    fn json_round_trips_losslessly() {
        for s in ["2022", "2022-04", "2022-04-01"] {
            let original: PartialDate = s.parse().unwrap();
            let json = serde_json::to_string(&original).unwrap();
            assert_eq!(json, format!("\"{s}\""));
            let back: PartialDate = serde_json::from_str(&json).unwrap();
            assert_eq!(back, original);
        }
    }

    #[test]
    fn chrono_interop() {
        let full = PartialDate::ymd(2022, 4, 1).unwrap();
        let naive = full.to_naive_date().unwrap();
        assert_eq!(PartialDate::from(naive), full);
        // Partial dates have no single calendar day.
        assert!(
            PartialDate::year_month(2022, 4)
                .unwrap()
                .to_naive_date()
                .is_none()
        );
        // Impossible calendar days are rejected even when fully specified.
        assert!(
            PartialDate::ymd(2022, 2, 30)
                .unwrap()
                .to_naive_date()
                .is_none()
        );
    }
}
