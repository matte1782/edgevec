# Week 28 Day 1: Metadata WASM Bindings

**Date:** 2025-12-23
**Focus:** Expose Metadata Storage through WASM API
**Estimated Duration:** 10 hours
**Phase:** RFC-002 Implementation Phase 3 (WASM & Integration)
**Dependencies:** Week 26 (Metadata Rust API) — COMPLETE

---

## Tasks

### W28.1.1: `insertWithMetadata()` WASM Binding

**Objective:** Allow JavaScript to insert vectors with associated metadata.

**Rust Implementation:**

```rust
// src/wasm/metadata.rs

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;
use std::collections::HashMap;
use crate::metadata::MetadataValue;
use crate::hnsw::VectorId;

#[wasm_bindgen]
impl WasmIndex {
    /// Insert a vector with associated metadata.
    ///
    /// # Arguments
    /// * `vector` - Float32Array of embedding values
    /// * `metadata` - JavaScript object with string keys and MetadataValue values
    ///
    /// # Returns
    /// VectorId as u64 on success, or throws JsValue error
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const id = index.insertWithMetadata(
    ///     new Float32Array([0.1, 0.2, 0.3, ...]),
    ///     { category: "news", score: 0.95, tags: ["featured", "trending"] }
    /// );
    /// ```
    #[wasm_bindgen(js_name = insertWithMetadata)]
    pub fn insert_with_metadata(
        &mut self,
        vector: &[f32],
        metadata: JsValue,
    ) -> Result<u64, JsValue> {
        // Validate vector dimension
        if vector.len() != self.config.dimension as usize {
            return Err(JsValue::from_str(&format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.config.dimension,
                vector.len()
            )));
        }

        // Deserialize metadata from JavaScript object
        let metadata: HashMap<String, MetadataValue> =
            serde_wasm_bindgen::from_value(metadata)
                .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

        // Insert with metadata
        self.index
            .insert_with_metadata(vector, metadata, &mut self.storage)
            .map(|id| id.0)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Supported metadata value types.
 */
export type MetadataValue = string | number | boolean | string[];

/**
 * Insert a vector with associated metadata.
 * @param vector Float32Array of embedding values (must match index dimensions)
 * @param metadata Key-value pairs for filtering
 * @returns VectorId of the inserted vector
 * @throws Error if vector dimension mismatch or invalid metadata
 */
insertWithMetadata(vector: Float32Array, metadata: Record<string, MetadataValue>): number;
```

**Acceptance Criteria:**
- [ ] `insertWithMetadata()` exported via wasm-bindgen
- [ ] Accepts Float32Array for vector
- [ ] Accepts JavaScript object for metadata
- [ ] Returns VectorId (u64 as number)
- [ ] Throws descriptive error on dimension mismatch
- [ ] Throws descriptive error on invalid metadata type

**Test Cases:**

```javascript
// tests/wasm/metadata_insert.js

describe('insertWithMetadata', () => {
    it('should insert vector with metadata', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(768).fill(0.1);
        const metadata = { category: 'test', score: 0.95 };

        const id = index.insertWithMetadata(vector, metadata);

        expect(typeof id).toBe('number');
        expect(id).toBeGreaterThanOrEqual(0);
    });

    it('should reject wrong dimension', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(384).fill(0.1);

        expect(() => index.insertWithMetadata(vector, {}))
            .toThrow(/dimension mismatch/i);
    });

    it('should handle all metadata types', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(768).fill(0.1);
        const metadata = {
            stringVal: 'hello',
            numberVal: 42.5,
            boolVal: true,
            arrayVal: ['a', 'b', 'c']
        };

        const id = index.insertWithMetadata(vector, metadata);
        expect(id).toBeGreaterThanOrEqual(0);
    });
});
```

**Estimated Duration:** 3 hours

**Agent:** WASM_SPECIALIST

---

### W28.1.2: `searchFiltered()` WASM Binding

**Objective:** Enable JavaScript to search with metadata filter expressions.

**Rust Implementation:**

```rust
// src/wasm/metadata.rs (continued)

