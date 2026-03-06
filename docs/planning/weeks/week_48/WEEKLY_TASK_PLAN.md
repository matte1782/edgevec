# Week 48: Growth Pivot — MetadataBoost, In-Browser Demo, Blog Post

**Status:** [APPROVED]
**Sprint Goal:** Deliver MetadataBoost API for entity-enhanced RAG, an in-browser demo, and a positioning blog post to 10x EdgeVec visibility (83 stars baseline)
**Dates:** 2026-04-14 to 2026-04-19 (5 core days + 1 overflow)
**Prerequisites:** W47 [COMPLETE or IN PROGRESS], WASM build pipeline functional (established v0.7.0), FilteredSearcher + FilterEvaluator + MetadataValue stable (pre-W47 code)
**Milestone:** Growth — v0.10.0 positioning

---

## Strategic Context

W47 resolves PQ validation (WASM exports, real-embedding recall, training optimization). W48 does NOT depend on PQ results. MetadataBoost builds entirely on FilterExpression/FilteredSearcher (pre-W47 code). The only W47 dependency is that WASM build pipeline works, which has been stable since v0.7.0.

EdgeVec has 83 GitHub stars. This week pivots to GROWTH by riding the GraphRAG/entity-enhanced RAG trend (hottest topic in LLM infrastructure 2024-2025). Three deliverables:

1. **MetadataBoost API** — soft metadata boosting (vs hard boolean filtering). Research shows entity-enhanced retrieval improves RAG accuracy from 0.52 to 0.71 (Palmonari, 2025; Xu et al., 2024). Full references:
   - Xu, Z., Cruz, M. J., Guevara, M., Wang, T., Deshpande, M., Wang, X., & Li, Z. (2024). Retrieval-Augmented Generation with Knowledge Graphs for Customer Service Question Answering. arXiv preprint arXiv:2404.17723.
   - Palmonari, M. (2025). Beyond Naive RAG: How Entities and Graphs Enhance Retrieval-Augmented Generation. Invited presentation, UNIMIB Data Semantics course, 2025. Data from Expert.AI/MEI System collaboration on ophthalmic device troubleshooting (Philips/Bosch business cases), presented at European Big Data Value Forum.
2. **In-browser demo** — "wow" moment: 1000 SQuAD paragraphs with entity boost ON/OFF toggle, zero API calls.
3. **Blog post** — position EdgeVec in the entity-enhanced RAG conversation. Hook: "GraphRAG needs GPT-4 + Neo4j. What if retrieval ran in 300KB?"

### Key References (READ BEFORE ANY TASK)

| Document | Path | Purpose |
|:---------|:-----|:--------|
| FilteredSearcher | `src/filter/filtered_search.rs` | Pattern for search_boosted — wrapper over HnswIndex + MetadataStore |
| FilterEvaluator | `src/filter/evaluator.rs` | Metadata evaluation engine (evaluate against HashMap) |
| MetadataValue | `src/metadata/types.rs` | 5 supported types: String, Integer, Float, Boolean, StringArray |
| Fusion patterns | `src/hybrid/fusion.rs` | Score combination: linear_fusion(), rrf_fusion() |
| WASM module | `src/wasm/mod.rs` | Existing WASM export patterns (handle/opaque-pointer) |
| Filter module | `src/filter/mod.rs` | Public re-exports |
| ROADMAP | `docs/planning/ROADMAP.md` | Current v7.2, needs update to v7.3 |

### W47 Items NOT Re-Opened

- PQ core implementation, benchmarks, GO/NO-GO decision
- Training optimization, recall validation
- All items accepted-as-is from GATE_W46_COMPLETE.md

---

## Estimation Notes

**Optimistic total:** 30h
**3x ceiling:** 90h (disaster scenario boundary — not a target)
**Planned:** ~35h across 6 days (~5.9h/day) — 1.17x multiplier on optimistic (D1: 8.5h, D2: 5h, D3: 3.6h, D4: 7h, D5: 6.5h, D6: 4.5h)

