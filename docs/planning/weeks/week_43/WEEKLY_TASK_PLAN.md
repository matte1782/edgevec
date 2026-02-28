# Week 43: LangChain.js Integration

**Status:** [REVISED]
**Sprint Goal:** Implement `edgevec-langchain` package — full LangChain.js VectorStore adapter
**Dates:** 2026-03-07 to 2026-03-15 (9 days — includes 30% contingency buffer)
**Prerequisite:** W42.1 LangChain Spike [APPROVED — GO]
**Reference:** `docs/research/LANGCHAIN_SPIKE.md`
**Test Framework:** Vitest (fast, ESM-native, TypeScript-first)

**Roadmap Note:** This work is an ADDITION to Phase 10 (v0.10.0), not originally in the approved ROADMAP v6.1 (2026-01-08). The LangChain spike (W42.1, 2026-02-28) produced a GO decision after the roadmap was written. The ROADMAP.md must be updated to add a "Milestone 10.0: LangChain.js Integration (Week 43)" before v0.10.0 ships. This update is tracked as a dependency in the Dependencies section below.

---

## Estimation Notes

**Spike estimate:** 21-33 hours (optimistic-pessimistic range).
**3x rule:** Applied to the spike's optimistic estimate: 21h x 3 = 63h theoretical worst case. The plan schedules ~38h of core task work across Days 1-7 (~5.4h/day), plus ~9.5h overflow on Days 8-9, totaling ~47.5h across 9 days (~5.3h/day). This represents a 1.4x multiplier on the spike pessimistic estimate (33h), well within the 3x ceiling (63h). Days 8-9 provide 17% schedule buffer beyond the core work.
**Contingency:** Days 8-9 are dedicated overflow/polish days. If Days 1-7 complete on schedule, Days 8-9 are used for additional testing and documentation polish.

---

## Critical Path

```
Day 1 (scaffold + metadata) → Day 2 (core class) → Day 3 (full API) → Day 4 (tests)
                                                                              ↓
                                                        Day 5 (integration) → Day 6 (docs)
                                                                              ↓
                                                              Day 7 (hostile review) → Day 8 (fix findings)
                                                                                        ↓
                                                                              Day 9 (final validation)
```

**Zero-float tasks:** Days 1-3 (core implementation) are sequential and on the critical path. If Day 2 slips by 1 day, Days 3-9 all shift by 1 day.
**Hostile review overflow:** Day 8 is dedicated to fixing hostile review findings. If Day 7 review produces REJECT, Day 8 provides full rework capacity. Day 9 handles re-review + final validation.

---

## Day-by-Day Plan

### Day 1 (2026-03-07): Package Scaffolding + Core Types + WASM Init Design

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Create `pkg/langchain/` directory structure | W43.1a | 0.5h | PENDING |
| Create `package.json` with peer deps (`@langchain/core >=0.3.0 <0.5.0`) | W43.1b | 0.5h | PENDING |
| Create `tsconfig.json` (ESM + CJS dual output) + Vitest config | W43.1c | 0.5h | PENDING |
| Design WASM initialization strategy (document in `src/init.ts`) | W43.1d | 1h | PENDING |
| Define `types.ts`: `EdgeVecStoreConfig`, `EdgeVecStoreArgs` interfaces | W43.1e | 1h | PENDING |
| Implement `metadata.ts`: serialize/deserialize helpers + `_serializedKeys` tracking | W43.1f | 2h | PENDING |
| Write unit tests for metadata serialization (nested objects, null, Date, arrays) | W43.1g | 1.5h | PENDING |

**Day 1 Artifacts:**
- `pkg/langchain/package.json`
- `pkg/langchain/tsconfig.json`
- `pkg/langchain/vitest.config.ts`
- `pkg/langchain/src/init.ts` (WASM init strategy)
- `pkg/langchain/src/types.ts`
- `pkg/langchain/src/metadata.ts`
- `pkg/langchain/tests/metadata.test.ts`

