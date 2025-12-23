# Week 28 Day 3: Memory Pressure + Integration Tests

**Date:** 2025-12-25
**Focus:** Memory Monitoring API and Initial Integration Tests
**Estimated Duration:** 8 hours
**Phase:** RFC-002 Implementation Phase 3 (WASM & Integration)
**Dependencies:** W28.1 (Metadata WASM), W28.2 (BQ WASM)

---

## Tasks

### W28.3.1: `getMemoryPressure()` WASM Binding

**Objective:** Provide JavaScript visibility into WASM heap usage.

**RFC-002 Specification:**
- Memory thresholds: 80% warning, 95% critical
- Real-time usage stats
- Guidance for graceful degradation

**Rust Implementation:**

```rust
// src/wasm/memory.rs

use wasm_bindgen::prelude::*;
use serde::Serialize;

/// Memory pressure levels.
#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryPressureLevel {
    Normal,
    Warning,
    Critical,
}

/// Memory pressure statistics.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPressure {
    pub level: MemoryPressureLevel,
    pub used_bytes: usize,
    pub total_bytes: usize,
    pub usage_percent: f64,
}

impl MemoryPressure {
    /// Calculate memory pressure from current WASM heap.
    pub fn current() -> Self {
        // Get WASM memory object
        let memory = wasm_bindgen::memory();
        let buffer = memory.unchecked_ref::<js_sys::WebAssembly::Memory>().buffer();
        let total_bytes = buffer.byte_length() as usize;

        // Estimate used bytes (this is an approximation)
        // In practice, we track allocations via the allocator
        let used_bytes = Self::estimate_used_bytes();

        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let level = if usage_percent >= 95.0 {
            MemoryPressureLevel::Critical
        } else if usage_percent >= 80.0 {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        };

        Self {
            level,
            used_bytes,
            total_bytes,
            usage_percent,
        }
    }

    /// Estimate used bytes by tracking allocations.
    fn estimate_used_bytes() -> usize {
        // Use a thread-local counter updated by allocator hooks
        // or use a sampling approach
        ALLOCATION_TRACKER.with(|tracker| tracker.borrow().used_bytes)
    }
}

// Thread-local allocation tracker
thread_local! {
    static ALLOCATION_TRACKER: std::cell::RefCell<AllocationTracker> =
        std::cell::RefCell::new(AllocationTracker::new());
}

struct AllocationTracker {
    used_bytes: usize,
}

impl AllocationTracker {
    fn new() -> Self {
        Self { used_bytes: 0 }
    }

    fn allocate(&mut self, size: usize) {
        self.used_bytes = self.used_bytes.saturating_add(size);
    }

    fn deallocate(&mut self, size: usize) {
        self.used_bytes = self.used_bytes.saturating_sub(size);
    }
}

#[wasm_bindgen]
impl WasmIndex {
    /// Get current memory pressure state.
    ///
    /// Returns memory usage statistics and pressure level.
    /// Use this to implement graceful degradation in your app.
    ///
    /// # Returns
    /// MemoryPressure object with:
    /// - `level`: "normal", "warning", or "critical"
    /// - `usedBytes`: Bytes currently allocated
    /// - `totalBytes`: Total WASM heap size
    /// - `usagePercent`: Usage as percentage (0-100)
    ///
    /// # Thresholds
    /// - Normal: <80% usage
    /// - Warning: 80-95% usage (consider reducing data)
    /// - Critical: >95% usage (risk of OOM, stop inserts)
    ///
    /// # Example (JavaScript)
    /// ```js
    /// const pressure = index.getMemoryPressure();
    /// if (pressure.level === 'warning') {
    ///     console.warn('Memory pressure high, consider compacting');
    ///     index.compact();
    /// } else if (pressure.level === 'critical') {
    ///     console.error('Memory critical, stopping inserts');
    ///     // Disable insert button, show warning to user
    /// }
    /// ```
    #[wasm_bindgen(js_name = getMemoryPressure)]
    pub fn get_memory_pressure(&self) -> Result<JsValue, JsValue> {
        let pressure = MemoryPressure::current();
        serde_wasm_bindgen::to_value(&pressure)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Memory pressure levels.
 * - `normal`: <80% usage, all operations allowed
 * - `warning`: 80-95% usage, consider reducing data
 * - `critical`: >95% usage, risk of OOM
 */
export type MemoryPressureLevel = 'normal' | 'warning' | 'critical';

/**
 * Memory pressure statistics.
 */
export interface MemoryPressure {
    /** Current pressure level */
    level: MemoryPressureLevel;
    /** Bytes currently allocated */
    usedBytes: number;
    /** Total WASM heap size */
    totalBytes: number;
    /** Usage as percentage (0-100) */
    usagePercent: number;
}

/**
 * Get current memory pressure state.
 *
 * Use this to implement graceful degradation:
 * - `normal`: All operations allowed
 * - `warning`: Consider compacting or reducing data
 * - `critical`: Stop inserts, show warning to user
 *
 * @returns MemoryPressure object
 *
 * @example
 * ```js
 * const pressure = index.getMemoryPressure();
 * if (pressure.level === 'critical') {
 *     showMemoryWarning();
 *     disableInserts();
 * }
 * ```
 */
getMemoryPressure(): MemoryPressure;
```

**Acceptance Criteria:**
- [ ] `getMemoryPressure()` exported via wasm-bindgen
- [ ] Returns correct level based on usage
- [ ] usedBytes is reasonably accurate
- [ ] totalBytes matches actual WASM heap
- [ ] usagePercent is calculated correctly

**Test Cases:**

```javascript
// tests/wasm/memory_pressure.js

describe('getMemoryPressure', () => {
    it('should return valid structure', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const pressure = index.getMemoryPressure();

        expect(pressure).toHaveProperty('level');
        expect(pressure).toHaveProperty('usedBytes');
        expect(pressure).toHaveProperty('totalBytes');
        expect(pressure).toHaveProperty('usagePercent');

        expect(['normal', 'warning', 'critical']).toContain(pressure.level);
        expect(typeof pressure.usedBytes).toBe('number');
        expect(typeof pressure.totalBytes).toBe('number');
        expect(typeof pressure.usagePercent).toBe('number');
    });

    it('should start at normal level', async () => {
        const index = new WasmIndex({ dimensions: 768 });
        const pressure = index.getMemoryPressure();

        expect(pressure.level).toBe('normal');
        expect(pressure.usagePercent).toBeLessThan(80);
    });

    it('should increase with data', async () => {
        const index = new WasmIndex({ dimensions: 768 });

        const initialPressure = index.getMemoryPressure();

        // Insert many vectors
        for (let i = 0; i < 10000; i++) {
            const vector = new Float32Array(768).map(() => Math.random());
            index.insert(vector);
        }

        const afterPressure = index.getMemoryPressure();

        expect(afterPressure.usedBytes).toBeGreaterThan(initialPressure.usedBytes);
    });
});
```

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

### W28.3.2: Memory Thresholds and Graceful Degradation

**Objective:** Implement configurable thresholds and degradation behavior.

**Implementation:**

```rust
// src/wasm/memory.rs (continued)

