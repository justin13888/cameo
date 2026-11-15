/// Conversions from AniList GraphQL response types to unified types.
#[cfg(feature = "anilist")]
pub mod anilist;

/// A provider's native numeric id as a [`MediaId`](crate::unified::MediaId)
/// number, or `None` when it is not a valid id.
///
/// TMDB and AniList both number their records from `1`, so `0` and negative
/// values name no record. Callers skip a record whose id this rejects (and log
/// it at `warn`) rather than wrap the value or fabricate an id.
#[cfg(any(feature = "tmdb", feature = "anilist"))]
pub(crate) fn native_id(id: i64) -> Option<u64> {
    u64::try_from(id).ok().filter(|&id| id != 0)
}

#[cfg(all(test, any(feature = "tmdb", feature = "anilist")))]
mod tests {
    use super::native_id;

    #[test]
    fn native_id_accepts_positive_ids() {
        assert_eq!(native_id(1), Some(1));
        assert_eq!(native_id(550), Some(550));
        assert_eq!(native_id(i64::MAX), Some(i64::MAX as u64));
    }

    #[test]
    fn native_id_rejects_zero_and_negative_ids() {
        assert_eq!(native_id(0), None);
        assert_eq!(native_id(-1), None);
        assert_eq!(native_id(i64::MIN), None);
    }
}