**W43.1d — WASM Init Strategy Decisions:**
- Constructor: throws `EdgeVecNotInitializedError` if WASM not loaded
- Factory methods (`fromTexts`, `fromDocuments`): call `await initEdgeVec()` automatically
- `initEdgeVec()`: global singleton, safe to call multiple times (idempotent)
- Explicit `ensureInitialized()` guard at entry of every public method

**Day 1 Acceptance Criteria:**
- [ ] `npm install` in `pkg/langchain/` succeeds
- [ ] Metadata round-trip: `deserialize(serialize(obj))` === `obj` for all LangChain types
- [ ] `_serializedKeys` correctly tracks which keys were JSON-stringified
- [ ] `null` coerced to empty string, round-trips back to `null`
- [ ] Circular object reference throws `MetadataSerializationError` (not silent corruption)
- [ ] WASM init strategy documented in `src/init.ts` with JSDoc

---

### Day 2 (2026-03-08): Core EdgeVecStore Implementation

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Implement `EdgeVecStore` class extending `VectorStore` | W43.2a | 2h | PENDING |
| Implement `addVectors(vectors, documents, options?)` | W43.2b | 1.5h | PENDING |
| Implement `similaritySearchVectorWithScore(query, k, filter?)` | W43.2c | 1.5h | PENDING |
| Implement ID management: string <-> numeric bidirectional map | W43.2d | 1h | PENDING |
| Implement score normalization: distance -> similarity (Spike C2) | W43.2e | 0.5h | PENDING |

**Day 2 Artifacts:**
- `pkg/langchain/src/index.ts` (EdgeVecStore class — core methods)

**Day 2 Acceptance Criteria:**
- [ ] `addVectors` stores vectors with metadata, returns string IDs
- [ ] `similaritySearchVectorWithScore` returns `[Document, score][]` with score in [0, 1]
- [ ] ID map: `stringId -> numericId` and `numericId -> stringId` both work
- [ ] Filter: `'category = "gpu" AND price < 500'` passes to EdgeVec DSL and filters results
- [ ] Filter: `'status IS NULL'` passes correctly
- [ ] Filter: `'count >= 10 OR name != "test"'` passes correctly
- [ ] Score normalization: L2 distance 0.0 maps to similarity 1.0; larger distances map to lower similarity
- [ ] `ensureInitialized()` guard present in `addVectors` and `similaritySearchVectorWithScore`

---

### Day 3 (2026-03-09): Delete, Factory Methods, Save/Load

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Implement `delete({ ids })` with ID map cleanup | W43.3a | 1h | PENDING |
| Implement `fromTexts(texts, metadatas, embeddings, config)` static factory | W43.3b | 1h | PENDING |
| Implement `fromDocuments(docs, embeddings, config)` static factory | W43.3c | 0.5h | PENDING |
| Implement `SaveableVectorStore` interface: map `save(directory)` → `EdgeVecIndex.save(name)` (IndexedDB) | W43.3d | 2h | PENDING |
| Ensure ID mapping persists across save/load cycles (Spike C3) | W43.3e | 1h | PENDING |

**Day 3 Artifacts:**
- `pkg/langchain/src/index.ts` (complete — all methods implemented)

