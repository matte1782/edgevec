# Week 37 Day 4: SparseVector Property Tests

**Date:** 2026-01-15
**Focus:** Verify SparseVector invariants with proptest
**Estimated Duration:** 2.75 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** Day 2 (SparseVector Implementation) — MUST BE COMPLETE

---

## Tasks

### W37.4.1: Create Test File Structure

**Objective:** Set up `tests/sparse_vector_test.rs` with proptest.

**Rust Implementation:**

```rust
// tests/sparse_vector_test.rs

//! Property tests for SparseVector.
//!
//! These tests verify that SparseVector maintains its invariants
//! under arbitrary valid inputs.

use proptest::prelude::*;
use edgevec::sparse::{SparseVector, SparseError};

/// Maximum dimension for generated vectors.
const MAX_DIM: u32 = 10_000;

/// Maximum non-zero elements for generated vectors.
const MAX_NNZ: usize = 500;
```

**Cargo.toml Check:**

```toml
[dev-dependencies]
proptest = "1.4"
```

**Acceptance Criteria:**
- [ ] Test file created at `tests/sparse_vector_test.rs`
- [ ] `proptest` in dev-dependencies (already present)
- [ ] Constants defined for test bounds

**Estimated Duration:** 15 minutes

**Agent:** TEST_ENGINEER

---

### W37.4.2: Implement Arbitrary SparseVector Generator

**Objective:** Create proptest strategy for valid `SparseVector`.

**Rust Implementation:**

```rust
// tests/sparse_vector_test.rs (continued)

/// Strategy to generate valid SparseVector instances.
///
/// Generates:
/// - Dimension in [1, MAX_DIM]
/// - NNZ in [1, min(MAX_NNZ, dim)]
/// - Sorted unique indices < dim
/// - Finite f32 values (no NaN/Infinity)
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    // First generate dimension
    (1u32..=MAX_DIM).prop_flat_map(|dim| {
        // NNZ must be at least 1 and at most dim
        let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

        (Just(dim), 1..=max_nnz).prop_flat_map(move |(dim, nnz)| {
            // Generate nnz unique indices in [0, dim)
            let indices_strategy = proptest::collection::btree_set(0u32..dim, nnz)
                .prop_map(|set| set.into_iter().collect::<Vec<_>>());

            // Generate nnz finite f32 values
            let values_strategy = proptest::collection::vec(
                prop::num::f32::NORMAL,  // Excludes NaN and Infinity
                nnz
            );

            (Just(dim), indices_strategy, values_strategy)
        })
    })
    .prop_map(|(dim, indices, values)| {
        SparseVector::new(indices, values, dim)
            .expect("Generator should produce valid vectors")
    })
}

/// Strategy to generate pairs of SparseVectors with same dimension.
fn arb_sparse_vector_pair() -> impl Strategy<Value = (SparseVector, SparseVector)> {
    (1u32..=MAX_DIM).prop_flat_map(|dim| {
        let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

        // Generate two vectors with same dimension
        let v1 = (1..=max_nnz).prop_flat_map(move |nnz| {
            let indices = proptest::collection::btree_set(0u32..dim, nnz)
                .prop_map(|set| set.into_iter().collect::<Vec<_>>());
            let values = proptest::collection::vec(prop::num::f32::NORMAL, nnz);
            (Just(dim), indices, values)
        });

        let v2 = (1..=max_nnz).prop_flat_map(move |nnz| {
            let indices = proptest::collection::btree_set(0u32..dim, nnz)
                .prop_map(|set| set.into_iter().collect::<Vec<_>>());
            let values = proptest::collection::vec(prop::num::f32::NORMAL, nnz);
            (Just(dim), indices, values)
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
```

**Acceptance Criteria:**
- [ ] `arb_sparse_vector()` generates valid vectors
- [ ] Indices are always sorted (uses BTreeSet)
- [ ] No duplicates (uses BTreeSet)
- [ ] No NaN/Infinity (uses `f32::NORMAL`)
- [ ] NNZ always >= 1
- [ ] Dimension respects bounds
- [ ] `arb_sparse_vector_pair()` generates same-dimension pairs

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

### W37.4.3: Write Unit Tests

**Objective:** Comprehensive unit tests for edge cases.

**Rust Implementation:**

```rust
// tests/sparse_vector_test.rs (continued)

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
```

**Acceptance Criteria:**
- [ ] All validation error cases tested
- [ ] `from_pairs` sorting verified
- [ ] `singleton` constructor tested
- [ ] Boundary cases tested (dim=1, index=dim-1)
- [ ] `get` method tested

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

### W37.4.4: Write Property Tests

**Objective:** Verify invariants hold for all generated vectors.

**Rust Implementation:**

```rust
// tests/sparse_vector_test.rs (continued)

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
}
```

**Acceptance Criteria:**
- [ ] 1000+ test cases per property
- [ ] `prop_indices_sorted` — indices always ascending
- [ ] `prop_nnz_matches_indices` — nnz equals indices.len()
- [ ] `prop_nnz_matches_values` — nnz equals values.len()
- [ ] `prop_values_finite` — no NaN/Infinity
- [ ] `prop_indices_in_bounds` — all indices < dim
- [ ] `prop_nnz_nonzero` — nnz >= 1
- [ ] `prop_roundtrip_pairs` — to_pairs/from_pairs roundtrip
- [ ] `prop_no_duplicate_indices` — no duplicates
- [ ] `prop_get_existing` — get returns correct values

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Day 4 Checklist

- [ ] W37.4.1: Test file structure created
- [ ] W37.4.2: Arbitrary SparseVector generator
- [ ] W37.4.3: Unit tests for edge cases
- [ ] W37.4.4: Property tests (8+ properties)
- [ ] All tests pass with 1000+ cases
- [ ] No test flakiness

## Day 4 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| All unit tests pass | `cargo test --features sparse` |
| Property tests run 1000+ cases | Config in proptest |
| No test flakiness | Run 3 times |
| Arbitrary generator valid | Produces valid vectors |

## Day 4 Verification Commands

```bash
# Run all sparse vector tests
cargo test --features sparse sparse_vector

# Run property tests with more cases
PROPTEST_CASES=10000 cargo test --features sparse sparse_vector

# Check for flakiness (run multiple times)
for i in {1..3}; do cargo test --features sparse sparse_vector; done
```

## Day 4 Handoff

After completing Day 4:

**Artifacts Generated:**
- `tests/sparse_vector_test.rs`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 5 — Metrics Property Tests

---

*Agent: PLANNER + TEST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-08*
