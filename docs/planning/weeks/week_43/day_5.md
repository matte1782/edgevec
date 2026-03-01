# Week 43 — Day 5: Integration Tests + Default Method Verification + Build

**Date:** 2026-03-11
**Status:** [REVISED] — Updated after Day 4 completion; removed W43.5d (MMR does not exist on installed @langchain/core)
**Focus:** End-to-end RAG pipeline tests, verify LangChain default method behavior, validate ESM + CJS dual build
**Prerequisite:** Day 4 complete (101 tests passing: 40 metadata + 61 store)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 1 (Default Methods)

---

## Gap Analysis (Post Day 4)

**Days 1-4 delivered:**
- `pkg/langchain/src/` — 5 modules (index, store, types, metadata, init)
- `pkg/langchain/tests/metadata.test.ts` — 40 tests
- `pkg/langchain/tests/store.test.ts` — 61 tests (0 `as any`, hostile review APPROVED)

**Day 5 must deliver:**
1. `pkg/langchain/tests/integration.test.ts` — **NEW FILE** (does not exist yet)
2. `pkg/langchain/dist/` — **NOT YET BUILT** (tsup configured but never run)
3. Verification of inherited LangChain methods (`similaritySearch`, `similaritySearchWithScore`, `asRetriever`)

**Plan correction:** `maxMarginalRelevanceSearch` does NOT exist on the installed `@langchain/core@0.3.x` VectorStore prototype. Task W43.5d is **DROPPED** and replaced with `similaritySearch` string-query test (the actual default method that calls embedQuery + similaritySearchVectorWithScore).

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Create integration test with mock embeddings (no external API) | W43.5a | 1.5h | PENDING | Day 4 |
| Test full RAG pipeline: embed -> store -> search -> retrieve | W43.5b | 1h | PENDING | W43.5a |
| Test `similaritySearch` default (string query -> embedQuery -> search) | W43.5c | 0.5h | PENDING | Day 3 |
| Test `similaritySearchWithScore` default (string query variant) | W43.5d | 0.5h | PENDING | Day 3 |
| Test `asRetriever()` returns functional `VectorStoreRetriever` | W43.5e | 0.5h | PENDING | Day 3 |
| Verify build: ESM + CJS dual output via `tsup` | W43.5f | 1h | PENDING | Day 3 |
| Verify peer dep: test with `@langchain/core@0.3.x` and `@langchain/core@0.4.x` | W43.5g | 0.5h | PENDING | W43.5f |

**Total Estimated Hours:** 5.5h

---

## Critical Path

```
W43.5a (integration setup) -> W43.5b (RAG pipeline)
W43.5c + W43.5d + W43.5e (default methods, parallel)
W43.5f (build) -> W43.5g (peer dep compat)
```

Three independent tracks can run in parallel.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Integration tests | `pkg/langchain/tests/integration.test.ts` | E2E RAG pipeline + default method tests |
| Build output | `pkg/langchain/dist/` | ESM + CJS compiled output |

---

## Test Scenarios

### W43.5a — Integration Test Setup

The integration test file uses the same mock infrastructure as `store.test.ts` but focuses on **multi-step workflows** rather than individual methods.

```typescript
import { Embeddings } from "@langchain/core/embeddings";

class DeterministicEmbeddings extends Embeddings {
  constructor(private dims = 128) { super({}); }

  async embedDocuments(texts: string[]): Promise<number[][]> {
    // Deterministic: hash-based vectors so similar texts get similar vectors
    return texts.map((text) => {
      let hash = 0;
      for (let i = 0; i < text.length; i++) hash = (hash * 31 + text.charCodeAt(i)) | 0;
      return Array.from({ length: this.dims }, (_, d) => Math.sin(hash + d) * 0.5 + 0.5);
    });
  }

  async embedQuery(text: string): Promise<number[]> {
    return (await this.embedDocuments([text]))[0];
  }
}
```

Key difference from unit test `MockEmbeddings`: **deterministic and text-dependent** — different texts produce different vectors, enabling meaningful similarity ranking tests.

### W43.5b — Full RAG Pipeline

```
Input texts -> Embed -> Add to store -> Query (string) -> Get documents back
```

Verify:
1. Documents returned have correct `pageContent`
2. Documents returned have correct `metadata`
3. Scores are in [0, 1] range
4. Document IDs are preserved through the pipeline
5. Results array length <= k

### W43.5c — Default `similaritySearch`

