# 🚀 `EdgeVec`

[![CI](https://github.com/matte1782/edgevec/actions/workflows/ci.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/ci.yml)
[![Performance](https://github.com/matte1782/edgevec/actions/workflows/benchmark.yml/badge.svg)](https://github.com/matte1782/edgevec/actions/workflows/benchmark.yml)
[![Crates.io](https://img.shields.io/crates/v/edgevec.svg)](https://crates.io/crates/edgevec)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/matte1782/edgevec/blob/main/LICENSE-MIT)

**High-performance vector search for Browser, Node, and Edge**

> ✅ **STATUS: Alpha Release Ready** — All performance targets exceeded.

---

## What's New in v0.4.0

### Documentation & Quality Sprint
- **`docs/TUTORIAL.md`** — Complete getting started guide
- **`docs/PERFORMANCE_TUNING.md`** — HNSW parameter optimization
- **`docs/TROUBLESHOOTING.md`** — Top 10 errors and solutions
- **`docs/INTEGRATION_GUIDE.md`** — Third-party embedding integrations
- **`docs/MIGRATION.md`** — Migration from hnswlib, FAISS, Pinecone

### Benchmark Dashboard
- **Interactive visualization** at `/wasm/examples/benchmark-dashboard.html`
- EdgeVec vs hnswlib-node vs voy comparison
- Real-time performance charts with Chart.js

### Quality Infrastructure
- **Chaos Testing** — 15 edge case tests (empty index, max dimensions, etc.)
- **Load Testing** — 100k vector stress tests, sustained search load
- **P99 Latency Tracking** — P50/P99/P999 percentile benchmarks
- **CI Regression Detection** — 10% threshold enforcement

### Previous (v0.3.0)
- Soft delete API with O(1) tombstone-based deletion
- Compaction API for reclaiming space
- Full WASM bindings for soft delete operations
- Persistence format v0.3 with automatic migration

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
- **Tiny Bundle** — 227 KB gzipped (55% under 500KB target)

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

## Interactive Examples

Try EdgeVec directly in your browser with our NVIDIA-grade cyberpunk demo suite:

<p align="center">
  <a href="wasm/examples/index.html">
    <img src="docs/screenshot/demo-catalog.png" alt="EdgeVec Demo Catalog" width="800">
  </a>
</p>

<p align="center">
  <strong>
    <a href="wasm/examples/index.html">View All Examples</a> |
    <a href="wasm/examples/benchmark-dashboard.html">Launch Dashboard</a>
  </strong>
</p>

### Demo Gallery

<table>
<tr>
<td width="50%">
<a href="wasm/examples/benchmark-dashboard.html">
<img src="docs/screenshot/benchmark-dashboard.png" alt="Benchmark Dashboard">
</a>
<h4>Performance Dashboard</h4>
<p>Competitive analysis vs hnswlib-node & voy with interactive Chart.js visualizations</p>
</td>
<td width="50%">
<a href="wasm/examples/soft_delete.html">
<img src="docs/screenshot/soft-delete-demo.png" alt="Soft Delete Demo">
</a>
<h4>Soft Delete & Compaction</h4>
<p>RFC-001 implementation with tombstone visualization and real-time metrics</p>
</td>
</tr>
<tr>
<td width="50%">
<a href="wasm/examples/batch_insert.html">
<img src="docs/screenshot/batch-insert-demo.png" alt="Batch Insert Demo">
</a>
<h4>Batch Insert</h4>
<p>Sequential vs batch comparison with progress tracking</p>
</td>
<td width="50%">
<a href="wasm/examples/stress-test.html">
<img src="docs/screenshot/stress-test-demo.png" alt="Stress Test Demo">
</a>
<h4>Stress Test</h4>
<p>Push EdgeVec to its limits with continuous operations</p>
</td>
</tr>
</table>

### Running Locally

```bash
# Clone the repository
git clone https://github.com/matte1782/edgevec.git
cd edgevec

# IMPORTANT: Start server from project root!
python -m http.server 8080

# Open in browser (include full path)
# http://localhost:8080/wasm/examples/index.html
```

> ⚠️ **Note:** Do NOT start server from `wasm/examples/` — WASM module requires `/pkg/` access from root.

---

## Development Status

`EdgeVec` follows a **military-grade development protocol**. No code is written without an approved plan.

### ✅ Alpha Release Ready (v0.1.0)

**All Performance Targets Exceeded:**
- ✅ **Search Mean:** 0.23ms (4.3x under 1ms target)
- ✅ **Search P99 (estimated):** <600µs (based on Mean + 2σ)
- ✅ **Memory:** 832 MB for 1M vectors (17% under 1GB target)
- ✅ **Bundle Size:** 227 KB (55% under 500KB target)

**What Works Now:**
- ✅ **HNSW Indexing** — Sub-millisecond search at 100k scale
- ✅ **Scalar Quantization (SQ8)** — 3.6x memory reduction
- ✅ **SIMD Optimization** — AVX2/FMA for 60-78% speedup
- ✅ **Crash Recovery (WAL)** — Log-based replay
- ✅ **Atomic Snapshots** — Safe background saving
- ✅ **Browser Integration** — WASM Bindings + IndexedDB
- ✅ **npm Package** — `edgevec@0.4.0` published

**Development Progress:**
- Phase 0: Environment Setup — ✅ COMPLETE
- Phase 1: Architecture — ✅ COMPLETE
- Phase 2: Planning — ✅ COMPLETE
- Phase 3: Implementation — ✅ COMPLETE
- Phase 4: WASM Integration — ✅ COMPLETE
- Phase 5: Alpha Release — ✅ **READY**

### Future Roadmap (v0.5.0+)

1. **ARM/NEON Optimization** — Cross-platform SIMD verification
2. **Mobile Support** — iOS Safari and Android Chrome formalized
3. **CLI Tools** — Optional developer command-line interface
4. **Enhanced Metadata Storage** — Native metadata support

### Path to v1.0

EdgeVec will reach v1.0 after:
- Production usage feedback from v0.4.0/v0.5.0
- Security audit
- API stability guarantee commitment

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
| `edgevec@0.4.0` | **227 KB** | <500 KB | ✅ **55% under** |

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
- ✅ **Tiny bundle** — 227 KB gzipped
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
