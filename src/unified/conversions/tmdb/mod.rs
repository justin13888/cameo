//! Conversions from TMDB generated types to unified models.
//!
//! Generated TMDB response fields are `Option<T>`, since TMDB may omit them or
//! send an explicit `null`. Numeric fields fall back to `0` (the spec's
//! documented default, and the pre-existing behaviour); boolean flags whose
//! spec default is `true` (`adult`, `video`, `in_production`) are passed
//! through as `Option<bool>` so an unknown flag never reads as `true`.

pub mod movie;
pub mod person;
pub mod tv;

use crate::{
    providers::tmdb::image_url::{BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize},
    unified::date::PartialDate,
};

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
