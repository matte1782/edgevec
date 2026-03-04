# PQ Benchmark Plan

**Author:** BENCHMARK_SCIENTIST
**Date:** 2026-03-27
**Status:** [REVISED]
**Purpose:** Reproducible benchmark methodology for W46 PQ GO/NO-GO decision
**Prerequisite:** PQ implementation (W46 Days 1-3)
**References:** `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md` (LEAN GO, pending benchmarks)

---

## 1. Benchmark Environment

### 1.1 Hardware Requirements

| Component | Minimum | Recommended | Notes |
|:----------|:--------|:------------|:------|
| CPU | Any x64 with SSE4.2+ | Modern multi-GHz (Zen 3+, Alder Lake+) | SIMD128 support required for WASM |
| RAM | 8 GB | 16 GB | 100K vectors at 768D raw = ~300 MB; PQ codes + codebook < 2 MB |
| Disk | SSD | NVMe SSD | For native persistence benchmarks only |

Document the following in every report:
- CPU model and core count (e.g., `lscpu` or `wmic cpu get name`)
- RAM size and speed
- OS and kernel/build version
- Rust version (`rustc --version`)
- Chrome version (for WASM benchmarks)
- Commit hash of EdgeVec under test

### 1.2 Software Requirements

| Tool | Version | Purpose |
|:-----|:--------|:--------|
| Rust | stable >= 1.70 | Native compilation |
| Criterion | 0.5.x | Native benchmark harness |
| wasm-pack | latest | WASM build |
| Chrome | 134+ | WASM SIMD128 runtime |
| `performance.now()` | Web API | WASM timing (sub-ms precision) |

### 1.3 Reproducibility Protocol

1. **Seeded RNG:** All synthetic datasets generated with `ChaCha8Rng::seed_from_u64(42)`.
2. **Warmup:** 3 iterations discarded before measurement.
3. **Measured iterations:** 10 iterations minimum. Report median and P99.
4. **Isolation:**
   - Native: Run with `--release`, no other CPU-intensive processes.
   - WASM: Single Chrome tab, no extensions, no DevTools open (DevTools deoptimizes V8).
5. **Variance check:** If P99 > 3x median, investigate and document the cause. Re-run if attributable to system interference.
6. **Dual-target:** Every benchmark runs on BOTH native (`cargo bench`) and WASM (Chrome). Native provides a baseline to isolate WASM overhead.

---

## 2. Dataset Generation

### 2.1 Synthetic Dataset Specification

| Parameter | Value | Rationale |
|:----------|:------|:----------|
| Dimensions | 768 | Primary target (all-mpnet-base-v2, BGE-base) |
| Distribution | Uniform [-1.0, 1.0] | Pessimistic for PQ (real embeddings have structure k-means exploits) |
| Seed | 42 | Reproducibility |
| Sizes | 10K, 50K, 100K | Match literature review benchmark plan |
| Query count | 1,000 | Statistically significant recall measurement |
| Query source | Separate seed (seed=137) | Queries must not be in the training/index set |

### 2.2 Dataset Generation Code (Canonical)

The following Rust code is the SINGLE SOURCE OF TRUTH for dataset generation across all benchmarks. Any deviation invalidates the results.

```rust
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

/// Generates a deterministic dataset of f32 vectors.
///
/// # Arguments
/// * `count` - Number of vectors to generate
/// * `dims` - Dimensionality of each vector
/// * `seed` - RNG seed for reproducibility
///
/// # Returns
/// Vec of Vec<f32>, each inner vec has length `dims`.
fn generate_dataset(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| {
            (0..dims)
                .map(|_| rng.gen_range(-1.0f32..1.0f32))
                .collect()
        })
        .collect()
}

// Datasets used across all PQ benchmarks:
const DIMS: usize = 768;
const SEED_BASE: u64 = 42;       // For index/training vectors
const SEED_QUERY: u64 = 137;     // For query vectors (MUST differ from base)

// Dataset sizes:
// - Small:  generate_dataset(10_000,  DIMS, SEED_BASE)
// - Medium: generate_dataset(50_000,  DIMS, SEED_BASE)
// - Large:  generate_dataset(100_000, DIMS, SEED_BASE)
// - Queries: generate_dataset(1_000,  DIMS, SEED_QUERY)
```

### 2.3 Ground Truth Computation

Ground truth (exact k-NN) is computed by brute-force L2 distance over the full-precision f32 dataset. This is the reference against which PQ recall is measured.

