use std::path::Path;

use img_hash::{HasherConfig, HashAlg};

/// Compute average hash (aHash, via `Mean` algorithm) and difference hash (dHash) for an image.
/// The aHash is stored in the `phash` field for historical reasons.
/// Returns (ahash, dhash) as u64 values, or None if the image cannot be processed.
/// Both hashes are 8x8 = 64-bit. Matching requires dual-hash consensus (both within threshold).
pub fn compute_perceptual_hashes(path: &Path) -> Option<(u64, u64)> {
    // Try img_hash's image crate first (supports JPEG, PNG, etc.)
    let img = img_hash::image::open(path).ok().or_else(|| {
        // Fallback: load with image v0.25 (supports more formats) and convert
        // to a raw RGB buffer that img_hash's image crate can use
        let modern_img = image::open(path).ok()?;
        let rgb = modern_img.to_rgb8();
        let (w, h) = rgb.dimensions();
        img_hash::image::RgbImage::from_raw(w, h, rgb.into_raw())
            .map(img_hash::image::DynamicImage::ImageRgb8)
    })?;

    let phash = compute_hash(&img, HashAlg::Mean)?;
    let dhash = compute_hash(&img, HashAlg::Gradient)?;
    Some((phash, dhash))
}

fn compute_hash(img: &img_hash::image::DynamicImage, alg: HashAlg) -> Option<u64> {
    let hasher = HasherConfig::new()
        .hash_alg(alg)
        .hash_size(8, 8)
        .to_hasher();
    let hash = hasher.hash_image(img);
    Some(hash_to_u64(hash.as_bytes()))
}

fn hash_to_u64(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for (i, &byte) in bytes.iter().take(8).enumerate() {
        result |= (byte as u64) << (i * 8);
    }
    result
}

/// Compute the Hamming distance between two hash values.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_jpeg(path: &Path, r: u8, g: u8, b: u8) {
        let img = image::RgbImage::from_fn(64, 64, |_, _| image::Rgb([r, g, b]));
        img.save(path).unwrap();
    }

    #[test]
    fn test_hamming_distance_identical() {
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(u64::MAX, u64::MAX), 0);
    }

    #[test]
    fn test_hamming_distance_different() {
        assert_eq!(hamming_distance(0, 1), 1);
        assert_eq!(hamming_distance(0, 3), 2);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
    }

    #[test]
    fn test_compute_perceptual_hashes_returns_values() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.jpg");
        create_test_jpeg(&path, 128, 128, 128);

        let result = compute_perceptual_hashes(&path);
        assert!(result.is_some());
        let (phash, dhash) = result.unwrap();
        assert!(phash > 0 || dhash > 0); // At least one should be non-zero
    }

    #[test]
    fn test_identical_images_same_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let path_a = tmp.path().join("a.jpg");
        let path_b = tmp.path().join("b.jpg");
        create_test_jpeg(&path_a, 200, 100, 50);
        create_test_jpeg(&path_b, 200, 100, 50);

        let (phash_a, dhash_a) = compute_perceptual_hashes(&path_a).unwrap();
        let (phash_b, dhash_b) = compute_perceptual_hashes(&path_b).unwrap();
        assert_eq!(phash_a, phash_b);
        assert_eq!(dhash_a, dhash_b);
    }

    #[test]
    fn test_different_images_different_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let path_a = tmp.path().join("gradient.jpg");
        let path_b = tmp.path().join("checkerboard.jpg");

        // Horizontal gradient
        let img_a = image::RgbImage::from_fn(64, 64, |x, _| {
            let v = (x * 4) as u8;
            image::Rgb([v, 0, 0])
        });
        img_a.save(&path_a).unwrap();

        // Checkerboard pattern
        let img_b = image::RgbImage::from_fn(64, 64, |x, y| {
            if (x / 8 + y / 8) % 2 == 0 {
                image::Rgb([255, 255, 255])
            } else {
                image::Rgb([0, 0, 0])
            }
        });
        img_b.save(&path_b).unwrap();

        let (phash_a, _) = compute_perceptual_hashes(&path_a).unwrap();
        let (phash_b, _) = compute_perceptual_hashes(&path_b).unwrap();
        assert_ne!(phash_a, phash_b);
    }

    #[test]
    fn test_nonexistent_file_returns_none() {
        let result = compute_perceptual_hashes(Path::new("/nonexistent/image.jpg"));
        assert!(result.is_none());
    }

    #[test]
    fn test_non_image_file_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("not_an_image.jpg");
        std::fs::write(&path, b"this is not a jpeg").unwrap();

        let result = compute_perceptual_hashes(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_hash_to_u64_small_input() {
        assert_eq!(hash_to_u64(&[]), 0);
        assert_eq!(hash_to_u64(&[0xFF]), 0xFF);
        assert_eq!(hash_to_u64(&[0, 0xFF]), 0xFF00);
    }

    #[test]
    fn test_png_support() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.png");
        let img = image::RgbImage::from_fn(32, 32, |_, _| image::Rgb([100, 150, 200]));
        img.save(&path).unwrap();

        let result = compute_perceptual_hashes(&path);
        assert!(result.is_some());
    }
}
