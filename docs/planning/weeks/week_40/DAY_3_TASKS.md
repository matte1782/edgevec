# Week 40 Day 3: Optimization & Quantization

**Date:** 2026-02-05
**Focus:** Performance tuning, deletion, and optional BQ quantization
**Estimated Duration:** 5 hours
**Phase:** RFC-008 Phase 3 (Optimization)
**Dependencies:** Day 2 COMPLETE (Search implementation)

---

## Context

Day 3 optimizes FlatIndex for production use:
1. Search performance (<50ms for 10k vectors)
2. Soft deletion with bitmap cleanup
3. Optional binary quantization (32x memory reduction)

**Performance Target:**
- Search (10k, k=10): <50ms (target), <100ms (acceptable)
- Insert: <100us
- Memory: ~30MB for 10k 768D vectors

---

## Tasks

### W40.3.1: Optimize Search Inner Loop

**Objective:** Profile and optimize the search hot path.

**File:** `src/index/flat.rs`

```rust
impl FlatIndex {
    /// Optimized search with prefetching and loop unrolling.
    pub fn search_optimized(&self, query: &[f32], k: usize) -> Result<Vec<FlatSearchResult>, IndexError> {
        // ... validation ...

        let dim = self.config.dimensions as usize;
        let is_similarity = matches!(self.config.metric, Metric::Cosine | Metric::DotProduct);

        // Pre-compute query norm for Cosine
        let query_norm = if matches!(self.config.metric, Metric::Cosine) {
            query.iter().map(|x| x * x).sum::<f32>().sqrt()
        } else {
            1.0
        };

        let mut heap: BinaryHeap<MaxScoreResult> = BinaryHeap::with_capacity(k + 1);

        // Process in chunks for cache efficiency
        const CHUNK_SIZE: usize = 64;
        let mut chunk_scores = Vec::with_capacity(CHUNK_SIZE);

        for chunk_start in (0..self.count as usize).step_by(CHUNK_SIZE) {
            let chunk_end = (chunk_start + CHUNK_SIZE).min(self.count as usize);
            chunk_scores.clear();

            // Batch distance computation
            for idx in chunk_start..chunk_end {
                if self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
                    chunk_scores.push((idx, f32::INFINITY));
                    continue;
                }

                let start = idx * dim;
                let vector = &self.vectors[start..start + dim];
                let score = self.compute_distance_fast(query, vector, query_norm);
                chunk_scores.push((idx, score));
            }

            // Update heap
            for (idx, score) in chunk_scores.iter() {
                if *score == f32::INFINITY {
                    continue;
                }

                let heap_score = if is_similarity { -*score } else { *score };

                if heap.len() < k {
                    heap.push(MaxScoreResult(FlatSearchResult {
                        id: *idx as u64,
                        score: heap_score,
                    }));
                } else if let Some(top) = heap.peek() {
                    if heap_score < top.0.score {
                        heap.pop();
                        heap.push(MaxScoreResult(FlatSearchResult {
                            id: *idx as u64,
                            score: heap_score,
                        }));
                    }
                }
            }
        }

        // Extract and sort results
        let mut results: Vec<FlatSearchResult> = heap
            .into_iter()
            .map(|r| FlatSearchResult {
                id: r.0.id,
                score: if is_similarity { -r.0.score } else { r.0.score },
            })
            .collect();

        if is_similarity {
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        } else {
            results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
        }

        Ok(results)
    }

    /// Fast distance computation with pre-computed query norm.
    #[inline]
    fn compute_distance_fast(&self, query: &[f32], vector: &[f32], query_norm: f32) -> f32 {
        match self.config.metric {
            Metric::Cosine => {
                let dot: f32 = query.iter().zip(vector.iter()).map(|(a, b)| a * b).sum();
                let vec_norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
                if vec_norm == 0.0 {
                    0.0
                } else {
                    dot / (query_norm * vec_norm)
                }
            }
            Metric::DotProduct => {
                query.iter().zip(vector.iter()).map(|(a, b)| a * b).sum()
            }
            Metric::L2 => {
                query.iter()
                    .zip(vector.iter())
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f32>()
                    .sqrt()
            }
            Metric::Hamming => {
                query.iter()
                    .zip(vector.iter())
                    .filter(|(a, b)| (**a != 0.0) != (**b != 0.0))
                    .count() as f32
            }
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Chunked processing implemented
- [ ] Query norm pre-computed for Cosine
- [ ] Inline annotations on hot functions
- [ ] Benchmark shows improvement or no regression
- [ ] Same correctness as original `search()`

**Deliverables:**
- Optimized `search()` method

**Dependencies:** W40.2.1

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.3.2: Implement Soft Delete

**Objective:** Add deletion with lazy cleanup.

**File:** `src/index/flat.rs`

```rust
impl FlatIndex {
    /// Mark a vector as deleted.
    ///
    /// The vector is not immediately removed; its slot is marked in the
    /// deletion bitmap and skipped during search. Call `compact()` to
    /// reclaim space when deletion rate exceeds the threshold.
    ///
    /// # Returns
    ///
    /// Returns `true` if the vector was deleted, `false` if it didn't exist
    /// or was already deleted.
    pub fn delete(&mut self, id: u64) -> bool {
        let idx = id as usize;

        // Check bounds
        if idx >= self.count as usize {
            return false;
        }

        // Check if already deleted
        if self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
            return false;
        }

