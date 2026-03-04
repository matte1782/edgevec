# PQ GO/NO-GO Decision — W46 Benchmark Results

**Date:** 2026-03-28
**Author:** BENCHMARK_SCIENTIST
**Status:** [REVISED]

---

## Environment

| Item | Value |
|:-----|:------|
| **OS** | Windows 11 Pro 10.0.26200 |
| **CPU** | Intel Core Ultra 9 285H (16 cores, 16 threads) |
| **RAM** | 32 GB |
| **Rust** | rustc 1.94.0-nightly (1aa9bab4e 2025-12-05) |
| **Commit** | 7ec4219 (W46 Day 3) |
| **Target** | Native x86_64 (non-WASM — see WASM caveat below) |

---

## Executive Summary

Product Quantization (PQ) implementation completed in W46 Days 1-3 (codebook training, encode, ADC scan, property tests, NaN validation). Days 4-5 benchmark results below.

**Overall Verdict: CONDITIONAL GO** — PQ is architecturally sound and performant on native. G3 (recall) is INCONCLUSIVE on synthetic data. G4 (training time) FAILS. WASM validation deferred to W47.

---

## Gate Results

| Gate | Criterion | Target | Result | Verdict |
|:-----|:----------|:-------|:-------|:--------|
| **G1** | PQ memory < 70% of BQ at 100K | <70% | **16.5%** | **PASS** |
| **G2** | ADC latency < 150ns/candidate | <150ns | **37.6 ns (native)** | **PASS (native) / UNTESTED (WASM)** |
| **G3** | Recall@10 > 0.90 | >0.90 | 0.02 (synthetic) | **INCONCLUSIVE** |
| **G4** | Training < 60s for 100K | <60s | **198.7s (native)** | **FAIL** |
| **G5** | Implementation < 16h | <16h | ~12h (3 days) | **PASS** |
| **G6** | No API breakage | 0 breaking | 0 | **PASS** |

### WASM Caveat

G2 and G4 thresholds in PQ_BENCHMARK_PLAN are defined against WASM targets. All measurements in this report are **native-only**. WASM benchmarks are deferred to W47 (requires Playwright MCP integration for Chrome profiling).

**G2 native headroom analysis:** At 37.6 ns/candidate native, even with a typical 3x WASM overhead, the estimated WASM latency would be ~113 ns — still below the 150 ns threshold. This provides reasonable confidence G2 will pass in WASM, but WASM validation remains a **mandatory W47 action item**.

**G4:** Already FAIL on native (3.3x over budget). WASM will be worse. Training optimizations (see B7 mitigations) are required regardless.

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

### G3 Verdict: INCONCLUSIVE

G3 cannot be evaluated on synthetic uniform data. Real-embedding validation is required before PQ is used for production search. The implementation is architecturally correct and functional.

**Recommended next step:** Validate recall on real sentence-transformer embeddings (e.g., all-MiniLM-L6-v2 at 384D or all-mpnet-base-v2 at 768D) in W47.

---

## B4: PQ vs BQ Recall Comparison

| Method | Recall@10 (10K, 768D) |
|:-------|:----------------------|
| PQ M=4 | 1.2% |
| PQ M=8 | 2.0% |
| BQ+rescore | Deferred to real-embedding validation |

On uniform random data, both PQ configurations show near-random recall. The BQ+rescore comparison (Hamming filter then f32 L2 rescore on top-100) is **deferred to W47** alongside G3 real-embedding validation, where both methods will have non-trivial recall and a meaningful comparison is possible.

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

## Final Verdict

| Gate | Status | Confidence |
|:-----|:-------|:-----------|
| G1 (Memory) | **PASS** | HIGH |
| G2 (ADC Latency) | **PASS (native) / UNTESTED (WASM)** | HIGH (native), MEDIUM (WASM estimated ~113ns) |
| G3 (Recall) | **INCONCLUSIVE** | LOW — needs real data |
| G4 (Training Time) | **FAIL** | HIGH — 198.7s vs 60s target |
| G5 (Impl Time) | **PASS** | HIGH |
| G6 (API Safety) | **PASS** | HIGH |

### Decision: CONDITIONAL GO

**Proceed with PQ as an available quantization method.** The implementation is correct, memory-efficient, and fast at search time. However:

1. **G3 requires real-embedding validation** before PQ search results can be trusted for production use
2. **G4 training time FAILS at 100K** (198.7s vs 60s target) — requires early-stopping, iteration reduction, or parallel subspace training
3. **G2 WASM validation** is a mandatory W47 action item (native result + 4x headroom provides reasonable confidence)
4. **Recommended:** Add a `PqConfig` builder pattern in v0.10.0 that lets users tune M, Ksub, and max_iters

### Mandatory W47 Action Items

| Action | Gate | Priority |
|:-------|:-----|:---------|
| WASM ADC benchmark (Playwright + Chrome) | G2 | HIGH |
| Real-embedding recall validation | G3 | HIGH |
| Training optimization (early-stop + parallel) | G4 | HIGH |
| BQ+rescore comparison on real data | B4 | MEDIUM |

### Risk Mitigation

| Risk | Severity | Mitigation |
|:-----|:---------|:-----------|
| G3 recall low on real data | HIGH | Spike with real embeddings in W47 |
| G4 training 3.3x over budget | HIGH | Parallel subspaces + early-stopping + 5 iters → target ~33s |
| G2 WASM exceeds 150ns | LOW | 4x native headroom; ~3x WASM overhead → ~113ns estimated |
| Codebook size at 768 KB | LOW | Acceptable for edge; compress if needed later |

---

**BENCHMARK_SCIENTIST: Task Complete**

Artifacts generated:
- `docs/benchmarks/PQ_GO_NOGO_DECISION.md` [REVISED]
- `benches/pq_bench.rs`

Status: PENDING_HOSTILE_REVIEW (Round 2)

Next: `/review docs/benchmarks/PQ_GO_NOGO_DECISION.md`
