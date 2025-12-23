# Week 28 Day 2: BQ WASM Bindings

**Date:** 2025-12-24
**Focus:** Expose Binary Quantization through WASM API
**Estimated Duration:** 8 hours
**Phase:** RFC-002 Implementation Phase 3 (WASM & Integration)
**Dependencies:** Week 27 (Binary Quantization Rust API) — COMPLETE

---

## Tasks

### W28.2.1: `searchBQ()` WASM Binding

**Objective:** Enable JavaScript to perform fast BQ search using Hamming distance.

**Rust Implementation:**

```rust
// src/wasm/bq.rs

use wasm_bindgen::prelude::*;
use serde::Serialize;

#[wasm_bindgen]
impl WasmIndex {
    /// Search using binary quantization (fast, approximate).
    ///
    /// Binary quantization converts vectors to bit arrays and uses
    /// Hamming distance for comparison. This is ~3-5x faster than
    /// F32 search but may have lower recall.
    ///
    /// # Arguments
    /// * `query` - Float32Array query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    /// Array of SearchResult sorted by Hamming distance
    ///
    /// # Requirements
    /// - Index must be created with BQ enabled
    /// - Query dimension must match index dimension
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const results = index.searchBQ(query, 10);
    /// // Returns quickly, but may miss some relevant results
    /// ```
    #[wasm_bindgen(js_name = searchBQ)]
    pub fn search_bq(
        &self,
        query: &[f32],
        k: usize,
    ) -> Result<JsValue, JsValue> {
        // Validate BQ is enabled
        if !self.index.has_bq() {
            return Err(JsValue::from_str(
                "Binary quantization not enabled. Create index with { useBQ: true }"
            ));
        }

        // Validate query dimension
        if query.len() != self.config.dimension as usize {
            return Err(JsValue::from_str(&format!(
                "Query dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.len()
            )));
        }

        // Validate k
        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Execute BQ search
        let results = self.index
            .search_bq(query, k, &self.storage)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JavaScript format
        let js_results: Vec<JsSearchResult> = results
            .into_iter()
            .map(|(id, distance)| JsSearchResult {
                id: id.0,
                distance,
            })
            .collect();

        serde_wasm_bindgen::to_value(&js_results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Search using binary quantization (fast, approximate).
 *
 * Binary quantization converts vectors to bit arrays (1 bit per dimension)
 * and uses Hamming distance for comparison. This provides:
 * - ~32x memory reduction
 * - ~3-5x faster search
 * - ~70-85% recall (use searchBQRescored for higher recall)
 *
 * @param query Float32Array query vector
 * @param k Number of results to return
 * @returns Array of SearchResult sorted by Hamming distance (converted to similarity)
 * @throws Error if BQ not enabled or dimension mismatch
 *
 * @example
 * ```js
 * // Fast search, lower recall
 * const results = index.searchBQ(query, 10);
 * ```
 */
searchBQ(query: Float32Array, k: number): SearchResult[];
```

**Acceptance Criteria:**
- [ ] `searchBQ()` exported via wasm-bindgen
- [ ] Returns results sorted by Hamming similarity
- [ ] Throws error if BQ not enabled
- [ ] Throws error on dimension mismatch
- [ ] Performance is faster than F32 search

**Test Cases:**

```javascript
// tests/wasm/bq_search.js

describe('searchBQ', () => {
    let index;

    beforeEach(async () => {
        index = new WasmIndex({ dimensions: 768, useBQ: true });

        // Insert test data
        for (let i = 0; i < 1000; i++) {
            const vector = new Float32Array(768).map(() => Math.random() * 2 - 1);
            index.insert(vector);
        }
    });

    it('should return k results', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);
        const results = index.searchBQ(query, 10);

        expect(results.length).toBe(10);
    });

    it('should be faster than F32 search', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        // Warm up
        index.search(query, 10);
        index.searchBQ(query, 10);

        // Time F32 search
        const f32Start = performance.now();
        for (let i = 0; i < 100; i++) {
            index.search(query, 10);
        }
        const f32Time = performance.now() - f32Start;

        // Time BQ search
        const bqStart = performance.now();
        for (let i = 0; i < 100; i++) {
            index.searchBQ(query, 10);
        }
        const bqTime = performance.now() - bqStart;

        // BQ should be at least 1.5x faster (conservative for WASM)
        expect(bqTime).toBeLessThan(f32Time * 0.7);
    });

    it('should throw if BQ not enabled', async () => {
        const nonBqIndex = new WasmIndex({ dimensions: 768, useBQ: false });
        const vector = new Float32Array(768).fill(0.1);
        nonBqIndex.insert(vector);

        const query = new Float32Array(768).fill(0.1);
        expect(() => nonBqIndex.searchBQ(query, 10))
            .toThrow(/not enabled/i);
    });
});
```

**Estimated Duration:** 3 hours

**Agent:** WASM_SPECIALIST

---

### W28.2.2: `searchBQRescored()` WASM Binding

**Objective:** Provide BQ search with F32 rescoring for high recall.

**Rust Implementation:**

```rust
// src/wasm/bq.rs (continued)