```rust
use std::collections::BinaryHeap;
use std::cmp::Reverse;

/// Computes exact k-nearest neighbors by brute-force L2 distance.
///
/// # Arguments
/// * `base` - The indexed dataset (f32 vectors)
/// * `query` - A single query vector
/// * `k` - Number of neighbors to return
///
/// # Returns
/// Vec of (index, distance) sorted by distance ascending.
fn exact_knn(base: &[Vec<f32>], query: &[f32], k: usize) -> Vec<(usize, f32)> {
    // O(n log k) algorithm using a MAX-heap of size k.
    // We keep the k smallest distances seen so far. When the heap exceeds
    // size k, we pop the largest (max) element. At the end, the heap
    // contains exactly the k nearest neighbors.
    use std::cmp::Ordering;

    let mut heap: BinaryHeap<(ordered_float::OrderedFloat<f32>, usize)> =
        BinaryHeap::new();

    for (idx, vec) in base.iter().enumerate() {
        let dist = l2_distance_squared(query, vec);
        let ord_dist = ordered_float::OrderedFloat(dist);
        if heap.len() < k {
            heap.push((ord_dist, idx));
        } else if let Some(&(max_dist, _)) = heap.peek() {
            if ord_dist < max_dist {
                heap.pop();
                heap.push((ord_dist, idx));
            }
        }
    }

    // Drain heap into results sorted by distance ascending
    let mut results: Vec<(usize, f32)> = heap
        .into_iter()
        .map(|(d, idx)| (idx, d.0))
        .collect();
    results.sort_by(|a, b| a.1.total_cmp(&b.1));
    results
}

fn l2_distance_squared(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}
```

**Pre-compute and cache:** Ground truth for each dataset size should be computed once and reused across B3, B3b, and B4. For 1,000 queries on 100K vectors, brute-force takes ~30-60 seconds. This cost is NOT included in benchmark timings.

### 2.4 Why Synthetic (Not SIFT-1M)

1. **Reproducibility without downloads:** No external data dependency.
2. **Pessimistic baseline:** Uniform [-1, 1] lacks the cluster structure that k-means exploits. If PQ passes on synthetic, it performs equal or better on real embeddings.
3. **768D target:** SIFT-1M is 128D. EdgeVec targets 768D embeddings. Synthetic data at 768D is more representative of the actual use case.
4. **Caveat:** Recall numbers on synthetic data are expected to be LOWER than on real embeddings. If recall is borderline (0.88-0.92), consider a supplementary run on a real embedding dataset before making a final NO-GO decision.

---

## 3. Benchmark Suite

### B1: PQ Encoding Speed

**What we measure:** Throughput of encoding f32 vectors into PQ codes, after codebook training is complete.

**Why it matters:** Encoding happens on every insert. If encoding is too slow, insert latency breaks the <5ms budget (ARCHITECTURE.md R4).

**Methodology:**

1. Train codebook on the full dataset (training is NOT measured here -- see B5/B7).
2. Measure time to encode `N` vectors using the trained codebook.
3. Compute throughput = N / elapsed_seconds.

**Measurement details:**
- Timer: `std::time::Instant` (native), `performance.now()` (WASM).
- Start timer AFTER codebook is loaded and vectors are in memory.
- Stop timer AFTER all N vectors are encoded.
- Do NOT include codebook training, memory allocation for output buffer, or I/O.

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Dataset | 50K vectors, 768D, seed=42 |
| PQ config | M=8, Ksub=256 |
| Iterations | 3 warmup + 10 measured |

**Code structure (native, Criterion):**

```rust
fn bench_pq_encoding(c: &mut Criterion) {
    let dims = 768;
    let count = 50_000;
    let vectors = generate_dataset(count, dims, 42);

    // Train codebook OUTSIDE the benchmark loop
    let codebook = PqCodebook::train(&vectors, 8, 256, 15).unwrap();

    let mut group = c.benchmark_group("pq_encoding");
    group.throughput(Throughput::Elements(count as u64));
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(5));

    group.bench_function("encode_50k_768d", |b| {
        b.iter(|| {
            let codes: Vec<_> = vectors
                .iter()
                .map(|v| codebook.encode(black_box(v)))
                .collect();
            black_box(codes)
        });
    });

    group.finish();
}
```

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Native | > 10,000 vectors/sec | From literature review B1 |
| WASM | > 5,000 vectors/sec | Expect 2x WASM overhead |

