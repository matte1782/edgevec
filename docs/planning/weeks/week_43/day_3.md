# Week 43 — Day 3: Delete, Factory Methods, Save/Load

**Date:** 2026-03-09
**Status:** [REVISED] — Hostile review fixes applied (M1, M3, M4)
**Focus:** Complete the `EdgeVecStore` API — delete (with return value check), static factories, save/load persistence
**Prerequisite:** Day 2 complete (core class with `addVectors` + `similaritySearchVectorWithScore`)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 2 + Conditions C3, Risk #5

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Implement `delete({ ids })` with ID map cleanup | W43.3a | 1h | PENDING | Day 2 (W43.2d) |
| Implement `fromTexts(texts, metadatas, embeddings, config)` static factory | W43.3b | 1h | PENDING | Day 2 (W43.2a, W43.2b) |
| Implement `fromDocuments(docs, embeddings, config)` static factory | W43.3c | 0.5h | PENDING | W43.3b |
| Implement `SaveableVectorStore` interface: map `save(directory)` → `EdgeVecIndex.save(name)` (IndexedDB) | W43.3d | 2h | PENDING | Day 2 (W43.2a) |
| Ensure ID mapping persists across save/load cycles (Spike C3) | W43.3e | 1h | PENDING | W43.3d, Day 2 (W43.2d) |

**Total Estimated Hours:** 5.5h

---

## Critical Path

```
W43.3a (delete) ──────────────────────────────────────┐
W43.3b (fromTexts) → W43.3c (fromDocuments)           │
W43.3d (save/load) → W43.3e (ID persistence)          │
                                                       ↓
                                            Day 3 Complete
```

Tasks W43.3a, W43.3b, and W43.3d can start in parallel (all depend only on Day 2). W43.3c depends on W43.3b. W43.3e depends on W43.3d.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Complete store class | `pkg/langchain/src/index.ts` | All methods implemented |

---

## Technical Decisions

### W43.3a — Delete with ID Map Cleanup

```typescript
async delete(params: { ids: string[] }): Promise<void> {
  ensureInitialized();
  for (const stringId of params.ids) {
    const numericId = this.idMap.get(stringId);
    if (numericId === undefined) continue; // no-op for unknown IDs
    // [M4 FIX] Check delete() return value — true if deleted, false if not found
    const deleted = this.index.delete(numericId);
    if (!deleted) {
      console.warn(`EdgeVecStore: delete(${numericId}) returned false — vector may already be deleted`);
    }
    // Clean maps regardless — if we had the mapping, it should be removed
    this.idMap.delete(stringId);
    this.reverseIdMap.delete(numericId);
  }
}
```

**Key:** Unknown string IDs are silently ignored (no-op), matching LangChain convention. If EdgeVec-level delete returns `false` (already soft-deleted), maps are still cleaned and a warning is logged. **[M4 FIX]**

### W43.3b/c — Factory Methods

```typescript
static async fromTexts(
  texts: string[],
  metadatas: Record<string, any>[] | Record<string, any>,
  embeddings: EmbeddingsInterface,
  config: EdgeVecStoreConfig
): Promise<EdgeVecStore> {
  await initEdgeVec(); // Spike Risk #5: auto-init WASM
  const docs = texts.map((text, i) => new Document({
    pageContent: text,
    metadata: Array.isArray(metadatas) ? metadatas[i] ?? {} : metadatas,
  }));
  const store = new EdgeVecStore(embeddings, config);
  await store.addDocuments(docs); // uses default addDocuments → embeddings.embedDocuments → addVectors
  return store;
}
```

**`fromDocuments`:** Takes `(docs: Document[], embeddings, config)` — the `embeddings` param is the embedding model, NOT pre-computed embeddings. Creates store, calls `store.addDocuments(docs)` which embeds via `this.embeddings.embedDocuments()`. Does NOT handle pre-computed embeddings — that's a separate use case outside the LangChain standard interface.

