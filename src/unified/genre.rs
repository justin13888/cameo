use std::{borrow::Cow, fmt};

use serde::{Deserialize, Serialize};

/// An unknown genre that could not be mapped to a known variant.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnknownGenre {
    /// A genre identified by name (e.g. from a provider's genre list).
    Named(String),
    /// A genre identified by a TMDB genre ID that is not in this enum.
    ///
    /// The raw provider id is preserved verbatim (including invalid/negative
    /// ids) so distinct unknown ids never collide.
    TmdbId(i64),
}

/// A canonical genre covering all known TMDB movie and TV genres, plus anime-specific genres.
///
/// Use [`Genre::from_tmdb_id`], [`Genre::from_name`], or [`Genre::from_anilist_genre`] to
/// convert provider data. Unknown genres are wrapped in [`Genre::Other`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Genre {
    // Movie-only genres
    /// TMDB ID 28 — Action
    Action,
    /// TMDB ID 12 — Adventure
    Adventure,
    /// TMDB ID 14 — Fantasy
    Fantasy,
    /// TMDB ID 36 — History
    History,
    /// TMDB ID 27 — Horror
    Horror,
    /// TMDB ID 10402 — Music
    Music,
    /// TMDB ID 10749 — Romance
    Romance,
    /// TMDB ID 878 — Science Fiction
    ScienceFiction,
    /// TMDB ID 53 — Thriller
    Thriller,
    /// TMDB ID 10770 — TV Movie
    TvMovie,
    /// TMDB ID 10752 — War
    War,
    // TV-only genres
    /// TMDB ID 10759 — Action & Adventure (TV)
    ActionAdventure,
    /// TMDB ID 10762 — Kids (TV)
    Kids,
    /// TMDB ID 10763 — News (TV)
    News,
    /// TMDB ID 10764 — Reality (TV)
    Reality,
    /// TMDB ID 10765 — Sci-Fi & Fantasy (TV)
    SciFiFantasy,
    /// TMDB ID 10766 — Soap (TV)
    Soap,
    /// TMDB ID 10767 — Talk (TV)
    Talk,
    /// TMDB ID 10768 — War & Politics (TV)
    WarPolitics,
    // Shared (movie + TV) genres
    /// TMDB ID 16 — Animation
    Animation,
    /// TMDB ID 35 — Comedy
    Comedy,
    /// TMDB ID 80 — Crime
    Crime,
    /// TMDB ID 99 — Documentary
    Documentary,
    /// TMDB ID 18 — Drama
    Drama,
    /// TMDB ID 10751 — Family
    Family,
    /// TMDB ID 9648 — Mystery
    Mystery,
    /// TMDB ID 37 — Western
    Western,
    // Anime-specific genres (AniList)
    /// Mecha — giant robot / mechanical suit anime.
    Mecha,
    /// Mahou Shoujo — magical girl anime.
    MahouShoujo,
    /// Slice of Life — everyday life stories.
    SliceOfLife,
    /// Sports — competitive athletics as the central theme.
    Sports,
    /// Supernatural — ghosts, demons, divine powers, and the occult.
    Supernatural,
    /// Ecchi — suggestive, mildly sexual content.
    Ecchi,
    /// A genre not covered by the known variants.
    Other(UnknownGenre),
}

impl Genre {
    /// Convert a TMDB genre ID to a [`Genre`].
    ///
    /// Returns `Genre::Other(UnknownGenre::TmdbId(id))` for unknown IDs,
    /// preserving the raw id (invalid or negative ids are kept as-is rather
    /// than collapsed to zero).
    pub fn from_tmdb_id(id: i64) -> Genre {
        match id {
            // Movie-only
            28 => Genre::Action,
            12 => Genre::Adventure,
            14 => Genre::Fantasy,
            36 => Genre::History,
            27 => Genre::Horror,
            10402 => Genre::Music,
            10749 => Genre::Romance,
            878 => Genre::ScienceFiction,
            53 => Genre::Thriller,
            10770 => Genre::TvMovie,
            10752 => Genre::War,
            // TV-only
            10759 => Genre::ActionAdventure,
            10762 => Genre::Kids,
            10763 => Genre::News,
            10764 => Genre::Reality,
            10765 => Genre::SciFiFantasy,
            10766 => Genre::Soap,
            10767 => Genre::Talk,
            10768 => Genre::WarPolitics,
            // Shared
            16 => Genre::Animation,
            35 => Genre::Comedy,
            80 => Genre::Crime,
            99 => Genre::Documentary,
            18 => Genre::Drama,
            10751 => Genre::Family,
            9648 => Genre::Mystery,
            37 => Genre::Western,
            // Fallback — preserve the raw id verbatim.
            other => Genre::Other(UnknownGenre::TmdbId(other)),
        }
    }

