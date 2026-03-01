# Week 43 — Day 1: Package Scaffolding + Core Types + WASM Init Design

**Date:** 2026-03-07
**Status:** [REVISED] — Hostile review fixes applied (C1, C2, m3)
**Focus:** Establish `pkg/langchain/` project structure, core types (including `metric` field), metadata serialization, WASM init strategy
**Prerequisite:** W42.1 LangChain Spike [APPROVED — GO]
**Reference:** `docs/research/LANGCHAIN_SPIKE.md`

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Create `pkg/langchain/` directory structure | W43.1a | 0.5h | PENDING | None |
| Create `package.json` with peer deps (`@langchain/core >=0.3.0 <0.5.0`) | W43.1b | 0.5h | PENDING | W43.1a |
| Create `tsconfig.json` (ESM + CJS dual output) + Vitest config | W43.1c | 0.5h | PENDING | W43.1a |
| Design WASM initialization strategy (document in `src/init.ts`) | W43.1d | 1h | PENDING | W43.1a |
| Define `types.ts`: `EdgeVecStoreConfig`, `EdgeVecStoreArgs` interfaces (including `metric` field) | W43.1e | 1h | PENDING | W43.1d |
| Implement `metadata.ts`: serialize/deserialize helpers + `_serializedKeys` tracking | W43.1f | 2h | PENDING | W43.1e |
| Write unit tests for metadata serialization (nested objects, null, Date, arrays) | W43.1g | 1.5h | PENDING | W43.1f |

**Total Estimated Hours:** 7h

---

## Critical Path

```
W43.1a (scaffold) → W43.1b + W43.1c (config, parallel) → W43.1d (WASM init) → W43.1e (types) → W43.1f (metadata) → W43.1g (tests)
```

Tasks W43.1b and W43.1c can run in parallel after W43.1a. Everything else is sequential.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Package manifest | `pkg/langchain/package.json` | npm package config with peer deps |
| TypeScript config | `pkg/langchain/tsconfig.json` | ESM + CJS dual output |
| Vitest config | `pkg/langchain/vitest.config.ts` | Test runner config |
| WASM init module | `pkg/langchain/src/init.ts` | Singleton WASM initialization |
| Type definitions | `pkg/langchain/src/types.ts` | Config interfaces |
| Metadata helpers | `pkg/langchain/src/metadata.ts` | Serialize/deserialize + `_serializedKeys` |
| Metadata tests | `pkg/langchain/tests/metadata.test.ts` | Unit tests for serialization |

---

## Technical Decisions

### W43.1d — WASM Init Strategy

- **Constructor:** Throws `EdgeVecNotInitializedError` if WASM not loaded
- **Factory methods** (`fromTexts`, `fromDocuments`): Call `await initEdgeVec()` automatically
- **`initEdgeVec()`:** Global singleton, safe to call multiple times (idempotent)
- **`ensureInitialized()`:** Guard at entry of every public method
- **Rationale:** Sync constructors cannot await WASM init; factory methods can. This matches LangChain conventions where `fromTexts`/`fromDocuments` are async.

### W43.1e — `EdgeVecStoreConfig` Must Include `metric`

**HOSTILE REVIEW FIX [C1]:** The underlying `IndexConfig` (from `edgevec-wrapper.d.ts`) does NOT have a `metric` field. It only has `dimensions`, `m`, `efConstruction`, `quantized`. The metric is configured at the low-level `EdgeVecConfig` layer, not the wrapper's `IndexConfig`.

**Solution:** `EdgeVecStoreConfig` adds a `metric` field that is NOT passed to `IndexConfig` but stored internally by the adapter for score normalization:

```typescript
export type EdgeVecMetric = "cosine" | "l2" | "dotproduct";

export interface EdgeVecStoreConfig extends IndexConfig {
  /** Metric used for score normalization. NOT passed to EdgeVec IndexConfig.
   *  Stored internally to select correct normalization formula at query time.
   *  @default "cosine" */
  metric?: EdgeVecMetric;
}
```

The `EdgeVecStore` constructor extracts `metric` from the config, stores it as `this.metric`, and passes the remaining fields to `new EdgeVecIndex(indexConfig)`. This solves **[C2]** — the adapter always knows which metric is active at query time.

### W43.1f — Metadata Serialization

**Type mapping (Spike Section 3):**

| LangChain Type | EdgeVec MetadataValue | Action |
|:---------------|:----------------------|:-------|
| `string` | `string` | Pass-through |
| `number` | `number` | Pass-through |
| `boolean` | `boolean` | Pass-through |
| `string[]` | `string[]` | Pass-through |
| `object` (nested) | NOT SUPPORTED | `JSON.stringify`, track in `_serializedKeys` |
| `null` | NOT SUPPORTED | Coerce to empty string `""`, track in `_serializedKeys` |
| `number[]`, `Date` | NOT SUPPORTED | `JSON.stringify`, track in `_serializedKeys` |

**Circular reference detection:** Use `JSON.stringify` with a replacer that tracks seen objects. Throw `MetadataSerializationError` on circular reference.

---

## Spike Conditions Addressed

| Condition | How Addressed |
|:----------|:-------------|
| C1 (Separate package) | W43.1a + W43.1b create `edgevec-langchain` package |
| C4 (Pin peer dep) | W43.1b sets `@langchain/core >=0.3.0 <0.5.0` |
| C5 (JSON.stringify unsupported types) | W43.1f implements serialization + `_serializedKeys` |

---

## Acceptance Criteria

- [ ] `npm install` in `pkg/langchain/` succeeds
- [ ] Metadata round-trip: `deserialize(serialize(obj))` === `obj` for all LangChain types
- [ ] `_serializedKeys` correctly tracks which keys were JSON-stringified
- [ ] `null` coerced to empty string, round-trips back to `null`
- [ ] Circular object reference throws `MetadataSerializationError` (not silent corruption)
- [ ] WASM init strategy documented in `src/init.ts` with JSDoc
- [ ] All unit tests pass: `npx vitest run tests/metadata.test.ts`
- [ ] TypeScript strict mode: zero errors in all new files

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| `@langchain/core` version not available | Pin to tested range, verify `npm install` |
| Metadata edge cases missed | Extensive test coverage in W43.1g |
| WASM init design doesn't account for SSR | Document browser-only limitation in `init.ts` JSDoc |
| `fake-indexeddb` missing from devDependencies | Add to `package.json` in W43.1b (needed for Day 4 save/load tests) |

---

## Exit Criteria

**Day 1 is complete when:**
1. All 7 tasks are DONE
2. `npm install` succeeds in `pkg/langchain/`
3. `npx vitest run tests/metadata.test.ts` passes
4. No TypeScript errors in strict mode

**Handoff to Day 2:** Core types and metadata module are ready. Day 2 builds `EdgeVecStore` class on top of these foundations.

---

**END OF DAY 1 PLAN**
