# EdgeVec WASM Batch Insert Benchmarks - Week 12

**Version:** v0.2.0-alpha.2
**Date:** 2025-12-13
**Author:** BENCHMARK_SCIENTIST
**Status:** [PENDING MANUAL TEST]

---

## Executive Summary

This report measures the performance of the WASM `insertBatch` API, focusing on FFI overhead compared to native Rust performance.

| Metric | Target | Status |
|:-------|:-------|:-------|
| FFI overhead | <5% | [PENDING] |
| Memory delta (5000 vectors) | <100MB | [PENDING] |
| Browser compatibility | Chrome, Firefox, Safari | [PENDING] |

---

## Test Environment

### Hardware

| Component | Specification |
|:----------|:--------------|
| CPU | [TO BE FILLED DURING MANUAL TEST] |
| RAM | [TO BE FILLED DURING MANUAL TEST] |
| OS | [TO BE FILLED DURING MANUAL TEST] |

### Software

| Component | Version |
|:----------|:--------|
| Browser | Chrome [VERSION] / Firefox [VERSION] / Safari [VERSION] |
| Node.js | [TO BE FILLED] |
| wasm-pack | 0.12.x |
| EdgeVec | v0.2.0-alpha.2 |

### Build Configuration

```bash
wasm-pack build --target web --release
```

---

## Benchmark Methodology

### Test Configurations

| ID | Vectors | Dimensions | Expected Duration | Memory Budget |
|:---|:--------|:-----------|:------------------|:--------------|
| C1 | 100 | 128 | <50ms | <10MB |
| C2 | 1000 | 128 | <500ms | <50MB |
| C3 | 1000 | 512 | <800ms | <100MB |
| C4 | 5000 | 128 | <2500ms | <100MB |

### FFI Overhead Formula

```
FFI Overhead = (WASM_Time - Rust_Baseline) / Rust_Baseline × 100%
```

### Test Data Generation

```javascript
function generateTestVectors(count, dims) {
    const vectors = [];
    for (let i = 0; i < count; i++) {
        const vec = new Float32Array(dims);
        for (let j = 0; j < dims; j++) {
            vec[j] = Math.random() * 2 - 1;
        }
        // Normalize to unit length
        let norm = 0;
        for (let j = 0; j < dims; j++) norm += vec[j] * vec[j];
        norm = Math.sqrt(norm);
        for (let j = 0; j < dims; j++) vec[j] /= norm;
        vectors.push(vec);
    }
    return vectors;
}
```

---

## Results

### Configuration C1: 100 vectors × 128 dimensions

| Metric | Value | Target | Status |
|:-------|:------|:-------|:-------|
| WASM batch time | [PENDING] | <50ms | [PENDING] |
| Rust baseline | [PENDING] | — | — |
| FFI overhead | [PENDING] | <5% | [PENDING] |
| Memory delta | [PENDING] | <10MB | [PENDING] |

**Browser Results:**

| Browser | Time (ms) | Memory (MB) | Notes |
|:--------|:----------|:------------|:------|
| Chrome | [PENDING] | [PENDING] | |
| Firefox | [PENDING] | [PENDING] | |
| Safari | [PENDING] | [PENDING] | |

---

### Configuration C2: 1000 vectors × 128 dimensions

| Metric | Value | Target | Status |
|:-------|:------|:-------|:-------|
| WASM batch time | [PENDING] | <500ms | [PENDING] |
| Rust baseline | [PENDING] | — | — |
| FFI overhead | [PENDING] | <5% | [PENDING] |
| Memory delta | [PENDING] | <50MB | [PENDING] |

**Browser Results:**

| Browser | Time (ms) | Memory (MB) | Notes |
|:--------|:----------|:------------|:------|
| Chrome | [PENDING] | [PENDING] | |
| Firefox | [PENDING] | [PENDING] | |
| Safari | [PENDING] | [PENDING] | |

---

### Configuration C3: 1000 vectors × 512 dimensions

| Metric | Value | Target | Status |
|:-------|:------|:-------|:-------|
| WASM batch time | [PENDING] | <800ms | [PENDING] |
| Rust baseline | [PENDING] | — | — |
| FFI overhead | [PENDING] | <5% | [PENDING] |
| Memory delta | [PENDING] | <100MB | [PENDING] |

**Browser Results:**

| Browser | Time (ms) | Memory (MB) | Notes |
|:--------|:----------|:------------|:------|
| Chrome | [PENDING] | [PENDING] | |
| Firefox | [PENDING] | [PENDING] | |
| Safari | [PENDING] | [PENDING] | |

---

### Configuration C4: 5000 vectors × 128 dimensions

| Metric | Value | Target | Status |
|:-------|:------|:-------|:-------|
| WASM batch time | [PENDING] | <2500ms | [PENDING] |
| Rust baseline | [PENDING] | — | — |
| FFI overhead | [PENDING] | <5% | [PENDING] |
| Memory delta | [PENDING] | <100MB | [PENDING] |

