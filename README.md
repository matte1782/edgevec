# 🚀 `EdgeVec`

[![CI](https://github.com/matte1782/edgevec/actions/workflows/ci.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/ci.yml)
[![Performance](https://github.com/matte1782/edgevec/actions/workflows/benchmark.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/benchmark.yml)
[![Crates.io](https://img.shields.io/crates/v/edgevec.svg)](https://crates.io/crates/edgevec)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/matte1782/edgevec/blob/main/LICENSE-MIT)

**High-performance vector search for Browser, Node, and Edge**

> ✅ **STATUS: Alpha Release Ready** — All performance targets exceeded.

---

## What's New in v0.3.0

### Soft Delete API (RFC-001)
- **`soft_delete(id)`** — O(1) tombstone-based deletion
- **`is_deleted(id)`** — Check deletion status
- **`deleted_count()` / `live_count()`** — Vector statistics
- **`tombstone_ratio()`** — Monitor index health

### Compaction API
- **`compact()`** — Rebuild index removing all tombstones
- **`needs_compaction()`** — Check if compaction recommended
- **`compaction_warning()`** — Get actionable warning message
- Configurable threshold (default: 30% tombstones)

### WASM Bindings
- Full soft delete API exposed to JavaScript/TypeScript
- `softDelete()`, `isDeleted()`, `deletedCount()`, `liveCount()`
- `compact()`, `needsCompaction()`, `compactionWarning()`
- Interactive browser demo at `/wasm/examples/soft_delete.html`

### Persistence Format v0.3
- Automatic migration from v0.2 snapshots
- Tombstone state preserved across save/load cycles

### Previous (v0.2.1)
- Safety hardening with `bytemuck` for alignment-verified operations
- Batch insert API with progress callback
- 24x faster search than voy (fastest pure-WASM competitor)

---

## What is `EdgeVec`?

`EdgeVec` is an embedded vector database built in Rust with first-class WASM support. It's designed to run anywhere: browsers, Node.js, mobile apps, and edge devices.

### Key Features

- **Sub-millisecond search** — 0.23ms at 100k vectors (768d, quantized)
- **HNSW Indexing** — O(log n) approximate nearest neighbor search
- **Scalar Quantization (SQ8)** — 3.6x memory compression
- **WASM-First** — Native browser support via WebAssembly
- **Persistent Storage** — `IndexedDB` in browser, file system elsewhere
- **Minimal Dependencies** — No C compiler required, WASM-ready
- **Tiny Bundle** — 213 KB gzipped (57% under 500KB target)

---

## Quick Start

### Installation

```bash
npm install edgevec
```

**For Rust users:** To achieve optimal performance, ensure your `.cargo/config.toml` includes:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

Without this configuration, performance will be 60-78% slower due to missing SIMD optimizations.

### Browser/Node.js Usage

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    // 1. Initialize WASM (required once)
    await init();

    // 2. Create Config and Index
    const config = new EdgeVecConfig(128);  // 128 dimensions
    config.metric = 'cosine';  // Optional: 'l2', 'cosine', or 'dot'
    const index = new EdgeVec(config);

    // 3. Insert Vectors
    const vector = new Float32Array(128).fill(0.1);
    const id = index.insert(vector);
    console.log(`Inserted vector with ID: ${id}`);

    // 4. Search
    const query = new Float32Array(128).fill(0.1);
    const results = index.search(query, 10);
    console.log("Results:", results);
    // Results: [{ id: 0, score: 0.0 }, ...]

    // 5. Save to IndexedDB (browser) or file system
    await index.save("my-vector-db");
}

main().catch(console.error);
```

### Load Existing Index

```javascript
import init, { EdgeVec } from 'edgevec';

await init();
const index = await EdgeVec.load("my-vector-db");
const results = index.search(queryVector, 10);
```

### Rust Usage

```rust,no_run
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use edgevec::persistence::{write_snapshot, MemoryBackend};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create Config & Storage
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);

    // 2. Create Index
    let mut index = HnswIndex::new(config, &storage)?;

    // 3. Insert Vectors
    let vec1 = vec![1.0; 128];
    let _id1 = index.insert(&vec1, &mut storage)?;

    // 4. Search
    let query = vec![1.0; 128];
    let results = index.search(&query, 10, &storage)?;
    println!("Found {} results", results.len());

    // 5. Save Snapshot
    let mut backend = MemoryBackend::new();
    write_snapshot(&index, &storage, &mut backend)?;

    Ok(())
}
```

### Batch Insert (Rust)

For inserting many vectors efficiently, use the batch insert API:

```rust,no_run
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use edgevec::batch::BatchInsertable;
use edgevec::error::BatchError;

fn main() -> Result<(), BatchError> {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    // Prepare vectors as (id, data) tuples
    let vectors: Vec<(u64, Vec<f32>)> = (1..=1000)
        .map(|i| (i as u64, vec![i as f32; 128]))
        .collect();

    // Batch insert with progress tracking
    let ids = index.batch_insert(vectors, &mut storage, Some(|inserted, total| {
        println!("Progress: {}/{}", inserted, total);
    }))?;

    println!("Inserted {} vectors", ids.len());
    Ok(())
}
```

**Features:** Progress tracking, best-effort semantics, and unified error handling.

### Soft Delete (Rust)

Delete vectors without rebuilding the index (v0.3.0+):

```rust,no_run
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage)?;

    // Insert a vector
    let vector = vec![1.0; 128];
    let id = index.insert(&vector, &mut storage)?;

    // Soft delete (O(1) operation)
    let was_deleted = index.soft_delete(id)?;
    println!("Deleted: {}", was_deleted);

    // Check deletion status
    println!("Is deleted: {}", index.is_deleted(id)?);

    // Get statistics
    println!("Live: {}, Deleted: {}", index.live_count(), index.deleted_count());
    println!("Tombstone ratio: {:.1}%", index.tombstone_ratio() * 100.0);

    // Compact when tombstones accumulate (rebuilds index)
    if index.needs_compaction() {
        let (new_index, new_storage, result) = index.compact(&mut storage)?;
        println!("Removed {} tombstones", result.tombstones_removed);
        // Use new_index and new_storage for future operations
    }

    Ok(())
}
```

### Soft Delete (JavaScript)

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);

// Insert vectors
const vector = new Float32Array(128).fill(0.5);
const id = index.insert(vector);

// Soft delete
const wasDeleted = index.softDelete(id);
console.log('Deleted:', wasDeleted);

// Statistics
console.log('Live:', index.liveCount());
console.log('Deleted:', index.deletedCount());
console.log('Tombstone ratio:', index.tombstoneRatio());

// Compact when needed
if (index.needsCompaction()) {
    const result = index.compact();
    console.log(`Removed ${result.tombstones_removed} tombstones`);
}
```

| Operation | Time Complexity | Notes |
|:----------|:----------------|:------|
| `soft_delete()` | O(1) | Set tombstone byte |
| `is_deleted()` | O(1) | Read tombstone byte |
| `search()` | O(log n) | Automatically excludes tombstones |
| `compact()` | O(n log n) | Full index rebuild |

---

## Development Status

`EdgeVec` follows a **military-grade development protocol**. No code is written without an approved plan.

### ✅ Alpha Release Ready (v0.1.0)

**All Performance Targets Exceeded:**
- ✅ **Search Mean:** 0.23ms (4.3x under 1ms target)
- ✅ **Search P99 (estimated):** <600µs (based on Mean + 2σ)
- ✅ **Memory:** 832 MB for 1M vectors (17% under 1GB target)
- ✅ **Bundle Size:** 213 KB (57% under 500KB target)

**What Works Now:**
- ✅ **HNSW Indexing** — Sub-millisecond search at 100k scale
- ✅ **Scalar Quantization (SQ8)** — 3.6x memory reduction
- ✅ **SIMD Optimization** — AVX2/FMA for 60-78% speedup
- ✅ **Crash Recovery (WAL)** — Log-based replay
- ✅ **Atomic Snapshots** — Safe background saving
- ✅ **Browser Integration** — WASM Bindings + IndexedDB
- ✅ **npm Package** — `edgevec@0.3.0` published

**Development Progress:**
- Phase 0: Environment Setup — ✅ COMPLETE
- Phase 1: Architecture — ✅ COMPLETE
- Phase 2: Planning — ✅ COMPLETE
- Phase 3: Implementation — ✅ COMPLETE
- Phase 4: WASM Integration — ✅ COMPLETE
- Phase 5: Alpha Release — ✅ **READY**

### What's Next (v0.4.0)

1. **Multi-vector Delete** — Batch delete API
2. **P99 Tracking** — Latency distribution metrics in CI
3. **ARM/NEON Optimization** — Cross-platform SIMD verification
4. **Mobile Support** — iOS Safari and Android Chrome formalized

---

## 📊 Performance (Alpha Release)

### Search Latency (768-dimensional vectors, k=10)

| Scale | Float32 | Quantized (SQ8) | Target | Status |
|:------|:--------|:----------------|:-------|:-------|
| **10k vectors** | 203 µs | **88 µs** | <1 ms | ✅ **11x under** |
| **50k vectors** | 480 µs | **167 µs** | <1 ms | ✅ **6x under** |
| **100k vectors** | 572 µs | **329 µs** | <1 ms | ✅ **3x under** |

**Note:** Mean latencies from Criterion benchmarks (10 samples). Max observed: 622µs (100k Float32). Outliers: 0-20% (mostly high mild/severe). P99 estimates are all <650µs. See `docs/benchmarks/` for full analysis.

### Memory Efficiency (768-dimensional vectors)

| Mode | Memory per Vector | 1M Vectors | Compression |
|:-----|:------------------|:-----------|:------------|
| **Float32** | 3,176 bytes | 3.03 GB | Baseline |
| **Quantized (SQ8)** | 872 bytes | **832 MB** | **3.6x smaller** |

Memory per vector includes: vector storage + HNSW graph overhead (node metadata + neighbor pool).
Measured using `index.memory_usage() + storage.memory_usage()` after building 100k index.

### Bundle Size

| Package | Size (Gzipped) | Target | Status |
|:--------|:---------------|:-------|:-------|
| `edgevec@0.3.0` | **213 KB** | <500 KB | ✅ **57% under** |

### Competitive Comparison (10k vectors, 128 dimensions)

| Library | Search P50 | Insert P50 | Type | Notes |
|:--------|:-----------|:-----------|:-----|:------|
| **EdgeVec** | **0.20ms** | 0.83ms | WASM | Fastest WASM solution |
| hnswlib-node | 0.05ms | 1.56ms | Native C++ | Requires compilation |
| voy | 4.78ms | 0.03ms | WASM | KD-tree, batch-only |

**EdgeVec is 24x faster than voy** for search while both are pure WASM.
Native bindings (hnswlib-node) are faster but require C++ compilation and don't work in browsers.

[Full competitive analysis →](docs/benchmarks/competitive_analysis.md)

### Key Advantages

- ✅ **Sub-millisecond search** at 100k scale
- ✅ **Fastest pure-WASM solution** — 24x faster than voy
- ✅ **Zero network latency** — runs 100% locally (browser, Node, edge)
- ✅ **Privacy-preserving** — no data leaves the device
- ✅ **Tiny bundle** — 213 KB gzipped
- ✅ **No compilation required** — unlike native bindings

### Test Environment

- **Hardware:** AMD Ryzen 7 5700U, 16GB RAM
- **OS:** Windows 11
- **Rust:** 1.94.0-nightly (2025-12-05)
- **Criterion:** 0.5.x
- **Compiler flags:** `-C target-cpu=native` (AVX2 SIMD enabled)

[Full benchmarks →](docs/benchmarks/)

---

## Development Protocol

### The Agents

| Agent | Role |
|:------|:-----|
| **META_ARCHITECT** | System design, data layouts |
| **PLANNER** | Roadmaps, weekly task plans |
| **`RUST_ENGINEER`** | Core Rust implementation |
| **`WASM_SPECIALIST`** | WASM bindings, browser integration |
| **`BENCHMARK_SCIENTIST`** | Performance testing |
| **HOSTILE_REVIEWER** | Quality gate (has veto power) |
| **DOCWRITER** | Documentation, README |

---

## Development Environment

### Local CI Simulation

Before pushing changes, run the local CI simulation to catch issues:

```bash
# Run full CI check with timing validation
cargo xtask ci-check

# Run pre-release validation (CI + docs + publish dry-run)
cargo xtask pre-release
```

The `ci-check` command:
- Sets CI environment variables (`RUSTFLAGS`, `PROPTEST_CASES`, `NUM_VECTORS`)
- Runs formatting, clippy, tests, and WASM checks
- Validates each step completes within CI timeout limits

**Timing Budgets (xtask / CI timeout):**
| Step | Local Limit | CI Timeout | Typical |
|:-----|:------------|:-----------|:--------|
| Formatting | 30s | 5min | <1s |
| Clippy | 180s | 10min | ~20s |
| Tests | 600s | 30min | ~50s |
| WASM Check | 120s | 10min | <1s |

If a step exceeds its local limit, the build fails to catch performance regressions before CI.

### Environment Variables

| Variable | Local Default | CI Value | Purpose |
|:---------|:--------------|:---------|:--------|
| `RUSTFLAGS` | (native) | `-C target-cpu=x86-64-v2` | Prevent SIGILL on CI runners |
| `PROPTEST_CASES` | 256 | 32 | Reduce proptest runtime |
| `NUM_VECTORS` | 10000 | 1000 | Reduce integration test runtime |

### Building

```bash
# Standard build
cargo build --release

# WASM build
wasm-pack build --release

# Run tests
cargo test --all

# Run benchmarks
cargo bench
```

### Release Process

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the full release process, including:
- [Release Checklist](./docs/RELEASE_CHECKLIST.md)
- [Rollback Procedures](./docs/ROLLBACK_PROCEDURES.md)

---

## Origins

`EdgeVec` builds upon lessons learned from [binary_semantic_cache](../binary_semantic_cache/), a high-performance semantic caching library. Specifically:

**Salvaged (MIT Licensed):**
- Hamming distance implementation (~10 lines)
- Binary quantization math (~100 lines)

**Built Fresh:**
- HNSW graph indexing
- WASM-native architecture
- `IndexedDB` persistence
- Everything else

---

## Acknowledgments

- Thanks to the **Reddit community** for identifying a potential alignment issue in the persistence layer, which led to improved safety via `bytemuck` in v0.2.1.
- Thanks to the **Hacker News community** for feedback on competitive positioning and benchmarking.

---

## License

Licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

---

<div align="center">

**Built with 🦀 Rust + 🕸️ WebAssembly**

*Correctness by Construction*

</div>