        // Mark as deleted
        self.deleted.set(idx, true);
        self.delete_count += 1;

        // Auto-compact if threshold exceeded
        if self.should_compact() {
            self.compact();
        }

        true
    }

    /// Check if compaction is needed.
    fn should_compact(&self) -> bool {
        if self.count == 0 {
            return false;
        }
        (self.delete_count as f32 / self.count as f32) > self.config.cleanup_threshold
    }

    /// Compact the index by removing deleted vectors.
    ///
    /// This rebuilds the internal storage, reassigning IDs to be contiguous.
    /// **Warning:** This changes vector IDs!
    pub fn compact(&mut self) {
        if self.delete_count == 0 {
            return;
        }

        let dim = self.config.dimensions as usize;
        let new_count = self.count as usize - self.delete_count;

        let mut new_vectors = Vec::with_capacity(new_count * dim);
        let mut new_deleted = BitVec::with_capacity(new_count);

        for idx in 0..self.count as usize {
            if !self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
                // Copy live vector
                let start = idx * dim;
                new_vectors.extend_from_slice(&self.vectors[start..start + dim]);
                new_deleted.push(false);
            }
        }

        self.vectors = new_vectors;
        self.deleted = new_deleted;
        self.count = new_count as u64;
        self.delete_count = 0;
        // Note: next_id stays the same to avoid ID reuse
    }

    /// Get deletion statistics.
    pub fn deletion_stats(&self) -> (usize, usize, f32) {
        let total = self.count as usize;
        let deleted = self.delete_count;
        let ratio = if total > 0 {
            deleted as f32 / total as f32
        } else {
            0.0
        };
        (total, deleted, ratio)
    }
}
```

**Acceptance Criteria:**
- [ ] `delete()` marks vectors as deleted
- [ ] `delete()` returns false for nonexistent IDs
- [ ] Search skips deleted vectors
- [ ] Auto-compact when threshold exceeded
- [ ] `compact()` rebuilds storage correctly
- [ ] `deletion_stats()` returns accurate info

**Deliverables:**
- `delete()` method
- `compact()` method
- `deletion_stats()` method

**Dependencies:** W40.1.2

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.3.3: Optional BQ Quantization

**Objective:** Add optional binary quantization for memory reduction.

**File:** `src/index/flat.rs`

```rust
impl FlatIndex {
    /// Enable binary quantization for memory reduction.
    ///
    /// Converts stored vectors to binary format (32x compression).
    /// Search uses Hamming distance on quantized vectors.
    ///
    /// # Warning
    ///
    /// This is a lossy operation. Recall will decrease from 100%
    /// but memory usage drops significantly.
    pub fn enable_quantization(&mut self) -> Result<(), IndexError> {
        if self.quantized.is_some() {
            return Ok(()); // Already enabled
        }

        let dim = self.config.dimensions as usize;
        let packed_dim = (dim + 7) / 8; // Bytes needed for dim bits

        let mut quantized = Vec::with_capacity(self.count as usize * packed_dim);

        for idx in 0..self.count as usize {
            if self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
                // Placeholder for deleted vectors
                quantized.extend(vec![0u8; packed_dim]);
                continue;
            }

            let start = idx * dim;
            let vector = &self.vectors[start..start + dim];

            // Binarize: value > 0 = 1, else 0
            let packed = self.binarize_vector(vector);
            quantized.extend_from_slice(&packed);
        }

