//! Portable (non-SIMD) Hamming distance implementation.
//!
//! This module provides a safe, platform-independent fallback implementation
//! using standard Rust operations.
//!
//! # Performance
//!
//! Expected: ~300 CPU cycles per comparison (96 iterations × ~3 ops each)
//!
//! # Use Cases
//!
//! 1. Platforms without SIMD support
//! 2. Testing baseline for SIMD implementations
//! 3. Correctness reference implementation

/// Computes Hamming distance for arbitrary-length byte slices.
///
/// # Algorithm
///
/// 1. XOR each byte pair to find differing bits
/// 2. Count set bits using `count_ones()`
/// 3. Sum across all bytes
///
/// # Arguments
///
/// * `a` - First byte slice
/// * `b` - Second byte slice (must be same length as `a`)
///
/// # Returns
///
/// The number of differing bits
///
/// # Panics
///
/// Panics if slices have different lengths.
///
/// # Example
///
/// ```
/// use edgevec::quantization::simd::portable::hamming_distance_slice;
///
/// let a = [0xAA; 32];
/// let b = [0x55; 32];
///
/// let distance = hamming_distance_slice(&a, &b);
/// assert_eq!(distance, 256); // All bits differ (32 * 8)
/// ```
#[inline]
#[must_use]
pub fn hamming_distance_slice(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len(), "Slice lengths must match");

    let mut distance = 0u32;
    for i in 0..a.len() {
        let xor = a[i] ^ b[i];
        distance += xor.count_ones();
    }
    distance
}

/// Computes Hamming distance using portable byte-by-byte operations.
///
/// # Algorithm
///
/// 1. XOR each byte pair to find differing bits
/// 2. Count set bits using `count_ones()`
/// 3. Sum across all 96 bytes
///
/// # Arguments
///
/// * `a` - First 96-byte binary vector
/// * `b` - Second 96-byte binary vector
///
/// # Returns
///
/// The number of differing bits (0..=768)
///
/// # Performance
///
/// This implementation is intentionally simple for correctness and portability.
/// Expected performance: ~300 cycles.
///
/// # Example
///
/// ```
/// use edgevec::quantization::simd::portable::hamming_distance_portable;
///
/// let a = [0xAA; 96]; // 10101010...
/// let b = [0x55; 96]; // 01010101...
///
/// let distance = hamming_distance_portable(&a, &b);
/// assert_eq!(distance, 768); // All bits differ
/// ```
#[inline]
#[must_use]
pub fn hamming_distance_portable(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    let mut distance = 0u32;

    // Byte-by-byte XOR and popcount
    // This is the Day 36 baseline implementation
    for i in 0..96 {
        let xor = a[i] ^ b[i];
        distance += xor.count_ones();
    }

    distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portable_identical() {
        let a = [0xAA; 96];
        let b = [0xAA; 96];
        assert_eq!(hamming_distance_portable(&a, &b), 0);
    }

    #[test]
    fn test_portable_opposite() {
        let a = [0x00; 96];
        let b = [0xFF; 96];
        assert_eq!(hamming_distance_portable(&a, &b), 768);
    }

    #[test]
    fn test_portable_alternating() {
        let a = [0xAA; 96]; // 10101010...
        let b = [0x55; 96]; // 01010101...
        assert_eq!(hamming_distance_portable(&a, &b), 768);
    }

    #[test]
    fn test_portable_half_bits() {
        let a = [0xF0; 96]; // 11110000...
        let b = [0x0F; 96]; // 00001111...
        assert_eq!(hamming_distance_portable(&a, &b), 768);
    }

    #[test]
    fn test_portable_single_bit() {
        let mut a = [0x00; 96];
        let b = [0x00; 96];
        a[0] = 0x01; // Only bit 0 differs

        assert_eq!(hamming_distance_portable(&a, &b), 1);
    }

    #[test]
    fn test_portable_bounds() {
        let a = [0xFF; 96];
        let b = [0x00; 96];
        let distance = hamming_distance_portable(&a, &b);

        assert!(distance <= 768);
        assert_eq!(distance, 768);
    }

    // Tests for slice-based function

    #[test]
    fn test_slice_empty() {
        let a: Vec<u8> = vec![];
        let b: Vec<u8> = vec![];
        assert_eq!(hamming_distance_slice(&a, &b), 0);
    }

    #[test]
    fn test_slice_single_byte() {
        let a = vec![0xFF];
        let b = vec![0x00];
        assert_eq!(hamming_distance_slice(&a, &b), 8);
    }

    #[test]
    fn test_slice_15_bytes() {
        let a = vec![0xFF; 15];
        let b = vec![0x00; 15];
        assert_eq!(hamming_distance_slice(&a, &b), 120); // 15 * 8
    }

    #[test]
    fn test_slice_16_bytes() {
        let a = vec![0xFF; 16];
        let b = vec![0x00; 16];
        assert_eq!(hamming_distance_slice(&a, &b), 128); // 16 * 8
    }

    #[test]
    fn test_slice_17_bytes() {
        let a = vec![0xFF; 17];
        let b = vec![0x00; 17];
        assert_eq!(hamming_distance_slice(&a, &b), 136); // 17 * 8
    }

    #[test]
    fn test_slice_matches_portable_96() {
        let a = [0xAA; 96];
        let b = [0x55; 96];
        assert_eq!(
            hamming_distance_slice(&a, &b),
            hamming_distance_portable(&a, &b)
        );
    }
}
