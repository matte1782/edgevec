# Week 48 — Day 4 Tasks (Thursday, Apr 17)

**Date:** 2026-04-17
**Focus:** In-Browser Entity-Enhanced RAG Demo
**Agents:** WASM_SPECIALIST, TEST_ENGINEER
**Status:** PENDING

---

## Day Objective

Build a single-page in-browser demo that loads EdgeVec WASM, searches 1000 SQuAD paragraphs with entity metadata boosting, and demonstrates the boost ON/OFF difference. Zero API calls, zero frameworks.

**Success Criteria:**
- `demo/entity-rag/index.html` loads in Chrome via `npx serve`
- Loading indicator shows "Building index... N/1000" during insertion
- Index build time for 1000 vectors at 384D < 3 seconds in Chrome
- User selects sample query from dropdown, sees top-10 results
- "Entity Boost ON/OFF" toggle shows different ranking
- Search latency < 100ms displayed in UI
- WASM bundle < 500KB
- No console errors
- All Rust regression tests pass

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `demo/entity-rag/data.json` — data format (documents array + sample_queries array)
- [ ] `src/wasm/mod.rs` — existing WASM API (EdgeVecIndex create, insert, search patterns)
- [ ] `src/filter/boost.rs` — MetadataBoost struct
- [ ] Day 2 hostile review verdict — confirm GO before proceeding

---

## Tasks

### W48.4a: Build WASM Release (0.5h) — WASM_SPECIALIST

**Dependency:** W48.2a complete (WASM boost export from Day 2)

**Commands:**
```bash
wasm-pack build --release --target web
ls -la pkg/edgevec_bg.wasm    # Must be < 500KB
```

**Decision Tree:**
- If `wasm-pack build` fails -> check Day 2 WASM compilation, fix errors
- If bundle > 500KB -> run `wasm-opt -O3 pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm` and re-check

**Acceptance:**
- [ ] `wasm-pack build --release --target web` succeeds
- [ ] `pkg/edgevec_bg.wasm` exists and < 500KB
- [ ] `pkg/edgevec.js` exists (JS glue)

---

### W48.4b: Create `demo/entity-rag/index.html` — Demo Page (5h) — WASM_SPECIALIST

**Dependency:** W48.4a complete, W48.3b complete (data.json exists)

**Architecture:**
```
index.html
├── <head>: title, minimal CSS (inline)
├── <body>:
│   ├── Header: "EdgeVec Entity-Enhanced RAG Demo"
│   ├── Query selector: <select> with 10 sample queries
│   ├── Boost toggle: <input type="checkbox" id="boost-toggle">
│   ├── Search button: "Search"
│   ├── Results area: <div id="results">
│   │   └── Result cards with:
│   │       ├── Score badge
│   │       ├── Text snippet
│   │       └── Entity badges (colored by type)
│   ├── Status bar: search latency, doc count, bundle size
│   └── Footer: "100% in-browser. Zero API calls."
└── <script type="module">: all JS logic inline
```

**JS Logic Flow:**
```javascript
// 1. On page load:
//    a. Fetch and parse data.json
//    b. Import WASM module from ../../pkg/edgevec.js
//    c. Create EdgeVecIndex (dim=384)
//    d. Insert all 1000 embeddings with progress indicator:
//       - Show "Building index... N/1000" during insertion
//       - Use requestAnimationFrame or setTimeout batching to keep UI responsive
//    e. Store metadata in a Map<id, metadata>
//    f. Populate query dropdown
//    g. Display "Ready" status (with index build time)

// 2. On search click:
//    a. Get selected query embedding
//    b. performance.now() start
//    c. If boost ON:
//       - Call searchWithBoost(query, 10, boosts_json, null)
//       - boosts_json based on query: e.g., for "capital of France"
//         -> boost entity_type=GPE with weight=0.5
//    d. If boost OFF:
//       - Call standard search(query, 10)
//    e. performance.now() end
//    f. Display results with entity badges
//    g. Display latency in status bar

// 3. Entity badge colors:
//    ORG = blue, PERSON = green, GPE = red, DATE = yellow,
//    NORP = purple, other = gray
```

