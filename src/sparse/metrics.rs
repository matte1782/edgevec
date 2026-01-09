//! Distance metrics for sparse vectors.
//!
//! Implementation for Day 3.

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
#[must_use]
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
#[must_use]
pub fn sparse_norm(v: &SparseVector) -> f32 {
    v.values().iter().map(|x| x * x).sum::<f32>().sqrt()
}

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
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============= Dot Product Tests =============

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
        assert!((dot - 0.0).abs() < 1e-6);
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

        let ab = sparse_dot_product(&a, &b);
        let ba = sparse_dot_product(&b, &a);
        assert!((ab - ba).abs() < 1e-6);
    }

    // ============= Norm Tests =============

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

    // ============= Cosine Tests =============

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
        assert!((-1.0..=1.0).contains(&cos));
    }
}
