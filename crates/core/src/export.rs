use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// Progress callback events for the export operation.
pub enum ExportProgress {
    /// Starting export with total count.
    Start { total: usize },
    /// A file was converted to HEIC.
    Converted { source: PathBuf, target: PathBuf },
    /// A file was skipped (already exists).
    Skipped { path: PathBuf },
    /// Export completed.
    Complete { converted: usize, skipped: usize },
}

/// Check if the `sips` command is available on this system.
pub fn check_sips_available() -> Result<()> {
    let output = Command::new("which")
        .arg("sips")
        .output()
        .map_err(|_| Error::SipsNotAvailable)?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::SipsNotAvailable)
    }
}

/// Convert a photo to HEIC format using the macOS `sips` command.
/// Quality: 0–100 (85 recommended for high quality).
pub fn convert_to_heic(source: &Path, target: &Path, quality: u8) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let output = Command::new("sips")
        .arg("-s")
        .arg("format")
        .arg("heic")
        .arg("-s")
        .arg("formatOptions")
        .arg(quality.to_string())
        .arg(source)
        .arg("--out")
        .arg(target)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ConversionFailed {
            path: source.to_path_buf(),
            message: stderr.to_string(),
        });
    }

    Ok(())
}

/// Build the export target path: export_dir/YYYY/MM/DD/stem.heic
/// Returns the existing path if a file already exists (enables incremental skip).
/// Handles collisions with _1, _2, etc. suffixes.
pub fn build_export_path(
    export_dir: &Path,
    date: (u32, u32, u32),
    original_path: &Path,
) -> PathBuf {
    let (year, month, day) = date;
    let dir = export_dir
        .join(format!("{:04}", year))
        .join(format!("{:02}", month))
        .join(format!("{:02}", day));

    let file_stem = original_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let base_name = format!("{}.heic", file_stem);
    let target = dir.join(&base_name);

    if !target.exists() {
        return target;
    }

    // Target exists — skip (return existing path for incremental behavior)
    target
}

