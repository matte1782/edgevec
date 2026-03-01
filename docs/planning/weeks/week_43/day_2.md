# Week 43 — Day 2: Core EdgeVecStore Implementation

**Date:** 2026-03-08
**Status:** [REVISED] — Hostile review fixes applied (C1, C2, C3, M2, M3)
**Focus:** Implement `EdgeVecStore` class with core methods: `addVectors`, `similaritySearchVectorWithScore`, metric-aware ID management, score normalization
**Prerequisite:** Day 1 complete (types, metadata, WASM init ready)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 2 (API Mapping)

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Implement `EdgeVecStore` class extending `VectorStore` | W43.2a | 2h | PENDING | Day 1 (W43.1e, W43.1d) |
| Implement `addVectors(vectors, documents, options?)` | W43.2b | 1.5h | PENDING | W43.2a |
| Implement `similaritySearchVectorWithScore(query, k, filter?)` | W43.2c | 1.5h | PENDING | W43.2a |
| Implement ID management: string <-> numeric bidirectional map | W43.2d | 1h | PENDING | W43.2a |
| Implement score normalization: distance -> similarity (Spike C2) | W43.2e | 0.5h | PENDING | W43.2c |

**Total Estimated Hours:** 6.5h

---

## Critical Path

```
W43.2a (class skeleton) → W43.2d (ID map) → W43.2b (addVectors) → W43.2c (search) → W43.2e (score norm)
```

ID management (W43.2d) must be ready before `addVectors` because every insert assigns a string ID mapped to EdgeVec's numeric ID.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Core store class | `pkg/langchain/src/index.ts` | `EdgeVecStore` — core methods only |

---

## Technical Decisions

### W43.2a — Class Structure

```typescript
import { VectorStore } from "@langchain/core/vectorstores";
import { Document } from "@langchain/core/documents";
import { EmbeddingsInterface } from "@langchain/core/embeddings";
import { ensureInitialized } from "./init.js";
import { serializeMetadata, deserializeMetadata } from "./metadata.js";
import type { EdgeVecStoreConfig } from "./types.js";

export class EdgeVecStore extends VectorStore {
  declare FilterType: string; // EdgeVec DSL string

  private index: EdgeVecIndex;
  private idMap: Map<string, number>;    // string → numeric
  private reverseIdMap: Map<number, string>; // numeric → string
  private metric: EdgeVecMetric; // [C1/C2 FIX] stored for score normalization

  constructor(embeddings: EmbeddingsInterface, config: EdgeVecStoreConfig) {
    super(embeddings, config);
    ensureInitialized(); // throws if WASM not loaded
    const { metric = "cosine", ...indexConfig } = config;
    this.metric = metric; // [C2 FIX] metric stored for query-time normalization
    this.index = new EdgeVecIndex(indexConfig);
    // ...
  }
}
```

### W43.2d — ID Management

- **String IDs:** Generated via `crypto.randomUUID()` (browser-native, no deps)
- **Numeric IDs:** Use the return value of `EdgeVecIndex.add()` as source of truth (NOT an independent counter) **[M3 FIX]**
- **Bidirectional map:** `Map<string, number>` + `Map<number, string>`
- **On delete:** Both maps are cleaned up
- **On save/load:** Maps serialized as part of index metadata (Spike C3, addressed Day 3)
- **No `nextNumericId` counter:** The EdgeVec WASM layer assigns IDs; adapter just records the mapping **[M3 FIX]**

### W43.2e — Score Normalization (Spike C2)

LangChain convention: higher score = more similar. EdgeVec returns distances.

| Metric | EdgeVec Returns | Normalization | Formula |
|:-------|:----------------|:-------------|:--------|
| L2 (Euclidean) | Distance (0 = identical) | `1 / (1 + distance)` | sim ∈ (0, 1], 0 → 1.0 |
| Cosine | Distance (0 = identical) | `1 - distance` | sim ∈ [0, 1], 0 → 1.0 |
| Dot Product | Negative inner product | `1 / (1 + Math.abs(distance))` | sim ∈ (0, 1] |

