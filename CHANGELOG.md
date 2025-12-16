# Changelog

All notable changes to EdgeVec will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned (v0.5.0)
- ARM/NEON SIMD optimization verification
- Mobile support (iOS Safari, Android Chrome)
- Enhanced metadata storage

---

## [0.4.0] - 2025-12-20 — Documentation & Quality Sprint

**Focus:** Production readiness — comprehensive documentation, P99 tracking, and quality hardening.

### Added

#### User Documentation
- **`docs/TUTORIAL.md`** — Complete getting started guide
  - Step-by-step installation instructions
  - First index creation walkthrough
  - Browser and Node.js examples
  - Persistence tutorial

- **`docs/PERFORMANCE_TUNING.md`** — HNSW parameter optimization guide
  - M, efConstruction, ef parameter explanations
  - Tuning recommendations for different use cases
  - Memory vs. recall tradeoff guidance
  - Quantization configuration

- **`docs/TROUBLESHOOTING.md`** — Debugging guide
  - Top 10 common errors and solutions
  - WASM initialization issues
  - Dimension mismatch debugging
  - Search returning empty results

- **`docs/INTEGRATION_GUIDE.md`** — Third-party integration guide
  - transformers.js integration
  - TensorFlow.js Universal Sentence Encoder
  - OpenAI embeddings API
  - Cohere embeddings

#### Benchmark Dashboard
- **`wasm/examples/benchmark-dashboard.html`** — Interactive visualization
  - Real-time performance charts (Chart.js)
  - EdgeVec vs hnswlib-node vs voy comparison
  - Search latency, insert latency, memory charts
  - Dark/light theme toggle

- **`docs/benchmarks/PERFORMANCE_BASELINES.md`** — Baseline documentation
  - Official baseline values for regression detection
  - Target metrics for different scales
  - CI threshold configuration

#### Quality Infrastructure

- **Chaos Testing** (`tests/chaos_hnsw.rs`)
  - 15 edge case tests (11 required + 4 bonus)
  - Empty index, single vector, all deleted
  - Zero vector, max dimensions (4096)
  - Duplicate vectors, delete/reinsert
  - Extreme values, rapid cycles
  - Compaction stress, recall accuracy

- **Load Testing** (`tests/load_test.rs`)
  - 100k vector insertion stress test
  - Sustained search load (60 seconds)
  - Mixed workload (insert + search + delete)
  - High tombstone ratio validation
  - Memory stability testing
  - Batch insert performance

- **P99 Latency Tracking** (`benches/p99_bench.rs`)
  - P50/P99/P999 percentile reporting
  - 10k index latency benchmark
  - Tombstone impact benchmark
  - Scaling benchmark (1k to 25k)

- **CI Regression Detection** (`.github/workflows/regression.yml`)
  - Automatic P99 benchmark on PRs
  - 10% regression threshold enforcement
  - Performance summary in PR comments
  - Artifact upload for historical tracking

#### Release Documentation
- **`CONTRIBUTING.md`** — Contribution guidelines
  - Code of Conduct reference
  - PR process and requirements
  - Development setup instructions
  - Commit message conventions

- **`docs/RELEASE_CHECKLIST_v0.4.md`** — Release verification
  - 25+ verification items
  - Pre-release, release, post-release steps
  - Rollback procedures

- **`docs/MIGRATION.md`** — Migration from competitors
  - hnswlib migration guide
  - FAISS migration guide
  - Pinecone migration guide
  - General migration tips

### Changed
- Version bumped from 0.3.0 to 0.4.0
- Updated README.md with v0.4.0 features
- CI pipeline enhanced with P99 tracking

### Documentation
- Week 16-18 work reconciled with gate files
- ROADMAP.md updated to reflect v0.4.0 completion
- All pending gates (16, 17, 18) documented

---

## [0.3.0] - 2025-12-15 — Soft Delete Release

**Focus:** RFC-001 Soft Delete implementation — non-destructive vector deletion with compaction.

### Added

#### Soft Delete API (RFC-001)
- **`soft_delete(VectorId)`** — Mark vector as deleted in O(1) time
  - Tombstone-based deletion (vector remains in index but excluded from search)
  - Idempotent: returns `false` if already deleted
  - Error on invalid vector ID

- **`is_deleted(VectorId)`** — Check if vector is deleted
  - Returns `true` for tombstoned vectors
  - Error on invalid vector ID

- **`deleted_count()`** — Count of tombstoned vectors
- **`live_count()`** — Count of active (non-deleted) vectors
- **`tombstone_ratio()`** — Ratio of deleted to total vectors (0.0 to 1.0)

