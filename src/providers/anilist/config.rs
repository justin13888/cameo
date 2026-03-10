const DEFAULT_PER_PAGE: u32 = 20;

/// Configuration for the [`AniListClient`](super::client::AniListClient).
#[derive(Debug, Clone)]
pub struct AniListConfig {
    /// Base URL for the AniList GraphQL endpoint.
    ///
    /// Defaults to `https://graphql.anilist.co`.  Override for testing.
    pub base_url: String,
    /// Number of results returned per page (default: 20, max: 50).
    pub per_page: u32,
}

impl Default for AniListConfig {
    fn default() -> Self {
        Self {
            base_url: "https://graphql.anilist.co".to_string(),
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl AniListConfig {
    /// Create a new config with default settings.
    ///
    /// No authentication is required for AniList public data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with a custom base URL (useful for testing with mock servers).
    pub fn new_with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::default()
        }
    }

    /// Set the number of results per page.
    ///
    /// Valid range is 1–50 (AniList API limit); values outside this range are clamped.
    pub fn with_per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page.clamp(1, 50);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::AniListConfig;

    #[test]
    fn with_per_page_normal() {
        let cfg = AniListConfig::new().with_per_page(25);
        assert_eq!(cfg.per_page, 25);
    }

    #[test]
    fn with_per_page_clamp_below_min() {
        let cfg = AniListConfig::new().with_per_page(0);
        assert_eq!(cfg.per_page, 1);
    }

    #[test]
    fn with_per_page_clamp_above_max() {
        let cfg = AniListConfig::new().with_per_page(100);
        assert_eq!(cfg.per_page, 50);
    }

    #[test]
    fn with_per_page_boundary_values() {
        assert_eq!(AniListConfig::new().with_per_page(1).per_page, 1);
        assert_eq!(AniListConfig::new().with_per_page(50).per_page, 50);
    }
}
