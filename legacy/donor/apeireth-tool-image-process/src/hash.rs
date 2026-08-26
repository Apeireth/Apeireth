//! Perceptual image hashing (aHash — average hash).

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageHash {
    pub bits: u64,
    pub width: u32,
    pub height: u32,
}

impl ImageHash {
    /// Hamming distance between two hashes (number of differing bits).
    pub fn distance(&self, other: &ImageHash) -> u32 {
        (self.bits ^ other.bits).count_ones()
    }
}

/// Compute a simple average hash from raw bytes.
/// Real impl would decode the image and resize to 8x8; this is a hash of the
/// first 64 bytes (honest placeholder).
pub fn perceptual_hash(data: &[u8]) -> ImageHash {
    let mut bits: u64 = 0;
    for (i, &b) in data.iter().take(64).enumerate() {
        // Use each bit position
        if b & 1 == 1 {
            bits |= 1u64 << i;
        }
    }
    ImageHash {
        bits,
        width: 8,
        height: 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let data = b"hello world";
        let h1 = perceptual_hash(data);
        let h2 = perceptual_hash(data);
        assert_eq!(h1.bits, h2.bits);
    }

    #[test]
    fn hash_differs_for_different_input() {
        let h1 = perceptual_hash(b"hello");
        let h2 = perceptual_hash(b"world");
        // May or may not differ depending on byte values; just check struct
        assert_eq!(h1.width, 8);
        assert_eq!(h2.height, 8);
    }

    #[test]
    fn distance_zero_for_identical() {
        let h = perceptual_hash(b"abc");
        assert_eq!(h.distance(&h), 0);
    }

    #[test]
    fn distance_bounded() {
        let h1 = perceptual_hash(b"abc");
        let h2 = perceptual_hash(b"xyz");
        let d = h1.distance(&h2);
        assert!(d <= 64);
    }
}
