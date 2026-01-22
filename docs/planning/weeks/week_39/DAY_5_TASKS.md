# Week 39 Day 5: WASM Bindings + TypeScript Types

**Date:** 2026-01-30
**Focus:** Expose hybrid search to JavaScript via WASM bindings
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 3 (WASM Integration)
**Dependencies:** Day 1-4 COMPLETE (SparseSearcher, RRF, Linear, HybridSearcher)

---

## Context

Day 5 adds WASM bindings for the hybrid search functionality, making it
accessible from JavaScript/TypeScript in the browser.

**Target API (from ROADMAP):**

```typescript
const hybridResults = await db.hybridSearch({
  dense: { vector: embedding, k: 20 },
  sparse: { vector: bm25Scores, k: 20 },
  fusion: 'rrf',  // or { type: 'linear', alpha: 0.7 }
  k: 10
});
```

**Files to Modify:**
- `src/wasm/mod.rs` - Add WASM bindings
- `pkg/edgevec.d.ts` - TypeScript type definitions

---

## Tasks

### W39.5.1: Add Sparse Search WASM Binding

**Objective:** Expose sparse-only search via WASM.

**File:** `src/wasm/mod.rs` (additions)

```rust
// =============================================================================
// SPARSE SEARCH WASM BINDINGS
// =============================================================================

impl EdgeVecWasm {
    /// Search sparse vectors by query.
    ///
    /// # Arguments
    ///
    /// * `indices` - Uint32Array of sparse query indices (sorted ascending)
    /// * `values` - Float32Array of sparse query values (same length as indices)
    /// * `dim` - Dimension of the sparse space (vocabulary size)
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// JSON string: `[{ "id": number, "score": number }, ...]`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - indices and values have different lengths
    /// - indices are not sorted ascending
    /// - k is 0
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const indices = new Uint32Array([0, 5, 10]);
    /// const values = new Float32Array([1.0, 2.0, 3.0]);
    /// const resultsJson = db.searchSparse(indices, values, 10000, 10);
    /// const results = JSON.parse(resultsJson);
    /// ```
    #[wasm_bindgen(js_name = searchSparse)]
    pub fn search_sparse(
        &self,
        indices: Uint32Array,
        values: Float32Array,
        dim: u32,
        k: usize,
    ) -> Result<String, JsValue> {
        use crate::sparse::{SparseSearcher, SparseVector};

        // Validate inputs
        if indices.length() != values.length() {
            return Err(JsValue::from_str(
                "indices and values must have the same length"
            ));
        }

        if k == 0 {
            return Err(JsValue::from_str("k must be greater than 0"));
        }

        // Convert TypedArrays to Rust types
        let indices_vec: Vec<u32> = indices.to_vec();
        let values_vec: Vec<f32> = values.to_vec();

        // Create sparse vector (validates sorted, no duplicates, etc.)
        let query = SparseVector::new(indices_vec, values_vec, dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get sparse storage (check if it exists)
        let sparse_storage = self.sparse_storage.as_ref()
            .ok_or_else(|| JsValue::from_str("Sparse storage not initialized"))?;

        // Execute search
        let searcher = SparseSearcher::new(sparse_storage);
        let results = searcher.search(&query, k);

        // Convert to JSON
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.as_u64(),
                    "score": r.score
                })
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Insert a sparse vector.
    ///
    /// # Arguments
    ///
    /// * `indices` - Uint32Array of sparse indices (sorted ascending)
    /// * `values` - Float32Array of sparse values (same length as indices)
    /// * `dim` - Dimension of the sparse space (vocabulary size)
    ///
    /// # Returns
    ///
    /// The assigned SparseId as a number (f64).
    ///
    /// [HOSTILE_REVIEW: m2 Resolution] - JavaScript numbers (f64) have
    /// integer precision up to 2^53 (9,007,199,254,740,992). IDs beyond
    /// this limit will lose precision. For most use cases (<9 quadrillion
    /// vectors), this is not a concern. If needed, consider returning
    /// BigInt in a future version.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const indices = new Uint32Array([0, 5, 10]);
    /// const values = new Float32Array([1.0, 2.0, 3.0]);
    /// const id = db.insertSparse(indices, values, 10000);
    /// ```
    #[wasm_bindgen(js_name = insertSparse)]
    pub fn insert_sparse(
        &mut self,
        indices: Uint32Array,
        values: Float32Array,
        dim: u32,
    ) -> Result<f64, JsValue> {
        use crate::sparse::SparseVector;

        // Validate inputs
        if indices.length() != values.length() {
            return Err(JsValue::from_str(
                "indices and values must have the same length"
            ));
        }

        // Convert TypedArrays
        let indices_vec: Vec<u32> = indices.to_vec();
        let values_vec: Vec<f32> = values.to_vec();

        // Create sparse vector
        let vector = SparseVector::new(indices_vec, values_vec, dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get or create sparse storage
        let sparse_storage = self.sparse_storage
            .get_or_insert_with(|| SparseStorage::new());

        // Insert
        let id = sparse_storage.insert(&vector)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Return as f64 (JavaScript number)
        Ok(id.as_u64() as f64)
    }
}
```

**Acceptance Criteria:**
- [ ] `searchSparse()` binding implemented
- [ ] `insertSparse()` binding implemented
- [ ] Input validation (length match, k > 0)
- [ ] Returns JSON string for search results
- [ ] Returns numeric ID for insert
- [ ] Error messages are descriptive
- [ ] Doc comments with JS examples

**Estimated Duration:** 45 minutes

**Agent:** WASM_SPECIALIST

---

### W39.5.2: Add Hybrid Search WASM Binding

**Objective:** Expose hybrid search via WASM with JSON config.

**File:** `src/wasm/mod.rs` (continued)

```rust
impl EdgeVecWasm {
    /// Perform hybrid search combining dense and sparse.
    ///
    /// # Arguments
    ///
    /// * `dense_query` - Float32Array dense embedding vector
    /// * `sparse_indices` - Uint32Array sparse query indices (sorted)
    /// * `sparse_values` - Float32Array sparse query values
    /// * `sparse_dim` - Dimension of sparse space (vocabulary size)
    /// * `options_json` - JSON configuration string
    ///
    /// # Options JSON Schema
    ///
    /// ```json
    /// {
    ///   "dense_k": 20,      // Results from dense search (default: 20)
    ///   "sparse_k": 20,     // Results from sparse search (default: 20)
    ///   "k": 10,            // Final results to return (required)
    ///   "fusion": "rrf"     // or { "type": "linear", "alpha": 0.7 }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// JSON string:
    /// ```json
    /// [
    ///   {
    ///     "id": 42,
    ///     "score": 0.032,
    ///     "dense_rank": 1,
    ///     "dense_score": 0.95,
    ///     "sparse_rank": 3,
    ///     "sparse_score": 4.2
    ///   },
    ///   ...
    /// ]
    /// ```
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const denseQuery = new Float32Array([0.1, 0.2, ...]);
    /// const sparseIndices = new Uint32Array([0, 5, 10]);
    /// const sparseValues = new Float32Array([1.0, 2.0, 3.0]);
    ///
    /// const results = JSON.parse(db.hybridSearch(
    ///   denseQuery,
    ///   sparseIndices,
    ///   sparseValues,
    ///   10000,
    ///   JSON.stringify({ k: 10, fusion: 'rrf' })
    /// ));
    /// ```
    #[wasm_bindgen(js_name = hybridSearch)]
    pub fn hybrid_search(
        &mut self,
        dense_query: Float32Array,
        sparse_indices: Uint32Array,
        sparse_values: Float32Array,
        sparse_dim: u32,
        options_json: &str,
    ) -> Result<String, JsValue> {
        use crate::hybrid::{HybridSearcher, HybridSearchConfig, FusionMethod};
        use crate::sparse::SparseVector;

        // Parse options
        let options: HybridSearchOptions = serde_json::from_str(options_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid options JSON: {}", e)))?;

        // Validate dense query dimensions
        let expected_dims = self.inner.config.dimensions;
        if dense_query.length() != expected_dims {
            return Err(JsValue::from_str(&format!(
                "Dense query dimension mismatch: expected {}, got {}",
                expected_dims, dense_query.length()
            )));
        }

        // Validate sparse inputs
        if sparse_indices.length() != sparse_values.length() {
            return Err(JsValue::from_str(
                "sparse_indices and sparse_values must have the same length"
            ));
        }

        // Convert inputs
        let dense_vec: Vec<f32> = dense_query.to_vec();
        let sparse_indices_vec: Vec<u32> = sparse_indices.to_vec();
        let sparse_values_vec: Vec<f32> = sparse_values.to_vec();

        // Create sparse query
        let sparse_query = SparseVector::new(sparse_indices_vec, sparse_values_vec, sparse_dim)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Get sparse storage
        let sparse_storage = self.sparse_storage.as_ref()
            .ok_or_else(|| JsValue::from_str("Sparse storage not initialized"))?;

        // Build config
        let fusion = match options.fusion {
            HybridFusionOption::Rrf => FusionMethod::rrf(),
            HybridFusionOption::Linear { alpha } => FusionMethod::linear(alpha),
        };

        let config = HybridSearchConfig::new(
            options.dense_k.unwrap_or(20),
            options.sparse_k.unwrap_or(20),
            options.k,
            fusion,
        );

        // Execute hybrid search
        let searcher = HybridSearcher::new(
            &self.inner.graph,
            &self.inner.storage,
            sparse_storage,
        );

        let results = searcher.search(&dense_vec, &sparse_query, &config)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JSON
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let mut obj = serde_json::json!({
                    "id": r.id.0,
                    "score": r.score
                });

                if let Some(rank) = r.dense_rank {
                    obj["dense_rank"] = serde_json::json!(rank);
                }
                if let Some(score) = r.dense_score {
                    obj["dense_score"] = serde_json::json!(score);
                }
                if let Some(rank) = r.sparse_rank {
                    obj["sparse_rank"] = serde_json::json!(rank);
                }
                if let Some(score) = r.sparse_score {
                    obj["sparse_score"] = serde_json::json!(score);
                }

                obj
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// =============================================================================
// WASM OPTION TYPES
// =============================================================================

/// Options for hybrid search (parsed from JSON).
#[derive(Debug, Deserialize)]
struct HybridSearchOptions {
    /// Final number of results to return
    k: usize,

    /// Number of dense results to retrieve (default: 20)
    dense_k: Option<usize>,

    /// Number of sparse results to retrieve (default: 20)
    sparse_k: Option<usize>,

    /// Fusion method
    #[serde(default)]
    fusion: HybridFusionOption,
}

/// Fusion method option (from JSON).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HybridFusionOption {
    /// Simple string: "rrf"
    Rrf,

    /// Object: { "type": "linear", "alpha": 0.7 }
    Linear {
        #[serde(rename = "type")]
        _type: String,
        alpha: f32,
    },
}

impl Default for HybridFusionOption {
    fn default() -> Self {
        HybridFusionOption::Rrf
    }
}

// Custom deserializer for fusion that handles both string and object
impl<'de> Deserialize<'de> for HybridFusionOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor, MapAccess};