/// Memory pressure configuration.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConfig {
    /// Warning threshold (default: 80%)
    pub warning_threshold: f64,
    /// Critical threshold (default: 95%)
    pub critical_threshold: f64,
    /// Auto-compact when warning threshold reached
    pub auto_compact_on_warning: bool,
    /// Block inserts when critical threshold reached
    pub block_inserts_on_critical: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            auto_compact_on_warning: false,
            block_inserts_on_critical: true,
        }
    }
}

#[wasm_bindgen]
impl WasmIndex {
    /// Configure memory pressure thresholds.
    ///
    /// # Arguments
    /// * `config` - MemoryConfig object
    ///
    /// # Example
    /// ```js
    /// index.setMemoryConfig({
    ///     warningThreshold: 70,
    ///     criticalThreshold: 90,
    ///     autoCompactOnWarning: true,
    ///     blockInsertsOnCritical: true
    /// });
    /// ```
    #[wasm_bindgen(js_name = setMemoryConfig)]
    pub fn set_memory_config(&mut self, config: JsValue) -> Result<(), JsValue> {
        let config: MemoryConfig = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

        self.memory_config = config;
        Ok(())
    }

    /// Check if inserts are allowed based on memory pressure.
    #[wasm_bindgen(js_name = canInsert)]
    pub fn can_insert(&self) -> bool {
        if !self.memory_config.block_inserts_on_critical {
            return true;
        }

        let pressure = MemoryPressure::current_with_config(&self.memory_config);
        pressure.level != MemoryPressureLevel::Critical
    }