#### Compaction API
- **`compact()`** — Rebuild index removing all tombstones
  - Returns `CompactionResult` with statistics
  - Creates new index with only live vectors
  - Preserves vector IDs during rebuild
  - Warning: blocking operation for large indices

- **`needs_compaction()`** — Check if tombstone ratio exceeds threshold
- **`compaction_warning()`** — Get warning message if compaction recommended
- **`compaction_threshold()`** — Get current threshold (default: 0.3 / 30%)
- **`set_compaction_threshold(ratio)`** — Configure threshold (0.01 to 0.99)

- **`CompactionResult`** struct:
  - `tombstones_removed: u32` — Number of deleted vectors removed
  - `new_size: u32` — Index size after compaction
  - `duration_ms: f64` — Time taken in milliseconds

#### Batch Delete API
- **`batch_delete(ids)`** — Delete multiple vectors efficiently
- **WASM bindings:** `softDeleteBatch()`, `softDeleteBatchCompat()`

#### WASM Bindings (v0.3.0)
- **`softDelete(vectorId)`** — JavaScript soft delete
- **`isDeleted(vectorId)`** — Check deletion status
- **`deletedCount()` / `liveCount()`** — Statistics methods
- **`tombstoneRatio()`** — Get tombstone ratio
- **`needsCompaction()`** — Check compaction recommendation
- **`compactionWarning()`** — Get warning string or null
- **`compact()`** — Execute compaction, returns `WasmCompactionResult`
- **`compactionThreshold()` / `setCompactionThreshold()`** — Threshold management

#### Persistence Format v0.3
- `deleted_count` field in snapshot header (offset 60-63)
- `deleted` field per `HnswNode` (1 byte, was padding — zero memory overhead)
- Automatic migration from v0.2 snapshots on load
- VERSION_MINOR bumped from 2 to 3

#### Browser Examples
- **`wasm/examples/soft_delete.html`** — Interactive cyberpunk-themed demo
  - Particle effects for visual feedback
  - Real-time statistics dashboard
  - Vector grid visualization (live vs deleted)
  - Warning banner for compaction recommendation
  - Activity log with color-coded entries

- **`wasm/examples/soft_delete.js`** — Reusable JavaScript module
  - `SoftDeleteDemo` class with full API
  - Event system for insert/delete/compact/search
  - Benchmark functionality
  - Accessibility: focus indicators, ARIA labels, keyboard navigation

#### TypeScript Support
- Updated `pkg/edgevec.d.ts` with soft delete types
- `WasmCompactionResult` interface
- Full JSDoc documentation

### Changed
- Search now automatically excludes tombstoned vectors
- `HnswNode.pad` renamed to `HnswNode.deleted` (repurposed padding byte)
- Internal `adjusted_k()` calculation compensates for tombstones during search
- Snapshot version bumped to v0.3 (reads v0.2, writes v0.3)
- License changed to dual MIT OR Apache-2.0

### Fixed
- Memory leak prevention in browser demo particle system (MAX_PARTICLES cap)
- Silent error swallowing replaced with proper logging

### Migration Notes

**From v0.2.x to v0.3.0:**
1. v0.2 snapshots are automatically migrated to v0.3 on load
2. v0.3 snapshots **cannot** be read by v0.2.x (forward-incompatible)
3. Always backup your index files before upgrading
4. New soft delete methods are additive — existing code continues to work

**Breaking Changes:** None for existing API users.

---

## [0.2.1] - 2025-12-14 — Safety Hardening Release

**Focus:** Community feedback response — UB elimination and competitive positioning.

### Security

- **Fixed potential undefined behavior in persistence layer** — Replaced unsafe pointer casts with alignment-verified `bytemuck` operations. All `#[allow(clippy::cast_ptr_alignment)]` suppressions removed. Runtime alignment checks now active via `try_cast_slice`. Thanks to Reddit community feedback for identifying this issue. (W13.2)

### Added

- **Competitive Benchmark Suite** — New benchmark infrastructure for comparing EdgeVec against WASM vector libraries (hnswlib-wasm, voy, usearch-wasm, vectra). See `docs/benchmarks/competitive_analysis.md`. (W13.3)

- **Alignment Safety Tests** — 13 new tests validating Pod/Zeroable compliance and alignment safety. (W13.2)

- **Batch Insert API** (`BatchInsertable` trait)
  - Single API call for bulk vector insertion
  - Progress callback support at ~10% intervals (<1% overhead)
  - Best-effort semantics (partial success on non-fatal errors)
  - `BatchError` type with 5 error variants
  - Example: `examples/batch_insert.rs`
  - Benchmarks: `benches/batch_vs_sequential.rs`

---

## [0.2.0] - 2025-12-12 — Initial Alpha Release

