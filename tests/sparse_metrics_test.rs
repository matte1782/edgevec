//! Property tests for sparse vector metrics.
//!
//! These tests verify mathematical properties of dot product,
//! cosine similarity, and normalization.

use edgevec::sparse::{sparse_cosine, sparse_dot_product, sparse_norm, SparseVector};
use proptest::prelude::*;

/// Maximum dimension for generated vectors.
const MAX_DIM: u32 = 10_000;

/// Maximum non-zero elements for generated vectors.
const MAX_NNZ: usize = 500;

/// Maximum absolute value for generated floats.
/// Chosen so that nnz * value^2 won't overflow f32 even with MAX_NNZ elements.
/// sqrt(f32::MAX / MAX_NNZ) ≈ 2.6e16, using 1e6 for extra safety margin.
const MAX_ABS_VALUE: f32 = 1e6;

/// Minimum absolute value for generated floats.
/// Prevents underflow when squaring (value^2 must be representable).
/// sqrt(f32::MIN_POSITIVE) ≈ 1.08e-19, using 1e-15 for safety.
const MIN_ABS_VALUE: f32 = 1e-15;

// =============================================================================
// ARBITRARY GENERATORS
// =============================================================================

/// Strategy to generate bounded f32 values that won't over/underflow in metric calculations.
fn arb_bounded_f32() -> impl Strategy<Value = f32> {
    // Generate values in range [-MAX_ABS_VALUE, -MIN_ABS_VALUE] or [MIN_ABS_VALUE, MAX_ABS_VALUE]
    prop::bool::ANY.prop_flat_map(|positive| {
        // Use uniform distribution within the safe range
        (MIN_ABS_VALUE..=MAX_ABS_VALUE).prop_map(
            move |abs_val| {
                if positive {
                    abs_val
                } else {
                    -abs_val
                }
            },
        )
    })
}

/// Check if two f32 values are approximately equal using relative tolerance.
fn approx_eq(a: f32, b: f32, rel_tol: f32) -> bool {
    if a == b {
        return true;
    }
    let diff = (a - b).abs();
    let max_abs = a.abs().max(b.abs());
    if max_abs == 0.0 {
        return diff < rel_tol;
    }
    diff / max_abs < rel_tol
}

/// Strategy to generate valid SparseVector instances.
///
/// Generates:
/// - Dimension in [10, MAX_DIM]
/// - NNZ in [1, min(MAX_NNZ, dim)]
/// - Sorted unique indices < dim
/// - Bounded f32 values that won't overflow in metric calculations
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    // Minimum dimension of 10 to avoid constraint issues
    (10u32..=MAX_DIM)
        .prop_flat_map(|dim| {
            let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

            (1..=max_nnz).prop_flat_map(move |nnz| {
                let all_indices: Vec<u32> = (0..dim).collect();
                let indices_strategy =
                    prop::sample::subsequence(all_indices, nnz).prop_map(|mut v| {
                        v.sort();
                        v
                    });

                let values_strategy = proptest::collection::vec(arb_bounded_f32(), nnz);

                (Just(dim), indices_strategy, values_strategy)
            })
        })
        .prop_map(|(dim, indices, values)| {
            SparseVector::new(indices, values, dim).expect("Generator should produce valid vectors")
        })
}

/// Strategy to generate pairs of SparseVectors with same dimension.
fn arb_sparse_vector_pair() -> impl Strategy<Value = (SparseVector, SparseVector)> {
    (10u32..=MAX_DIM)
        .prop_flat_map(|dim| {
            let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);
            let all_indices: Vec<u32> = (0..dim).collect();

            let v1 = (1..=max_nnz).prop_flat_map({
                let all_indices = all_indices.clone();
                move |nnz| {
                    let indices =
                        prop::sample::subsequence(all_indices.clone(), nnz).prop_map(|mut v| {
                            v.sort();
                            v
                        });
                    let values = proptest::collection::vec(arb_bounded_f32(), nnz);
                    (Just(dim), indices, values)
                }
            });

            let v2 = (1..=max_nnz).prop_flat_map({
                let all_indices = all_indices.clone();
                move |nnz| {
                    let indices =
                        prop::sample::subsequence(all_indices.clone(), nnz).prop_map(|mut v| {
                            v.sort();
                            v
                        });
                    let values = proptest::collection::vec(arb_bounded_f32(), nnz);
                    (Just(dim), indices, values)
                }
            });

            (v1, v2)
        })
        .prop_map(|((dim, i1, v1), (_, i2, v2))| {
            (
                SparseVector::new(i1, v1, dim).expect("valid"),
                SparseVector::new(i2, v2, dim).expect("valid"),
            )
        })
}