/// Export a single photo to HEIC.
/// Returns `Ok(false)` if skipped (target exists), `Ok(true)` if converted.
pub fn export_photo_to_heic(source: &Path, target: &Path, quality: u8) -> Result<bool> {
    if target.exists() {
        return Ok(false);
    }

    convert_to_heic(source, target, quality)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_export_path ───────────────────────────────────────────

    #[test]
    fn test_build_export_path_basic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 6, 15),
            Path::new("/source/photo.jpg"),
        );
        assert_eq!(target, PathBuf::from("/export/2024/06/15/photo.heic"));
    }

    #[test]
    fn test_build_export_path_changes_jpg_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/IMG_1234.jpg"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "IMG_1234");
    }

    #[test]
    fn test_build_export_path_changes_cr2_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/IMG_1234.CR2"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "IMG_1234");
    }

    #[test]
    fn test_build_export_path_changes_png_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 3, 20),
            Path::new("/photos/screenshot.png"),
        );
        assert_eq!(target, PathBuf::from("/export/2024/03/20/screenshot.heic"));
    }

    #[test]
    fn test_build_export_path_changes_tiff_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/scan.tiff"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "scan");
    }

    #[test]
    fn test_build_export_path_changes_dng_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/RAW_5432.DNG"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "RAW_5432");
    }

    #[test]
    fn test_build_export_path_changes_nef_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 5, 10),
            Path::new("/photos/DSC_0001.NEF"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "DSC_0001");
    }

    #[test]
    fn test_build_export_path_changes_webp_to_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/image.webp"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
    }

    #[test]
    fn test_build_export_path_heic_source_stays_heic() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 1),
            Path::new("/photos/already.heic"),
        );
        assert_eq!(target.extension().unwrap(), "heic");
        assert_eq!(target.file_stem().unwrap(), "already");
    }

    #[test]
    fn test_build_export_path_zero_padding() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 1, 5),
            Path::new("/source/photo.png"),
        );
        assert_eq!(target, PathBuf::from("/export/2024/01/05/photo.heic"));
    }

    #[test]
    fn test_build_export_path_no_extension() {
        let target = build_export_path(
            Path::new("/export"),
            (2024, 6, 15),
            Path::new("/source/noext"),
        );
        assert_eq!(target.file_name().unwrap(), "noext.heic");
    }

    #[test]
    fn test_build_export_path_skip_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let date_dir = tmp.path().join("2024/06/15");
        fs::create_dir_all(&date_dir).unwrap();
        fs::write(date_dir.join("photo.heic"), b"existing").unwrap();

        let target = build_export_path(tmp.path(), (2024, 6, 15), Path::new("/source/photo.jpg"));
        assert_eq!(target.file_name().unwrap(), "photo.heic");
        assert!(target.exists());
    }

    #[test]
    fn test_build_export_path_different_dates_no_collision() {
        let export_dir = Path::new("/export");
        let t1 = build_export_path(export_dir, (2024, 1, 1), Path::new("/a/photo.jpg"));
        let t2 = build_export_path(export_dir, (2024, 1, 2), Path::new("/b/photo.jpg"));
        assert_ne!(t1, t2);
        assert_eq!(t1, PathBuf::from("/export/2024/01/01/photo.heic"));
        assert_eq!(t2, PathBuf::from("/export/2024/01/02/photo.heic"));
    }

    #[test]
    fn test_build_export_path_different_stems_no_collision() {
        let export_dir = Path::new("/export");
        let t1 = build_export_path(export_dir, (2024, 1, 1), Path::new("/a/sunset.jpg"));
        let t2 = build_export_path(export_dir, (2024, 1, 1), Path::new("/b/portrait.jpg"));
        assert_ne!(t1, t2);
    }

    // ── export_photo_to_heic ────────────────────────────────────────

    #[test]
    fn test_export_photo_to_heic_skips_existing_target() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        let target = tmp.path().join("target.heic");
        fs::write(&source, b"jpeg data").unwrap();
        fs::write(&target, b"existing heic").unwrap();

        let result = export_photo_to_heic(&source, &target, 85).unwrap();
        assert!(!result, "should skip when target exists");
        // Content should NOT change
        assert_eq!(fs::read(&target).unwrap(), b"existing heic");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_export_photo_to_heic_converts_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        let target = tmp.path().join("output.heic");

        // Create a real JPEG
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&source).unwrap();

        let result = export_photo_to_heic(&source, &target, 85).unwrap();
        assert!(result, "should convert when target doesn't exist");
        assert!(target.exists());
        assert!(target.metadata().unwrap().len() > 0);
    }

    // ── convert_to_heic ─────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn test_convert_to_heic_creates_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        let target = tmp.path().join("deep/nested/dir/output.heic");

        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&source).unwrap();

        convert_to_heic(&source, &target, 85).unwrap();
        assert!(target.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_convert_to_heic_invalid_source_produces_no_output() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("nonexistent.jpg");
        let target = tmp.path().join("output.heic");

        // sips exits 0 even for missing files, but produces no output file
        let _ = convert_to_heic(&source, &target, 85);
        assert!(!target.exists(), "no output should be created for missing source");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_convert_to_heic_output_differs_from_source() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        let target = tmp.path().join("output.heic");

        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&source).unwrap();

        convert_to_heic(&source, &target, 85).unwrap();

        let source_bytes = fs::read(&source).unwrap();
        let target_bytes = fs::read(&target).unwrap();
        assert_ne!(source_bytes, target_bytes, "HEIC output should differ from JPEG source");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_convert_to_heic_quality_affects_size() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source.jpg");
        let target_low = tmp.path().join("low.heic");
        let target_high = tmp.path().join("high.heic");

        let img = image::RgbImage::from_fn(128, 128, |x, y| {
            image::Rgb([(x * 2) as u8, (y * 2) as u8, 100])
        });
        img.save(&source).unwrap();

        convert_to_heic(&source, &target_low, 10).unwrap();
        convert_to_heic(&source, &target_high, 100).unwrap();

        let low_size = target_low.metadata().unwrap().len();
        let high_size = target_high.metadata().unwrap().len();
        assert!(
            high_size > low_size,
            "quality 100 ({high_size}) should be larger than quality 10 ({low_size})"
        );
    }

    // ── check_sips_available ────────────────────────────────────────

    #[test]
    fn test_check_sips_available_on_macos() {
        #[cfg(target_os = "macos")]
        {
            assert!(check_sips_available().is_ok());
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert!(check_sips_available().is_err());
        }
    }
}
