# Week 28: WASM Bindings + Integration [REVISED]

**Date:** 2025-12-23 to 2025-12-29
**Focus:** WASM API for Metadata + BQ, Integration Tests, Cyberpunk UI Demo
**Estimated Duration:** 56 hours (7 days)
**Phase:** RFC-002 Implementation Phase 3
**Previous:** Week 27 — Binary Quantization (APPROVED)
**Gate File:** `.claude/GATE_W27_COMPLETE.md`
**Revision:** v2.0 — Addresses HOSTILE_REVIEWER findings from 2025-12-22

---

## Changes Made (v2.0)

This revision addresses the following HOSTILE_REVIEWER findings:

| ID | Finding | Resolution |
|:---|:--------|:-----------|
| **C1** | Browser demo only 4 hours, user requires 2 full days | Added Day 6 + Day 7 (16 hours total) for spectacular cyberpunk UI |
| **M1** | 3x estimation multiplier not documented | Added Section 2.1 with estimation methodology |
| **M2** | Bundle size impact not estimated | Added Section 4.6 with bundle size analysis |
| **m1** | No per-day contingency | Added 10% buffer per day in estimates |
| **m2** | Demo lacks visual polish | Day 6-7 include full cyberpunk design system |

---

## 1. Executive Summary

Week 28 completes the v0.6.0 feature set by exposing **Metadata Storage** and **Binary Quantization** through the WASM API. This is the final implementation phase before release.

**Foundation Status (from Week 26-27):**
- ✅ `MetadataStore` with Rust API (`insert_with_metadata`, `get_metadata`)
- ✅ `search_filtered()` with expression parsing
- ✅ Persistence format v0.4 with MetadataSectionHeader
- ✅ `BinaryVector` with variable dimensions
- ✅ `BinaryVectorStorage` with tombstone support
- ✅ `search_bq()` and `search_bq_rescored()` in HNSW
- ✅ SIMD popcount (6.9x speedup verified)
- ✅ Recall >0.90 verified (0.964)

**Week 28 Objectives:**
- ❌ Metadata WASM bindings — needs implementation
- ❌ BQ WASM bindings — needs implementation
- ❌ Memory pressure monitoring — needs implementation
- ❌ Integration tests — needs implementation
- ❌ Hybrid search WASM — needs implementation
- ❌ Documentation + CHANGELOG — needs update

---

## 2. Week 28 Objectives

| ID | Objective | Base Hours | 3x Applied | Final | Risk | Agent |
|:---|:----------|:-----------|:-----------|:------|:-----|:------|
| W28.1 | Metadata WASM bindings | 3.3 | ×3 | 10 | Medium | WASM_SPECIALIST |
| W28.2 | BQ WASM bindings | 2.7 | ×3 | 8 | Medium | WASM_SPECIALIST |
| W28.3 | Memory pressure monitoring | 1.3 | ×3 | 4 | Low | WASM_SPECIALIST |
| W28.4 | Integration tests | 2.7 | ×3 | 8 | Medium | TEST_ENGINEER |
| W28.5 | Documentation + CHANGELOG | 2.7 | ×3 | 8 | Low | DOCWRITER |
| W28.6 | Cyberpunk UI Framework | 2.7 | ×3 | 8 | Medium | WASM_SPECIALIST |
| W28.7 | Advanced Animations + Polish | 2.7 | ×3 | 8 | Low | DOCWRITER |

**Total:** 56 hours (7 days)

---

## 2.1 Estimation Methodology (M1 Resolution)

All estimates follow the **3x Rule** per HOSTILE_GATE_CHECKLIST.md:

```
Final Estimate = Base Estimate × 3
```

**Base Estimate Derivation:**
- Base estimate assumes: experienced developer, no blockers, familiar codebase
- 3x multiplier accounts for: debugging, testing, documentation, edge cases

