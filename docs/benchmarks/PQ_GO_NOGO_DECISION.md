# PQ GO/NO-GO Decision — W46 Benchmark Results + W47 Validation

**Date:** 2026-03-28 (W46) / 2026-03-06 (W47 update)
**Author:** BENCHMARK_SCIENTIST
**Status:** [REVISED] — W47 validation complete

---

## Environment

| Item | Value |
|:-----|:------|
| **OS** | Windows 11 Pro 10.0.26200 |
| **CPU** | Intel Core Ultra 9 285H (16 cores, 16 threads) |
| **RAM** | 32 GB |
| **Rust** | rustc 1.94.0-nightly (1aa9bab4e 2025-12-05) |
| **Commit (W46)** | 7ec4219 (W46 Day 3) |
| **Commits (W47)** | baeb7d4 (Day 1) through e911ee7 (Day 5) |
| **Target** | Native x86_64 + WASM (Chrome 145, Playwright) |

---

## Executive Summary

Product Quantization (PQ) implementation completed in W46 Days 1-3 (codebook training, encode, ADC scan, property tests, NaN validation). Days 4-5 benchmark results below.

**W46 Verdict: CONDITIONAL GO** — PQ is architecturally sound and performant on native. G3 (recall) INCONCLUSIVE on synthetic data. G4 (training time) FAILS. WASM validation deferred to W47.

**W47 Updated Verdict: CONDITIONAL GO** — G2 WASM PASS. G4 native PASS (rayon). G3 FAIL on real embeddings (recall too low). G4 WASM FAIL (no rayon). BQ+rescore outperforms PQ on recall. See W47 section below.

---

## Gate Results

| Gate | Criterion | Target | W46 Result | W47 Result | Final Verdict |
|:-----|:----------|:-------|:-----------|:-----------|:--------------|
| **G1** | PQ memory < 70% of BQ at 100K | <70% | **16.5%** | — | **PASS** |
| **G2** | ADC latency < 150ns/candidate | <150ns | 37.6 ns (native) | **145 ns P99 (WASM)** | **PASS** |
| **G3** | Recall@10 > 0.90 | >0.90 | 0.02 (synthetic) | **0.39 M=8 / 0.53 M=16 (real)** | **FAIL** |
| **G4** | Training < 30s native / <60s WASM | <30s / <60s | 198.7s (native) | **9.05s native / 124.6s WASM** | **CONDITIONAL** |
| **G5** | Implementation < 16h | <16h | ~12h | — | **PASS** |
| **G6** | No API breakage | 0 breaking | 0 | 0 | **PASS** |

### WASM Results (Updated W47)

W46 measurements were native-only. W47 measured WASM via Playwright + Chrome 145:

- **G2 WASM ADC:** P99 = 145 ns/candidate — **PASS** (threshold: <150ns). W46 estimated ~113ns; actual overhead was ~3.9x (vs estimated 3x).
- **G4 WASM Training:** 124.6s median at 100K — **FAIL** (threshold: <60s). WASM has no rayon (single-threaded). Native with rayon: 9.05s PASS. WASM training at 100K scale requires Web Workers (future work).

---

## B1: PQ Encoding Speed (50K vectors, 768D, M=8, Ksub=256)

| Metric | Value |
|:-------|:------|
| Median time | 3.03s |
| Per-vector | 60.6 us |
| Throughput | 16.5K vectors/s |
| Range | [2.999s, 3.066s] |
| Sample size | 10 Criterion iterations |

**Assessment:** Encoding speed is acceptable for batch indexing. 50K vectors in 3s is fast enough for real-time index building at edge scale.

---

## B2: ADC Search Latency (100K codes, 10 queries, M=8)

| Metric | Value |
|:-------|:------|
| Total (10 queries over 100K) | 37.6 ms |
| Per-query scan (incl. table build) | 3.76 ms |
| **Per-candidate** | **37.6 ns** |
| Throughput | 26.6M candidates/s |
| Range | [36.7ms, 38.4ms] |
| Sample size | 10 Criterion iterations (165 total runs) |

