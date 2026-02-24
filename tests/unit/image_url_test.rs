use cameo::providers::tmdb::image_url::{
    BackdropSize, ImageUrl, LogoSize, PosterSize, ProfileSize, StillSize,
};

const BASE: &str = "https://image.tmdb.org/t/p/";

#[test]
fn poster_w500() {
    let url = ImageUrl::poster("/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg", PosterSize::W500);
    assert_eq!(url, format!("{BASE}w500/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg"));
}

#[test]
fn poster_original() {
    let url = ImageUrl::poster("/test.jpg", PosterSize::Original);
    assert_eq!(url, format!("{BASE}original/test.jpg"));
}

#[test]
fn backdrop_w1280() {
    let url = ImageUrl::backdrop("/hZkgoQYus5vegHoetLkCJzb17zJ.jpg", BackdropSize::W1280);
    assert_eq!(url, format!("{BASE}w1280/hZkgoQYus5vegHoetLkCJzb17zJ.jpg"));
}

#[test]
fn backdrop_w300() {
    let url = ImageUrl::backdrop("/path.jpg", BackdropSize::W300);
    assert_eq!(url, format!("{BASE}w300/path.jpg"));
}

#[test]
fn profile_h632() {
    let url = ImageUrl::profile("/profile.jpg", ProfileSize::H632);
    assert_eq!(url, format!("{BASE}h632/profile.jpg"));
}

#[test]
fn profile_w185() {
    let url = ImageUrl::profile("/profile.jpg", ProfileSize::W185);
    assert_eq!(url, format!("{BASE}w185/profile.jpg"));
}

#[test]
fn still_w300() {
    let url = ImageUrl::still("/still.jpg", StillSize::W300);
    assert_eq!(url, format!("{BASE}w300/still.jpg"));
}

#[test]
fn logo_w500() {
    let url = ImageUrl::logo("/logo.png", LogoSize::W500);
    assert_eq!(url, format!("{BASE}w500/logo.png"));
}

#[test]
fn logo_w45() {
    let url = ImageUrl::logo("/logo.png", LogoSize::W45);
    assert_eq!(url, format!("{BASE}w45/logo.png"));
}

#[test]
fn all_poster_sizes() {
    let sizes = [
        (PosterSize::W92, "w92"),
        (PosterSize::W154, "w154"),
        (PosterSize::W185, "w185"),
        (PosterSize::W342, "w342"),
        (PosterSize::W500, "w500"),
        (PosterSize::W780, "w780"),
        (PosterSize::Original, "original"),
    ];
    for (size, label) in sizes {
        let url = ImageUrl::poster("/img.jpg", size);
        assert_eq!(url, format!("{BASE}{label}/img.jpg"), "size {label}");
    }
}
