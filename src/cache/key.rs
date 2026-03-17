//! Cache key types for the cameo cache layer.

/// The media type of a cached item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaType {
    /// Movie media type.
    Movie,
    /// TV show media type.
    TvShow,
    /// Person (cast/crew) media type.
    Person,
}

impl MediaType {
    fn as_str(&self) -> &'static str {
        match self {
            MediaType::Movie => "movie",
            MediaType::TvShow => "tv",
            MediaType::Person => "person",
        }
    }
}

/// A key identifying a cache entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    /// Full detail response: (media_type, provider_id).
    Detail {
        /// Media type of the cached item.
        media_type: MediaType,
        /// Provider-qualified ID (e.g. `"tmdb:550"`).
        provider_id: String,
    },
    /// Individual item indexed from a list result: (media_type, provider_id).
    Item {
        /// Media type of the cached item.
        media_type: MediaType,
        /// Provider-qualified ID (e.g. `"tmdb:550"`).
        provider_id: String,
    },
    /// Search results page: (optional media_type, normalized query, page).
    Search {
        /// Optional media type filter (`None` means multi-search).
        media_type: Option<MediaType>,
        /// Search query string (will be normalized for the key).
        query: String,
        /// Page number (1-based).
        page: u32,
    },
    /// Discovery/list results page: (endpoint name, page).
    Discovery {
        /// Endpoint or list name (e.g. `"trending_movies"`).
        endpoint: String,
        /// Page number (1-based).
        page: u32,
    },
}

impl CacheKey {
    /// Returns the `key_type` string used in the SQLite table (discriminant).
    pub fn key_type(&self) -> &'static str {
        match self {
            CacheKey::Detail { .. } => "detail",
            CacheKey::Item { .. } => "item",
            CacheKey::Search { .. } => "search",
            CacheKey::Discovery { .. } => "discovery",
        }
    }

    /// Returns the `key_id` string used as the second column of the composite
    /// primary key, uniquely identifying the entry within its `key_type`.
    pub fn key_id(&self) -> String {
        match self {
            CacheKey::Detail {
                media_type,
                provider_id,
            } => {
                format!("{}:{}", media_type.as_str(), provider_id)
            }
            CacheKey::Item {
                media_type,
                provider_id,
            } => {
                format!("{}:{}", media_type.as_str(), provider_id)
            }
            CacheKey::Search {
                media_type,
                query,
                page,
            } => {
                let mt = media_type.map_or("multi", |m| m.as_str());
                format!("{}:{}:{}", mt, normalize_query(query), page)
            }
            CacheKey::Discovery { endpoint, page } => {
                format!("{}:{}", endpoint, page)
            }
        }
    }
}

/// Normalize a search query for cache key consistency.
pub(crate) fn normalize_query(query: &str) -> String {
    query.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_key_id() {
        let key = CacheKey::Detail {
            media_type: MediaType::Movie,
            provider_id: "tmdb:550".to_string(),
        };
        assert_eq!(key.key_type(), "detail");
        assert_eq!(key.key_id(), "movie:tmdb:550");
    }

    #[test]
    fn item_key_id() {
        let key = CacheKey::Item {
            media_type: MediaType::TvShow,
            provider_id: "tmdb:1396".to_string(),
        };
        assert_eq!(key.key_type(), "item");
        assert_eq!(key.key_id(), "tv:tmdb:1396");
    }

    #[test]
    fn search_key_normalizes_query() {
        let key = CacheKey::Search {
            media_type: Some(MediaType::Movie),
            query: "  Dune  ".to_string(),
            page: 1,
        };
        assert_eq!(key.key_id(), "movie:dune:1");
    }

    #[test]
    fn search_key_multi() {
        let key = CacheKey::Search {
            media_type: None,
            query: "brad pitt".to_string(),
            page: 2,
        };
        assert_eq!(key.key_id(), "multi:brad pitt:2");
    }

    #[test]
    fn discovery_key_id() {
        let key = CacheKey::Discovery {
            endpoint: "trending_movies".to_string(),
            page: 1,
        };
        assert_eq!(key.key_id(), "trending_movies:1");
    }
}
