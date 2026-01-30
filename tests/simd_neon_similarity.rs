//! NEON Similarity Function Property Tests
//!
//! Tests verifying correctness of NEON dot product and Euclidean distance.
//! On x86, tests verify the portable implementations.
//! On ARM64, tests verify NEON matches portable within epsilon.

use proptest::prelude::*;

/// Epsilon for floating-point comparisons.
/// FMA operations can produce slightly different results than separate mul+add.
const EPSILON: f32 = 1e-4;

/// Relative epsilon for larger values
const REL_EPSILON: f32 = 1e-5;

/// Check if two floats are approximately equal.
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
// Portable Implementation Tests (Run on all platforms)
// =============================================================================

/// Portable dot product reference
fn portable_dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Portable euclidean distance reference
fn portable_euclidean(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: Dot product is symmetric for commutative addition
    #[test]
    fn prop_dot_product_commutative(
        a in prop::collection::vec(-100.0f32..100.0f32, 0..256),
    ) {
        let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

        let ab = portable_dot_product(&a, &b);
        let ba = portable_dot_product(&b, &a);

        prop_assert!(
            approx_eq(ab, ba),
            "Dot product should be commutative: {} vs {}",
            ab, ba
        );
    }

    /// Property: Dot product with zeros is zero
    #[test]
    fn prop_dot_product_zero(
        len in 0usize..256,
    ) {
        let a = vec![0.0f32; len];
        let b: Vec<f32> = (0..len).map(|i| (i as f32) * 0.1).collect();

        let result = portable_dot_product(&a, &b);
        prop_assert!(result.abs() < EPSILON, "Dot product with zeros should be 0");
    }

    /// Property: Euclidean distance to self is zero
    #[test]
    fn prop_euclidean_self_zero(
        a in prop::collection::vec(-100.0f32..100.0f32, 0..256),
    ) {
        let result = portable_euclidean(&a, &a);
        prop_assert!(result < EPSILON, "Distance to self should be ~0, got {}", result);
    }

    /// Property: Euclidean distance is symmetric
    #[test]
    fn prop_euclidean_symmetric(
        a in prop::collection::vec(-100.0f32..100.0f32, 0..256),
    ) {
        let b: Vec<f32> = a.iter().map(|x| x + 1.0).collect();

        let ab = portable_euclidean(&a, &b);
        let ba = portable_euclidean(&b, &a);

        prop_assert!(
            approx_eq(ab, ba),
            "Euclidean distance should be symmetric: {} vs {}",
            ab, ba
        );
    }

    /// Property: Euclidean distance is non-negative
    #[test]
    fn prop_euclidean_non_negative(
        a in prop::collection::vec(-100.0f32..100.0f32, 1..256),
    ) {
        let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();
        let result = portable_euclidean(&a, &b);
        prop_assert!(result >= 0.0, "Euclidean distance should be non-negative");
    }
}

// =============================================================================
// Edge Case Tests (All platforms)
// =============================================================================

#[test]
fn test_portable_dot_product_empty() {
    assert_eq!(portable_dot_product(&[], &[]), 0.0);
}

#[test]
fn test_portable_dot_product_single() {
    let a = [3.0f32];
    let b = [4.0f32];
    assert!((portable_dot_product(&a, &b) - 12.0).abs() < EPSILON);
}

#[test]
fn test_portable_dot_product_orthogonal() {
    let a = [1.0f32, 0.0, 0.0];
    let b = [0.0f32, 1.0, 0.0];
    assert!(portable_dot_product(&a, &b).abs() < EPSILON);
}

#[test]
fn test_portable_euclidean_empty() {
    assert_eq!(portable_euclidean(&[], &[]), 0.0);
}

#[test]
fn test_portable_euclidean_single() {
    let a = [0.0f32];
    let b = [5.0f32];
    assert!((portable_euclidean(&a, &b) - 5.0).abs() < EPSILON);
}

#[test]
fn test_portable_euclidean_pythagoras() {
    // 3-4-5 triangle
    let a = [0.0f32, 0.0];
    let b = [3.0f32, 4.0];
    assert!((portable_euclidean(&a, &b) - 5.0).abs() < EPSILON);
}

#[test]
fn test_portable_euclidean_768_dims() {
    // Common embedding dimension
    let a = vec![0.5f32; 768];
    let b = a.clone();
    assert!(portable_euclidean(&a, &b) < EPSILON);
}

// =============================================================================
// NEON-specific tests (Only run on ARM64)
// =============================================================================

#[cfg(target_arch = "aarch64")]
mod neon_tests {
    use super::*;
    use edgevec::simd::neon;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property: NEON dot product matches portable within epsilon
        #[test]
        fn prop_neon_dot_product_matches_portable(
            a in prop::collection::vec(-100.0f32..100.0f32, 0..1024),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

            let neon_result = neon::dot_product(&a, &b);
            let portable_result = neon::dot_product_portable(&a, &b);

            prop_assert!(
                approx_eq(neon_result, portable_result),
                "NEON ({}) != Portable ({}) for len={}, diff={}",
                neon_result, portable_result, a.len(), (neon_result - portable_result).abs()
            );
        }

        /// Property: NEON euclidean distance matches portable within epsilon
        #[test]
        fn prop_neon_euclidean_matches_portable(
            a in prop::collection::vec(-100.0f32..100.0f32, 0..1024),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

            let neon_result = neon::euclidean_distance(&a, &b);
            let portable_result = neon::euclidean_distance_portable(&a, &b);

            prop_assert!(
                approx_eq(neon_result, portable_result),
                "NEON ({}) != Portable ({}) for len={}, diff={}",
                neon_result, portable_result, a.len(), (neon_result - portable_result).abs()
            );
        }