/// Strategy to generate non-zero sparse vectors (guaranteed non-zero norm).
///
/// Uses arb_bounded_f32() which already guarantees non-zero values,
/// so vectors will always have a positive norm and can be normalized.
fn arb_nonzero_sparse_vector() -> impl Strategy<Value = SparseVector> {
    // arb_bounded_f32() already produces non-zero values, so this is equivalent
    // to arb_sparse_vector() but explicitly documented as non-zero
    arb_sparse_vector()
}

/// Strategy to generate pairs of small-dimension SparseVectors for dense cross-check.
/// Uses dim in [10, 100] to avoid excessive memory allocation.
fn arb_small_sparse_vector_pair() -> impl Strategy<Value = (SparseVector, SparseVector)> {
    (10u32..=100u32)
        .prop_flat_map(|dim| {
            let max_nnz = std::cmp::min(50usize, dim as usize);
            let all_indices: Vec<u32> = (0..dim).collect();

            let v1 = (1..=max_nnz).prop_flat_map({
                let all_indices = all_indices.clone();
                move |nnz| {
                    let indices =
                        prop::sample::subsequence(all_indices.clone(), nnz).prop_map(|mut v| {
                            v.sort();
                            v
                        });
                    let values = proptest::collection::vec(arb_bounded_f32(), nnz);
                    (Just(dim), indices, values)
                }
            });

            let v2 = (1..=max_nnz).prop_flat_map({
                let all_indices = all_indices.clone();
                move |nnz| {
                    let indices =
                        prop::sample::subsequence(all_indices.clone(), nnz).prop_map(|mut v| {
                            v.sort();
                            v
                        });
                    let values = proptest::collection::vec(arb_bounded_f32(), nnz);
                    (Just(dim), indices, values)
                }
            });

            (v1, v2)
        })
        .prop_map(|((dim, i1, v1), (_, i2, v2))| {
            (
                SparseVector::new(i1, v1, dim).expect("valid"),
                SparseVector::new(i2, v2, dim).expect("valid"),
            )
        })
}

// =============================================================================
// UNIT TESTS
// =============================================================================

mod unit_tests {
    use super::*;

    // === Dot Product Tests ===

    #[test]
    fn test_dot_product_zero() {
        // Orthogonal vectors (no common indices)
        let a = SparseVector::new(vec![0, 1], vec![1.0, 1.0], 100).unwrap();
        let b = SparseVector::new(vec![2, 3], vec![1.0, 1.0], 100).unwrap();
        assert_eq!(sparse_dot_product(&a, &b), 0.0);
    }

