use crate::domain::PhotoFormat;

/// Map a file extension (lowercase, without dot) to a PhotoFormat.
pub fn format_from_extension(ext: &str) -> Option<PhotoFormat> {
    match ext {
        "jpg" | "jpeg" => Some(PhotoFormat::Jpeg),
        "png" => Some(PhotoFormat::Png),
        "tif" | "tiff" => Some(PhotoFormat::Tiff),
        "webp" => Some(PhotoFormat::Webp),
        "heic" | "heif" => Some(PhotoFormat::Heic),
        "cr2" => Some(PhotoFormat::Cr2),
        "cr3" => Some(PhotoFormat::Cr3),
        "nef" => Some(PhotoFormat::Nef),
        "arw" => Some(PhotoFormat::Arw),
        "orf" => Some(PhotoFormat::Orf),
        "raf" => Some(PhotoFormat::Raf),
        "rw2" => Some(PhotoFormat::Rw2),
        "dng" => Some(PhotoFormat::Dng),
        _ => None,
    }
}

/// All supported file extensions.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "tif", "tiff", "webp", "heic", "heif",
    "cr2", "cr3", "nef", "arw", "orf", "raf", "rw2", "dng",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_extensions() {
        assert_eq!(format_from_extension("jpg"), Some(PhotoFormat::Jpeg));
        assert_eq!(format_from_extension("jpeg"), Some(PhotoFormat::Jpeg));
        assert_eq!(format_from_extension("cr2"), Some(PhotoFormat::Cr2));
        assert_eq!(format_from_extension("dng"), Some(PhotoFormat::Dng));
        assert_eq!(format_from_extension("heic"), Some(PhotoFormat::Heic));
    }

    #[test]
    fn test_unknown_extension() {
        assert_eq!(format_from_extension("txt"), None);
        assert_eq!(format_from_extension("mp4"), None);
    }
}