        struct FusionVisitor;

        impl<'de> Visitor<'de> for FusionVisitor {
            type Value = HybridFusionOption;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("\"rrf\" or { \"type\": \"linear\", \"alpha\": number }")
            }

            fn visit_str<E>(self, value: &str) -> Result<HybridFusionOption, E>
            where
                E: de::Error,
            {
                match value.to_lowercase().as_str() {
                    "rrf" => Ok(HybridFusionOption::Rrf),
                    _ => Err(de::Error::unknown_variant(value, &["rrf"])),
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<HybridFusionOption, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut fusion_type: Option<String> = None;
                let mut alpha: Option<f32> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => fusion_type = Some(map.next_value()?),
                        "alpha" => alpha = Some(map.next_value()?),
                        _ => { let _ = map.next_value::<serde_json::Value>()?; }
                    }
                }

                match fusion_type.as_deref() {
                    Some("linear") => {
                        let alpha = alpha.ok_or_else(|| de::Error::missing_field("alpha"))?;
                        Ok(HybridFusionOption::Linear { _type: "linear".to_string(), alpha })
                    }
                    Some("rrf") | None => Ok(HybridFusionOption::Rrf),
                    Some(other) => Err(de::Error::unknown_variant(other, &["rrf", "linear"])),
                }
            }
        }

        deserializer.deserialize_any(FusionVisitor)
    }
}
```

**Acceptance Criteria:**
- [ ] `hybridSearch()` binding implemented
- [ ] Parses JSON options correctly
- [ ] Supports `"rrf"` string for fusion
- [ ] Supports `{ "type": "linear", "alpha": 0.7 }` object for fusion
- [ ] Returns detailed JSON results with ranks and scores
- [ ] Input validation for dimensions
- [ ] Error messages are descriptive
- [ ] Doc comments with JS examples

**Estimated Duration:** 1.5 hours

**Agent:** WASM_SPECIALIST

---

### W39.5.3: Add SparseStorage Field to EdgeVecWasm

**Objective:** Add sparse storage to WASM wrapper struct.

**File:** `src/wasm/mod.rs` (struct modification)

```rust
// Modify EdgeVecWasm struct to include sparse storage