        /// Property: NEON dot product is commutative
        #[test]
        fn prop_neon_dot_product_commutative(
            a in prop::collection::vec(-100.0f32..100.0f32, 0..256),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

            let ab = neon::dot_product(&a, &b);
            let ba = neon::dot_product(&b, &a);

            prop_assert!(
                approx_eq(ab, ba),
                "NEON dot product should be commutative: {} vs {}",
                ab, ba
            );
        }

        /// Property: NEON euclidean distance is symmetric
        #[test]
        fn prop_neon_euclidean_symmetric(
            a in prop::collection::vec(-100.0f32..100.0f32, 0..256),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 1.0).collect();

            let ab = neon::euclidean_distance(&a, &b);
            let ba = neon::euclidean_distance(&b, &a);

            prop_assert!(
                approx_eq(ab, ba),
                "NEON euclidean distance should be symmetric: {} vs {}",
                ab, ba
            );
        }
    }

    #[test]
    fn test_neon_dot_product_empty() {
        assert_eq!(neon::dot_product(&[], &[]), 0.0);
    }

    #[test]
    fn test_neon_dot_product_single() {
        let a = [3.0f32];
        let b = [4.0f32];
        assert!((neon::dot_product(&a, &b) - 12.0).abs() < EPSILON);
    }

    #[test]
    fn test_neon_dot_product_4_exact_chunk() {
        let a = [1.0f32, 2.0, 3.0, 4.0];
        let b = [1.0f32, 1.0, 1.0, 1.0];
        assert!((neon::dot_product(&a, &b) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn test_neon_dot_product_5_with_tail() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0];
        let b = [1.0f32, 1.0, 1.0, 1.0, 1.0];
        assert!((neon::dot_product(&a, &b) - 15.0).abs() < EPSILON);
    }

    #[test]
    fn test_neon_dot_product_768_dims() {
        let a: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001).collect();
        let b: Vec<f32> = vec![1.0; 768];

        let neon_result = neon::dot_product(&a, &b);
        let portable_result = neon::dot_product_portable(&a, &b);

        assert!(
            (neon_result - portable_result).abs() < 0.01,
            "768 dims: NEON {} != Portable {}",
            neon_result,
            portable_result
        );
    }

    #[test]
    fn test_neon_euclidean_empty() {
        assert_eq!(neon::euclidean_distance(&[], &[]), 0.0);
    }

    #[test]
    fn test_neon_euclidean_identical() {
        let a = vec![1.0f32; 768];
        let b = a.clone();
        assert!(neon::euclidean_distance(&a, &b) < EPSILON);
    }

    #[test]
    fn test_neon_euclidean_pythagoras() {
        let a = [0.0f32, 0.0];
        let b = [3.0f32, 4.0];
        assert!((neon::euclidean_distance(&a, &b) - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_neon_euclidean_4_exact_chunk() {
        let a = [0.0f32, 0.0, 0.0, 0.0];
        let b = [1.0f32, 1.0, 1.0, 1.0];
        // sqrt(4) = 2
        assert!((neon::euclidean_distance(&a, &b) - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_neon_euclidean_5_with_tail() {
        let a = [0.0f32, 0.0, 0.0, 0.0, 0.0];
        let b = [1.0f32, 1.0, 1.0, 1.0, 1.0];
        // sqrt(5) ≈ 2.236
        assert!((neon::euclidean_distance(&a, &b) - 5.0f32.sqrt()).abs() < EPSILON);
    }

    #[test]
    fn test_neon_euclidean_768_dims() {
        let a: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001).collect();
        let b: Vec<f32> = (0..768).map(|i| ((768 - i) as f32) * 0.001).collect();

        let neon_result = neon::euclidean_distance(&a, &b);
        let portable_result = neon::euclidean_distance_portable(&a, &b);

        assert!(
            (neon_result - portable_result).abs() < 0.01,
            "768 dims: NEON {} != Portable {}",
            neon_result,
            portable_result
        );
    }

    #[test]
    fn test_neon_matches_portable_various_sizes() {
        // Test sizes that exercise NEON vector boundaries
        for size in [
            0, 1, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 100, 768, 1024,
        ] {
            let a: Vec<f32> = (0..size).map(|i| (i as f32) * 0.1).collect();
            let b: Vec<f32> = (0..size).map(|i| ((size - i) as f32) * 0.1).collect();

            // Dot product - use relative tolerance for large magnitudes
            let neon_dot = neon::dot_product(&a, &b);
            let portable_dot = neon::dot_product_portable(&a, &b);
            let max_dot = neon_dot.abs().max(portable_dot.abs()).max(1.0);
            let rel_tol_dot = (neon_dot - portable_dot).abs() / max_dot;
            assert!(
                rel_tol_dot < 1e-5,
                "dot_product size={}: NEON {} != Portable {} (rel_err: {:.2e})",
                size,
                neon_dot,
                portable_dot,
                rel_tol_dot
            );

            // Euclidean - use relative tolerance for large magnitudes
            let neon_euc = neon::euclidean_distance(&a, &b);
            let portable_euc = neon::euclidean_distance_portable(&a, &b);
            let max_euc = neon_euc.abs().max(portable_euc.abs()).max(1.0);
            let rel_tol_euc = (neon_euc - portable_euc).abs() / max_euc;
            assert!(
                rel_tol_euc < 1e-5,
                "euclidean size={}: NEON {} != Portable {} (rel_err: {:.2e})",
                size,
                neon_euc,
                portable_euc,
                rel_tol_euc
            );
        }
    }
}