**GO/NO-GO mapping:** Informational only. Encoding speed does not directly map to G1-G6, but a catastrophic failure (<1,000 vec/sec) would indicate implementation problems affecting G5.

---

### B2: ADC Search Latency

**What we measure:** Per-candidate Asymmetric Distance Computation (ADC) latency -- the cost of computing one approximate distance from a query to a PQ-encoded vector using the precomputed lookup table.

**Why it matters:** This is the inner-loop cost during search. Maps directly to **G2: ADC search latency < 150ns per candidate in WASM**.

**Methodology:**

1. Train codebook on 100K vectors.
2. Encode all 100K vectors into PQ codes.
3. Precompute the ADC distance table for a single query vector (table construction is NOT measured).
4. Measure time to compute ADC distances for all 100K PQ codes using the lookup table.
5. Compute per-candidate latency = total_time / 100,000.

**Measurement details:**
- Timer: `std::time::Instant` (native), `performance.now()` (WASM).
- Start timer AFTER the distance table is constructed and PQ codes are in memory.
- Stop timer AFTER all 100K distances are computed.
- Use `black_box` on the output to prevent dead-code elimination.
- Run with multiple queries (10 different queries), report median of medians.

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Dataset | 100K PQ codes, M=8 |
| Dimensions | 768 |
| Queries | 10 queries (seed=137) |
| Iterations per query | 3 warmup + 10 measured |

**Code structure (native, Criterion):**

```rust
fn bench_adc_latency(c: &mut Criterion) {
    let dims = 768;
    let base = generate_dataset(100_000, dims, 42);
    let queries = generate_dataset(10, dims, 137);

    let codebook = PqCodebook::train(&base, 8, 256, 15).unwrap();
    let codes: Vec<_> = base.iter().map(|v| codebook.encode(v)).collect();

    let mut group = c.benchmark_group("adc_latency");
    group.throughput(Throughput::Elements(100_000));
    group.sample_size(10);

    for (qi, query) in queries.iter().enumerate() {
        let distance_table = codebook.compute_distance_table(query);

        group.bench_with_input(
            BenchmarkId::new("100k_candidates", qi),
            &qi,
            |b, _| {
                b.iter(|| {
                    let distances: Vec<f32> = codes
                        .iter()
                        .map(|code| distance_table.compute_distance(black_box(code)))
                        .collect();
                    black_box(distances)
                });
            },
        );
    }

    group.finish();
}
```

**WASM measurement (Chrome, manual harness):**

```javascript
// In a WASM test harness (not Criterion -- Criterion does not run in WASM)
const iterations = 10;
const latencies = [];
for (let i = 0; i < 3 + iterations; i++) {
    const start = performance.now();
    edgevec.adc_scan_all(queryIndex); // Scans all 100K codes
    const elapsed = performance.now() - start;
    if (i >= 3) latencies.push(elapsed); // Skip warmup
}
latencies.sort((a, b) => a - b);
const median_ms = latencies[Math.floor(latencies.length / 2)];
const per_candidate_ns = (median_ms * 1_000_000) / 100_000;
console.log(`ADC per-candidate: ${per_candidate_ns.toFixed(1)} ns`);
```

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Native | < 100 ns/candidate | Baseline for WASM comparison |
| **WASM** | **< 150 ns/candidate (median)** | **GO/NO-GO criterion G2** |

**GO/NO-GO mapping:** **G2 (direct).** If WASM median exceeds 150 ns/candidate, this is an automatic NO-GO.

---

### B3: Recall@10 vs Exact (10K)

**What we measure:** The fraction of true top-10 nearest neighbors that PQ's approximate search returns, on a 10K vector dataset.

**Why it matters:** Maps directly to **G3: Recall@10 > 0.90 for PQ standalone**. The 10K test validates basic correctness before scaling up.

**Methodology:**

1. Generate 10K base vectors (seed=42) and 1,000 query vectors (seed=137).
2. Compute exact ground truth: brute-force L2 top-10 for each query.
3. Train PQ codebook on the 10K base vectors. (M=8, Ksub=256, 15 k-means iterations.)
4. Encode all 10K vectors into PQ codes.
5. For each query:
   a. Compute ADC distance table.
   b. Scan all 10K PQ codes, collect top-10 by ADC distance.
   c. Compare PQ top-10 IDs against ground truth top-10 IDs.
6. Recall@10 = (sum of intersections across all queries) / (1,000 * 10).

**Recall formula:**