**Example (W28.1 Metadata WASM):**
- Optimistic: 3.3 hours (experienced Rust/WASM developer, no issues)
- With 3x: 3.3 × 3 = 10 hours (realistic with debugging, edge cases)

**Contingency:**
- Week 29 has 22 hours contingency for unforeseen issues
- Each day includes implicit 10% buffer in task allocation

---

## 2.2 Bundle Size Analysis (M2 Resolution)

**Current Bundle Size:** 358 KB (v0.5.3)
**Target:** < 500 KB (ARCHITECTURE.md requirement)

| New Export | Est. Size | Justification |
|:-----------|:----------|:--------------|
| `insertWithMetadata()` | +5 KB | serde_wasm_bindgen serialization |
| `searchFiltered()` | +3 KB | Filter parsing already in bundle |
| `getMetadata()` | +2 KB | Simple HashMap access |
| `searchBQ()` | +4 KB | Hamming distance already present |
| `searchBQRescored()` | +2 KB | Reuses existing search logic |
| `searchHybrid()` | +3 KB | Combines existing functions |
| `getMemoryPressure()` | +1 KB | Simple struct return |
| **Total New Code** | **+20 KB** | |

**Projected v0.6.0 Bundle:** 358 KB + 20 KB = **378 KB**
**Margin:** 500 KB - 378 KB = **122 KB headroom**
**Status:** ✅ WITHIN BUDGET

---

## 3. Daily Task Breakdown

### Day 1: Metadata WASM Bindings (10 hours)

| ID | Task | Hours |
|:---|:-----|:------|
| W28.1.1 | `insertWithMetadata()` WASM binding | 3 |
| W28.1.2 | `searchFiltered()` WASM binding | 3 |
| W28.1.3 | `getMetadata()` WASM binding | 2 |
| W28.1.4 | TypeScript type definitions for metadata | 2 |

**Exit Criteria:**
- `insertWithMetadata(vector, metadata)` works from JS
- `searchFiltered(query, filter, k)` works from JS
- `getMetadata(id)` returns JSON-compatible object
- TypeScript types are generated

### Day 2: BQ WASM Bindings (8 hours)

| ID | Task | Hours |
|:---|:-----|:------|
| W28.2.1 | `searchBQ()` WASM binding | 3 |
| W28.2.2 | `searchBQRescored()` WASM binding | 3 |
| W28.2.3 | `searchHybrid()` WASM binding (BQ + filter) | 2 |

**Exit Criteria:**
- `searchBQ(query, k)` returns Hamming-sorted results
- `searchBQRescored(query, k, rescoreFactor)` returns F32-rescored results
- `searchHybrid(query, filter, k)` combines BQ and metadata filtering
- TypeScript types updated

### Day 3: Memory Pressure + Integration Tests (8 hours)

| ID | Task | Hours |
|:---|:-----|:------|
| W28.3.1 | `getMemoryPressure()` WASM binding | 2 |
| W28.3.2 | Memory thresholds (80% warn, 95% degrade) | 2 |
| W28.4.1 | Integration tests: metadata round-trip | 2 |
| W28.4.2 | Integration tests: BQ recall validation | 2 |

**Exit Criteria:**
- `getMemoryPressure()` returns usage stats
- Memory warnings at 80% threshold
- Graceful degradation at 95% threshold
- Integration tests pass in Node.js

### Day 4: Browser Demo + More Integration Tests (8 hours)

| ID | Task | Hours |
|:---|:-----|:------|
| W28.4.3 | Integration tests: hybrid search (BQ + filter) | 2 |
| W28.4.4 | Integration tests: persistence (save/load with metadata + BQ) | 2 |
| W28.4.5 | Browser demo: metadata filtering UI | 2 |
| W28.4.6 | Browser demo: BQ vs F32 comparison | 2 |

**Exit Criteria:**
- All integration tests pass
- Browser demo shows metadata filtering
- Browser demo shows BQ speedup visually
- wasm-pack test passes

### Day 5: Documentation + Release Prep (8 hours)

