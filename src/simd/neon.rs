//! ARM NEON SIMD implementations.
//!
//! This module provides NEON-optimized versions of SIMD operations for ARM64.
//! Uses NEON intrinsics for high-performance vectorized computation.
//!
//! # Platform Support
//!
//! This module is only compiled on `aarch64` targets. On other platforms,
//! the module is not available.
//!
//! # Safety
//!
//! All NEON intrinsics are encapsulated behind safe public APIs.
//! The unsafe code within this module:
//! - Uses NEON intrinsics for vectorized memory reads (read-only, bounds-checked)
//! - All memory accesses are verified to be within bounds before execution
//! - Functions verify NEON availability via `#[target_feature(enable = "neon")]`
//!
//! # Example
//!
//! ```ignore
//! // This module is only available on aarch64
//! #[cfg(target_arch = "aarch64")]
//! use edgevec::simd::neon;
//!
//! #[cfg(target_arch = "aarch64")]
//! {
//!     let a = vec![0xAA_u8; 100];
//!     let b = vec![0x55_u8; 100];
//!     let distance = neon::hamming_distance_slice(&a, &b);
//! }
//! ```

use crate::quantization::simd::portable::hamming_distance_portable;
use crate::quantization::simd::portable::hamming_distance_slice as hamming_distance_portable_generic;
use std::arch::aarch64::*;

/// NEON-optimized Hamming distance for arbitrary-length byte slices.
///
/// Computes the number of differing bits between two byte slices using
/// NEON SIMD instructions for maximum performance on ARM64.
///
/// # Algorithm
///
/// 1. Process 16 bytes at a time using NEON 128-bit vectors
/// 2. Use `veorq_u8` to XOR vectors (find differing bits)
/// 3. Use `vcntq_u8` to count set bits in each byte
/// 4. Use `vaddlvq_u8` to horizontally sum all byte counts
/// 5. Handle remaining tail bytes with scalar operations
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
/// # Safety
///
/// This function uses unsafe NEON intrinsics internally. Safety is guaranteed by:
/// - Slice length equality is verified before processing
/// - All pointer arithmetic stays within slice bounds (verified by chunk calculation)
/// - NEON feature availability is verified by `#[target_feature(enable = "neon")]`
///
/// # Performance
///
/// - Processes 16 bytes per iteration (vs 1 byte for portable)
/// - Expected speedup: ~8-16x for large inputs
/// - Falls back to scalar for tail elements (0-15 bytes)
///
/// # Example
///
/// ```ignore
/// #[cfg(target_arch = "aarch64")]
/// {
///     use edgevec::simd::neon;
///     let a = vec![0xAA; 100];
///     let b = vec![0x55; 100];
///     let distance = neon::hamming_distance_slice(&a, &b);
///     assert_eq!(distance, 800); // 100 * 8 bits all differ
/// }
/// ```
#[inline]
#[must_use]
pub fn hamming_distance_slice(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len(), "Slice lengths must match");

    // SAFETY: We've verified equal lengths. The unsafe function handles
    // all bounds checking internally and NEON is available on aarch64.
    unsafe { hamming_distance_neon_unchecked(a, b) }
}

/// NEON-optimized Hamming distance (unchecked).
///
/// # Safety
///
/// - Caller must ensure `a.len() == b.len()`
/// - NEON must be available (guaranteed by `#[target_feature(enable = "neon")]`)
///
/// # Implementation Notes
///
/// All memory accesses are bounds-checked by the chunk calculation:
/// - `chunks = len / 16` ensures we only read complete 16-byte blocks
/// - `offset = i * 16` where `i < chunks` ensures `offset + 16 <= len`
/// - Tail processing uses safe Rust indexing with bounds checking
#[inline]
#[target_feature(enable = "neon")]
unsafe fn hamming_distance_neon_unchecked(a: &[u8], b: &[u8]) -> u32 {
    debug_assert_eq!(a.len(), b.len(), "Slices must have equal length");

    let len = a.len();
    let chunks = len / 16;
    let mut count: u64 = 0;

    // Process 16 bytes at a time using NEON
    for i in 0..chunks {
        let offset = i * 16;

        // SAFETY: offset + 16 <= len is guaranteed by chunks = len / 16
        // We're reading 16 bytes starting at offset, which is within bounds.
        let va = vld1q_u8(a.as_ptr().add(offset));
        let vb = vld1q_u8(b.as_ptr().add(offset));

        // XOR to find differing bits
        let xor = veorq_u8(va, vb);

        // Count bits in each byte (vcntq_u8 returns popcount per byte)
        let bit_counts = vcntq_u8(xor);

        // Sum all 16 byte counts into a single value
        // vaddlvq_u8 performs unsigned horizontal add across vector
        count += u64::from(vaddlvq_u8(bit_counts));
    }

    // Handle remaining bytes (0-15 bytes) with scalar operations
    let tail_start = chunks * 16;
    for i in tail_start..len {
        // SAFETY: i < len is guaranteed by the loop bounds
        count += u64::from((a[i] ^ b[i]).count_ones());
    }

    // Result fits in u32: max is len * 8, and len is bounded by usize
    count as u32
}