**3x Rule Exception (documented):**
1. **MetadataBoost struct (Day 1):** Small, well-defined struct (~200 lines). Follows existing FilteredSearcher patterns exactly. Pattern-matching, not greenfield.
2. **Demo HTML (Day 4):** Vanilla HTML + JS, no framework. Loads pre-computed data. Mechanical.
3. **Blog post (Day 5):** Structure and talking points pre-defined. Writer fills template.

**For uncertain work (WASM export, data prep with Python + spaCy):** 2x multiplier applied.

**Buffer strategy:** Day 6 is overflow. Per-task estimates include ~30% padding.

---

## Critical Path

```
Track A (MetadataBoost — Days 1-2):
  W48.1a (MetadataBoost struct)
  W48.1b (search_boosted method)
  W48.1c (11 unit tests)
  W48.1d (regression)
         |
  W48.2a (WASM boost export)─────────────┐
  W48.2b (test_boost_wasm_smoke)          |
         |                                |
  HOSTILE REVIEW #1 (Mid-Week, Day 2)     |
  [Scope: MetadataBoost API + tests]      |
                                          |
Track B (Demo Data — Day 3):              |
  W48.3a (Python prep script)             |
  W48.3b (data.json generation)           |
  W48.3c (data verification)──────────────┤
                                          |
Track C (Demo + Blog — Days 4-5):         |
  W48.4a (demo HTML)◄────────────────────┘
  W48.4b (demo JS logic)
  W48.4c (demo smoke test in Chrome)
         |
  W48.5a (blog post draft)
  W48.5b (README update)
         |
  HOSTILE REVIEW #2 (End-of-Week, Day 5)
  [Scope: Demo + Blog + README]
         |
  Day 6 (Fix findings + ROADMAP v7.3 + Commit)
```

**Track Independence Note:** Track A (Rust code) and Track B (Python data prep) are independent. Track C (demo) depends on BOTH Track A (WASM boost export) and Track B (data.json).

**DOUBLE HOSTILE REVIEW GATES:**
1. **Mid-week (end of Day 2):** Review MetadataBoost API + tests + WASM export
2. **End-of-week (end of Day 5):** Review demo + blog post + README update

**Key decision points:**
- End of Day 1: Does MetadataBoost pass 11 unit tests? (Feature gate)
- End of Day 2: Does WASM boost export compile? (WASM gate)
- End of Day 3: Does data.json contain 1000 docs with embeddings + metadata? (Data gate)
- End of Day 4: Does demo load and search < 100ms in Chrome? (Demo gate)
- End of Day 5: Is blog post hostile-reviewed and approved? (Content gate)

---

## Day-by-Day Summary