    /// The TMDB genre id for this genre, the inverse of [`Genre::from_tmdb_id`].
    ///
    /// Returns `None` for genres TMDB has no id for: the anime-specific
    /// variants ([`Genre::Mecha`], [`Genre::SliceOfLife`], ...), a
    /// [`UnknownGenre::Named`] genre, and a [`UnknownGenre::TmdbId`] holding a
    /// negative (invalid) id. A non-negative unknown id is returned verbatim, so
    /// `Genre::from_tmdb_id(n).tmdb_id() == Some(n as u64)` for every `n >= 0`.
    ///
    /// Movie and TV genres use different TMDB ids (e.g. [`Genre::Action`] is a
    /// movie genre; TV uses [`Genre::ActionAdventure`]).
    ///
    /// ```
    /// use cameo::Genre;
    /// assert_eq!(Genre::Action.tmdb_id(), Some(28));
    /// assert_eq!(Genre::SciFiFantasy.tmdb_id(), Some(10765));
    /// assert_eq!(Genre::Mecha.tmdb_id(), None);
    /// ```
    pub fn tmdb_id(&self) -> Option<u64> {
        Some(match self {
            // Movie-only
            Genre::Action => 28,
            Genre::Adventure => 12,
            Genre::Fantasy => 14,
            Genre::History => 36,
            Genre::Horror => 27,
            Genre::Music => 10402,
            Genre::Romance => 10749,
            Genre::ScienceFiction => 878,
            Genre::Thriller => 53,
            Genre::TvMovie => 10770,
            Genre::War => 10752,
            // TV-only
            Genre::ActionAdventure => 10759,
            Genre::Kids => 10762,
            Genre::News => 10763,
            Genre::Reality => 10764,
            Genre::SciFiFantasy => 10765,
            Genre::Soap => 10766,
            Genre::Talk => 10767,
            Genre::WarPolitics => 10768,
            // Shared
            Genre::Animation => 16,
            Genre::Comedy => 35,
            Genre::Crime => 80,
            Genre::Documentary => 99,
            Genre::Drama => 18,
            Genre::Family => 10751,
            Genre::Mystery => 9648,
            Genre::Western => 37,
            // Anime-specific genres have no TMDB id.
            Genre::Mecha
            | Genre::MahouShoujo
            | Genre::SliceOfLife
            | Genre::Sports
            | Genre::Supernatural
            | Genre::Ecchi
            | Genre::Other(UnknownGenre::Named(_)) => return None,
            Genre::Other(UnknownGenre::TmdbId(id)) => return u64::try_from(*id).ok(),
        })
    }

    /// Convert a genre name string to a [`Genre`] (case-insensitive).
    ///
    /// Returns `Genre::Other(UnknownGenre::Named(s))` for unrecognized names.
    pub fn from_name(name: &str) -> Genre {
        match name.to_lowercase().as_str() {
            "action" => Genre::Action,
            "adventure" => Genre::Adventure,
            "fantasy" => Genre::Fantasy,
            "history" => Genre::History,
            "horror" => Genre::Horror,
            "music" => Genre::Music,
            "romance" => Genre::Romance,
            "science fiction" => Genre::ScienceFiction,
            "thriller" => Genre::Thriller,
            "tv movie" => Genre::TvMovie,
            "war" => Genre::War,
            "action & adventure" => Genre::ActionAdventure,
            "kids" => Genre::Kids,
            "news" => Genre::News,
            "reality" => Genre::Reality,
            "sci-fi & fantasy" => Genre::SciFiFantasy,
            "soap" => Genre::Soap,
            "talk" => Genre::Talk,
            "war & politics" => Genre::WarPolitics,
            "animation" => Genre::Animation,
            "comedy" => Genre::Comedy,
            "crime" => Genre::Crime,
            "documentary" => Genre::Documentary,
            "drama" => Genre::Drama,
            "family" => Genre::Family,
            "mystery" => Genre::Mystery,
            "western" => Genre::Western,
            // Anime-specific
            "mecha" => Genre::Mecha,
            "mahou shoujo" => Genre::MahouShoujo,
            "slice of life" => Genre::SliceOfLife,
            "sports" => Genre::Sports,
            "supernatural" => Genre::Supernatural,
            "ecchi" => Genre::Ecchi,
            other => Genre::Other(UnknownGenre::Named(other.to_string())),
        }
    }

