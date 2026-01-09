# Week 37 Day 2: SparseVector Implementation

**Date:** 2026-01-13
**Focus:** Implement `SparseVector` struct with full validation
**Estimated Duration:** 2.25 hours
**Phase:** RFC-007 Implementation Phase 1 (Core Types)
**Dependencies:** Day 1 (Module Structure) — MUST BE COMPLETE

---

## Tasks

### W37.2.1: Define `SparseVector` Struct

**Objective:** Complete the `SparseVector` struct with serde support.

**Rust Implementation:**

```rust
// src/sparse/vector.rs

use serde::{Deserialize, Serialize};
use crate::sparse::error::SparseError;

/// Sparse vector using Compressed Sparse Row (CSR) format.
///
/// # Memory Layout
///
/// - `indices`: `[u32; N]` — sorted positions of non-zero elements (8 bytes overhead + 4N bytes)
/// - `values`: `[f32; N]` — corresponding values (8 bytes overhead + 4N bytes)
/// - `dim`: `u32` — maximum dimension (4 bytes)
///
/// Total: ~20 bytes overhead + 8N bytes for N non-zero elements.
///
/// # Invariants
///
/// The following invariants are enforced at construction time:
///
/// 1. `indices` are sorted in strictly ascending order
/// 2. No duplicate indices
/// 3. No NaN or Infinity in `values`
/// 4. At least one element (`nnz >= 1`)
/// 5. All indices are `< dim`
/// 6. `indices.len() == values.len()`
///
/// # Example
///
/// ```rust
/// use edgevec::sparse::SparseVector;
///
/// // Create a sparse vector representing [0, 0.5, 0, 0, 0, 0.3, 0, 0, 0, 0, 0.2]
/// let sparse = SparseVector::new(
///     vec![1, 5, 10],      // indices of non-zero elements
///     vec![0.5, 0.3, 0.2], // values at those indices
///     100                   // dimension (vocabulary size)
/// ).expect("valid sparse vector");
///
/// assert_eq!(sparse.nnz(), 3);
/// assert_eq!(sparse.dim(), 100);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SparseVector {
    indices: Vec<u32>,
    values: Vec<f32>,
    dim: u32,
}
```

**Acceptance Criteria:**
- [ ] `SparseVector` struct defined with `indices`, `values`, `dim`
- [ ] Derives `Clone`, `Debug`, `Serialize`, `Deserialize`, `PartialEq`
- [ ] Doc comments with memory layout documentation
- [ ] Example code in doc comment

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W37.2.2: Implement Validation Helper

**Objective:** Create validation function that enforces all invariants.

**Rust Implementation:**

```rust
// src/sparse/vector.rs (continued)

impl SparseVector {
    /// Validate indices and values against all invariants.
    ///
    /// # Errors
    ///
    /// Returns `SparseError` if any invariant is violated:
    /// - `LengthMismatch` if indices and values have different lengths
    /// - `EmptyVector` if no elements provided
    /// - `UnsortedIndices` if indices are not strictly ascending
    /// - `DuplicateIndex` if any index appears twice
    /// - `IndexOutOfBounds` if any index >= dim
    /// - `InvalidValue` if any value is NaN or Infinity
    fn validate(indices: &[u32], values: &[f32], dim: u32) -> Result<(), SparseError> {
        // Check length mismatch
        if indices.len() != values.len() {
            return Err(SparseError::LengthMismatch {
                indices: indices.len(),
                values: values.len(),
            });
        }

        // Check empty vector
        if indices.is_empty() {
            return Err(SparseError::EmptyVector);
        }

        // Check sorted and no duplicates
        for i in 1..indices.len() {
            match indices[i - 1].cmp(&indices[i]) {
                std::cmp::Ordering::Greater => return Err(SparseError::UnsortedIndices),
                std::cmp::Ordering::Equal => return Err(SparseError::DuplicateIndex(i)),
                std::cmp::Ordering::Less => {}
            }
        }

        // Check bounds
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= dim {
                return Err(SparseError::IndexOutOfBounds { index: idx, dim });
            }

            // Check value validity (NaN or Infinity)
            if !values[i].is_finite() {
                return Err(SparseError::InvalidValue(i));
            }
        }