### Day 1 (2026-04-14): MetadataBoost Implementation + Unit Tests

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.1a | Create `src/filter/boost.rs` — MetadataBoost struct and matching logic | RUST_ENGINEER | 3h | Unit | `MetadataBoost { field: String, value: MetadataValue, weight: f32 }` struct with `MetadataBoost::new() -> Result<Self, BoostError>` (rejects NaN/Inf weights per CLAUDE.md Section 3.1 NaN guard). `compute_boost(metadata: &HashMap<String, MetadataValue>) -> f32` method. Returns `weight` if metadata field matches value (including `StringArray.contains(value)` for array fields), else `0.0`. Type mismatch returns `0.0` (no error). `BoostError` enum for construction-time validation. Max 200 lines. `cargo build` succeeds |
| W48.1b | Add `search_boosted()` to `FilteredSearcher` in `src/filter/filtered_search.rs` | RUST_ENGINEER | 3h | Unit | `pub fn search_boosted(&mut self, query: &[f32], k: usize, boosts: &[MetadataBoost], filter: Option<&FilterExpr>, strategy: FilterStrategy) -> Result<FilteredSearchResult, FilteredSearchError>`. Semantics: run `search_filtered()` with `oversample = (k * 3).max(50).min(500)`, then rerank results by MULTIPLICATIVE boosting: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boost_i.compute_boost(meta_i)).clamp(-1.0, 0.99)`. This is scale-independent: a weight of 0.3 with one match reduces distance by 30%, regardless of whether L2 distances are 0.001 or 50000. Returns top-k by final_distance. If boosts is empty, delegates to `search_filtered()`. `cargo build` succeeds |
| W48.1c | Write 11 MetadataBoost unit tests in `src/filter/boost.rs` | TEST_ENGINEER | 2h | Unit | All 11 tests pass with `cargo test test_boost`: `test_boost_single_field_match`, `test_boost_multiple_fields`, `test_boost_no_match_no_effect`, `test_boost_all_match_reranks`, `test_boost_weight_zero_neutral`, `test_boost_negative_weight_penalty`, `test_boost_combined_with_filter`, `test_boost_large_weight_dominates`, `test_boost_type_mismatch_ignored`, `test_boost_string_array_contains`, `test_boost_wasm_smoke` (native-side test verifying boost struct is serde-friendly) |
| W48.1d | Regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | All existing tests pass (1013+ lib + 11 new boost tests), 0 clippy warnings, WASM build succeeds |

**Day 1 exit criterion:** MetadataBoost struct exists. search_boosted() method works. 11 unit tests pass. WASM builds.

### Day 2 (2026-04-15): WASM Boost Export + MID-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.2a | Add `search_with_boost()` WASM export in `src/wasm/mod.rs` | WASM_SPECIALIST | 3h | Compile + Unit | `#[wasm_bindgen] pub fn search_with_boost(handle: &EdgeVecHandle, query: &[f32], k: u32, boosts_json: &str, filter_str: Option<String>) -> Result<JsValue, JsValue>`. Parses boosts from JSON array: `[{"field": "entity_type", "value": "ORG", "weight": 0.3}]`. **IMPORTANT:** `BoostConfig` is a SEPARATE struct with its own serde deserialization using bare JSON values, NOT MetadataValue's adjacently-tagged format (`{"type":"string","value":"ORG"}`). The `"value"` field is a bare JSON value that gets manually converted to MetadataValue internally. Acceptance criterion: BoostConfig JSON format uses bare values, not adjacently-tagged MetadataValue format. Returns search results with boosted distances. No `unwrap()`. `cargo check --target wasm32-unknown-unknown` succeeds |
| W48.2b | Add `src/filter/mod.rs` re-export for boost module | RUST_ENGINEER | 0.25h | Compile | `pub mod boost;` added to `src/filter/mod.rs`. `pub use boost::MetadataBoost;` in re-exports. `cargo build` succeeds |
| W48.2c | Update `src/lib.rs` with MetadataBoost re-export | RUST_ENGINEER | 0.25h | Compile | `MetadataBoost` accessible as `edgevec::filter::MetadataBoost`. `cargo build` succeeds |
| **W48.2d** | **MID-WEEK HOSTILE REVIEW: MetadataBoost API + tests + WASM export** | **HOSTILE_REVIEWER** | **1.5h** | **Review** | **GO verdict on `src/filter/boost.rs` + `search_boosted()` + WASM export. 0 Critical, 0 Major. Attack vectors: (1) boost.rs < 200 lines, (2) no `unwrap()` in library code, (3) all 11 test names match spec, (4) search_boosted uses MULTIPLICATIVE boosting: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)` — scale-independent, (5) type mismatch silently returns 0.0 (no panic), (6) WASM export follows existing handle pattern, (7) negative weights increase distance (penalty), (8) NaN/Inf weights rejected at construction (`MetadataBoost::new()` returns `Result<Self, BoostError>`), (9) StringArray fields match via `contains(value)` not equality** |

**Day 2 exit criterion:** WASM boost export compiles. Hostile review GO on MetadataBoost API.

**HALT CONDITION:** If mid-week hostile review returns NO-GO, Day 3 Track B (data prep) can proceed regardless — it is independent Python code. Day 4 demo work is BLOCKED until MetadataBoost fixes pass Round 2.

### Day 3 (2026-04-16): Demo Data Preparation (Python)

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.3a | Create `demo/entity-rag/prepare_data.py` — data preparation script | BENCHMARK_SCIENTIST | 3h | Script | Python script that: (1) loads 1000 SQuAD v2 paragraphs via HuggingFace `datasets`, (2) embeds with `all-MiniLM-L6-v2` (384D) via `sentence-transformers`, (3) runs spaCy `en_core_web_sm` NER to extract entities per paragraph, (4) rounds all embeddings to 6 decimal places before JSON serialization, (5) outputs `data.json`. Dependencies: `sentence-transformers`, `spacy`, `datasets`. Script is self-contained and reproducible |
| W48.3b | Generate `demo/entity-rag/data.json` | BENCHMARK_SCIENTIST | 1.5h | File | JSON file containing array of 1000 objects: `{ "id": 0, "text": "...", "embedding": [0.1, ...], "metadata": { "entities": ["NASA", ...], "entity_types": ["ORG", ...], "topic": "space" } }`. Embeddings are 384D f32 arrays. File size < 5MB. All embeddings are finite (no NaN/Inf) |
| W48.3c | Verify data quality | BENCHMARK_SCIENTIST | 0.5h | Script | `python -c "import json; d=json.load(open('demo/entity-rag/data.json')); assert len(d)==1000; assert len(d[0]['embedding'])==384; assert all(isinstance(x, float) for x in d[0]['embedding']); print(f'OK: {len(d)} docs, {len(d[0][\"embedding\"])}D')"` passes. At least 80% of docs have at least 1 entity. Entity types include at least 3 distinct types (e.g., ORG, PERSON, GPE) |

**Day 3 exit criterion:** `demo/entity-rag/data.json` exists, verified, < 5MB, 1000 docs with 384D embeddings and entity metadata.

**Decision Tree:**
- If SQuAD v2 unavailable → use first 1000 paragraphs from English Wikipedia
- If spaCy install fails → use simple regex-based entity extraction (capitalized multi-word phrases)
- If `all-MiniLM-L6-v2` download fails → use `all-MiniLM-L12-v2` (same 384D)
- If data.json > 5MB → reduce to 500 docs or compress embeddings to float16 in JSON

### Day 4 (2026-04-17): In-Browser Demo

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.4a | Create `demo/entity-rag/index.html` — demo page | WASM_SPECIALIST | 3h | Browser | Single HTML file with: (1) search input box, (2) results list with entity badges, (3) "Entity Boost ON/OFF" toggle, (4) status bar showing search latency and bundle size, (5) loading indicator: "Building index... N/1000" progress during insertion, (6) footer: "100% in-browser. Zero API calls. Bundle: XXX KB". Vanilla HTML + CSS. NO React/Vue/framework. Loads WASM module + data.json on page load. Index build time for 1000 vectors at 384D < 3 seconds in Chrome |
| W48.4b | Implement JS search logic in `demo/entity-rag/index.html` (inline script) | WASM_SPECIALIST | 3h | Browser | On query submit: (1) encode query text NOT needed — use pre-computed query embeddings for 10 sample queries, user selects from dropdown. (2) Call `search_with_boost()` via WASM with boosts from entity metadata. (3) Display top-10 results with entity badges colored by type (ORG=blue, PERSON=green, GPE=red). (4) Toggle switches between `search_with_boost()` and standard `search()`. (5) Search latency < 100ms for 1000 docs displayed in UI |
| W48.4c | Smoke test demo in Chrome | WASM_SPECIALIST | 1h | Manual | Open `demo/entity-rag/index.html` via `npx serve demo/entity-rag/`. Select sample query. Results appear in < 100ms. Toggle boost ON/OFF shows different ranking. No console errors. WASM bundle < 500KB verified |
| W48.4d | Regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | All existing tests pass, 0 clippy warnings, WASM build succeeds |

**Day 4 exit criterion:** Demo loads in Chrome, search works with boost toggle, latency < 100ms, no console errors.

**Design Note on Query Encoding:** The demo does NOT include a text encoder in WASM (that would require an ML model). Instead, 10 sample queries are pre-embedded in `data.json` alongside the paragraphs. User selects a query from a dropdown. This keeps the demo simple and the bundle small.

### Day 5 (2026-04-18): Blog Post + README Update + END-OF-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.5a | Write `docs/blog/entity-enhanced-rag.md` — blog post | DOCWRITER | 3h | Document | 1500-2000 words. Structure: (1) Hook: "GraphRAG needs GPT-4 + Neo4j. What if retrieval ran in 300KB?", (2) Problem: naive vector search misses entity context, (3) Solution: entity metadata as boost signals with EdgeVec code example, (4) Live demo link + screenshot, (5) Comparison table: EdgeVec vs Qdrant/Weaviate on size/offline/WASM, (6) CTA: star, install, try demo. Tone: conversational, not academic. Show, don't tell |
| W48.5b | Update README.md — add MetadataBoost section + demo link | DOCWRITER | 1h | Document | New "Entity-Enhanced RAG" section in README with: (1) 3-line code example of MetadataBoost, (2) link to live demo, (3) badge or screenshot. README still renders correctly in GitHub markdown |
| W48.5c | Update CHANGELOG.md with W48 work | PLANNER | 0.5h | Document | W48 section: MetadataBoost API, in-browser entity-RAG demo, blog post |
| **W48.5d** | **END-OF-WEEK HOSTILE REVIEW: Demo + Blog + README** | **HOSTILE_REVIEWER** | **2h** | **Review** | **GO verdict on demo + blog + README. 0 Critical, 0 Major. Attack vectors: (1) demo loads without errors in Chrome, (2) blog claims are factual — verify citations: Xu et al. (2024, arXiv:2404.17723) and Palmonari (2025, UNIMIB invited talk), (3) comparison table is fair and verifiable, (4) README code example actually compiles, (5) demo search < 100ms, (6) WASM bundle < 500KB, (7) no broken links, (8) blog is conversational not academic** |

**Day 5 exit criterion:** Blog post hostile-reviewed. README updated. CHANGELOG updated.

### Day 6 (2026-04-19): Overflow / Fix Findings + ROADMAP v7.3 + Commit

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W48.6a | Fix all hostile review findings (both rounds) | RUST_ENGINEER | 2h | Review fixes | All critical + major issues from both hostile reviews resolved |
| W48.6b | Update ROADMAP.md v7.2 to v7.3 — add Growth phase + W48 actuals | PLANNER | 1h | Document | ROADMAP v7.3 reflects: MetadataBoost feature, demo, blog post, growth pivot strategy. Milestone 10.5 or new Growth milestone added |
| W48.6c | Full regression: `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` | TEST_ENGINEER | 0.5h | Full suite | All tests pass (1013+ existing + 11 new boost tests), 0 clippy warnings, WASM build succeeds |
| W48.6d | Commit all W48 work | PLANNER | 0.5h | Git | Conventional commit: `feat(w48): MetadataBoost API + entity-RAG demo + blog post` |
| W48.6e | Create `.claude/GATE_W48_COMPLETE.md` | PLANNER | 0.5h | Gate | Gate file documents: both hostile review verdicts, MetadataBoost test count, demo verification, blog approval, carry-forward items |

**If surplus time available**, prioritize:
1. Publish blog to dev.to
2. npm publish `edgevec-langchain@0.2.0` (if user ready with OTP)
3. Add screenshot to README

---

## Daily Execution Files

Each day has a dedicated execution file following the established project pattern (W40, W45, W47). These are atomic — an engineer can pick up any `DAY_N_TASKS.md` and execute without re-reading this weekly plan.

| File | Content | Created |
|:-----|:--------|:--------|
| `docs/planning/weeks/week_48/DAY_1_TASKS.md` | MetadataBoost struct + search_boosted + 11 tests | Before Day 1 |
| `docs/planning/weeks/week_48/DAY_2_TASKS.md` | WASM boost export + mid-week hostile review | Before Day 2 |
| `docs/planning/weeks/week_48/DAY_3_TASKS.md` | Python data prep (SQuAD + embeddings + NER) | Before Day 3 |
| `docs/planning/weeks/week_48/DAY_4_TASKS.md` | In-browser demo HTML + JS | Before Day 4 |
| `docs/planning/weeks/week_48/DAY_5_TASKS.md` | Blog post + README + end-of-week hostile review | Before Day 5 |
| `docs/planning/weeks/week_48/DAY_6_TASKS.md` | Overflow + fixes + ROADMAP v7.3 + commit | Before Day 6 |

Each daily file contains:
1. **Day objective** and success criteria
2. **Pre-task context loading checklist** (which files to read)
3. **Task sequence** with dependencies clearly marked
4. **Specific command sequences** (cargo, Python, browser commands)
5. **Expected output format** for each task
6. **Decision trees** for conditional paths
7. **Handoff notes** (codebase state at EOD, next day prereqs)
8. **Time tracking template** (estimated vs actual per task)

---

## Deliverables

| Deliverable | Target Day | Owner |
|:------------|:-----------|:------|
| `DAY_1_TASKS.md` through `DAY_6_TASKS.md` — Daily execution files | Pre-sprint | PLANNER |
| `src/filter/boost.rs` — MetadataBoost struct + compute_boost | Day 1 | RUST_ENGINEER |
| `search_boosted()` method in `src/filter/filtered_search.rs` | Day 1 | RUST_ENGINEER |
| 11 unit tests for MetadataBoost | Day 1 | TEST_ENGINEER |
| `search_with_boost()` WASM export in `src/wasm/mod.rs` | Day 2 | WASM_SPECIALIST |
| Mid-week hostile review report | Day 2 | HOSTILE_REVIEWER |
| `demo/entity-rag/prepare_data.py` — Python data prep script | Day 3 | BENCHMARK_SCIENTIST |
| `demo/entity-rag/data.json` — 1000 docs + 384D embeddings + NER metadata | Day 3 | BENCHMARK_SCIENTIST |
| `demo/entity-rag/index.html` — In-browser entity-RAG demo | Day 4 | WASM_SPECIALIST |
| `docs/blog/entity-enhanced-rag.md` — Blog post (1500-2000 words) | Day 5 | DOCWRITER |
| Updated README.md with MetadataBoost section + demo link | Day 5 | DOCWRITER |
| Updated CHANGELOG.md | Day 5 | PLANNER |
| End-of-week hostile review report | Day 5 | HOSTILE_REVIEWER |
| Updated `docs/planning/ROADMAP.md` v7.3 | Day 6 | PLANNER |
| `.claude/GATE_W48_COMPLETE.md` | Day 6 | PLANNER |

---

## Carry-Forward Traceability

W48 introduces new work (growth pivot), not carry-forwards from W47. The following W47 items are explicitly NOT in scope:

| W47 Item | Status | Why Not W48 |
|:---------|:-------|:------------|
| PQ WASM exports | W47 responsibility | Independent track |
| G3 recall validation | W47 responsibility | Independent track |
| G4 training optimization | W47 responsibility | Independent track |
| npm publish edgevec-langchain@0.2.0 | Surplus time only | User handles OTP |

**No items silently dropped.** W48 is a fresh growth sprint.

---

## Double Hostile Review Protocol

### Review #1: Mid-Week MetadataBoost Review (Day 2)

**Scope:** `src/filter/boost.rs` + `search_boosted()` in `filtered_search.rs` + WASM export
**Attack vectors:**
1. **Size guard:** boost.rs < 200 lines (over-engineering check)
2. **Safety:** No `unwrap()` in library code. No panics.
3. **Correctness:** MULTIPLICATIVE boosting: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)` — scale-independent
4. **Edge cases:** weight=0.0 is neutral, negative weights increase distance (penalty), NaN/Inf weights rejected at construction
5. **Type safety:** MetadataValue type mismatch returns 0.0, not error. StringArray fields match via `contains(value)`.
6. **WASM:** Export follows existing handle pattern, no `unwrap()`. BoostConfig uses bare JSON values, NOT MetadataValue's adjacently-tagged format.
7. **Tests:** All 11 specific test names present and pass (including `test_boost_string_array_contains`)
8. **NaN sort defense:** `sort_by` uses `unwrap_or(Ordering::Greater)` to push NaN distances to end, not `Ordering::Equal`
9. **Regression:** Existing 1013+ tests unbroken

