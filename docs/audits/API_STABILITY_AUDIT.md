# EdgeVec API Stability Audit

**Author:** META_ARCHITECT
**Date:** 2026-03-27
**Status:** [REVISED]
**Purpose:** Identify breaking changes needed before v1.0 API freeze
**Scope:** Rust lib (`edgevec` crate) + WASM (`wasm::EdgeVec`) + edgevec-langchain adapter
**Current Version:** v0.9.0
**Companion:** [`API_SURFACE_INVENTORY.md`](API_SURFACE_INVENTORY.md) — complete API inventory (338 items)

---

## Requirements I Identified

- [R1] All public API surfaces must be audited before v1.0 semver lock
- [R2] Naming consistency across Rust, WASM (JS), and TypeScript layers
- [R3] Search result field naming must be consistent (score vs distance)
- [R4] Error handling must be uniform (no mix of plain JsValue strings and structured error objects)
- [R5] Legacy/duplicate APIs must be deprecated or removed
- [R6] Type safety at WASM boundary (avoid untyped JsValue where structured types are possible)
- [R7] Internal implementation details must not leak through public API

## Hard Constraints

- [C1] v1.0 = semver guarantee: no breaking changes until v2.0
- [C2] WASM bundle < 500KB gzipped
- [C3] ~980 Rust tests + 149 LangChain tests must continue to pass
- [C4] Backwards compatibility with existing IndexedDB persisted data
- [C5] `@langchain/core@0.3.x` peer dependency compatibility

---

## 1. Breaking Change Candidates

### 1.1 Rust API