**Design Decision — Query-Specific Boosts:**
Each sample query has a pre-defined boost configuration that makes sense for that query:
- "Who invented the telephone?" -> boost PERSON entities, weight=0.3
- "What is the capital of France?" -> boost GPE entities, weight=0.5
- "When did WWII end?" -> boost DATE entities, weight=0.3
- Default: boost entities matching query keywords, weight=0.2

Store boost configs alongside sample queries in data.json or inline in HTML.

**CSS Styling (inline, minimal):**
- Max-width 800px, centered
- Result cards with border, padding
- Entity badges: rounded, colored background, white text
- Toggle switch styled
- Mobile-responsive (flexbox)

**Commands:**
```bash
# Serve demo
npx serve demo/entity-rag/

# Open in Chrome
# Navigate to http://localhost:3000 (or whatever port)
```

**Acceptance:**
- [ ] `index.html` loads without console errors
- [ ] Loading indicator shows "Building index... N/1000" during insertion
- [ ] Index build time for 1000 vectors at 384D < 3 seconds in Chrome
- [ ] Query dropdown has 10 options
- [ ] Search returns 10 results
- [ ] Entity badges visible with correct colors
- [ ] Boost toggle changes result ranking
- [ ] Latency displayed in UI (< 100ms target)
- [ ] Footer shows "100% in-browser. Zero API calls."
- [ ] Vanilla HTML + JS only (no React/Vue/framework)
- [ ] File is self-contained (no external CDN dependencies except WASM pkg)

---

### W48.4c: Smoke Test Demo in Chrome (1h) — WASM_SPECIALIST

**Dependency:** W48.4b complete

**Test Procedure:**
1. Start server: `npx serve demo/entity-rag/`
2. Open Chrome, navigate to demo URL
3. Wait for "Ready" status (data loaded, index built). Verify build time < 3 seconds.
4. Select each of the 10 sample queries, click Search
5. Verify results appear with entity badges
6. Toggle boost ON -> OFF -> ON, verify ranking changes
7. Check latency is < 100ms for each search
8. Open DevTools Console: verify 0 errors
9. Check Network tab: verify no external API calls
10. Record WASM bundle size from Network tab

**Acceptance:**
- [ ] All 10 queries return results
- [ ] Boost toggle visibly changes ranking for at least 5/10 queries
- [ ] Search latency < 100ms for all queries
- [ ] Index build time < 3 seconds in Chrome
- [ ] Loading indicator visible during index build
- [ ] 0 console errors
- [ ] 0 external API calls (all local)
- [ ] WASM bundle < 500KB

---

### W48.4d: Regression (0.5h) — TEST_ENGINEER

**Dependency:** No Rust changes expected today, but verify anyway

**Commands:**
```bash
cargo test --lib
cargo clippy -- -D warnings
cargo check --target wasm32-unknown-unknown
```

**Acceptance:**
- [ ] `cargo test --lib` — 1024+ passed, 0 failed
- [ ] `cargo clippy -- -D warnings` — 0 warnings
- [ ] WASM build succeeds

---

## Day 4 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~7h |
| New files | 1 (demo/entity-rag/index.html) |
| WASM bundle | < 500KB |
| Search latency | < 100ms |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.4a | 0.5h | | |
| W48.4b | 5h | | |
| W48.4c | 1h | | |
| W48.4d | 0.5h | | |
| **Total** | **7h** | | |

---

## Handoff to Day 5

**Codebase state at EOD:**
- Demo loads and works in Chrome
- All 10 sample queries return results with entity badges
- Boost toggle demonstrates ranking difference
- Search < 100ms, bundle < 500KB
- All Rust tests pass

**Day 5 prerequisites satisfied:**
- [ ] Demo works (needed for blog post screenshot/link)
- [ ] MetadataBoost API stable (needed for blog code examples)

**Day 5 focus:** Blog post + README update + end-of-week hostile review

---

**END OF DAY 4 TASKS**