/// NEON-optimized Hamming distance for fixed 96-byte vectors.
///
/// Computes the number of differing bits between two 96-byte binary vectors.
/// This is a specialized version for the common case of 768-bit binary embeddings.
///
/// # Arguments
///
/// * `a` - First 96-byte binary vector (768 bits)
/// * `b` - Second 96-byte binary vector (768 bits)
///
/// # Returns
///
/// The number of differing bits (0..=768)
///
/// # Example
///
/// ```ignore
/// #[cfg(target_arch = "aarch64")]
/// {
///     use edgevec::simd::neon;
///     let a = [0xAA; 96];
///     let b = [0x55; 96];
///     let distance = neon::hamming_distance(&a, &b);
///     assert_eq!(distance, 768); // All bits differ
/// }
/// ```
#[inline]
#[must_use]
pub fn hamming_distance(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    // Use the slice version which handles all the NEON optimization
    hamming_distance_slice(a.as_slice(), b.as_slice())
}

/// Portable fallback Hamming distance for arbitrary slices.
///
/// This delegates to the portable implementation for use in comparisons
/// and testing against the NEON version.
#[inline]
#[must_use]
pub fn hamming_distance_portable_ref(a: &[u8], b: &[u8]) -> u32 {
    hamming_distance_portable_generic(a, b)
}