**Browser Results:**

| Browser | Time (ms) | Memory (MB) | Notes |
|:--------|:----------|:------------|:------|
| Chrome | [PENDING] | [PENDING] | |
| Firefox | [PENDING] | [PENDING] | |
| Safari | [PENDING] | [PENDING] | |

---

## FFI Overhead Summary

| Config | Vectors | Dims | WASM Time | Rust Baseline | FFI Overhead | Target | Status |
|:-------|:--------|:-----|:----------|:--------------|:-------------|:-------|:-------|
| C1 | 100 | 128 | [PENDING] | [PENDING] | [PENDING] | <5% | [PENDING] |
| C2 | 1000 | 128 | [PENDING] | [PENDING] | [PENDING] | <5% | [PENDING] |
| C3 | 1000 | 512 | [PENDING] | [PENDING] | [PENDING] | <5% | [PENDING] |
| C4 | 5000 | 128 | [PENDING] | [PENDING] | [PENDING] | <5% | [PENDING] |

### Expected FFI Costs

1. **Array copying:** JavaScript → WASM linear memory (~2-3%)
2. **Type conversion:** f64 → f32 if needed (~0-1%)
3. **Call overhead:** JS ↔ WASM boundary (<1%)

**Total Expected:** <5%

---

## Memory Analysis

### Per-Vector Memory (Expected)

```
Memory per vector = (dims × 4 bytes) + index_overhead
Index overhead ≈ 52 bytes (from HNSW graph structure)

For 128 dims: 512 + 52 = 564 bytes/vector
For 512 dims: 2048 + 52 = 2100 bytes/vector
```

### Memory Delta by Configuration

| Config | Vectors | Expected | Actual | Δ | Target | Status |
|:-------|:--------|:---------|:-------|:--|:-------|:-------|
| C1 | 100 | ~55KB | [PENDING] | [PENDING] | <10MB | [PENDING] |
| C2 | 1000 | ~550KB | [PENDING] | [PENDING] | <50MB | [PENDING] |
| C3 | 1000 | ~2.1MB | [PENDING] | [PENDING] | <100MB | [PENDING] |
| C4 | 5000 | ~2.8MB | [PENDING] | [PENDING] | <100MB | [PENDING] |

---

## Browser Compatibility

### Chrome

| Feature | Status | Notes |
|:--------|:-------|:------|
| WebAssembly | [PENDING] | Native support since Chrome 57 |
| ES6 Modules | [PENDING] | Native support |
| Performance API | [PENDING] | Full support including memory |

### Firefox

| Feature | Status | Notes |
|:--------|:-------|:------|
| WebAssembly | [PENDING] | Native support since Firefox 52 |
| ES6 Modules | [PENDING] | Native support |
| Performance API | [PENDING] | Memory API may be limited |

### Safari

| Feature | Status | Notes |
|:--------|:-------|:------|
| WebAssembly | [PENDING] | Native support since Safari 11 |
| ES6 Modules | [PENDING] | Native support |
| Performance API | [PENDING] | Memory API not available |

---

## How to Run Benchmarks

### 1. Build WASM Module

```bash
cd edgevec
wasm-pack build --target web --release
```

### 2. Start Local Server

```bash
python -m http.server 8000
# or
npx http-server -p 8000
```

### 3. Open Demo Page

```
http://localhost:8000/wasm/examples/batch_insert.html
```

### 4. Run Tests

1. Set "Number of Vectors" to match configuration (100, 1000, 5000)
2. Set "Vector Dimensions" to match configuration (128, 512)
3. Click "Run Comparison" button
4. Record times from results panel
5. Open DevTools → Memory tab for memory measurements
6. Repeat for each browser

### 5. Record Results

Fill in [PENDING] values in this document with actual measurements.

---

## Acceptance Criteria Verification

- [x] **AC5.1:** Report created at `docs/benchmarks/week_12_wasm_batch.md`
- [x] **AC5.2:** All 4 configurations documented (C1-C4)
- [x] **AC5.3:** FFI overhead target stated (<5%)
- [x] **AC5.4:** Memory delta target stated (<100MB for 5000 vectors)
- [x] **AC5.5:** Environment section present

**Template Status:** COMPLETE — Ready for manual browser testing

---

## Conclusions

[TO BE FILLED AFTER MANUAL TESTING]

### Key Findings

1. **FFI Overhead:** [PENDING]
2. **Memory Efficiency:** [PENDING]
3. **Browser Performance:** [PENDING]
4. **Recommendations:** [PENDING]

---

## Appendix: Rust Baseline Reference

To obtain Rust baseline times, run:

```bash
cargo bench --bench batch_insert
```

Or use the validation benchmarks:

```bash
cargo test --release bench_ -- --nocapture
```

---

*Report Version: 1.0.0 (Template)*
*Agent: BENCHMARK_SCIENTIST*
*Status: PENDING_MANUAL_TEST*