| # | API | Current | Proposed Change | Priority | Impact | Rationale |
|:--|:----|:--------|:----------------|:---------|:-------|:----------|
| R1 | `HnswGraph` type alias | `pub type HnswGraph = HnswIndex;` in `hnsw/mod.rs` line 27 | Remove the alias. `HnswIndex` is the canonical name. | P0 | LOW | Legacy alias pollutes the public API. Anyone using `HnswGraph` should have migrated to `HnswIndex` by now. Keeping it in v1.0 means supporting it forever. |
| R2 | `HnswConfig.metric` field | `pub metric: u32` with magic constants (`METRIC_L2_SQUARED = 0`, etc.) | **DEFERRED to v1.1.** Replace with a proper `enum Metric { L2Squared, Cosine, DotProduct, Hamming }` and derive Serialize/Deserialize. Config keeps `metric: Metric`. | P2 | MEDIUM | Raw `u32` with associated constants is error-prone, but changing this requires a persistence format migration (it's a `#[repr(C)]` struct). Deferring to v1.1 avoids coupling this with the v1.0 format freeze. The existing `METRIC_*` constants are well-documented and work correctly. |
| R3 | `HnswConfig._reserved` field | `pub _reserved: [u32; 2]` | Make private: `_reserved: [u32; 2]` (remove `pub`). | P0 | LOW | Reserved fields should never be public. Users should not read or write them. The `#[allow(clippy::pub_underscore_fields)]` lint suppression in `lib.rs` exists solely because of this. |
| R4 | `VectorId.0` field visibility | `pub struct VectorId(pub u64)` | Keep public but document that the inner value is 1-based for HNSW (see ID conventions in MEMORY.md). Consider adding `VectorId::as_u64()` method as the preferred accessor. | P2 | LOW | Tuple struct with public field is idiomatic for newtype wrappers. No change needed but documentation is required. |
| R5 | `SearchResult.distance` field name | `pub distance: f32` in `hnsw::search::SearchResult` | No change. The field name `distance` is correct since HNSW returns distance values (lower = closer). | -- | -- | Audit confirms naming is correct for the Rust layer. |
| R6 | `pub use simd::*` re-exports | `lib.rs` re-exports `capabilities`, `detect_neon`, `select_backend`, `warn_if_suboptimal`, `SimdBackend`, `SimdCapabilities` | Reduce to `pub use simd::{SimdBackend, SimdCapabilities, capabilities}`. The functions `detect_neon`, `select_backend`, `warn_if_suboptimal` are internal diagnostics, not user-facing API. | P1 | LOW | These are implementation details that should not be part of the stable public API. Users care about `capabilities()` to check SIMD support; the rest are internal. |
| R7 | `FLAT_INDEX_MAGIC`, `FLAT_INDEX_VERSION`, `FlatIndexHeader` re-exports | Re-exported from `lib.rs` at top level | Move behind `persistence` module: `edgevec::persistence::{FLAT_INDEX_MAGIC, FLAT_INDEX_VERSION, FlatIndexHeader}`. Or remove from public API entirely since these are persistence internals. | P1 | LOW | File format constants and header structs are implementation details. Exposing them at the crate root suggests they are primary API. Users who need persistence already use `edgevec::persistence::*`. |
| R8 | `ChunkedWriter` re-export at crate root | `pub use persistence::ChunkedWriter;` in `lib.rs` | Remove crate-root re-export. Access via `edgevec::persistence::ChunkedWriter`. | P1 | LOW | Same rationale as R7. Persistence types belong in the persistence module namespace. |
| R9 | `BatchInsertable` trait re-export | `pub use batch::BatchInsertable;` in `lib.rs` | Evaluate whether this trait should be public. If it is only implemented internally, make it `pub(crate)`. If users need to implement it, keep it but move to a more discoverable location. | P2 | LOW | Unclear if external users implement this trait. |
| R10 | `StorageError` not in error hierarchy | `storage::StorageError` exists but is not wrapped by `EdgeVecError` | Add `EdgeVecError::Storage(StorageError)` variant. This ensures all errors flow through the unified error type. | P1 | LOW | Currently `VectorStorage::insert` returns `Result<VectorId, StorageError>` which does not compose with the top-level error type. Users must handle two separate error hierarchies. |
| R11 | `HnswConfig` fields are all `pub` | `pub m: u32`, `pub m0: u32`, etc. | Keep public (builder pattern would be too heavy for a config struct), but validate values in a `HnswConfig::validate()` method that is called in `HnswIndex::new()`. Currently invalid values (e.g., `m = 0`) are not caught until runtime. | P2 | LOW | Not a breaking change to add validation, but worth noting for v1.0 quality. |
| R12 | `BinaryFlatIndex::new` takes `usize` for dimensions | `pub fn new(dimensions: usize)` | Standardize to `u32` to match `HnswConfig.dimensions: u32`, `FlatIndexConfig.dimensions: u32`, and WASM API `EdgeVecConfig.dimensions: u32`. | P0 | MEDIUM | Inconsistent dimension type across index implementations. `BinaryFlatIndex` uses `usize` while everything else uses `u32`. |

### 1.2 WASM API

| # | API | Current | Proposed Change | Priority | Impact | Rationale |
|:--|:----|:--------|:----------------|:---------|:-------|:----------|
| W1 | Search result field: `score` vs `distance` inconsistency | `search()` returns `{ id, score }` but the value is actually a distance. `searchFiltered()` returns `{ id, score }` in JSON with distance semantics. `searchWithFilter()` returns `{ id, distance }`. `searchBQ()` returns `{ id, distance }` with similarity semantics. | Standardize ALL search methods to return `{ id, distance }` where `distance` is the raw metric distance (lower = closer for L2/cosine, higher = more similar for dot product). Users who need similarity scores should use the LangChain adapter which normalizes. | P0 | HIGH | This is the most critical inconsistency. `search()` calls the field `score` but fills it with `result.distance`. `searchWithFilter()` correctly uses `distance`. `searchBQ()` uses `distance` but the value is a similarity (higher = better). Users cannot rely on consistent semantics. |
| W2 | `search()` returns `JsValue` (untyped) | `pub fn search(...) -> Result<JsValue, JsValue>` | No change for v1.0 -- wasm-bindgen does not support returning typed arrays of structs. Document the shape clearly in TypeScript `.d.ts`. | -- | -- | WASM boundary limitation. |
| W3 | `searchFiltered()` returns JSON `String` | `pub fn search_filtered(...) -> Result<String, JsValue>` | This is inconsistent with `search()` which returns a JS Array directly. However, `searchFiltered` returns richer data (metadata, timing, strategy). Changing to JS objects would be a large refactor with minimal benefit. Keep as-is but document clearly. | P2 | MEDIUM | Inconsistency is documented and justified by the richer return type. |
| W4 | `searchSparse()` and `hybridSearch()` return JSON strings | Return `Result<String, JsValue>` | Same pattern as W3. Keep JSON for complex return types. | P2 | LOW | Consistent with searchFiltered pattern. |
| W5 | `insertBatchFlat` (legacy batch API) | `pub fn insert_batch_flat(...)` with flat array format | Deprecate in v0.10, remove in v1.0. `insertBatch` (Array-of-Float32Array) is the canonical API. | P0 | MEDIUM | The docs already mark this as "legacy API". Keeping it in v1.0 means supporting two batch insert patterns forever. |
| W6 | `set_metric()` accepts raw `String` | `pub fn set_metric(&mut self, metric: String)` takes `"l2"`, `"cosine"`, `"dot"`, `"hamming"` | Deprecate in favor of `setMetricType(MetricType)` which uses the typed enum. Keep `set_metric` for backwards compat but mark `#[deprecated]`. | P1 | MEDIUM | Two ways to set the same thing. The typed enum is safer. But removing the string version would break existing JS code that does `config.metric = "cosine"`. |
| W7 | Error handling inconsistency | Some errors return structured objects (`{ code, message }`), others return plain strings (`JsValue::from_str(...)`) | Standardize ALL error returns to use structured `{ code, message }` objects. Audit shows `searchWithFilter`, `searchSparse`, `hybridSearch`, `softDelete` return plain strings while `insert`, `search`, `insertBatch` return structured errors. | P0 | HIGH | Users cannot write consistent error handling code when some methods throw structured errors and others throw plain strings. |
| W8 | `softDeleteBatch` vs `softDeleteBatchCompat` | Two methods doing the same thing with slightly different input types | Remove `softDeleteBatchCompat` in v1.0. Keep only `softDeleteBatch` with `Uint32Array` input. | P1 | LOW | The "compat" variant exists for backwards compatibility with an older API. v1.0 is the time to clean this up. |
| W9 | `EdgeVecConfig.dimensions` is `pub` | Directly accessible field on wasm_bindgen struct | This is fine for WASM. No change needed. | -- | -- | |
| W10 | `searchHybrid` naming confusion | `searchHybrid` in WASM means "BQ + filter + optional rescore". But `hybridSearch` means "dense + sparse fusion". | Rename `searchHybrid` to `searchAdvanced` or `searchCombined`. Keep `hybridSearch` for the dense+sparse meaning which is the industry-standard definition. | P0 | MEDIUM | "Hybrid search" universally means dense+sparse in the vector DB ecosystem (Weaviate, Qdrant, Pinecone all use this term). EdgeVec uses it for two different things. |
| W11 | `benchmarkHamming` and `benchmarkHammingBatch` | Public WASM exports for internal benchmarking | Move to a `benchmarks` feature flag or remove from public WASM API. These are developer tools, not user-facing API. | P1 | LOW | Benchmark functions bloat the WASM binary and confuse the API surface. |
| W12 | `init_logging()` is public | `pub fn init_logging()` | Keep but rename to `initLogging` for JS naming convention (it currently uses snake_case in the wasm_bindgen export). Verify the `js_name` attribute. | P2 | LOW | Already auto-called by `EdgeVec::new()`. Low impact. |

### 1.3 edgevec-langchain API

| # | API | Current | Proposed Change | Priority | Impact | Rationale |
|:--|:----|:--------|:----------------|:---------|:-------|:----------|
| L1 | `EdgeVecStoreConfig` extends `IndexConfig` | `export interface EdgeVecStoreConfig extends IndexConfig` where `IndexConfig` comes from `edgevec/edgevec-wrapper.js` | Ensure `IndexConfig` is stable. If the wrapper's `IndexConfig` changes, all LangChain users break. Pin the interface or copy it into the adapter types. | P1 | HIGH | External type dependency. If the WASM wrapper changes `IndexConfig`, it silently breaks the LangChain adapter's public type. |
| L2 | Constructor `_internal` parameter | `constructor(embeddings, config, _internal?)` | The `_internal` parameter is an implementation detail leaking through the public constructor signature. It appears in `.d.ts` output. | P1 | MEDIUM | Users could pass `_internal` and corrupt state. Move load-path reconstruction to a private static method and remove `_internal` from the constructor. |
| L3 | `metadatas` parameter in `fromTexts` | `metadatas: object[] \| object` | Type is too loose. Should be `Record<string, unknown>[] \| Record<string, unknown>`. | P2 | LOW | Follows LangChain convention (`object`), but `Record<string, unknown>` is more precise. This matches what `Document.metadata` expects. |
| L4 | Score normalization is adapter-only | The `metric` field on `EdgeVecStoreConfig` is not passed to EdgeVec's index | Document this clearly. Consider adding a `getMetric()` accessor so users can query which normalization is in use. | P2 | LOW | Already documented in the `types.ts` comment. No breaking change needed, just API polish. |
| L5 | `EdgeVecPersistenceError` and `MetadataSerializationError` | Custom error classes in `types.ts` | These are well-defined. No change needed. | -- | -- | |
| L6 | `save(directory)` / `load(directory)` parameter name | Parameter is `directory` but it is an IndexedDB key, not a filesystem path | Rename parameter to `name` or `key` in v1.0 to avoid confusion. The JSDoc already explains this, but the parameter name itself is misleading. | P1 | LOW | Every call site reads `store.save("my-index")` which looks fine, but IDE tooltips show `directory: string` which is confusing for browser environments. |

---

## 2. Deprecation Plan

### 2.1 APIs to Deprecate Before v1.0

| API | Replacement | Deprecation Version | Removal Version |
|:----|:------------|:--------------------|:----------------|
| `HnswGraph` type alias | `HnswIndex` | v0.10.0 | v1.0.0 |
| `insertBatchFlat()` (WASM) | `insertBatch()` | v0.10.0 | v1.0.0 |
| `softDeleteBatchCompat()` (WASM) | `softDeleteBatch()` | v0.10.0 | v1.0.0 |
| `set_metric(String)` (WASM) | `setMetricType(MetricType)` | v0.10.0 | v1.0.0 (keep setter for convenience but validate) |
| `benchmarkHamming()` (WASM) | Feature-gated or removed | v0.10.0 | v1.0.0 |
| `benchmarkHammingBatch()` (WASM) | Feature-gated or removed | v0.10.0 | v1.0.0 |

### 2.2 Deprecation Strategy

**Rust crate:**
1. Add `#[deprecated(since = "0.10.0", note = "Use HnswIndex instead")]` to `HnswGraph`
2. Keep deprecated items for exactly one minor version (v0.10.x)
3. Remove in v1.0.0

**WASM/JS:**
1. Add `console.warn` deprecation notices in deprecated methods
2. Update TypeScript `.d.ts` with `@deprecated` JSDoc tags
3. Document migration path in CHANGELOG

**LangChain adapter:**
1. No deprecations needed -- API is clean
2. Only parameter renames and type tightening

---

## 3. API Stability Recommendations

### 3.1 v1.0 Public API Surface (Rust Crate)

**INCLUDE in stable API:**

| Module | Types/Functions | Justification |
|:-------|:----------------|:-------------|
| `edgevec` (root) | `HnswConfig`, `HnswIndex`, `VectorStorage`, `SearchResult`, `Metric`, `IndexType` | Core vector DB types |
| `edgevec` (root) | `BinaryFlatIndex`, `BinaryFlatSearchResult`, `BinaryFlatIndexError` | Binary flat index |
| `edgevec` (root) | `FlatIndex`, `FlatIndexConfig`, `FlatSearchResult`, `FlatIndexError`, `DistanceMetric` | F32 flat index |
| `edgevec::error` | `EdgeVecError`, `BatchError` | Unified error types |
| `edgevec::persistence` | `write_snapshot`, `read_snapshot`, `MemoryBackend`, `StorageBackend`, `ChunkedWriter` | Persistence API |
| `edgevec::quantization` | `BinaryQuantizer`, `QuantizedVector`, `QuantizerConfig`, `ScalarQuantizer` | Quantization |
| `edgevec::batch` | `BatchInsertable` | Batch operations trait |
| `edgevec::metadata` | `MetadataStore`, `MetadataValue` | Metadata operations |
| `edgevec::filter` | `parse`, `evaluate`, `FilterExpression` | Filter DSL |
| `edgevec::hnsw` | `VectorId`, `NodeId`, `GraphError`, `CompactionResult`, `BatchDeleteResult` | HNSW types |

**EXCLUDE from stable API (make `pub(crate)` or feature-gate):**

| Item | Current Status | Action |
|:-----|:---------------|:-------|
| `detect_neon()` | `pub` re-exported | `pub(crate)` |
| `select_backend()` | `pub` re-exported | `pub(crate)` |
| `warn_if_suboptimal()` | `pub` re-exported | `pub(crate)` |
| `FLAT_INDEX_MAGIC`, `FLAT_INDEX_VERSION` | `pub` re-exported at root | Remove root re-export; keep in `persistence` module |
| `FlatIndexHeader` | `pub` re-exported at root | Remove root re-export; keep in `index` module |
| `HnswGraph` type alias | `pub type` | Remove |
| `Candidate`, `SearchContext`, `Searcher` | `pub` in `hnsw::search` | Evaluate: if users need custom search, keep. Otherwise `pub(crate)`. |
| `NeighborPool` | `pub` in `hnsw::neighbor` | `pub(crate)` -- internal graph optimization |
| `HnswNode` | `pub` in `hnsw::graph` | Evaluate: needed for custom graph traversal? Likely `pub(crate)`. |

### 3.2 Internal APIs

The following modules should be marked as unstable or hidden:

```rust
// In lib.rs, add doc-hidden for internal modules
#[doc(hidden)]
pub mod simd; // Implementation detail; expose only SimdCapabilities

// In hnsw/mod.rs, restrict re-exports
pub use config::HnswConfig;
pub use graph::{
    BatchDeleteError, BatchDeleteResult, CompactionResult, GraphError,
    HnswIndex, VectorId, NodeId,
    // KEEP NodeId: used in public method signatures (get_neighbors, add_node, etc.)
    // REMOVE: HnswNode (internal graph structure, not needed by users)
};
// REMOVE: pub use neighbor::NeighborPool;
```

### 3.3 Semantic Versioning Policy

Post-v1.0, the following rules apply:

| Change Type | Version Bump | Examples |
|:------------|:-------------|:--------|
| **Major (breaking)** | `2.0.0` | Removing public types, changing function signatures, changing persistence format |
| **Minor (additive)** | `1.x.0` | New public types/functions, new optional parameters, new feature flags |
| **Patch (fixes)** | `1.0.x` | Bug fixes, performance improvements, documentation |

**Persistence format changes:**
- Adding optional fields: minor version bump (old data loads fine)
- Changing field layout: major version bump (old data incompatible)
- The `FileHeader` version fields (`VERSION_MAJOR`, `VERSION_MINOR`) must be incremented accordingly

**WASM ABI:**
- Adding new `#[wasm_bindgen]` functions: minor version
- Changing return types of existing functions: major version
- Adding optional parameters to existing functions: minor version (JS is lenient)

### 3.4 TypeScript Type Stability

**Rules for `.d.ts` changes:**
1. Adding optional fields to interfaces: minor version (non-breaking in TS)
2. Removing fields: major version
3. Changing field types: major version
4. Adding new exported types: minor version

**For `edgevec-langchain`:**
1. `EdgeVecStoreConfig` interface must be frozen at v1.0
2. `EdgeVecMetric` type union must be frozen (adding new metrics = minor, removing = major)
3. `EdgeVecStore` public method signatures must be frozen

### 3.5 WASM ABI Stability

**Binary compatibility considerations:**
1. postcard serialization format is NOT guaranteed stable across versions -- document this
2. IndexedDB persisted data from v0.x may not load in v1.0 if format changes
3. Provide a migration utility if format changes: `EdgeVec.migrate(name: string): Promise<void>`

**Recommendation:** Before v1.0, finalize the persistence format and commit to it. The current postcard-based serialization via `serde` is convenient but fragile -- adding or reordering fields breaks deserialization. Consider:
- [HYPOTHESIS] Adding a format version byte at the start of serialized data
- [HYPOTHESIS] Using a more stable format like flatbuffers for the WASM persistence path
- [FACT] postcard is a compact binary format that does NOT include field names; it relies on struct field order. Reordering fields in `EdgeVec`, `IndexVariant`, `HnswIndex`, `VectorStorage`, or `MetadataStore` will silently corrupt deserialization.

---

## 4. Implementation Timeline

> **Note:** This timeline starts from W46 (the earliest available implementation week). W44-W45 were consumed by research spikes (WebGPU, Relaxed SIMD, PQ literature review) and this audit.

| Week | Action | Priority | Issues Addressed |
|:-----|:-------|:---------|:-----------------|
| 46 | Add `#[deprecated]` to `HnswGraph`, `insertBatchFlat`, `softDeleteBatchCompat`, benchmark functions | P0 | R1, W5, W8, W11 |
| 46 | Standardize WASM error handling to structured `{ code, message }` objects | P0 | W7 |
| 46 | Standardize search result field naming to `{ id, distance }` across all WASM search methods | P0 | W1 |
| 46 | Rename `searchHybrid` to `searchAdvanced` (WASM) | P0 | W10 |
| 47 | Make `HnswConfig._reserved` private | P0 | R3 |
| 47 | Standardize `BinaryFlatIndex::new` to take `u32` dimensions | P0 | R12 |
| 47 | Reduce SIMD re-exports, move persistence constants out of crate root | P1 | R6, R7, R8 |
| 47 | Deprecate `set_metric(String)` in favor of `setMetricType(MetricType)` | P1 | W6 |
| 47 | Add `StorageError` to `EdgeVecError` hierarchy | P1 | R10 |
| 47 | Fix `EdgeVecStore._internal` constructor leak | P1 | L2 |
| 47 | Rename `save(directory)` to `save(name)` in LangChain adapter | P1 | L6 |
| 47 | Pin or copy `IndexConfig` type into LangChain adapter | P1 | L1 |
| 48 | Remove all deprecated items | P0 | Final cleanup |
| 48 | Finalize persistence format, add migration test | P0 | W ABI stability |
| 49 | API freeze -- tag v1.0.0-rc.1 | P0 | All |
| 50 | Release v1.0.0 | P0 | |

---

## 5. v1.0 API Freeze Criteria

The API can be frozen when ALL of the following are true:

- [ ] All P0 breaking changes from Section 1 are implemented
- [ ] All P1 breaking changes from Section 1 are implemented or deferred to v2.0 with justification
- [ ] All deprecated APIs from Section 2 are removed
- [ ] All WASM search methods return consistent `{ id, distance }` result objects
- [ ] All WASM error paths return structured `{ code, message }` objects
- [ ] `HnswGraph` type alias is removed
- [ ] `insertBatchFlat` and `softDeleteBatchCompat` are removed
- [ ] Benchmark functions are feature-gated or removed from WASM
- [ ] Persistence format has been tested with round-trip migration (v0.9 -> v1.0)
- [ ] All public types have `#[must_use]` where appropriate
- [ ] All public functions have `/// # Errors` documentation
- [ ] TypeScript `.d.ts` files have been reviewed for leaked internal types
- [ ] `cargo doc --no-deps` produces clean output with no broken links
- [ ] `clippy::missing_errors_doc` and `clippy::missing_panics_doc` are enforced (remove current `#![allow]`)
- [ ] Full test suite passes: `cargo test` + `wasm-pack test` + LangChain tests

---

## 6. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|:-----|:------------|:-------|:-----------|
| Search result field rename (`score` -> `distance`) breaks existing WASM users | HIGH | HIGH | Provide migration guide in CHANGELOG. The rename is necessary -- inconsistency now is worse than a one-time migration. |
| postcard format change between v0.9 and v1.0 breaks persisted data | MEDIUM | HIGH | Add migration utility. Test with fixtures saved from v0.9. Consider format version prefix. |
| Removing `HnswGraph` alias breaks downstream Rust users | LOW | LOW | The alias was never documented as primary API. `HnswIndex` has been canonical since v0.2. |
| `searchHybrid` rename breaks JS users | MEDIUM | MEDIUM | Keep the old name as a deprecated alias for one version. |
| LangChain adapter `IndexConfig` type drift | MEDIUM | HIGH | Copy the interface into adapter types rather than extending it. |
| Removing benchmark WASM exports breaks users who depend on them | LOW | LOW | These are clearly developer tools, not user API. No external user should depend on `benchmarkHamming`. |
| `BinaryFlatIndex::new(usize)` -> `new(u32)` breaks Rust callers | MEDIUM | LOW | Most callers already pass `u32` values and rely on implicit conversion. `as u32` may be needed in a few places. |

---

## 7. Tradeoffs Accepted

| Tradeoff | What We Gain | What We Lose | Risk Level |
|:---------|:-------------|:-------------|:-----------|
| Standardizing search result fields now | Consistent API for all v1.0+ users | Breaking change for current v0.9 WASM users | MEDIUM |
| Removing legacy batch API | Cleaner API surface, less code to maintain | Breaks users who adopted `insertBatchFlat` | LOW |
| Keeping postcard for WASM persistence | Compact binary, fast ser/de, no new dependency | Format stability risk on struct field reorder | MEDIUM |
| Not changing `HnswConfig.metric` to enum for v1.0 | Avoids persistence format break | Users still deal with magic `u32` constants | LOW (can add enum in v1.1 with backwards compat) |
| Keeping `set_metric(String)` as deprecated | Smooth migration for JS users | Two ways to do the same thing during transition | LOW |

---

## READY FOR HOSTILE REVIEW

Artifacts submitted:
- [x] `docs/audits/API_STABILITY_AUDIT.md` (this document)

Known risks:
- [R1] Search result field rename is high-impact breaking change
- [R2] postcard persistence format stability is not guaranteed across field reordering
- [R3] `searchHybrid` naming collision requires rename in one of two contexts

Open questions requiring validation:
- [Q1] Should `HnswConfig.metric` be changed to an enum in v1.0 or deferred to v1.1? The enum change would also require persistence format migration.
- [Q2] Should `Candidate`, `SearchContext`, and `Searcher` remain public for advanced users doing custom search, or should they be `pub(crate)`?
- [Q3] Is the postcard persistence format stable enough for v1.0, or should we add a version prefix byte and migration path?
- [Q4] Should we provide a `EdgeVec.migrate()` WASM function for v0.9 -> v1.0 data migration?

---

## META_ARCHITECT: Task Complete

Artifacts generated:
- `docs/audits/API_STABILITY_AUDIT.md`

Status: PENDING_HOSTILE_REVIEW

Next: Run `/review docs/audits/API_STABILITY_AUDIT.md` to validate before implementation phase.