/// NEON-optimized dot product for f32 slices.
///
/// Computes the dot product of two f32 slices.
///
/// # Current Implementation
///
/// **STUB** - Currently delegates to scalar implementation.
/// Will be replaced with NEON intrinsics in W20.4.
///
/// # Arguments
///
/// * `a` - First f32 slice
/// * `b` - Second f32 slice (must be same length as `a`)
///
/// # Returns
///
/// The dot product: sum(a[i] * b[i])
///
/// # Panics
///
/// Panics if slices have different lengths (in debug builds).
#[inline]
#[must_use]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    // TODO(W20.4): Replace with NEON intrinsics
    // Implementation will use:
    // - vld1q_f32: Load 4 floats
    // - vmulq_f32: Multiply 4 floats
    // - vaddvq_f32: Horizontal sum
    debug_assert_eq!(a.len(), b.len(), "Slice lengths must match");
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// NEON-optimized Euclidean distance for f32 slices.
///
/// Computes the Euclidean distance (L2 norm) between two f32 slices.
///
/// # Current Implementation
///
/// **STUB** - Currently delegates to scalar implementation.
/// Will be replaced with NEON intrinsics in W20.4.
///
/// # Arguments
///
/// * `a` - First f32 slice
/// * `b` - Second f32 slice (must be same length as `a`)
///
/// # Returns
///
/// The Euclidean distance: sqrt(sum((a[i] - b[i])^2))
///
/// # Panics
///
/// Panics if slices have different lengths (in debug builds).
#[inline]
#[must_use]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    // TODO(W20.4): Replace with NEON intrinsics
    // Implementation will use:
    // - vld1q_f32: Load 4 floats
    // - vsubq_f32: Subtract 4 floats
    // - vmulq_f32: Square differences
    // - vaddvq_f32: Horizontal sum
    // - vsqrtf: Square root
    debug_assert_eq!(a.len(), b.len(), "Slice lengths must match");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixed-size 96-byte tests (backwards compatibility)

    #[test]
    fn test_hamming_identical() {
        let a = [0xAA; 96];
        let b = [0xAA; 96];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn test_hamming_opposite() {
        let a = [0x00; 96];
        let b = [0xFF; 96];
        assert_eq!(hamming_distance(&a, &b), 768);
    }

    #[test]
    fn test_hamming_alternating() {
        let a = [0xAA; 96]; // 10101010...
        let b = [0x55; 96]; // 01010101...
        assert_eq!(hamming_distance(&a, &b), 768);
    }

    #[test]
    fn test_hamming_single_bit() {
        let mut a = [0x00; 96];
        let b = [0x00; 96];
        a[0] = 0x01;
        assert_eq!(hamming_distance(&a, &b), 1);
    }

    // Slice-based NEON hamming tests

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
    fn test_slice_15_bytes_tail_only() {
        // 15 bytes = 0 NEON chunks + 15 tail bytes
        let a = vec![0xFF; 15];
        let b = vec![0x00; 15];
        assert_eq!(hamming_distance_slice(&a, &b), 120); // 15 * 8
    }

    #[test]
    fn test_slice_16_bytes_exact_chunk() {
        // 16 bytes = 1 NEON chunk + 0 tail bytes
        let a = vec![0xFF; 16];
        let b = vec![0x00; 16];
        assert_eq!(hamming_distance_slice(&a, &b), 128); // 16 * 8
    }

    #[test]
    fn test_slice_17_bytes_with_tail() {
        // 17 bytes = 1 NEON chunk + 1 tail byte
        let a = vec![0xFF; 17];
        let b = vec![0x00; 17];
        assert_eq!(hamming_distance_slice(&a, &b), 136); // 17 * 8
    }

    #[test]
    fn test_slice_32_bytes_two_chunks() {
        // 32 bytes = 2 NEON chunks + 0 tail bytes
        let a = vec![0xFF; 32];
        let b = vec![0x00; 32];
        assert_eq!(hamming_distance_slice(&a, &b), 256); // 32 * 8
    }

    #[test]
    fn test_slice_100_bytes() {
        // 100 bytes = 6 NEON chunks + 4 tail bytes
        let a = vec![0xAA; 100];
        let b = vec![0x55; 100];
        assert_eq!(hamming_distance_slice(&a, &b), 800); // 100 * 8
    }

    #[test]
    fn test_slice_identical() {
        let a = vec![42u8; 1000];
        let b = a.clone();
        assert_eq!(hamming_distance_slice(&a, &b), 0);
    }

    #[test]
    fn test_slice_matches_portable() {
        // Verify NEON matches portable for various sizes
        for size in [0, 1, 15, 16, 17, 31, 32, 33, 64, 96, 100, 128, 1000] {
            let a: Vec<u8> = (0..size).map(|i| i as u8).collect();
            let b: Vec<u8> = (0..size).map(|i| (i + 1) as u8).collect();

            let neon_result = hamming_distance_slice(&a, &b);
            let portable_result = hamming_distance_portable_ref(&a, &b);

            assert_eq!(
                neon_result, portable_result,
                "NEON != Portable for size={}: {} != {}",
                size, neon_result, portable_result
            );
        }
    }

    #[test]
    fn test_slice_matches_fixed_96() {
        // Verify slice and fixed-size functions produce same result
        let a = [0xAA; 96];
        let b = [0x55; 96];
        assert_eq!(hamming_distance(&a, &b), hamming_distance_slice(&a, &b));
    }

    // Dot product tests

    #[test]
    fn test_dot_product_basic() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 1.0, 1.0, 1.0];
        let result = dot_product(&a, &b);
        assert!((result - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_zero() {
        let a = [1.0, 0.0, 1.0, 0.0];
        let b = [0.0, 1.0, 0.0, 1.0];
        let result = dot_product(&a, &b);
        assert!((result - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_empty() {
        let a: [f32; 0] = [];
        let b: [f32; 0] = [];
        let result = dot_product(&a, &b);
        assert!((result - 0.0).abs() < 1e-6);
    }

    // Euclidean distance tests

    #[test]
    fn test_euclidean_identical() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0, 3.0, 4.0];
        let result = euclidean_distance(&a, &b);
        assert!((result - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_unit() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let result = euclidean_distance(&a, &b);
        assert!((result - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_pythagoras() {
        // 3-4-5 triangle
        let a = [0.0, 0.0];
        let b = [3.0, 4.0];
        let result = euclidean_distance(&a, &b);
        assert!((result - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_empty() {
        let a: [f32; 0] = [];
        let b: [f32; 0] = [];
        let result = euclidean_distance(&a, &b);
        assert!((result - 0.0).abs() < 1e-6);
    }
}
