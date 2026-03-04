//! Product Quantization (PQ) for vector compression.
//!
//! Compresses high-dimensional f32 vectors into compact M-byte codes by
//! decomposing the vector space into M subspaces and quantizing each
//! independently using k-means codebooks (typically 256 centroids per subspace).
//!
//! # Algorithm
//!
//! 1. **Partition:** Split each D-dimensional vector into M subvectors of D/M dimensions.
//! 2. **Train:** Run k-means on each subspace independently to learn 256 centroids.
//! 3. **Encode:** Replace each subvector with the index of its nearest centroid (1 byte).
//! 4. **Search (ADC):** For a query, precompute a distance table (M × 256 entries),
//!    then approximate the distance to any encoded vector with M table lookups + additions.
//!
//! # Memory Layout
//!
//! - **PQ code:** `[u8; M]` — one centroid index per subspace
//! - **Codebook:** `Vec<f32>` of shape `[M][Ksub][D/M]`, stored flat as `M * Ksub * (D/M)` f32s
//! - **Distance table:** `Vec<f32>` of shape `[M][Ksub]`, stored flat as `M * Ksub` f32s
//!
//! # Size Analysis (D=768, M=8, Ksub=256)
//!
//! | Component | Size | Notes |
//! |-----------|------|-------|
//! | PQ code per vector | 8 bytes | M bytes per vector |
//! | Codebook (total) | 786,432 bytes (768 KB) | `256 * 768 * 4` (constant for fixed D, Ksub) |
//! | Distance table per query | 8,192 bytes (8 KB) | `8 * 256 * 4`, fits in L1 cache |
//!
//! # Compression Ratio
//!
//! - Raw: 768 × 4 = 3072 bytes → 8 bytes = **384× compression**
//! - vs BQ (96 bytes): **12× smaller per vector** (but codebook adds 768 KB fixed cost)
//!
//! # References
//!
//! - Jegou, Douze, Schmid. "Product Quantization for Nearest Neighbor Search."
//!   IEEE TPAMI, 2011.
//! - `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md` — EdgeVec-specific analysis

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

/// Error type for Product Quantization operations.
///
/// Integrates with the unified `EdgeVecError` hierarchy via `From<PqError>`.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PqError {
    /// Dimensions must be evenly divisible by M (number of subquantizers).
    #[error(
        "dimensions ({dimensions}) must be divisible by num_subquantizers ({num_subquantizers})"
    )]
    DimensionNotDivisible {
        /// Vector dimensionality.
        dimensions: usize,
        /// Number of subquantizers.
        num_subquantizers: usize,
    },
    /// Training requires at least Ksub vectors (one per centroid).
    #[error("need at least {required} training vectors, got {provided}")]
    InsufficientTrainingData {
        /// Vectors provided.
        provided: usize,
        /// Minimum required (= Ksub).
        required: usize,
    },
    /// All training vectors must have the same dimensionality.
    #[error("vector {vector_index} has {actual} dims, expected {expected}")]
    InconsistentDimensions {
        /// Expected dimensionality (from first vector).
        expected: usize,
        /// Actual dimensionality of the offending vector.
        actual: usize,
        /// Index of the offending vector.
        vector_index: usize,
    },
    /// The vector to encode has wrong dimensionality.
    #[error("expected {expected} dims, got {actual}")]
    DimensionMismatch {
        /// Expected dimensionality (from codebook).
        expected: usize,
        /// Actual dimensionality.
        actual: usize,
    },
    /// M must be at least 1.
    #[error("M (num_subquantizers) must be >= 1")]
    InvalidM,
    /// Ksub must be at least 2 and at most 256.
    #[error("ksub must be in [2, 256], got {ksub}")]
    InvalidKsub {
        /// The invalid value.
        ksub: usize,
    },
    /// Training data contains non-finite values (NaN or Infinity).
    ///
    /// PQ k-means requires finite floating-point values. NaN propagates
    /// through centroid averages and corrupts the codebook. Infinity
    /// distorts distance computations.
    #[error("vector {vector_index} contains non-finite value at dimension {dimension}")]
    NonFiniteValue {
        /// Index of the offending vector.
        vector_index: usize,
        /// Dimension index of the non-finite value.
        dimension: usize,
    },
}

/// A PQ-encoded vector: M bytes, one centroid index per subspace.
///
/// # Size
///
/// `PqCode` wraps a `Vec<u8>` of length M. For M=8 (the default configuration),
/// each code is 8 bytes — a 384× compression from 768D f32 vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PqCode {
    /// Centroid indices, one per subspace. Length = M.
    codes: Vec<u8>,
}

impl PqCode {
    /// Returns the centroid indices as a slice.
    #[must_use]
    pub fn codes(&self) -> &[u8] {
        &self.codes
    }

    /// Returns the number of subspaces (M).
    #[must_use]
    pub fn num_subquantizers(&self) -> usize {
        self.codes.len()
    }
}

/// Result of a PQ scan: vector index and approximate distance.
///
/// Returned by [`DistanceTable::scan_topk`] in ascending distance order
/// (nearest first).
// Implements: D2T1 — Exhaustive PQ scan top-k
#[derive(Debug, Clone, PartialEq)]
pub struct PqSearchResult {
    /// Index of the vector in the `codes` slice passed to `scan_topk`.
    pub index: usize,
    /// Approximate squared L2 distance from the query.
    pub distance: f32,
}