#[wasm_bindgen]
impl WasmIndex {
    /// Search with metadata filter expression.
    ///
    /// # Arguments
    /// * `query` - Float32Array query vector
    /// * `filter` - Filter expression string (e.g., 'category == "news" AND score > 0.5')
    /// * `k` - Number of results to return
    ///
    /// # Returns
    /// Array of SearchResult objects, or throws JsValue error
    ///
    /// # Filter Syntax
    /// - Comparison: `field == value`, `field != value`, `field > value`, etc.
    /// - Logical: `expr AND expr`, `expr OR expr`, `NOT expr`
    /// - Grouping: `(expr)`
    /// - Array contains: `field CONTAINS value`
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const results = index.searchFiltered(
    ///     query,
    ///     'category == "news" AND score > 0.5',
    ///     10
    /// );
    /// ```
    #[wasm_bindgen(js_name = searchFiltered)]
    pub fn search_filtered(
        &self,
        query: &[f32],
        filter: &str,
        k: usize,
    ) -> Result<JsValue, JsValue> {
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

        // Execute filtered search
        let results = self.index
            .search_filtered(query, filter, k, &self.storage)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Convert to JavaScript-friendly format
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

/// Search result for JavaScript.
#[derive(Serialize)]
struct JsSearchResult {
    id: u64,
    distance: f32,
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Search result returned from search operations.
 */
export interface SearchResult {
    id: number;
    distance: number;
}

/**
 * Search with metadata filter expression.
 * @param query Float32Array query vector
 * @param filter Filter expression (see Filter Syntax below)
 * @param k Number of results to return
 * @returns Array of SearchResult sorted by distance
 * @throws Error if query dimension mismatch or invalid filter syntax
 *
 * ## Filter Syntax
 * - Comparison: `field == value`, `field != value`, `field > value`, `field >= value`, `field < value`, `field <= value`
 * - Logical: `expr AND expr`, `expr OR expr`, `NOT expr`
 * - Grouping: `(expr)`
 * - Array contains: `field CONTAINS value`
 *
 * ## Examples
 * - `category == "news"`
 * - `score > 0.5 AND score < 1.0`
 * - `tags CONTAINS "featured"`
 * - `(category == "sports" OR category == "news") AND active == true`
 */
searchFiltered(query: Float32Array, filter: string, k: number): SearchResult[];
```

**Acceptance Criteria:**
- [ ] `searchFiltered()` exported via wasm-bindgen
- [ ] Accepts Float32Array query
- [ ] Accepts filter expression string
- [ ] Accepts k (number of results)
- [ ] Returns array of SearchResult
- [ ] Throws descriptive error on invalid filter
- [ ] Error includes filter syntax suggestion

**Test Cases:**

```javascript
// tests/wasm/metadata_search.js

describe('searchFiltered', () => {
    let index;

    beforeEach(async () => {
        index = new WasmIndex({ dimensions: 768 });

        // Insert test data
        for (let i = 0; i < 100; i++) {
            const vector = new Float32Array(768).map(() => Math.random());
            index.insertWithMetadata(vector, {
                category: ['news', 'sports', 'tech'][i % 3],
                score: Math.random(),
                active: i % 2 === 0
            });
        }
    });

    it('should filter by equality', async () => {
        const query = new Float32Array(768).map(() => Math.random());
        const results = index.searchFiltered(query, 'category == "news"', 10);

        expect(results.length).toBeLessThanOrEqual(10);
        // Verify all results have category == "news"
        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.category).toBe('news');
        }
    });

    it('should filter by comparison', async () => {
        const query = new Float32Array(768).map(() => Math.random());
        const results = index.searchFiltered(query, 'score > 0.5', 10);

        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.score).toBeGreaterThan(0.5);
        }
    });

    it('should filter with AND', async () => {
        const query = new Float32Array(768).map(() => Math.random());
        const results = index.searchFiltered(
            query,
            'category == "news" AND active == true',
            10
        );

        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.category).toBe('news');
            expect(meta.active).toBe(true);
        }
    });

    it('should throw on invalid filter', async () => {
        const query = new Float32Array(768).map(() => Math.random());

        expect(() => index.searchFiltered(query, 'invalid syntax !!!', 10))
            .toThrow();
    });
});
```

**Estimated Duration:** 3 hours

**Agent:** WASM_SPECIALIST

---

### W28.1.3: `getMetadata()` WASM Binding

**Objective:** Allow JavaScript to retrieve metadata for a vector by ID.

**Rust Implementation:**

```rust
// src/wasm/metadata.rs (continued)