```
recall@k = (1/Q) * sum_{q=1}^{Q} |PQ_topk(q) intersection GT_topk(q)| / k
```

Where:
- Q = number of queries (1,000)
- k = 10
- PQ_topk(q) = set of k IDs returned by PQ for query q
- GT_topk(q) = set of k IDs from exact brute-force for query q

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Base dataset | 10K vectors, 768D, seed=42 |
| Query dataset | 1,000 vectors, 768D, seed=137 |
| PQ config | M=8, Ksub=256, 15 k-means iterations |
| k | 10 |
| Search method | Exhaustive ADC scan (no HNSW -- isolates PQ quality) |

**Code structure:**

```rust
fn measure_pq_recall(
    base: &[Vec<f32>],
    queries: &[Vec<f32>],
    codebook: &PqCodebook,
    codes: &[PqCode],
    k: usize,
) -> f64 {
    let mut total_hits = 0usize;

    for query in queries {
        // Ground truth (exact)
        let gt = exact_knn(base, query, k);
        let gt_ids: HashSet<usize> = gt.iter().map(|(id, _)| *id).collect();

        // PQ approximate
        let distance_table = codebook.compute_distance_table(query);
        let mut pq_distances: Vec<(usize, f32)> = codes
            .iter()
            .enumerate()
            .map(|(idx, code)| (idx, distance_table.compute_distance(code)))
            .collect();
        pq_distances.sort_by(|a, b| a.1.total_cmp(&b.1)); // NaN-safe: NaN sorts last
        let pq_ids: HashSet<usize> = pq_distances[..k].iter().map(|(id, _)| *id).collect();

        total_hits += gt_ids.intersection(&pq_ids).count();
    }

    total_hits as f64 / (queries.len() * k) as f64
}
```

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Recall@10 | > 0.90 | GO/NO-GO criterion G3 |
| Expected range | 0.85-0.95 | Synthetic uniform data is pessimistic |

**GO/NO-GO mapping:** **G3 (partial).** B3 at 10K is a necessary but not sufficient condition. B3b at 50K provides the confirming data point.

**If recall is borderline (0.88-0.92):**
1. Run B3 with M=16 to determine if more subspaces help.
2. Run B3 on a real embedding dataset (e.g., 10K GloVe-100 vectors from ann-benchmarks).
3. If M=16 at 768D exceeds 0.90, document the M=8 gap and recommend M=16 as default.

---

### B3b: Recall@10 vs Exact (50K)

**What we measure:** Same as B3, but on 50K vectors. Validates that codebook quality improves with more training data and that recall holds at larger scale.

**Why it matters:** PQ codebooks trained on more data should produce better centroids. If recall DROPS from 10K to 50K, something is wrong with the implementation.

**Methodology:** Identical to B3, substituting:
- Base dataset: 50K vectors (seed=42, first 50K from the generator)
- Training set: same 50K vectors

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Base dataset | 50K vectors, 768D, seed=42 |
| Query dataset | 1,000 vectors, 768D, seed=137 |
| PQ config | M=8, Ksub=256, 15 k-means iterations |
| k | 10 |
| Search method | Exhaustive ADC scan |

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Recall@10 | > 0.90 | GO/NO-GO criterion G3 |
| Recall@10 vs B3 | >= B3 recall | Recall must not degrade with more training data |

**GO/NO-GO mapping:** **G3 (confirming).** Both B3 and B3b must independently exceed 0.90.

---

### B4: Recall@10 PQ vs BQ+Rescore

**What we measure:** Comparative recall between PQ standalone (ADC) and the existing BQ+rescore pipeline. This determines whether PQ is competitive with EdgeVec's current best search path.

**Why it matters:** If PQ recall is significantly below BQ+rescore, PQ becomes a memory-only optimization with a quality penalty, which may not be acceptable to users.

**Methodology:**

1. Generate 10K base vectors (seed=42) and 1,000 queries (seed=137).
2. Compute exact ground truth (brute-force L2 top-10).
3. **PQ path:** Train codebook, encode, ADC scan top-10. (Same as B3.)
4. **BQ+rescore path:** Binary-quantize all vectors. BQ scan top-100 by Hamming distance. Rescore top-100 candidates using full f32 L2 distance. Return top-10 after rescore.
5. Compare both against ground truth.

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Base dataset | 10K vectors, 768D, seed=42 |
| Query dataset | 1,000 vectors, 768D, seed=137 |
| PQ config | M=8, Ksub=256, 15 iterations |
| BQ config | Standard EdgeVec BQ (sign-bit, 96 bytes/vector) |
| BQ rescore depth | Top-100 candidates rescored with f32 L2 |
| k | 10 |

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| PQ recall@10 | > 0.85 relative to BQ+rescore | From literature review B4 |
| Expected | PQ standalone ~ 0.90, BQ+rescore ~ 0.95 | PQ slightly lower but avoids f32 storage |

