//! SIMD-accelerated Hamming distance for binary quantization.
//!
//! Provides platform-specific SIMD implementations with runtime feature detection.
//!
//! # Performance Targets
//!
//! - AVX2 (x86_64): <50 CPU cycles per comparison
//! - Portable fallback: ~300 cycles (baseline)
//!
//! # Safety
//!
//! All unsafe SIMD operations are encapsulated behind safe public APIs.
//! CPU feature detection ensures SIMD instructions are only used when supported.

// Platform-specific implementations
#[cfg(target_arch = "x86_64")]
pub(crate) mod avx2;

/// Portable (non-SIMD) implementation for all platforms.
///
/// This module provides the baseline implementation used when SIMD is not
/// available or for benchmarking purposes.
pub mod portable;

/// Computes Hamming distance using the best available SIMD implementation.
///
/// This function automatically detects CPU capabilities at runtime and
/// dispatches to the fastest available implementation:
///
/// - **AVX2** (x86_64 with AVX2): ~47 cycles per comparison
/// - **Portable**: ~300 cycles (safe fallback)
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
/// # Performance
///
/// Target: <50 CPU cycles on AVX2-capable hardware.
///
/// # Safety
///
/// This function is completely safe. All unsafe operations are internal
/// and guarded by runtime CPU feature detection.
///
/// # Example
///
/// ```
/// use edgevec::quantization::simd;
///
/// let a = [0xAA; 96]; // 10101010...
/// let b = [0x55; 96]; // 01010101...
///
/// // All 768 bits differ
/// let distance = simd::hamming_distance(&a, &b);
/// assert_eq!(distance, 768);
/// ```
#[inline]
#[must_use]
pub fn hamming_distance(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: We just verified AVX2 is available via runtime detection
            return unsafe { avx2::hamming_distance_avx2(a, b) };
        }
    }

    // Safe fallback for all other platforms
    portable::hamming_distance_portable(a, b)
}

/// Forces use of the portable (non-SIMD) implementation.
///
/// This function is exposed for testing and benchmarking purposes
/// to compare SIMD vs non-SIMD performance.
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
/// # Use Cases
///
/// - Benchmarking: Compare SIMD vs portable performance
/// - Testing: Verify SIMD correctness against portable baseline
/// - Platforms: Use when SIMD is unavailable or disabled
#[inline]
#[must_use]
pub fn hamming_distance_portable(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    portable::hamming_distance_portable(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_dispatch_identical() {
        let a = [0xAA; 96];
        let b = [0xAA; 96];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn test_simd_dispatch_opposite() {
        let a = [0x00; 96];
        let b = [0xFF; 96];
        assert_eq!(hamming_distance(&a, &b), 768);
    }

    #[test]
    fn test_simd_matches_portable() {
        let a = [0xAA; 96];
        let b = [0x55; 96];

        let simd_result = hamming_distance(&a, &b);
        let portable_result = hamming_distance_portable(&a, &b);

        assert_eq!(simd_result, portable_result);
        assert_eq!(simd_result, 768); // All bits differ
    }
}
