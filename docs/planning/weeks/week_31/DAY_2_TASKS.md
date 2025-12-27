# Week 31 Day 2: Filter Playground Completion

**Date:** 2025-12-27
**Focus:** Complete filter playground enhancement with live WASM sandbox
**Estimated Duration:** 4 hours
**Priority:** P0 — Key demo for v0.7.0

---

## Objectives

1. Verify existing filter-playground.html status
2. Add LiveSandbox class for real EdgeVec WASM execution
3. Add performance timing display
4. Update version references to v0.7.0
5. Cross-browser testing

---

## Context

**Existing Asset:** `wasm/examples/filter-playground.html` (1709 lines)

**What Already Exists:**
- Full cyberpunk theme (JetBrains Mono, Orbitron)
- Theme toggle (dark/light)
- Accessibility (ARIA, skip links)
- 16 example filters
- AST/JSON/Info output tabs
- Debounced real-time parsing
- Error display with suggestions
- Responsive design

**What's Needed:**
- LiveSandbox class for actual WASM execution
- Sample data generation
- Performance timing display
- Version update to v0.7.0

---

## Tasks

### W31.2.1: Verify Existing Filter Playground

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Checklist:**

| Feature | Expected | Verification |
|:--------|:---------|:-------------|
| File exists | Yes | `wasm/examples/filter-playground.html` |
| Loads in browser | Yes | Open locally |
| Filter parsing works | Yes | Type filter, see AST |
| Example buttons work | Yes | Click each example |
| Theme toggle works | Yes | Switch dark/light |
| Version shown | v0.6.0 or earlier | Check header |

**Acceptance Criteria:**
- [ ] Existing functionality verified
- [ ] Baseline established
- [ ] Gaps identified

---

### W31.2.2: Add LiveSandbox Class

**Duration:** 1.5 hours
**Agent:** WASM_SPECIALIST

**File:** `wasm/examples/filter-playground.html`

**Code to Add (in `<script type="module">` section):**

```javascript
// LiveSandbox - Real EdgeVec WASM execution
class LiveSandbox {
    constructor() {
        this.db = null;
        this.initialized = false;
        this.vectorCount = 0;
    }

    async init() {
        try {
            const { default: init, EdgeVec } = await import('../../pkg/edgevec.js');
            await init();
            this.db = new EdgeVec({ dimensions: 128 });
            this.initialized = true;
            return true;
        } catch (e) {
            console.error('LiveSandbox init failed:', e);
            return false;
        }
    }

    async loadSampleData(count = 1000) {
        if (!this.initialized) await this.init();

        const categories = ['electronics', 'clothing', 'books', 'home', 'sports'];
        const tags = ['featured', 'sale', 'new', 'popular', 'limited'];

        for (let i = 0; i < count; i++) {
            // Generate random embedding
            const embedding = new Float32Array(128);
            for (let j = 0; j < 128; j++) {
                embedding[j] = Math.random() * 2 - 1;
            }

            // Generate sample metadata
            const metadata = {
                category: categories[i % categories.length],
                price: Math.floor(Math.random() * 900) + 100,
                rating: Math.round((Math.random() * 2 + 3) * 10) / 10,
                in_stock: Math.random() > 0.2,
                tags: [tags[Math.floor(Math.random() * tags.length)]]
            };

            this.db.insertWithMetadata(embedding, metadata);
            this.vectorCount++;
        }

        return this.vectorCount;
    }

    async executeFilter(filterExpr, k = 10) {
        if (!this.initialized) {
            return { error: 'Sandbox not initialized' };
        }

        // Generate random query vector
        const query = new Float32Array(128);
        for (let i = 0; i < 128; i++) {
            query[i] = Math.random() * 2 - 1;
        }

        const start = performance.now();
        try {
            const results = filterExpr
                ? this.db.searchFiltered(query, k, { filter: filterExpr })
                : this.db.search(query, k);
            const elapsed = performance.now() - start;

            return {
                results: results.length,
                elapsed: elapsed.toFixed(3),
                matches: results
            };
        } catch (e) {
            return { error: e.message };
        }
    }

    getStats() {
        return {
            initialized: this.initialized,
            vectorCount: this.vectorCount,
            hasBQ: this.db?.hasBQ() || false
        };
    }
}

// Global sandbox instance
window.sandbox = new LiveSandbox();
```

**UI to Add:**

