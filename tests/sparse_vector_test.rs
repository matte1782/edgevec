//! Property tests for SparseVector.
//!
//! These tests verify that SparseVector maintains its invariants
//! under arbitrary valid inputs.

use edgevec::sparse::{SparseError, SparseVector};
use proptest::prelude::*;

/// Maximum dimension for generated vectors.
const MAX_DIM: u32 = 10_000;

/// Maximum non-zero elements for generated vectors.
const MAX_NNZ: usize = 500;

// =============================================================================
// ARBITRARY GENERATORS
// =============================================================================

/// Strategy to generate valid SparseVector instances.
///
/// Generates:
/// - Dimension in [1, MAX_DIM]
/// - NNZ in [1, min(MAX_NNZ, dim)]
/// - Sorted unique indices < dim
/// - Finite f32 values (no NaN/Infinity)
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    // First generate dimension (minimum 10 to avoid constraint issues)
    (10u32..=MAX_DIM)
        .prop_flat_map(|dim| {
            // NNZ must be at least 1 and at most min(MAX_NNZ, dim)
            let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

            // Generate nnz, then indices and values
            (1..=max_nnz).prop_flat_map(move |nnz| {
                // Use prop::sample::subsequence to select nnz indices from 0..dim
                // This avoids the BTreeSet constraint issues
                let all_indices: Vec<u32> = (0..dim).collect();
                let indices_strategy =
                    prop::sample::subsequence(all_indices, nnz).prop_map(|mut v| {
                        v.sort();
                        v
                    });

                // Generate nnz finite f32 values (use POSITIVE to avoid edge cases)
                let values_strategy = proptest::collection::vec(
                    prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE,
                    nnz,
                );

                (Just(dim), indices_strategy, values_strategy)
            })
        })
        .prop_map(|(dim, indices, values)| {
            SparseVector::new(indices, values, dim).expect("Generator should produce valid vectors")
        })
}