        Ok(())
    }
}
```

**Acceptance Criteria:**
- [ ] Validates length mismatch
- [ ] Validates empty vector
- [ ] Validates sorted ascending order
- [ ] Validates no duplicates
- [ ] Validates all indices < dim
- [ ] Validates no NaN/Infinity values
- [ ] Returns appropriate `SparseError` variant for each failure

**Test Cases:**

```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_valid() {
        assert!(SparseVector::validate(&[0, 5, 10], &[0.1, 0.2, 0.3], 100).is_ok());
    }

    #[test]
    fn test_validate_length_mismatch() {
        let result = SparseVector::validate(&[0, 5], &[0.1], 100);
        assert!(matches!(result, Err(SparseError::LengthMismatch { .. })));
    }

    #[test]
    fn test_validate_empty() {
        let result = SparseVector::validate(&[], &[], 100);
        assert!(matches!(result, Err(SparseError::EmptyVector)));
    }

    #[test]
    fn test_validate_unsorted() {
        let result = SparseVector::validate(&[5, 0, 10], &[0.1, 0.2, 0.3], 100);
        assert!(matches!(result, Err(SparseError::UnsortedIndices)));
    }

    #[test]
    fn test_validate_duplicate() {
        let result = SparseVector::validate(&[0, 5, 5], &[0.1, 0.2, 0.3], 100);
        assert!(matches!(result, Err(SparseError::DuplicateIndex(2))));
    }

    #[test]
    fn test_validate_out_of_bounds() {
        let result = SparseVector::validate(&[0, 100], &[0.1, 0.2], 100);
        assert!(matches!(result, Err(SparseError::IndexOutOfBounds { index: 100, dim: 100 })));
    }

    #[test]
    fn test_validate_nan() {
        let result = SparseVector::validate(&[0, 5], &[0.1, f32::NAN], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(1))));
    }

    #[test]
    fn test_validate_infinity() {
        let result = SparseVector::validate(&[0, 5], &[f32::INFINITY, 0.2], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(0))));
    }

    #[test]
    fn test_validate_neg_infinity() {
        let result = SparseVector::validate(&[0], &[f32::NEG_INFINITY], 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(0))));
    }
}
```

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W37.2.3: Implement Constructors

**Objective:** Implement `new`, `from_pairs`, and `singleton` constructors.

**Rust Implementation:**

```rust
// src/sparse/vector.rs (continued)

