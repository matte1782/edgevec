# Changelog

All notable changes to EdgeVec will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

---

## [0.9.0] - 2026-03-07 — Sparse Vectors, Hybrid Search, FlatIndex, BinaryFlatIndex

### Added (v0.9.0) — Sparse Vectors (RFC-007)

**Sparse Vector Storage** — Weeks 36-37:
- **`SparseVector`** struct (CSR format: indices, values, dim)
- **`SparseStorage`** with inverted index for fast search
- **`SparseSearcher`** with dot product similarity
- Insert, search, delete, batch operations
- WASM bindings: `initSparseStorage()`, `insertSparse()`, `searchSparse()`
- TypeScript types: `SparseVector`, `SparseSearchResult`

### Added (v0.9.0) — RRF Hybrid Search (RFC-007)

**Hybrid Search Engine** — Weeks 38-39:
- **`HybridSearchEngine`** combining dense (HNSW) + sparse search
- Reciprocal Rank Fusion (RRF) with configurable k parameter
- Linear fusion with alpha parameter
- **`HybridSearchResult`** with per-source rank/score tracking
- WASM binding: `hybridSearch()`
- TypeScript types: `HybridSearchOptions`, `HybridSearchResult`, `FusionMethod`

### Added (v0.9.0) — FlatIndex Implementation

**FlatIndex** (RFC from @jsonMartin) — Week 40:

#### Core (Days 1-2)
- **`FlatIndex`** — Brute-force exact nearest neighbor search
  - O(1) insert, O(n·d) search with 100% recall guarantee
  - Row-major vector storage for cache-friendly access
  - 4 distance metrics: Cosine, DotProduct, L2, Hamming
  - Configurable via `FlatIndexConfig` builder pattern

```rust
use edgevec::{FlatIndex, FlatIndexConfig, DistanceMetric};

let config = FlatIndexConfig::new(768)
    .with_metric(DistanceMetric::Cosine)
    .with_capacity(10_000);
let mut index = FlatIndex::new(config);

let id = index.insert(&embedding)?;
let results = index.search(&query, 10)?;
```

#### Deletion & Compaction (Day 3)
- **Soft delete** with bitmap tracking
- **Auto-compaction** when deletion ratio exceeds threshold
- **`deletion_stats()`** for monitoring

#### Binary Quantization (Day 3)
- **32x memory reduction** (768D: 3072 → 96 bytes per vector)
- **`enable_quantization()`** / **`disable_quantization()`**
- **`search_quantized()`** with Hamming distance
- Recall: ~40% on random data, 70-90% on real embeddings

#### Persistence (Day 4)
- **`to_snapshot()`** / **`from_snapshot()`** serialization
- **CRC32 checksum** for integrity validation
- **Postcard serialization** (WASM-compatible)
- Magic number "EVFI", version 1

```rust
// Save
let snapshot = index.to_snapshot()?;
storage.write("index.bin", &snapshot)?;

// Load
let data = storage.read("index.bin")?;
let restored = FlatIndex::from_snapshot(&data)?;
```

#### Benchmarks
- `benches/flat_bench.rs` with 6 benchmark groups
- Insert, search (128D/768D), BQ comparison, metrics, snapshot

**Test Coverage:** 77 FlatIndex tests, 988 total library tests

---

### Added (v0.9.0) — BinaryFlatIndex (PR #7 by @jsonMartin)

**Native Binary Vector Storage** — 2026-02-02:

#### BinaryFlatIndex
- **`BinaryFlatIndex`** — Optimized for binary vectors with O(1) insert, O(n) SIMD search
  - Native packed binary storage (8 bits per byte)
  - Hamming and Jaccard distance metrics
  - 32x memory reduction vs f32 (768D: 3072 → 96 bytes)
  - ~1μs insert latency (vs ~2ms for HNSW)

```rust
use edgevec::BinaryFlatIndex;

let mut index = BinaryFlatIndex::new(768);
let id = index.insert(&binary_vector)?;
let results = index.search(&query, 10)?;
```