    /// Get memory recommendation based on current state.
    #[wasm_bindgen(js_name = getMemoryRecommendation)]
    pub fn get_memory_recommendation(&self) -> Result<JsValue, JsValue> {
        let pressure = MemoryPressure::current_with_config(&self.memory_config);

        let recommendation = match pressure.level {
            MemoryPressureLevel::Normal => MemoryRecommendation {
                action: "none".to_string(),
                message: "Memory usage is healthy.".to_string(),
                can_insert: true,
                suggest_compact: false,
            },
            MemoryPressureLevel::Warning => MemoryRecommendation {
                action: "compact".to_string(),
                message: format!(
                    "Memory usage at {:.1}%. Consider running compact() to free deleted vectors.",
                    pressure.usage_percent
                ),
                can_insert: true,
                suggest_compact: self.index.needs_compaction(),
            },
            MemoryPressureLevel::Critical => MemoryRecommendation {
                action: "reduce".to_string(),
                message: format!(
                    "Memory usage critical at {:.1}%. Inserts blocked. Run compact() or delete vectors.",
                    pressure.usage_percent
                ),
                can_insert: false,
                suggest_compact: true,
            },
        };

        serde_wasm_bindgen::to_value(&recommendation)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRecommendation {
    action: String,
    message: String,
    can_insert: bool,
    suggest_compact: bool,
}
```

**TypeScript Types:**

```typescript
// To be added to pkg/edgevec.d.ts

/**
 * Memory pressure configuration.
 */
export interface MemoryConfig {
    /** Warning threshold percentage (default: 80) */
    warningThreshold?: number;
    /** Critical threshold percentage (default: 95) */
    criticalThreshold?: number;
    /** Auto-compact when warning threshold reached */
    autoCompactOnWarning?: boolean;
    /** Block inserts when critical threshold reached */
    blockInsertsOnCritical?: boolean;
}

/**
 * Memory recommendation based on current state.
 */
export interface MemoryRecommendation {
    /** Recommended action: "none", "compact", or "reduce" */
    action: 'none' | 'compact' | 'reduce';
    /** Human-readable message */
    message: string;
    /** Whether inserts are currently allowed */
    canInsert: boolean;
    /** Whether compaction would help */
    suggestCompact: boolean;
}

/**
 * Configure memory pressure thresholds.
 */
setMemoryConfig(config: MemoryConfig): void;

/**
 * Check if inserts are allowed based on memory pressure.
 */
canInsert(): boolean;

/**
 * Get memory recommendation based on current state.
 */
getMemoryRecommendation(): MemoryRecommendation;
```

**Acceptance Criteria:**
- [ ] `setMemoryConfig()` allows threshold customization
- [ ] `canInsert()` returns false when critical
- [ ] `getMemoryRecommendation()` provides actionable guidance
- [ ] Auto-compact triggers when configured

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

### W28.4.1: Integration Tests — Metadata Round-Trip

**Objective:** Verify metadata survives insert → save → load → search cycle.

**Test Implementation:**

```javascript
// tests/integration/metadata_roundtrip.test.js

import init, { WasmIndex } from '../pkg/edgevec.js';
import { openDB, deleteDB } from 'idb';

describe('Metadata Round-Trip', () => {
    const DB_NAME = 'edgevec_test_metadata';

    beforeAll(async () => {
        await init();
    });

    afterEach(async () => {
        await deleteDB(DB_NAME);
    });

    it('should persist metadata across save/load', async () => {
        // Create index with metadata
        const index1 = new WasmIndex({ dimensions: 128, useBQ: true });

        const testData = [
            { vector: new Float32Array(128).fill(0.1), metadata: { category: 'news', score: 0.9 } },
            { vector: new Float32Array(128).fill(0.2), metadata: { category: 'sports', score: 0.8 } },
            { vector: new Float32Array(128).fill(0.3), metadata: { category: 'tech', score: 0.7 } },
        ];

        const ids = [];
        for (const { vector, metadata } of testData) {
            const id = index1.insertWithMetadata(vector, metadata);
            ids.push(id);
        }

        // Save to IndexedDB
        const snapshot = index1.createSnapshot();
        const db = await openDB(DB_NAME, 1, {
            upgrade(db) {
                db.createObjectStore('snapshots');
            }
        });
        await db.put('snapshots', snapshot, 'main');
        db.close();

        // Create new index and load
        const index2 = new WasmIndex({ dimensions: 128, useBQ: true });
        const db2 = await openDB(DB_NAME, 1);
        const loadedSnapshot = await db2.get('snapshots', 'main');
        index2.loadSnapshot(loadedSnapshot);
        db2.close();

        // Verify metadata survived
        for (let i = 0; i < testData.length; i++) {
            const meta = index2.getMetadata(ids[i]);
            expect(meta).not.toBeNull();
            expect(meta.category).toBe(testData[i].metadata.category);
            expect(meta.score).toBe(testData[i].metadata.score);
        }

        // Verify filtered search works
        const query = new Float32Array(128).fill(0.15);
        const results = index2.searchFiltered(query, 'category == "news"', 10);
        expect(results.length).toBeGreaterThan(0);

        const firstMeta = index2.getMetadata(results[0].id);
        expect(firstMeta.category).toBe('news');
    });

    it('should handle deleted vectors correctly', async () => {
        const index1 = new WasmIndex({ dimensions: 128 });

        const id1 = index1.insertWithMetadata(
            new Float32Array(128).fill(0.1),
            { keep: true }
        );
        const id2 = index1.insertWithMetadata(
            new Float32Array(128).fill(0.2),
            { keep: false }
        );

        // Delete second vector
        index1.softDelete(id2);

        // Save and reload
        const snapshot = index1.createSnapshot();
        const index2 = new WasmIndex({ dimensions: 128 });
        index2.loadSnapshot(snapshot);

        // First should exist, second should not
        expect(index2.getMetadata(id1)).not.toBeNull();
        expect(index2.getMetadata(id2)).toBeNull();
    });
});
```

**Acceptance Criteria:**
- [ ] Metadata survives save/load cycle
- [ ] Filtered search works after reload
- [ ] Deleted vectors don't have metadata after reload
- [ ] All metadata types preserved (string, number, bool, array)

**Estimated Duration:** 2 hours

**Agent:** TEST_ENGINEER

---

### W28.4.2: Integration Tests — BQ Recall Validation

**Objective:** Verify BQ recall meets RFC-002 target (>0.90) in WASM.

**Test Implementation:**

```javascript
// tests/integration/bq_recall.test.js

import init, { WasmIndex } from '../pkg/edgevec.js';

describe('BQ Recall Validation', () => {
    let index;
    const NUM_VECTORS = 1000;
    const DIMENSIONS = 768;

    beforeAll(async () => {
        await init();
    });

    beforeEach(() => {
        index = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });

        // Insert random vectors
        for (let i = 0; i < NUM_VECTORS; i++) {
            const vector = new Float32Array(DIMENSIONS)
                .map(() => Math.random() * 2 - 1);
            index.insert(vector);
        }
    });

    function calculateRecall(groundTruth, results, k) {
        const gtIds = new Set(groundTruth.slice(0, k).map(r => r.id));
        const resultIds = new Set(results.slice(0, k).map(r => r.id));

        let matches = 0;
        for (const id of resultIds) {
            if (gtIds.has(id)) matches++;
        }
        return matches / k;
    }

    it('should achieve >0.90 recall with rescoring', () => {
        const NUM_QUERIES = 50;
        const K = 10;
        let totalRecall = 0;

        for (let q = 0; q < NUM_QUERIES; q++) {
            const query = new Float32Array(DIMENSIONS)
                .map(() => Math.random() * 2 - 1);

            // Ground truth: F32 search
            const f32Results = index.search(query, K);

            // BQ rescored search
            const bqResults = index.searchBQRescored(query, K, 5);

            totalRecall += calculateRecall(f32Results, bqResults, K);
        }

        const avgRecall = totalRecall / NUM_QUERIES;
        console.log(`Average recall@${K} with rescoring: ${avgRecall.toFixed(3)}`);

        // RFC-002 requirement: >0.90 recall
        expect(avgRecall).toBeGreaterThanOrEqual(0.90);
    });

    it('should show recall improvement with higher rescore factor', () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);
        const K = 10;

        const f32Results = index.search(query, K);

        const recalls = [];
        for (const factor of [1, 2, 3, 5, 10]) {
            const bqResults = index.searchBQRescored(query, K, factor);
            const recall = calculateRecall(f32Results, bqResults, K);
            recalls.push({ factor, recall });
        }

        console.log('Recall by rescore factor:', recalls);

        // Recall should generally increase with factor
        expect(recalls[4].recall).toBeGreaterThanOrEqual(recalls[0].recall);
    });

    it('should maintain recall after save/load', async () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);
        const K = 10;

        // Get recall before save
        const f32Before = index.search(query, K);
        const bqBefore = index.searchBQRescored(query, K, 5);
        const recallBefore = calculateRecall(f32Before, bqBefore, K);

        // Save and reload
        const snapshot = index.createSnapshot();
        const index2 = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });
        index2.loadSnapshot(snapshot);

        // Get recall after load
        const f32After = index2.search(query, K);
        const bqAfter = index2.searchBQRescored(query, K, 5);
        const recallAfter = calculateRecall(f32After, bqAfter, K);

        // Recall should be preserved
        expect(recallAfter).toBeCloseTo(recallBefore, 1);
    });
});
```

**Acceptance Criteria:**
- [ ] Average recall@10 >= 0.90 with rescore factor 5
- [ ] Recall improves with higher rescore factor
- [ ] Recall maintained after save/load
- [ ] Test runs in Node.js and browser

**Estimated Duration:** 2 hours

**Agent:** TEST_ENGINEER

---

## Day 3 Checklist

- [ ] W28.3.1: `getMemoryPressure()` WASM binding
- [ ] W28.3.2: Memory thresholds and graceful degradation
- [ ] W28.4.1: Metadata round-trip integration tests
- [ ] W28.4.2: BQ recall validation integration tests
- [ ] All integration tests pass
- [ ] Memory API documented
- [ ] Clippy clean

## Day 3 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `getMemoryPressure()` works | Integration test |
| Memory thresholds work | Integration test |
| Metadata round-trip passes | Integration test |
| BQ recall >0.90 | Recall benchmark |
| wasm-pack builds | `wasm-pack build` |
| No clippy warnings | `cargo clippy -- -D warnings` |

## Day 3 Handoff

After completing Day 3:

**Artifacts Generated:**
- `src/wasm/memory.rs` (new file)
- Updated `src/wasm/mod.rs`
- Updated `pkg/edgevec.d.ts`
- `tests/integration/metadata_roundtrip.test.js`
- `tests/integration/bq_recall.test.js`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 4 — Browser Demo + More Integration Tests

---

*Agent: PLANNER + WASM_SPECIALIST + TEST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2025-12-22*