#[wasm_bindgen]
impl WasmIndex {
    /// Search using BQ with F32 rescoring (fast + accurate).
    ///
    /// This method:
    /// 1. Uses BQ to quickly find k * rescoreFactor candidates
    /// 2. Rescores candidates using exact F32 distance
    /// 3. Returns the final top-k results
    ///
    /// This provides near-F32 recall (~95%) with most of the BQ speedup.
    ///
    /// # Arguments
    /// * `query` - Float32Array query vector
    /// * `k` - Number of results to return
    /// * `rescore_factor` - Overfetch multiplier (3-10 recommended)
    ///
    /// # Returns
    /// Array of SearchResult sorted by F32 distance
    ///
    /// # Example (JavaScript)
    /// ```js
    /// // Fast search with high recall
    /// const results = index.searchBQRescored(query, 10, 5);
    /// // ~95% recall, still ~2-3x faster than F32
    /// ```
    #[wasm_bindgen(js_name = searchBQRescored)]
    pub fn search_bq_rescored(
        &self,
        query: &[f32],
        k: usize,
        rescore_factor: usize,
    ) -> Result<JsValue, JsValue> {
        // Validate BQ is enabled
        if !self.index.has_bq() {
            return Err(JsValue::from_str(
                "Binary quantization not enabled. Create index with { useBQ: true }"
            ));
        }

        // Validate query dimension
        if query.len() != self.config.dimension as usize {
            return Err(JsValue::from_str(&format!(
                "Query dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.len()
            )));
        }

        // Validate parameters
        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }
        if rescore_factor == 0 {
            return Err(JsValue::from_str("rescoreFactor must be greater than 0"));
        }

        // Execute rescored search
        let results = self.index
            .search_bq_rescored(query, k, rescore_factor, &self.storage)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JavaScript format
        let js_results: Vec<JsSearchResult> = results
            .into_iter()
            .map(|(id, distance)| JsSearchResult {
                id: id.0,
                distance,
            })
            .collect();

