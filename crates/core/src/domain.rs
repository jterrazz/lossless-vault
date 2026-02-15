use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A photo file tracked in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoFile {
    pub id: i64,
    pub source_id: i64,
    pub path: PathBuf,
    pub size: u64,
    pub format: PhotoFormat,
    pub sha256: String,
    pub phash: Option<u64>,
    pub dhash: Option<u64>,
    pub exif: Option<ExifData>,
    pub mtime: i64,
}

/// Supported photo formats, ordered by quality tier for ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhotoFormat {
    // RAW formats (highest quality)
    Cr2,
    Cr3,
    Nef,
    Arw,
    Orf,
    Raf,
    Rw2,
    Dng,
    // Lossless
    Tiff,
    Png,
    // Lossy / other
    Jpeg,
    Heic,
    Webp,
}

impl PhotoFormat {
    /// Quality tier for ranking (lower = better).
    pub fn quality_tier(&self) -> u8 {
        match self {
            // RAW
            Self::Cr2 | Self::Cr3 | Self::Nef | Self::Arw | Self::Orf | Self::Raf
            | Self::Rw2 | Self::Dng => 0,
            // Lossless
            Self::Tiff => 1,
            Self::Png => 2,
            // Original lossy
            Self::Jpeg => 3,
            Self::Heic => 4,
            Self::Webp => 5,
        }
    }

    /// Whether the `image` crate can decode this format for perceptual hashing.
    pub fn supports_perceptual_hash(&self) -> bool {
        matches!(self, Self::Jpeg | Self::Png | Self::Tiff | Self::Webp)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cr2 => "CR2",
            Self::Cr3 => "CR3",
            Self::Nef => "NEF",
            Self::Arw => "ARW",
            Self::Orf => "ORF",
            Self::Raf => "RAF",
            Self::Rw2 => "RW2",
            Self::Dng => "DNG",
            Self::Tiff => "TIFF",
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Heic => "HEIC",
            Self::Webp => "WebP",
        }
    }
}

impl std::fmt::Display for PhotoFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A duplicate group with its members and elected source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: i64,
    pub members: Vec<PhotoFile>,
    pub source_of_truth_id: i64,
    pub confidence: Confidence,
}

/// Confidence level for a duplicate match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Confidence {
    Low = 0,
    Probable = 1,
    High = 2,
    NearCertain = 3,
    Certain = 4,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Probable => "Probable",
            Self::High => "High",
            Self::NearCertain => "Near-Certain",
            Self::Certain => "Certain",
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Extracted EXIF metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub date: Option<String>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// A registered scan source (directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub path: PathBuf,
    pub last_scanned: Option<i64>,
}

/// Summary statistics for the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogStats {
    pub total_sources: usize,
    pub total_photos: usize,
    pub total_groups: usize,
    pub total_duplicates: usize,
}

/// A file discovered during scanning (before hashing).
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size: u64,
    pub format: PhotoFormat,
    pub mtime: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photo_format_quality_tier_ordering() {
        // RAW formats should have tier 0 (best)
        assert_eq!(PhotoFormat::Cr2.quality_tier(), 0);
        assert_eq!(PhotoFormat::Nef.quality_tier(), 0);
        assert_eq!(PhotoFormat::Dng.quality_tier(), 0);
        assert_eq!(PhotoFormat::Arw.quality_tier(), 0);

        // Lossless
        assert_eq!(PhotoFormat::Tiff.quality_tier(), 1);
        assert_eq!(PhotoFormat::Png.quality_tier(), 2);

        // Lossy — higher tier = worse
        assert!(PhotoFormat::Jpeg.quality_tier() > PhotoFormat::Png.quality_tier());
        assert!(PhotoFormat::Heic.quality_tier() > PhotoFormat::Jpeg.quality_tier());
        assert!(PhotoFormat::Webp.quality_tier() > PhotoFormat::Heic.quality_tier());
    }

    #[test]
    fn test_photo_format_as_str_and_display() {
        assert_eq!(PhotoFormat::Jpeg.as_str(), "JPEG");
        assert_eq!(PhotoFormat::Cr2.as_str(), "CR2");
        assert_eq!(PhotoFormat::Dng.as_str(), "DNG");
        assert_eq!(PhotoFormat::Webp.as_str(), "WebP");
        assert_eq!(format!("{}", PhotoFormat::Png), "PNG");
    }

    #[test]
    fn test_confidence_ordering() {
        assert!(Confidence::Low < Confidence::Probable);
        assert!(Confidence::Probable < Confidence::High);
        assert!(Confidence::High < Confidence::NearCertain);
        assert!(Confidence::NearCertain < Confidence::Certain);
    }

    #[test]
    fn test_confidence_as_str_and_display() {
        assert_eq!(Confidence::Certain.as_str(), "Certain");
        assert_eq!(Confidence::NearCertain.as_str(), "Near-Certain");
        assert_eq!(Confidence::Low.as_str(), "Low");
        assert_eq!(format!("{}", Confidence::High), "High");
    }

    #[test]
    fn test_supports_perceptual_hash() {
        // Formats the image crate can decode
        assert!(PhotoFormat::Jpeg.supports_perceptual_hash());
        assert!(PhotoFormat::Png.supports_perceptual_hash());
        assert!(PhotoFormat::Tiff.supports_perceptual_hash());
        assert!(PhotoFormat::Webp.supports_perceptual_hash());

        // Formats the image crate cannot decode — must NOT attempt
        assert!(!PhotoFormat::Heic.supports_perceptual_hash());
        assert!(!PhotoFormat::Cr2.supports_perceptual_hash());
        assert!(!PhotoFormat::Cr3.supports_perceptual_hash());
        assert!(!PhotoFormat::Nef.supports_perceptual_hash());
        assert!(!PhotoFormat::Arw.supports_perceptual_hash());
        assert!(!PhotoFormat::Orf.supports_perceptual_hash());
        assert!(!PhotoFormat::Raf.supports_perceptual_hash());
        assert!(!PhotoFormat::Rw2.supports_perceptual_hash());
        assert!(!PhotoFormat::Dng.supports_perceptual_hash());
    }
}
