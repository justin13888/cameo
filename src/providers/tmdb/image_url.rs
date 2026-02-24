const IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p/";

/// Poster image sizes available from TMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosterSize {
    W92,
    W154,
    W185,
    W342,
    W500,
    W780,
    Original,
}

impl PosterSize {
    fn as_str(&self) -> &'static str {
        match self {
            PosterSize::W92 => "w92",
            PosterSize::W154 => "w154",
            PosterSize::W185 => "w185",
            PosterSize::W342 => "w342",
            PosterSize::W500 => "w500",
            PosterSize::W780 => "w780",
            PosterSize::Original => "original",
        }
    }
}

/// Backdrop image sizes available from TMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropSize {
    W300,
    W780,
    W1280,
    Original,
}

impl BackdropSize {
    fn as_str(&self) -> &'static str {
        match self {
            BackdropSize::W300 => "w300",
            BackdropSize::W780 => "w780",
            BackdropSize::W1280 => "w1280",
            BackdropSize::Original => "original",
        }
    }
}

/// Profile (person) image sizes available from TMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSize {
    W45,
    W185,
    H632,
    Original,
}

impl ProfileSize {
    fn as_str(&self) -> &'static str {
        match self {
            ProfileSize::W45 => "w45",
            ProfileSize::W185 => "w185",
            ProfileSize::H632 => "h632",
            ProfileSize::Original => "original",
        }
    }
}

/// Still (episode) image sizes available from TMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StillSize {
    W92,
    W185,
    W300,
    Original,
}

impl StillSize {
    fn as_str(&self) -> &'static str {
        match self {
            StillSize::W92 => "w92",
            StillSize::W185 => "w185",
            StillSize::W300 => "w300",
            StillSize::Original => "original",
        }
    }
}

/// Logo image sizes available from TMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoSize {
    W45,
    W92,
    W154,
    W185,
    W300,
    W500,
    Original,
}

impl LogoSize {
    fn as_str(&self) -> &'static str {
        match self {
            LogoSize::W45 => "w45",
            LogoSize::W92 => "w92",
            LogoSize::W154 => "w154",
            LogoSize::W185 => "w185",
            LogoSize::W300 => "w300",
            LogoSize::W500 => "w500",
            LogoSize::Original => "original",
        }
    }
}

/// Helper for constructing TMDB image URLs from file paths.
pub struct ImageUrl;

impl ImageUrl {
    /// Build a full poster image URL.
    ///
    /// `path` is the raw path from the API (e.g. `"/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg"`).
    pub fn poster(path: &str, size: PosterSize) -> String {
        format!("{IMAGE_BASE_URL}{}{path}", size.as_str())
    }

    /// Build a full backdrop image URL.
    pub fn backdrop(path: &str, size: BackdropSize) -> String {
        format!("{IMAGE_BASE_URL}{}{path}", size.as_str())
    }

    /// Build a full profile (person) image URL.
    pub fn profile(path: &str, size: ProfileSize) -> String {
        format!("{IMAGE_BASE_URL}{}{path}", size.as_str())
    }

    /// Build a full still (episode) image URL.
    pub fn still(path: &str, size: StillSize) -> String {
        format!("{IMAGE_BASE_URL}{}{path}", size.as_str())
    }

    /// Build a full logo image URL.
    pub fn logo(path: &str, size: LogoSize) -> String {
        format!("{IMAGE_BASE_URL}{}{path}", size.as_str())
    }
}
