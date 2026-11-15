//! Builders for the watch-availability family: [`UnifiedWatchProviders`],
//! [`UnifiedWatchProviderEntry`] and [`UnifiedStreamingService`].

use crate::unified::{media_id::MediaId, models::*};

impl UnifiedWatchProviders {
    /// Create an empty watch-providers set for the given media id.
    pub fn new(provider_id: MediaId) -> Self {
        Self {
            provider_id,
            results: std::collections::HashMap::new(),
        }
    }

    /// Insert a per-country entry.
    pub fn with_country(
        mut self,
        country_code: impl Into<String>,
        entry: UnifiedWatchProviderEntry,
    ) -> Self {
        self.results.insert(country_code.into(), entry);
        self
    }
}

impl UnifiedWatchProviderEntry {
    /// Create an empty entry (equivalent to [`Default::default`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the flat-rate / subscription services.
    pub fn with_flatrate(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.flatrate = value;
        self
    }
    /// Set the rental services.
    pub fn with_rent(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.rent = value;
        self
    }
    /// Set the purchase services.
    pub fn with_buy(mut self, value: Vec<UnifiedStreamingService>) -> Self {
        self.buy = value;
        self
    }
}

impl UnifiedStreamingService {
    /// Create a streaming service with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            logo_url: None,
        }
    }

    /// Set the logo image URL.
    pub fn with_logo_url(mut self, value: impl Into<String>) -> Self {
        self.logo_url = Some(value.into());
        self
    }
}
