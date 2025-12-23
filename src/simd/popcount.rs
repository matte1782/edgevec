//! SIMD-accelerated popcount for Hamming distance computation.
//!
//! This module provides hardware-accelerated XOR + popcount operations
//! for computing Hamming distance between binary vectors of arbitrary length.
//!
//! # Supported Platforms
//!
//! - **x86_64 with AVX2**: Uses `_mm256_xor_si256` + native popcnt extraction
//! - **x86_64 with popcnt**: Uses native `popcnt` instruction on u64 chunks
//! - **aarch64 with NEON**: Uses `veorq_u8` + `vcntq_u8` for parallel popcount
//! - **All platforms**: Scalar fallback using `count_ones()`
//!
//! # Performance
//!
//! | Platform | Expected Speedup vs Scalar |
//! |----------|---------------------------|
//! | AVX2     | 3-5x                      |
//! | popcnt   | 2-3x                      |
//! | NEON     | 2-3x                      |
//! | Scalar   | 1x (baseline)             |
//!
//! # Example
//!
//! ```
//! use edgevec::simd::popcount::{simd_popcount_xor, scalar_popcount_xor};
//!
//! let a = vec![0xAA; 96]; // 10101010...
//! let b = vec![0x55; 96]; // 01010101...
//!
//! let distance = simd_popcount_xor(&a, &b);
//! assert_eq!(distance, 768); // All bits differ
//!
//! // Scalar version for comparison
//! let scalar_distance = scalar_popcount_xor(&a, &b);
//! assert_eq!(distance, scalar_distance);
//! ```

/// Computes popcount of XOR between two byte slices.
///
/// Uses the fastest available implementation:
/// - AVX2 (x86_64 with AVX2): 32-byte parallel processing
/// - popcnt (x86_64 with popcnt): Native instruction per u64
/// - NEON (aarch64): 16-byte parallel processing
/// - Scalar: Portable fallback using `count_ones()`
///
/// # Arguments
///
/// * `a` - First byte slice.
/// * `b` - Second byte slice (must have same length as `a`).
///
/// # Returns
///
/// The total number of bits that differ between `a` and `b`.
///
/// # Panics
///
/// Panics in debug mode if `a.len() != b.len()`.
///
/// # Example
///
/// ```
/// use edgevec::simd::popcount::simd_popcount_xor;
///
/// let a = [0xFF; 16]; // All ones
/// let b = [0x00; 16]; // All zeros
///
/// let distance = simd_popcount_xor(&a, &b);
/// assert_eq!(distance, 128); // 16 * 8 = 128 bits
/// ```
#[inline]
#[must_use]
pub fn simd_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len(), "slices must have equal length");

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: We verified AVX2 is available
            return unsafe { avx2_popcount_xor(a, b) };
        }
        if is_x86_feature_detected!("popcnt") {
            return native_popcount_xor(a, b);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            // SAFETY: We verified NEON is available
            return unsafe { neon_popcount_xor(a, b) };
        }
    }

    scalar_popcount_xor(a, b)
}

/// Scalar (non-SIMD) popcount of XOR.
///
/// This is the baseline implementation used when SIMD is not available.
/// Also useful for testing to verify SIMD correctness.
///
/// # Arguments
///
/// * `a` - First byte slice.
/// * `b` - Second byte slice (must have same length as `a`).
///
/// # Returns
///
/// The total number of bits that differ between `a` and `b`.
#[inline]
#[must_use]
pub fn scalar_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

/// x86_64 AVX2 implementation using native popcnt instruction.
///
/// Processes 32 bytes at a time using AVX2 XOR, then extracts 4×u64 values
/// and uses native popcnt via count_ones(). This is faster than the PSHUFB
/// lookup table method on modern CPUs (Haswell+) that have hardware popcnt.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
unsafe fn avx2_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    use std::arch::x86_64::{__m256i, _mm256_extract_epi64, _mm256_loadu_si256, _mm256_xor_si256};

    let mut total = 0u32;
    let len = a.len();

    // Process 32 bytes at a time
    let chunks = len / 32;
    for i in 0..chunks {
        let offset = i * 32;
        let va = _mm256_loadu_si256(a.as_ptr().add(offset).cast::<__m256i>());
        let vb = _mm256_loadu_si256(b.as_ptr().add(offset).cast::<__m256i>());

        // XOR the vectors
        let xor = _mm256_xor_si256(va, vb);

        // Extract 4×u64 and use native popcnt instruction.
        // count_ones() compiles to popcnt on x86_64 with hardware support.
        let v0 = _mm256_extract_epi64(xor, 0) as u64;
        let v1 = _mm256_extract_epi64(xor, 1) as u64;
        let v2 = _mm256_extract_epi64(xor, 2) as u64;
        let v3 = _mm256_extract_epi64(xor, 3) as u64;

        total += v0.count_ones() + v1.count_ones() + v2.count_ones() + v3.count_ones();
    }

    // Handle remainder with scalar
    let remainder_start = chunks * 32;
    for i in remainder_start..len {
        total += (a[i] ^ b[i]).count_ones();
    }

    total
}