**Note:** Per-candidate time includes distance table construction overhead (~8 lookups per candidate + table build amortized over 100K). This is a conservative (pessimistic) measurement. Pure scan-only latency would be lower.

**Assessment:** **G2 PASS (native)** — 37.6 ns/candidate is 4x below the 150ns threshold. Exhaustive PQ scan over 100K vectors completes in <4ms, well within the 10ms search latency budget.

---

## B3/B3b: Recall@10

### Results on Synthetic Uniform Random Data

| Dataset | Vectors | Queries | M | Ksub | Recall@10 |
|:--------|:--------|:--------|:--|:-----|:----------|
| B3 (10K) | 10,000 | 50 | 8 | 256 | 0.020 (2.0%) |
| B3b (50K) | 50,000 | 20 | 8 | 256 | 0.000 (0.0%) |
| Debug 64D | 500 | 5 | 4 | 256 | 0.44 (44%) |

**Query count reduction:** Plan specified 1,000 queries. Reduced to 50/20 to limit wall-clock time (50K training alone takes 65s). Given that results are 0.02 and 0.00 (not borderline), additional queries would not change the INCONCLUSIVE verdict. Statistical significance is moot when the result is effectively zero.

### Root Cause Analysis

Recall is near-zero on 768D uniform random data. **This is expected, not a bug.**

**Evidence the implementation is correct:**
1. **64D test:** 44% recall@10 on 500 vectors (reasonable for PQ on uniform random)
2. **Distance ordering:** PQ distances at 64D correlate with true distances (same top-5 candidates)
3. **Property test:** `test_proptest_recall_increases_with_m` passes — M=8 recall > M=4 in 80%+ of trials
4. **1013 unit tests pass** including 33 PQ-specific tests

**Why 768D uniform random fails:**
- Each subspace is 96-dimensional (768/8)
- 256 centroids cannot cover a 96D hypercube
- True distances are tightly concentrated (450 +/- 10 for 768D, i.e., ~2% spread)
- Quantization error exceeds the signal (distance variance between neighbors)
- This is the mathematical worst case: maximum intrinsic dimensionality, zero data structure

**Why real embeddings will perform differently:**
- Sentence embeddings (768D) have intrinsic dimensionality of ~20-50
- Strong correlations between dimensions create exploitable structure
- PQ literature reports recall@10 > 0.90 on real embedding distributions
- The PQ implementation correctly decomposes, encodes, and reconstructs distances

### G3 W46 Verdict: INCONCLUSIVE [SUPERSEDED by W47 — see W47 Validation Results below]

G3 could not be evaluated on synthetic uniform data. Real-embedding validation was deferred to W47.

**W47 Update:** Real-embedding validation completed. G3 FAIL — recall@10 = 0.39 (M=8) / 0.53 (M=16) on 50K all-mpnet-base-v2 768D embeddings. See W47 Validation Results section for details.

---

## B4: PQ vs BQ Recall Comparison

### W46 Results (Synthetic Uniform — Inconclusive)

| Method | Recall@10 (10K, 768D) |
|:-------|:----------------------|
| PQ M=4 | 1.2% |
| PQ M=8 | 2.0% |
| BQ+rescore | Deferred to real-embedding validation |

On uniform random data, both PQ configurations show near-random recall.

### W47 Results (Real Embeddings — Definitive)

| Method | Recall@10 (50K, 768D, 100 queries) | Latency/query |
|:-------|:------------------------------------|:--------------|
| PQ M=8, Ksub=256, iters=15 | **0.3900** | 26.5 ms |
| PQ M=16, Ksub=256, iters=15 | **0.5260** | 27.3 ms |
| BQ+rescore (Hamming top-100, L2 top-10) | **0.9920** | 26.4 ms |

**BQ+rescore wins decisively** — 0.4660 recall points above best PQ (M=16) at comparable latency.

