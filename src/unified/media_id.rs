//! A typed, provider-qualified media identifier.
//!
//! Replaces the old `provider_id: String` / raw `i32` split: [`MediaId`]
//! preserves which provider an id came from and round-trips the canonical
//! `"provider:native"` string form used across the API and cache.

use std::{borrow::Cow, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// The provider-native portion of a [`MediaId`].
///
/// Numeric ids (TMDB, AniList media) are stored as `u64` — rejecting the
/// negatives an `i32` admitted and leaving room for larger ids. Non-numeric or
/// namespaced ids (e.g. AniList staff ids, rendered `"staff:95061"`) are kept
/// verbatim as [`NativeId::Text`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeId {
    /// A numeric identifier.
    Num(u64),
    /// A namespaced or non-numeric identifier.
    Text(String),
}

impl fmt::Display for NativeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NativeId::Num(n) => write!(f, "{n}"),
            NativeId::Text(s) => f.write_str(s),
        }
    }
}

/// A provider-qualified media identifier, e.g. `tmdb:550` or `anilist:staff:95061`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaId {
    provider: Cow<'static, str>,
    native: NativeId,
}

/// Error returned when a string cannot be parsed as a [`MediaId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MediaIdParseError {
    /// The string did not contain a `provider:native` separator.
    #[error("media id `{0}` is missing a ':' separator")]
    MissingSeparator(String),
    /// The provider or native portion was empty.
    #[error("media id `{0}` has an empty provider or native part")]
    Empty(String),
}

impl MediaId {
    /// Construct a `MediaId` from a provider tag and native id.
    pub fn new(provider: impl Into<Cow<'static, str>>, native: NativeId) -> Self {
        Self {
            provider: provider.into(),
            native,
        }
    }

    /// A numeric TMDB id (`tmdb:<id>`).
    pub fn tmdb(id: u64) -> Self {
        Self::new("tmdb", NativeId::Num(id))
    }

    /// A numeric AniList media id (`anilist:<id>`).
    pub fn anilist(id: u64) -> Self {
        Self::new("anilist", NativeId::Num(id))
    }

    /// An AniList staff id (`anilist:staff:<id>`).
    pub fn anilist_staff(id: u64) -> Self {
        Self::new("anilist", NativeId::Text(format!("staff:{id}")))
    }

    /// The provider tag (e.g. `"tmdb"`).
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The native identifier.
    pub fn native(&self) -> &NativeId {
        &self.native
    }

    /// The native id as a `u64`, if it is numeric.
    pub fn as_u64(&self) -> Option<u64> {
        match &self.native {
            NativeId::Num(n) => Some(*n),
            NativeId::Text(_) => None,
        }
    }

    /// Parse a `"provider:native"` string. Splits on the **first** `:`, so a
    /// namespaced native part such as `"staff:95061"` is preserved.
    pub fn parse(s: &str) -> Result<Self, MediaIdParseError> {
        let (provider, native) = s
            .split_once(':')
            .ok_or_else(|| MediaIdParseError::MissingSeparator(s.to_string()))?;
        if provider.is_empty() || native.is_empty() {
            return Err(MediaIdParseError::Empty(s.to_string()));
        }
        let native = match native.parse::<u64>() {
            Ok(n) => NativeId::Num(n),
            Err(_) => NativeId::Text(native.to_string()),
        };
        Ok(Self::new(provider.to_string(), native))
    }
}

impl fmt::Display for MediaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.provider, self.native)
    }
}

impl FromStr for MediaId {
    type Err = MediaIdParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<str> for MediaId {
    /// Compare against the canonical `"provider:native"` rendering without
    /// allocating.
    fn eq(&self, other: &str) -> bool {
        let Some((provider, native)) = other.split_once(':') else {
            return false;
        };
        if provider != self.provider.as_ref() {
            return false;
        }
        match &self.native {
            NativeId::Text(s) => native == s,
            // Numeric ids render as canonical decimal (no leading zeros / sign).
            NativeId::Num(n) => {
                native.parse::<u64>() == Ok(*n) && (native == "0" || !native.starts_with('0'))
            }
        }
    }
}

impl PartialEq<&str> for MediaId {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

// Serialize/deserialize as the canonical "provider:native" string so the JSON
// shape (and cache keys) match the previous `provider_id: String`.
impl Serialize for MediaId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MediaId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        MediaId::parse(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_numeric() {
        let id = MediaId::tmdb(550);
        assert_eq!(id.to_string(), "tmdb:550");
        assert_eq!(MediaId::parse("tmdb:550").unwrap(), id);
        assert_eq!(id.provider(), "tmdb");
        assert_eq!(id.as_u64(), Some(550));
    }

    #[test]
    fn round_trips_namespaced() {
        let id = MediaId::anilist_staff(95061);
        assert_eq!(id.to_string(), "anilist:staff:95061");
        // Parse splits on the first ':', preserving "staff:95061".
        let parsed = MediaId::parse("anilist:staff:95061").unwrap();
        assert_eq!(parsed, id);
        assert_eq!(parsed.provider(), "anilist");
        assert_eq!(parsed.as_u64(), None);
    }

    #[test]
    fn serde_round_trip_is_a_string() {
        let id = MediaId::anilist(1);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"anilist:1\"");
        let back: MediaId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn parse_rejects_malformed() {
        assert!(MediaId::parse("no-separator").is_err());
        assert!(MediaId::parse(":empty-provider").is_err());
        assert!(MediaId::parse("empty-native:").is_err());
    }

    #[test]
    fn compares_to_str() {
        assert_eq!(MediaId::tmdb(550), "tmdb:550");
    }
}
