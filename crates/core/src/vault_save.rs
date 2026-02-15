use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{DuplicateGroup, PhotoFile, PhotoFormat};
use crate::error::Result;
use crate::manifest::Manifest;

/// Progress callback events for the vault save operation.
pub enum VaultSaveProgress {
    /// Starting save with total count.
    Start { total: usize },
    /// A file was copied.
    Copied { source: PathBuf, target: PathBuf },
    /// A file was skipped (already exists).
    Skipped { path: PathBuf },
    /// A stale file was removed from the pack.
    Removed { path: PathBuf },
    /// Save completed.
    Complete {
        copied: usize,
        skipped: usize,
        removed: usize,
    },
}

/// Parse an EXIF date string into (year, month, day).
/// Handles both "2024-01-15 12:00:00" (display_value) and "2024:01:15 12:00:00" (raw EXIF).
pub fn parse_exif_date(date_str: &str) -> Option<(u32, u32, u32)> {
    let date_part = date_str.split_whitespace().next()?;
    let parts: Vec<&str> = date_part.split([':', '-']).collect();
    if parts.len() < 3 {
        return None;
    }
    let year: u32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if !(1970..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some((year, month, day))
}

/// Extract (year, month, day) from a photo's EXIF date, falling back to mtime.
pub fn date_for_photo(photo: &PhotoFile) -> (u32, u32, u32) {
    if let Some(ref exif) = photo.exif {
        if let Some(ref date_str) = exif.date {
            if let Some(date) = parse_exif_date(date_str) {
                return date;
            }
        }
    }

    // Fallback to mtime
    let dt = chrono::DateTime::from_timestamp(photo.mtime, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    use chrono::Datelike;
    (dt.year() as u32, dt.month(), dt.day())
}

/// Build a content-addressable path: `pack_path/{sha256[..2]}/{sha256}.{ext}`
/// Pure function — no I/O, no collision handling needed.
pub fn build_content_path(pack_path: &Path, sha256: &str, format: PhotoFormat) -> PathBuf {
    let prefix = &sha256[..2];
    pack_path
        .join(prefix)
        .join(format!("{}.{}", sha256, format.extension()))
}

/// Determine which photos to save to the vault:
/// - For each duplicate group, take only the source-of-truth.
/// - For ungrouped photos, take the photo itself.
pub fn select_photos_to_export<'a>(
    all_photos: &'a [PhotoFile],
    groups: &[DuplicateGroup],
) -> Vec<&'a PhotoFile> {
    let mut grouped_ids: HashSet<i64> = HashSet::new();
    let mut sot_ids: HashSet<i64> = HashSet::new();

    for group in groups {
        for member in &group.members {
            grouped_ids.insert(member.id);
        }
        sot_ids.insert(group.source_of_truth_id);
    }

    all_photos
        .iter()
        .filter(|p| {
            if grouped_ids.contains(&p.id) {
                sot_ids.contains(&p.id)
            } else {
                true
            }
        })
        .collect()
}

/// Copy a single file to a content-addressed target path.
/// Returns Ok(false) if skipped (file already exists — content-addressed: existence = correct).
/// Returns Ok(true) if copied.
pub fn copy_photo_to_pack(source: &Path, target: &Path) -> Result<bool> {
    if target.exists() {
        return Ok(false);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, target)?;
    Ok(true)
}