The `VectorStore` base class provides `similaritySearch(query, k, filter)` which:
1. Calls `this.embeddings.embedQuery(query)` to get a vector
2. Calls `this.similaritySearchVectorWithScore(vector, k, filter)` to get results
3. Returns just the `Document[]` (strips scores)

Verify this chain works correctly with EdgeVecStore.

### W43.5d — Default `similaritySearchWithScore`

Same as `similaritySearch` but returns `[Document, score][]` (preserves scores). Verify scores are normalized.

### W43.5e — `asRetriever()`

```typescript
const retriever = store.asRetriever({ k: 3 });
const results = await retriever.invoke("test query");
// results should be Document[]
```

Verify:
- `retriever.invoke()` returns `Document[]`
- Results have `pageContent` and `metadata`
- Result count <= k

### W43.5f — Build Verification

```bash
cd pkg/langchain && npm run build
```

Verify:
- `dist/index.js` or `dist/index.mjs` exists (ESM output)
- `dist/index.cjs` exists (CJS output)
- `dist/index.d.ts` or `dist/index.d.cts` exists (type declarations)
- No TypeScript errors in strict mode
- Bundle size < 10KB (excluding edgevec WASM)

**Note:** tsup.config.ts output structure may differ from the `dist/esm/` + `dist/cjs/` layout described in the original plan. Check actual tsup output paths.

### W43.5g — Peer Dep Compatibility

Test installation with:
- `@langchain/core@0.3.0` (lower bound)
- `@langchain/core@0.4.x` (latest in range, if available)

Verify `npm install` succeeds and basic import works. This is a **verification task**, not a separate test suite — run in a temp directory with `npm pack` output.

---

## Implementation Notes

### Mock Strategy

Integration tests use the same WASM mock pattern as `store.test.ts`:
- `vi.mock("edgevec", ...)` — mock WASM init
- `vi.mock("edgevec/edgevec-wrapper.js", ...)` — mock EdgeVecIndex

But the mock search implementation should be **smarter** for RAG pipeline tests: return results that correspond to what was added, so the pipeline test is meaningful.

### Reuse from Day 4

- `MockEmbeddings` class pattern (extends `Embeddings`)
- `testInternals()` accessor pattern
- IDB cleanup in `beforeEach`
- Same `vi.mock` setup (can be extracted to a shared test helper if needed)

### Available Default Methods (Verified)

From `@langchain/core@0.3.x` VectorStore prototype:
- `similaritySearch(query, k, filter)` — returns `Document[]`
- `similaritySearchWithScore(query, k, filter)` — returns `[Document, score][]`
- `asRetriever(kOrFields, filter, callbacks, tags, metadata, verbose)` — returns `VectorStoreRetriever`
- `delete(params)` — overridden by EdgeVecStore

**NOT available:** `maxMarginalRelevanceSearch` — does not exist on installed version.

---

## Acceptance Criteria

- [ ] Integration tests pass with mock embeddings (no external API calls)
- [ ] Full RAG pipeline: text -> embed -> add -> query -> get document back with correct content
- [ ] `similaritySearch` default: calls `embedQuery`, returns `Document[]` without scores
- [ ] `similaritySearchWithScore` default: calls `embedQuery`, returns `[Document, score][]`
- [ ] `asRetriever()`: returned retriever can `.invoke(query)` and get `Document[]`
- [ ] `npm run build` produces both ESM and CJS output
- [ ] TypeScript strict mode: zero errors (`npx tsc --noEmit`)
- [ ] Bundle size of langchain adapter < 10KB (excluding edgevec WASM)
- [ ] Peer dep: installs cleanly with `@langchain/core@0.3.x`

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| `asRetriever().invoke()` requires async chain through VectorStoreRetriever | Test with proper await chain; inspect return type |
| CJS build fails due to ESM-only dependencies | tsup handles dual output; if `@langchain/core` is ESM-only, CJS may need `require()` shim |
| Bundle size exceeds 10KB | Profile imports; tsup externalizes peer deps already (`edgevec`, `@langchain/core`) |
| tsup output paths differ from expected `dist/esm/` + `dist/cjs/` | Check actual tsup output after first build; update package.json `exports` if needed |

---

## Exit Criteria

**Day 5 is complete when:**
1. All 7 tasks are DONE
2. `npx vitest run` passes (all tests: metadata + store + integration)
3. `npm run build` produces ESM + CJS output
4. TypeScript strict mode: zero errors
5. Bundle size < 10KB

**Handoff to Day 6:** Implementation and tests are complete. Day 6 writes documentation.

---

**END OF DAY 5 PLAN**
