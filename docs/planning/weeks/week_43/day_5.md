# Week 43 — Day 5: Integration Tests + Default Method Verification + Build

**Date:** 2026-03-11
**Status:** [REVISED] — Hostile review fixes applied (C1 config clarification)
**Focus:** End-to-end RAG pipeline tests, verify LangChain default method behavior, validate ESM + CJS dual build
**Prerequisite:** Day 4 complete (unit tests passing)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 1 (Default Methods)

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Create integration test with mock embeddings (no external API) | W43.5a | 1.5h | PENDING | Day 4 |
| Test full RAG pipeline: embed → store → search → retrieve | W43.5b | 1h | PENDING | W43.5a |
| Test `addDocuments` default implementation (embeds via `this.embeddings`) | W43.5c | 0.5h | PENDING | Day 3 |
| Test `maxMarginalRelevanceSearch` default with EdgeVec scores | W43.5d | 0.5h | PENDING | Day 3 |
| Test `asRetriever()` returns functional `VectorStoreRetriever` | W43.5e | 0.5h | PENDING | Day 3 |
| Verify build: ESM + CJS dual output | W43.5f | 1h | PENDING | Day 3 |
| Verify peer dep: test with `@langchain/core@0.3.x` and `@langchain/core@0.4.x` | W43.5g | 0.5h | PENDING | W43.5f |

**Total Estimated Hours:** 5.5h

---

## Critical Path

```
W43.5a (integration setup) → W43.5b (RAG pipeline)
W43.5c + W43.5d + W43.5e (default methods, parallel)
W43.5f (build) → W43.5g (peer dep compat)
```

Three independent tracks can run in parallel.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Integration tests | `pkg/langchain/tests/integration.test.ts` | E2E RAG pipeline tests |
| Build output | `pkg/langchain/dist/` | ESM + CJS compiled output |

---

## Test Scenarios

### W43.5a — Integration Test Setup

```typescript
describe("EdgeVecStore Integration", () => {
  let store: EdgeVecStore;
  let embeddings: MockEmbeddings;

  beforeAll(async () => {
    await initEdgeVec();
    embeddings = new MockEmbeddings(128);
  });

  beforeEach(() => {
    // [C1 FIX] metric is an EdgeVecStoreConfig field, NOT IndexConfig.
    // EdgeVecStore constructor extracts it before passing to EdgeVecIndex.
    store = new EdgeVecStore(embeddings, {
      dimensions: 128,
      metric: "cosine", // EdgeVecStoreConfig.metric (default), NOT IndexConfig
    });
  });
});
```

### W43.5b — Full RAG Pipeline

```
Input texts → Embed → Add to store → Query → Get documents back
```

Verify:
1. Documents returned have correct `pageContent`
2. Documents returned have correct `metadata`
3. Scores are in [0, 1] range
4. Most similar document is ranked first

### W43.5c — Default `addDocuments`

The `VectorStore` base class provides `addDocuments` which calls `this.embeddings.embedDocuments()` then `this.addVectors()`. Verify this works correctly with `EdgeVecStore`.

### W43.5d — Default `maxMarginalRelevanceSearch`

Verify that the MMR default implementation works with EdgeVec's score format. May return fewer results than `k` — that's expected behavior.

### W43.5e — `asRetriever()`

```typescript
const retriever = store.asRetriever({ k: 3 });
const results = await retriever.invoke("test query");
// results should be Document[]
```

### W43.5f — Build Verification

```bash
cd pkg/langchain && npm run build
```

Verify:
- `dist/esm/index.js` exists (ESM output)
- `dist/cjs/index.js` exists (CJS output)
- `dist/types/index.d.ts` exists (type declarations)
- No TypeScript errors in strict mode

### W43.5g — Peer Dep Compatibility

Test installation with:
- `@langchain/core@0.3.0` (lower bound)
- `@langchain/core@0.4.x` (latest in range)

Verify `npm install` succeeds and basic import works.

---

## Acceptance Criteria

- [ ] Integration tests pass with mock embeddings (no external API calls)
- [ ] Full RAG pipeline: text → embed → add → query → get document back with correct content
- [ ] `addDocuments` default: calls `embeddings.embedDocuments`, then `addVectors` — works correctly
- [ ] `maxMarginalRelevanceSearch`: returns results without error, result count <= k
- [ ] `asRetriever()`: returned retriever can `.invoke(query)` and get `Document[]`
- [ ] `npm run build` produces both ESM and CJS output
- [ ] TypeScript strict mode: zero errors
- [ ] Bundle size of langchain adapter < 10KB (excluding edgevec WASM)
- [ ] Peer dep: installs cleanly with `@langchain/core@0.3.x` and `@langchain/core@0.4.x`

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| `maxMarginalRelevanceSearch` requires specific score format | Test and adapt if needed; may need to override default |
| CJS build fails due to ESM-only dependencies | Use `tsup` or `unbuild` for dual output if `tsc` is insufficient |
| Bundle size exceeds 10KB | Profile imports, tree-shake unused LangChain internals |

---

## Exit Criteria

**Day 5 is complete when:**
1. All 7 tasks are DONE
2. `npx vitest run tests/integration.test.ts` passes
3. `npm run build` produces ESM + CJS output
4. Peer dep compatibility verified
5. Bundle size < 10KB

**Handoff to Day 6:** Implementation and tests are complete. Day 6 writes documentation.

---

**END OF DAY 5 PLAN**