#[wasm_bindgen]
pub struct EdgeVecWasm {
    inner: HnswIndex,
    sparse_storage: Option<SparseStorage>,  // NEW: Week 39
}

// Update constructors
impl EdgeVecWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(dimensions: u32, options: JsValue) -> Result<EdgeVecWasm, JsValue> {
        // ... existing constructor code ...

        Ok(EdgeVecWasm {
            inner: index,
            sparse_storage: None,  // Initialize as None
        })
    }

    /// Initialize sparse storage for hybrid search.
    ///
    /// Must be called before using sparse or hybrid search functions.
    ///
    /// # Example (JavaScript)
    ///
    /// ```javascript
    /// const db = new EdgeVec(384);
    /// db.initSparseStorage();  // Enable hybrid search
    /// ```
    #[wasm_bindgen(js_name = initSparseStorage)]
    pub fn init_sparse_storage(&mut self) {
        if self.sparse_storage.is_none() {
            self.sparse_storage = Some(SparseStorage::new());
        }
    }

    /// Check if sparse storage is initialized.
    #[wasm_bindgen(js_name = hasSparseStorage)]
    pub fn has_sparse_storage(&self) -> bool {
        self.sparse_storage.is_some()
    }

    /// Get the number of sparse vectors.
    #[wasm_bindgen(js_name = sparseCount)]
    pub fn sparse_count(&self) -> usize {
        self.sparse_storage.as_ref().map_or(0, |s| s.len())
    }
}
```

**Acceptance Criteria:**
- [ ] `sparse_storage` field added to `EdgeVecWasm`
- [ ] `initSparseStorage()` method implemented
- [ ] `hasSparseStorage()` check method
- [ ] `sparseCount()` accessor
- [ ] Existing constructors updated
- [ ] Load/save methods handle sparse storage (if applicable)

**Estimated Duration:** 30 minutes

**Agent:** WASM_SPECIALIST

---

### W39.5.4: TypeScript Type Definitions

**Objective:** Add TypeScript types for hybrid search API.

**File:** `pkg/edgevec.d.ts` (additions)

```typescript
// =============================================================================
// SPARSE VECTOR TYPES
// =============================================================================

