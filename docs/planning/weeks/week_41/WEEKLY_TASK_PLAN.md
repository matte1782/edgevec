# EdgeVec Weekly Task Plan -- Week 41: v0.9.0 Release Preparation

**Date Range:** 2026-02-26 to 2026-03-07 (10 working days)
**Author:** PLANNER
**Status:** [REVISED]
**Version Target:** v0.9.0
**Roadmap Reference:** `docs/planning/ROADMAP.md` v6.1, Phase 9 (v0.9.0)

---

## EXECUTIVE SUMMARY

Week 41 is the **release engineering sprint** for v0.9.0. All implementation code is complete (Sparse Vectors, RRF Hybrid Search, FlatIndex, BinaryFlatIndex). This week closes the gap between implemented code and shippable release by addressing TypeScript/WASM bindings, documentation, testing, version engineering, and hostile review.

**Gap Analysis (18 days since Week 40):**

| Area | Current State | Required State | Gap Severity |
|:-----|:-------------|:---------------|:-------------|
| TypeScript types | v0.5.0 wrapper, v0.8.0 wasm-pack | v0.9.0 unified | **HIGH** |
| WASM bindings | Sparse/Hybrid/Flat all in `wasm/mod.rs` | Rebuilt `.d.ts` from `wasm-pack build` | **MEDIUM** |
| README.md | Mentions up to v0.7.0 features | Must cover Sparse, Hybrid, FlatIndex, BinaryFlatIndex | **HIGH** |
| API docs | 0/4 missing docs exist | 4/4 must exist | **HIGH** |
| CHANGELOG.md | Partial (FlatIndex + BinaryFlatIndex only) | Complete v0.9.0 section | **MEDIUM** |
| Examples/demos | 0/3 new demos | 3/3 HTML demos for new features | **LOW** |
| Version numbers | 0.8.0 in Cargo.toml + package.json | 0.9.0 everywhere | **LOW** |
| Hostile review | Not started | Full v0.9.0 approval | **BLOCKING** |

**Critical Path:**
```
Day 0 (Pre-flight) --> Day 1 (WASM) --> Day 2 (API Docs) --> Day 3 (CHANGELOG+README)
                                                                      |
                                                                      v
                                                 Day 4 (Testing) --> Day 5 (Version Bump)
                                                                            |
                                                                            v
                                                                    Day 6 (HOSTILE_REVIEW) --> Day 7 (Release)
```

**Total Estimated Hours:** 52 hours across 10 days (~5.2h/day avg, realistic pace)
**Buffer:** Days 8-10 reserved for Hostile Reviewer rejection fixes (if NO_GO on Day 6)

---

## THIS WEEK'S GOAL

**Ship EdgeVec v0.9.0 to crates.io and npm with complete documentation, updated TypeScript bindings, and HOSTILE_REVIEWER approval.**

---

## DAY 0: Pre-Flight Checks

**Agent:** Human / PROMPT_MAKER
**Theme:** Validate environment before starting release engineering
**Estimated Hours:** 1

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.0.1 | Verify `wasm-pack` is installed and `wasm-pack build --target web` compiles current codebase | 0.25 | Build | `wasm-pack build` exits 0 |
| W41.0.2 | Verify `cargo test --all-features` passes on current HEAD | 0.25 | Tests | Exit code 0, 1019+ tests |
| W41.0.3 | Audit known pre-existing issues: CHANGELOG `[Unreleased]` link points to v0.7.0 (should be v0.8.0), missing `[0.8.0]` comparison link. Document for Day 3 fix. | 0.25 | Notes | Issues documented in this file |
| W41.0.4 | Clean up temp files: remove all `tmpclaude-*` directories from repo root before any `git add` | 0.25 | `ls tmpclaude-*` returns empty | No temp dirs in working tree |

### Known Pre-Existing Issues (for Day 3)

- **CHANGELOG link bug**: `[Unreleased]` link at line 993 points to `v0.7.0...HEAD` — should be `v0.8.0...HEAD`
- **Missing `[0.8.0]` link**: No comparison link for v0.8.0 at bottom of CHANGELOG
- **Version comparison table**: Missing v0.8.0 row
- **pkg/package.json `files` array**: Only 5 entries, should be 10+ (pre-existing from v0.5.3)

### Exit Criteria (Day 0)

- [ ] `wasm-pack build --target web` succeeds
- [ ] `cargo test --all-features` passes
- [ ] Pre-existing issues documented
- [ ] Temp files cleaned from repo root

---

## DAY 1: WASM Bindings and TypeScript Types

**Agent:** WASM_SPECIALIST
**Theme:** Ensure all v0.9.0 features are accessible from JavaScript/TypeScript
**Estimated Hours:** 10

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.1.1 | Rebuild WASM package via `wasm-pack build --target web` and verify `.d.ts` output includes `insertSparse`, `searchSparse`, `hybridSearch`, `insertBatchFlat`, `searchBinary`, `searchBinaryWithEf`, `searchBinaryFiltered` | 2 | Build | `wasm-pack build` succeeds; `grep -c 'insertSparse\|searchSparse\|hybridSearch' pkg/edgevec.d.ts` returns 3+ |
| W41.1.2 | Update `pkg/edgevec-wrapper.d.ts`: add `FlatIndex` creation path, `searchBinary()` method, sparse/hybrid methods to `EdgeVecIndex` class. Update `@version` header from `0.5.0` to `0.9.0` (line 7). | 3 | Manual review | `EdgeVecIndex` class has methods: `insertSparse()`, `searchSparse()`, `hybridSearch()`, `searchBinary()`; `@version` reads `0.9.0` |
| W41.1.3 | Update `pkg/edgevec-types.d.ts` version header from v0.5.0 to v0.9.0; verify `SparseVector`, `HybridSearchOptions`, `FusionMethod`, `HybridSearchResult` types already present (they are from Week 39). Both `.d.ts` files have `@version 0.5.0` headers that must be updated. | 1 | Diff | `edgevec-types.d.ts` version string reads `@version 0.9.0`; all sparse/hybrid types confirmed present |
| W41.1.4 | Update `pkg/package.json`: add `edgevec-types.d.ts`, `edgevec-wrapper.d.ts`, `edgevec-wrapper.js`, `filter.js`, `filter.d.ts`, `filter-builder.js`, `filter-builder.d.ts` to `files` array; add keywords `sparse`, `hybrid-search`, `flat-index`, `binary-quantization` | 1 | JSON parse | `node -e "JSON.parse(require('fs').readFileSync('pkg/package.json'))"` succeeds; `files` array has 10+ entries |
| W41.1.5 | Create `wasm/__tests__/sparse_hybrid.test.js` -- minimal WASM integration test: init EdgeVec, insertSparse, searchSparse, hybridSearch round-trip | 3 | Test | `wasm-pack test --headless --chrome` passes (or node-based test passes) |

