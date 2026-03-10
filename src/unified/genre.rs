use std::fmt;

use serde::{Deserialize, Serialize};

/// An unknown genre that could not be mapped to a known variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnknownGenre {
    /// A genre identified by name (e.g. from a provider's genre list).
    Named(String),
    /// A genre identified by a TMDB genre ID that is not in this enum.
    TmdbId(u32),
}

/// A canonical genre covering all known TMDB movie and TV genres, plus anime-specific genres.
///
/// Use [`Genre::from_tmdb_id`], [`Genre::from_name`], or [`Genre::from_anilist_genre`] to
/// convert provider data. Unknown genres are wrapped in [`Genre::Other`].
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
    /// Returns `Genre::Other(UnknownGenre::TmdbId(id))` for unknown IDs.
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
            // Fallback
            other => Genre::Other(UnknownGenre::TmdbId(other.max(0) as u32)),
        }
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
    pub fn name(&self) -> &str {
        match self {
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
            Genre::Other(UnknownGenre::Named(s)) => s.as_str(),
            Genre::Other(UnknownGenre::TmdbId(_)) => "Unknown",
        }
    }
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::{Genre, UnknownGenre};

    #[test]
    fn from_anilist_genre_common() {
        assert_eq!(Genre::from_anilist_genre("Action"), Genre::Action);
        assert_eq!(Genre::from_anilist_genre("adventure"), Genre::Adventure);
        assert_eq!(Genre::from_anilist_genre("Comedy"), Genre::Comedy);
        assert_eq!(Genre::from_anilist_genre("Drama"), Genre::Drama);
        assert_eq!(Genre::from_anilist_genre("Fantasy"), Genre::Fantasy);
        assert_eq!(Genre::from_anilist_genre("Horror"), Genre::Horror);
        assert_eq!(Genre::from_anilist_genre("Music"), Genre::Music);
        assert_eq!(Genre::from_anilist_genre("Mystery"), Genre::Mystery);
        assert_eq!(Genre::from_anilist_genre("Romance"), Genre::Romance);
        assert_eq!(Genre::from_anilist_genre("Sci-Fi"), Genre::ScienceFiction);
        assert_eq!(Genre::from_anilist_genre("Thriller"), Genre::Thriller);
    }

    #[test]
    fn from_anilist_genre_anime_specific() {
        assert_eq!(Genre::from_anilist_genre("Mecha"), Genre::Mecha);
        assert_eq!(Genre::from_anilist_genre("Mahou Shoujo"), Genre::MahouShoujo);
        assert_eq!(Genre::from_anilist_genre("Slice of Life"), Genre::SliceOfLife);
        assert_eq!(Genre::from_anilist_genre("Sports"), Genre::Sports);
        assert_eq!(Genre::from_anilist_genre("Supernatural"), Genre::Supernatural);
        assert_eq!(Genre::from_anilist_genre("Ecchi"), Genre::Ecchi);
    }

    #[test]
    fn from_anilist_genre_fallback() {
        assert_eq!(
            Genre::from_anilist_genre("Isekai"),
            Genre::Other(UnknownGenre::Named("isekai".to_string()))
        );
        assert_eq!(
            Genre::from_anilist_genre("Harem"),
            Genre::Other(UnknownGenre::Named("harem".to_string()))
        );
    }

    #[test]
    fn from_tmdb_id_unknown() {
        assert_eq!(
            Genre::from_tmdb_id(99999),
            Genre::Other(UnknownGenre::TmdbId(99999))
        );
        assert_eq!(
            Genre::from_tmdb_id(0),
            Genre::Other(UnknownGenre::TmdbId(0))
        );
    }

    #[test]
    fn name_anime_specific() {
        assert_eq!(Genre::Mecha.name(), "Mecha");
        assert_eq!(Genre::MahouShoujo.name(), "Mahou Shoujo");
        assert_eq!(Genre::SliceOfLife.name(), "Slice of Life");
        assert_eq!(Genre::Sports.name(), "Sports");
        assert_eq!(Genre::Supernatural.name(), "Supernatural");
        assert_eq!(Genre::Ecchi.name(), "Ecchi");
    }
}