#[wasm_bindgen]
impl WasmIndex {
    /// Get metadata for a vector by ID.
    ///
    /// # Arguments
    /// * `id` - VectorId to look up
    ///
    /// # Returns
    /// Metadata object if found, null if not found or deleted
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const id = index.insertWithMetadata(vector, { category: 'news' });
    /// const meta = index.getMetadata(id);
    /// console.log(meta.category); // 'news'
    /// ```
    #[wasm_bindgen(js_name = getMetadata)]
    pub fn get_metadata(&self, id: u64) -> Result<JsValue, JsValue> {
        match self.index.get_metadata(VectorId(id)) {
            Some(metadata) => serde_wasm_bindgen::to_value(metadata)
                .map_err(|e| JsValue::from_str(&e.to_string())),
            None => Ok(JsValue::NULL),
        }
    }
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Get metadata for a vector by ID.
 * @param id VectorId to look up
 * @returns Metadata object if found, null if not found or deleted
 */
getMetadata(id: number): Record<string, MetadataValue> | null;
```

**Acceptance Criteria:**
- [ ] `getMetadata()` exported via wasm-bindgen
- [ ] Returns metadata object for valid ID
- [ ] Returns null for non-existent ID
- [ ] Returns null for deleted vector
- [ ] Preserves all metadata value types

**Test Cases:**

```javascript
// tests/wasm/metadata_get.js

describe('getMetadata', () => {
    it('should return metadata for valid id', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(768).fill(0.1);
        const metadata = { category: 'test', score: 0.95 };

        const id = index.insertWithMetadata(vector, metadata);
        const retrieved = index.getMetadata(id);

        expect(retrieved).not.toBeNull();
        expect(retrieved.category).toBe('test');
        expect(retrieved.score).toBe(0.95);
    });

    it('should return null for non-existent id', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const retrieved = index.getMetadata(99999);

        expect(retrieved).toBeNull();
    });

    it('should return null for deleted vector', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(768).fill(0.1);
        const id = index.insertWithMetadata(vector, { x: 1 });

        index.softDelete(id);
        const retrieved = index.getMetadata(id);

        expect(retrieved).toBeNull();
    });

    it('should preserve array metadata', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const vector = new Float32Array(768).fill(0.1);
        const metadata = { tags: ['a', 'b', 'c'] };

        const id = index.insertWithMetadata(vector, metadata);
        const retrieved = index.getMetadata(id);

        expect(retrieved.tags).toEqual(['a', 'b', 'c']);
    });
});
```

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

### W28.1.4: TypeScript Type Definitions for Metadata

**Objective:** Generate comprehensive TypeScript types for metadata operations.

**TypeScript Definitions:**

```typescript
// pkg/edgevec.d.ts (metadata section)

// ============================================================================
// METADATA TYPES
// ============================================================================

/**
 * Supported metadata value types.
 *
 * - `string`: Text values (e.g., category names)
 * - `number`: Numeric values (e.g., scores, counts)
 * - `boolean`: Boolean flags (e.g., active, featured)
 * - `string[]`: Tag arrays (e.g., categories, labels)
 */
export type MetadataValue = string | number | boolean | string[];