        serde_wasm_bindgen::to_value(&js_results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Search using BQ with F32 rescoring (fast + accurate).
 *
 * Combines BQ speed with F32 accuracy:
 * 1. Uses BQ to quickly find `k * rescoreFactor` candidates
 * 2. Rescores candidates using exact F32 distance
 * 3. Returns the final top-k results
 *
 * @param query Float32Array query vector
 * @param k Number of results to return
 * @param rescoreFactor Overfetch multiplier (3-10 recommended)
 * @returns Array of SearchResult sorted by F32 distance
 * @throws Error if BQ not enabled or dimension mismatch
 *
 * @example
 * ```js
 * // Fast search with high recall (~95%)
 * const results = index.searchBQRescored(query, 10, 5);
 * ```
 *
 * ## Rescore Factor Guide
 * | Factor | Recall | Relative Speed |
 * |--------|--------|----------------|
 * | 1      | ~70%   | 5x             |
 * | 3      | ~90%   | 3x             |
 * | 5      | ~95%   | 2.5x           |
 * | 10     | ~98%   | 2x             |
 */
searchBQRescored(query: Float32Array, k: number, rescoreFactor: number): SearchResult[];
```

**Acceptance Criteria:**
- [ ] `searchBQRescored()` exported via wasm-bindgen
- [ ] Returns results sorted by F32 distance
- [ ] Higher recall than raw BQ search
- [ ] Faster than pure F32 search
- [ ] rescore_factor affects recall/speed tradeoff

**Test Cases:**

```javascript
// tests/wasm/bq_rescore.js

describe('searchBQRescored', () => {
    let index;
    const numVectors = 1000;

    beforeEach(async () => {
        index = new WasmIndex({ dimensions: 768, useBQ: true });

        for (let i = 0; i < numVectors; i++) {
            const vector = new Float32Array(768).map(() => Math.random() * 2 - 1);
            index.insert(vector);
        }
    });

    it('should have higher recall than raw BQ', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        // Ground truth: F32 search
        const f32Results = index.search(query, 10);
        const f32Ids = new Set(f32Results.map(r => r.id));

        // BQ search
        const bqResults = index.searchBQ(query, 10);
        const bqIds = new Set(bqResults.map(r => r.id));

        // BQ rescored search
        const rescoredResults = index.searchBQRescored(query, 10, 5);
        const rescoredIds = new Set(rescoredResults.map(r => r.id));

        // Calculate recall
        const bqRecall = [...bqIds].filter(id => f32Ids.has(id)).length / 10;
        const rescoredRecall = [...rescoredIds].filter(id => f32Ids.has(id)).length / 10;

        // Rescored should have higher recall
        expect(rescoredRecall).toBeGreaterThanOrEqual(bqRecall);
        // And should achieve >90% recall with factor=5
        expect(rescoredRecall).toBeGreaterThanOrEqual(0.9);
    });

    it('should be faster than F32', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        // Time F32
        const f32Start = performance.now();
        for (let i = 0; i < 50; i++) {
            index.search(query, 10);
        }
        const f32Time = performance.now() - f32Start;

        // Time rescored
        const rescoredStart = performance.now();
        for (let i = 0; i < 50; i++) {
            index.searchBQRescored(query, 10, 5);
        }
        const rescoredTime = performance.now() - rescoredStart;

        // Should be at least 1.3x faster (conservative for WASM + rescoring)
        expect(rescoredTime).toBeLessThan(f32Time * 0.8);
    });
});
```

**Estimated Duration:** 3 hours

**Agent:** WASM_SPECIALIST

---

### W28.2.3: `searchHybrid()` WASM Binding

**Objective:** Combine BQ speed with metadata filtering.

**Rust Implementation:**

```rust
// src/wasm/bq.rs (continued)