```html
<!-- Add after filter input section -->
<div class="sandbox-panel">
    <h3>🔬 Live Sandbox</h3>
    <div class="sandbox-controls">
        <button id="init-sandbox" onclick="initSandbox()">Initialize WASM</button>
        <button id="load-data" onclick="loadData()" disabled>Load 1000 Vectors</button>
        <button id="run-filter" onclick="runFilter()" disabled>Execute Filter</button>
    </div>
    <div id="sandbox-status" class="sandbox-status">
        Click "Initialize WASM" to start
    </div>
    <div id="sandbox-results" class="sandbox-results"></div>
</div>
```

**Acceptance Criteria:**
- [ ] LiveSandbox class implemented
- [ ] Initialize button works
- [ ] Load data button works
- [ ] Execute filter button works
- [ ] Results displayed

---

### W31.2.3: Add Performance Timing Display

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**UI Enhancement:**

```html
<div class="performance-panel">
    <h4>⚡ Performance</h4>
    <table class="perf-table">
        <tr>
            <td>Parse Time:</td>
            <td id="parse-time">-</td>
        </tr>
        <tr>
            <td>Execute Time:</td>
            <td id="execute-time">-</td>
        </tr>
        <tr>
            <td>Results:</td>
            <td id="result-count">-</td>
        </tr>
        <tr>
            <td>Vectors Scanned:</td>
            <td id="vectors-scanned">-</td>
        </tr>
    </table>
</div>
```

**JavaScript:**

```javascript
function updatePerformance(parseMs, executeMs, resultCount, totalVectors) {
    document.getElementById('parse-time').textContent = `${parseMs}ms`;
    document.getElementById('execute-time').textContent = `${executeMs}ms`;
    document.getElementById('result-count').textContent = resultCount;
    document.getElementById('vectors-scanned').textContent = totalVectors;
}
```

**Acceptance Criteria:**
- [ ] Parse time displayed
- [ ] Execute time displayed
- [ ] Result count shown
- [ ] Updates on each filter execution

---

### W31.2.4: Update Version References

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Updates Required:**

1. **Header version:**
```html
<span class="version">v0.7.0</span>
```

2. **Import path (if needed):**
```javascript
import { default: init, EdgeVec } from '../../pkg/edgevec.js?v=0.7.0';
```

3. **Footer:**
```html
<footer>EdgeVec v0.7.0 — WASM-Native Vector Database</footer>
```

4. **Meta tags:**
```html
<meta name="version" content="0.7.0">
```

**Acceptance Criteria:**
- [ ] All version references updated to v0.7.0
- [ ] No v0.6.0 references remaining
- [ ] Cache busting query param added to imports

---

### W31.2.5: Cross-Browser Testing

**Duration:** 1 hour
**Agent:** TEST_ENGINEER

**Test Matrix:**

| Browser | Test | Expected |
|:--------|:-----|:---------|
| Chrome | Initialize WASM | ✅ Success |
| Chrome | Load 1000 vectors | ✅ <2s |
| Chrome | Execute filter | ✅ <50ms |
| Firefox | Initialize WASM | ✅ Success |
| Firefox | Load 1000 vectors | ✅ <2s |
| Firefox | Execute filter | ✅ <50ms |
| Safari macOS | Initialize WASM | ✅ Success |
| Safari macOS | Execute filter | ✅ <50ms |
| Safari iOS | Initialize WASM | ✅ Success (scalar) |

**Test Procedure:**

```bash
# Start local server
npx serve wasm/examples

# Open http://localhost:3000/filter-playground.html
# Test each browser
```

**Test Cases:**

1. **Basic Filter:** `category = "electronics"`
2. **Compound Filter:** `price > 100 AND rating >= 4.0`
3. **Complex Filter:** `category = "electronics" AND (price < 500 OR tags ANY ["sale"])`
4. **Invalid Filter:** `invalid syntax here` (should show error)

**Acceptance Criteria:**
- [ ] Chrome passes all tests
- [ ] Firefox passes all tests
- [ ] Safari macOS passes all tests
- [ ] Safari iOS loads (scalar mode)
- [ ] Error handling works

---

## Day 2 Exit Criteria

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| LiveSandbox works | Initialize + load + execute | [ ] |
| Performance timing | Shows parse/execute times | [ ] |
| Version v0.7.0 | All references updated | [ ] |
| Chrome tested | All tests pass | [ ] |
| Firefox tested | All tests pass | [ ] |
| Safari tested | All tests pass | [ ] |

---

**Day 2 Total:** 4 hours
**Agent:** WASM_SPECIALIST, TEST_ENGINEER