**Verdict required before Day 3 demo data prep proceeds with confidence (though data prep can start independently).**

### Review #2: End-of-Week Demo + Blog Review (Day 5)

**Scope:** Demo page + blog post + README update
**Attack vectors:**
1. **Demo functionality:** Loads in Chrome, search < 100ms, boost toggle works
2. **Blog accuracy:** Research citations are real and correctly attributed
3. **Comparison fairness:** EdgeVec vs Qdrant/Weaviate table uses verifiable numbers
4. **README quality:** Code example compiles, links work, renders correctly
5. **No broken promises:** Blog doesn't claim features that don't exist
6. **Bundle size:** WASM < 500KB verified
7. **Data quality:** demo data.json has real embeddings, not random noise
8. **Accessibility:** Demo is usable without instructions

**Verdict required before Day 6 commit.**

---

## Sprint-Level Acceptance Criteria

| Criterion | Pass/Fail |
|:----------|:----------|
| `src/filter/boost.rs` exists with MetadataBoost struct (< 200 lines) | [ ] |
| `MetadataBoost::new()` returns `Result<Self, BoostError>`, rejects NaN/Inf weights | [ ] |
| `search_boosted()` method in FilteredSearcher uses multiplicative boosting (scale-independent) | [ ] |
| 11 named MetadataBoost tests pass (`cargo test test_boost`) | [ ] |
| `search_with_boost()` WASM export compiles for `wasm32-unknown-unknown` | [ ] |
| BoostConfig JSON uses bare values, not adjacently-tagged MetadataValue format | [ ] |
| `demo/entity-rag/data.json` exists (1000 docs, 384D, < 5MB, embeddings rounded to 6 decimals) | [ ] |
| Demo loads in Chrome, search < 100ms, boost toggle works | [ ] |
| Index build time for 1000 vectors at 384D < 3 seconds in Chrome | [ ] |
| Blog post 1500-2000 words, hostile-reviewed and approved | [ ] |
| README updated with MetadataBoost section + demo link | [ ] |
| No existing tests broken (1013+ lib pass) | [ ] |
| Clippy clean (`-D warnings`) | [ ] |
| WASM build: `cargo check --target wasm32-unknown-unknown` succeeds | [ ] |
| WASM bundle < 500KB | [ ] |
| Mid-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| End-of-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| ROADMAP updated to v7.3 | [ ] |
| GATE_W48_COMPLETE.md created | [ ] |