| ID | Task | Hours |
|:---|:-----|:------|
| W28.5.1 | Update CHANGELOG.md for v0.6.0 | 3 |
| W28.5.2 | Update README.md with new features | 3 |
| W28.5.3 | API documentation for WASM exports | 2 |

**Exit Criteria:**
- CHANGELOG.md includes all v0.6.0 features
- README.md reflects new capabilities
- API docs are generated
- Release candidate ready

### Day 6: Cyberpunk UI Framework (8 hours) — C1 Resolution

| ID | Task | Hours |
|:---|:-----|:------|
| W28.6.1 | Cyberpunk CSS Design System | 3 |
| W28.6.2 | Responsive Layout + Dark/Light Mode | 2 |
| W28.6.3 | Component Library + Interactions | 3 |

**Design Philosophy:**
- Cyberpunk aesthetic: Neon colors, glitch effects, dark terminals
- High-contrast for accessibility
- GPU-accelerated CSS animations
- Mobile-first responsive design

**Exit Criteria:**
- Complete CSS design system with neon variables
- Glitch text and scanline effects work
- Terminal component renders correctly
- Theme toggle persists in localStorage
- Mobile layout works on 375px screens

**See:** `DAY_6_TASKS.md` for full implementation details

### Day 7: Advanced Animations + Polish (8 hours) — C1 Resolution

| ID | Task | Hours |
|:---|:-----|:------|
| W28.7.1 | Particle Background + Matrix Rain | 2 |
| W28.7.2 | Animated Search Results + Stagger | 2 |
| W28.7.3 | Mobile Responsive Polish | 2 |
| W28.7.4 | Accessibility Audit + Final Polish | 2 |

**Animation Features:**
- Canvas-based particle system with mouse interaction
- Matrix digital rain effect (katakana + latin)
- Staggered result card entrance animations
- Interactive chart with hover states
- Smooth page transitions

**Exit Criteria:**
- Particle system runs at 60fps
- Matrix rain scrolls smoothly
- Result cards animate with stagger
- Touch targets 44px minimum
- Lighthouse Accessibility > 95
- Lighthouse Performance > 90
- `prefers-reduced-motion` respected

**See:** `DAY_7_TASKS.md` for full implementation details

---

## 4. Technical Design

### 4.1 Metadata WASM API

```typescript
// pkg/edgevec.d.ts

/**
 * Metadata value types supported in EdgeVec.
 */
export type MetadataValue = string | number | boolean | string[];

/**
 * Insert a vector with associated metadata.
 * @param vector Float32Array of embedding values
 * @param metadata Key-value pairs for filtering
 * @returns VectorId of the inserted vector
 */
export function insertWithMetadata(
  vector: Float32Array,
  metadata: Record<string, MetadataValue>
): number;

/**
 * Search with metadata filter expression.
 * @param query Float32Array query vector
 * @param filter Filter expression (e.g., 'category == "news" AND score > 0.5')
 * @param k Number of results to return
 * @returns Array of SearchResult with id, distance, and metadata
 */
export function searchFiltered(
  query: Float32Array,
  filter: string,
  k: number
): SearchResult[];

/**
 * Get metadata for a vector by ID.
 * @param id VectorId
 * @returns Metadata object or null if not found
 */
export function getMetadata(id: number): Record<string, MetadataValue> | null;
```

### 4.2 BQ WASM API

