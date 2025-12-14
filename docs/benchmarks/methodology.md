# Benchmark Methodology

**Document:** EdgeVec Competitive Benchmark Methodology
**Created:** 2025-12-14
**Task:** W13.3c
**Status:** COMPLETE

---

## Overview

This document describes the methodology used for EdgeVec performance benchmarks and competitive analysis. Following these guidelines ensures reproducible and fair comparisons.

---

## Test Environment

### Hardware Requirements

| Component | Minimum | Recommended |
|:----------|:--------|:------------|
| **CPU** | Multi-core x86_64 | Modern Intel/AMD with AVX2 |
| **RAM** | 8 GB | 16 GB+ |
| **Storage** | SSD | NVMe SSD |
| **OS** | Windows 10 / Linux | Windows 11 / Ubuntu 22.04+ |

### Software Requirements

| Component | Version |
|:----------|:--------|
| **Rust** | 1.70+ (MSRV) |
| **Cargo** | Latest stable |
| **Node.js** | 18+ LTS |
| **wasm-pack** | 0.12+ |

### Compiler Configuration

For optimal performance, ensure `.cargo/config.toml` includes:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

Without this, performance will be **60-78% slower** due to missing SIMD optimizations.

---

## Dataset Specifications

### Standard Benchmark Dataset

| Parameter | Value | Rationale |
|:----------|:------|:----------|
| **Dimensions** | 128 | Common embedding size |
| **Vector Count** | 10k / 50k / 100k | Scalability validation |
| **Distribution** | Uniform random [0, 1) | Reproducible baseline |
| **Data Type** | Float32 | Standard precision |

### Query Set

| Parameter | Value |
|:----------|:------|
| **Query Count** | 100-1000 |
| **Distribution** | Uniform random [0, 1) |
| **k (neighbors)** | 10 |

---

## HNSW Parameters

All benchmarks use consistent HNSW parameters unless otherwise noted:

| Parameter | Value | Description |
|:----------|:------|:------------|
| **M** | 16 | Max connections per layer |
| **M0** | 32 | Max connections at layer 0 |
| **ef_construction** | 200 | Build-time search width |
| **ef_search** | 50 | Query-time search width |

---

## Measurement Procedure

### Rust Native Benchmarks

Using [Criterion.rs](https://github.com/bheisler/criterion.rs):

1. **Warmup Phase:** 3 seconds
2. **Measurement Phase:** 10 samples minimum
3. **Statistical Analysis:** Mean, standard deviation, confidence intervals
4. **Outlier Detection:** Automatic via Criterion

```bash
# Run benchmarks
cargo bench --bench search_bench
cargo bench --bench insert_bench
cargo bench --bench scaling_bench
cargo bench --bench persistence_bench
```

### WASM Benchmarks (Competitive)

Using Node.js harness:

1. **Warmup Runs:** 3 iterations (discarded)
2. **Measurement Runs:** 5 iterations
3. **Metrics Collected:** Per-query latency
4. **Statistical Analysis:** P50, P90, P95, P99, P99.9

```bash
cd benches/competitive
npm install
node harness.js
```

---

## Metrics Collected

### Latency Metrics

| Metric | Description | Unit |
|:-------|:------------|:-----|
| **Mean** | Average latency | ms |
| **P50** | Median latency | ms |
| **P90** | 90th percentile | ms |
| **P95** | 95th percentile | ms |
| **P99** | 99th percentile | ms |
| **P99.9** | 99.9th percentile | ms |

### Memory Metrics

| Metric | Description | Unit |
|:-------|:------------|:-----|
| **Per Vector** | Memory per indexed vector | bytes |
| **Total Heap** | Total heap usage after indexing | MB |
| **Bundle Size** | WASM bundle size (gzipped) | KB |

### Quality Metrics

| Metric | Description |
|:-------|:------------|
| **Recall@k** | Fraction of true k-NN found |
| **Build Time** | Time to build index | seconds |
| **Throughput** | Queries per second | QPS |

---

## Recall Calculation

Ground truth is computed via brute-force k-NN:

```javascript
function calculateRecall(predicted, groundTruth) {
    const gtSet = new Set(groundTruth);
    let hits = 0;
    for (const id of predicted) {
        if (gtSet.has(id)) hits++;
    }
    return hits / groundTruth.length;
}
```

**Recall@10** = (true positives in top-10) / 10

---

## Reproducibility Checklist

Before running benchmarks:

- [ ] Close unnecessary applications
- [ ] Disable browser extensions (for browser benchmarks)
- [ ] Ensure stable power (no battery saving mode)
- [ ] Run garbage collection before measurement
- [ ] Document exact software versions
- [ ] Record hardware specifications
- [ ] Use consistent HNSW parameters across libraries

---

## Known Limitations

1. **Single-threaded:** WASM benchmarks are single-threaded
2. **Memory Measurement:** JavaScript heap size is approximate
3. **SharedArrayBuffer:** Not used (browser security restrictions)
4. **SIMD Availability:** Varies by browser/Node.js version

---

## Reporting Results

All benchmark results should include:

1. **Hardware specifications** (CPU model, RAM, OS)
2. **Software versions** (Rust, Node.js, browser)
3. **Build configuration** (release flags, SIMD enabled)
4. **Dataset parameters** (dimensions, count, distribution)
5. **HNSW parameters** (M, ef_construction, ef_search)
6. **Raw data** (JSON files in `results/`)
7. **Commit hash** for reproducibility

---

## Changelog

| Date | Version | Changes |
|:-----|:--------|:--------|
| 2025-12-14 | 1.0 | Initial methodology document (W13.3c) |