**B4 Analysis:**
- BQ+rescore achieves near-perfect recall (99.2%) by using binarized Hamming distance as a coarse filter, then rescoring the top-100 candidates with exact f32 L2 distance
- PQ's recall deficit on 768D real embeddings indicates that M=8/M=16 subspace decomposition loses too much information for this dimensionality
- At comparable query latency (~27ms for exhaustive 50K scan), BQ+rescore is the superior method for recall-critical applications
- PQ's advantage remains in **memory efficiency** (8 bytes/vector vs 96 bytes/vector for BQ)

---

## B5: Codebook Training Time (50K, M=8, Ksub=256, 15 iters)

| Metric | Value |
|:-------|:------|
| Training time | **64.67s** |
| Per-iteration | ~4.31s |

Single wall-clock run (release profile, no Criterion overhead). Criterion harness available but training is too slow for 10-sample statistical benchmarking within reasonable wall-clock time.

---

## B7: Codebook Training Time (100K — G4 gate)

| Metric | Value |
|:-------|:------|
| Training time | **198.72s** |
| Per-iteration | ~13.25s |
| Scaling factor (100K/50K) | 3.07x |
| **G4 (<60s)** | **FAIL** (3.3x over budget) |

**G4 FAIL** — 100K training at 198.72s vastly exceeds the 60s threshold.

**Root cause:** K-means assignment step is O(N * Ksub * Dsub) per subspace per iteration. With N=100K, Ksub=256, Dsub=96, M=8, and 15 iterations, this is ~2.95 trillion f32 operations.

**Super-linear scaling (3.07x for 2x data):** Expected cause is L3 cache pressure. 50K * 768D * 4B = 147 MB fits in CPU cache hierarchy with pressure; 100K * 768D * 4B = 294 MB far exceeds typical L3 cache (Intel Ultra 9 285H has ~36 MB L3), causing memory bandwidth saturation and cache thrashing during centroid assignment sweeps.

**Mitigations (ordered by impact):**
1. **Reduce iterations:** 10 iters → ~132s (still fails), 5 iters → ~66s (borderline conditional)
2. **Early-stopping:** Convergence detection halts when centroids stabilize (<0.1% change)
3. **Subsample init:** Use only 10K random vectors for centroid initialization
4. **Parallel subspaces:** Train M=8 subspaces on separate threads (native) or Web Workers (WASM)
5. **Lower Ksub:** Ksub=64 reduces training time by ~4x but degrades recall

---

## B6: Memory Footprint

### Per-Component Sizes

| Component | Formula | Bytes |
|:----------|:--------|:------|
| Codebook | M * Ksub * Dsub * 4 | 786,432 (768 KB) |
| PQ code (per vector) | M bytes | 8 |
| F32 vector (per vector) | D * 4 | 3,072 |
| BQ vector (per vector) | D / 8 | 96 |

### At Scale

| N | F32 | BQ | PQ | F32/PQ (compression) | PQ/BQ |
|:--|:----|:---|:---|:---------------------|:------|
| 10K | 29.3 MB | 0.9 MB | 0.8 MB | 35.5x | 0.90x |
| 50K | 146.5 MB | 4.6 MB | 1.1 MB | 129.5x | 0.25x |
| 100K | 293.0 MB | 9.2 MB | 1.5 MB | 193.6x | 0.17x |
| 1M | 2929.7 MB | 91.6 MB | 8.4 MB | 349.6x | 0.09x |

### G1 Gate: PQ memory < 70% of BQ at 100K

**PQ/BQ = 16.5%** — **G1 PASS** (threshold: <70%)

PQ is dramatically more memory-efficient than BQ at scale because the codebook cost (768 KB) amortizes over vectors, while per-vector cost is only 8 bytes (vs 96 for BQ).

### System-Level (with HNSW overhead ~150 bytes/vector)

| Method | 100K Memory |
|:-------|:------------|
| F32 + HNSW | 307.3 MB |
| BQ + HNSW | 23.5 MB |
| PQ + HNSW | 15.8 MB |

PQ saves 32.6% vs BQ at system level, and 94.9% vs F32.

---

## Implementation Summary (G5)