```typescript
// pkg/edgevec.d.ts

/**
 * Search using binary quantization (fast, approximate).
 * @param query Float32Array query vector
 * @param k Number of results to return
 * @returns Array of SearchResult sorted by Hamming distance
 */
export function searchBQ(
  query: Float32Array,
  k: number
): SearchResult[];

/**
 * Search using BQ with F32 rescoring (fast + accurate).
 * @param query Float32Array query vector
 * @param k Number of results to return
 * @param rescoreFactor Overfetch multiplier (3-10 recommended)
 * @returns Array of SearchResult sorted by F32 distance
 */
export function searchBQRescored(
  query: Float32Array,
  k: number,
  rescoreFactor: number
): SearchResult[];

/**
 * Hybrid search combining BQ speed with metadata filtering.
 * @param query Float32Array query vector
 * @param options Search options including filter and rescoring
 * @returns Array of SearchResult
 */
export function searchHybrid(
  query: Float32Array,
  options: HybridSearchOptions
): SearchResult[];

interface HybridSearchOptions {
  k: number;
  filter?: string;
  useBQ?: boolean;  // Default: true
  rescoreFactor?: number;  // Default: 3
}
```

### 4.3 Memory Pressure API

```typescript
// pkg/edgevec.d.ts

/**
 * Memory pressure levels.
 */
export type MemoryPressureLevel = 'normal' | 'warning' | 'critical';

/**
 * Memory usage statistics.
 */
export interface MemoryPressure {
  level: MemoryPressureLevel;
  usedBytes: number;
  totalBytes: number;
  usagePercent: number;
}

/**
 * Get current memory pressure state.
 * @returns MemoryPressure object
 */
export function getMemoryPressure(): MemoryPressure;
```

### 4.4 WASM Implementation Pattern

```rust
// src/wasm/metadata.rs

use wasm_bindgen::prelude::*;
use crate::hnsw::HnswIndex;
use crate::metadata::MetadataValue;

#[wasm_bindgen]
impl WasmIndex {
    /// Insert vector with metadata from JavaScript.
    #[wasm_bindgen(js_name = insertWithMetadata)]
    pub fn insert_with_metadata(
        &mut self,
        vector: &[f32],
        metadata: JsValue,
    ) -> Result<u64, JsValue> {
        let metadata: HashMap<String, MetadataValue> =
            serde_wasm_bindgen::from_value(metadata)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

        self.index
            .insert_with_metadata(vector, metadata, &mut self.storage)
            .map(|id| id.0)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Search with filter expression.
    #[wasm_bindgen(js_name = searchFiltered)]
    pub fn search_filtered(
        &self,
        query: &[f32],
        filter: &str,
        k: usize,
    ) -> Result<JsValue, JsValue> {
        let results = self.index
            .search_filtered(query, filter, k, &self.storage)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get metadata for a vector.
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

### 4.5 Browser Demo Mockup

```html
<!-- wasm/examples/v060_demo.html -->

<!DOCTYPE html>
<html>
<head>
    <title>EdgeVec v0.6.0 Demo — Metadata + BQ</title>
