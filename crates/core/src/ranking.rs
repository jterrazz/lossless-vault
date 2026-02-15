use crate::domain::PhotoFile;

/// Elect the source of truth from a group of duplicate photo references.
///
/// Priority:
/// 1. Lowest format quality tier (RAW > TIFF > PNG > JPEG > HEIC > WebP)
/// 2. Largest file size
/// 3. Oldest mtime (earliest capture is likely the original)
pub fn elect_source_of_truth<'a>(members: &[&'a PhotoFile]) -> &'a PhotoFile {
    assert!(!members.is_empty(), "cannot elect from empty group");

    members
        .iter()
        .min_by(|a, b| {
            a.format
                .quality_tier()
                .cmp(&b.format.quality_tier())
                .then(b.size.cmp(&a.size))
                .then(a.mtime.cmp(&b.mtime))
        })
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PhotoFile, PhotoFormat};
    use std::path::PathBuf;

    fn make_photo(id: i64, format: PhotoFormat, size: u64, mtime: i64) -> PhotoFile {
        PhotoFile {
            id,
            source_id: 1,
            path: PathBuf::from(format!("/test/{id}.jpg")),
            size,
            format,
            sha256: "hash".to_string(),
            phash: None,
            dhash: None,
            exif: None,
            mtime,
        }
    }

    #[test]
    fn test_raw_beats_jpeg() {
        let photos = vec![
            make_photo(1, PhotoFormat::Jpeg, 5_000_000, 1000),
            make_photo(2, PhotoFormat::Cr2, 20_000_000, 1000),
        ];
        let members: Vec<&PhotoFile> = photos.iter().collect();
        let winner = elect_source_of_truth(&members);
        assert_eq!(winner.id, 2);
    }

    #[test]
    fn test_larger_file_wins_same_format() {
        let photos = vec![
            make_photo(1, PhotoFormat::Jpeg, 3_000_000, 1000),
            make_photo(2, PhotoFormat::Jpeg, 5_000_000, 1000),
        ];
        let members: Vec<&PhotoFile> = photos.iter().collect();
        let winner = elect_source_of_truth(&members);
        assert_eq!(winner.id, 2);
    }

    #[test]
    fn test_older_mtime_wins_tiebreak() {
        let photos = vec![
            make_photo(1, PhotoFormat::Jpeg, 5_000_000, 2000),
            make_photo(2, PhotoFormat::Jpeg, 5_000_000, 1000),
        ];
        let members: Vec<&PhotoFile> = photos.iter().collect();
        let winner = elect_source_of_truth(&members);
        assert_eq!(winner.id, 2);
    }
}