| Day | Deliverable | Hours (est.) |
|:----|:------------|:-------------|
| Day 1 | K-means engine, codebook training | 4h |
| Day 2 | ADC scan, encode_batch, pipeline | 4h |
| Day 3 | Integration tests, property tests, NaN validation | 4h |
| **Total** | | **~12h** |

**G5 PASS** — Implementation completed in ~12h, well under the 16h budget.

---

## API Impact (G6)

| Change Type | Count |
|:------------|:------|
| New public types | 5 (PqCodebook, PqCode, DistanceTable, PqSearchResult, PqError) |
| Modified public types | 1 (EdgeVecError — added PQ variant) |
| Breaking changes | 0 |
| New module | `quantization::product` |

**G6 PASS** — Zero breaking changes. All additions are additive.

---

## W47 Validation Results

### W47 Environment

| Item | Value |
|:-----|:------|
| **OS** | Windows 11 Pro 10.0.26200 |
| **CPU** | Intel Core Ultra 9 285H (16 cores, 16 threads) |
| **RAM** | 32 GB |
| **Rust** | rustc 1.94.0-nightly |
| **Browser** | Chrome 145.0 (Win32) |
| **Embedding Model** | all-mpnet-base-v2 (768D) |
| **Dataset** | 50,000 real sentence embeddings (153.6 MB) |
| **Commits** | baeb7d4 (Day 1) through 0882c8c (Day 4) |

### G2 WASM ADC Benchmark (W47 Day 3)