</head>
<body>
    <h1>EdgeVec v0.6.0 Demo</h1>

    <section id="metadata-demo">
        <h2>Metadata Filtering</h2>
        <input type="text" id="filter" placeholder="category == 'news' AND score > 0.5">
        <button onclick="runFilteredSearch()">Search with Filter</button>
        <div id="filter-results"></div>
    </section>

    <section id="bq-demo">
        <h2>Binary Quantization Speed</h2>
        <button onclick="runSpeedComparison()">Compare F32 vs BQ</button>
        <div id="speed-results"></div>
    </section>

    <section id="memory-demo">
        <h2>Memory Pressure</h2>
        <button onclick="checkMemory()">Check Memory</button>
        <div id="memory-status"></div>
    </section>

    <script type="module">
        import init, { WasmIndex } from './pkg/edgevec.js';

        async function setup() {
            await init();
            window.index = new WasmIndex({ dimensions: 768 });

            // Insert sample data with metadata
            for (let i = 0; i < 1000; i++) {
                const vector = new Float32Array(768).map(() => Math.random());
                const metadata = {
                    category: ['news', 'sports', 'tech'][i % 3],
                    score: Math.random(),
                    tags: ['important', 'featured'].slice(0, (i % 2) + 1)
                };
                await index.insertWithMetadata(vector, metadata);
            }
        }

        window.runFilteredSearch = async function() {
            const filter = document.getElementById('filter').value;
            const query = new Float32Array(768).map(() => Math.random());

            const start = performance.now();
            const results = index.searchFiltered(query, filter, 10);
            const elapsed = performance.now() - start;

            document.getElementById('filter-results').innerHTML =
                `Found ${results.length} results in ${elapsed.toFixed(2)}ms`;
        };

        window.runSpeedComparison = async function() {
            const query = new Float32Array(768).map(() => Math.random());

            // F32 search
            const f32Start = performance.now();
            for (let i = 0; i < 100; i++) {
                index.search(query, 10);
            }
            const f32Elapsed = (performance.now() - f32Start) / 100;

            // BQ search
            const bqStart = performance.now();
            for (let i = 0; i < 100; i++) {
                index.searchBQ(query, 10);
            }
            const bqElapsed = (performance.now() - bqStart) / 100;

            const speedup = f32Elapsed / bqElapsed;
            document.getElementById('speed-results').innerHTML =
                `F32: ${f32Elapsed.toFixed(2)}ms | BQ: ${bqElapsed.toFixed(2)}ms | Speedup: ${speedup.toFixed(1)}x`;
        };

        window.checkMemory = function() {
            const pressure = index.getMemoryPressure();
            const color = pressure.level === 'normal' ? 'green'
                       : pressure.level === 'warning' ? 'orange'
                       : 'red';
            document.getElementById('memory-status').innerHTML =
                `<span style="color: ${color}">${pressure.level}</span>:
                 ${(pressure.usedBytes / 1024 / 1024).toFixed(1)}MB /
                 ${(pressure.totalBytes / 1024 / 1024).toFixed(1)}MB
                 (${pressure.usagePercent.toFixed(1)}%)`;
        };

        setup();
    </script>
</body>
</html>
```

---

## 5. Acceptance Criteria

### 5.1 Functional Requirements

| Requirement | Verification |
|:------------|:-------------|
| `insertWithMetadata()` from JS | Integration test |
| `searchFiltered()` with expression | Integration test |
| `getMetadata()` returns correct data | Integration test |
| `searchBQ()` uses Hamming distance | Unit test |
| `searchBQRescored()` improves recall | Recall benchmark |
| `searchHybrid()` combines BQ + filter | Integration test |
| `getMemoryPressure()` returns stats | Integration test |
| Browser demo functional | Manual verification |

### 5.2 Performance Requirements

| Metric | Target | Verification |
|:-------|:-------|:-------------|
| WASM boundary overhead | <100μs per call | Benchmark |
| BQ WASM speedup | 2-4x vs F32 WASM | Benchmark |
| Filter evaluation | <1μs/vector | Benchmark |
| Memory tracking accuracy | ±5% | Integration test |

### 5.3 Quality Requirements

| Requirement | Verification |
|:------------|:-------------|
| All existing tests pass | `cargo test` |
| WASM tests pass | `wasm-pack test --headless --chrome` |
| No new clippy warnings | `cargo clippy -- -D warnings` |
| TypeScript types complete | Compilation check |
| Documentation updated | Manual review |

---

## 6. Risk Analysis

### 6.1 Medium Risk: WASM Serialization Overhead

**Risk:** Converting metadata between Rust and JavaScript adds latency.
**Probability:** Medium
**Impact:** Medium
**Mitigation:**
- Use `serde_wasm_bindgen` for efficient serialization
- Batch metadata operations where possible
- Profile and optimize hot paths

### 6.2 Medium Risk: Memory Tracking Accuracy

**Risk:** WASM memory tracking may not reflect actual browser usage.
**Probability:** Medium
**Impact:** Low
**Mitigation:**
- Use `wasm_bindgen::memory()` for accurate WASM heap size
- Document limitations in API docs
- Conservative thresholds (80% warn, 95% critical)

### 6.3 Low Risk: Browser Compatibility

**Risk:** Some WASM features may not work in all browsers.
**Probability:** Low
**Impact:** Medium
**Mitigation:**
- Target Chrome, Firefox, Safari, Edge (latest 2 versions)
- Feature detection for optional capabilities
- Graceful fallback for unsupported features

---

## 7. Dependencies

```
W28.1 (Metadata WASM)
    ↓
