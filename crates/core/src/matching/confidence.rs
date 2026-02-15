use crate::domain::Confidence;

/// Perceptual hash Hamming distance thresholds (for 64-bit hashes).
/// Research shows max safe threshold for 64-bit hashes is ~5 bits (~8%).
/// Beyond that, false positives increase dramatically at scale.
pub const PHASH_NEAR_CERTAIN_THRESHOLD: u32 = 2;
pub const PHASH_HIGH_THRESHOLD: u32 = 3;
pub const PHASH_PROBABLE_THRESHOLD: u32 = 5;

/// Determine confidence from a perceptual hash Hamming distance.
pub fn confidence_from_hamming(distance: u32) -> Option<Confidence> {
    if distance <= PHASH_NEAR_CERTAIN_THRESHOLD {
        Some(Confidence::NearCertain)
    } else if distance <= PHASH_HIGH_THRESHOLD {
        Some(Confidence::High)
    } else if distance <= PHASH_PROBABLE_THRESHOLD {
        Some(Confidence::Probable)
    } else {
        None
    }
}

/// Combine two confidence levels, taking the lower (more conservative) one.
pub fn combine_confidence(a: Confidence, b: Confidence) -> Confidence {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_from_hamming() {
        assert_eq!(confidence_from_hamming(0), Some(Confidence::NearCertain));
        assert_eq!(confidence_from_hamming(2), Some(Confidence::NearCertain));
        assert_eq!(confidence_from_hamming(3), Some(Confidence::High));
        assert_eq!(confidence_from_hamming(4), Some(Confidence::Probable));
        assert_eq!(confidence_from_hamming(5), Some(Confidence::Probable));
        assert_eq!(confidence_from_hamming(6), None);
        assert_eq!(confidence_from_hamming(10), None);
    }

    #[test]
    fn test_combine_confidence() {
        assert_eq!(combine_confidence(Confidence::Certain, Confidence::High), Confidence::High);
        assert_eq!(combine_confidence(Confidence::Low, Confidence::Certain), Confidence::Low);
    }
}
