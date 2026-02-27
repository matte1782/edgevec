# LangChain.js Integration Spike — W42.1

**Date:** 2026-02-28
**Status:** [APPROVED] — GO
**Effort Estimate:** 21-33 hours (~3-4 days)
**Decision:** Proceed with implementation in W43.1

---

## 1. VectorStore Interface Analysis

### Required Methods (MUST Override)

```typescript
// From @langchain/core VectorStore abstract class:

abstract addVectors(
  vectors: number[][],
  documents: DocumentInterface[],
  options?: AddDocumentOptions
): Promise<string[] | void>;

abstract similaritySearchVectorWithScore(
  query: number[],
  k: number,
  filter?: this["FilterType"]
): Promise<[DocumentInterface, number][]>;
```

### Methods with Default Implementations (FREE)

| Method | Default Behavior |
|:-------|:-----------------|
| `addDocuments(docs)` | Embeds via `this.embeddings`, calls `addVectors` |
| `similaritySearch(query, k, filter?)` | Embeds query, calls `similaritySearchVectorWithScore` |
| `similaritySearchWithScore(query, k, filter?)` | Embeds query, calls above |
| `maxMarginalRelevanceSearch(query, options)` | Implements MMR using search results |
| `asRetriever(kOrFields?)` | Returns VectorStoreRetriever wrapper |
| `delete(params?)` | Optional — many stores leave as no-op |

### Static Factory Methods (Conventional)

```typescript
static fromTexts(texts, metadatas, embeddings, config): Promise<VectorStore>;
static fromDocuments(docs, embeddings, config): Promise<VectorStore>;
```

---

## 2. EdgeVec API Mapping

| LangChain Method | EdgeVec Equivalent | Complexity |
|:-----------------|:-------------------|:-----------|
| `addVectors(vectors, docs)` | `index.add(vector, metadata)` per vector | LOW |
| `similaritySearchVectorWithScore(query, k, filter?)` | `index.search(query, k, { filter })` | LOW |
| `delete({ ids })` | `index.delete(id)` per ID | LOW |
| `fromTexts(texts, metadatas, embeddings)` | Create index, embed, add | LOW |
| `fromDocuments(docs, embeddings)` | Create index, call `addDocuments` | LOW |
| Save/Load | `index.save(name)` / `EdgeVecIndex.load(name)` | LOW |

### Document Storage Strategy

LangChain stores `pageContent` + `metadata`. EdgeVec stores metadata per vector.

```typescript
// On insert: store pageContent as metadata field
const edgevecMetadata = {
  _pageContent: doc.pageContent,
  _id: doc.id ?? crypto.randomUUID(),
  ...doc.metadata
};

// On retrieval: reconstruct Document
const doc = new Document({
  pageContent: result.metadata._pageContent,
  metadata: { ...result.metadata },
  id: result.metadata._id,
});
delete doc.metadata._pageContent;
delete doc.metadata._id;
```

---

## 3. Metadata Type Compatibility

| LangChain Type | EdgeVec MetadataValue | Status |
|:---------------|:----------------------|:-------|
| `string` | `string` | SUPPORTED |
| `number` | `number` | SUPPORTED |
| `boolean` | `boolean` | SUPPORTED |
| `string[]` | `string[]` | SUPPORTED |
| `object` (nested) | NOT SUPPORTED | Serialize as JSON string |
| `null` | NOT SUPPORTED | Coerce to empty string |
| `number[]`, `Date` | NOT SUPPORTED | Serialize as JSON string |

**Mitigation:** JSON.stringify unsupported types, parse on retrieval. Store a `_serializedKeys` metadata field listing which keys were serialized.

---

## 4. Filter Format

LangChain's filter is implementation-defined. Options for EdgeVec:

| Filter Type | Example | Recommended |
|:------------|:--------|:------------|
| EdgeVec DSL string | `'category = "gpu" AND price < 500'` | YES — primary |
| FilterExpression object | Compiled filter | YES — advanced users |
| Callback `(doc) => boolean` | MemoryVectorStore compat | NO — too slow for WASM |

---

## 5. Competitive Analysis: Voy (Direct Comparable)

| Feature | Voy | EdgeVec |
|:--------|:----|:--------|
| Algorithm | k-d tree | HNSW |
| Metadata filtering | None | Full DSL + programmatic |
| Persistence | None | IndexedDB |
| Sparse/Hybrid search | None | Full support |
| Quantization | None | BQ (32x) + SQ8 (4x) |
| Status | Abandoned | Active (v0.9.0) |
| LangChain integration | `@langchain/community` | New package |

**EdgeVec is strictly more capable.** Voy's abandoned status creates an adoption opportunity.

---

## 6. Effort Breakdown

| Task | Hours | Notes |
|:-----|:------|:------|
| Core adapter (`EdgeVecStore`) | 4-6 | `addVectors`, `search`, `delete`, constructor |
| Metadata serialization | 2-3 | `any` -> EdgeVec types, round-trip fidelity |
| Static factory methods | 1-2 | `fromTexts`, `fromDocuments` |
| Save/Load (SaveableVectorStore) | 2-3 | Map IndexedDB persistence |
| Filter adapter | 1-2 | Pass-through EdgeVec DSL |
| ID management | 1-2 | string <-> numeric bidirectional map |
| Unit tests | 4-6 | All methods, edge cases, metadata round-trips |
| Integration tests | 2-3 | End-to-end with embeddings |
| Package setup | 2-3 | package.json, peer deps, build |
| Documentation | 2-3 | README, usage examples |
| **TOTAL** | **21-33** | ~3-4 days |

---

## 7. Risks

### HIGH
1. **Metadata type mismatch** — LangChain allows `any`, EdgeVec restricts to primitives. Must serialize/deserialize carefully.
2. **pageContent storage overhead** — Large documents stored in metadata increase memory usage.
3. **ID mapping persistence** — String↔numeric map must survive save/load cycles.

### MEDIUM
4. **Score semantics** — EdgeVec returns distance (lower=better), LangChain convention varies. Normalize to similarity.
5. **Async init** — WASM init is async, VectorStore constructors are sync. Use factory methods.

### LOW
6. **Bundle size** — Minimal JS overhead on top of WASM.
7. **Peer dep churn** — `@langchain/core` releases frequently.

---

## 8. Decision

### GO

**Conditions:**
- C1: Publish as `edgevec-langchain` (separate package), not PR to `@langchain/community`
- C2: Normalize scores to cosine similarity (higher = better)
- C3: Store ID mapping in EdgeVec metadata (survives save/load)
- C4: Pin `@langchain/core` peer dep to tested range (`>=0.3.0 <0.5.0`)
- C5: JSON.stringify unsupported metadata types with `_serializedKeys` tracking

---

## 9. Implementation Plan (W43.1)

```
pkg/langchain/
├── package.json          # edgevec-langchain, peer dep @langchain/core
├── tsconfig.json
├── src/
│   ├── index.ts          # EdgeVecStore class (~300-400 lines)
│   ├── metadata.ts       # Serialization helpers
│   └── types.ts          # Config interfaces
├── tests/
│   ├── store.test.ts     # Unit tests (10+)
│   └── integration.test.ts
└── README.md
```

---

**END OF SPIKE DOCUMENT**
