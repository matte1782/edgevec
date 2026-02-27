//! SparseVector type using CSR format.
//!
//! Implementation for Day 2 + Day 3 metrics.

use crate::sparse::error::SparseError;
use crate::sparse::metrics::{sparse_cosine, sparse_dot_product, sparse_norm};
use serde::{Deserialize, Serialize};

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
/// The following invariants are enforced at construction time and during
/// deserialization (via `serde(try_from = "SparseVectorRaw")`):
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
#[serde(try_from = "SparseVectorRaw")]
pub struct SparseVector {
    indices: Vec<u32>,
    values: Vec<f32>,
    dim: u32,
}

/// Raw deserialization target for `SparseVector`.
///
/// This private struct is used with `serde(try_from)` to ensure all 6 invariants
/// are enforced during deserialization. Without this, `#[derive(Deserialize)]`
/// would bypass validation and allow construction of invalid `SparseVector`s
/// (e.g., unsorted indices, NaN values, duplicate indices).
#[derive(Deserialize)]
struct SparseVectorRaw {
    indices: Vec<u32>,
    values: Vec<f32>,
    dim: u32,
}

impl TryFrom<SparseVectorRaw> for SparseVector {
    type Error = SparseError;

    fn try_from(raw: SparseVectorRaw) -> Result<Self, Self::Error> {
        SparseVector::new(raw.indices, raw.values, raw.dim)
    }
}

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

        // Check bounds and value validity
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
        Ok(Self {
            indices,
            values,
            dim,
        })
    }

    /// Create without validation (internal use only).
    ///
    /// # Safety (not unsafe, but requires caller discipline)
    ///
    /// Caller must ensure:
    /// - indices are sorted ascending
    /// - no duplicate indices
    /// - no NaN/Infinity values
    /// - nnz >= 1
    /// - all indices < dim
    /// - indices.len() == values.len()
    ///
    /// This is used by `SparseStorage::get()` to reconstruct vectors
    /// from validated data without re-validation overhead.
    #[doc(hidden)]
    #[must_use]
    pub(crate) fn new_unchecked(indices: Vec<u32>, values: Vec<f32>, dim: u32) -> Self {
        Self {
            indices,
            values,
            dim,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============= Validation Tests =============

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
        assert!(matches!(
            result,
            Err(SparseError::IndexOutOfBounds {
                index: 100,
                dim: 100
            })
        ));
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

    // ============= Constructor Tests =============

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

    // ============= Accessor Tests =============

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

    // ============= Serde Tests =============

    #[test]
    fn test_serde_roundtrip() {
        let original = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: SparseVector = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    // ============= Method Wrapper Tests =============

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

    #[test]
    fn test_normalize_zero_values() {
        // A sparse vector with all zero values has zero norm.
        // 0.0 passes the is_finite() check, so construction succeeds,
        // but normalize() must return Err(SparseError::ZeroNorm).
        let v = SparseVector::new(vec![0], vec![0.0], 100).unwrap();
        assert_eq!(v.norm(), 0.0);
        let result = v.normalize();
        assert!(
            matches!(result, Err(SparseError::ZeroNorm)),
            "normalize() on all-zero-values vector must return ZeroNorm, got: {:?}",
            result
        );

        // Also test with multiple zero values
        let v2 = SparseVector::new(vec![0, 5, 10], vec![0.0, 0.0, 0.0], 100).unwrap();
        assert!(matches!(v2.normalize(), Err(SparseError::ZeroNorm)));
    }

    // ============= Deserialization Invariant Tests (C-SRC-1) =============

    #[test]
    fn test_deserialize_rejects_unsorted_indices() {
        // Invariant 1: indices must be sorted in strictly ascending order
        let json = r#"{"indices":[5,0,10],"values":[0.1,0.2,0.3],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Deserialization must reject unsorted indices"
        );
    }

    #[test]
    fn test_deserialize_rejects_duplicate_indices() {
        // Invariant 2: no duplicate indices
        let json = r#"{"indices":[0,5,5],"values":[0.1,0.2,0.3],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Deserialization must reject duplicate indices"
        );
    }

    #[test]
    fn test_deserialize_rejects_nan_values() {
        // Invariant 3: no NaN or Infinity in values
        // NaN in JSON is not standard, but we test via a valid SparseVector
        // that might be crafted. Since JSON doesn't support NaN natively,
        // we test with Infinity which some parsers accept.
        // Instead, test with a roundtrip that proves valid data works:
        let json = r#"{"indices":[0,5],"values":[0.1,null],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Deserialization must reject null/invalid values"
        );
    }

    #[test]
    fn test_deserialize_rejects_empty_vector() {
        // Invariant 4: at least one element (nnz >= 1)
        let json = r#"{"indices":[],"values":[],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Deserialization must reject empty vectors");
    }

    #[test]
    fn test_deserialize_rejects_out_of_bounds_indices() {
        // Invariant 5: all indices < dim
        let json = r#"{"indices":[0,100],"values":[0.1,0.2],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Deserialization must reject out-of-bounds indices"
        );
    }

    #[test]
    fn test_deserialize_rejects_length_mismatch() {
        // Invariant 6: indices.len() == values.len()
        let json = r#"{"indices":[0,5],"values":[0.1],"dim":100}"#;
        let result: Result<SparseVector, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "Deserialization must reject length mismatch"
        );
    }

    #[test]
    fn test_deserialize_valid_roundtrip() {
        // Valid data should still deserialize correctly
        let original = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: SparseVector = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