**Focus:** First public alpha — core HNSW engine with WASM support.

### Added

#### Core Engine
- **HNSW Indexing Engine**
  - O(log n) approximate nearest neighbor search
  - Configurable `m` (connections per node, default: 16)
  - Configurable `ef_construction` (build quality, default: 200)
  - Layer-based graph structure with probabilistic level assignment
  - Efficient neighbor selection with heuristic pruning

- **Distance Metrics**
  - L2 (Euclidean distance) — default metric
  - Cosine similarity — normalized vectors
  - Dot product (inner product) — unnormalized similarity

- **Scalar Quantization (SQ8)**
  - 8-bit scalar quantization for 3.6x memory reduction
  - Configurable min/max range for precision tuning
  - AVX2 SIMD-optimized distance calculations
  - Maintains competitive recall at k=10

#### Persistence Layer
- **Write-Ahead Log (WAL)**
  - Append-only log for crash recovery
  - Automatic replay on startup
  - Configurable sync interval

- **Atomic Snapshots**
  - Safe background saves without blocking reads
  - Magic number + version + checksum validation
  - Compatible format between native and WASM builds

- **Storage Backends**
  - `FileBackend` — Native file system persistence
  - `IndexedDbBackend` — Browser IndexedDB storage
  - `MemoryBackend` — In-memory for testing

#### WASM Support
- **First-class WebAssembly support** via `wasm-pack`
- **Browser-native** ES module exports
- **Node.js compatibility** via CommonJS wrapper
- **IndexedDB integration** for browser persistence
- **148 KB gzipped bundle** (70% under 500KB target)

#### TypeScript Wrapper
- `EdgeVecClient` class with auto WASM initialization
- `EdgeVecConfigBuilder` for fluent configuration
- Promise-based async API for persistence operations
- Full TypeScript type definitions (`.d.ts`)
- Comprehensive JSDoc documentation

### Performance

Benchmarked on AMD Ryzen 7 5700U, 16GB RAM, Windows 11, Rust 1.94.0-nightly.
Criterion 0.5.x with 10 samples per configuration. `-C target-cpu=native` enabled.

#### Search Latency (768-dimensional vectors, k=10)

| Scale | Float32 | Quantized (SQ8) | Target | Status |
|:------|:--------|:----------------|:-------|:-------|
| 10k vectors | 203 µs | **88 µs** | <1 ms | 11x under target |
| 50k vectors | 480 µs | **167 µs** | <1 ms | 6x under target |
| 100k vectors | 572 µs | **329 µs** | <1 ms | 3x under target |

#### Memory Efficiency

| Mode | Per Vector | 100k Vectors | 1M Projection | Target |
|:-----|:-----------|:-------------|:--------------|:-------|
| Float32 | 3,176 bytes | 303 MB | 3.03 GB | N/A |
| Quantized (SQ8) | 872 bytes | 83 MB | **832 MB** | <1 GB (17% under) |

#### Bundle Size

| Package | Size (Gzipped) | Target | Status |
|:--------|:---------------|:-------|:-------|
| `@edgevec/core` | **213 KB** | <500 KB | 57% under target |

---

## [0.1.0] - 2025-12-05 — Genesis Release (Internal)

**Focus:** Initial architecture validation and core infrastructure.

### Added
- Project structure and Cargo configuration
- HNSW algorithm prototype
- Distance metric implementations
- Basic persistence framework
- Architecture documentation (ARCHITECTURE.md, DATA_LAYOUT.md)
- Testing infrastructure (proptest, criterion)
- CI/CD pipeline

### Notes
This version was internal only, not published to crates.io or npm.

---

## Version Comparison

| Version | Date | Highlights |
|:--------|:-----|:-----------|
| 0.4.0 | 2025-12-20 | Documentation sprint, P99 tracking, chaos testing |
| 0.3.0 | 2025-12-15 | Soft delete API, compaction, dual-license |
| 0.2.1 | 2025-12-14 | Safety hardening, batch insert |
| 0.2.0 | 2025-12-12 | Initial alpha release |
| 0.1.0 | 2025-12-05 | Internal genesis release |

---

## Links

- [GitHub Repository](https://github.com/matte1782/edgevec)
- [Documentation](docs/)
- [Performance Guide](docs/PERFORMANCE_GUIDE.md)
- [Tutorial](docs/TUTORIAL.md)
- [API Reference](docs/API_REFERENCE.md)

---

[Unreleased]: https://github.com/matte1782/edgevec/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/matte1782/edgevec/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/matte1782/edgevec/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/matte1782/edgevec/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/matte1782/edgevec/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/matte1782/edgevec/releases/tag/v0.1.0