**Day 3 Acceptance Criteria:**
- [ ] `delete` removes vectors AND cleans up ID mapping
- [ ] `fromTexts` creates store, embeds texts, adds vectors in one call
- [ ] `fromDocuments` handles Documents with existing embeddings
- [ ] Save/load roundtrip preserves: vectors, metadata, ID mapping, pageContent
- [ ] `save(directory)` maps directory string to IndexedDB name; `load(directory, embeddings)` reconstructs from IndexedDB. Browser has no filesystem — directory arg is used as IndexedDB key prefix.
- [ ] Factory methods call `await initEdgeVec()` before creating store (Spike Risk #5)
- [ ] Double-init is safe: calling `initEdgeVec()` twice does not error

---

### Day 4 (2026-03-10): Unit Tests + Edge Cases

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Test `addVectors` with 0, 1, 100 documents | W43.4a | 1h | PENDING |
| Test `similaritySearchVectorWithScore` with filters, empty index, k > count | W43.4b | 1.5h | PENDING |
| Test `delete` with valid IDs, invalid IDs, already-deleted IDs | W43.4c | 1h | PENDING |
| Test metadata edge cases: very large pageContent, unicode, empty metadata, circular refs | W43.4d | 1h | PENDING |
| Test ID mapping persistence: save, reload, search, delete | W43.4e | 1h | PENDING |
| Test score normalization: verify similarity in [0, 1] for L2, cosine, dot product | W43.4f | 0.5h | PENDING |
| Test error paths: WASM not initialized, dimension mismatch, invalid filter syntax | W43.4g | 1h | PENDING |

**Day 4 Artifacts:**
- `pkg/langchain/tests/store.test.ts`

**Day 4 Required Test Coverage:**

| Test | Method Under Test | Behavior Verified |
|:-----|:-----------------|:-----------------|
| `test_addVectors_empty` | `addVectors` | 0 docs returns empty array |
| `test_addVectors_single` | `addVectors` | 1 doc stored, ID returned |
| `test_addVectors_batch` | `addVectors` | 100 docs, all IDs unique |
| `test_search_with_filter` | `similaritySearchVectorWithScore` | DSL filter applied |
| `test_search_empty_index` | `similaritySearchVectorWithScore` | Returns `[]` |
| `test_search_k_exceeds_count` | `similaritySearchVectorWithScore` | Returns min(k, count) |
| `test_delete_valid` | `delete` | Vector removed, ID map cleaned |
| `test_delete_invalid_id` | `delete` | No error, no-op |
| `test_metadata_roundtrip` | serialize/deserialize | All types preserved |
| `test_metadata_circular_ref` | serialize | Throws `MetadataSerializationError` |
| `test_id_persistence` | save/load | ID map survives roundtrip |
| `test_score_normalization_l2` | score conversion | L2=0 → sim=1.0 |
| `test_score_normalization_cosine` | score conversion | cos=1.0 → sim=1.0 |
| `test_wasm_not_initialized` | ensureInitialized | Throws `EdgeVecNotInitializedError` |
| `test_dimension_mismatch` | addVectors | Throws on mismatched dims |

**Day 4 Acceptance Criteria:**
- [ ] 15+ unit tests, all passing (see table above)
- [ ] Zero `any` type assertions in tests
- [ ] Error paths: WASM not init → specific error type, dimension mismatch → specific error
- [ ] Edge cases: empty index search returns `[]`, delete non-existent returns gracefully

---

### Day 5 (2026-03-11): Integration Tests + Default Method Verification + Build

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Create integration test with mock embeddings (no external API) | W43.5a | 1.5h | PENDING |
| Test full RAG pipeline: embed -> store -> search -> retrieve | W43.5b | 1h | PENDING |
| Test `addDocuments` default implementation (embeds via this.embeddings) | W43.5c | 0.5h | PENDING |
| Test `maxMarginalRelevanceSearch` default with EdgeVec scores | W43.5d | 0.5h | PENDING |
| Test `asRetriever()` returns functional VectorStoreRetriever | W43.5e | 0.5h | PENDING |
| Verify build: ESM + CJS dual output | W43.5f | 1h | PENDING |
| Verify peer dep: test with `@langchain/core@0.3.x` and `@langchain/core@0.4.x` | W43.5g | 0.5h | PENDING |

**Day 5 Artifacts:**
- `pkg/langchain/tests/integration.test.ts`
- Build output verified (ESM + CJS)

**Day 5 Acceptance Criteria:**
- [ ] Integration tests pass with mock embeddings
- [ ] Full RAG pipeline: text -> embed -> add -> query -> get document back
- [ ] `addDocuments` default: calls embeddings.embedDocuments, then addVectors — works correctly
- [ ] `maxMarginalRelevanceSearch`: returns results without error and result count <= k
- [ ] `asRetriever()`: returned retriever can `.invoke(query)` and get Documents
- [ ] `npm run build` produces both ESM and CJS output
- [ ] TypeScript strict mode: zero errors
- [ ] Bundle size of langchain adapter < 10KB (excluding edgevec WASM)

---

### Day 6 (2026-03-12): Documentation

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Write `pkg/langchain/README.md` with usage examples | W43.6a | 1.5h | PENDING |
| Add quick start: 5-line example showing embed + store + search | W43.6b | 0.5h | PENDING |
| Document filter format with EdgeVec DSL examples | W43.6c | 0.5h | PENDING |
| Document save/load with IndexedDB persistence | W43.6d | 0.5h | PENDING |
| Document WASM initialization requirements | W43.6e | 0.5h | PENDING |
| Run final lint + type check | W43.6f | 0.5h | PENDING |

**Day 6 Artifacts:**
- `pkg/langchain/README.md`

**Day 6 Acceptance Criteria:**
- [ ] README has: installation, quick start, full API reference, filter examples, WASM init guide
- [ ] Quick start example is copy-pasteable and works

**Note on FilterExpression:** The spike recommends supporting both DSL string and `FilterExpression` object (Section 4, line 108). `FilterExpression` IS exported from the WASM API (`pkg/edgevec-wrapper.d.ts` exports it). For W43, we implement DSL string only and **defer `FilterExpression` object support to W44** because: (1) DSL string covers all user-facing use cases, (2) the adapter's `filter` parameter type would need union handling and validation logic for the object form, (3) keeping scope tight for first release reduces risk. The README documents `FilterExpression` support as "Coming in next release."

---

### Day 7 (2026-03-13): Hostile Review

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Run `/review` on full `pkg/langchain/` package | W43.7a | 2h | PENDING |
| Triage findings: classify as critical/major/minor | W43.7b | 0.5h | PENDING |

**Day 7 Acceptance Criteria:**
- [ ] Hostile review completed, findings documented in `docs/reviews/`
- [ ] Findings triaged and prioritized

**Hostile Review Overflow Plan:** If REJECT with critical/major findings → Day 8 is dedicated rework. If APPROVE or APPROVE with minor → proceed to Day 9 final validation.

---

### Day 8 (2026-03-14): Hostile Review Fixes / Overflow Buffer

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Fix all critical hostile reviewer findings | W43.8a | 3h | PENDING |
| Fix all major hostile reviewer findings | W43.8b | 2h | PENDING |
| Resubmit for re-review if needed | W43.8c | 1h | PENDING |

**Day 8 Acceptance Criteria:**
- [ ] All critical findings fixed
- [ ] All major findings fixed
- [ ] Re-review verdict: GO (or original was GO and this day used for polish)

**If no hostile review fixes needed:** Day 8 is used for additional test coverage, documentation polish, or early start on W44 prep.

---

### Day 9 (2026-03-15): Final Validation + Session Close

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Run full EdgeVec regression: `cargo test` + `cargo clippy` | W43.9a | 0.5h | PENDING |
| Run full langchain package tests: `npx vitest run` | W43.9b | 0.5h | PENDING |
| Update main `README.md` with LangChain integration section | W43.9c | 1h | PENDING |
| Update `CHANGELOG.md` with W43 additions | W43.9d | 0.5h | PENDING |
| Update `MEMORY.md` with session progress and lessons learned | W43.9e | 0.5h | PENDING |
| Commit and push all work | W43.9f | 0.5h | PENDING |

**Day 9 Acceptance Criteria:**
- [ ] `cargo test` — all passing (1018+ lib + integration, current baseline)
- [ ] `npx vitest run` in `pkg/langchain/` — all passing (15+ unit + integration)
- [ ] Clippy clean
- [ ] TypeScript strict mode clean
- [ ] All work committed and pushed
- [ ] HOSTILE_REVIEWER verdict: GO

---

## Acceptance Criteria

### W43.1: Package Structure
- [ ] `pkg/langchain/` directory with package.json, tsconfig.json, vitest.config.ts, src/, tests/
- [ ] Peer dep: `@langchain/core >=0.3.0 <0.5.0`
- [ ] Metadata serialization handles all LangChain types (string, number, boolean, array, object, null, Date)
- [ ] WASM init strategy documented and implemented in `src/init.ts`

### W43.2: Core Implementation
- [ ] `EdgeVecStore` extends `VectorStore` from `@langchain/core`
- [ ] `addVectors` — stores vectors + Documents, returns string IDs
- [ ] `similaritySearchVectorWithScore` — returns `[Document, score][]`
- [ ] Score normalized to similarity (higher = better) per Spike C2
- [ ] Filter support via EdgeVec DSL string

### W43.3: Full API
- [ ] `delete({ ids })` with ID map cleanup
- [ ] `fromTexts` and `fromDocuments` static factories
- [ ] Save/Load with ID mapping persistence (Spike C3)
- [ ] Factory methods auto-init WASM (Spike Risk #5)

### W43.4: Quality
- [ ] 15+ unit tests covering all methods, edge cases, and error paths
- [ ] Integration tests: RAG pipeline, `addDocuments`, `maxMarginalRelevanceSearch`, `asRetriever`
- [ ] TypeScript strict mode: zero errors
- [ ] Hostile reviewer approval

### W43.5: Documentation
- [ ] README.md with quick start, API reference, filter examples, WASM init guide
- [ ] Copy-pasteable examples that work
- [ ] FilterExpression deferred to W44 — documented in README

---

## Spike Conditions Mapping

| Condition | From Spike | Task | Verification |
|:----------|:-----------|:-----|:-------------|
| C1 | Separate `edgevec-langchain` package | W43.1b | package.json exists with correct name |
| C2 | Normalize all distance scores to similarity [0,1] (higher = better) | W43.2e | W43.4f tests L2/cosine/dot → [0,1] |
| C3 | ID mapping survives save/load | W43.3e | W43.4e tests save/reload/search/delete |
| C4 | Pin `@langchain/core` peer dep | W43.1b | `>=0.3.0 <0.5.0` in package.json |
| C5 | JSON.stringify unsupported types | W43.1f | W43.1g + W43.4d test round-trips |

**Spike Risk #5 (WASM async init):** Addressed by W43.1d (design) + W43.4g (error path tests). NOT a spike condition — it is a risk mitigation.

---

## Risk Register

| Risk | Prob | Impact | Worst Case | Mitigation |
|:-----|:-----|:-------|:-----------|:-----------|
| `@langchain/core` API breaks between 0.3-0.4 | LOW | MEDIUM | Adapter fails to compile with newer core version | Pin peer dep, test both versions (W43.5g) |
| Metadata serialization loses fidelity | MEDIUM | MEDIUM | Documents retrieved with corrupted metadata, broken RAG pipeline | Extensive round-trip tests (W43.1g, W43.4d), circular ref detection |
| WASM async init breaks factory methods | MEDIUM | MEDIUM | `fromTexts`/`fromDocuments` throw cryptic WASM errors | Dedicated init design (W43.1d), `ensureInitialized` guard, error path tests (W43.4g) |
| pageContent storage bloats metadata | MEDIUM | HIGH | Large documents exceed EdgeVec metadata limits, inserts fail silently | Document size recommendation in README, test with 10KB+ pageContent (W43.4d). **Note:** Spike classifies this as HIGH risk. If EdgeVec has metadata size limits, we must detect and error clearly. |
| Score normalization incorrect for non-cosine metrics | LOW | MEDIUM | Search returns wrong document ordering, subtle RAG quality degradation | Test all three metrics (W43.4f): L2, cosine, dot product |

---

## Deferred Items (Tracked)

| Item | Reason | Target |
|:-----|:-------|:-------|
| `FilterExpression` object support | Requires WASM binding changes; DSL string covers all use cases | W44 |
| npm publish of `edgevec-langchain` | Needs W43 hostile review GO first | W44 |

---

## Dependencies

| This Week | Blocks |
|:----------|:-------|
| W43.1-5 (LangChain implementation) | W44 (npm publish decision) |
| W43.7 (Hostile review) | W44 (release gate) |

| Depends On | From |
|:-----------|:-----|
| W42.1 LangChain Spike [DONE] | Architecture decisions, C1-C5 conditions |
| v0.9.0 WASM bindings [DONE] | Runtime dependency |

| Required Update | When |
|:----------------|:-----|
| Update `docs/planning/ROADMAP.md` to add Milestone 10.0: LangChain.js Integration | Before v0.10.0 release |

---

## Session Log

*(To be filled during execution)*

---

**END OF WEEKLY TASK PLAN**