/**
 * Metadata object for a vector.
 */
export type Metadata = Record<string, MetadataValue>;

// ============================================================================
// SEARCH TYPES
// ============================================================================

/**
 * Search result from any search operation.
 */
export interface SearchResult {
    /** Vector ID */
    id: number;
    /** Distance from query (lower is more similar) */
    distance: number;
}

/**
 * Search result with metadata included.
 */
export interface SearchResultWithMetadata extends SearchResult {
    /** Metadata for this result (if requested) */
    metadata?: Metadata;
}

/**
 * Options for filtered search.
 */
export interface FilteredSearchOptions {
    /** Filter expression (see Filter Syntax) */
    filter: string;
    /** Number of results to return */
    k: number;
    /** Include metadata in results (default: false) */
    includeMetadata?: boolean;
}

// ============================================================================
// FILTER SYNTAX DOCUMENTATION
// ============================================================================

/**
 * Filter expression syntax for searchFiltered().
 *
 * ## Operators
 *
 * ### Comparison Operators
 * - `==` Equal: `category == "news"`
 * - `!=` Not equal: `category != "spam"`
 * - `>` Greater than: `score > 0.5`
 * - `>=` Greater or equal: `score >= 0.5`
 * - `<` Less than: `score < 1.0`
 * - `<=` Less or equal: `score <= 1.0`
 *
 * ### Logical Operators
 * - `AND` Logical and: `category == "news" AND score > 0.5`
 * - `OR` Logical or: `category == "news" OR category == "sports"`
 * - `NOT` Logical not: `NOT category == "spam"`
 *
 * ### Array Operators
 * - `CONTAINS` Array contains: `tags CONTAINS "featured"`
 *
 * ### Grouping
 * - `()` Parentheses: `(category == "news" OR category == "sports") AND active == true`
 *
 * ## Value Types
 * - Strings: `"value"` or `'value'`
 * - Numbers: `42`, `3.14`, `-1.5`
 * - Booleans: `true`, `false`
 *
 * ## Examples
 * ```
 * category == "news"
 * score > 0.5 AND score < 1.0
 * tags CONTAINS "featured"
 * (category == "sports" OR category == "news") AND active == true
 * NOT (category == "spam" OR score < 0.1)
 * ```
 */
export type FilterExpression = string;
```

**Acceptance Criteria:**
- [ ] All metadata types defined
- [ ] SearchResult interface defined
- [ ] FilteredSearchOptions interface defined
- [ ] Filter syntax documented in JSDoc
- [ ] Types compile without errors

**Verification:**

```bash
# Verify TypeScript types compile
cd pkg
npx tsc --noEmit edgevec.d.ts
```

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST + DOCWRITER

---

## Day 1 Checklist

- [ ] W28.1.1: `insertWithMetadata()` WASM binding
- [ ] W28.1.2: `searchFiltered()` WASM binding
- [ ] W28.1.3: `getMetadata()` WASM binding
- [ ] W28.1.4: TypeScript type definitions
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] wasm-pack build succeeds
- [ ] Clippy clean

## Day 1 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `insertWithMetadata()` works | Integration test |
| `searchFiltered()` works | Integration test |
| `getMetadata()` works | Integration test |
| TypeScript compiles | `tsc --noEmit` |
| wasm-pack builds | `wasm-pack build` |
| No clippy warnings | `cargo clippy -- -D warnings` |

## Day 1 Handoff

After completing Day 1:

**Artifacts Generated:**
- `src/wasm/metadata.rs` (new file)
- Updated `src/wasm/mod.rs`
- Updated `pkg/edgevec.d.ts`
- `tests/wasm/metadata_insert.js`
- `tests/wasm/metadata_search.js`
- `tests/wasm/metadata_get.js`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 2 — BQ WASM Bindings

---

*Agent: PLANNER + WASM_SPECIALIST*
*Status: [PROPOSED]*
*Date: 2025-12-22*