### W43.3d — Save/Load (IndexedDB Mapping)

- **`save(directory)`:** `directory` string is used as IndexedDB key prefix (browser has no filesystem)
- **`load(directory, embeddings)`:** Reconstructs `EdgeVecStore` from IndexedDB
- **ID map storage:** Serialized as JSON metadata alongside the EdgeVec index data

```typescript
async save(directory: string): Promise<void> {
  ensureInitialized();
  // Save EdgeVec index
  await this.index.save(directory);
  // Save ID mapping separately in IndexedDB
  // [M3 FIX] No nextNumericId — EdgeVec.add() return value is source of truth
  const idMapData = {
    idMap: Object.fromEntries(this.idMap),
    reverseIdMap: Object.fromEntries(this.reverseIdMap),
    metric: this.metric, // [C2 FIX] persist metric for correct normalization on load
  };
  await saveToIndexedDB(`${directory}__idmap`, JSON.stringify(idMapData));
}
```

### W43.3e — ID Mapping Persistence (Spike C3)

The ID map is stored as a separate IndexedDB entry keyed by `${directory}__idmap`. On load:

1. Load EdgeVec index from IndexedDB
2. Load ID map JSON from IndexedDB
3. Reconstruct `Map<string, number>` and `Map<number, string>`
4. Restore `metric` field for score normalization **[C2/M3 FIX]** (no `nextNumericId` — IDs come from EdgeVec)

**Failure mode:** If ID map is missing (corrupted save), throw `EdgeVecPersistenceError` with clear message.

---

## Spike Conditions Addressed

| Condition | How Addressed |
|:----------|:-------------|
| C3 (ID mapping survives save/load) | W43.3e stores/loads ID maps alongside index |
| Risk #5 (WASM async init) | W43.3b/c factory methods call `await initEdgeVec()` |

---

## Acceptance Criteria

- [ ] `delete` removes vectors AND cleans up ID mapping (both maps)
- [ ] `delete` with unknown IDs is a no-op (no error)
- [ ] `fromTexts` creates store, embeds texts, adds vectors in one call
- [ ] `fromDocuments` handles `Document[]` with metadata
- [ ] Save/load roundtrip preserves: vectors, metadata, ID mapping, pageContent
- [ ] `save(directory)` maps directory string to IndexedDB name
- [ ] `load(directory, embeddings)` reconstructs from IndexedDB with full state
- [ ] Factory methods call `await initEdgeVec()` before creating store (Spike Risk #5)
- [ ] Double-init is safe: calling `initEdgeVec()` twice does not error
- [ ] Missing ID map on load throws `EdgeVecPersistenceError` (not silent corruption)
- [ ] TypeScript strict mode: zero errors

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| IndexedDB quota exceeded with large indices | Document size limits in README (Day 6) |
| ID map JSON too large for very large stores | Test with 10k+ vectors to verify (Day 4 integration tests can cover) |
| `SaveableVectorStore` interface not exported from `@langchain/core` | **[M1 FIX] Concrete fallback:** If `SaveableVectorStore` does not exist in the target `@langchain/core` range (>=0.3.0 <0.5.0), implement `save(directory: string): Promise<void>` and `static load(directory: string, embeddings: EmbeddingsInterface): Promise<EdgeVecStore>` as standalone methods on `EdgeVecStore` WITHOUT extending `SaveableVectorStore`. Verify existence at Day 3 start before writing any save/load code. |

---

## Exit Criteria

**Day 3 is complete when:**
1. All 5 tasks are DONE
2. `EdgeVecStore` has complete API: add, search, delete, fromTexts, fromDocuments, save, load
3. Factory methods auto-init WASM
4. ID mapping persists across save/load
5. TypeScript strict mode: zero errors

**Handoff to Day 4:** Full API is implemented. Day 4 writes comprehensive unit tests.

---

**END OF DAY 3 PLAN**