/**
 * Sparse vector representation for keyword/BM25 features.
 *
 * @example
 * ```typescript
 * const sparse: SparseVector = {
 *   indices: new Uint32Array([0, 5, 10]),
 *   values: new Float32Array([1.0, 2.0, 3.0]),
 *   dim: 10000  // vocabulary size
 * };
 * ```
 */
export interface SparseVector {
  /** Sorted indices of non-zero elements */
  indices: Uint32Array;
  /** Values corresponding to indices */
  values: Float32Array;
  /** Dimension of the sparse space (vocabulary size) */
  dim: number;
}

// =============================================================================
// HYBRID SEARCH TYPES
// =============================================================================

/**
 * Fusion method for combining dense and sparse results.
 *
 * @example
 * ```typescript
 * // RRF fusion (recommended default)
 * const fusion: FusionMethod = 'rrf';
 *
 * // Linear combination (70% dense, 30% sparse)
 * const fusion: FusionMethod = { type: 'linear', alpha: 0.7 };
 * ```
 */
export type FusionMethod =
  | 'rrf'
  | { type: 'linear'; alpha: number };

/**
 * Options for hybrid search.
 *
 * @example
 * ```typescript
 * const options: HybridSearchOptions = {
 *   dense_k: 20,    // Get 20 from dense search
 *   sparse_k: 20,   // Get 20 from sparse search
 *   k: 10,          // Return top 10 after fusion
 *   fusion: 'rrf'   // Use RRF fusion
 * };
 * ```
 */