impl SparseVector {
    /// Create a sparse vector from pre-sorted indices and values.
    ///
    /// # Arguments
    ///
    /// * `indices` - Sorted indices of non-zero elements
    /// * `values` - Values at those indices
    /// * `dim` - Maximum dimension (vocabulary size)
    ///
    /// # Errors
    ///
    /// Returns `SparseError` if validation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let sparse = SparseVector::new(
    ///     vec![0, 5, 10],
    ///     vec![0.5, 0.3, 0.2],
    ///     100
    /// )?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn new(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Result<Self, SparseError> {
        Self::validate(&indices, &values, dim)?;
        Ok(Self { indices, values, dim })
    }

    /// Create a sparse vector from unsorted index-value pairs.
    ///
    /// This constructor sorts the pairs internally before validation.
    ///
    /// # Arguments
    ///
    /// * `pairs` - Slice of (index, value) tuples
    /// * `dim` - Maximum dimension
    ///
    /// # Errors
    ///
    /// Returns `SparseError` if validation fails after sorting.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// // Order doesn't matter - will be sorted
    /// let sparse = SparseVector::from_pairs(&[(10, 0.2), (0, 0.5), (5, 0.3)], 100)?;
    /// assert_eq!(sparse.indices(), &[0, 5, 10]);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn from_pairs(pairs: &[(u32, f32)], dim: u32) -> Result<Self, SparseError> {
        if pairs.is_empty() {
            return Err(SparseError::EmptyVector);
        }

        // Sort by index
        let mut sorted: Vec<(u32, f32)> = pairs.to_vec();
        sorted.sort_by_key(|(idx, _)| *idx);

        // Split into indices and values
        let (indices, values): (Vec<u32>, Vec<f32>) = sorted.into_iter().unzip();

        Self::new(indices, values, dim)
    }

    /// Create a sparse vector with a single element.
    ///
    /// This is the minimum valid sparse vector (nnz = 1).
    ///
    /// # Arguments
    ///
    /// * `index` - Index of the single non-zero element
    /// * `value` - Value at that index
    /// * `dim` - Maximum dimension
    ///
    /// # Errors
    ///
    /// Returns `SparseError` if index >= dim or value is NaN/Infinity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let sparse = SparseVector::singleton(42, 1.0, 100)?;
    /// assert_eq!(sparse.nnz(), 1);
    /// assert_eq!(sparse.indices(), &[42]);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn singleton(index: u32, value: f32, dim: u32) -> Result<Self, SparseError> {
        Self::new(vec![index], vec![value], dim)
    }
}
```

**Acceptance Criteria:**
- [ ] `new()` validates and creates from pre-sorted data
- [ ] `from_pairs()` sorts internally then validates
- [ ] `singleton()` creates minimal valid vector
- [ ] All constructors return `Result<Self, SparseError>`
- [ ] Doc comments with examples

**Test Cases:**

```rust
#[cfg(test)]
mod constructor_tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let sparse = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100);
        assert!(sparse.is_ok());
        let sparse = sparse.unwrap();
        assert_eq!(sparse.nnz(), 3);
        assert_eq!(sparse.dim(), 100);
    }

    #[test]
    fn test_new_invalid() {
        let result = SparseVector::new(vec![10, 5, 0], vec![0.1, 0.2, 0.3], 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_pairs_sorts() {
        let sparse = SparseVector::from_pairs(&[(10, 0.3), (0, 0.1), (5, 0.2)], 100);
        assert!(sparse.is_ok());
        let sparse = sparse.unwrap();
        assert_eq!(sparse.indices(), &[0, 5, 10]);
        assert_eq!(sparse.values(), &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_from_pairs_duplicate_fails() {
        let result = SparseVector::from_pairs(&[(5, 0.1), (5, 0.2)], 100);
        assert!(matches!(result, Err(SparseError::DuplicateIndex(_))));
    }

    #[test]
    fn test_from_pairs_empty_fails() {
        let result = SparseVector::from_pairs(&[], 100);
        assert!(matches!(result, Err(SparseError::EmptyVector)));
    }

    #[test]
    fn test_singleton() {
        let sparse = SparseVector::singleton(42, 1.0, 100);
        assert!(sparse.is_ok());
        let sparse = sparse.unwrap();
        assert_eq!(sparse.nnz(), 1);
        assert_eq!(sparse.indices(), &[42]);
        assert_eq!(sparse.values(), &[1.0]);
    }

    #[test]
    fn test_singleton_out_of_bounds() {
        let result = SparseVector::singleton(100, 1.0, 100);
        assert!(matches!(result, Err(SparseError::IndexOutOfBounds { .. })));
    }

    #[test]
    fn test_singleton_nan() {
        let result = SparseVector::singleton(0, f32::NAN, 100);
        assert!(matches!(result, Err(SparseError::InvalidValue(_))));
    }
}
```

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W37.2.4: Implement Accessors

**Objective:** Implement accessor methods for `SparseVector`.

**Rust Implementation:**

```rust
// src/sparse/vector.rs (continued)

impl SparseVector {
    /// Get the indices of non-zero elements.
    ///
    /// Indices are always sorted in ascending order.
    #[must_use]
    #[inline]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Get the values of non-zero elements.
    ///
    /// Values correspond to indices at the same position.
    #[must_use]
    #[inline]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Get the maximum dimension (vocabulary size).
    #[must_use]
    #[inline]
    pub fn dim(&self) -> u32 {
        self.dim
    }

    /// Get the number of non-zero elements.
    #[must_use]
    #[inline]
    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    /// Convert to index-value pairs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let sparse = SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100)?;
    /// let pairs = sparse.to_pairs();
    /// assert_eq!(pairs, vec![(0, 0.1), (5, 0.2)]);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn to_pairs(&self) -> Vec<(u32, f32)> {
        self.indices
            .iter()
            .copied()
            .zip(self.values.iter().copied())
            .collect()
    }

    /// Get value at a specific index, or None if not present.
    ///
    /// Uses binary search for O(log n) lookup.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::sparse::SparseVector;
    ///
    /// let sparse = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100)?;
    /// assert_eq!(sparse.get(5), Some(0.2));
    /// assert_eq!(sparse.get(3), None);
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    #[must_use]
    pub fn get(&self, index: u32) -> Option<f32> {
        self.indices
            .binary_search(&index)
            .ok()
            .map(|pos| self.values[pos])
    }
}
```

**Acceptance Criteria:**
- [ ] `indices()` returns sorted indices slice
- [ ] `values()` returns values slice
- [ ] `dim()` returns dimension
- [ ] `nnz()` returns non-zero count
- [ ] `to_pairs()` returns vec of tuples
- [ ] `get()` uses binary search for O(log n) lookup
- [ ] All methods have `#[must_use]` and `#[inline]` where appropriate

**Test Cases:**

```rust
#[cfg(test)]
mod accessor_tests {
    use super::*;

    #[test]
    fn test_accessors() {
        let sparse = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();

        assert_eq!(sparse.indices(), &[0, 5, 10]);
        assert_eq!(sparse.values(), &[0.1, 0.2, 0.3]);
        assert_eq!(sparse.dim(), 100);
        assert_eq!(sparse.nnz(), 3);
    }

    #[test]
    fn test_to_pairs() {
        let sparse = SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100).unwrap();
        assert_eq!(sparse.to_pairs(), vec![(0, 0.1), (5, 0.2)]);
    }

    #[test]
    fn test_get_present() {
        let sparse = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        assert_eq!(sparse.get(0), Some(0.1));
        assert_eq!(sparse.get(5), Some(0.2));
        assert_eq!(sparse.get(10), Some(0.3));
    }

    #[test]
    fn test_get_absent() {
        let sparse = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        assert_eq!(sparse.get(1), None);
        assert_eq!(sparse.get(99), None);
    }
}
```

**Estimated Duration:** 15 minutes

**Agent:** RUST_ENGINEER

---

## Day 2 Checklist

- [ ] W37.2.1: `SparseVector` struct with serde
- [ ] W37.2.2: `validate()` helper function
- [ ] W37.2.3: `new()`, `from_pairs()`, `singleton()` constructors
- [ ] W37.2.4: Accessor methods
- [ ] All unit tests pass
- [ ] `cargo clippy --features sparse -- -D warnings` passes
- [ ] `cargo test --features sparse` passes

## Day 2 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `SparseVector::new` validates all invariants | Unit tests |
| `SparseVector::from_pairs` auto-sorts | Unit tests |
| Invalid input returns appropriate `SparseError` | Unit tests |
| Serde serialization works | Integration test |
| Clippy clean | `cargo clippy -- -D warnings` |

## Day 2 Handoff

After completing Day 2:

**Artifacts Generated:**
- Completed `src/sparse/vector.rs`
- Unit tests in `src/sparse/vector.rs`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 3 — Sparse Metrics Implementation

---

*Agent: PLANNER + RUST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2026-01-08*