    #[test]
    fn test_dot_product_self() {
        // v · v = ||v||²
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let dot = sparse_dot_product(&v, &v);
        let norm_sq = 1.0 + 4.0 + 9.0; // 14.0
        assert!((dot - norm_sq).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_partial_overlap() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10, 20], vec![0.5, 0.5, 1.0], 100).unwrap();
        // Common: 5 (2.0 * 0.5 = 1.0) and 10 (3.0 * 0.5 = 1.5)
        let expected = 1.0 + 1.5;
        assert!((sparse_dot_product(&a, &b) - expected).abs() < 1e-6);
    }

    // === Cosine Tests ===

    #[test]
    fn test_cosine_self_is_one() {
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let cos = sparse_cosine(&v, &v);
        assert!((cos - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal_is_zero() {
        let a = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let b = SparseVector::new(vec![1], vec![1.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_parallel() {
        let a = SparseVector::new(vec![0, 1], vec![1.0, 0.0], 100).unwrap();
        let b = SparseVector::new(vec![0, 1], vec![5.0, 0.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_antiparallel() {
        let a = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let b = SparseVector::new(vec![0], vec![-1.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - (-1.0)).abs() < 1e-6);
    }

    // === Normalize Tests ===

    #[test]
    fn test_normalize_has_unit_norm() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        let normalized = v.normalize().unwrap();
        assert!((normalized.norm() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_preserves_direction() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        let normalized = v.normalize().unwrap();
        // Cosine between v and normalized(v) should be 1
        let cos = sparse_cosine(&v, &normalized);
        assert!((cos - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_values() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        let normalized = v.normalize().unwrap();
        // ||v|| = 5, so values should be 3/5=0.6 and 4/5=0.8
        assert!((normalized.values()[0] - 0.6).abs() < 1e-6);
        assert!((normalized.values()[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_preserves_indices() {
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let normalized = v.normalize().unwrap();
        assert_eq!(v.indices(), normalized.indices());
        assert_eq!(v.dim(), normalized.dim());
    }

    // === Norm Tests ===

    #[test]
    fn test_norm_345() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        assert!((sparse_norm(&v) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_norm_unit() {
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        assert!((sparse_norm(&v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_norm_sqrt_dot_self() {
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let norm = sparse_norm(&v);
        let dot_self = sparse_dot_product(&v, &v);
        assert!((norm - dot_self.sqrt()).abs() < 1e-6);
    }

    // === Cross-check with dense ===

    #[test]
    fn test_dot_product_matches_dense() {
        // Sparse: indices [0, 2, 4], values [1.0, 2.0, 3.0], dim=5
        // Dense: [1.0, 0.0, 2.0, 0.0, 3.0]
        let a = SparseVector::new(vec![0, 2, 4], vec![1.0, 2.0, 3.0], 5).unwrap();
        let b = SparseVector::new(vec![0, 1, 2, 3, 4], vec![0.5, 0.1, 0.5, 0.1, 0.5], 5).unwrap();

        let sparse_dot = sparse_dot_product(&a, &b);

        // Dense computation: 1.0*0.5 + 0.0*0.1 + 2.0*0.5 + 0.0*0.1 + 3.0*0.5
        // = 0.5 + 0 + 1.0 + 0 + 1.5 = 3.0
        let dense_dot = 1.0 * 0.5 + 2.0 * 0.5 + 3.0 * 0.5;

        assert!((sparse_dot - dense_dot).abs() < 1e-6);
    }
}

// =============================================================================
// PROPERTY TESTS
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // === Dot Product Properties ===

    /// Property: dot(a, b) == dot(b, a) (commutativity)
    #[test]
    fn prop_dot_commutative((a, b) in arb_sparse_vector_pair()) {
        let dot_ab = sparse_dot_product(&a, &b);
        let dot_ba = sparse_dot_product(&b, &a);
        prop_assert!((dot_ab - dot_ba).abs() < 1e-5,
            "Dot product not commutative: {} vs {}", dot_ab, dot_ba);
    }

    /// Property: dot(a, a) >= 0 (positive semi-definite)
    #[test]
    fn prop_dot_positive_semidefinite(v in arb_sparse_vector()) {
        let dot = sparse_dot_product(&v, &v);
        prop_assert!(dot >= 0.0, "Dot product with self is negative: {}", dot);
    }

    /// Property: dot(v, v) == ||v||² (norm relationship)
    #[test]
    fn prop_dot_self_equals_norm_squared(v in arb_sparse_vector()) {
        let dot = sparse_dot_product(&v, &v);
        let norm_sq = sparse_norm(&v).powi(2);
        // Use relative tolerance for large values
        prop_assert!(approx_eq(dot, norm_sq, 1e-5),
            "dot(v,v)={} != ||v||²={}", dot, norm_sq);
    }

    // === Cosine Properties ===

    /// Property: cos(a, a) == 1.0 for non-zero vectors
    #[test]
    fn prop_cosine_self_is_one(v in arb_nonzero_sparse_vector()) {
        let cos = sparse_cosine(&v, &v);
        prop_assert!((cos - 1.0).abs() < 1e-5,
            "Cosine with self is not 1.0: {}", cos);
    }

    /// Property: -1.0 <= cos(a, b) <= 1.0
    #[test]
    fn prop_cosine_in_range((a, b) in arb_sparse_vector_pair()) {
        let cos = sparse_cosine(&a, &b);
        prop_assert!((-1.0 - 1e-6..=1.0 + 1e-6).contains(&cos),
            "Cosine out of range: {}", cos);
    }

    /// Property: cos(a, b) == cos(b, a) (commutativity)
    #[test]
    fn prop_cosine_commutative((a, b) in arb_sparse_vector_pair()) {
        let cos_ab = sparse_cosine(&a, &b);
        let cos_ba = sparse_cosine(&b, &a);
        prop_assert!((cos_ab - cos_ba).abs() < 1e-5,
            "Cosine not commutative: {} vs {}", cos_ab, cos_ba);
    }

    // === Norm Properties ===

    /// Property: ||v|| >= 0 (non-negative)
    #[test]
    fn prop_norm_nonnegative(v in arb_sparse_vector()) {
        let norm = sparse_norm(&v);
        prop_assert!(norm >= 0.0, "Norm is negative: {}", norm);
    }

    /// Property: ||normalize(v)|| ≈ 1.0
    #[test]
    fn prop_normalize_has_unit_norm(v in arb_nonzero_sparse_vector()) {
        if let Ok(normalized) = v.normalize() {
            let norm = sparse_norm(&normalized);
            prop_assert!((norm - 1.0).abs() < 1e-5,
                "Normalized vector has norm {}, expected 1.0", norm);
        }
    }

    /// Property: cos(v, normalize(v)) == 1.0 (same direction)
    #[test]
    fn prop_normalize_preserves_direction(v in arb_nonzero_sparse_vector()) {
        if let Ok(normalized) = v.normalize() {
            let cos = sparse_cosine(&v, &normalized);
            prop_assert!((cos - 1.0).abs() < 1e-5,
                "Normalized vector not parallel: cos={}", cos);
        }
    }

    /// Property: normalize preserves indices and dimension
    #[test]
    fn prop_normalize_preserves_indices(v in arb_nonzero_sparse_vector()) {
        if let Ok(normalized) = v.normalize() {
            prop_assert_eq!(v.indices(), normalized.indices(),
                "Indices changed after normalization");
            prop_assert_eq!(v.dim(), normalized.dim(),
                "Dimension changed after normalization");
        }
    }

    // === Cross-validation ===

    /// Property: sparse dot equals dense dot for small vectors
    #[test]
    fn prop_dot_matches_dense((a, b) in arb_small_sparse_vector_pair()) {
        // Convert to dense and compute
        let dim = a.dim() as usize;
        let mut dense_a = vec![0.0f32; dim];
        let mut dense_b = vec![0.0f32; dim];

        for (idx, val) in a.indices().iter().zip(a.values().iter()) {
            dense_a[*idx as usize] = *val;
        }
        for (idx, val) in b.indices().iter().zip(b.values().iter()) {
            dense_b[*idx as usize] = *val;
        }

        let dense_dot: f32 = dense_a.iter()
            .zip(dense_b.iter())
            .map(|(x, y)| x * y)
            .sum();

        let sparse_dot = sparse_dot_product(&a, &b);

        // Use relative tolerance for comparison
        prop_assert!(approx_eq(dense_dot, sparse_dot, 1e-5),
            "Dense dot {} != sparse dot {}", dense_dot, sparse_dot);
    }
}
