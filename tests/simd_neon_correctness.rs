//! Comprehensive NEON SIMD Correctness Tests
//!
//! This file validates that all NEON implementations produce
//! identical (or epsilon-close) results to portable implementations.
//!
//! # Test Coverage
//!
//! - Hamming distance: 6 tests
//! - Dot product: 6 tests
//! - Euclidean distance: 5 tests
//! - Sanity check: 1 test
//!
//! Total: 18 tests

/// Epsilon for floating-point comparisons.
/// FMA operations can produce slightly different results than separate mul+add.
#[cfg(target_arch = "aarch64")]
const EPSILON: f32 = 1e-4;

/// Relative epsilon for larger values
#[cfg(target_arch = "aarch64")]
const REL_EPSILON: f32 = 1e-4;

/// Check if two floats are approximately equal.
#[cfg(target_arch = "aarch64")]
fn approx_eq(a: f32, b: f32) -> bool {
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let abs_diff = (a - b).abs();
    let max_val = a.abs().max(b.abs());
    // Absolute epsilon for small values, relative for large
    abs_diff < EPSILON || abs_diff < max_val * REL_EPSILON
}

// =============================================================================
// NEON Correctness Tests (Only run on ARM64)
// =============================================================================

#[cfg(target_arch = "aarch64")]
mod neon_correctness {
    use super::*;
    use edgevec::quantization::simd::portable::hamming_distance_slice as portable_hamming;
    use edgevec::simd::neon;

    // ============ HAMMING DISTANCE ============

    #[test]
    fn hamming_empty() {
        assert_eq!(neon::hamming_distance_slice(&[], &[]), 0);
    }

    #[test]
    fn hamming_single_byte_all_different() {
        assert_eq!(neon::hamming_distance_slice(&[0xFF], &[0x00]), 8);
    }

    #[test]
    fn hamming_single_byte_identical() {
        assert_eq!(neon::hamming_distance_slice(&[0xAB], &[0xAB]), 0);
    }

    #[test]
    fn hamming_16_bytes_all_different() {
        let a = vec![0xFFu8; 16];
        let b = vec![0x00u8; 16];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 128);
    }

    #[test]
    fn hamming_17_bytes_with_tail() {
        let a = vec![0xFFu8; 17];
        let b = vec![0x00u8; 17];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 136);
    }

    #[test]
    fn hamming_large_random() {
        let a: Vec<u8> = (0..1000).map(|i| i as u8).collect();
        let b: Vec<u8> = (0..1000).map(|i| (i + 1) as u8).collect();
        let neon_result = neon::hamming_distance_slice(&a, &b);
        let portable_result = portable_hamming(&a, &b);
        assert_eq!(neon_result, portable_result);
    }

    // ============ DOT PRODUCT ============

    #[test]
    fn dot_product_empty() {
        assert!((neon::dot_product(&[], &[]) - 0.0).abs() < EPSILON);
    }

    #[test]
    fn dot_product_single_element() {
        assert!(approx_eq(neon::dot_product(&[2.0], &[3.0]), 6.0));
    }

    #[test]
    fn dot_product_4_elements() {
        let a = vec![1.0f32, 2.0, 3.0, 4.0];
        let b = vec![1.0f32, 1.0, 1.0, 1.0];
        assert!(approx_eq(neon::dot_product(&a, &b), 10.0));
    }

    #[test]
    fn dot_product_5_elements_with_tail() {
        let a = vec![1.0f32; 5];
        let b = vec![2.0f32; 5];
        assert!(approx_eq(neon::dot_product(&a, &b), 10.0));
    }

    #[test]
    fn dot_product_768_dimensions() {
        let a: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001).collect();
        let b: Vec<f32> = (0..768).map(|i| ((768 - i) as f32) * 0.001).collect();
        let neon_result = neon::dot_product(&a, &b);
        let portable_result = neon::dot_product_portable(&a, &b);
        assert!(
            approx_eq(neon_result, portable_result),
            "NEON {} != Portable {}",
            neon_result,
            portable_result
        );
    }

    #[test]
    fn dot_product_1024_dimensions() {
        let a: Vec<f32> = (0..1024).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = vec![1.0; 1024];
        let neon_result = neon::dot_product(&a, &b);
        let portable_result = neon::dot_product_portable(&a, &b);
        // Use relative tolerance for large results
        let tolerance = neon_result.abs().max(portable_result.abs()) * REL_EPSILON;
        assert!(
            (neon_result - portable_result).abs() < tolerance,
            "NEON {} != Portable {}, diff {}",
            neon_result,
            portable_result,
            (neon_result - portable_result).abs()
        );
    }

    // ============ EUCLIDEAN DISTANCE ============

    #[test]
    fn euclidean_empty() {
        assert!((neon::euclidean_distance(&[], &[]) - 0.0).abs() < EPSILON);
    }

    #[test]
    fn euclidean_identical_vectors() {
        let a = vec![1.0f32; 100];
        let b = a.clone();
        assert!(neon::euclidean_distance(&a, &b) < EPSILON);
    }

    #[test]
    fn euclidean_known_value() {
        // Distance between (0,0) and (3,4) should be 5
        let a = vec![0.0f32, 0.0];
        let b = vec![3.0f32, 4.0];
        assert!(approx_eq(neon::euclidean_distance(&a, &b), 5.0));
    }

    #[test]
    fn euclidean_4_elements() {
        // sqrt(4 * 1^2) = 2
        let a = vec![0.0f32; 4];
        let b = vec![1.0f32; 4];
        assert!(approx_eq(neon::euclidean_distance(&a, &b), 2.0));
    }

    #[test]
    fn euclidean_768_dimensions() {
        let a: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let b: Vec<f32> = (0..768).map(|_| -0.5).collect();
        let neon_result = neon::euclidean_distance(&a, &b);
        let portable_result = neon::euclidean_distance_portable(&a, &b);
        assert!(
            approx_eq(neon_result, portable_result),
            "NEON {} != Portable {}",
            neon_result,
            portable_result
        );
    }
}

// =============================================================================
// Sanity Check (Runs on all platforms)
// =============================================================================

#[test]
fn sanity_check_test_runs() {
    // This test always passes, confirms test file is included
    // Sanity check that the test infrastructure works
}

/// Document total test count
#[test]
fn test_count_verification() {
    // This test documents expected test count:
    // - 6 hamming tests (ARM64 only)
    // - 6 dot product tests (ARM64 only)
    // - 5 euclidean tests (ARM64 only)
    // - 2 sanity tests (all platforms)
    // Total: 19 tests (17 on ARM64, 2 on x86)
    #[cfg(target_arch = "aarch64")]
    {
        // On ARM64, all NEON tests run
        println!("ARM64: Running all NEON correctness tests");
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        // On other platforms, only sanity tests run
        println!("Non-ARM64: Running sanity tests only");
    }
}
