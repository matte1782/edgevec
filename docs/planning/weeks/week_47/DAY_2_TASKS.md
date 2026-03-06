# Week 47 — Day 2 Tasks (Tuesday, Apr 8)

**Date:** 2026-04-08
**Focus:** WASM Benchmark Harness + Playwright Integration
**Agents:** BENCHMARK_SCIENTIST, WASM_SPECIALIST
**Status:** COMPLETE

---

## Day Objective

Build the WASM PQ benchmark harness (HTML + JS) and verify Playwright can drive it programmatically. This unblocks the G2 WASM ADC benchmark on Day 3.

**Success Criteria:**
- `tests/wasm/pq_bench.html` loads WASM module and displays "PQ WASM Ready"
- `tests/wasm/pq_bench.js` implements `benchADC()` and `benchTraining()` functions
- Playwright can navigate to the harness and execute benchmarks via `browser_evaluate`

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `src/wasm/mod.rs` — the 3 new PQ WASM exports from Day 1
- [ ] `benches/pq_bench.rs` — existing native benchmark patterns (reuse, don't rebuild)
- [ ] `docs/research/PQ_BENCHMARK_PLAN.md` Section 1.3 — reproducibility protocol
- [ ] `docs/research/PQ_BENCHMARK_PLAN.md` Section 2.3 — ground truth pre-computation

**Playwright MCP pre-check (m5 fix):**
```
Verify Playwright MCP plugin is available by attempting:
- browser_navigate to about:blank
If unavailable → use manual Chrome fallback (see Decision Tree below)
```

---

## Tasks

### W47.2a: Create WASM PQ Benchmark Page (2h) — BENCHMARK_SCIENTIST

**Dependency:** W47.1b (WASM exports exist)

**Commands:**
```bash
# Build WASM release first
wasm-pack build --release

# Verify pkg/ contains the module
ls pkg/edgevec_bg.wasm pkg/edgevec.js
```

**File:** `tests/wasm/pq_bench.html`

**Expected Output:** HTML page that:
1. Imports WASM module from `../../pkg/edgevec.js`
2. Calls `train_pq()` with 1K synthetic vectors on load
3. Logs "PQ WASM Ready" to console on success
4. Displays benchmark controls (or exposes JS API for Playwright)

**Acceptance:**
- [ ] `wasm-pack build --release` succeeds
- [ ] HTML page loads in Chrome without errors
- [ ] Console shows "PQ WASM Ready" after WASM init
- [ ] No CORS errors (served via `npx serve .`)

---

### W47.2b: Create Benchmark Harness JS (2.5h) — BENCHMARK_SCIENTIST

**Dependency:** W47.2a complete

**File:** `tests/wasm/pq_bench.js`

**Functions to implement:**

```javascript
// 1. ADC benchmark (for G2)
async function benchADC(nCodes, nQueries, iterations) {
  // - Train codebook on nCodes synthetic vectors (768D, M=8, Ksub=256)
  // - Encode nCodes vectors to PQ codes
  // - For each iteration:
  //   - Run nQueries ADC scans over nCodes
  //   - Record time via performance.now()
  // - Report: { median_ns_per_candidate, p99_ns_per_candidate, total_ms }
  // - Throughput: nQueries * nCodes candidates per iteration (lesson #65)
}

// 2. Training benchmark (for G4 WASM)
async function benchTraining(nVectors, iterations) {
  // - For each iteration:
  //   - Generate nVectors synthetic vectors (seeded for reproducibility)
  //   - Train codebook (M=8, Ksub=256, max_iters=5)
  //   - Record wall-clock time
  // - Report: { median_s, p99_s }
}

// 3. Ground truth (for recall — cost excluded from timing)
function computeGroundTruth(vectors, queries, k) {
  // - Brute-force L2 distance for each query
  // - Return top-k indices per query
  // - Pre-computed ONCE, cached for all recall measurements
}
```

**Expected Output:** JS module with 3 exported functions returning JSON timing results.

**Acceptance:**
- [ ] `benchADC(1000, 1, 1)` returns valid JSON with `median_ns_per_candidate`
- [ ] `benchTraining(1000, 1)` returns valid JSON with `median_s`
- [ ] `computeGroundTruth(vectors, queries, 10)` returns correct indices
- [ ] 3 warmup iterations discarded before measurement (per PQ_BENCHMARK_PLAN Section 1.3)

---

### W47.2c: Playwright Integration Verification (1.5h) — WASM_SPECIALIST

**Dependency:** W47.2a, W47.2b complete

**Decision Tree:**
- **If Playwright MCP available:**
  ```
  1. npx serve . -l 8080           # Serve project root
  2. browser_navigate → http://localhost:8080/tests/wasm/pq_bench.html
  3. browser_evaluate('benchADC(1000, 1, 1)')  → expect JSON result
  4. browser_evaluate('navigator.userAgent')    → capture Chrome version
  5. browser_console_messages                   → verify no errors
  ```
- **If Playwright MCP unavailable:**
  ```
  1. npx serve . -l 8080
  2. Open Chrome manually → navigate to URL
  3. Open DevTools Console
  4. Run: benchADC(1000, 1, 1)
  5. Copy-paste JSON result
  6. Document: "Manual Chrome profiling — Playwright unavailable"
  ```
- **If neither works:**
  ```
  1. wasm-pack test --node
  2. Use Node.js performance.now() for timing
  3. Document: "Node.js WASM — lower fidelity than Chrome"
  ```

**Expected Output:** Playwright (or fallback) successfully executes `benchADC(1000, 1, 1)` and returns timing JSON.

**Acceptance:**
- [ ] Benchmark harness accessible via localhost
- [ ] `benchADC(1000, 1, 1)` returns valid timing result
- [ ] Chrome version captured and documented
- [ ] No console errors during execution

---

## Day 2 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6h |
| New files | 2 (`pq_bench.html`, `pq_bench.js`) |
| Playwright verified | Yes (or documented fallback) |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W47.2a | 2h | | |
| W47.2b | 2.5h | | |
| W47.2c | 1.5h | | |
| **Total** | **6h** | | |

---

## Handoff to Day 3

**Codebase state at EOD:**
- WASM benchmark harness functional at `tests/wasm/pq_bench.html`
- Playwright (or fallback) verified to execute benchmarks
- Chrome version documented

**Day 3 prerequisites satisfied:**
- [ ] `benchADC(nCodes, nQueries, iterations)` callable from Playwright/Chrome
- [ ] `benchTraining(nVectors, iterations)` callable from Playwright/Chrome

**Day 3 focus:** G2 WASM ADC benchmark (100K) + training optimization start + mid-week hostile review

---

**END OF DAY 2 TASKS**