/// Precomputed distance lookup table for Asymmetric Distance Computation (ADC).
///
/// For a given query vector, stores the L2 distance from each query subvector
/// to every centroid in the corresponding subspace. Shape: `[M][Ksub]`.
///
/// # Size
///
/// For M=8, Ksub=256: `8 * 256 * 4 = 8,192 bytes` (8 KB), fits in L1 cache.
#[derive(Debug, Clone)]
pub struct DistanceTable {
    /// Flat array of distances: `table[m * ksub + k]` = distance from query
    /// subvector m to centroid k in subspace m.
    table: Vec<f32>,
    /// Number of subquantizers (M).
    num_subquantizers: usize,
    /// Number of centroids per subspace (Ksub).
    ksub: usize,
}

impl DistanceTable {
    /// Compute the approximate L2 distance from the query to an encoded vector
    /// using M table lookups + additions (ADC).
    ///
    /// This is the hot inner loop during search. Each call performs exactly M
    /// memory loads and M additions.
    ///
    /// # Contract
    ///
    /// The `code` MUST have been produced by a `PqCodebook` with the same M
    /// as the codebook that produced this `DistanceTable`. Passing a code from
    /// a different codebook will panic.
    ///
    /// # Panics
    ///
    /// Panics if `code.num_subquantizers() != self.num_subquantizers`.
    ///
    /// # Returns
    ///
    /// Approximate squared L2 distance (sum of per-subspace distances).
    /// Always non-negative.
    #[must_use]
    #[inline]
    pub fn compute_distance(&self, code: &PqCode) -> f32 {
        assert_eq!(
            code.codes.len(),
            self.num_subquantizers,
            "PqCode has {} subquantizers but DistanceTable expects {}",
            code.codes.len(),
            self.num_subquantizers
        );
        let mut dist = 0.0f32;
        for (m, &centroid_idx) in code.codes.iter().enumerate() {
            // SAFETY: centroid_idx is u8 (0..255), ksub <= 256, so index is always valid.
            dist += self.table[m * self.ksub + centroid_idx as usize];
        }
        dist
    }

    /// Returns the number of subquantizers (M).
    #[must_use]
    pub fn num_subquantizers(&self) -> usize {
        self.num_subquantizers
    }

    /// Returns the number of centroids per subspace (Ksub).
    #[must_use]
    pub fn ksub(&self) -> usize {
        self.ksub
    }

    /// Exhaustive scan over PQ codes, returning the top-k nearest by ADC distance.
    ///
    /// Computes ADC distance for every code, then returns the k smallest.
    /// Results are sorted by distance ascending (nearest first).
    ///
    /// If `k > codes.len()`, returns all codes sorted by distance.
    /// If `codes` is empty, returns an empty vec.
    ///
    /// # Panics
    ///
    /// Panics if any code has a different number of subquantizers than this table.
    ///
    /// # NaN Handling
    ///
    /// If any ADC distance is NaN (should not occur with a validly trained codebook),
    /// NaN values sort to arbitrary positions in the result set. This is defined
    /// behavior but results are not meaningful — callers should validate codebook
    /// training data is free of NaN/Inf (enforced by `PqCodebook::train`).
    ///
    /// # Performance
    ///
    /// Time: O(n * M + n * log n) where n = codes.len(), M = num_subquantizers.
    /// Uses a simple sort; BinaryHeap optimization can reduce to O(n * M + n * log k)
    /// in a future iteration.
    // Implements: D2T1 — Exhaustive PQ scan top-k
    #[must_use]
    pub fn scan_topk(&self, codes: &[PqCode], k: usize) -> Vec<PqSearchResult> {
        if codes.is_empty() || k == 0 {
            return Vec::new();
        }

        // Compute all distances
        let mut results: Vec<PqSearchResult> = codes
            .iter()
            .enumerate()
            .map(|(i, code)| PqSearchResult {
                index: i,
                distance: self.compute_distance(code),
            })
            .collect();

        // Sort by distance ascending (nearest first).
        // Use `partial_cmp` with a fallback so NaN does not cause undefined order.
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to k
        results.truncate(k);
        results
    }
}

/// Trained PQ codebook: M subspace codebooks, each with Ksub centroids.
///
/// # Memory Layout
///
/// The codebook is stored as a flat `Vec<f32>` with shape `[M][Ksub][sub_dim]`:
/// - `codebook[m * ksub * sub_dim + k * sub_dim + d]` = dimension d of centroid k in subspace m.
///
/// # Size (D=768, M=8, Ksub=256)
///
/// - Codebook: `8 * 256 * 96 * 4 = 786,432 bytes` (768 KB)
/// - This is constant for any N (number of vectors) — amortized over the dataset.
///
/// # Thread Safety
///
/// `PqCodebook` is `Send + Sync` (contains only `Vec<f32>` and `usize` fields).
#[derive(Debug, Clone)]
pub struct PqCodebook {
    /// Flat codebook data: `[M][Ksub][sub_dim]`.
    centroids: Vec<f32>,
    /// Number of subquantizers (M).
    num_subquantizers: usize,
    /// Number of centroids per subspace.
    ksub: usize,
    /// Dimensionality of the original vectors.
    dimensions: usize,
    /// Dimensionality of each subspace (= dimensions / num_subquantizers).
    sub_dim: usize,
}

