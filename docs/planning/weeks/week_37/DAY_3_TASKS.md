# Week 37 Day 3: Sparse Metrics Implementation

**Date:** 2026-01-14
**Focus:** Implement dot product, norm, and cosine similarity
**Estimated Duration:** 2 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** Day 2 (SparseVector Implementation) — MUST BE COMPLETE

---

## Tasks

### W37.3.1: Implement `sparse_dot_product`

**Objective:** Implement O(|a| + |b|) merge-intersection dot product.

**Rust Implementation:**

```rust
// src/sparse/metrics.rs

use crate::sparse::SparseVector;

/// Sparse dot product using merge-intersection algorithm.
///
/// # Algorithm
///
/// Uses a two-pointer merge to find matching indices. Only indices
/// present in both vectors contribute to the dot product.
///
/// # Complexity
///
/// - Time: O(|a| + |b|) worst case, O(min(|a|, |b|)) best case
/// - Space: O(1)
///
/// # Arguments
///
/// * `a` - First sparse vector
/// * `b` - Second sparse vector
///
/// # Returns
///
/// The dot product: sum of a[i] * b[i] for all common indices i.
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::{SparseVector, sparse_dot_product};
///
/// let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100)?;
/// let b = SparseVector::new(vec![5, 10, 20], vec![0.5, 0.5, 1.0], 100)?;
///
/// // Only indices 5 and 10 are common: 2.0*0.5 + 3.0*0.5 = 2.5
/// let dot = sparse_dot_product(&a, &b);
/// assert!((dot - 2.5).abs() < 1e-6);
/// # Ok::<(), edgevec::sparse::SparseError>(())
/// ```
#[inline]
pub fn sparse_dot_product(a: &SparseVector, b: &SparseVector) -> f32 {
    let a_indices = a.indices();
    let a_values = a.values();
    let b_indices = b.indices();
    let b_values = b.values();

    let mut result = 0.0f32;
    let mut i = 0usize;
    let mut j = 0usize;

    // Merge-intersection: advance pointers until one is exhausted
    while i < a_indices.len() && j < b_indices.len() {
        match a_indices[i].cmp(&b_indices[j]) {
            std::cmp::Ordering::Less => {
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                // Matching index: accumulate product
                result += a_values[i] * b_values[j];
                i += 1;
                j += 1;
            }
        }
    }

    result
}
```

**Acceptance Criteria:**
- [ ] Two-pointer merge algorithm implemented
- [ ] O(|a| + |b|) time complexity
- [ ] O(1) space complexity
- [ ] Handles non-overlapping vectors (returns 0)
- [ ] Handles identical vectors (returns sum of squares)
- [ ] Doc comments with complexity analysis

**Test Cases:**

```rust
#[cfg(test)]
mod dot_product_tests {
    use super::*;