export interface HybridSearchOptions {
  /** Number of results to retrieve from dense (HNSW) search. Default: 20 */
  dense_k?: number;

  /** Number of results to retrieve from sparse search. Default: 20 */
  sparse_k?: number;

  /** Final number of results to return after fusion. Required. */
  k: number;

  /** Fusion method. Default: 'rrf' */
  fusion?: FusionMethod;
}

/**
 * Result from hybrid search.
 *
 * Includes the combined score and optional information about
 * the document's rank/score in each individual search.
 */
export interface HybridSearchResult {
  /** Vector/document ID */
  id: number;

  /** Combined score from fusion algorithm */
  score: number;

  /** Rank in dense search results (1-indexed). Undefined if not found in dense. */
  dense_rank?: number;

  /** Original score from dense search. Undefined if not found in dense. */
  dense_score?: number;

  /** Rank in sparse search results (1-indexed). Undefined if not found in sparse. */
  sparse_rank?: number;

  /** Original score from sparse search. Undefined if not found in sparse. */
  sparse_score?: number;
}

/**
 * Result from sparse-only search.
 */
export interface SparseSearchResult {
  /** Sparse vector ID */
  id: number;

  /** Dot product similarity score */
  score: number;
}

// =============================================================================
// EDGEVEC CLASS EXTENSIONS
// =============================================================================

declare module './edgevec' {
  interface EdgeVec {
    /**
     * Initialize sparse storage for hybrid search.
     * Must be called before using sparse or hybrid search.
     *
     * @example
     * ```typescript
     * const db = new EdgeVec(384);
     * db.initSparseStorage();  // Enable hybrid search
     * ```
     */
    initSparseStorage(): void;

    /**
     * Check if sparse storage is initialized.
     */
    hasSparseStorage(): boolean;

    /**
     * Get the number of sparse vectors stored.
     */
    sparseCount(): number;

    /**
     * Insert a sparse vector (e.g., BM25 scores).
     *
     * @param indices - Sorted indices of non-zero elements
     * @param values - Values corresponding to indices
     * @param dim - Dimension of sparse space (vocabulary size)
     * @returns The assigned sparse vector ID
     *
     * @example
     * ```typescript
     * const indices = new Uint32Array([0, 5, 10]);
     * const values = new Float32Array([1.0, 2.0, 3.0]);
     * const id = db.insertSparse(indices, values, 10000);
     * ```
     */
    insertSparse(indices: Uint32Array, values: Float32Array, dim: number): number;

    /**
     * Search sparse vectors by query.
     *
     * @param indices - Query sparse indices (sorted)
     * @param values - Query sparse values
     * @param dim - Dimension of sparse space
     * @param k - Number of results
     * @returns JSON string of results (parse with JSON.parse)
     *
     * @example
     * ```typescript
     * const indices = new Uint32Array([0, 5, 10]);
     * const values = new Float32Array([1.0, 2.0, 3.0]);
     * const resultsJson = db.searchSparse(indices, values, 10000, 10);
     * const results: SparseSearchResult[] = JSON.parse(resultsJson);
     * ```
     */
    searchSparse(indices: Uint32Array, values: Float32Array, dim: number, k: number): string;

    /**
     * Perform hybrid search combining dense and sparse.
     *
     * @param denseQuery - Dense embedding vector
     * @param sparseIndices - Sparse query indices (sorted)
     * @param sparseValues - Sparse query values
     * @param sparseDim - Dimension of sparse space
     * @param optionsJson - JSON string of HybridSearchOptions
     * @returns JSON string of results (parse with JSON.parse)
     *
     * @example
     * ```typescript
     * const denseQuery = new Float32Array([0.1, 0.2, ...]);
     * const sparseIndices = new Uint32Array([0, 5, 10]);
     * const sparseValues = new Float32Array([1.0, 2.0, 3.0]);
     *
     * const options: HybridSearchOptions = {
     *   dense_k: 20,
     *   sparse_k: 20,
     *   k: 10,
     *   fusion: 'rrf'
     * };
     *
     * const resultsJson = db.hybridSearch(
     *   denseQuery,
     *   sparseIndices,
     *   sparseValues,
     *   10000,
     *   JSON.stringify(options)
     * );
     * const results: HybridSearchResult[] = JSON.parse(resultsJson);
     * ```
     */
    hybridSearch(
      denseQuery: Float32Array,
      sparseIndices: Uint32Array,
      sparseValues: Float32Array,
      sparseDim: number,
      optionsJson: string
    ): string;
  }
}