/// Remove pack files whose hashes are not in `desired_hashes`.
/// Queries the manifest for all entries, removes stale files from disk and manifest.
/// Returns the list of removed file paths.
pub fn cleanup_pack_files(
    pack_path: &Path,
    desired_hashes: &HashSet<String>,
    manifest: &Manifest,
) -> Vec<PathBuf> {
    let entries = match manifest.list_entries() {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut removed = Vec::new();
    for (sha256, format_str) in &entries {
        if !desired_hashes.contains(sha256.as_str()) {
            // Reconstruct format from string to get extension
            let ext = format_str_to_extension(format_str);
            let prefix = &sha256[..2];
            let file_path = pack_path
                .join(prefix)
                .join(format!("{}.{}", sha256, ext));
            if fs::remove_file(&file_path).is_ok() {
                removed.push(file_path);
            }
            let _ = manifest.remove(sha256);
        }
    }

    removed
}

/// Map a format string (as stored in manifest) back to file extension.
fn format_str_to_extension(format_str: &str) -> &str {
    match format_str {
        "CR2" => "cr2",
        "CR3" => "cr3",
        "NEF" => "nef",
        "ARW" => "arw",
        "ORF" => "orf",
        "RAF" => "raf",
        "RW2" => "rw2",
        "DNG" => "dng",
        "TIFF" => "tiff",
        "PNG" => "png",
        "JPEG" => "jpg",
        "HEIC" => "heic",
        "WebP" => "webp",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::*;

    // ── parse_exif_date ─────────────────────────────────────────

    #[test]
    fn test_parse_exif_date_hyphenated() {
        assert_eq!(
            parse_exif_date("2024-06-15 12:30:00"),
            Some((2024, 6, 15))
        );
    }

    #[test]
    fn test_parse_exif_date_colons() {
        assert_eq!(
            parse_exif_date("2024:01:15 12:00:00"),
            Some((2024, 1, 15))
        );
    }

    #[test]
    fn test_parse_exif_date_date_only() {
        assert_eq!(parse_exif_date("2024:01:01"), Some((2024, 1, 1)));
    }

    #[test]
    fn test_parse_exif_date_invalid() {
        assert_eq!(parse_exif_date("not-a-date"), None);
        assert_eq!(parse_exif_date(""), None);
    }

    #[test]
    fn test_parse_exif_date_out_of_range() {
        assert_eq!(parse_exif_date("1969:01:01 00:00:00"), None);
        assert_eq!(parse_exif_date("2024:13:01 00:00:00"), None);
        assert_eq!(parse_exif_date("2024:01:32 00:00:00"), None);
    }

    // ── date_for_photo ──────────────────────────────────────────

    fn make_photo(id: i64, mtime: i64) -> PhotoFile {
        PhotoFile {
            id,
            source_id: 1,
            path: PathBuf::from(format!("/test/{id}.jpg")),
            size: 1000,
            format: PhotoFormat::Jpeg,
            sha256: format!("sha_{id}"),
            phash: None,
            dhash: None,
            exif: None,
            mtime,
        }
    }

    #[test]
    fn test_date_for_photo_uses_exif() {
        let mut photo = make_photo(1, 0);
        photo.exif = Some(ExifData {
            date: Some("2024-06-15 12:30:00".to_string()),
            camera_make: None,
            camera_model: None,
            gps_lat: None,
            gps_lon: None,
            width: None,
            height: None,
        });
        assert_eq!(date_for_photo(&photo), (2024, 6, 15));
    }

    #[test]
    fn test_date_for_photo_falls_back_to_mtime() {
        // 1718444400 = 2024-06-15 11:00:00 UTC
        let photo = make_photo(1, 1718444400);
        let (year, month, day) = date_for_photo(&photo);
        assert_eq!(year, 2024);
        assert_eq!(month, 6);
        assert_eq!(day, 15);
    }

    // ── select_photos_to_export ─────────────────────────────────

    fn make_photo_with_path(id: i64, path: &str) -> PhotoFile {
        let mut p = make_photo(id, 1000);
        p.path = PathBuf::from(path);
        p
    }

    #[test]
    fn test_select_ungrouped_all_included() {
        let photos = vec![
            make_photo_with_path(1, "/a.jpg"),
            make_photo_with_path(2, "/b.jpg"),
        ];
        let selected = select_photos_to_export(&photos, &[]);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_select_group_only_sot() {
        let photos = vec![
            make_photo_with_path(1, "/a.jpg"),
            make_photo_with_path(2, "/b.jpg"),
            make_photo_with_path(3, "/c.jpg"),
        ];
        let groups = vec![DuplicateGroup {
            id: 1,
            members: vec![photos[0].clone(), photos[1].clone()],
            source_of_truth_id: 1,
            confidence: Confidence::Certain,
        }];
        let selected = select_photos_to_export(&photos, &groups);
        assert_eq!(selected.len(), 2);
        let ids: HashSet<i64> = selected.iter().map(|p| p.id).collect();
        assert!(ids.contains(&1), "SoT should be included");
        assert!(ids.contains(&3), "ungrouped should be included");
        assert!(!ids.contains(&2), "non-SoT group member should be excluded");
    }

    // ── date_for_photo edge cases ───────────────────────────────

    #[test]
    fn test_date_for_photo_invalid_exif_falls_back_to_mtime() {
        let mut photo = make_photo(1, 1718444400); // 2024-06-15
        photo.exif = Some(ExifData {
            date: Some("garbage".to_string()),
            camera_make: None,
            camera_model: None,
            gps_lat: None,
            gps_lon: None,
            width: None,
            height: None,
        });
        let (year, month, day) = date_for_photo(&photo);
        assert_eq!(year, 2024);
        assert_eq!(month, 6);
        assert_eq!(day, 15);
    }

    #[test]
    fn test_date_for_photo_exif_no_date_falls_back_to_mtime() {
        let mut photo = make_photo(1, 1718444400); // 2024-06-15
        photo.exif = Some(ExifData {
            date: None,
            camera_make: Some("Canon".to_string()),
            camera_model: None,
            gps_lat: None,
            gps_lon: None,
            width: None,
            height: None,
        });
        let (year, month, day) = date_for_photo(&photo);
        assert_eq!(year, 2024);
        assert_eq!(month, 6);
        assert_eq!(day, 15);
    }

    // ── select_photos_to_export edge cases ──────────────────────

    #[test]
    fn test_select_photos_multiple_groups() {
        let photos = vec![
            make_photo_with_path(1, "/a.jpg"),
            make_photo_with_path(2, "/b.jpg"),
            make_photo_with_path(3, "/c.jpg"),
            make_photo_with_path(4, "/d.jpg"),
            make_photo_with_path(5, "/e.jpg"),
        ];
        let groups = vec![
            DuplicateGroup {
                id: 1,
                members: vec![photos[0].clone(), photos[1].clone()],
                source_of_truth_id: 1,
                confidence: Confidence::Certain,
            },
            DuplicateGroup {
                id: 2,
                members: vec![photos[2].clone(), photos[3].clone()],
                source_of_truth_id: 3,
                confidence: Confidence::High,
            },
        ];
        let selected = select_photos_to_export(&photos, &groups);
        // SoT 1 from group 1 + SoT 3 from group 2 + ungrouped 5 = 3
        assert_eq!(selected.len(), 3);
        let ids: HashSet<i64> = selected.iter().map(|p| p.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
        assert!(ids.contains(&5));
    }

    #[test]
    fn test_select_photos_all_grouped() {
        let photos = vec![
            make_photo_with_path(1, "/a.jpg"),
            make_photo_with_path(2, "/b.jpg"),
        ];
        let groups = vec![DuplicateGroup {
            id: 1,
            members: vec![photos[0].clone(), photos[1].clone()],
            source_of_truth_id: 2,
            confidence: Confidence::Certain,
        }];
        let selected = select_photos_to_export(&photos, &groups);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, 2);
    }

    #[test]
    fn test_select_photos_empty_input() {
        let selected = select_photos_to_export(&[], &[]);
        assert!(selected.is_empty());
    }

    // ── build_content_path ──────────────────────────────────────

    #[test]
    fn test_build_content_path_basic() {
        let pack = PathBuf::from("/pack");
        let sha = "a3b1c9e8f7001122334455667788990011223344556677889900aabbccddeeff";
        let target = build_content_path(&pack, sha, PhotoFormat::Jpeg);
        assert_eq!(
            target,
            PathBuf::from("/pack/a3/a3b1c9e8f7001122334455667788990011223344556677889900aabbccddeeff.jpg")
        );
    }

    #[test]
    fn test_build_content_path_different_formats() {
        let pack = PathBuf::from("/pack");
        let sha = "a3b1c9e8f7001122334455667788990011223344556677889900aabbccddeeff";

        let cr2_path = build_content_path(&pack, sha, PhotoFormat::Cr2);
        let jpg_path = build_content_path(&pack, sha, PhotoFormat::Jpeg);
        let png_path = build_content_path(&pack, sha, PhotoFormat::Png);

        assert!(cr2_path.to_string_lossy().ends_with(".cr2"));
        assert!(jpg_path.to_string_lossy().ends_with(".jpg"));
        assert!(png_path.to_string_lossy().ends_with(".png"));

        // Same prefix directory
        assert_eq!(cr2_path.parent(), jpg_path.parent());
    }

    // ── copy_photo_to_pack ──────────────────────────────────────

    #[test]
    fn test_copy_photo_to_pack_creates_prefix_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        fs::write(&source, b"photo data").unwrap();

        let target = tmp.path().join("a3/abcdef1234.jpg");
        let result = copy_photo_to_pack(&source, &target).unwrap();
        assert!(result, "should copy when target doesn't exist");
        assert!(target.exists());
        assert_eq!(fs::read(&target).unwrap(), b"photo data");
    }

    #[test]
    fn test_copy_photo_to_pack_skips_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        fs::write(&source, b"photo data").unwrap();

        let target = tmp.path().join("target.jpg");
        fs::write(&target, b"existing content").unwrap();

        let result = copy_photo_to_pack(&source, &target).unwrap();
        assert!(!result, "should skip when target exists (content-addressed)");
        // Content should NOT be overwritten
        assert_eq!(fs::read(&target).unwrap(), b"existing content");
    }

    #[test]
    fn test_copy_photo_to_pack_source_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("nonexistent.jpg");
        let target = tmp.path().join("target.jpg");

        let result = copy_photo_to_pack(&source, &target);
        assert!(result.is_err());
    }
}