        self.quantized = Some(quantized);
        Ok(())
    }

    /// Disable quantization, revert to F32 search.
    pub fn disable_quantization(&mut self) {
        self.quantized = None;
    }

    /// Check if quantization is enabled.
    pub fn is_quantized(&self) -> bool {
        self.quantized.is_some()
    }

    /// Binarize a vector to packed bytes.
    fn binarize_vector(&self, vector: &[f32]) -> Vec<u8> {
        let dim = vector.len();
        let packed_dim = (dim + 7) / 8;
        let mut packed = vec![0u8; packed_dim];

        for (i, &val) in vector.iter().enumerate() {
            if val > 0.0 {
                packed[i / 8] |= 1 << (7 - (i % 8));
            }
        }

        packed
    }

    /// Compute Hamming distance between binary vectors.
    fn hamming_distance_binary(&self, a: &[u8], b: &[u8]) -> u32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum()
    }

    /// Search using quantized vectors (if enabled).
    pub fn search_quantized(&self, query: &[f32], k: usize) -> Result<Vec<FlatSearchResult>, IndexError> {
        let quantized = self.quantized.as_ref().ok_or(IndexError::QuantizationNotEnabled)?;

        // Validate
        if query.len() != self.config.dimensions as usize {
            return Err(IndexError::DimensionMismatch {
                expected: self.config.dimensions as usize,
                actual: query.len(),
            });
        }

        if k == 0 {
            return Err(IndexError::InvalidK);
        }

        // Binarize query
        let query_packed = self.binarize_vector(query);
        let packed_dim = query_packed.len();

        let mut heap: BinaryHeap<MaxScoreResult> = BinaryHeap::with_capacity(k + 1);

        for idx in 0..self.count as usize {
            if self.deleted.get(idx).map(|b| *b).unwrap_or(true) {
                continue;
            }

            let start = idx * packed_dim;
            let vector_packed = &quantized[start..start + packed_dim];

            let distance = self.hamming_distance_binary(&query_packed, vector_packed);

            if heap.len() < k {
                heap.push(MaxScoreResult(FlatSearchResult {
                    id: idx as u64,
                    score: distance as f32,
                }));
            } else if let Some(top) = heap.peek() {
                if (distance as f32) < top.0.score {
                    heap.pop();
                    heap.push(MaxScoreResult(FlatSearchResult {
                        id: idx as u64,
                        score: distance as f32,
                    }));
                }
            }
        }

        // Extract and sort (ascending distance)
        let mut results: Vec<FlatSearchResult> = heap
            .into_iter()
            .map(|r| r.0)
            .collect();

        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));

        Ok(results)
    }
}
```

**Acceptance Criteria:**
- [ ] `enable_quantization()` converts vectors to binary
- [ ] `disable_quantization()` reverts to F32
- [ ] `is_quantized()` returns correct state
- [ ] `search_quantized()` uses Hamming distance
- [ ] Memory reduction ~32x verified
- [ ] Recall degradation documented

**Deliverables:**
- Quantization methods
- `search_quantized()` method

**Dependencies:** W40.3.1

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.3.4: Benchmarks

**Objective:** Create benchmark suite to measure performance.

**File:** `benches/flat_bench.rs`

```rust
//! Benchmarks for FlatIndex performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edgevec::index::{FlatIndex, FlatIndexConfig};
use edgevec::metric::Metric;

fn generate_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut s = seed;
    let lcg = |s: &mut u64| -> f32 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*s >> 33) as f32) / (u32::MAX as f32)
    };

    (0..count)
        .map(|_| (0..dim).map(|_| lcg(&mut s)).collect())
        .collect()
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_insert");

    for count in [100, 1000, 10000] {
        let vectors = generate_vectors(count, 128, 42);

        group.bench_with_input(
            BenchmarkId::new("single", count),
            &vectors,
            |b, vecs| {
                b.iter(|| {
                    let mut index = FlatIndex::new(FlatIndexConfig::new(128));
                    for v in vecs {
                        black_box(index.insert(v).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_search");
    group.sample_size(50);

    for count in [1000, 10000] {
        let vectors = generate_vectors(count, 128, 42);
        let mut index = FlatIndex::new(FlatIndexConfig::new(128));
        for v in &vectors {
            index.insert(v).unwrap();
        }

        let query = generate_vectors(1, 128, 999)[0].clone();

        group.bench_with_input(
            BenchmarkId::new("k10", count),
            &count,
            |b, _| {
                b.iter(|| black_box(index.search(&query, 10).unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_search_quantized(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_search_quantized");
    group.sample_size(50);

    for count in [1000, 10000] {
        let vectors = generate_vectors(count, 128, 42);
        let mut index = FlatIndex::new(FlatIndexConfig::new(128));
        for v in &vectors {
            index.insert(v).unwrap();
        }
        index.enable_quantization().unwrap();

        let query = generate_vectors(1, 128, 999)[0].clone();

        group.bench_with_input(
            BenchmarkId::new("k10", count),
            &count,
            |b, _| {
                b.iter(|| black_box(index.search_quantized(&query, 10).unwrap()));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert,
    bench_search,
    bench_search_quantized,
);

criterion_main!(benches);
```

**Acceptance Criteria:**
- [ ] Insert benchmark for 100, 1k, 10k vectors
- [ ] Search benchmark for 1k, 10k vectors
- [ ] Quantized search benchmark
- [ ] Results logged and within targets
- [ ] Cargo.toml updated with benchmark config

**Deliverables:**
- `benches/flat_bench.rs`
- Updated `Cargo.toml`

**Dependencies:** W40.3.1, W40.3.3

**Estimated Duration:** 0.5 hours

**Agent:** BENCHMARK_SCIENTIST

---

## Verification Strategy

### Performance Tests
```bash
cargo bench --bench flat_bench
```

Expected results:
- Insert (10k): <10ms total
- Search (10k, k=10): <50ms
- Quantized search (10k, k=10): <10ms

### Unit Tests
- Deletion tests
- Compaction tests
- Quantization tests

---

## Exit Criteria for Day 3

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Search <50ms (10k) | Benchmark | [ ] |
| Insert <100us | Benchmark | [ ] |
| Soft delete works | Unit tests | [ ] |
| Compaction works | Unit tests | [ ] |
| BQ quantization available | Unit tests | [ ] |
| Memory reduction 32x | Measurement | [ ] |
| Benchmark suite created | `cargo bench` | [ ] |
| Clippy clean | 0 warnings | [ ] |

---

**Day 3 Total:** 5 hours
**Agent:** RUST_ENGINEER
**Status:** [DRAFT]