**GO/NO-GO mapping:** Informational for the GO/NO-GO decision. No automatic gate, but if PQ recall@10 < 0.80 while BQ+rescore > 0.95, the quality gap should be flagged as a risk in the decision document.

**Important note:** This benchmark compares PQ WITHOUT rescore to BQ WITH rescore. This is intentionally asymmetric because PQ's value proposition is avoiding full f32 storage for rescoring. If PQ WITH rescore is needed for acceptable recall, the memory savings argument weakens significantly (must store both PQ codes AND f32 vectors).

---

### B5: Codebook Training Time (50K)

**What we measure:** Wall-clock time to train a PQ codebook from scratch on 50K vectors.

**Why it matters:** Training is a user-visible blocking operation. Contributes data to **G4: Training time < 60 seconds for 100K vectors**.

**Methodology:**

1. Generate 50K vectors (seed=42), 768D.
2. Measure end-to-end time for `PqCodebook::train(&vectors, M=8, Ksub=256, max_iters=15)`.
3. Timer starts BEFORE any allocation inside `train()`.
4. Timer stops AFTER `train()` returns a fully usable codebook.
5. Include convergence check cost, centroid initialization, all k-means iterations.

**Measurement details:**
- Native: `std::time::Instant::now()` around the `train()` call.
- WASM: `performance.now()` around the WASM export `train_pq()`.
- Report: wall-clock seconds, number of k-means iterations until convergence (or max_iters reached).

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Dataset | 50K vectors, 768D, seed=42 |
| PQ config | M=8, Ksub=256, max_iters=15 |
| Iterations | 3 measured runs, report median |

**Code structure (native):**

```rust
fn bench_codebook_training_50k() {
    let vectors = generate_dataset(50_000, 768, 42);

    let mut times = Vec::new();
    for _ in 0..3 {
        let start = std::time::Instant::now();
        let codebook = PqCodebook::train(&vectors, 8, 256, 15).unwrap();
        let elapsed = start.elapsed();
        black_box(&codebook);
        times.push(elapsed);
    }

    times.sort();
    let median = times[times.len() / 2];
    println!("B5 codebook training (50K): {:.2}s (median)", median.as_secs_f64());
}
```

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Native | < 15 seconds | Baseline; native should be ~2x faster than WASM |
| WASM | < 30 seconds | Literature review B5 target |

**GO/NO-GO mapping:** Contributes to **G4** (extrapolation to 100K). If 50K takes >30s in WASM, 100K will almost certainly exceed 60s.

---

### B6: Memory Footprint

**What we measure:** Total memory consumed by PQ codes + codebook, compared to BQ memory for the same dataset. This is a CALCULATION, not a runtime measurement.

**Why it matters:** Maps directly to **G1: PQ total memory < 70% of BQ total memory at 100K vectors**.

**Methodology:**

This is a static analysis, not a timing benchmark. Compute:

```
PQ_total = (N * M) + codebook_bytes
BQ_total = N * ceil(D / 8)
ratio = PQ_total / BQ_total
```

For D=768, M=8, N=100K, Ksub=256:

```
PQ per-vector:    M bytes = 8 bytes
PQ codebook:      M * Ksub * (D/M) * sizeof(f32) = 8 * 256 * 96 * 4 = 786,432 bytes
                  (simplifies to Ksub * D * sizeof(f32) = 256 * 768 * 4 since M cancels)
PQ total (100K):  100,000 * 8 + 786,432 = 1,586,432 bytes (1.51 MB)

BQ per-vector:    768 / 8 = 96 bytes
BQ total (100K):  100,000 * 96 = 9,600,000 bytes (9.16 MB)

Ratio: 1,586,432 / 9,600,000 = 0.1653 = 16.5%
```

**Runtime validation:** Verify the calculated values against actual allocated memory using a tracking allocator:

```rust
fn measure_pq_memory(vectors: &[Vec<f32>]) {
    let before = ALLOCATED.load(Ordering::SeqCst);

    let codebook = PqCodebook::train(vectors, 8, 256, 15).unwrap();
    let codes: Vec<PqCode> = vectors.iter().map(|v| codebook.encode(v)).collect();

    let after = ALLOCATED.load(Ordering::SeqCst);
    let actual = after - before;

    // Also compute theoretical:
    let theoretical = vectors.len() * 8 + 786_432;

    println!("PQ memory ({}): actual={} bytes, theoretical={} bytes, overhead={:.1}%",
        vectors.len(), actual, theoretical,
        ((actual as f64 / theoretical as f64) - 1.0) * 100.0);
}
```

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Dataset sizes | 10K, 50K, 100K |
| PQ config | M=8, Ksub=256 |
| BQ config | Standard (96 bytes/vector for 768D) |

**Results table format:**

| N | PQ codes (bytes) | PQ codebook (bytes) | PQ total (bytes) | BQ total (bytes) | PQ/BQ ratio | Per-vector amortized (PQ) |
|:--|:-----------------|:--------------------|:-----------------|:-----------------|:------------|:--------------------------|
| 10K | | | | | | |
| 50K | | | | | | |
| 100K | | | | | | |

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| PQ/BQ ratio at 100K | < 0.70 (70%) | GO/NO-GO criterion G1 |
| PQ per-vector amortized at 100K | < 20 bytes | Literature review B6 |

**GO/NO-GO mapping:** **G1 (direct).** The literature review already calculates 16.5% -- this benchmark VALIDATES that the implementation matches the theoretical prediction. If actual memory exceeds theoretical by > 50% (due to Vec overhead, alignment padding, etc.), investigate and document.

---

### B7: Training Time (100K)

**What we measure:** Wall-clock time to train a PQ codebook on 100K vectors. This is the definitive G4 measurement.

**Why it matters:** Maps directly to **G4: Training time < 60 seconds for 100K vectors in Chrome**.

**Methodology:** Identical to B5, substituting 100K vectors.

**Parameters:**

| Parameter | Value |
|:----------|:------|
| Dataset | 100K vectors, 768D, seed=42 |
| PQ config | M=8, Ksub=256, max_iters=15 |
| Iterations | 3 measured runs, report median |

**WASM-specific concerns:**
- Chrome may trigger a "page unresponsive" dialog if training runs on the main thread for >5 seconds. If this occurs, document it and note that production deployment requires Web Worker offloading.
- Monitor memory pressure: 100K vectors at 768D = ~300 MB of f32 data. Chrome tabs may be memory-limited.
- If Chrome kills the tab (OOM), reduce to 50K and extrapolate (document the extrapolation).

**Pass/fail threshold:**

| Target | Threshold | Rationale |
|:-------|:----------|:----------|
| Native | < 30 seconds | Baseline |
| **WASM** | **< 60 seconds** | **GO/NO-GO criterion G4** |

**GO/NO-GO mapping:** **G4 (direct).** If WASM exceeds 60 seconds median, this is an automatic NO-GO unless:
1. The excess is < 10% (i.e., < 66 seconds) AND
2. Profiling shows an optimization opportunity with > 20% potential improvement.

In that case, document as CONDITIONAL GO pending optimization in W47.

---

## 4. Results Template

Fill in after running benchmarks. Leave blank until execution.

### 4.1 Environment Record

| Component | Value |
|:----------|:------|
| CPU | |
| RAM | |
| OS | |
| Rust version | |
| Chrome version | |
| EdgeVec commit | |
| Date | |

### 4.2 B1: PQ Encoding Speed

| Metric | Native | WASM | Target |
|:-------|:-------|:-----|:-------|
| Throughput (vectors/sec) | | | > 10,000 (native), > 5,000 (WASM) |
| Total time for 50K (sec) | | | |
| P99 per-vector (us) | | | |

### 4.3 B2: ADC Search Latency

| Metric | Native | WASM | Target |
|:-------|:-------|:-----|:-------|
| Median per-candidate (ns) | | | < 150 ns (WASM, G2) |
| P99 per-candidate (ns) | | | |
| Total scan 100K (ms) | | | |
| WASM / Native overhead | | N/A | Expected: 1.5-3x |

### 4.4 B3: Recall@10 (10K)

| Metric | Value | Target |
|:-------|:------|:-------|
| Recall@10 (PQ, M=8) | | > 0.90 (G3) |
| Recall@1 (PQ, M=8) | | Informational |
| Recall@100 (PQ, M=8) | | Informational |