---

## Risk Register

| Risk | Prob | Impact | Mitigation |
|:-----|:-----|:-------|:-----------|
| MetadataBoost over-engineering (> 200 lines) | MEDIUM | MEDIUM | KEEP SIMPLE: one struct, one method, one WASM export. If > 200 lines, STOP and decompose |
| Demo data.json > 5MB | MEDIUM | LOW | Use 384D (not 768D), limit to 1000 docs. If still too large, use float16 in JSON or reduce to 500 docs |
| spaCy install fails on Windows | LOW | MEDIUM | Use WSL2 or conda. Fallback: regex-based entity extraction (capitalized phrases) |
| sentence-transformers model download fails | LOW | MEDIUM | Use `all-MiniLM-L12-v2` as fallback (same 384D). If all fail, use pre-computed embeddings from HuggingFace Hub |
| Blog post too academic | MEDIUM | LOW | Use conversational tone. Show code, not math. Have hostile reviewer specifically check tone |
| WASM bundle > 500KB after boost export | LOW | LOW | MetadataBoost adds ~50 lines of WASM glue. If somehow > 500KB, investigate `wasm-opt -O3` |
| Demo search > 100ms for 1000 docs | LOW | LOW | 1000 docs at 384D is small. If slow, check if data.json parsing is the bottleneck (parse once on load, not per query) |
| Mid-week hostile review returns NO-GO | LOW | HIGH | Fix all findings Day 2 evening, resubmit Day 3 morning. Track B (data prep) can proceed regardless |
| Python deps conflict (spaCy vs sentence-transformers) | LOW | LOW | Use separate virtualenv. `python -m venv demo_env && pip install sentence-transformers spacy datasets` |
| W47 not complete | LOW | LOW | W48 does not depend on W47 PQ work. Only dependency is WASM build pipeline (stable since v0.7.0) |