/// Strategy to generate pairs of SparseVectors with same dimension.
fn arb_sparse_vector_pair() -> impl Strategy<Value = (SparseVector, SparseVector)> {
    // Minimum dimension of 10 to avoid constraint issues
    (10u32..=MAX_DIM)
        .prop_flat_map(|dim| {
            let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);
            let all_indices: Vec<u32> = (0..dim).collect();

            // Generate two vectors with same dimension
            let v1 = (1..=max_nnz).prop_flat_map({
                let all_indices = all_indices.clone();
                move |nnz| {
                    let indices =
                        prop::sample::subsequence(all_indices.clone(), nnz).prop_map(|mut v| {
                            v.sort();
                            v
                        });
                    let values = proptest::collection::vec(
                        prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE,
                        nnz,
                    );
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
                    let values = proptest::collection::vec(
                        prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE,
                        nnz,
                    );
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

    #[test]
    fn test_new_valid() {
        let result = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_unsorted_fails() {
        let result = SparseVector::new(vec![10, 5, 0], vec![0.1, 0.2, 0.3], 100);
        assert!(matches!(result, Err(SparseError::UnsortedIndices)));
    }

    #[test]
    fn test_new_duplicate_fails() {
        let result = SparseVector::new(vec![0, 5, 5], vec![0.1, 0.2, 0.3], 100);
        assert!(matches!(result, Err(SparseError::DuplicateIndex(_))));
    }

    #[test]
    fn test_new_nan_fails() {
        let result = SparseVector::new(vec![0], vec![f32::NAN], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(_))));
    }

    #[test]
    fn test_new_infinity_fails() {
        let result = SparseVector::new(vec![0], vec![f32::INFINITY], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(_))));
    }

    #[test]
    fn test_new_neg_infinity_fails() {
        let result = SparseVector::new(vec![0], vec![f32::NEG_INFINITY], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(_))));
    }

    #[test]
    fn test_new_empty_fails() {
        let result = SparseVector::new(vec![], vec![], 100);
        assert!(matches!(result, Err(SparseError::EmptyVector)));
    }

    #[test]
    fn test_new_out_of_bounds_fails() {
        let result = SparseVector::new(vec![100], vec![1.0], 100);
        assert!(matches!(result, Err(SparseError::IndexOutOfBounds { .. })));
    }

    #[test]
    fn test_new_length_mismatch_fails() {
        let result = SparseVector::new(vec![0, 5], vec![0.1], 100);
        assert!(matches!(result, Err(SparseError::LengthMismatch { .. })));
    }

    #[test]
    fn test_from_pairs_sorts() {
        let result = SparseVector::from_pairs(&[(10, 0.3), (0, 0.1), (5, 0.2)], 100);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert_eq!(v.indices(), &[0, 5, 10]);
        assert_eq!(v.values(), &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_singleton() {
        let result = SparseVector::singleton(42, 1.0, 100);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert_eq!(v.nnz(), 1);
        assert_eq!(v.indices(), &[42]);
    }

    #[test]
    fn test_singleton_boundary() {
        // Index at dim-1 should work
        let result = SparseVector::singleton(99, 1.0, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_existing() {
        let v = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        assert_eq!(v.get(5), Some(0.2));
    }

    #[test]
    fn test_get_missing() {
        let v = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        assert_eq!(v.get(3), None);
    }

    #[test]
    fn test_dim_1_vector() {
        let v = SparseVector::singleton(0, 1.0, 1);
        assert!(v.is_ok());
        assert_eq!(v.unwrap().dim(), 1);
    }
}

// =============================================================================
// PROPERTY TESTS
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: Indices are always sorted after construction.
    #[test]
    fn prop_indices_sorted(v in arb_sparse_vector()) {
        let indices = v.indices();
        for i in 1..indices.len() {
            prop_assert!(indices[i - 1] < indices[i],
                "Indices not sorted: {:?}", indices);
        }
    }

    /// Property: NNZ matches indices length.
    #[test]
    fn prop_nnz_matches_indices(v in arb_sparse_vector()) {
        prop_assert_eq!(v.nnz(), v.indices().len());
    }

    /// Property: NNZ matches values length.
    #[test]
    fn prop_nnz_matches_values(v in arb_sparse_vector()) {
        prop_assert_eq!(v.nnz(), v.values().len());
    }

    /// Property: All values are finite (not NaN or Infinity).
    #[test]
    fn prop_values_finite(v in arb_sparse_vector()) {
        for (i, &val) in v.values().iter().enumerate() {
            prop_assert!(val.is_finite(),
                "Value at {} is not finite: {}", i, val);
        }
    }

    /// Property: All indices are less than dimension.
    #[test]
    fn prop_indices_in_bounds(v in arb_sparse_vector()) {
        let dim = v.dim();
        for (i, &idx) in v.indices().iter().enumerate() {
            prop_assert!(idx < dim,
                "Index {} at position {} exceeds dim {}", idx, i, dim);
        }
    }

    /// Property: NNZ is always at least 1.
    #[test]
    fn prop_nnz_nonzero(v in arb_sparse_vector()) {
        prop_assert!(v.nnz() >= 1, "NNZ is zero");
    }

    /// Property: from_pairs(to_pairs(v)) produces equivalent vector.
    #[test]
    fn prop_roundtrip_pairs(v in arb_sparse_vector()) {
        let pairs = v.to_pairs();
        let reconstructed = SparseVector::from_pairs(&pairs, v.dim())
            .expect("Should reconstruct from pairs");

        prop_assert_eq!(v.indices(), reconstructed.indices());
        // Float comparison with tolerance
        for (a, b) in v.values().iter().zip(reconstructed.values().iter()) {
            prop_assert!((a - b).abs() < 1e-10,
                "Values differ: {} vs {}", a, b);
        }
    }

    /// Property: Indices have no duplicates.
    #[test]
    fn prop_no_duplicate_indices(v in arb_sparse_vector()) {
        let indices = v.indices();
        for i in 1..indices.len() {
            prop_assert!(indices[i - 1] != indices[i],
                "Duplicate index at position {}: {}", i, indices[i]);
        }
    }

    /// Property: get() returns correct values for existing indices.
    #[test]
    fn prop_get_existing(v in arb_sparse_vector()) {
        for (idx, &val) in v.indices().iter().zip(v.values().iter()) {
            let got = v.get(*idx);
            prop_assert_eq!(got, Some(val),
                "get({}) returned {:?}, expected Some({})", idx, got, val);
        }
    }

    /// Property: get() returns None for non-existing indices.
    #[test]
    fn prop_get_nonexisting(v in arb_sparse_vector()) {
        // Test a few indices that are not in the vector
        let indices_set: std::collections::HashSet<_> = v.indices().iter().copied().collect();
        for candidate in 0..v.dim() {
            if !indices_set.contains(&candidate) {
                prop_assert_eq!(v.get(candidate), None,
                    "get({}) should return None", candidate);
                break; // Just test one missing index
            }
        }
    }

    /// Property: Dimension is preserved.
    #[test]
    fn prop_dim_preserved(v in arb_sparse_vector()) {
        prop_assert!(v.dim() >= 1, "Dimension must be >= 1");
        prop_assert!(v.dim() <= MAX_DIM, "Dimension exceeds MAX_DIM");
    }

    /// Property: Same-dimension pairs have matching dims.
    #[test]
    fn prop_pair_same_dim((v1, v2) in arb_sparse_vector_pair()) {
        prop_assert_eq!(v1.dim(), v2.dim(),
            "Pair dimensions don't match: {} vs {}", v1.dim(), v2.dim());
    }
}