### 4.5 B3b: Recall@10 (50K)

| Metric | Value | Target |
|:-------|:------|:-------|
| Recall@10 (PQ, M=8) | | > 0.90 (G3) |
| Recall@10 vs B3 | | Must be >= B3 |

### 4.6 B4: PQ vs BQ+Rescore Recall

| Metric | PQ Standalone | BQ + Rescore (top-100) | Target |
|:-------|:-------------|:-----------------------|:-------|
| Recall@10 | | | PQ > 0.85 relative to BQ+rescore |
| Recall@1 | | | Informational |

### 4.7 B5: Training Time (50K)

| Metric | Native | WASM | Target |
|:-------|:-------|:-----|:-------|
| Median wall-clock (sec) | | | < 30 s (WASM) |
| K-means iterations | | | |
| Convergence achieved? | | | |

### 4.8 B6: Memory Footprint

| N | PQ codes (B) | PQ codebook (B) | PQ total (B) | BQ total (B) | PQ/BQ ratio | Per-vector amortized |
|:--|:-------------|:----------------|:-------------|:-------------|:------------|:---------------------|
| 10K | | | | | | |
| 50K | | | | | | |
| 100K | | | | | | Target: < 70% (G1) |

### 4.9 B7: Training Time (100K)

| Metric | Native | WASM | Target |
|:-------|:-------|:-----|:-------|
| Median wall-clock (sec) | | | < 60 s (WASM, G4) |
| K-means iterations | | | |
| Convergence achieved? | | | |
| Chrome tab unresponsive? | N/A | | Must not crash |

---

## 5. GO/NO-GO Decision Matrix

**ALL six criteria must pass for a GO decision. Any single FAIL is an automatic NO-GO.**

| # | Criterion | Threshold | Benchmark | Result | Status |
|:--|:----------|:----------|:----------|:-------|:-------|
| G1 | Memory savings meaningful | PQ total < 70% of BQ total at 100K | B6 | _fill_ | _fill_ |
| G2 | ADC latency acceptable | Median < 150 ns/candidate in WASM | B2 | _fill_ | _fill_ |
| G3 | Recall competitive | Recall@10 > 0.90 (PQ standalone) | B3 + B3b | _fill_ | _fill_ |
| G4 | Training time acceptable | < 60 seconds for 100K in Chrome | B7 | _fill_ | _fill_ |
| G5 | Implementation fits budget | < 16 hours total | Engineering assessment | _fill_ | _fill_ |
| G6 | No API breakage | All existing tests pass | `cargo test` + `npx vitest run` | _fill_ | _fill_ |

**Decision:**

| Condition | Verdict |
|:----------|:--------|
| All G1-G6 PASS | **GO** -- proceed with PQ integration into v0.10.0 |
| Any G1-G6 FAIL | **NO-GO** -- defer PQ, document failure reasons |
| G3 borderline (0.88-0.92) | Run supplementary benchmarks (M=16, real embeddings) before final decision |
| G4 borderline (60-66s) | **CONDITIONAL GO** if optimization path identified |

---

## 6. Risk Mitigations

### 6.1 If B2 (ADC Latency) Fails Marginally

**Symptom:** WASM median is 150-200 ns/candidate.

**Mitigations:**
1. Profile the ADC inner loop for V8 deoptimization triggers (type instability, megamorphic calls).
2. Try loop unrolling (manually unroll the 8 subspace lookups).
3. Try the multi-vector batching approach described in the literature review (Section 4.3.2): process 4 vectors per iteration to amortize SIMD accumulation.
4. If <200 ns after optimization: CONDITIONAL GO with documented performance caveat.
5. If >200 ns after optimization: NO-GO.

### 6.2 If B3/B3b (Recall) Fails

**Symptom:** Recall@10 < 0.90 on synthetic data.

**Mitigations:**
1. Test with M=16 (doubles PQ code size to 16 bytes, but still 6x smaller than BQ).
2. Test on a real embedding dataset (GloVe-100 from ann-benchmarks). Synthetic uniform is pessimistic; real data may push recall above 0.90.
3. Increase k-means iterations from 15 to 30 (may improve codebook quality at training cost).
4. If M=16 at 768D exceeds 0.90: GO with M=16 as default, document M=8 as an advanced option.
5. If M=16 still fails on real data: NO-GO.

### 6.3 If B7 (Training Time 100K) Fails

**Symptom:** WASM training exceeds 60 seconds.