---

## Dependencies

| This Week Depends On | What |
|:---------------------|:-----|
| FilteredSearcher pattern [STABLE] | `src/filter/filtered_search.rs` — wrapper pattern for search_boosted |
| FilterEvaluator [STABLE] | `src/filter/evaluator.rs` — metadata evaluation engine |
| MetadataValue types [STABLE] | `src/metadata/types.rs` — 5 supported types |
| VectorMetadataStore [STABLE] | `src/filter/filtered_search.rs` — metadata store impl |
| WASM build pipeline [STABLE since v0.7.0] | `wasm-pack build --release` succeeds |
| Python + sentence-transformers + spaCy | For demo data generation (Day 3) |

| This Week Blocks | What |
|:-----------------|:-----|
| Growth campaign (Week 49+) | Blog post + demo are prerequisites |
| v0.10.0 release notes | Needs final feature list (MetadataBoost included or not) |
| dev.to publication | Needs hostile-reviewed blog post |

---

## NOT IN SCOPE THIS WEEK

| Task | Why Deferred |
|:-----|:-------------|
| BM25 engine | Not needed for MetadataBoost — boost is metadata-based, not text-based |
| Graph adjacency / community detection | GraphRAG-level features — we position vs GraphRAG, don't replicate it |
| Changes to PQ code | PQ is W47's domain. W48 is independent |
| New dependencies > 50KB WASM impact | Bundle must stay < 500KB |
| Text embedding in WASM (ML model) | Too large for bundle. Demo uses pre-computed embeddings |
| React/Vue framework for demo | Vanilla HTML + JS only. Framework adds bundle weight |
| npm publish edgevec-langchain@0.2.0 | User handles OTP independently (surplus time only) |
| PQ persistence format | v0.10.0 polish, not growth sprint |