### Exit Criteria (Day 1)

- [ ] `wasm-pack build --target web` succeeds with zero errors
- [ ] `pkg/edgevec.d.ts` contains type signatures for all v0.9.0 WASM exports
- [ ] `pkg/edgevec-wrapper.d.ts` updated with sparse/hybrid/binary methods; `@version` header reads v0.9.0
- [ ] `pkg/edgevec-types.d.ts` `@version` header reads v0.9.0
- [ ] `pkg/package.json` `files` array includes all distribution files
- [ ] At least one integration test exercises sparse insert + sparse search + hybrid search via WASM

### Agent Prompt (Day 1 -- WASM_SPECIALIST)

```
You are the WASM_SPECIALIST for EdgeVec. Execute Week 41 Day 1 tasks.

CONTEXT:
- v0.9.0 code is COMPLETE in Rust (sparse vectors, hybrid search, FlatIndex, BinaryFlatIndex)
- WASM bindings exist in src/wasm/mod.rs with #[wasm_bindgen] exports for:
  - insertSparse (line 3542, #[cfg(feature = "sparse")])
  - searchSparse (line 3610, #[cfg(feature = "sparse")])
  - hybridSearch (line 3723, #[cfg(feature = "sparse")])
  - searchBinary (line 1054)
  - searchBinaryWithEf (line 1165)
  - searchBinaryFiltered (line 1260)
  - insertBatchFlat (line 1410)
  - searchHybrid (line 2829) -- NOTE: this is the FILTER+BQ hybrid, NOT sparse hybrid
- "sparse" feature is DEFAULT in Cargo.toml: default = ["sparse"]
- pkg/edgevec-wrapper.d.ts has @version 0.5.0 header (line 7) -- needs update to 0.9.0 + new methods added
- pkg/edgevec-types.d.ts has @version 0.5.0 header but ALREADY contains sparse/hybrid types (added Week 39) -- update header to 0.9.0
- pkg/package.json files array only has: edgevec_bg.wasm, edgevec.js, edgevec.d.ts, LICENSE-*

TASKS:
1. Run `wasm-pack build --target web` and verify the generated pkg/edgevec.d.ts
2. Update pkg/edgevec-wrapper.d.ts: add sparse/hybrid/binary methods + update @version header from 0.5.0 to 0.9.0 (line 7)
3. Update pkg/edgevec-types.d.ts @version header from 0.5.0 to 0.9.0
4. Update pkg/package.json files array and keywords
5. Create a WASM integration test for sparse/hybrid round-trip

FILES TO READ FIRST:
- src/wasm/mod.rs (WASM bindings source)
- pkg/edgevec.d.ts (auto-generated types)
- pkg/edgevec-wrapper.d.ts (handwritten wrapper types)
- pkg/edgevec-types.d.ts (handwritten supplementary types)
- pkg/package.json (npm config)
- Cargo.toml (features config)

CONSTRAINTS:
- Do NOT modify src/*.rs files (code is frozen for release)
- Do NOT change any Rust behavior
- All TypeScript types must match actual WASM exports exactly
- Test must be runnable via wasm-pack test or node
```

---

## DAY 2: API Documentation

**Agent:** DOCWRITER
**Theme:** Create the 4 missing API reference documents
**Estimated Hours:** 8

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.2.1 | Create `docs/api/FLAT_INDEX.md` -- API reference for `FlatIndex`: constructor, config, insert, search, delete, compaction, quantization, persistence. Include Rust and JS examples. | 2 | File exists + content review | File exists; has sections: Overview, Config, Insert, Search, Delete, Compaction, BQ, Persistence, Performance; all code examples compile-valid |
| W41.2.2 | Create `docs/api/BINARY_FLAT_INDEX.md` -- API reference for `BinaryFlatIndex`: constructor, insert, search, Hamming/Jaccard metrics, HNSW binary methods, persistence. Include Rust and JS examples. | 2 | File exists + content review | File exists; has sections: Overview, Constructor, Insert, Search, Metrics, HNSW Integration, Persistence, Use Cases |
| W41.2.3 | Create `docs/api/SPARSE_VECTORS.md` -- API reference for sparse vectors (RFC-007): `SparseVector` struct, `SparseStorage`, insert, search, TypeScript API. Include BM25 use case. | 2 | File exists + content review | File exists; has sections: Overview, Data Model, Insert, Search, TypeScript API, BM25 Example, Limitations |
| W41.2.4 | Create `docs/api/HYBRID_SEARCH.md` -- API reference for hybrid search: `HybridSearchEngine`, RRF fusion, linear fusion, options, TypeScript API. Include end-to-end example. | 2 | File exists + content review | File exists; has sections: Overview, Architecture, Fusion Methods, Options, Rust API, TypeScript API, End-to-End Example, Performance |

### Exit Criteria (Day 2)

- [ ] `docs/api/FLAT_INDEX.md` exists with complete API reference
- [ ] `docs/api/BINARY_FLAT_INDEX.md` exists with complete API reference
- [ ] `docs/api/SPARSE_VECTORS.md` exists with complete API reference
- [ ] `docs/api/HYBRID_SEARCH.md` exists with complete API reference
- [ ] All 4 documents use consistent formatting matching existing `docs/api/` files
- [ ] All code examples are valid (Rust compiles, JS is syntactically correct)
- [ ] Cross-references between docs are correct (links work)

### Agent Prompt (Day 2 -- DOCWRITER)

