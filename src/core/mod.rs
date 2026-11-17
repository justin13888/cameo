//! The leaf modules are private; every item is re-exported here and from the
//! crate root, which is its canonical path.

/// Shared configuration types.
pub(crate) mod config;
/// Provider-agnostic error type.
pub(crate) mod error;
/// Pagination helpers.
pub(crate) mod pagination;

pub use config::TimeWindow;
pub use error::{DecodeError, ProviderError, TransportError};
pub use pagination::{Page, PageToken, into_stream};
