use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag, Value};

use crate::domain::ExifData;

/// Extract EXIF metadata from a file. Returns None if EXIF data is unavailable or unreadable.
pub fn extract_exif(path: &Path) -> Option<ExifData> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok()?;

    let date = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::DateTime, In::PRIMARY))
        .map(|f| f.display_value().to_string());

    let camera_make = exif
        .get_field(Tag::Make, In::PRIMARY)
        .map(|f| f.display_value().to_string().trim_matches('"').to_string());

    let camera_model = exif
        .get_field(Tag::Model, In::PRIMARY)
        .map(|f| f.display_value().to_string().trim_matches('"').to_string());

    let gps_lat = extract_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef);
    let gps_lon = extract_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef);

    let width = exif
        .get_field(Tag::PixelXDimension, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::ImageWidth, In::PRIMARY))
        .and_then(|f| match &f.value {
            Value::Long(v) => v.first().copied(),
            Value::Short(v) => v.first().map(|&x| x as u32),
            _ => None,
        });

    let height = exif
        .get_field(Tag::PixelYDimension, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::ImageLength, In::PRIMARY))
        .and_then(|f| match &f.value {
            Value::Long(v) => v.first().copied(),
            Value::Short(v) => v.first().map(|&x| x as u32),
            _ => None,
        });

    // Only return Some if we got at least one useful field
    if date.is_some()
        || camera_make.is_some()
        || camera_model.is_some()
        || gps_lat.is_some()
        || width.is_some()
    {
        Some(ExifData {
            date,
            camera_make,
            camera_model,
            gps_lat,
            gps_lon,
            width,
            height,
        })
    } else {
        None
    }
}

/// Convert GPS DMS (degrees, minutes, seconds) to decimal degrees.
fn extract_gps_coord(
    exif: &exif::Exif,
    coord_tag: Tag,
    ref_tag: Tag,
) -> Option<f64> {
    let field = exif.get_field(coord_tag, In::PRIMARY)?;
    let rationals = match &field.value {
        Value::Rational(v) if v.len() >= 3 => v,
        _ => return None,
    };

    let degrees = rationals[0].to_f64();
    let minutes = rationals[1].to_f64();
    let seconds = rationals[2].to_f64();

    let mut decimal = degrees + minutes / 60.0 + seconds / 3600.0;

    // Check reference direction (S or W means negative)
    if let Some(ref_field) = exif.get_field(ref_tag, In::PRIMARY) {
        let ref_str = ref_field.display_value().to_string();
        if ref_str.contains('S') || ref_str.contains('W') {
            decimal = -decimal;
        }
    }

    Some(decimal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exif_nonexistent_file() {
        let result = extract_exif(Path::new("/nonexistent/photo.jpg"));
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_non_image_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("text.txt");
        std::fs::write(&path, b"not an image").unwrap();

        let result = extract_exif(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_jpeg_without_exif() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("no_exif.jpg");
        // Create a minimal JPEG via the image crate — no EXIF data
        let img = image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([128, 128, 128]));
        img.save(&path).unwrap();

        let result = extract_exif(&path);
        // image crate doesn't write EXIF, so this should be None
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_png_no_exif() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.png");
        let img = image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([0, 0, 0]));
        img.save(&path).unwrap();

        let result = extract_exif(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_exif_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("empty.jpg");
        std::fs::write(&path, b"").unwrap();

        let result = extract_exif(&path);
        assert!(result.is_none());
    }
}