```
You are the DOCWRITER for EdgeVec. Execute Week 41 Day 2 tasks.

CONTEXT:
- v0.9.0 adds 4 major features that lack API documentation:
  1. FlatIndex (brute-force search, Week 40)
  2. BinaryFlatIndex (native binary vectors, PR #7 by @jsonMartin)
  3. Sparse Vectors (RFC-007, Weeks 36-37)
  4. Hybrid Search with RRF Fusion (RFC-007, Weeks 38-39)
- Existing API docs for reference style: docs/api/FILTER_SYNTAX.md, docs/api/DATABASE_OPERATIONS.md
- CHANGELOG.md has feature descriptions you can reference

TASKS:
Create these 4 files, each as a complete API reference:

1. docs/api/FLAT_INDEX.md
   Source: src/flat/mod.rs (ALL FlatIndex code is in this single file — NO separate config.rs/search.rs/persistence.rs)
   Key APIs: FlatIndex::new(), insert(), search(), search_quantized(), to_snapshot(), from_snapshot()
   JS: EdgeVec with indexType="flat" in config

2. docs/api/BINARY_FLAT_INDEX.md
   Source: src/flat/mod.rs (BinaryFlatIndex is ALSO in this file), src/wasm/mod.rs (insertBinary, searchBinary)
   Key APIs: BinaryFlatIndex::new(), insert(), search(), insert_binary() on HNSW
   JS: EdgeVec with vectorType="binary"

3. docs/api/SPARSE_VECTORS.md
   Source: src/sparse/mod.rs, src/sparse/storage.rs, src/sparse/vector.rs, src/sparse/error.rs, src/sparse/metrics.rs, src/sparse/search.rs
   Key APIs: SparseVector::new(), SparseStorage::insert(), SparseStorage::search()
   JS: db.initSparseStorage(), db.insertSparse(), db.searchSparse()

4. docs/api/HYBRID_SEARCH.md
   Source: src/hybrid/mod.rs, src/hybrid/search.rs, src/hybrid/fusion.rs
   Key APIs: HybridSearchEngine::search(), FusionMethod::Rrf, FusionMethod::Linear
   JS: db.hybridSearch(dense, sparseIdx, sparseVal, dim, optionsJson)

FILES TO READ FIRST:
- src/flat/mod.rs (ALL FlatIndex + BinaryFlatIndex code — single file, ~16K lines for FlatIndex, ~495 lines for BinaryFlatIndex)
- src/sparse/vector.rs, src/sparse/storage.rs, src/sparse/search.rs (Sparse vectors)
- src/hybrid/search.rs, src/hybrid/fusion.rs (Hybrid search + RRF)
- src/wasm/mod.rs (WASM bindings for all features)
- docs/api/FILTER_SYNTAX.md (style reference)
- docs/api/DATABASE_OPERATIONS.md (style reference)
- CHANGELOG.md (feature descriptions)

CRITICAL FILE STRUCTURE NOTE:
- src/flat/ contains ONLY mod.rs — there is NO config.rs, search.rs, persistence.rs, or binary.rs
- src/sparse/ contains: mod.rs, storage.rs, vector.rs, error.rs, metrics.rs, search.rs
- src/hybrid/ contains: mod.rs, search.rs, fusion.rs

FORMAT:
Each document must have:
- Title with version badge (v0.9.0)
- Overview paragraph (what, why, when to use)
- Quick Start code example (Rust + JavaScript)
- Complete API reference table (method, params, returns, errors)
- Detailed method documentation with examples
- Performance characteristics section
- Limitations/caveats section
- See Also links to related docs

CONSTRAINTS:
- Match existing docs/api/ style exactly
- All Rust examples must be valid (types, method names match src/)
- All JS examples must match actual WASM binding signatures
- No emojis in documentation
- Version references must say v0.9.0
```

---

## DAY 3: CHANGELOG Consolidation + README Update

**Agent:** DOCWRITER
**Theme:** User-facing documentation completeness
**Estimated Hours:** 6

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.3.1 | Consolidate CHANGELOG.md `[Unreleased]` section: merge existing FlatIndex + BinaryFlatIndex entries with NEW entries for Sparse Vectors (RFC-007 Phase 1+2) and RRF Hybrid Search. Change header from `[Unreleased]` to `[0.9.0] - 2026-03-07`. **FIX PRE-EXISTING BUGS:** (1) Line 993: change `[Unreleased]` link from `v0.7.0...HEAD` to `v0.9.0...HEAD` (new unreleased section), (2) Add missing `[0.8.0]` comparison link: `v0.7.0...v0.8.0`, (3) Add `[0.9.0]` link: `v0.8.0...v0.9.0`, (4) Add v0.8.0 row to Version Comparison table. | 2.5 | Diff review | `[0.9.0]` section exists; all 4 link bugs fixed; Version Comparison table has v0.8.0 AND v0.9.0 rows. |
| W41.3.2 | Update README.md: add FlatIndex section, BinaryFlatIndex section, Sparse Vectors section, Hybrid Search section. Update feature comparison table. Update "Why EdgeVec?" table to include Sparse/Hybrid. Update Version History. | 3 | Diff review | README mentions all 4 new features; feature table has Sparse and Hybrid rows; Version History includes v0.8.0 and v0.9.0 entries |
| W41.3.3 | Update `docs/api/README.md` index: add links to the 4 new API docs created on Day 2. Update version references. | 0.5 | Link check | All 4 new doc links present and paths correct |
| W41.3.4 | Update `pkg/README.md` for npm display: sync with main README's Quick Start but npm-focused. Add sparse/hybrid examples. | 0.5 | File review | pkg/README.md mentions v0.9.0 features |

### Exit Criteria (Day 3)

- [ ] CHANGELOG.md has complete `[0.9.0]` section with all features
- [ ] CHANGELOG.md `[0.9.0]` link: `compare/v0.8.0...v0.9.0`
- [ ] CHANGELOG.md `[0.8.0]` link: `compare/v0.7.0...v0.8.0` (was missing)
- [ ] CHANGELOG.md `[Unreleased]` link: `compare/v0.9.0...HEAD` (was pointing to v0.7.0)
- [ ] Version Comparison table has v0.8.0 AND v0.9.0 rows
- [ ] README.md mentions FlatIndex, BinaryFlatIndex, Sparse Vectors, Hybrid Search
- [ ] README.md feature comparison table updated
- [ ] README.md Version History updated through v0.9.0
- [ ] docs/api/README.md links to all 4 new API docs
- [ ] pkg/README.md updated for npm v0.9.0