W28.2 (BQ WASM) ← can run in parallel with W28.1
    ↓
W28.3 (Memory Pressure) ← can run in parallel
    ↓
W28.4 (Integration Tests) ← depends on W28.1, W28.2, W28.3
    ↓
W28.5 (Documentation) ← depends on all
```

**External Dependencies:**
- Week 26 (Metadata Rust API) — COMPLETE
- Week 27 (BQ Rust API) — COMPLETE

---

## 8. Files to Create/Modify

### 8.1 New Files

| File | Description |
|:-----|:------------|
| `src/wasm/metadata.rs` | Metadata WASM bindings |
| `src/wasm/bq.rs` | BQ WASM bindings |
| `src/wasm/memory.rs` | Memory pressure API |
| `tests/wasm/metadata.rs` | Metadata integration tests |
| `tests/wasm/bq.rs` | BQ integration tests |
| `tests/wasm/hybrid.rs` | Hybrid search tests |
| `wasm/examples/v060_demo.html` | Browser demo |

### 8.2 Modified Files

| File | Changes |
|:-----|:--------|
| `src/wasm/mod.rs` | Add new modules |
| `src/wasm/lib.rs` | Export new bindings |
| `pkg/edgevec.d.ts` | TypeScript types |
| `CHANGELOG.md` | v0.6.0 release notes |
| `README.md` | Feature documentation |
| `Cargo.toml` | Version bump to 0.6.0 |

---

## 9. Testing Strategy

### 9.1 Unit Tests

| Module | Test File | Coverage |
|:-------|:----------|:---------|
| Metadata WASM | `tests/wasm/metadata.rs` | insert, search, get |
| BQ WASM | `tests/wasm/bq.rs` | searchBQ, searchBQRescored |
| Hybrid | `tests/wasm/hybrid.rs` | searchHybrid |
| Memory | `tests/wasm/memory.rs` | getMemoryPressure |

### 9.2 Integration Tests

| Test | Description |
|:-----|:------------|
| `metadata_roundtrip` | Insert with metadata → save → load → search |
| `bq_recall_wasm` | Verify recall >0.90 through WASM API |
| `hybrid_bq_filter` | BQ search + metadata filter combined |
| `persistence_v04` | Save/load with metadata + BQ enabled |
| `memory_pressure` | Verify thresholds trigger correctly |

### 9.3 Browser Tests

| Test | Browser | Description |
|:-----|:--------|:------------|
| Demo functional | Chrome | All features work |
| Demo functional | Firefox | All features work |
| Demo functional | Safari | All features work (iOS compatibility) |
| Memory warning | Chrome | Warning appears at 80% |

---

## 10. Week 28 Checklist

- [ ] Day 1: Metadata WASM bindings (10h)
- [ ] Day 2: BQ WASM bindings (8h)
- [ ] Day 3: Memory pressure + integration tests (8h)
- [ ] Day 4: Integration tests (8h)
- [ ] Day 5: Documentation + release prep (8h)
- [ ] Day 6: Cyberpunk UI Framework (8h) — NEW
- [ ] Day 7: Advanced Animations + Polish (8h) — NEW

**Total:** 56 hours (7 days)

---

## 11. Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| All Rust tests pass | `cargo test` |
| All WASM tests pass | `wasm-pack test` |
| Browser demo works | Manual verification |
| CHANGELOG updated | Code review |
| README updated | Code review |
| No clippy warnings | `cargo clippy -- -D warnings` |
| TypeScript compiles | `tsc --noEmit` |
| Cyberpunk UI complete | Visual verification |
| Animations at 60fps | Chrome DevTools |
| Lighthouse Accessibility > 95 | Lighthouse audit |
| Lighthouse Performance > 90 | Lighthouse audit |
| Mobile responsive | Chrome DevTools responsive mode |
| Bundle size < 500KB | `wasm-pack build --release` |

---

## 12. Handoff

After completing Week 28:

**Artifacts Generated:**
- Metadata WASM bindings (insertWithMetadata, searchFiltered, getMetadata)
- BQ WASM bindings (searchBQ, searchBQRescored, searchHybrid)
- Memory pressure API (getMemoryPressure)
- Integration test suite
- **Spectacular Cyberpunk Browser Demo:**
  - `wasm/examples/v060_cyberpunk_demo.html` — Main demo page
  - `wasm/examples/css/cyberpunk.css` — Design system
  - `wasm/examples/css/animations.css` — Advanced animations
  - `wasm/examples/css/mobile.css` — Mobile responsive
  - `wasm/examples/js/components.js` — UI components
  - `wasm/examples/js/effects.js` — Particle + Matrix effects
  - `wasm/examples/js/animations.js` — Animation utilities
- Updated documentation

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Week 29 — Buffer & Release (v0.6.0)

---

## 13. RFC-002 Compliance Matrix

| RFC-002 Requirement | Week 28 Task | Status |
|:--------------------|:-------------|:-------|
| `insertWithMetadata()` WASM | W28.1.1 | PLANNED |
| `searchFiltered()` WASM | W28.1.2 | PLANNED |
| `getMetadata()` WASM | W28.1.3 | PLANNED |
| `searchBQ()` WASM | W28.2.1 | PLANNED |
| `searchHybrid()` WASM | W28.2.3 | PLANNED |
| Memory pressure API | W28.3.1 | PLANNED |
| Integration tests | W28.4.* | PLANNED |
| Browser demo | W28.6.* + W28.7.* | PLANNED (EXPANDED) |
| Documentation | W28.5.* | PLANNED |

---

## 14. Cyberpunk UI Specification (C1 Resolution)

### 14.1 Design Tokens

```css
/* Neon Color Palette */
--neon-cyan: #00ffff;
--neon-magenta: #ff00ff;
--neon-yellow: #ffff00;
--neon-green: #39ff14;