    #[test]
    fn test_dot_product_overlap() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10, 20], vec![0.5, 0.5, 1.0], 100).unwrap();

        let dot = sparse_dot_product(&a, &b);
        // 2.0*0.5 + 3.0*0.5 = 2.5
        assert!((dot - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_no_overlap() {
        let a = SparseVector::new(vec![0, 1, 2], vec![1.0, 1.0, 1.0], 100).unwrap();
        let b = SparseVector::new(vec![10, 11, 12], vec![1.0, 1.0, 1.0], 100).unwrap();

        let dot = sparse_dot_product(&a, &b);
        assert_eq!(dot, 0.0);
    }

    #[test]
    fn test_dot_product_self() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();

        let dot = sparse_dot_product(&a, &a);
        // 1.0^2 + 2.0^2 + 3.0^2 = 14.0
        assert!((dot - 14.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_exact_match() {
        let a = SparseVector::new(vec![0, 1, 2], vec![1.0, 2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![0, 1, 2], vec![4.0, 5.0, 6.0], 100).unwrap();

        let dot = sparse_dot_product(&a, &b);
        // 1*4 + 2*5 + 3*6 = 32
        assert!((dot - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_singleton() {
        let a = SparseVector::singleton(5, 3.0, 100).unwrap();
        let b = SparseVector::singleton(5, 4.0, 100).unwrap();

        let dot = sparse_dot_product(&a, &b);
        assert!((dot - 12.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_commutative() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10, 20], vec![0.5, 0.5, 1.0], 100).unwrap();

        assert_eq!(sparse_dot_product(&a, &b), sparse_dot_product(&b, &a));
    }
}
```

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W37.3.2: Implement `sparse_norm`

**Objective:** Implement L2 norm calculation.

**Rust Implementation:**

```rust
// src/sparse/metrics.rs (continued)

/// L2 norm (Euclidean length) of a sparse vector.
///
/// # Formula
///
/// ||v|| = sqrt(sum(v_i^2)) for all non-zero v_i
///
/// # Complexity
///
/// - Time: O(nnz)
/// - Space: O(1)
///
/// # Arguments
///
/// * `v` - Sparse vector
///
/// # Returns
///
/// The L2 norm of the vector. Always >= 0.
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::{SparseVector, sparse_norm};
///
/// let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100)?;
/// let norm = sparse_norm(&v);
/// assert!((norm - 5.0).abs() < 1e-6); // 3-4-5 triangle
/// # Ok::<(), edgevec::sparse::SparseError>(())
/// ```
#[inline]
pub fn sparse_norm(v: &SparseVector) -> f32 {
    v.values()
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        .sqrt()
}
```

**Acceptance Criteria:**
- [ ] Returns sqrt(sum of squares)
- [ ] O(nnz) time complexity
- [ ] O(1) space complexity
- [ ] Works with singleton vectors

**Test Cases:**

```rust
#[cfg(test)]
mod norm_tests {
    use super::*;

    #[test]
    fn test_norm_345() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        let norm = sparse_norm(&v);
        assert!((norm - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_norm_unit() {
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        let norm = sparse_norm(&v);
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_norm_multiple() {
        let v = SparseVector::new(vec![0, 1, 2], vec![1.0, 1.0, 1.0], 100).unwrap();
        let norm = sparse_norm(&v);
        // sqrt(3) ≈ 1.732
        assert!((norm - 3.0f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_norm_equals_sqrt_dot_self() {
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let norm = sparse_norm(&v);
        let dot_self = sparse_dot_product(&v, &v);
        assert!((norm - dot_self.sqrt()).abs() < 1e-6);
    }
}
```

**Estimated Duration:** 15 minutes

**Agent:** RUST_ENGINEER

---

### W37.3.3: Implement `sparse_cosine`

**Objective:** Implement cosine similarity with zero-vector handling.

**Rust Implementation:**

```rust
// src/sparse/metrics.rs (continued)

/// Cosine similarity between two sparse vectors.
///
/// # Formula
///
/// cos(a, b) = dot(a, b) / (||a|| * ||b||)
///
/// # Complexity
///
/// - Time: O(|a| + |b|) for dot product + O(|a| + |b|) for norms
/// - Space: O(1)
///
/// # Arguments
///
/// * `a` - First sparse vector
/// * `b` - Second sparse vector
///
/// # Returns
///
/// Cosine similarity in range [-1, 1]. Returns 0.0 if either vector has zero norm
/// (which shouldn't happen with valid SparseVectors since nnz >= 1).
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::{SparseVector, sparse_cosine};
///
/// // Identical directions have cosine = 1.0
/// let a = SparseVector::new(vec![0, 1], vec![1.0, 0.0], 100)?;
/// let b = SparseVector::new(vec![0, 1], vec![2.0, 0.0], 100)?;
/// let cos = sparse_cosine(&a, &b);
/// assert!((cos - 1.0).abs() < 1e-6);
/// # Ok::<(), edgevec::sparse::SparseError>(())
/// ```
#[inline]
pub fn sparse_cosine(a: &SparseVector, b: &SparseVector) -> f32 {
    let dot = sparse_dot_product(a, b);
    let norm_a = sparse_norm(a);
    let norm_b = sparse_norm(b);

    let denom = norm_a * norm_b;

    if denom == 0.0 {
        // Zero vector (shouldn't happen with valid SparseVector)
        return 0.0;
    }

    dot / denom
}
```

**Acceptance Criteria:**
- [ ] Returns dot(a,b) / (||a|| * ||b||)
- [ ] Returns 0.0 for zero norm (defensive)
- [ ] Result in range [-1, 1] for valid vectors
- [ ] Cosine of vector with itself is 1.0

**Test Cases:**

```rust
#[cfg(test)]
mod cosine_tests {
    use super::*;

    #[test]
    fn test_cosine_self_is_one() {
        let v = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let cos = sparse_cosine(&v, &v);
        assert!((cos - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_parallel() {
        let a = SparseVector::new(vec![0, 1], vec![1.0, 0.0], 100).unwrap();
        let b = SparseVector::new(vec![0, 1], vec![5.0, 0.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        // No overlap = orthogonal in sparse space
        let a = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let b = SparseVector::new(vec![1], vec![1.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_antiparallel() {
        let a = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let b = SparseVector::new(vec![0], vec![-1.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!((cos - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_commutative() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10, 20], vec![0.5, 0.5, 1.0], 100).unwrap();
        assert!((sparse_cosine(&a, &b) - sparse_cosine(&b, &a)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_in_range() {
        let a = SparseVector::new(vec![0, 5, 10], vec![1.0, -2.0, 3.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10, 20], vec![0.5, -0.5, 1.0], 100).unwrap();
        let cos = sparse_cosine(&a, &b);
        assert!(cos >= -1.0 && cos <= 1.0);
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W37.3.4: Add Method Wrappers to `SparseVector`

**Objective:** Add convenience methods to `SparseVector`.

**Rust Implementation:**

```rust
// src/sparse/vector.rs (add these methods)

use crate::sparse::metrics::{sparse_dot_product, sparse_norm, sparse_cosine};

impl SparseVector {
    /// Compute dot product with another sparse vector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let a = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100)?;
    /// let b = SparseVector::new(vec![5, 10], vec![3.0, 1.0], 100)?;
    /// let dot = a.dot(&b);
    /// assert!((dot - 6.0).abs() < 1e-6);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn dot(&self, other: &SparseVector) -> f32 {
        sparse_dot_product(self, other)
    }

    /// Compute L2 norm.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100)?;
    /// assert!((v.norm() - 5.0).abs() < 1e-6);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn norm(&self) -> f32 {
        sparse_norm(self)
    }

    /// Compute cosine similarity with another sparse vector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let a = SparseVector::new(vec![0], vec![1.0], 100)?;
    /// let b = SparseVector::new(vec![0], vec![2.0], 100)?;
    /// assert!((a.cosine(&b) - 1.0).abs() < 1e-6);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn cosine(&self, other: &SparseVector) -> f32 {
        sparse_cosine(self, other)
    }

    /// Return a normalized copy with unit L2 norm.
    ///
    /// # Errors
    ///
    /// Returns `SparseError::ZeroNorm` if the vector has zero norm.
    /// This shouldn't happen with valid `SparseVector` since nnz >= 1.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100)?;
    /// let normalized = v.normalize()?;
    /// assert!((normalized.norm() - 1.0).abs() < 1e-6);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn normalize(&self) -> Result<Self, SparseError> {
        let norm = self.norm();

        if norm == 0.0 {
            return Err(SparseError::ZeroNorm);
        }

        let normalized_values: Vec<f32> = self.values.iter().map(|v| v / norm).collect();

        // Safety: indices are already valid, and normalized values are finite
        // (dividing finite by non-zero finite gives finite)
        Ok(Self {
            indices: self.indices.clone(),
            values: normalized_values,
            dim: self.dim,
        })
    }
}
```

**Acceptance Criteria:**
- [ ] `dot()` method wraps `sparse_dot_product`
- [ ] `norm()` method wraps `sparse_norm`
- [ ] `cosine()` method wraps `sparse_cosine`
- [ ] `normalize()` returns unit vector or `ZeroNorm` error
- [ ] All methods have doc comments with examples

**Test Cases:**

```rust
#[cfg(test)]
mod method_tests {
    use super::*;

    #[test]
    fn test_dot_method() {
        let a = SparseVector::new(vec![0, 5], vec![1.0, 2.0], 100).unwrap();
        let b = SparseVector::new(vec![5, 10], vec![3.0, 1.0], 100).unwrap();
        assert!((a.dot(&b) - 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_norm_method() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        assert!((v.norm() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_method() {
        let a = SparseVector::new(vec![0], vec![1.0], 100).unwrap();
        let b = SparseVector::new(vec![0], vec![2.0], 100).unwrap();
        assert!((a.cosine(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let v = SparseVector::new(vec![0, 1], vec![3.0, 4.0], 100).unwrap();
        let normalized = v.normalize().unwrap();

        assert!((normalized.norm() - 1.0).abs() < 1e-6);
        // Values should be 3/5 = 0.6 and 4/5 = 0.8
        assert!((normalized.values()[0] - 0.6).abs() < 1e-6);
        assert!((normalized.values()[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_preserves_indices() {
        let v = SparseVector::new(vec![5, 10, 20], vec![1.0, 2.0, 3.0], 100).unwrap();
        let normalized = v.normalize().unwrap();

        assert_eq!(normalized.indices(), &[5, 10, 20]);
        assert_eq!(normalized.dim(), 100);
    }
}
```

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

## Day 3 Checklist

- [ ] W37.3.1: `sparse_dot_product` implemented
- [ ] W37.3.2: `sparse_norm` implemented
- [ ] W37.3.3: `sparse_cosine` implemented
- [ ] W37.3.4: Method wrappers on `SparseVector`
- [ ] All unit tests pass
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] `cargo test --features sparse` passes

## Day 3 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `sparse_dot_product` matches dense computation | Cross-check test |
| `sparse_cosine` returns value in [-1, 1] | Unit tests |
| `normalize` returns unit vector | Unit tests |
| All methods documented | `cargo doc` |
| Clippy clean | `cargo clippy -- -D warnings` |

## Day 3 Handoff

After completing Day 3:

**Artifacts Generated:**
- Completed `src/sparse/metrics.rs`
- Updated `src/sparse/vector.rs` with method wrappers
- Unit tests for all metrics

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 4 — SparseVector Property Tests

---

*Agent: PLANNER + RUST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-08*