/// x86_64 native popcnt implementation.
///
/// Uses the native `popcnt` instruction on u64 chunks.
///
/// # Complexity
///
/// O(n/8) where n is the byte length of the input slices.
#[cfg(target_arch = "x86_64")]
fn native_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    let mut count = 0u32;

    // Process 8 bytes at a time using u64.
    // chunks_exact guarantees each chunk is exactly 8 bytes.
    for (chunk_a, chunk_b) in a.chunks_exact(8).zip(b.chunks_exact(8)) {
        // SAFETY: chunks_exact(8) guarantees exactly 8 bytes.
        // Using array indexing is panic-free since chunk length is guaranteed.
        let va = u64::from_le_bytes([
            chunk_a[0], chunk_a[1], chunk_a[2], chunk_a[3], chunk_a[4], chunk_a[5], chunk_a[6],
            chunk_a[7],
        ]);
        let vb = u64::from_le_bytes([
            chunk_b[0], chunk_b[1], chunk_b[2], chunk_b[3], chunk_b[4], chunk_b[5], chunk_b[6],
            chunk_b[7],
        ]);
        count += (va ^ vb).count_ones();
    }

    // Handle remainder (0-7 bytes)
    let remainder_start = (a.len() / 8) * 8;
    for i in remainder_start..a.len() {
        count += (a[i] ^ b[i]).count_ones();
    }

    count
}

/// ARM64 NEON implementation.
///
/// Uses `veorq_u8` for XOR and `vcntq_u8` for parallel popcount.
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    use std::arch::aarch64::*;

    let mut total = 0u32;
    let len = a.len();

    // Process 16 bytes at a time
    let chunks = len / 16;
    for i in 0..chunks {
        let offset = i * 16;
        let va = vld1q_u8(a.as_ptr().add(offset));
        let vb = vld1q_u8(b.as_ptr().add(offset));

        // XOR and popcount
        let xor = veorq_u8(va, vb);
        let cnt = vcntq_u8(xor);

        // Horizontal sum: u8x16 -> u16x8 -> u32x4 -> u64x2
        let sum16 = vpaddlq_u8(cnt);
        let sum32 = vpaddlq_u16(sum16);
        let sum64 = vpaddlq_u32(sum32);

        // Extract and accumulate
        total += (vgetq_lane_u64(sum64, 0) + vgetq_lane_u64(sum64, 1)) as u32;
    }

    // Handle remainder with scalar
    let remainder_start = chunks * 16;
    for i in remainder_start..len {
        total += (a[i] ^ b[i]).count_ones();
    }

    total
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_popcount_identical() {
        let a = vec![0xAA; 96];
        let b = vec![0xAA; 96];
        assert_eq!(scalar_popcount_xor(&a, &b), 0);
    }

    #[test]
    fn test_scalar_popcount_opposite() {
        let a = vec![0x00; 96];
        let b = vec![0xFF; 96];
        assert_eq!(scalar_popcount_xor(&a, &b), 768);
    }

    #[test]
    fn test_scalar_popcount_half() {
        let a = vec![0xF0; 96]; // 11110000
        let b = vec![0x0F; 96]; // 00001111
        assert_eq!(scalar_popcount_xor(&a, &b), 768); // All bits differ
    }

    #[test]
    fn test_simd_matches_scalar_96_bytes() {
        let a: Vec<u8> = (0..96).map(|i| i as u8).collect();
        let b: Vec<u8> = (0..96).map(|i| (i * 2) as u8).collect();

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
    }

    #[test]
    fn test_simd_matches_scalar_16_bytes() {
        let a = vec![0xAA; 16];
        let b = vec![0x55; 16];

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
        assert_eq!(simd_result, 128); // All bits differ
    }

    #[test]
    fn test_simd_matches_scalar_128_bytes() {
        let a: Vec<u8> = (0..128).map(|i| (i * 3) as u8).collect();
        let b: Vec<u8> = (0..128).map(|i| (i * 7) as u8).collect();

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
    }

    #[test]
    fn test_simd_matches_scalar_192_bytes() {
        // 1536D vectors = 192 bytes
        let a: Vec<u8> = (0..192).map(|i| (i * 5) as u8).collect();
        let b: Vec<u8> = (0..192).map(|i| (i * 11) as u8).collect();

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
    }

    #[test]
    fn test_simd_odd_length() {
        // Test with non-power-of-2 length (7 bytes)
        let a = vec![0xFF; 7];
        let b = vec![0x00; 7];

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
        assert_eq!(simd_result, 56); // 7 * 8 = 56 bits
    }

    #[test]
    fn test_simd_empty_slices() {
        let a: Vec<u8> = vec![];
        let b: Vec<u8> = vec![];
        assert_eq!(simd_popcount_xor(&a, &b), 0);
    }

    #[test]
    fn test_simd_single_byte() {
        let a = vec![0b1010_1010];
        let b = vec![0b0101_0101];
        assert_eq!(simd_popcount_xor(&a, &b), 8);
    }

    #[test]
    fn test_simd_large_vectors() {
        // Test with 4096 bytes (very large)
        let a: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
        let b: Vec<u8> = (0..4096).map(|i| ((i + 128) % 256) as u8).collect();

        let simd_result = simd_popcount_xor(&a, &b);
        let scalar_result = scalar_popcount_xor(&a, &b);

        assert_eq!(simd_result, scalar_result);
    }
}