// =============================================================================
// HELPER FUNCTIONS (Optional TypeScript Wrappers)
// =============================================================================

/**
 * Create a sparse vector from term-score pairs.
 *
 * @param termScores - Object mapping term IDs to scores
 * @param dim - Vocabulary size
 * @returns SparseVector ready for insertion/search
 *
 * @example
 * ```typescript
 * // From BM25 scores
 * const bm25Scores = { 42: 2.5, 100: 1.8, 500: 3.2 };
 * const sparse = createSparseVector(bm25Scores, 10000);
 * ```
 */
export function createSparseVector(
  termScores: Record<number, number>,
  dim: number
): SparseVector;

/**
 * Parse hybrid search results from JSON.
 *
 * @param json - JSON string from hybridSearch()
 * @returns Typed array of HybridSearchResult
 */
export function parseHybridResults(json: string): HybridSearchResult[];

/**
 * Parse sparse search results from JSON.
 *
 * @param json - JSON string from searchSparse()
 * @returns Typed array of SparseSearchResult
 */
export function parseSparseResults(json: string): SparseSearchResult[];
```

**Acceptance Criteria:**
- [ ] `SparseVector` interface defined
- [ ] `FusionMethod` type with both variants
- [ ] `HybridSearchOptions` interface with all fields
- [ ] `HybridSearchResult` interface with optional fields
- [ ] `SparseSearchResult` interface
- [ ] EdgeVec class extended with new methods
- [ ] Helper functions declared
- [ ] JSDoc comments with examples
- [ ] Examples show correct usage patterns

**Estimated Duration:** 30 minutes

**Agent:** WASM_SPECIALIST

---

## Day 5 Checklist

- [ ] W39.5.1: `searchSparse()` and `insertSparse()` bindings implemented
- [ ] W39.5.2: `hybridSearch()` binding with JSON config parsing
- [ ] W39.5.3: `sparse_storage` field added to `EdgeVecWasm`
- [ ] W39.5.4: TypeScript type definitions complete
- [ ] `cargo check --target wasm32-unknown-unknown` passes
- [ ] `cargo clippy --target wasm32-unknown-unknown -- -D warnings` passes
- [ ] `wasm-pack build --target web` succeeds
- [ ] TypeScript compilation passes

---

## Day 5 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Sparse insert works from JS | Manual test in browser console |
| Sparse search works from JS | Manual test in browser console |
| Hybrid search works from JS | Manual test in browser console |
| RRF fusion parses correctly | JSON parsing test |
| Linear fusion parses correctly | JSON parsing test |
| TypeScript compiles | `tsc --noEmit` |
| WASM builds | `wasm-pack build` |

---

## Day 5 Handoff

After completing Day 5:

**Artifacts Generated:**
- Updated `src/wasm/mod.rs` with sparse/hybrid bindings
- Updated `pkg/edgevec.d.ts` with TypeScript types
- WASM build with hybrid search support

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 6 - Integration Tests + Benchmarks + Hostile Review

---

## Notes for Implementation

### JSON vs TypedArray Trade-off

We use JSON strings for options and results because:
1. Complex nested structures (fusion config) are awkward with wasm-bindgen
2. JavaScript developers expect JSON for configs
3. Performance is acceptable (JSON parse < 1ms for typical result sets)

TypedArrays are used for the vectors themselves (dense/sparse data) for:
1. Zero-copy transfer from JavaScript
2. Better memory efficiency
3. Familiar pattern for ML/embedding use cases

### Sparse Storage Initialization

The sparse storage is optional and initialized separately because:
1. Not all users need hybrid search
2. Keeps memory footprint low for dense-only use cases
3. Explicit initialization is clearer than auto-creation

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-21*
