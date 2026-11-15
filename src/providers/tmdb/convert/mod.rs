//! Conversions from TMDB generated types to unified models and to the
//! TMDB-native [`models`](crate::providers::tmdb::models) types.
//!
//! Image paths become full CDN URLs only through the `resolve_*` helpers
//! here, so each image kind has one size everywhere.
//!
//! Generated TMDB response fields are `Option<T>`, since TMDB may omit them or
//! send an explicit `null`. The conversions keep that optionality honest:
//!
//! - Unified `Option` fields (counts, scores, flags such as `adult`, `video`,
//!   `in_production`, nested company/network/creator ids) are `None` when TMDB
//!   omits the field or sends `null`; a value TMDB actually sends (including
//!   `0`) is kept. Boolean flags whose spec default is `true` therefore never
//!   read as `true` when unknown.
//! - Season-list and episode-list items without a number are dropped: a season
//!   or episode with no number cannot be addressed, and defaulting it to `0`
//!   would pass it off as the Specials season or invent a phantom episode 0.
//! - A missing number on a single season or episode: the conversions fall back
//!   to `0`, but [`TmdbClient`](crate::providers::tmdb::TmdbClient) never relies
//!   on that — season and episode details requests fill in the requested
//!   number before converting.
//! - Genre entries without an id are dropped.
//! - A missing or invalid (zero or negative) top-level id: the `From` impls
//!   fall back to `tmdb:0` rather than wrap the value, but
//!   [`TmdbClient`](crate::providers::tmdb::TmdbClient) never relies on that —
//!   details requests fill in the requested id before converting, and list or
//!   search items without a valid id are dropped and logged (see
//!   [`TmdbListItem`](page::TmdbListItem)).

pub(crate) mod certifications;
pub(crate) mod credits;
mod images;
pub(crate) mod movie;
mod page;
pub(crate) mod person;
mod tmdb_credits;
pub(crate) mod tv;
pub(crate) mod watch_providers;

pub(crate) use page::{MAX_PAGE, TmdbPaged, single_page};

use crate::{
    providers::tmdb::image_url::{
        BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize, StillSize,
    },
    unified::{conversions::native_id, date::PartialDate, media_id::MediaId},
};

/// The [`MediaId`] of a top-level TMDB record.
///
/// A missing or invalid id reads as `tmdb:0`, never a wrapped value; the
/// client screens ids before converting (see the module docs), so that
/// fallback is not reached through [`TmdbClient`](crate::providers::tmdb::TmdbClient).
pub(super) fn tmdb_media_id(id: Option<i64>) -> MediaId {
    MediaId::tmdb(id.and_then(native_id).unwrap_or_default())
}

pub(super) fn resolve_logo(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::logo(p, LogoSize::W92))
}

/// Parse a TMDB date string (`YYYY-MM-DD`, occasionally empty or malformed)
/// into a [`PartialDate`], dropping empty/invalid values to `None`.
pub(super) fn resolve_date(raw: &Option<String>) -> Option<PartialDate> {
    raw.as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}

pub(super) fn resolve_poster(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::poster(p, PosterSize::W500))
}

pub(super) fn resolve_backdrop(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::backdrop(p, BackdropSize::W780))
}

pub(super) fn resolve_profile(path: &Option<String>) -> Option<String> {
    path.as_deref()
        .map(|p| ImageUrl::profile(p, ProfileSize::H632))
}

pub(super) fn resolve_still(path: &Option<String>) -> Option<String> {
    path.as_deref().map(|p| ImageUrl::still(p, StillSize::W300))
}