#### HNSW Binary Methods
- **`insert_binary()`** — Insert raw binary vectors into HNSW
- **`search_binary()`** — Search with raw binary queries
- **`search_binary_with_ef()`** — Search with custom ef parameter

#### WASM Integration
- **`JsIndexType`** enum — Runtime selection between Flat and HNSW
- **`VectorType`** enum — Float32 or Binary
- **`insertBinary()`** / **`searchBinary()`** JavaScript methods
- Auto-conversion from f32 to binary via sign-bit quantization

#### Storage
- **`StorageType::Binary(u32)`** — Native binary storage variant
- Full persistence support (snapshot save/load)
- Soft delete and compaction support

**Use Cases:**
- Semantic caching (insert-heavy, exact recall required)
- Datasets < 100K vectors
- When insert latency is critical (~1μs vs ~2ms for HNSW)

**Test Coverage:** 20+ new tests, 1019 total library tests

---

### Fixed

#### CI Workflow (2026-02-02)
- **Fork PR comment permissions** — Added `continue-on-error: true` to PR comment steps
  - Prevents workflow failure when fork PRs lack write permission (HTTP 403)
  - Benchmarks and regression checks still run and report correctly

---

## [0.8.0] - 2026-02-02 — Consolidation + Developer Experience

**Focus:** Developer experience improvements, framework integrations, and technical debt reduction.

### Added

#### Vue 3 Composables (Week 34)
- **`useEdgeVec`** — Reactive database initialization with loading states
  - Async initialization with `isLoading`, `error`, `isReady` refs
  - Full TypeScript support with `MaybeRef`/`MaybeRefOrGetter`
  - Automatic cleanup on component unmount

- **`useSearch`** — Reactive search with debouncing
  - Configurable debounce (default 300ms)
  - Reactive result updates
  - Works with Vue's reactivity system

```typescript
import { useEdgeVec, useSearch } from 'edgevec/vue';

const { db, isLoading, isReady } = useEdgeVec({ dimensions: 384 });
const { results, search, isSearching } = useSearch(db);
```

#### Standalone Filter Functions (Week 34)
- Export filter functions directly from main package:
  - Comparison: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
  - String: `contains`, `startsWith`, `endsWith`
  - Logical: `and`, `or`, `not`, `all`, `any`

```typescript
import { eq, gt, and, contains } from 'edgevec';

const filter = and(
  eq('category', 'electronics'),
  gt('price', 100)
);
```

#### Documentation (Weeks 33-35)
- **Filter Examples Guide** — 25 real-world filter examples
  - E-commerce, document management, user profiles
  - Complex nested filters, date ranges, arrays

- **Embedding Integration Guide** — 5 provider integrations
  - Ollama (local), Transformers.js (browser)
  - OpenAI, Cohere, HuggingFace Inference API

- **EdgeVec vs pgvector Comparison** — Architecture and use case guide
  - Feature comparison tables
  - When to choose each solution
  - Migration considerations

#### SIMD Optimizations (Week 32)
- **Euclidean distance SIMD acceleration**
  - Consolidated SIMD dispatch system
  - Unified architecture across all distance metrics

### Fixed

#### Technical Debt (Week 35)
- **WAL chunk_size edge case** — Added `MIN_CHUNK_SIZE` constant (64 bytes)
  - Prevents header split across chunks
  - 5 new edge case tests

- **Safety documentation** — Proper `#[doc]` placement for `unsafe` blocks
  - All 62 SAFETY comments verified compliant

- **Cast truncation warnings** — Resolved 65+ `cast_possible_truncation` warnings
  - All casts documented with `#[allow]` justifications
  - Module-level documentation in graph.rs, neighbor.rs, search_bq.rs

- **Test clippy warnings** — Clean test and bench code
  - Fixed binary literal formatting
  - Inlined format arguments
  - Zero warnings in `cargo clippy --tests --benches`

### Changed

