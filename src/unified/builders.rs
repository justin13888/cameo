//! Constructors and builder-style setters for the unified models.
//!
//! Every `Unified*` model is `#[non_exhaustive]`, which forbids struct-literal
//! construction outside this crate. Out-of-crate providers implementing the
//! unified traits still need to *build* the values their impls return, so each
//! model exposes a `new(...)` constructor (taking the genuinely required
//! fields) plus chainable `with_*` setters for the optional ones.
//!
//! The `impl` blocks are split by model family, one submodule each.
//!
//! ```
//! use cameo::{MediaId, PartialDate, UnifiedMovie};
//!
//! let movie = UnifiedMovie::new(MediaId::tmdb(550), "Fight Club")
//!     .with_overview("A ticking-time-bomb insomniac...")
//!     .with_release_date(PartialDate::from_year(1999))
//!     .with_vote_average(8.4);
//! assert_eq!(movie.title, "Fight Club");
//! ```

/// Builders shared across media kinds (alternative titles, companies, search results).
mod common;
/// Builders for movies and collections.
mod movie;
/// Builders for people and credits.
mod person;
/// Builders for TV shows, seasons, episodes, networks and creators.
mod tv;
/// Builders for watch-provider availability.
mod watch;
