pub mod formats;

use std::path::Path;

use walkdir::WalkDir;

use crate::domain::ScannedFile;
use crate::error::Result;
use formats::format_from_extension;

/// Recursively scan a directory for supported photo files.
pub fn scan_directory(path: &Path) -> Result<Vec<ScannedFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();

        // Get extension, skip if none or unsupported
        let ext = match file_path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };

        let format = match format_from_extension(&ext) {
            Some(f) => f,
            None => continue,
        };

        // Get metadata for size and mtime
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        files.push(ScannedFile {
            path: file_path.to_path_buf(),
            size: metadata.len(),
            format,
            mtime,
        });
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_directory_finds_photos() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("photo.jpg"), b"fake jpeg").unwrap();
        fs::write(tmp.path().join("photo.png"), b"fake png").unwrap();
        fs::write(tmp.path().join("readme.txt"), b"not a photo").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scan_nested_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("sub/deep");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("photo.cr2"), b"fake raw").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_scan_deeply_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let deep = tmp.path().join("a/b/c/d/e");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.jpg"), b"fake").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("a/b/c/d/e/deep.jpg"));
    }

    #[test]
    fn test_scan_photos_at_multiple_nesting_levels() {
        let tmp = tempfile::tempdir().unwrap();
        let sub1 = tmp.path().join("level1");
        let sub2 = sub1.join("level2");
        let sub3 = sub2.join("level3");
        fs::create_dir_all(&sub3).unwrap();

        fs::write(tmp.path().join("root.jpg"), b"root").unwrap();
        fs::write(sub1.join("one.png"), b"one").unwrap();
        fs::write(sub2.join("two.cr2"), b"two").unwrap();
        fs::write(sub3.join("three.tiff"), b"three").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_scan_multiple_sibling_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let vacation = tmp.path().join("vacation");
        let birthday = tmp.path().join("birthday");
        let work = tmp.path().join("work");
        fs::create_dir_all(&vacation).unwrap();
        fs::create_dir_all(&birthday).unwrap();
        fs::create_dir_all(&work).unwrap();

        fs::write(vacation.join("beach.jpg"), b"beach").unwrap();
        fs::write(vacation.join("sunset.png"), b"sunset").unwrap();
        fs::write(birthday.join("cake.jpg"), b"cake").unwrap();
        fs::write(work.join("notes.txt"), b"not a photo").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_scan_empty_subdirectories_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("empty1/empty2")).unwrap();
        fs::write(tmp.path().join("photo.jpg"), b"photo").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_scan_nested_preserves_full_path() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("2024/06/15");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("photo.jpg"), b"data").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 1);
        // Full path must include the nested directory structure
        assert_eq!(files[0].path, sub.join("photo.jpg"));
    }

    #[test]
    fn test_scan_mixed_supported_unsupported_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("documents");
        let photos = tmp.path().join("photos/raw");
        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&photos).unwrap();

        // Unsupported files scattered in subdirs
        fs::write(docs.join("report.pdf"), b"pdf").unwrap();
        fs::write(docs.join("notes.md"), b"markdown").unwrap();
        fs::write(tmp.path().join("readme.txt"), b"text").unwrap();

        // Supported files in nested subdirs
        fs::write(photos.join("shot.cr2"), b"raw").unwrap();
        fs::write(photos.join("edit.jpg"), b"jpeg").unwrap();

        let files = scan_directory(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scan_symlinked_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let real_dir = tmp.path().join("real_photos");
        let source = tmp.path().join("source");
        fs::create_dir_all(&real_dir).unwrap();
        fs::create_dir_all(&source).unwrap();

        fs::write(real_dir.join("linked.jpg"), b"linked").unwrap();

        // Create symlink: source/link -> real_photos
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_dir, source.join("link")).unwrap();

        let files = scan_directory(&source).unwrap();
        #[cfg(unix)]
        assert_eq!(files.len(), 1);
    }
}