---

## Anti-Error Checklist (Self-Validation)

- [x] No task > 16 hours (largest: W48.1a + W48.1b combined at 6h, but they are separate tasks at 3h each)
- [x] All acceptance criteria are binary pass/fail with specific numbers
- [x] Dependencies reference specific files/functions, not vague descriptions
- [x] MetadataBoost struct definition is explicit: 3 fields (field, value, weight). `new()` returns `Result<Self, BoostError>` rejecting NaN/Inf.
- [x] search_boosted formula is explicit and SCALE-INDEPENDENT: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)`. This is multiplicative boosting — a weight of 0.3 reduces distance by 30% regardless of L2 scale.
- [x] 11 test names are listed explicitly in W48.1c acceptance criteria (including `test_boost_string_array_contains`)
- [x] Demo uses 384D embeddings (all-MiniLM-L6-v2), not 768D — keeps data.json < 5MB
- [x] Demo uses pre-computed query embeddings (dropdown), not real-time text encoding
- [x] Blog word count: 1500-2000 words (neither too short nor too long)
- [x] WASM bundle < 500KB constraint explicitly stated
- [x] boost.rs < 200 lines size guard explicitly stated
- [x] Python data prep is on Day 3 (after MetadataBoost code is reviewed), not Day 1
- [x] Track independence documented (Track A = Rust, Track B = Python, Track C = Demo)
- [x] No PQ code modifications planned
- [x] DAY_1_TASKS.md through DAY_6_TASKS.md listed as deliverables
- [x] Commit convention: `feat(w48):` style specified (W48.6d)
- [x] Two hostile review gates with specific scopes and attack vectors

---

## HOSTILE REVIEW REQUIRED

**Before demo construction begins (end of Day 2):**
- [ ] HOSTILE_REVIEWER has approved MetadataBoost API + tests + WASM export

**Before committing (end of Day 5):**
- [ ] HOSTILE_REVIEWER has approved demo + blog post + README update

---

## APPROVALS

| Role | Name | Signature | Date |
|:-----|:-----|:----------|:-----|
| PLANNER | | [APPROVED] | 2026-03-05 |
| HOSTILE_REVIEWER | | R1: REJECTED (2C, 5M, 7m) — all fixed. R2: CONDITIONAL GO (0C, 0M, 2m advisory) | 2026-03-05 |

---

**END OF WEEKLY TASK PLAN**