    /// Convert an AniList genre name string to a [`Genre`] (case-insensitive).
    ///
    /// Handles AniList-specific genre names in addition to common genre names.
    /// Returns `Genre::Other(UnknownGenre::Named(s))` for unrecognized names.
    pub fn from_anilist_genre(name: &str) -> Genre {
        match name.to_lowercase().as_str() {
            "action" => Genre::Action,
            "adventure" => Genre::Adventure,
            "comedy" => Genre::Comedy,
            "drama" => Genre::Drama,
            "ecchi" => Genre::Ecchi,
            "fantasy" => Genre::Fantasy,
            "horror" => Genre::Horror,
            "mahou shoujo" => Genre::MahouShoujo,
            "mecha" => Genre::Mecha,
            "music" => Genre::Music,
            "mystery" => Genre::Mystery,
            "romance" => Genre::Romance,
            "sci-fi" => Genre::ScienceFiction,
            "slice of life" => Genre::SliceOfLife,
            "sports" => Genre::Sports,
            "supernatural" => Genre::Supernatural,
            "thriller" => Genre::Thriller,
            other => Genre::Other(UnknownGenre::Named(other.to_string())),
        }
    }

    /// Returns a human-readable display name for this genre.
    ///
    /// Unknown genres keep their identifying information: a named unknown
    /// returns its name, and an unknown TMDB id returns `"TMDB genre {id}"`
    /// (the id is never discarded).
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            Genre::Other(UnknownGenre::Named(s)) => Cow::Borrowed(s.as_str()),
            Genre::Other(UnknownGenre::TmdbId(id)) => Cow::Owned(format!("TMDB genre {id}")),
            known => Cow::Borrowed(known.known_name().unwrap_or("Unknown")),
        }
    }

    /// The static display name for a known genre variant; `None` for
    /// [`Genre::Other`].
    fn known_name(&self) -> Option<&'static str> {
        Some(match self {
            Genre::Action => "Action",
            Genre::Adventure => "Adventure",
            Genre::Fantasy => "Fantasy",
            Genre::History => "History",
            Genre::Horror => "Horror",
            Genre::Music => "Music",
            Genre::Romance => "Romance",
            Genre::ScienceFiction => "Science Fiction",
            Genre::Thriller => "Thriller",
            Genre::TvMovie => "TV Movie",
            Genre::War => "War",
            Genre::ActionAdventure => "Action & Adventure",
            Genre::Kids => "Kids",
            Genre::News => "News",
            Genre::Reality => "Reality",
            Genre::SciFiFantasy => "Sci-Fi & Fantasy",
            Genre::Soap => "Soap",
            Genre::Talk => "Talk",
            Genre::WarPolitics => "War & Politics",
            Genre::Animation => "Animation",
            Genre::Comedy => "Comedy",
            Genre::Crime => "Crime",
            Genre::Documentary => "Documentary",
            Genre::Drama => "Drama",
            Genre::Family => "Family",
            Genre::Mystery => "Mystery",
            Genre::Western => "Western",
            Genre::Mecha => "Mecha",
            Genre::MahouShoujo => "Mahou Shoujo",
            Genre::SliceOfLife => "Slice of Life",
            Genre::Sports => "Sports",
            Genre::Supernatural => "Supernatural",
            Genre::Ecchi => "Ecchi",
            Genre::Other(_) => return None,
        })
    }
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests;