impl PqCodebook {
    /// Train a PQ codebook on a set of vectors using k-means clustering.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Training data: each inner slice is one D-dimensional vector.
    /// * `num_subquantizers` - M: number of subspaces. D must be divisible by M.
    /// * `ksub` - Number of centroids per subspace (typically 256). Must be in [2, 256].
    /// * `max_iters` - Maximum k-means iterations per subspace.
    ///
    /// # Determinism
    ///
    /// Training is deterministic for the same input (uses `ChaCha8Rng` with seed=42).
    /// The seed is fixed to ensure reproducible codebooks for benchmarking.
    ///
    /// # Returns
    ///
    /// A trained `PqCodebook` ready for encoding and distance computation.
    ///
    /// # Errors
    ///
    /// - `PqError::InvalidM` if M < 1
    /// - `PqError::InvalidKsub` if ksub < 2 or ksub > 256
    /// - `PqError::DimensionNotDivisible` if D is not divisible by M
    /// - `PqError::InsufficientTrainingData` if fewer than `ksub` vectors provided
    /// - `PqError::InconsistentDimensions` if vectors have different dimensions
    pub fn train(
        vectors: &[&[f32]],
        num_subquantizers: usize,
        ksub: usize,
        max_iters: usize,
    ) -> Result<Self, PqError> {
        // Validate parameters
        if num_subquantizers == 0 {
            return Err(PqError::InvalidM);
        }
        if !(2..=256).contains(&ksub) {
            return Err(PqError::InvalidKsub { ksub });
        }
        if vectors.len() < ksub {
            return Err(PqError::InsufficientTrainingData {
                provided: vectors.len(),
                required: ksub,
            });
        }

        let dimensions = vectors[0].len();
        if dimensions % num_subquantizers != 0 {
            return Err(PqError::DimensionNotDivisible {
                dimensions,
                num_subquantizers,
            });
        }

        // Validate all vectors have consistent dimensions and finite values
        for (i, v) in vectors.iter().enumerate() {
            if v.len() != dimensions {
                return Err(PqError::InconsistentDimensions {
                    expected: dimensions,
                    actual: v.len(),
                    vector_index: i,
                });
            }
            // Reject NaN/Infinity — these corrupt k-means centroids
            for (d, &val) in v.iter().enumerate() {
                if !val.is_finite() {
                    return Err(PqError::NonFiniteValue {
                        vector_index: i,
                        dimension: d,
                    });
                }
            }
        }

        let sub_dim = dimensions / num_subquantizers;
        let n = vectors.len();

        // Allocate codebook: [M][Ksub][sub_dim]
        let codebook_size = num_subquantizers * ksub * sub_dim;
        let mut centroids = vec![0.0f32; codebook_size];

        // Fixed seed for deterministic training
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        // Train each subspace independently
        for m in 0..num_subquantizers {
            let sub_offset = m * sub_dim;

            // Extract subvectors for this subspace
            let mut sub_vectors: Vec<Vec<f32>> = Vec::with_capacity(n);
            for v in vectors {
                sub_vectors.push(v[sub_offset..sub_offset + sub_dim].to_vec());
            }

            // Run k-means for this subspace
            let sub_centroids = kmeans(&sub_vectors, ksub, sub_dim, max_iters, &mut rng);

            // Copy trained centroids into the flat codebook
            let codebook_offset = m * ksub * sub_dim;
            for k in 0..ksub {
                let dst_start = codebook_offset + k * sub_dim;
                centroids[dst_start..dst_start + sub_dim]
                    .copy_from_slice(&sub_centroids[k * sub_dim..(k + 1) * sub_dim]);
            }
        }

        Ok(Self {
            centroids,
            num_subquantizers,
            ksub,
            dimensions,
            sub_dim,
        })
    }

    /// Encode a vector into a PQ code.
    ///
    /// For each subspace, finds the nearest centroid and stores its index.
    ///
    /// # Errors
    ///
    /// Returns `PqError::DimensionMismatch` if the vector has wrong dimensionality.
    pub fn encode(&self, vector: &[f32]) -> Result<PqCode, PqError> {
        if vector.len() != self.dimensions {
            return Err(PqError::DimensionMismatch {
                expected: self.dimensions,
                actual: vector.len(),
            });
        }

        let mut codes = Vec::with_capacity(self.num_subquantizers);

        for m in 0..self.num_subquantizers {
            let sub_offset = m * self.sub_dim;
            let sub_vector = &vector[sub_offset..sub_offset + self.sub_dim];
            let codebook_offset = m * self.ksub * self.sub_dim;

            let nearest = find_nearest_centroid(
                sub_vector,
                &self.centroids[codebook_offset..codebook_offset + self.ksub * self.sub_dim],
                self.ksub,
                self.sub_dim,
            );

            // nearest is always < ksub <= 256, so it fits in u8
            #[allow(clippy::cast_possible_truncation)]
            codes.push(nearest as u8);
        }

        Ok(PqCode { codes })
    }