### Agent Prompt (Day 3 -- DOCWRITER)

```
You are the DOCWRITER for EdgeVec. Execute Week 41 Day 3 tasks.

CONTEXT:
- CHANGELOG.md currently has [Unreleased] with FlatIndex + BinaryFlatIndex entries
- Missing from CHANGELOG: Sparse Vectors (RFC-007 Phase 1+2, Weeks 36-37) and
  RRF Hybrid Search (RFC-007, Weeks 38-39)
- README.md last updated for v0.7.0 features -- does NOT mention:
  - v0.8.0 (Vue composables, filter functions, SIMD Euclidean, tech debt)
  - v0.9.0 (FlatIndex, BinaryFlatIndex, Sparse Vectors, Hybrid Search)
- Release date target: 2026-03-07

TASKS:

1. CHANGELOG.md consolidation:
   - Read current CHANGELOG.md
   - Read src/sparse/ for feature details
   - Read src/hybrid/ for feature details
   - Add "Sparse Vectors (RFC-007)" subsection under [0.9.0]
   - Add "RRF Hybrid Search (RFC-007)" subsection under [0.9.0]
   - Change [Unreleased] header to [0.9.0] - 2026-03-07
   - FIX PRE-EXISTING LINK BUGS (CRITICAL):
     * Line 993: `[Unreleased]: .../compare/v0.7.0...HEAD` is WRONG — v0.7.0 should be v0.8.0
     * There is NO `[0.8.0]` comparison link — add: `[0.8.0]: .../compare/v0.7.0...v0.8.0`
   - Add NEW links:
     * `[0.9.0]: https://github.com/matte1782/edgevec/compare/v0.8.0...v0.9.0`
     * Update `[Unreleased]` to point to `v0.9.0...HEAD`
   - Update Version Comparison table: add v0.8.0 AND v0.9.0 rows