#[wasm_bindgen]
impl WasmIndex {
    /// Hybrid search combining BQ speed with metadata filtering.
    ///
    /// This method:
    /// 1. Uses BQ for fast candidate generation
    /// 2. Applies metadata filter to candidates
    /// 3. Optionally rescores with F32 for accuracy
    ///
    /// Best for: Large datasets where you need both speed and filtering.
    ///
    /// # Arguments
    /// * `query` - Float32Array query vector
    /// * `options` - HybridSearchOptions object
    ///
    /// # Returns
    /// Array of SearchResult matching filter, sorted by distance
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const results = index.searchHybrid(query, {
    ///     k: 10,
    ///     filter: 'category == "news" AND score > 0.5',
    ///     useBQ: true,
    ///     rescoreFactor: 3
    /// });
    /// ```
    #[wasm_bindgen(js_name = searchHybrid)]
    pub fn search_hybrid(
        &self,
        query: &[f32],
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        // Deserialize options
        let opts: HybridSearchOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsValue::from_str(&format!("Invalid options: {}", e)))?;

        // Validate query dimension
        if query.len() != self.config.dimension as usize {
            return Err(JsValue::from_str(&format!(
                "Query dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.len()
            )));
        }

        // Validate k
        if opts.k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Determine search strategy
        let use_bq = opts.use_bq.unwrap_or(true) && self.index.has_bq();
        let rescore_factor = opts.rescore_factor.unwrap_or(3);

        let results = if use_bq {
            // BQ + filter + optional rescore
            if let Some(ref filter) = opts.filter {
                self.index.search_bq_filtered_rescored(
                    query,
                    filter,
                    opts.k,
                    rescore_factor,
                    &self.storage,
                )
            } else {
                self.index.search_bq_rescored(query, opts.k, rescore_factor, &self.storage)
            }
        } else {
            // F32 + filter
            if let Some(ref filter) = opts.filter {
                self.index.search_filtered(query, filter, opts.k, &self.storage)
            } else {
                self.index.search(query, opts.k, &self.storage)
                    .map(|results| results.into_iter().map(|r| (r.vector_id, r.distance)).collect())
            }
        }.map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JavaScript format
        let js_results: Vec<JsSearchResult> = results
            .into_iter()
            .map(|(id, distance)| JsSearchResult {
                id: id.0,
                distance,
            })
            .collect();

        serde_wasm_bindgen::to_value(&js_results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Options for hybrid search.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HybridSearchOptions {
    k: usize,
    filter: Option<String>,
    #[serde(default)]
    use_bq: Option<bool>,
    #[serde(default)]
    rescore_factor: Option<usize>,
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Options for hybrid search.
 */
export interface HybridSearchOptions {
    /** Number of results to return */
    k: number;
    /** Optional filter expression */
    filter?: string;
    /** Use binary quantization (default: true if enabled) */
    useBQ?: boolean;
    /** Rescore factor for BQ (default: 3) */
    rescoreFactor?: number;
}

/**
 * Hybrid search combining BQ speed with metadata filtering.
 *
 * This is the most flexible search method, combining:
 * - Binary quantization for speed
 * - Metadata filtering for precision
 * - Optional F32 rescoring for accuracy
 *
 * @param query Float32Array query vector
 * @param options Search options
 * @returns Array of SearchResult matching filter, sorted by distance
 *
 * @example
 * ```js
 * // Fast filtered search
 * const results = index.searchHybrid(query, {
 *     k: 10,
 *     filter: 'category == "news" AND score > 0.5',
 *     useBQ: true,
 *     rescoreFactor: 3
 * });
 * ```
 */
searchHybrid(query: Float32Array, options: HybridSearchOptions): SearchResult[];
```

**Acceptance Criteria:**
- [ ] `searchHybrid()` exported via wasm-bindgen
- [ ] Combines BQ with filter correctly
- [ ] Falls back to F32 when useBQ=false
- [ ] Falls back to F32 when BQ not enabled
- [ ] All results match filter

**Test Cases:**

```javascript
// tests/wasm/bq_hybrid.js

describe('searchHybrid', () => {
    let index;

    beforeEach(async () => {
        index = new WasmIndex({ dimensions: 768, useBQ: true });

        for (let i = 0; i < 1000; i++) {
            const vector = new Float32Array(768).map(() => Math.random() * 2 - 1);
            index.insertWithMetadata(vector, {
                category: ['news', 'sports', 'tech'][i % 3],
                score: Math.random()
            });
        }
    });

    it('should combine BQ with filter', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            filter: 'category == "news"',
            useBQ: true,
            rescoreFactor: 3
        });

        expect(results.length).toBeLessThanOrEqual(10);

        // All results should match filter
        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.category).toBe('news');
        }
    });

    it('should fall back to F32 when useBQ=false', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            filter: 'category == "news"',
            useBQ: false
        });

        expect(results.length).toBeLessThanOrEqual(10);
    });

    it('should work without filter', async () => {
        const query = new Float32Array(768).map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            useBQ: true
        });

        expect(results.length).toBe(10);
    });
});
```

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

## Day 2 Checklist

- [ ] W28.2.1: `searchBQ()` WASM binding
- [ ] W28.2.2: `searchBQRescored()` WASM binding
- [ ] W28.2.3: `searchHybrid()` WASM binding
- [ ] All unit tests pass
- [ ] Recall benchmarks validate >0.90 with rescoring
- [ ] Speed benchmarks validate speedup
- [ ] wasm-pack build succeeds
- [ ] Clippy clean

## Day 2 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `searchBQ()` works | Integration test |
| `searchBQRescored()` works | Integration test |
| `searchHybrid()` works | Integration test |
| BQ faster than F32 | Benchmark |
| Recall >0.90 with rescoring | Benchmark |
| wasm-pack builds | `wasm-pack build` |
| No clippy warnings | `cargo clippy -- -D warnings` |

## Day 2 Handoff

After completing Day 2:

**Artifacts Generated:**
- `src/wasm/bq.rs` (new file)
- Updated `src/wasm/mod.rs`
- Updated `pkg/edgevec.d.ts`
- `tests/wasm/bq_search.js`
- `tests/wasm/bq_rescore.js`
- `tests/wasm/bq_hybrid.js`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 3 — Memory Pressure + Integration Tests

---

*Agent: PLANNER + WASM_SPECIALIST*
*Status: [PROPOSED]*
*Date: 2025-12-22*