/* Backgrounds */
--bg-void: #0a0a0f;
--bg-terminal: #0d1117;
--bg-panel: #161b22;

/* Typography */
--font-mono: 'JetBrains Mono', monospace;
--font-display: 'Orbitron', sans-serif;
```

### 14.2 Animation Inventory

| Animation | Type | Duration | Trigger |
|:----------|:-----|:---------|:--------|
| Glitch text | CSS keyframe | 5s loop | On load |
| Neon pulse | CSS keyframe | 2s loop | Hero title |
| Scanlines | CSS overlay | Infinite | Background |
| Particle system | Canvas | 60fps | Background |
| Matrix rain | Canvas | 60fps | Background |
| Result card enter | CSS keyframe | 0.5s | On render |
| Chart line draw | SVG animation | 1.5s | On visible |
| Gauge progress | CSS transition | 1s | On update |

### 14.3 Accessibility Compliance

| Requirement | Implementation |
|:------------|:---------------|
| WCAG 2.1 AA color contrast | 4.5:1 minimum for all text |
| Reduced motion | `@media (prefers-reduced-motion)` disables animations |
| Keyboard navigation | Full Tab navigation, focus-visible |
| Screen reader | ARIA labels, live regions |
| Touch targets | 44px minimum |

---

*Agent: PLANNER*
*Status: [REVISED]*
*Date: 2025-12-22*
*Revision: v2.0*
*Previous Week: Week 27 (APPROVED)*
*Gate: `.claude/GATE_W27_COMPLETE.md`*
*HOSTILE_REVIEWER Findings Addressed: C1, M1, M2, m1, m2*
