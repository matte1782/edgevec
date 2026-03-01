# Week 43 — Day 4: Unit Tests + Edge Cases

**Date:** 2026-03-10
**Status:** [REVISED] — Hostile review fixes applied (M5, MockEmbeddings fix)
**Focus:** Comprehensive unit test coverage for all `EdgeVecStore` methods, edge cases, and error paths
**Prerequisite:** Day 3 complete (full API implemented)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 7 (Risks)

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Test `addVectors` with 0, 1, 100 documents | W43.4a | 1h | PENDING | Day 3 |
| Test `similaritySearchVectorWithScore` with filters, empty index, k > count | W43.4b | 1.5h | PENDING | Day 3 |
| Test `delete` with valid IDs, invalid IDs, already-deleted IDs | W43.4c | 1h | PENDING | Day 3 |
| Test metadata edge cases: very large pageContent, unicode, empty metadata, circular refs | W43.4d | 1h | PENDING | Day 1 (W43.1f) |
| Test ID mapping persistence: save, reload, search, delete | W43.4e | 1h | PENDING | Day 3 (W43.3e) |
| Test score normalization: verify similarity in [0, 1] for L2, cosine, dot product | W43.4f | 0.5h | PENDING | Day 2 (W43.2e) |
| Test error paths: WASM not initialized, dimension mismatch, invalid filter syntax | W43.4g | 1h | PENDING | Day 2, Day 3 |

**Total Estimated Hours:** 7h

---

## Critical Path

```
All tasks can run in parallel (all depend on Day 3 completion, not each other)
```

Day 4 tasks are independent — they test different methods. A developer can write them in any order.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Unit test suite | `pkg/langchain/tests/store.test.ts` | 15+ unit tests |

---

## Required Test Matrix

| # | Test Name | Method Under Test | Behavior Verified | Priority |
|:--|:----------|:-----------------|:-----------------|:---------|
| 1 | `test_addVectors_empty` | `addVectors` | 0 docs returns empty array | HIGH |
| 2 | `test_addVectors_single` | `addVectors` | 1 doc stored, ID returned | HIGH |
| 3 | `test_addVectors_batch` | `addVectors` | 100 docs, all IDs unique | HIGH |
| 4 | `test_search_basic` | `similaritySearchVectorWithScore` | Returns correct docs | HIGH |
| 5 | `test_search_with_filter` | `similaritySearchVectorWithScore` | DSL filter applied | HIGH |
| 6 | `test_search_empty_index` | `similaritySearchVectorWithScore` | Returns `[]` | MEDIUM |
| 7 | `test_search_k_exceeds_count` | `similaritySearchVectorWithScore` | Returns min(k, count) | MEDIUM |
| 8 | `test_delete_valid` | `delete` | Vector removed, ID map cleaned | HIGH |
| 9 | `test_delete_invalid_id` | `delete` | No error, no-op | MEDIUM |
| 10 | `test_delete_already_deleted` | `delete` | No error, no-op | MEDIUM |
| 11 | `test_metadata_roundtrip` | serialize/deserialize | All types preserved | HIGH |
| 12 | `test_metadata_circular_ref` | serialize | Throws `MetadataSerializationError` | HIGH |
| 13 | `test_metadata_large_pageContent` | addVectors + search | 10KB+ pageContent survives | MEDIUM |
| 14 | `test_metadata_unicode` | addVectors + search | Unicode metadata preserved | MEDIUM |
| 15 | `test_id_persistence` | save/load | ID map survives roundtrip | HIGH |
| 16 | `test_score_normalization_l2` | score conversion | l2_distance=0 → sim=1.0; l2_distance=10 → sim<0.1 | HIGH |
| 17 | `test_score_normalization_cosine` | score conversion | cosine_distance=0 → sim=1.0; cosine_distance=1 → sim=0.0 **[M5 FIX]** | HIGH |
| 18 | `test_wasm_not_initialized` | ensureInitialized | Throws `EdgeVecNotInitializedError` | HIGH |
| 19 | `test_dimension_mismatch` | addVectors | Throws on mismatched dims | MEDIUM |
| 20 | `test_invalid_filter_syntax` | similaritySearchVectorWithScore | Throws on malformed DSL | MEDIUM |

**Minimum:** 15 tests passing. **Target:** 20 tests.

---

## Test Infrastructure

### Mock Embeddings

```typescript
// [Day 4 inconsistency note] MockEmbeddings must satisfy the full EmbeddingsInterface.
// In practice, extend the Embeddings base class from @langchain/core to inherit
// required fields (caller, name, etc.) rather than implementing the interface directly.
class MockEmbeddings extends Embeddings {
  private dimension: number;

  constructor(dimension = 128) {
    super({});
    this.dimension = dimension;
  }

  async embedDocuments(texts: string[]): Promise<number[][]> {
    return texts.map(() => Array.from({ length: this.dimension }, () => Math.random()));
  }

  async embedQuery(text: string): Promise<number[]> {
    return Array.from({ length: this.dimension }, () => Math.random());
  }
}
```

### WASM Mock / Init

Tests need WASM initialized. Two strategies:
1. **Real WASM:** `beforeAll(() => initEdgeVec())` — heavier but tests real behavior
2. **Mock index:** Mock `EdgeVecIndex` class — faster but less realistic

**Decision:** Use real WASM for integration-like tests (W43.4a-c, W43.4e), mock for unit tests (W43.4d, W43.4f, W43.4g).

---

## Acceptance Criteria

- [ ] 15+ unit tests, all passing
- [ ] Zero `any` type assertions in tests
- [ ] Error paths: WASM not init → `EdgeVecNotInitializedError`
- [ ] Error paths: dimension mismatch → specific error with clear message
- [ ] Error paths: invalid filter → specific error (not generic WASM panic)
- [ ] Edge cases: empty index search returns `[]`
- [ ] Edge cases: delete non-existent ID returns gracefully (no throw)
- [ ] Edge cases: `k > count` returns `min(k, count)` results
- [ ] Metadata roundtrip: all LangChain types preserved
- [ ] Score normalization: all scores in [0, 1] range
- [ ] `npx vitest run tests/store.test.ts` passes

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| WASM init slow in tests | Use `beforeAll` with single init, share across test suite |
| Flaky tests from random embeddings | Use seeded random or deterministic test vectors |
| IndexedDB not available in Node test environment | Use `fake-indexeddb` polyfill for save/load tests |

---

## Exit Criteria

**Day 4 is complete when:**
1. All 7 tasks are DONE
2. `npx vitest run tests/store.test.ts` passes with 15+ tests
3. Zero `any` type assertions
4. All error paths verified

**Handoff to Day 5:** Unit tests are comprehensive. Day 5 adds integration tests and verifies build output.

---

**END OF DAY 4 PLAN**