    /// Encode a batch of vectors into PQ codes.
    ///
    /// Encodes each vector independently by delegating to [`encode`](Self::encode).
    /// Returns all codes if every vector succeeds, or the first error encountered.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Slice of vector slices, each D-dimensional.
    ///
    /// # Returns
    ///
    /// A `Vec<PqCode>` with one code per input vector, in the same order.
    ///
    /// # Errors
    ///
    /// Returns `PqError::DimensionMismatch` if any vector has wrong dimensionality.
    ///
    /// # Performance
    ///
    /// Time: O(n * M * Ksub * sub_dim) where n = vectors.len()
    /// Space: O(n * M) for the output codes
    // Implements: D2T2 -- Batch encode for vector collections
    pub fn encode_batch(&self, vectors: &[&[f32]]) -> Result<Vec<PqCode>, PqError> {
        vectors.iter().map(|v| self.encode(v)).collect()
    }

    /// Compute the ADC distance table for a query vector.
    ///
    /// For each subspace m and centroid k, precomputes the squared L2 distance
    /// from the query's subvector m to centroid k. This table enables O(M) distance
    /// computation to any encoded vector.
    ///
    /// # Errors
    ///
    /// Returns `PqError::DimensionMismatch` if the query has wrong dimensionality.
    pub fn compute_distance_table(&self, query: &[f32]) -> Result<DistanceTable, PqError> {
        if query.len() != self.dimensions {
            return Err(PqError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let table_size = self.num_subquantizers * self.ksub;
        let mut table = Vec::with_capacity(table_size);

        for m in 0..self.num_subquantizers {
            let query_offset = m * self.sub_dim;
            let query_sub = &query[query_offset..query_offset + self.sub_dim];
            let codebook_offset = m * self.ksub * self.sub_dim;

            for k in 0..self.ksub {
                let centroid_start = codebook_offset + k * self.sub_dim;
                let centroid = &self.centroids[centroid_start..centroid_start + self.sub_dim];

                let dist = l2_squared(query_sub, centroid);
                table.push(dist);
            }
        }

        Ok(DistanceTable {
            table,
            num_subquantizers: self.num_subquantizers,
            ksub: self.ksub,
        })
    }

    /// Returns the number of subquantizers (M).
    #[must_use]
    pub fn num_subquantizers(&self) -> usize {
        self.num_subquantizers
    }

    /// Returns the number of centroids per subspace (Ksub).
    #[must_use]
    pub fn ksub(&self) -> usize {
        self.ksub
    }

    /// Returns the original vector dimensionality.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Returns the sub-dimension (dimensions / M).
    #[must_use]
    pub fn sub_dim(&self) -> usize {
        self.sub_dim
    }

    /// Returns the codebook size in bytes.
    #[must_use]
    pub fn codebook_size_bytes(&self) -> usize {
        self.centroids.len() * std::mem::size_of::<f32>()
    }
}

// =============================================================================
// K-means clustering (internal)
// =============================================================================

/// Run k-means clustering on a set of sub-dimensional vectors.
///
/// Returns a flat array of centroids: `[ksub][sub_dim]`.
///
/// Uses k-means++ initialization for better convergence.
fn kmeans(
    vectors: &[Vec<f32>],
    ksub: usize,
    sub_dim: usize,
    max_iters: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<f32> {
    let n = vectors.len();

    // k-means++ initialization
    let mut centroids = kmeans_plus_plus_init(vectors, ksub, sub_dim, rng);

    // Assignment buffer
    let mut assignments = vec![0usize; n];

    for _iter in 0..max_iters {
        // Assignment step: assign each vector to nearest centroid
        let mut changed = false;
        for (i, v) in vectors.iter().enumerate() {
            let nearest = find_nearest_centroid(v, &centroids, ksub, sub_dim);
            if assignments[i] != nearest {
                assignments[i] = nearest;
                changed = true;
            }
        }

        // Early stopping if no assignments changed
        if !changed {
            break;
        }

        // Update step: recompute centroids as mean of assigned vectors
        let mut new_centroids = vec![0.0f32; ksub * sub_dim];
        let mut counts = vec![0usize; ksub];

        for (i, v) in vectors.iter().enumerate() {
            let k = assignments[i];
            counts[k] += 1;
            let offset = k * sub_dim;
            for (d, &val) in v.iter().enumerate() {
                new_centroids[offset + d] += val;
            }
        }

        // Divide by count to get mean; handle empty clusters
        for (k, &count) in counts.iter().enumerate().take(ksub) {
            let offset = k * sub_dim;
            if count > 0 {
                #[allow(clippy::cast_precision_loss)]
                let count_f = count as f32;
                for d in 0..sub_dim {
                    new_centroids[offset + d] /= count_f;
                }
            } else {
                // Empty cluster: reinitialize to a random training vector.
                // Alternative strategy: pick the vector farthest from its current
                // centroid (better convergence but O(n) scan per empty cluster).
                // Random reinitialization is simpler and sufficient for typical data.
                let random_idx = rng.gen_range(0..n);
                new_centroids[offset..offset + sub_dim].copy_from_slice(&vectors[random_idx]);
            }
        }

        centroids = new_centroids;
    }

    centroids
}

/// K-means++ initialization: select initial centroids with probability
/// proportional to squared distance from nearest existing centroid.
///
/// This produces better initial centroids than random selection and
/// typically reduces the number of iterations needed for convergence.
fn kmeans_plus_plus_init(
    vectors: &[Vec<f32>],
    ksub: usize,
    sub_dim: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<f32> {
    let n = vectors.len();
    let mut centroids = vec![0.0f32; ksub * sub_dim];

    // First centroid: random
    let first_idx = rng.gen_range(0..n);
    centroids[..sub_dim].copy_from_slice(&vectors[first_idx]);

    // Distance from each point to nearest centroid so far
    let mut min_dists = vec![f32::MAX; n];

    for k in 1..ksub {
        // Update min distances with the newly added centroid (k-1)
        let prev_offset = (k - 1) * sub_dim;
        let prev_centroid = &centroids[prev_offset..prev_offset + sub_dim];

        let mut total_dist = 0.0f64; // Use f64 to avoid precision loss in sum
        for (i, v) in vectors.iter().enumerate() {
            let d = l2_squared(v, prev_centroid);
            if d < min_dists[i] {
                min_dists[i] = d;
            }
            total_dist += f64::from(min_dists[i]);
        }

        // Sample next centroid with probability proportional to min_dists
        if total_dist <= 0.0 {
            // All points are at distance 0 — pick randomly
            let idx = rng.gen_range(0..n);
            let offset = k * sub_dim;
            centroids[offset..offset + sub_dim].copy_from_slice(&vectors[idx]);
        } else {
            let threshold = rng.gen_range(0.0..total_dist);
            let mut cumulative = 0.0f64;
            let mut chosen = n - 1; // Fallback to last
            for (i, &d) in min_dists.iter().enumerate() {
                cumulative += f64::from(d);
                if cumulative >= threshold {
                    chosen = i;
                    break;
                }
            }
            let offset = k * sub_dim;
            centroids[offset..offset + sub_dim].copy_from_slice(&vectors[chosen]);
        }
    }

    centroids
}

/// Find the index of the nearest centroid to `vector` by squared L2 distance.
///
/// `centroids` is a flat array of shape `[ksub][sub_dim]`.
#[inline]
fn find_nearest_centroid(vector: &[f32], centroids: &[f32], ksub: usize, sub_dim: usize) -> usize {
    let mut best_idx = 0;
    let mut best_dist = f32::MAX;

    for k in 0..ksub {
        let offset = k * sub_dim;
        let centroid = &centroids[offset..offset + sub_dim];
        let dist = l2_squared(vector, centroid);
        if dist < best_dist {
            best_dist = dist;
            best_idx = k;
        }
    }

    best_idx
}

/// Squared L2 (Euclidean) distance between two vectors.
#[inline]
fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a deterministic dataset for testing.
    fn generate_test_data(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        (0..count)
            .map(|_| (0..dims).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect())
            .collect()
    }

    // =========================================================================
    // D1T1: Module scaffold / type tests
    // =========================================================================

    #[test]
    fn test_pq_code_structure() {
        let code = PqCode {
            codes: vec![0, 1, 2, 3, 4, 5, 6, 7],
        };
        assert_eq!(code.num_subquantizers(), 8);
        assert_eq!(code.codes(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_pq_error_display() {
        let err = PqError::DimensionNotDivisible {
            dimensions: 768,
            num_subquantizers: 7,
        };
        assert!(err.to_string().contains("768"));
        assert!(err.to_string().contains("7"));
    }

    // =========================================================================
    // D1T2: K-means convergence tests
    // =========================================================================

    #[test]
    fn test_kmeans_convergence() {
        // 256 centroids on 1K vectors (96-dim subspace) should converge
        let data = generate_test_data(1000, 96, 42);
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let centroids = kmeans(&data, 256, 96, 20, &mut rng);

        // Should have exactly 256 * 96 floats
        assert_eq!(centroids.len(), 256 * 96);

        // All centroids should be finite
        assert!(centroids.iter().all(|&v| v.is_finite()));
    }

    #[test]
    fn test_kmeans_small_dataset() {
        // Minimal: 4 vectors, 2 centroids, 2D
        let data = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![10.0, 10.0],
            vec![10.0, 11.0],
        ];
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let centroids = kmeans(&data, 2, 2, 50, &mut rng);

        assert_eq!(centroids.len(), 4); // 2 centroids * 2 dims

        // Centroids should be near (0, 0.5) and (10, 10.5) approximately
        let c0 = &centroids[0..2];
        let c1 = &centroids[2..4];

        // One centroid should be near origin, the other near (10, 10)
        let near_origin =
            (c0[0].abs() < 2.0 && c0[1].abs() < 2.0) || (c1[0].abs() < 2.0 && c1[1].abs() < 2.0);
        let near_ten = (c0[0] > 8.0 && c0[1] > 8.0) || (c1[0] > 8.0 && c1[1] > 8.0);
        assert!(near_origin, "one centroid should be near origin");
        assert!(near_ten, "one centroid should be near (10, 10)");
    }

    #[test]
    fn test_kmeans_plus_plus_deterministic() {
        let data = generate_test_data(100, 16, 42);
        let mut rng1 = ChaCha8Rng::seed_from_u64(42);
        let mut rng2 = ChaCha8Rng::seed_from_u64(42);

        let c1 = kmeans_plus_plus_init(&data, 8, 16, &mut rng1);
        let c2 = kmeans_plus_plus_init(&data, 8, 16, &mut rng2);

        assert_eq!(c1, c2, "same seed must produce identical initialization");
    }

    // =========================================================================
    // D1T3: PqCodebook::train() tests
    // =========================================================================

    #[test]
    fn test_codebook_train_deterministic() {
        let data = generate_test_data(500, 768, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        let cb1 = PqCodebook::train(&refs, 8, 256, 15).expect("train should succeed");
        let cb2 = PqCodebook::train(&refs, 8, 256, 15).expect("train should succeed");

        assert_eq!(
            cb1.centroids, cb2.centroids,
            "same input + seed=42 must produce identical codebook"
        );
    }

    #[test]
    fn test_codebook_train_validation_errors() {
        let data = generate_test_data(300, 768, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        // M = 0
        assert!(matches!(
            PqCodebook::train(&refs, 0, 256, 15),
            Err(PqError::InvalidM)
        ));

        // Ksub = 0
        assert!(matches!(
            PqCodebook::train(&refs, 8, 0, 15),
            Err(PqError::InvalidKsub { ksub: 0 })
        ));

        // Ksub = 257
        assert!(matches!(
            PqCodebook::train(&refs, 8, 257, 15),
            Err(PqError::InvalidKsub { ksub: 257 })
        ));

        // D not divisible by M
        let bad_data = generate_test_data(300, 100, 42);
        let bad_refs: Vec<&[f32]> = bad_data.iter().map(|v| v.as_slice()).collect();
        assert!(matches!(
            PqCodebook::train(&bad_refs, 7, 16, 15),
            Err(PqError::DimensionNotDivisible { .. })
        ));

        // Too few vectors
        let small = generate_test_data(10, 768, 42);
        let small_refs: Vec<&[f32]> = small.iter().map(|v| v.as_slice()).collect();
        assert!(matches!(
            PqCodebook::train(&small_refs, 8, 256, 15),
            Err(PqError::InsufficientTrainingData { .. })
        ));
    }

    #[test]
    fn test_codebook_inconsistent_dimensions() {
        let v1 = vec![0.0f32; 768];
        let v2 = vec![0.0f32; 512];
        let refs: Vec<&[f32]> = vec![v1.as_slice(), v2.as_slice()];

        let result = PqCodebook::train(&refs, 8, 2, 5);
        assert!(matches!(
            result,
            Err(PqError::InconsistentDimensions {
                expected: 768,
                actual: 512,
                vector_index: 1,
            })
        ));
    }

    // =========================================================================
    // D1T4: Codebook shape verification
    // =========================================================================

    #[test]
    fn test_codebook_shape() {
        let data = generate_test_data(1000, 768, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        let cb = PqCodebook::train(&refs, 8, 256, 15).expect("train should succeed");

        assert_eq!(cb.num_subquantizers(), 8, "M should be 8");
        assert_eq!(cb.ksub(), 256, "Ksub should be 256");
        assert_eq!(cb.dimensions(), 768, "D should be 768");
        assert_eq!(cb.sub_dim(), 96, "D/M should be 96");
        assert_eq!(
            cb.codebook_size_bytes(),
            786_432,
            "codebook should be 768KB (8 * 256 * 96 * 4)"
        );

        // All centroids should be finite (no NaN/Inf from k-means)
        assert!(
            cb.centroids.iter().all(|&v| v.is_finite()),
            "all centroids must be finite"
        );
    }

    #[test]
    fn test_codebook_small_config() {
        // M=4, Ksub=16, D=32 — small config for fast testing
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train should succeed");

        assert_eq!(cb.num_subquantizers(), 4);
        assert_eq!(cb.ksub(), 16);
        assert_eq!(cb.sub_dim(), 8);
        assert_eq!(cb.codebook_size_bytes(), 4 * 16 * 8 * 4);
    }

    // =========================================================================
    // Encode + distance table tests (Day 2 scope, basic smoke tests here)
    // =========================================================================

    #[test]
    fn test_encode_produces_correct_length() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let code = cb.encode(&data[0]).expect("encode");
        assert_eq!(code.num_subquantizers(), 4);
        // All codes should be < ksub
        assert!(code.codes().iter().all(|&c| (c as usize) < 16));
    }

    #[test]
    fn test_encode_dimension_mismatch() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let bad = vec![0.0f32; 64];
        assert!(matches!(
            cb.encode(&bad),
            Err(PqError::DimensionMismatch {
                expected: 32,
                actual: 64,
            })
        ));
    }

    #[test]
    fn test_distance_table_shape() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let dt = cb.compute_distance_table(&data[0]).expect("dist table");
        assert_eq!(dt.num_subquantizers(), 4);
        assert_eq!(dt.ksub(), 16);
    }

    #[test]
    fn test_adc_distance_nonnegative() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let dt = cb.compute_distance_table(&data[0]).expect("dist table");
        for v in &data {
            let code = cb.encode(v).expect("encode");
            let dist = dt.compute_distance(&code);
            assert!(dist >= 0.0, "ADC distance must be non-negative, got {dist}");
            assert!(dist.is_finite(), "ADC distance must be finite");
        }
    }

    #[test]
    fn test_adc_self_distance_near_zero() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        // ADC(v, encode(v)) should be small (not exactly 0 due to quantization)
        let v = &data[0];
        let code = cb.encode(v).expect("encode");
        let dt = cb.compute_distance_table(v).expect("dist table");
        let self_dist = dt.compute_distance(&code);

        // The self-distance is the quantization error. For a well-trained codebook
        // on 100 vectors with 16 centroids, this should be small.
        assert!(
            self_dist < 10.0,
            "self-distance should be small (quantization error), got {self_dist}"
        );
    }

    #[test]
    fn test_train_rejects_nan() {
        let mut data = generate_test_data(50, 32, 42);
        data[10][5] = f32::NAN;
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        assert!(matches!(
            PqCodebook::train(&refs, 4, 16, 10),
            Err(PqError::NonFiniteValue {
                vector_index: 10,
                ..
            })
        ));
    }

    #[test]
    fn test_train_rejects_infinity() {
        let mut data = generate_test_data(50, 32, 42);
        data[3][0] = f32::INFINITY;
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        assert!(matches!(
            PqCodebook::train(&refs, 4, 16, 10),
            Err(PqError::NonFiniteValue {
                vector_index: 3,
                ..
            })
        ));
    }

    #[test]
    fn test_train_rejects_neg_infinity() {
        let mut data = generate_test_data(50, 32, 42);
        data[0][31] = f32::NEG_INFINITY;
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        assert!(matches!(
            PqCodebook::train(&refs, 4, 16, 10),
            Err(PqError::NonFiniteValue {
                vector_index: 0,
                ..
            })
        ));
    }

    #[test]
    fn test_kmeans_handles_empty_clusters() {
        // Create data with 2 tight clusters but request 16 centroids.
        // This forces many centroids to become empty during k-means,
        // exercising the empty-cluster reinitialization path.
        let mut data = Vec::new();
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        // Cluster A near origin
        for _ in 0..50 {
            let v: Vec<f32> = (0..8).map(|_| rng.gen::<f32>() * 0.01).collect();
            data.push(v);
        }
        // Cluster B far away
        for _ in 0..50 {
            let v: Vec<f32> = (0..8).map(|_| 100.0 + rng.gen::<f32>() * 0.01).collect();
            data.push(v);
        }
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        // 16 centroids for 2 natural clusters — many will start empty
        let result = PqCodebook::train(&refs, 1, 16, 50);
        assert!(
            result.is_ok(),
            "training must succeed even with empty clusters"
        );

        let cb = result.unwrap();
        // Verify all vectors can be encoded
        for v in &data {
            let code = cb.encode(v).expect("encode must work");
            assert_eq!(code.num_subquantizers(), 1);
        }
    }

    // =========================================================================
    // D2T2: encode_batch() tests
    // =========================================================================

    /// Verifies: encode_batch returns Ok with correct count and code length.
    #[test]
    fn test_encode_batch_basic() {
        // Train on 100 vectors, 32D, M=4, Ksub=16
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train should succeed");

        // Batch-encode all 100 training vectors
        let codes = cb.encode_batch(&refs).expect("encode_batch should succeed");

        // Must return exactly 100 PqCodes
        assert_eq!(codes.len(), 100, "should produce one code per input vector");

        // Each code must have M=4 bytes
        for (i, code) in codes.iter().enumerate() {
            assert_eq!(
                code.num_subquantizers(),
                4,
                "code {i} should have M=4 subquantizers"
            );
            // All centroid indices must be < Ksub=16
            assert!(
                code.codes().iter().all(|&c| (c as usize) < 16),
                "code {i} has centroid index >= Ksub"
            );
        }
    }

    /// Verifies: encode_batch returns DimensionMismatch on mixed dimensions.
    #[test]
    fn test_encode_batch_dimension_mismatch() {
        // Train on 32D vectors
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train should succeed");

        // Create a batch with one 64D vector mixed in
        let good_vec = vec![0.0f32; 32];
        let bad_vec = vec![0.0f32; 64];
        let batch: Vec<&[f32]> = vec![good_vec.as_slice(), bad_vec.as_slice()];

        let result = cb.encode_batch(&batch);
        assert!(
            matches!(
                result,
                Err(PqError::DimensionMismatch {
                    expected: 32,
                    actual: 64,
                })
            ),
            "should fail with DimensionMismatch for the 64D vector, got: {result:?}"
        );
    }

    /// Verifies: encode_batch on empty input returns Ok(vec![]).
    #[test]
    fn test_encode_batch_empty() {
        let data = generate_test_data(100, 32, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train should succeed");

        let empty: Vec<&[f32]> = vec![];
        let codes = cb.encode_batch(&empty).expect("empty batch should succeed");
        assert!(codes.is_empty(), "empty input should produce empty output");
    }

    // =========================================================================
    // D2T1: scan_topk tests
    // =========================================================================

    /// Train on 100 vectors (32D, M=4, Ksub=16), encode all, scan for top-5.
    /// Verify: returns exactly 5 results, sorted ascending by distance,
    /// all indices in range 0..100.
    #[test]
    fn test_pq_scan_topk_basic() {
        let data = generate_test_data(100, 32, 99);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let codes: Vec<PqCode> = data.iter().map(|v| cb.encode(v).expect("encode")).collect();

        let query = &data[0];
        let dt = cb.compute_distance_table(query).expect("dist table");
        let results = dt.scan_topk(&codes, 5);

        // Exactly 5 results
        assert_eq!(results.len(), 5, "should return exactly k=5 results");

        // Sorted ascending by distance
        for w in results.windows(2) {
            assert!(
                w[0].distance <= w[1].distance,
                "results must be sorted ascending: {} <= {}",
                w[0].distance,
                w[1].distance
            );
        }

        // All indices in range
        for r in &results {
            assert!(r.index < 100, "index {} out of range 0..100", r.index);
        }

        // All distances non-negative and finite
        for r in &results {
            assert!(r.distance >= 0.0, "distance must be non-negative");
            assert!(r.distance.is_finite(), "distance must be finite");
        }
    }

    /// k > n: 10 codes, k=20. Should return all 10 sorted by distance.
    #[test]
    fn test_pq_scan_topk_k_greater_than_n() {
        let data = generate_test_data(20, 32, 77);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        // Only encode 10 of the 20 vectors
        let codes: Vec<PqCode> = data[..10]
            .iter()
            .map(|v| cb.encode(v).expect("encode"))
            .collect();

        let dt = cb.compute_distance_table(&data[0]).expect("dist table");
        let results = dt.scan_topk(&codes, 20);

        assert_eq!(results.len(), 10, "k=20 > n=10, should return all 10");

        // Still sorted ascending
        for w in results.windows(2) {
            assert!(
                w[0].distance <= w[1].distance,
                "results must be sorted ascending"
            );
        }
    }

    /// k=0 should return empty vec regardless of codes.
    #[test]
    fn test_pq_scan_topk_k_zero() {
        let data = generate_test_data(20, 32, 55);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let codes: Vec<PqCode> = data.iter().map(|v| cb.encode(v).expect("encode")).collect();
        let dt = cb.compute_distance_table(&data[0]).expect("dist table");
        let results = dt.scan_topk(&codes, 0);

        assert!(results.is_empty(), "k=0 should return empty results");
    }

    /// Empty codes slice, k=5. Should return empty vec.
    #[test]
    fn test_pq_scan_topk_empty() {
        let data = generate_test_data(20, 32, 55);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let dt = cb.compute_distance_table(&data[0]).expect("dist table");
        let results = dt.scan_topk(&[], 5);

        assert!(
            results.is_empty(),
            "empty codes should return empty results"
        );
    }

    // =========================================================================
    // D2T4: Integration test — full pipeline
    // =========================================================================

    /// Full pipeline: train on 1K vectors, encode all, search 5 queries, verify top-10.
    #[test]
    fn test_pq_integration_pipeline() {
        // 1. Generate 1K vectors (64D) with 3 distinct seeds for variety
        let data = generate_test_data(1000, 64, 42);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();

        // 2. Train codebook: M=8 subspaces, 32 centroids each, 20 iterations
        let cb = PqCodebook::train(&refs, 8, 32, 20).expect("training should succeed");
        assert_eq!(cb.num_subquantizers(), 8);
        assert_eq!(cb.ksub(), 32);
        assert_eq!(cb.dimensions(), 64);
        assert_eq!(cb.sub_dim(), 8);

        // 3. Encode all 1K vectors via batch
        let codes = cb.encode_batch(&refs).expect("batch encode should succeed");
        assert_eq!(codes.len(), 1000);

        // 4. Search 5 queries — each query is a different training vector
        let query_indices = [0, 100, 500, 750, 999];
        for &qi in &query_indices {
            let dt = cb
                .compute_distance_table(&data[qi])
                .expect("distance table");
            let results = dt.scan_topk(&codes, 10);

            // Must return exactly 10
            assert_eq!(results.len(), 10, "query {qi} should return top-10");

            // All indices valid
            for r in &results {
                assert!(r.index < 1000, "query {qi}: index {} out of range", r.index);
                assert!(r.distance >= 0.0, "query {qi}: negative distance");
                assert!(r.distance.is_finite(), "query {qi}: non-finite distance");
            }

            // Sorted ascending
            for w in results.windows(2) {
                assert!(
                    w[0].distance <= w[1].distance,
                    "query {qi}: results not sorted"
                );
            }

            // Self should be in top-10 (the query vector's own encoding
            // should have very small distance)
            assert!(
                results.iter().any(|r| r.index == qi),
                "query {qi}: self should appear in top-10, got indices {:?}",
                results.iter().map(|r| r.index).collect::<Vec<_>>()
            );
        }
    }

    /// Encode vector v, scan for top-1 using v as query. The result should be
    /// the index of v (self-distance is smallest).
    #[test]
    fn test_pq_scan_self_is_nearest() {
        let data = generate_test_data(100, 32, 123);
        let refs: Vec<&[f32]> = data.iter().map(|v| v.as_slice()).collect();
        let cb = PqCodebook::train(&refs, 4, 16, 10).expect("train");

        let codes: Vec<PqCode> = data.iter().map(|v| cb.encode(v).expect("encode")).collect();

        // Pick vector 42 as query
        let query_idx = 42;
        let dt = cb
            .compute_distance_table(&data[query_idx])
            .expect("dist table");
        let results = dt.scan_topk(&codes, 1);

        assert_eq!(results.len(), 1, "top-1 should return exactly 1 result");
        assert_eq!(
            results[0].index, query_idx,
            "self-encoded vector should be its own nearest neighbor, \
             got index {} with distance {}, expected index {}",
            results[0].index, results[0].distance, query_idx
        );
    }
}