**[C2 FIX] Metric-Aware Normalization:** The `EdgeVecStore` stores `this.metric` (from `EdgeVecStoreConfig.metric`, default `"cosine"`). The `normalizeScore(rawScore)` private method selects the correct formula based on `this.metric`. This resolves the hostile review finding that no mechanism existed to determine the metric at query time.

**Note:** Exact formulas depend on what EdgeVec WASM search returns. Verify against `pkg/edgevec-wrapper.d.ts` during implementation.

---

## Spike Conditions Addressed

| Condition | How Addressed |
|:----------|:-------------|
| C2 (Score normalization) | W43.2e normalizes distance → similarity [0,1] |
| C3 (ID mapping) | W43.2d creates bidirectional map (persistence in Day 3) |

---

## Acceptance Criteria

- [ ] `EdgeVecStore` extends `VectorStore` from `@langchain/core`
- [ ] `addVectors` stores vectors with metadata, returns string IDs
- [ ] `similaritySearchVectorWithScore` returns `[Document, score][]` with score in [0, 1]
- [ ] ID map: `stringId → numericId` and `numericId → stringId` both work
- [ ] Filter: `'category = "gpu" AND price < 500'` passes to EdgeVec DSL and filters results
- [ ] Filter: `'status IS NULL'` passes correctly
- [ ] Filter: `'count >= 10 OR name != "test"'` passes correctly
- [ ] Score normalization: L2 distance 0.0 maps to similarity 1.0; larger distances map to lower similarity
- [ ] `ensureInitialized()` guard present in `addVectors` and `similaritySearchVectorWithScore`
- [ ] TypeScript strict mode: zero errors

---

## Implementation Notes

### Document Storage (Spike Section 2)

```typescript
// On insert: store pageContent as metadata field
const edgevecMetadata = {
  _pageContent: doc.pageContent,
  _id: stringId,
  ...serializeMetadata(doc.metadata)
};

// On retrieval: reconstruct Document
const doc = new Document({
  pageContent: result.metadata._pageContent,
  metadata: deserializeMetadata(result.metadata),
  id: result.metadata._id,
});
```

### Filter Pass-through

EdgeVec DSL string is passed directly to the WASM search function. No translation layer needed.

```typescript
async similaritySearchVectorWithScore(query, k, filter?) {
  // [C3 FIX] search() returns Promise<SearchResult[]> — must await
  // [M2 FIX] includeMetadata MUST be true, otherwise result.metadata is undefined
  const results = await this.index.search(query, k, {
    ...(filter ? { filter } : {}),
    includeMetadata: true,
  });
  // normalize scores using this.metric [C2 FIX]
  return results.map(r => [
    new Document({
      pageContent: r.metadata!._pageContent as string,
      metadata: deserializeMetadata(r.metadata!),
      id: this.reverseIdMap.get(r.id) ?? String(r.id),
    }),
    this.normalizeScore(r.score), // uses this.metric
  ]);
}
```

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| EdgeVec WASM API shape differs from spike assumptions | Verify against `pkg/edgevec-wrapper.d.ts` at implementation time |
| Score normalization formula wrong for a metric | Day 4 tests (W43.4f) verify all three metrics |
| `crypto.randomUUID()` not available in all browsers | Polyfill or fallback to simple UUID generator |

---

## Exit Criteria

**Day 2 is complete when:**
1. All 5 tasks are DONE
2. `EdgeVecStore` class compiles with TypeScript strict mode
3. Core methods (`addVectors`, `similaritySearchVectorWithScore`) are implemented
4. ID management works bidirectionally

**Handoff to Day 3:** Core class is functional for add + search. Day 3 adds delete, factory methods, and save/load.

---

**END OF DAY 2 PLAN**