- Improved TypeScript type exports for Vue composables
- Consistent high-level API documentation across all guides

### Documentation

- `docs/guides/FILTER_EXAMPLES.md` — 25 filter examples
- `docs/guides/EMBEDDING_GUIDE.md` — 5 embedding providers
- `docs/guides/COMPARISON_PGVECTOR.md` — Architecture comparison
- `pkg/vue/README.md` — Vue composables documentation

### Performance

All v0.7.0 performance targets maintained:
| Metric | Target | Achieved |
|:-------|:-------|:---------|
| Search (100K, 768D) | <10ms | 4.2ms |
| Insert (single) | <5ms | 0.8ms |
| WASM bundle | <500KB | 477KB |

---

## [0.7.0] - 2025-12-27 — SIMD Acceleration + First Community Contribution

**Focus:** Performance optimization via SIMD and celebrating our first external contributor!

### Added

#### Community Contribution 🎉

- **WASM SIMD128 Hamming Distance** — 8.75x faster binary distance calculations
  - LUT-based popcount algorithm (Warren, "Hacker's Delight", 2nd ed.)
  - Comprehensive test coverage (10 tests including edge cases)
  - Thanks to **[@jsonMartin](https://github.com/jsonMartin)** for this excellent first contribution!

- **AVX2 Native Hamming Distance** — Native popcount for x86_64
  - 4-way ILP optimization with separate accumulators
  - Also contributed by **@jsonMartin**

#### WASM SIMD Acceleration
- **SIMD128 enabled by default** — 2x+ faster vector operations on modern browsers
  - Dot product, L2 distance, cosine similarity accelerated
  - Automatic scalar fallback for iOS Safari (no SIMD support)
  - Enabled via `-C target-feature=+simd128` build flag

- **Performance improvements:**
  | Dimension | Speedup | Notes |
  |:----------|:--------|:------|
  | 128D | 2.3x | 55ns dot product |
  | 768D | 2.1x | 374ns dot product |
  | 1536D | 2.0x | 761ns dot product |
  | Hamming | **8.75x** | 40ns (768-bit) — @jsonMartin |

- **Browser compatibility:**
  | Browser | SIMD | Status |
  |:--------|:-----|:-------|
  | Chrome 91+ | ✅ | Full speed |
  | Firefox 89+ | ✅ | Full speed |
  | Safari 16.4+ (macOS) | ✅ | Full speed |
  | Edge 91+ | ✅ | Full speed |
  | iOS Safari | ❌ | Scalar fallback (~2x slower) |

#### Interactive Filter Playground
- **[Filter Playground](https://matte1782.github.io/edgevec/demo/)** — Interactive filter expression builder
  - Visual filter construction with AND/OR/clause controls
  - 10 ready-to-use examples (e-commerce, documents, users, etc.)
  - Live WASM execution sandbox
  - Copy-paste code snippets (JavaScript, TypeScript, React)
  - Operator reference panel

#### API Additions
- **`enableBQ()`** — Enable binary quantization after index creation
  - Required for BQ search methods (`searchBQ`, `searchBQRescored`)
  - Dimensions must be divisible by 8
  - Automatically encodes existing vectors on enable

### Changed

- **WASM Bundle Optimized** — Applied wasm-opt `-Oz` with `--strip-debug` and `--strip-producers`
  - **524 KB → 477 KB** (9.2% reduction, 47 KB saved)
  - Gzipped: 217 KB (unchanged — gzip already compresses efficiently)
  - Optimization flags: `--enable-bulk-memory --enable-nontrapping-float-to-int`

- **Build configuration** — SIMD enabled by default in `.cargo/config.toml`

### Fixed

- **AVX2 popcount optimization** — Native `popcnt` instruction replaces lookup table
  - Feedback from Reddit user chillfish8: extract 4×u64, use hardware popcnt
  - ~15% faster Hamming distance on x86_64

- **Code cleanup** — Removed internal monologue comments from chunking.rs
  - Professional comment style throughout codebase

- **Safety documentation** — Moved SAFETY docs to function-level per Rust conventions
  - `# Safety` sections on `#[target_feature]` functions

### Documentation

- **README.md** — Added "Try It Now" section with playground link
- **docs/api/FILTER_SYNTAX.md** — Added interactive playground link
- **docs/benchmarks/2025-12-24_simd_benchmark.md** — Full SIMD benchmark report

### Performance (v0.7.0 Targets)

| Metric | Result | Target | Status |
|:-------|:-------|:-------|:-------|
| SIMD speedup | 2x+ | 2x | ✅ Achieved |
| Search 10k (768D) | 938 µs | <1 ms | ✅ Achieved |
| Bundle size | 477 KB | <500 KB | ✅ Achieved |
| iOS fallback | Works | Functional | ✅ Achieved |

---

## [0.6.0] - 2025-12-22 — RFC-002: Binary Quantization + Metadata Storage

**Focus:** RFC-002 Implementation — Binary Quantization for 32x memory savings and integrated metadata storage.

### Added

#### Binary Quantization (RFC-002 Phase 2)

- **`searchBQ(query, k)`** — Fast binary search using Hamming distance
  - 3-5x faster than F32 search
  - 32x memory reduction (768D: 3072 bytes → 96 bytes)
  - SIMD-optimized popcount (AVX2/SSE on x86, NEON on ARM)

- **`searchBQRescored(query, k, rescoreFactor)`** — High-recall hybrid search
  - BQ candidate retrieval + F32 rescoring
  - >0.90 recall@10 with rescoreFactor=15 (RFC-002 target achieved)
  - Factor 5: ~95% recall, 2.5x faster
  - Factor 10: ~98% recall, 2x faster

- **`HnswIndex::with_bq(config, storage)`** — Create BQ-enabled index
- **`insert_bq(vector, storage)`** — Insert with automatic BQ encoding
- **`has_bq()`** — Check if BQ is enabled

#### Metadata Storage (RFC-002 Phase 1)

- **`insertWithMetadata(vector, metadata)`** — Insert vectors with key-value metadata
  - Supports: String, Integer, Float, Boolean, StringArray
  - Automatic cleanup on soft-delete

- **`searchFiltered(query, filter, k)`** — Search with metadata filter expressions
  - Comparison: `=`, `!=`, `>`, `>=`, `<`, `<=`
  - Logical: `AND`, `OR`, `NOT`
  - Array membership: `ANY ["value1", "value2"]`
  - Grouping with parentheses

- **`getMetadata(id)`** — Retrieve metadata for a vector
- **`MetadataStore`** — Core Rust metadata storage with HashMap-based indexing

#### Memory Management (RFC-002 Phase 3)

- **`getMemoryPressure()`** — Monitor WASM heap usage
  - Returns: level (normal/warning/critical), usedBytes, totalBytes, usagePercent

- **`setMemoryConfig(config)`** — Configure thresholds
  - warning_threshold: default 70%
  - critical_threshold: default 90%
  - block_inserts_at_critical: default true

- **`canInsert()`** — Check if inserts allowed (respects memory pressure)
- **`getMemoryRecommendation()`** — Actionable memory management guidance
- **Allocation tracking** — Track memory usage per insert operation

#### WASM Bindings (RFC-002 Phase 3)

- Complete TypeScript type definitions for all new APIs
- **`EdgeVec`** class with full BQ + metadata + memory pressure support
- **`validateFilter()`** — Validate filter expression syntax

#### Integration Tests

- **`tests/hybrid_search.rs`** — 5 tests for BQ + filter search
  - Basic hybrid search, complex filters, array ANY operator
  - Fallback when BQ disabled, recall validation

- **`tests/bq_persistence.rs`** — 7 tests for BQ index persistence
  - Save/load roundtrip, F32 search after load
  - Metadata preservation, BQ state documentation

- **`tests/bq_recall_roundtrip.rs`** — 7 tests for BQ recall validation
  - RFC-002 target validation (>0.90 recall)
  - High-recall mode testing

#### Browser Demo

- **`wasm/examples/v060_cyberpunk_demo.html`** — Interactive v0.6.0 showcase
  - Cyberpunk-themed UI matching previous demos
  - BQ vs F32 performance comparison with visual bars
  - Metadata filter tags with preset expressions
  - Memory pressure monitoring with live updates
  - Recall metrics display

### Changed

- Filter syntax: `=` operator (not `==`), `ANY ["value"]` for array membership
- Persistence format: v0.4 with metadata section (Postcard serialization)
- BQ not persisted: regenerated from F32 vectors on load (expected behavior)

### Performance (RFC-002 Targets)

| Metric | Result | Target | Status |
|:-------|:-------|:-------|:-------|
| BQ memory reduction | 32x | 8-32x | ✅ Achieved |
| SIMD popcount speedup | 6.9x vs scalar | >5x | ✅ Achieved |
| BQ search speedup | 3-5x vs F32 | 2-5x | ✅ Achieved |
| BQ+rescore recall@10 | 0.936 | >0.90 | ✅ Achieved |
| Filter evaluation | <1μs/vector | <10μs | ✅ Achieved |

### Migration Guide

#### From v0.5.x

v0.6.0 is backward compatible with v0.5.x snapshots:

```javascript
// v0.5.x snapshots load automatically
import { EdgeVec } from 'edgevec';

const index = new EdgeVec({ dimensions: 768 });
index.loadSnapshot(v05Snapshot);  // Auto-migrates

// New features available immediately
index.insertWithMetadata(vector, { category: 'news' });
const results = index.searchFiltered(query, 'category = "news"', 10);
```

#### Enabling Binary Quantization

```javascript
const index = new EdgeVec({ dimensions: 768 });

// Insert vectors (BQ auto-enabled for dimension divisible by 8)
index.insertWithMetadata(vector, { category: 'tech' });

// Fast BQ search with rescoring (~95% recall, 3x faster)
const results = index.searchBQ(query, 10);
```

#### Filter Syntax (Important Changes)

```javascript
// Correct v0.6.0 syntax
index.searchFiltered(query, 'category = "news"', 10);          // = not ==
index.searchFiltered(query, 'tags ANY ["featured"]', 10);      // ANY for arrays
index.searchFiltered(query, 'score > 0.5 AND active = true', 10);
```

---

## [0.5.4] - 2025-12-20 — iOS Safari Compatibility

**Focus:** Mobile browser support — EdgeVec now works correctly on iOS Safari.

### Fixed

#### iOS Safari WASM Compatibility
- **`parse_filter_js is not a function` error** — Stale `wasm/pkg/` directory was shadowing the correct `pkg/` directory, causing old WASM bindings (without filter functions) to load
  - Deleted stale `wasm/pkg/` directory
  - Updated import paths to use only correct paths

- **Browser caching old WASM modules** — ES module caching was serving stale versions even after rebuilds
  - Added cache-busting query parameter `?v=${Date.now()}` to all WASM imports
  - Ensures fresh module loads after each rebuild

- **iOS Safari showing 0ms benchmark timings** — Safari limits `performance.now()` to 1ms resolution (Spectre mitigation)
  - Changed from per-iteration timing to batch timing (50 iterations averaged)
  - Now shows accurate sub-millisecond timings on iOS

- **NaN% filter overhead on iOS** — Division by zero when unfiltered time was 0ms
  - Added null check: display "0.0%" instead of "NaN%"

### Added

- **Embedding Integration Guide** (`docs/guides/EMBEDDING_GUIDE.md`) — Complete guide for generating embeddings with EdgeVec
  - Transformers.js browser-native examples (MiniLM, BGE, Nomic)
  - API examples: OpenAI, Cohere, HuggingFace
  - Web Worker pattern for non-blocking embedding
  - Model caching and batching best practices
  - Complete example applications (Semantic Notes, FAQ Bot, Image Search with CLIP)
  - Troubleshooting guide and model comparison table

### Changed
- Filter Playground: Removed stale import paths, added cache buster
- Benchmark Dashboard: Batch timing for iOS precision, added cache buster

### Verified Platforms
- Desktop Chrome ✓
- Desktop Firefox ✓
- Desktop Safari ✓
- iOS Safari (iPhone) ✓
- iOS Safari (iPad) ✓

---

## [0.5.3] - 2025-12-19 — crates.io Publishing Fix

**Type:** Release engineering fix

### Fixed

- **crates.io 413 Payload Too Large** — Package was 28.0 MiB (11.0 MiB compressed), exceeding crates.io's 10 MiB limit
  - Added `exclude` patterns to Cargo.toml to strip internal development files
  - Excluded: `docs/`, `tests/`, `.claude/`, `.cursor/`, `.github/`, `benches/competitive/`, `scripts/`
  - **New size: 1.7 MiB (358 KiB compressed)** — 96% reduction

### Changed
- Version sync: Cargo.toml and pkg/package.json both at 0.5.3
- Previous versions: crates.io was stuck at v0.4.0, npm was at v0.5.2

### Note
This release enables crates.io publishing that was blocked since v0.5.0.

---

## [0.5.2] - 2025-12-19 — npm TypeScript Compilation Fix

**Type:** Hotfix

### Fixed

- **npm package missing compiled JavaScript** — v0.5.0/v0.5.1 only included TypeScript source files (`.ts`), not compiled JavaScript (`.js`)
  - Users with bundlers that don't handle TypeScript got import errors
  - Added compiled `.js` and `.d.ts` files to pkg/

### Added
- Week 25 Day 1 metrics documentation
- Enterprise-grade hostile audit review

---

## [0.5.1] - 2025-12-19 — README Update

**Type:** Documentation patch

### Changed
- Updated `pkg/README.md` with v0.5.0 Filter API content for npm display
- Tagline: "The first WASM-native vector database"
- Added Filter API Quick Start example with `Filter.parse()`
- Updated version references from v0.4.0 to v0.5.0

### Note
No code changes. This release ensures npm displays the correct v0.5.0 documentation.

---

## [0.5.0] - 2025-12-19 — Filter API Release

**Focus:** Metadata filtering — The feature that transforms EdgeVec from a search library into a vector database.

### Added

#### Filter API
- **SQL-like filter expressions** — 15 operators for metadata filtering
  - Comparison: `=`, `!=`, `>`, `<`, `>=`, `<=`
  - Set: `IN`, `NOT IN`
  - String: `CONTAINS`, `STARTS_WITH`, `ENDS_WITH`
  - Null: `IS NULL`, `IS NOT NULL`
  - Boolean: `AND`, `OR`, `NOT`
- **`Filter.parse()`** — Parse filter expressions with detailed error messages
- **`Filter.evaluate()`** — Evaluate filters against metadata objects
- **`FilterBuilder`** — TypeScript fluent API for type-safe filter construction
- **Strategy selection** — Automatic prefilter/postfilter/hybrid selection

#### Interactive Demos
- **Filter Playground** (`wasm/examples/filter-playground.html`)
  - Real-time filter parsing with syntax highlighting
  - AST visualization
  - Example expressions gallery
  - Dark/light theme toggle
  - Keyboard shortcuts (Ctrl+Enter, Ctrl+/)

- **Demo Catalog** (`wasm/examples/index.html`)
  - Professional landing page with all demos
  - Mobile responsive design
  - Filter integration across all demos

#### Documentation
- **`docs/api/FILTER_SYNTAX.md`** — Complete filter expression reference
- **`docs/api/DATABASE_OPERATIONS.md`** — CRUD operations guide
- **`docs/api/TYPESCRIPT_API.md`** — TypeScript API reference
- **`docs/COMPARISON.md`** — EdgeVec vs alternatives guide
- **`docs/design/ACCESSIBILITY_AUDIT.md`** — WCAG 2.1 AA compliance

#### Competitive Analysis
- **`docs/benchmarks/competitive_analysis_v2.md`** — Full methodology
- **`docs/benchmarks/w24_voy_comparison.md`** — EdgeVec vs voy (24x faster)
- **`docs/benchmarks/w24_hnswlib_comparison.md`** — EdgeVec vs hnswlib-node
- **`docs/benchmarks/w24_tier2_feature_matrix.md`** — Feature comparison

### Changed
- **README.md** — Repositioned as "vector database" with feature matrix
- **`pkg/package.json`** — 16 keywords for npm discoverability
- **All demos** — Added filter capabilities and mobile responsiveness

### Fixed
- **UTF-8 panic** — Filter parser now handles multi-byte UTF-8 correctly (`f75a4c0`)
- **XSS vulnerabilities** — Added `escapeHtml()` to all demos (`359cd7d`, `d60770c`)

### Security
- All user input in demos escaped via `escapeHtml()`
- Filter parser fuzz tested for 24+ hours (14.4B executions, 0 crashes)

### Performance
| Metric | Result | Target |
|:-------|:-------|:-------|
| Search P50 (10k) | 0.20 ms | <1 ms |
| Bundle (gzip) | 262 KB | <500 KB |
| Fuzz testing | 24h+ | 0 crashes |

---

## [0.4.1] - 2025-12-17 — Hotfix: NPM Package Fix

**Type:** HOTFIX (Critical Bug Fix)

### Fixed

- **NPM package missing `snippets` directory** — Build failures with Vite, webpack, and other bundlers due to missing `snippets` directory in published npm package. The `package.json` `files` array now correctly includes `"snippets"`. ([GitHub Issue #1](https://github.com/matte1782/edgevec/issues/1))

### Upgrade

```bash
npm install edgevec@0.4.1
```

### Affected Versions

| Version | Status |
|:--------|:-------|
| 0.4.0 | ❌ BROKEN (do not use with bundlers) |
| 0.4.1 | ✅ FIXED |

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
| 0.9.0 | 2026-03-07 | **FlatIndex**, **BinaryFlatIndex** (PR #7), **Sparse Vectors** (RFC-007), **RRF Hybrid Search** |
| 0.8.0 | 2026-02-02 | **Vue 3 Composables**, **Filter Functions**, **SIMD Euclidean**, Tech Debt |
| 0.7.0 | 2025-12-27 | **SIMD Acceleration** (2x+), **First Community Contribution** (@jsonMartin — 8.75x Hamming) |
| 0.6.0 | 2025-12-22 | **RFC-002:** Binary Quantization (32x memory), Metadata Storage, Memory Pressure |
| 0.5.4 | 2025-12-20 | iOS Safari compatibility fixes |
| 0.5.3 | 2025-12-19 | **FIX:** crates.io publishing (package size reduction) |
| 0.5.2 | 2025-12-19 | **FIX:** npm TypeScript compilation |
| 0.5.1 | 2025-12-19 | README update for npm display |
| 0.5.0 | 2025-12-19 | **Filter API:** 15 SQL-like operators, Filter Playground |
| 0.4.1 | 2025-12-17 | **HOTFIX:** NPM package snippets fix |
| 0.4.0 | 2025-12-16 | Documentation sprint, P99 tracking, chaos testing |
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

[Unreleased]: https://github.com/matte1782/edgevec/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/matte1782/edgevec/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/matte1782/edgevec/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/matte1782/edgevec/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/matte1782/edgevec/compare/v0.5.4...v0.6.0
[0.5.4]: https://github.com/matte1782/edgevec/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/matte1782/edgevec/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/matte1782/edgevec/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/matte1782/edgevec/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/matte1782/edgevec/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/matte1782/edgevec/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/matte1782/edgevec/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/matte1782/edgevec/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/matte1782/edgevec/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/matte1782/edgevec/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/matte1782/edgevec/releases/tag/v0.1.0