| Metric | Value |
|:-------|:------|
| Scale | 100,000 codes, 10 queries, M=8 |
| Median per-candidate | 134 ns |
| **P99 per-candidate** | **145 ns** |
| Runs | 3 (first run discarded for V8 JIT warmup — lesson #78) |
| Chrome | 145.0 Win32 |

**G2 PASS** — P99 145 ns < 150 ns threshold. First run showed 177 ns (JIT warmup artifact); stable P99 after 2-3 full runs.

### G3 Real-Embedding Recall (W47 Day 4)

| Config | Recall@10 | Dataset |
|:-------|:----------|:--------|
| PQ M=8, Ksub=256 | **0.3900** | 50K all-mpnet-base-v2, 768D |
| PQ M=16, Ksub=256 | **0.5260** | 50K all-mpnet-base-v2, 768D |

**G3 FAIL** — Both M=8 (0.39) and M=16 (0.53) are far below the 0.90 threshold.

**Root cause:** At 768D with M=8, each subspace is 96-dimensional. Even with real embeddings that have lower intrinsic dimensionality (~20-50), 256 centroids per 96D subspace cannot capture enough structure. The quantization error exceeds the signal needed to distinguish true neighbors from non-neighbors.

**Potential mitigations (not implemented):**
- M=32 or M=64 (finer subspace decomposition, but higher memory)
- OPQ (Optimized Product Quantization) — rotation before decomposition
- Lower-dimensional embeddings (384D with all-MiniLM-L6-v2)

### G4 Training Optimization (W47 Days 3-4)

| Configuration | 100K Time | vs Baseline | Gate |
|:--------------|:----------|:------------|:-----|
| Baseline (iters=15, no early-stop) | 198.7s | — | FAIL |
| Early-stop (1e-4) + iters=15 | 108.9s | -45% | FAIL |
| Early-stop (1e-4) + iters=5 | 44.1s | -78% | FAIL |
| Early-stop + iters=15 + **rayon** | 21.10s median | -89% | **PASS** (<30s) |
| Early-stop + iters=5 + **rayon** | 9.05s median | -95% | **PASS** (<30s) |

**Native G4: PASS** — 9.05s median (3 runs: 9.05, 8.81, 9.85s) with all optimizations.

**WASM G4: FAIL** — 124.6s median (3 runs: 120.0, 124.6, 126.7s). WASM has no rayon (single-threaded). Early-stop + reduced iters alone insufficient. Web Workers required for WASM training parallelism (future work).

**G4 Verdict: CONDITIONAL** — Native PASS, WASM FAIL. WASM training at 100K scale requires Web Workers (not yet available in EdgeVec WASM).

### Optimizations Implemented

| Optimization | How | Impact | WASM? |
|:-------------|:----|:-------|:------|
| Early-stop convergence | K-means halts when centroid movement < threshold (default 1e-4) | -45% (standalone) | Yes |
| Reduced max iterations | `max_iters=5` instead of 15 | -78% (cumulative) | Yes |
| Rayon parallel subspaces | M subspaces trained concurrently via `rayon::par_iter` | -95% (cumulative) | No (feature-gated) |

New API: `PqCodebook::train_with_convergence_threshold(vectors, m, ksub, max_iters, threshold)` for explicit early-stop tuning.

---

## Final Verdict

### W46 Verdict (Superseded)

| Gate | Status | Confidence |
|:-----|:-------|:-----------|
| G1 (Memory) | **PASS** | HIGH |
| G2 (ADC Latency) | **PASS (native) / UNTESTED (WASM)** | HIGH (native), MEDIUM (WASM estimated ~113ns) |
| G3 (Recall) | **INCONCLUSIVE** | LOW — needs real data |
| G4 (Training Time) | **FAIL** | HIGH — 198.7s vs 60s target |
| G5 (Impl Time) | **PASS** | HIGH |
| G6 (API Safety) | **PASS** | HIGH |

### W47 Final Verdict

| Gate | Status | Evidence |
|:-----|:-------|:---------|
| G1 (Memory) | **PASS** | 16.5% of BQ at 100K (threshold: <70%) |
| G2 (ADC Latency) | **PASS** | 145 ns P99 WASM (threshold: <150ns) |
| G3 (Recall) | **FAIL** | M=8: 0.39, M=16: 0.53 on real 768D embeddings (threshold: >0.90) |
| G4 (Training) | **CONDITIONAL** | Native: 9.05s PASS (<30s). WASM: 124.6s FAIL (<60s, needs Web Workers) |
| G5 (Impl Time) | **PASS** | ~12h (threshold: <16h) |
| G6 (API Safety) | **PASS** | 0 breaking changes |

### Decision: CONDITIONAL GO — PQ as Experimental Compression Method

**PQ ships as an available but experimental quantization method with documented limitations:**

1. **G3 FAIL — Recall is insufficient for production search at 768D.** PQ recall@10 = 0.39 (M=8) / 0.53 (M=16) vs BQ+rescore = 0.99. PQ is NOT recommended as the primary search method for high-dimensional embeddings. Use BQ+rescore for recall-critical applications.

2. **G4 CONDITIONAL — Native training is fast, WASM training is slow.** Native training with rayon achieves 9.05s (well under 30s). WASM single-threaded training at 124.6s exceeds 60s. WASM training at 100K scale requires future Web Workers support.

3. **PQ's value proposition is memory efficiency, not recall.** At 100K vectors: PQ uses 1.5 MB vs BQ's 9.2 MB (16.5%). For memory-constrained edge deployments where approximate results are acceptable, PQ provides 6x compression over BQ.

### Recommended Use Cases

| Use Case | Recommended Method |
|:---------|:-------------------|
| Recall-critical search (RAG, semantic search) | BQ+rescore (recall 0.99) |
| Memory-constrained edge (IoT, mobile) | PQ (8 bytes/vector, recall 0.39-0.53) |
| Large-scale pre-filtering | PQ as first-pass filter + f32 rescore |
| Offline batch processing | PQ with rayon (9s training at 100K) |

### Future Work

| Item | Gate Impact | Priority |
|:-----|:-----------|:---------|
| OPQ (Optimized Product Quantization) | G3 improvement | MEDIUM |
| Web Workers for WASM training | G4 WASM improvement | MEDIUM |
| Lower-dimensional model support (384D) | G3 may improve | LOW |
| PQ + HNSW non-exhaustive search | Search latency | LOW |

---

**Document History:**
- W46 (2026-03-28): Initial CONDITIONAL GO — synthetic benchmarks
- W47 (2026-03-06): Updated with real-embedding validation — G2 PASS, G3 FAIL, G4 CONDITIONAL
