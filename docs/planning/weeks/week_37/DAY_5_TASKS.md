# Week 37 Day 5: Metrics Property Tests

**Date:** 2026-01-16
**Focus:** Verify metric correctness with proptest
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** Day 3 (Sparse Metrics), Day 4 (Arbitrary Generator) — MUST BE COMPLETE

---

## Tasks

### W37.5.1: Create Metrics Test File

**Objective:** Set up `tests/sparse_metrics_test.rs`.

**Rust Implementation:**

```rust
// tests/sparse_metrics_test.rs

//! Property tests for sparse vector metrics.
//!
//! These tests verify mathematical properties of dot product,
//! cosine similarity, and normalization.

use proptest::prelude::*;
use edgevec::sparse::{SparseVector, SparseError, sparse_dot_product, sparse_cosine, sparse_norm};

// Import the arbitrary generators from sparse_vector_test
// Or duplicate them here for isolation

/// Maximum dimension for generated vectors.
const MAX_DIM: u32 = 10_000;

/// Maximum non-zero elements for generated vectors.
const MAX_NNZ: usize = 500;

/// Strategy to generate valid SparseVector instances.
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    (1u32..=MAX_DIM).prop_flat_map(|dim| {
        let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

        (Just(dim), 1..=max_nnz).prop_flat_map(move |(dim, nnz)| {
            let indices_strategy = proptest::collection::btree_set(0u32..dim, nnz)
                .prop_map(|set| set.into_iter().collect::<Vec<_>>());

            let values_strategy = proptest::collection::vec(
                prop::num::f32::NORMAL,
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

/// Strategy for non-zero vectors (guaranteed non-zero values).
fn arb_nonzero_sparse_vector() -> impl Strategy<Value = SparseVector> {
    (1u32..=MAX_DIM).prop_flat_map(|dim| {
        let max_nnz = std::cmp::min(MAX_NNZ, dim as usize);

        (Just(dim), 1..=max_nnz).prop_flat_map(move |(dim, nnz)| {
            let indices_strategy = proptest::collection::btree_set(0u32..dim, nnz)
                .prop_map(|set| set.into_iter().collect::<Vec<_>>());

            // Use non-zero values only
            let values_strategy = proptest::collection::vec(
                prop::num::f32::NORMAL.prop_filter("non-zero", |&x| x != 0.0),
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
```

**Acceptance Criteria:**
- [ ] Test file created at `tests/sparse_metrics_test.rs`
- [ ] Generators imported or duplicated
- [ ] `arb_nonzero_sparse_vector()` for cosine tests

**Estimated Duration:** 15 minutes

**Agent:** TEST_ENGINEER

---

### W37.5.2: Write Unit Tests for Metrics

**Objective:** Comprehensive unit tests for metric edge cases.

**Rust Implementation:**

```rust
// tests/sparse_metrics_test.rs (continued)

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
        let dense_dot = 1.0*0.5 + 2.0*0.5 + 3.0*0.5;

        assert!((sparse_dot - dense_dot).abs() < 1e-6);
    }
}
```

**Acceptance Criteria:**
- [ ] Dot product zero case (orthogonal)
- [ ] Dot product self case (equals norm squared)
- [ ] Cosine self is 1.0
- [ ] Cosine orthogonal is 0.0
- [ ] Normalize produces unit vector
- [ ] Cross-check with dense computation

**Estimated Duration:** 45 minutes

**Agent:** TEST_ENGINEER

---

### W37.5.3: Write Property Tests for Metrics

**Objective:** Verify mathematical properties hold for all inputs.

**Rust Implementation:**

```rust
// tests/sparse_metrics_test.rs (continued)

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
        prop_assert!((dot - norm_sq).abs() < 1e-4,
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
        prop_assert!(cos >= -1.0 - 1e-6 && cos <= 1.0 + 1e-6,
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

    /// Property: normalize preserves indices
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
    fn prop_dot_matches_dense((a, b) in arb_sparse_vector_pair()
        .prop_filter("small dim", |(a, _)| a.dim() <= 100))
    {
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

        prop_assert!((dense_dot - sparse_dot).abs() < 1e-4,
            "Dense dot {} != sparse dot {}", dense_dot, sparse_dot);
    }
}
```

**Acceptance Criteria:**
- [ ] 1000+ test cases per property
- [ ] `prop_dot_commutative` — dot(a,b) == dot(b,a)
- [ ] `prop_dot_positive_semidefinite` — dot(v,v) >= 0
- [ ] `prop_dot_self_equals_norm_squared` — dot(v,v) == ||v||²
- [ ] `prop_cosine_self_is_one` — cos(v,v) == 1.0
- [ ] `prop_cosine_in_range` — -1 <= cos <= 1
- [ ] `prop_cosine_commutative` — cos(a,b) == cos(b,a)
- [ ] `prop_norm_nonnegative` — ||v|| >= 0
- [ ] `prop_normalize_has_unit_norm` — ||normalize(v)|| == 1
- [ ] `prop_normalize_preserves_direction` — cos(v, norm(v)) == 1
- [ ] `prop_dot_matches_dense` — cross-check with dense

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Day 5 Checklist

- [ ] W37.5.1: Metrics test file created
- [ ] W37.5.2: Unit tests for all metric edge cases
- [ ] W37.5.3: Property tests (10+ properties)
- [ ] All tests pass with 1000+ cases
- [ ] Dot product matches dense computation (cross-check)
- [ ] Cosine similarity always in valid range

## Day 5 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| All property tests pass | `cargo test --features sparse sparse_metrics` |
| 1000+ cases per property | ProptestConfig |
| No numerical instability | Range checks pass |
| Cross-check with dense | `prop_dot_matches_dense` |

## Day 5 Verification Commands

```bash
# Run all metrics tests
cargo test --features sparse sparse_metrics

# Run property tests with more cases
PROPTEST_CASES=10000 cargo test --features sparse sparse_metrics

# Run with verbose output
cargo test --features sparse sparse_metrics -- --nocapture
```

## Day 5 Handoff

After completing Day 5:

**Artifacts Generated:**
- `tests/sparse_metrics_test.rs`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 6 — Benchmarks + Hostile Review

---

*Agent: PLANNER + TEST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-08*