2. README.md v0.9.0 update:
   - Read current README.md
   - Add "FlatIndex" section under Database Features (after Persistence)
   - Add "BinaryFlatIndex" section
   - Add "Sparse Vectors" section with BM25 example
   - Add "Hybrid Search" section with RRF example
   - Update "Why EdgeVec?" comparison table (add Sparse Search, Hybrid Search rows)
   - Update "Interactive Demos" table (placeholder for new demos)
   - Update Version History section
   - Update contributor table (@jsonMartin now has PR #4 AND PR #7)

3. docs/api/README.md -- add 4 new links
4. pkg/README.md -- npm-focused v0.9.0 update

FILES TO READ FIRST:
- CHANGELOG.md (current state)
- README.md (current state)
- src/sparse/vector.rs, src/sparse/storage.rs (sparse feature details)
- src/hybrid/search.rs, src/hybrid/fusion.rs (hybrid feature details)
- docs/api/README.md (API docs index)
- pkg/README.md (npm README)

CONSTRAINTS:
- Preserve existing CHANGELOG entries exactly (only ADD, do not modify old versions)
- README style must match existing sections (same heading levels, table formats)
- All code examples must be syntactically valid
- No emojis
- Version must be 0.9.0, date 2026-03-07
```

---

## DAY 4: Integration Testing Sprint

**Agent:** TEST_ENGINEER
**Theme:** Full test suite validation, WASM build verification, cross-feature testing
**Estimated Hours:** 10

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.4.1 | Run full native test suite: `cargo test --all-features` and document results. Fix any failures. | 2 | All tests pass | `cargo test --all-features` exits 0; test count documented (expected: 1019+) |
| W41.4.2 | Run clippy: `cargo clippy --all-features --all-targets -- -D warnings`. Fix any new warnings. | 1 | Zero warnings | `cargo clippy` exits 0 with zero warnings |
| W41.4.3 | Run WASM build: `wasm-pack build --target web`. Verify bundle size < 500KB gzipped. | 1 | Build succeeds | `wasm-pack build` exits 0; gzipped size < 500KB |
| W41.4.4 | Run WASM test suite: `wasm-pack test --headless --chrome` (or `--node`). Document results. | 2 | All WASM tests pass | `wasm-pack test` exits 0 |
| W41.4.5 | Cross-feature integration test: write `tests/v090_integration.rs` that exercises FlatIndex + BinaryFlatIndex + Sparse + Hybrid in a single test scenario. Must cover: create FlatIndex, insert vectors, search, create HNSW with sparse storage, insert sparse, hybrid search. | 3 | Test passes | `cargo test v090_integration` passes; covers all 4 new features |
| W41.4.6 | Verify benchmarks compile: `cargo bench --no-run --all-features`. Ensure no build errors in any bench target. | 1 | Build succeeds | `cargo bench --no-run` exits 0 for all 22 bench targets |

### Exit Criteria (Day 4)

- [ ] `cargo test --all-features` passes with 1019+ tests
- [ ] `cargo clippy --all-features --all-targets -- -D warnings` exits 0
- [ ] `wasm-pack build --target web` succeeds, bundle < 500KB gzipped
- [ ] `wasm-pack test` passes (headless or node)
- [ ] `tests/v090_integration.rs` exists and passes
- [ ] `cargo bench --no-run --all-features` succeeds for all bench targets
- [ ] Test results documented in `docs/planning/weeks/week_41/DAY_4_RESULTS.md`

### Agent Prompt (Day 4 -- TEST_ENGINEER)

```
You are the TEST_ENGINEER for EdgeVec. Execute Week 41 Day 4 tasks.

CONTEXT:
- v0.9.0 has 4 new features: FlatIndex, BinaryFlatIndex, Sparse Vectors, Hybrid Search
- Current test count: ~1019 tests (from BinaryFlatIndex PR #7)
- All features implemented and individually tested in their respective weeks
- This is a VALIDATION sprint, not a development sprint
- Goal: confirm everything still works together before release

TASKS:

1. Run `cargo test --all-features` -- document pass count, any failures
2. Run `cargo clippy --all-features --all-targets -- -D warnings` -- fix any warnings
3. Run `wasm-pack build --target web` -- verify build, measure bundle size
4. Run `wasm-pack test --headless --chrome` (or --node if headless unavailable)
5. Create tests/v090_integration.rs:
   ```rust
   // Integration test covering all v0.9.0 features in one scenario
   // - FlatIndex: create, insert 100 vectors, search, verify recall
   // - BinaryFlatIndex: create, insert binary vectors, search
   // - Sparse: create SparseStorage, insert sparse vectors, search
   // - Hybrid: create HybridSearchEngine, run hybrid search with RRF
   // Each section must have at least 2 assertions
   ```
6. Run `cargo bench --no-run --all-features` -- verify all benchmarks compile

FILES TO READ FIRST:
- Cargo.toml (features, bench targets)
- src/flat/mod.rs (ALL FlatIndex + BinaryFlatIndex API — single file)
- src/sparse/storage.rs (SparseStorage API)
- src/sparse/search.rs (SparseSearch API)
- src/hybrid/search.rs (HybridSearchEngine API)
- src/hybrid/fusion.rs (RRF/Linear fusion)
- tests/ (existing test files for reference patterns)

NOTE: src/flat/ contains ONLY mod.rs. There is NO binary.rs, config.rs, or search.rs.

CONSTRAINTS:
- Do NOT modify src/*.rs unless fixing a genuine bug (document any fix)
- Integration test must be self-contained (no external data files)
- All assertions must be meaningful (not just "assert!(true)")
- Document all results in docs/planning/weeks/week_41/DAY_4_RESULTS.md
```

---

## DAY 5: Version Bump + Release Engineering

**Agent:** RUST_ENGINEER
**Theme:** Version numbers, git tag preparation, package metadata
**Estimated Hours:** 4

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.5.1 | Bump `Cargo.toml` version from `0.8.0` to `0.9.0` | 0.25 | `grep 'version = "0.9.0"' Cargo.toml` | Version reads `0.9.0` |
| W41.5.2 | Bump `pkg/package.json` version from `0.8.0` to `0.9.0` | 0.25 | `grep '"version": "0.9.0"' pkg/package.json` | Version reads `0.9.0` |
| W41.5.3 | Run `cargo test --all-features` after version bump to confirm nothing broke | 1 | All tests pass | Same test count as Day 4 |
| W41.5.4 | Run `cargo package --allow-dirty` to verify crate packaging. Check package size < 10MB. | 1 | Package succeeds | `cargo package` exits 0; `.crate` size < 2MB |
| W41.5.5 | Verify `wasm-pack build --target web` still works after version bump | 0.5 | Build succeeds | `wasm-pack build` exits 0 |
| W41.5.6 | Create git tag preparation script or document exact commands: `git tag -a v0.9.0 -m "..."` (do NOT execute yet -- Day 7) | 1 | Script/doc exists | `docs/planning/weeks/week_41/RELEASE_COMMANDS.md` exists with exact commands |

### Exit Criteria (Day 5)

- [ ] Cargo.toml version = "0.9.0"
- [ ] pkg/package.json version = "0.9.0"
- [ ] `cargo test --all-features` passes at v0.9.0
- [ ] `cargo package --allow-dirty` succeeds, size < 2MB
- [ ] `wasm-pack build --target web` succeeds at v0.9.0
- [ ] Release command document exists with exact publish sequence

### Agent Prompt (Day 5 -- RUST_ENGINEER)

```
You are the RUST_ENGINEER for EdgeVec. Execute Week 41 Day 5 tasks.

CONTEXT:
- All code is complete and tested (Day 4 validation passed)
- Current versions: Cargo.toml 0.8.0, pkg/package.json 0.8.0
- Target version: 0.9.0
- Previous release: v0.8.0 on 2026-01-08

TASKS:
1. Edit Cargo.toml: change version = "0.8.0" to version = "0.9.0"
2. Edit pkg/package.json: change "version": "0.8.0" to "version": "0.9.0"
3. Run cargo test --all-features (verify nothing broke)
4. Run cargo package --allow-dirty (verify crate packages correctly)
5. Run wasm-pack build --target web (verify WASM builds at new version)
6. Create docs/planning/weeks/week_41/RELEASE_COMMANDS.md with exact commands:
   - git add, commit, tag sequence
   - cargo publish command
   - wasm-pack publish command
   - npm publish command
   - GitHub release creation (gh release create)

FILES TO READ FIRST:
- Cargo.toml
- pkg/package.json

CONSTRAINTS:
- ONLY change version numbers, nothing else
- Do NOT actually publish or create tags (that is Day 7)
- Do NOT modify any Rust source code
- Verify tests pass AFTER version bump before proceeding
```

---

## DAY 6: HOSTILE_REVIEWER Final Gate

**Agent:** HOSTILE_REVIEWER (MUST be a fresh agent session — NOT the same session that created Days 1-5 artifacts)
**Theme:** Comprehensive v0.9.0 release audit
**Estimated Hours:** 10
**Independence Note:** To avoid self-review bias, Day 6 MUST be executed in a new Claude Code session that has NOT produced any of the artifacts being reviewed.

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.6.1 | Audit TypeScript types: verify every `#[wasm_bindgen]` export in `src/wasm/mod.rs` has a corresponding type in `.d.ts` files | 2 | Checklist | Every exported function has TypeScript type; no phantom types |
| W41.6.2 | Audit API documentation: verify all 4 new docs are accurate against source code. Check: method signatures match, return types match, error conditions documented | 2 | Checklist | Zero discrepancies between docs and source |
| W41.6.3 | Audit CHANGELOG: verify every entry in v0.9.0 section corresponds to actual code changes. Check commit history `v0.8.0..HEAD`. | 1 | Checklist | No phantom features, no missing features |
| W41.6.4 | Audit README: verify all claims are accurate (performance numbers, feature matrix, API examples). Run any code examples that can be verified. | 2 | Checklist | All README claims verifiable |
| W41.6.5 | Security audit: scan for `unsafe` blocks added since v0.8.0, check for `unwrap()` in library code, verify no debug prints left in production code | 1 | Checklist | No new unreviewed `unsafe`; no `unwrap()` in lib; no `println!` or `dbg!` in lib |
| W41.6.6 | Write hostile review verdict: `docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md` with GO or NO_GO decision. If GO, create `.claude/GATE_3_COMPLETE.md` for v0.9.0. | 2 | File exists | Review document exists with clear verdict and justification |

### Exit Criteria (Day 6)

- [ ] TypeScript type audit complete -- all exports covered
- [ ] API documentation audit complete -- zero discrepancies
- [ ] CHANGELOG audit complete -- all entries verified
- [ ] README audit complete -- all claims verified
- [ ] Security scan complete -- no violations
- [ ] `docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md` exists with verdict
- [ ] If GO: `.claude/GATE_3_COMPLETE.md` created (or updated for v0.9.0)

### Agent Prompt (Day 6 -- HOSTILE_REVIEWER)

```
You are the HOSTILE_REVIEWER for EdgeVec. Execute Week 41 Day 6 tasks.

YOUR MANDATE: You have KILL AUTHORITY. If this release is not ready, you REJECT it.
Do not be lenient. Do not overlook issues because "it's close enough."
Every claim in documentation MUST be backed by code. Every type MUST match reality.

CONTEXT:
- v0.9.0 release candidate. All code complete, docs written, tests passing.
- This is the FINAL gate before public release to crates.io and npm.
- Previous releases (v0.5.3) had packaging bugs. v0.4.1 had missing snippets dir.
  We CANNOT afford another broken release.

AUDIT CHECKLIST:

1. WASM TYPE AUDIT:
   For EACH function with #[wasm_bindgen] in src/wasm/mod.rs:
   - Does pkg/edgevec.d.ts have a matching export?
   - Do the parameter types match? (Float32Array, Uint8Array, JsValue, etc.)
   - Do the return types match? (number, string, any, void, etc.)
   - Are #[cfg(feature = "sparse")] functions documented as requiring the feature?

2. API DOC AUDIT:
   For EACH method documented in docs/api/FLAT_INDEX.md:
   - Does the method exist in src/flat/mod.rs?
   - Does the signature match exactly?
   - Are error conditions documented?
   For BINARY_FLAT_INDEX.md, SPARSE_VECTORS.md, HYBRID_SEARCH.md: same process.

3. CHANGELOG AUDIT:
   - Run: git log v0.8.0..HEAD --oneline
   - Every commit should map to a CHANGELOG entry
   - Every CHANGELOG entry should map to a commit
   - Performance claims must reference a benchmark

4. README AUDIT:
   - Feature comparison table: every "Yes" must be backed by code
   - Performance numbers: must match latest benchmarks
   - Code examples: must be syntactically valid and use correct API

5. SECURITY AUDIT:
   - grep -rn 'unsafe' src/ -- review each block
   - grep -rn 'unwrap()' src/lib.rs src/flat/ src/sparse/ src/hybrid/ src/wasm/
   - grep -rn 'println!' src/ (should be zero in lib code)
   - grep -rn 'dbg!' src/ (should be zero)
   - grep -rn 'todo!' src/ (should be zero in release code)
   - grep -rn 'unimplemented!' src/ (should be zero)

6. VERDICT:
   Write docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md with:
   - Summary of findings
   - CRITICAL issues (blocks release)
   - MAJOR issues (should fix before release)
   - MINOR issues (fix in next release)
   - VERDICT: GO or NO_GO
   - If NO_GO: exact list of what must be fixed

FILES TO READ:
- src/wasm/mod.rs (ALL of it)
- pkg/edgevec.d.ts
- pkg/edgevec-types.d.ts
- pkg/edgevec-wrapper.d.ts
- docs/api/FLAT_INDEX.md
- docs/api/BINARY_FLAT_INDEX.md
- docs/api/SPARSE_VECTORS.md
- docs/api/HYBRID_SEARCH.md
- CHANGELOG.md
- README.md
- Cargo.toml
- pkg/package.json

CONSTRAINTS:
- You MUST find at least 3 issues (there are always issues)
- You MUST categorize each as CRITICAL/MAJOR/MINOR
- You MUST provide specific file:line references for each issue
- Do NOT approve a release with any CRITICAL issues
- Do NOT be swayed by "we need to ship" pressure
```

---

## DAY 7: Release Day

**Agent:** PROMPT_MAKER (Coordinator)
**Theme:** Execute release sequence, announcements
**Estimated Hours:** 4

### Tasks

| ID | Task | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:-----------|:-------------|:--------------------|
| W41.7.1 | Verify HOSTILE_REVIEWER verdict is GO. If NO_GO, STOP and address issues. | 0.25 | File check | `docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md` contains "GO" verdict |
| W41.7.2 | Execute release commands from `RELEASE_COMMANDS.md`: git commit, git tag v0.9.0 | 0.5 | Git log | `git tag` shows `v0.9.0`; `git log --oneline -1` shows release commit |
| W41.7.3 | Publish to crates.io: `cargo publish` | 0.5 | crates.io | `https://crates.io/crates/edgevec` shows version 0.9.0 |
| W41.7.4 | Publish to npm: `wasm-pack build --target web && cd pkg && npm publish` | 0.5 | npmjs.com | `https://www.npmjs.com/package/edgevec` shows version 0.9.0 |
| W41.7.5 | Create GitHub release: `gh release create v0.9.0` with release notes from CHANGELOG | 0.5 | GitHub | `https://github.com/matte1782/edgevec/releases/tag/v0.9.0` exists |
| W41.7.6 | Push tag: `git push origin v0.9.0` | 0.25 | Remote | CI triggers on tag push |
| W41.7.7 | Post-release smoke test: install from npm in fresh project, verify import + basic search works | 1 | Manual test | Fresh `npm install edgevec@0.9.0` + basic search returns results |
| W41.7.8 | Document release completion: update ROADMAP.md v0.9.0 status to RELEASED | 0.5 | File update | ROADMAP shows v0.9.0 as RELEASED with date |

### Exit Criteria (Day 7)

- [ ] HOSTILE_REVIEWER verdict confirmed as GO
- [ ] Git tag v0.9.0 exists
- [ ] crates.io/crates/edgevec shows v0.9.0
- [ ] npmjs.com/package/edgevec shows v0.9.0
- [ ] GitHub release v0.9.0 exists with release notes
- [ ] Smoke test passes on fresh npm install
- [ ] ROADMAP.md updated

### Agent Prompt (Day 7 -- PROMPT_MAKER / Human Coordinator)

```
You are the PROMPT_MAKER coordinating the v0.9.0 release of EdgeVec.

PRE-FLIGHT CHECK:
1. Confirm docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md exists and verdict is GO
2. Confirm Cargo.toml version = "0.9.0"
3. Confirm pkg/package.json version = "0.9.0"
4. Confirm cargo test --all-features passes
5. Confirm wasm-pack build --target web succeeds
6. Confirm CHANGELOG.md has [0.9.0] section with correct date

If ANY pre-flight check fails, STOP and report which check failed.

RELEASE SEQUENCE (execute in order):
1. git add -A -- ':!tmpclaude-*'  # EXCLUDE temp dirs from staging
2. git commit -m "release: v0.9.0 -- Sparse Vectors, Hybrid Search, FlatIndex, BinaryFlatIndex"
3. git tag -a v0.9.0 -m "v0.9.0: Sparse Vectors (RFC-007), RRF Hybrid Search, FlatIndex, BinaryFlatIndex (PR #7)"
4. git push origin main
5. git push origin v0.9.0
6. cargo publish
7. cd pkg && npm publish && cd ..
8. gh release create v0.9.0 --title "v0.9.0 -- Sparse Vectors, Hybrid Search, FlatIndex" --notes-file RELEASE_NOTES.md

POST-RELEASE:
1. Verify crates.io listing
2. Verify npm listing
3. Verify GitHub release page
4. Smoke test: fresh npm install + basic usage
5. Update ROADMAP.md

If any step fails, STOP immediately and document the failure.
Do NOT proceed past a failed step.
```

---

## BLOCKED TASKS

| ID | Task | Blocked By | Unblock Condition |
|:---|:-----|:-----------|:------------------|
| W41.7.* | All Day 7 tasks | W41.6.6 HOSTILE_REVIEWER verdict | Verdict must be GO |
| W41.6.* | All Day 6 tasks | W41.5.* Day 5 completion | Version bumped + tests pass |
| W41.4.5 | Integration test | W41.1.1 WASM build | Need confirmed build before testing |

---

## NOT IN SCOPE THIS WEEK

| Task | Why Deferred |
|:-----|:-------------|
| HTML demos (binary_flat_search.html, sparse_search.html, hybrid_search.html) | LOW priority for release; can ship in v0.9.1 patch |
| WebGPU research (v0.10.0) | Next version scope |
| Product Quantization (v0.10.0) | Next version scope |
| Vue/React wrapper updates for sparse/hybrid | Framework wrappers are supplementary; core TS types sufficient |
| Performance regression benchmarks | Existing benchmarks cover; full regression suite is v0.10.0 |

---

## RISK REGISTER

| ID | Risk | Impact | Likelihood | Mitigation |
|:---|:-----|:-------|:-----------|:-----------|
| R1 | `wasm-pack build` fails due to sparse feature interaction with WASM target | HIGH | LOW | Sparse feature is default and has been building; fallback: conditional compilation |
| R2 | TypeScript type mismatch between wrapper and auto-generated `.d.ts` | MEDIUM | MEDIUM | Day 1 task W41.1.2 specifically addresses this; Day 6 audit catches remainder |
| R3 | HOSTILE_REVIEWER issues NO_GO verdict | HIGH | MEDIUM | Buffer Day 7 for fixes; release date slips to 2026-03-05 max |
| R4 | `cargo publish` fails on package size (>10MB) | HIGH | LOW | v0.5.3 already fixed this; Cargo.toml excludes are in place; Day 5 verifies |
| R5 | npm publish fails on missing files in `files` array | HIGH | MEDIUM | Day 1 task W41.1.4 fixes files array; Day 7 smoke test catches |
| R6 | Cross-feature test reveals interaction bug between FlatIndex and sparse | MEDIUM | LOW | Day 4 integration test designed to catch this; fix would be a Day 4 subtask |

---

## PERFORMANCE TARGETS (v0.9.0)

These targets must be MET or DOCUMENTED as known limitations:

| Metric | Target | Source |
|:-------|:-------|:-------|
| Search latency (100K, 768D, HNSW) | <10ms P99 | Architecture spec |
| FlatIndex search (10K, 768D) | <50ms | Brute-force linear expectation |
| Sparse search (10K sparse vectors) | <5ms | Inverted index O(terms * docs_per_term) |
| Hybrid search (RRF, 10K dense + 10K sparse) | <15ms | Dense + Sparse + fusion overhead |
| WASM bundle size (gzipped) | <500KB | Architecture spec |
| Insert latency (single, HNSW) | <5ms mean | Architecture spec |
| BinaryFlatIndex insert | <0.01ms | O(1) append |
| Memory per vector (768D, F32) | <100 bytes index overhead | Architecture spec |

---

## VALIDATION CRITERIA

This week is COMPLETE when:
- [ ] All tasks in Days 1-7 "APPROVED TASKS" are done
- [ ] All daily exit criteria checklists pass
- [ ] HOSTILE_REVIEWER has issued GO verdict
- [ ] v0.9.0 is published to crates.io AND npm
- [ ] GitHub release v0.9.0 exists
- [ ] Smoke test passes on fresh install
- [ ] ROADMAP.md updated to reflect v0.9.0 RELEASED

---

## HOSTILE REVIEW REQUIRED

**Before coding begins (Day 1):**
- [ ] HOSTILE_REVIEWER has approved this weekly plan

**During execution (Day 6):**
- [ ] HOSTILE_REVIEWER conducts comprehensive release audit

**After release (Day 7):**
- [ ] HOSTILE_REVIEWER validates published artifacts

---

## APPROVALS

| Role | Name | Signature | Date |
|:-----|:-----|:----------|:-----|
| PLANNER | PLANNER Agent | PROPOSED | 2026-02-26 |
| HOSTILE_REVIEWER | HOSTILE_REVIEWER | REJECTED (5/10) | 2026-02-26 |
| PLANNER | PLANNER Agent | REVISED (fixes: C1,C2,C3,M1-M5,m1-m6) | 2026-02-26 |
| HOSTILE_REVIEWER | HOSTILE_REVIEWER | CONDITIONAL GO (8/10) | 2026-02-26 |
| PLANNER | PLANNER Agent | REVISED Rev 1.1 (fix M-NEW-1: wrapper version header) | 2026-02-26 |
| HOSTILE_REVIEWER | | CONDITION MET — APPROVED FOR EXECUTION | |

---

## APPENDIX A: File Inventory (What Exists vs What's Needed)

### Files That EXIST (confirmed by filesystem scan)

| File | Version | Status |
|:-----|:--------|:-------|
| `Cargo.toml` | 0.8.0 | Needs version bump |
| `pkg/package.json` | 0.8.0 | Needs version bump |
| `pkg/edgevec.d.ts` | auto-generated | Needs rebuild (`wasm-pack build`) |
| `pkg/edgevec-wrapper.d.ts` | @version 0.5.0 (line 7) | Needs version bump to 0.9.0 + new methods for sparse/hybrid/binary |
| `pkg/edgevec-types.d.ts` | v0.5.0 header | Has sparse/hybrid types; needs header update |
| `CHANGELOG.md` | Has [Unreleased] | Needs consolidation to [0.9.0] |
| `README.md` | v0.7.0 content | Needs v0.8.0 + v0.9.0 updates |
| `docs/api/README.md` | Exists | Needs 4 new links |
| `src/wasm/mod.rs` | Complete | NO changes needed |
| `src/flat/mod.rs` | Complete | NO changes needed |
| `src/sparse/*.rs` | Complete | NO changes needed |
| `src/hybrid/*.rs` | Complete | NO changes needed |

### Files That MUST BE CREATED

| File | Day | Agent |
|:-----|:----|:------|
| `docs/api/FLAT_INDEX.md` | 2 | DOCWRITER |
| `docs/api/BINARY_FLAT_INDEX.md` | 2 | DOCWRITER |
| `docs/api/SPARSE_VECTORS.md` | 2 | DOCWRITER |
| `docs/api/HYBRID_SEARCH.md` | 2 | DOCWRITER |
| `tests/v090_integration.rs` | 4 | TEST_ENGINEER |
| `docs/planning/weeks/week_41/DAY_4_RESULTS.md` | 4 | TEST_ENGINEER |
| `docs/planning/weeks/week_41/RELEASE_COMMANDS.md` | 5 | RUST_ENGINEER |
| `docs/reviews/2026-03-03_v0.9.0_RELEASE_REVIEW.md` | 6 | HOSTILE_REVIEWER |

---

## APPENDIX B: Dependency Graph

```
W41.1.1 (WASM build)
    |
    +---> W41.1.2 (wrapper .d.ts update)
    |         |
    |         +---> W41.1.5 (WASM integration test)
    |
    +---> W41.1.3 (types .d.ts update)
    |
    +---> W41.1.4 (package.json update)

W41.2.1-2.4 (API docs) --- independent of Day 1, can parallel

W41.3.1-3.4 (CHANGELOG + README) --- depends on Day 2 docs for links

W41.4.1-4.6 (Testing) --- depends on Day 1 WASM build for W41.4.3-4.4
    |
    +---> W41.5.1-5.6 (Version bump)
              |
              +---> W41.6.1-6.6 (Hostile review)
                        |
                        +---> W41.7.1-7.8 (Release)
```

---

## REVISION LOG

### Rev 1 (2026-02-26) — Hostile Reviewer Rejection Fixes

**Score before:** 5/10 | **Verdict:** REJECTED

**Critical fixes applied:**
- **C1 FIXED**: Removed all references to non-existent files (`src/flat/config.rs`, `src/flat/search.rs`, `src/flat/persistence.rs`, `src/flat/binary.rs`). All FlatIndex + BinaryFlatIndex code is in `src/flat/mod.rs` only. Updated Day 2 and Day 4 agent prompts with correct file paths and explicit NOTE about file structure.
- **C2 FIXED**: Day 3 task W41.3.1 now explicitly addresses pre-existing CHANGELOG link bugs: `[Unreleased]` pointing to v0.7.0 (should be v0.8.0), missing `[0.8.0]` comparison link. Added 4 specific link fix instructions to agent prompt.
- **C3 FIXED** (Rev 1.1): Both `edgevec-wrapper.d.ts` AND `edgevec-types.d.ts` have `@version 0.5.0` headers. Updated Day 1 tasks, exit criteria, agent prompt, and File Inventory to correctly require BOTH version headers be updated to 0.9.0. (Rev 1.0 overcorrected by denying the header existed; Rev 1.1 verified line 7 of wrapper file confirms `@version 0.5.0`.)

**Major fixes applied:**
- **M1 FIXED**: Extended timeline from 7 days to 10 days (Days 8-10 as buffer for hostile reviewer rejection fixes). Average drops from 7.4h/day to 5.2h/day.
- **M2 ADDRESSED**: Integration test scope kept at 3h but file references corrected to actual source paths.
- **M3 FIXED**: Added independence note to Day 6 — Hostile Reviewer MUST be a fresh session, NOT the session that produced the artifacts.
- **M4 FIXED**: Added Day 0 pre-flight section documenting all known pre-existing issues (CHANGELOG links, package.json files array, version comparison table).
- **M5 FIXED**: Day 0 task W41.0.1 verifies `wasm-pack build` succeeds before any work begins.

**Minor fixes applied:**
- **m2 FIXED**: Changed `git add -A` in release sequence to exclude `tmpclaude-*` temp dirs.
- **m3 FIXED**: Day 3 task now explicitly requires adding v0.8.0 row to Version Comparison table.
- Release date updated from 2026-03-04 to 2026-03-07 to match realistic 10-day timeline.

---

**END OF WEEK 41 PLAN**

*Generated by: PLANNER Agent v2.0.0*
*Project: EdgeVec*
*Date: 2026-02-26*
