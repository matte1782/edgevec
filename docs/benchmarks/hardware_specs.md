# Benchmark Hardware Specifications

**Document:** Competitive Benchmark Environment
**Created:** 2025-12-14
**Task:** W13.3a

---

## Test Environment

### Hardware

| Component | Specification |
|:----------|:--------------|
| **CPU** | [To be filled during benchmark run] |
| **Cores/Threads** | [To be filled] |
| **RAM** | [To be filled] |
| **Storage** | [To be filled] |

### Software

| Component | Version |
|:----------|:--------|
| **OS** | Windows 11 |
| **Node.js** | v18+ (LTS) |
| **Browser** | Chrome (latest) |
| **WASM Engine** | V8 (via Chrome/Node) |
| **Rust** | 1.70+ (MSRV) |
| **wasm-pack** | 0.12+ |

### EdgeVec Build

| Property | Value |
|:---------|:------|
| **Version** | 0.2.0-alpha.2 |
| **Commit** | [To be filled at benchmark time] |
| **Build Profile** | Release (--release) |
| **WASM Target** | wasm32-unknown-unknown |
| **Optimizations** | LTO enabled, opt-level=3 |

---

## Benchmark Configuration

### Dataset: SIFT-like Vectors

| Parameter | Value |
|:----------|:------|
| **Dimensions** | 128 |
| **Vector Count** | 10,000 / 100,000 (scalability test) |
| **Data Type** | Float32 |
| **Distribution** | Uniform random [0, 1) |

### HNSW Parameters

| Parameter | EdgeVec Default | Notes |
|:----------|:----------------|:------|
| **M** | 16 | Max connections per layer |
| **M0** | 32 | Max connections at layer 0 |
| **ef_construction** | 200 | Build-time search width |
| **ef_search** | 50 | Query-time search width |

### Metrics Collected

| Metric | Unit | Collection Method |
|:-------|:-----|:------------------|
| **Insert Latency** | ms/vector | Mean, P50, P99 |
| **Search Latency** | ms/query | Mean, P50, P99 |
| **Recall@10** | % | Ground truth comparison |
| **Memory Usage** | MB | Before/after measurement |
| **Bundle Size** | KB | gzip compressed |

---

## Competitor Libraries

| Library | Version | WASM Support | Notes |
|:--------|:--------|:-------------|:------|
| **hnswlib-wasm** | [TBD] | Yes | Reference HNSW implementation |
| **voy** | [TBD] | Yes | WASM vector search |
| **usearch** | [TBD] | Yes | Fast approximate search |
| **vectra** | [TBD] | JS-native | Pure JS baseline |

---

## Reproducibility Notes

### Pre-Benchmark Checklist

- [ ] Close all unnecessary applications
- [ ] Disable browser extensions
- [ ] Run garbage collection before each test
- [ ] Warm-up runs (3x) before measurement
- [ ] Multiple measurement runs (5x minimum)

### Environment Variables

```bash
# Recommended Node.js flags
export NODE_OPTIONS="--max-old-space-size=4096"
```

### Known Limitations

1. **WASM Memory:** Default 256MB, expandable to ~1GB (Safari limit)
2. **SharedArrayBuffer:** Not used (single-threaded benchmarks)
3. **SIMD:** Enabled when available (Chrome 91+, Firefox 89+)

---

## Test Date

**Benchmark Run:** [To be filled when benchmarks are executed]

---

## Changelog

| Date | Change |
|:-----|:-------|
| 2025-12-14 | Initial template created (W13.3a) |

---

*This document will be updated with actual values when benchmarks are run.*
