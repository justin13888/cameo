//! TMDB images responses → [`TmdbImages`].

use super::{resolve_backdrop, resolve_logo, resolve_poster};
use crate::{
    generated::tmdb::types,
    providers::tmdb::models::{TmdbImage, TmdbImages},
};

impl From<types::MovieImagesResponse> for TmdbImages {
    fn from(i: types::MovieImagesResponse) -> Self {
        TmdbImages {
            backdrops: i
                .backdrops
                .into_option()
                .into_iter()
                .flatten()
                .map(|b| {
                    image_from(
                        resolve_backdrop(&b.file_path),
                        b.width.into_option(),
                        b.height.into_option(),
                        b.aspect_ratio.into_option(),
                        b.vote_average.into_option(),
                        b.vote_count.into_option(),
                    )
                })
                .collect(),
            posters: i
                .posters
                .into_option()
                .into_iter()
                .flatten()
                .map(|p| {
                    image_from(
                        resolve_poster(&p.file_path),
                        p.width.into_option(),
                        p.height.into_option(),
                        p.aspect_ratio.into_option(),
                        p.vote_average.into_option(),
                        p.vote_count.into_option(),
                    )
                })
                .collect(),
            logos: i
                .logos
                .into_option()
                .into_iter()
                .flatten()
                .map(|l| {
                    image_from(
                        resolve_logo(&l.file_path),
                        l.width.into_option(),
                        l.height.into_option(),
                        l.aspect_ratio.into_option(),
                        l.vote_average.into_option(),
                        l.vote_count.into_option(),
                    )
                })
                .collect(),
        }
    }
}

/// Build a [`TmdbImage`] from its already-resolved `url`; an absent/`null`
/// numeric field (or a negative size or count) is `None`, never a fabricated
/// `0`.
fn image_from(
    url: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    aspect_ratio: Option<f64>,
    vote_average: Option<f64>,
    vote_count: Option<i64>,
) -> TmdbImage {
    TmdbImage {
        url,
        width: width.and_then(|w| u32::try_from(w).ok()),
        height: height.and_then(|h| u32::try_from(h).ok()),
        aspect_ratio,
        vote_average,
        vote_count: vote_count.and_then(|c| u64::try_from(c).ok()),
        language: None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Image fields TMDB omits (or reports negative) are `None`, not `0`;
    /// reported values pass through.
    #[test]
    fn image_numeric_fields_are_honestly_optional() {
        let raw: types::MovieImagesResponse = serde_json::from_value(json!({
            "backdrops": [
                { "file_path": "/b.jpg", "width": 1920, "height": 1080,
                  "aspect_ratio": 1.778, "vote_average": 5.3, "vote_count": 12 }
            ],
            "posters": [{ "file_path": "/p.jpg" }],
            "logos": [{ "file_path": "/l.png", "width": -1, "height": -1, "vote_count": -1 }]
        }))
        .expect("images deserialize");

        let images = TmdbImages::from(raw);

        let backdrop = &images.backdrops[0];
        assert_eq!(backdrop.width, Some(1920));
        assert_eq!(backdrop.height, Some(1080));
        assert_eq!(backdrop.aspect_ratio, Some(1.778));
        assert_eq!(backdrop.vote_average, Some(5.3));
        assert_eq!(backdrop.vote_count, Some(12));
        for image in [&images.posters[0], &images.logos[0]] {
            assert_eq!(image.width, None);
            assert_eq!(image.height, None);
            assert_eq!(image.vote_count, None);
        }
        assert_eq!(images.posters[0].aspect_ratio, None);
        assert_eq!(images.posters[0].vote_average, None);
    }

    /// Each image kind resolves at its own size: backdrops W780, posters
    /// W500, logos W92; an image without a path has no URL.
    #[test]
    fn image_urls_resolve_per_kind() {
        let raw: types::MovieImagesResponse = serde_json::from_value(json!({
            "backdrops": [{ "file_path": "/b.jpg" }, {}],
            "posters": [{ "file_path": "/p.jpg" }],
            "logos": [{ "file_path": "/l.png" }]
        }))
        .expect("images deserialize");

        let images = TmdbImages::from(raw);

        assert_eq!(
            images.backdrops[0].url.as_deref(),
            Some("https://image.tmdb.org/t/p/w780/b.jpg")
        );
        assert_eq!(images.backdrops[1].url, None);
        assert_eq!(
            images.posters[0].url.as_deref(),
            Some("https://image.tmdb.org/t/p/w500/p.jpg")
        );
        assert_eq!(
            images.logos[0].url.as_deref(),
            Some("https://image.tmdb.org/t/p/w92/l.png")
        );
    }
}