**Mitigations:**
1. Profile k-means iterations: are all iterations necessary? Check convergence rate. If 80% of training time is in iterations where centroid movement is negligible, add early-stopping.
2. Reduce max_iters from 15 to 10 if convergence is achieved earlier.
3. Use mini-batch k-means (sample 10K vectors per iteration instead of using all 100K). This typically converges in 2-5x fewer wall-clock seconds.
4. Recommend offline training (server-side) with codebook import as the primary path for 100K+ datasets.
5. If <75s after optimization: CONDITIONAL GO with Web Worker requirement documented.
6. If >75s: NO-GO for in-browser training at 100K. GO for offline training + import path only.

### 6.4 If B6 (Memory) Shows Unexpected Overhead

**Symptom:** Actual memory > 2x theoretical.

**Mitigations:**
1. Audit Vec allocations: Rust Vec over-allocates by 2x on growth. Pre-allocate with `Vec::with_capacity`.
2. Check for retained temporary allocations (k-means scratch space not freed after training).
3. Ensure codebook is stored as a flat `Vec<f32>` not as `Vec<Vec<f32>>` (inner Vec headers add 24 bytes each).
4. If overhead is in training temporaries only (freed after `train()` returns): acceptable, document peak vs steady-state.

### 6.5 Chrome Tab Crash During B7

**Symptom:** Chrome kills the tab during 100K training (OOM or unresponsive timeout).

**Mitigations:**
1. Reduce to 50K, measure, and linearly extrapolate.
2. Document that 100K in-browser requires Web Worker with dedicated memory.
3. If 50K completes in <25s: extrapolated 100K = ~50s, borderline CONDITIONAL GO.
4. If 50K also crashes: NO-GO for in-browser training. GO for offline training path only.

---

## 7. Execution Checklist

This checklist is for the engineer running the benchmarks during W46/W47.

### Pre-Benchmark

- [ ] PQ implementation compiles: `cargo build --release`
- [ ] All existing tests pass: `cargo test` (980+ tests, 0 failures)
- [ ] WASM build succeeds: `wasm-pack build --release`
- [ ] Ground truth pre-computed and cached for 10K, 50K, 100K datasets
- [ ] Hardware/software environment documented (Section 4.1 template filled)

### Benchmark Execution Order

Run benchmarks in this order (dependencies noted):

1. [ ] **B6** (Memory) -- no runtime dependency, can be computed from code inspection
2. [ ] **B1** (Encoding speed) -- requires trained codebook
3. [ ] **B5** (Training 50K) -- measures training itself
4. [ ] **B7** (Training 100K) -- must run after B5 to validate scaling
5. [ ] **B2** (ADC latency) -- requires trained codebook + encoded vectors
6. [ ] **B3** (Recall 10K) -- requires trained codebook + encoded vectors + ground truth
7. [ ] **B3b** (Recall 50K) -- same as B3, larger scale
8. [ ] **B4** (PQ vs BQ recall) -- requires B3 results + BQ pipeline

### Post-Benchmark

- [ ] All results entered in Section 4 tables
- [ ] GO/NO-GO matrix (Section 5) filled with verdicts
- [ ] Any mitigation actions from Section 6 documented
- [ ] Report reviewed by HOSTILE_REVIEWER (`/review docs/benchmarks/pq_benchmark_report.md`)

---

## 8. File Locations

| Artifact | Path |
|:---------|:-----|
| This plan | `docs/research/PQ_BENCHMARK_PLAN.md` |
| Literature review | `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md` |
| Benchmark code (native) | `benches/pq_bench.rs` (to be created in W46) |
| Benchmark code (WASM) | `tests/wasm/pq_bench.js` (to be created in W46) |
| Results report | `docs/benchmarks/pq_benchmark_report.md` (to be created in W47) |
| PQ implementation | `src/quantization/product.rs` (to be created in W46) |

---

## BENCHMARK_SCIENTIST: Plan Complete

**Artifacts generated:**
- `docs/research/PQ_BENCHMARK_PLAN.md` (this document)

**Summary:**
- 7 benchmarks (B1-B7) with exact methodology, code templates, and thresholds
- 6 GO/NO-GO criteria mapped to specific benchmarks
- Risk mitigations for each potential failure mode
- Pre-formatted results tables ready for W46/W47 data entry
- Execution checklist for the implementing engineer

**Status:** PENDING_HOSTILE_REVIEW

**Next:** `/review docs/research/PQ_BENCHMARK_PLAN.md`
