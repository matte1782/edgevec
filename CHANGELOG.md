# Changelog

All notable changes to EdgeVec will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Bulk insert API for faster batch operations
- Delete/update operations with tombstone-based deletion
- P99 latency tracking in CI
- ARM/NEON optimization verification

---

## [0.2.0-alpha.2] - 2025-12-12 — HOTFIX

**HOTFIX RELEASE** — Fixes critical packaging bug in v0.2.0-alpha.1.

### Fixed
- **[CRITICAL] npm package missing snippets directory** — The v0.2.0-alpha.1 package was missing the `snippets/` directory containing IndexedDB storage JS code, causing complete import failures in Node.js. This hotfix adds the missing files. (Incident: INC-2025-12-12-001)

### Changed
- Updated package.json `files` array to include `snippets` directory

### Incident Reference
- **Incident Report:** [docs/incidents/2025-12-12_alpha1_missing_snippets.md](docs/incidents/2025-12-12_alpha1_missing_snippets.md)
- **Root Cause:** wasm-pack generated `snippets/` directory was not included in package.json files array
- **Time to Resolution:** ~10 minutes

---

## [0.2.0-alpha.1] - 2025-12-12 — DEPRECATED (DO NOT USE)

> **CRITICAL BUG — DO NOT USE THIS VERSION**
>
> This version was published without the `snippets/` directory due to incomplete
> `package.json` files array. All imports will fail with `ERR_MODULE_NOT_FOUND`.
>
> **Use v0.2.0-alpha.2 instead:** `npm install edgevec@0.2.0-alpha.2`
>
> See [Incident Report](docs/incidents/2025-12-12_alpha1_missing_snippets.md) for details.

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

#### Throughput (Queries Per Second)

| Scale | Float32 | Quantized (SQ8) |
|:------|:--------|:----------------|
| 10k | 4,930 qps | **11,360 qps** |
| 50k | 2,080 qps | **5,990 qps** |
| 100k | 1,750 qps | **3,040 qps** |

#### Bundle Size

| Package | Size (Gzipped) | Target | Status |
|:--------|:---------------|:-------|:-------|
| `@edgevec/core` | **148 KB** | <500 KB | 70% under target |

#### Build/Insert Time (768-dimensional vectors)

**Note:** Build time is not optimized in this alpha release. See [Known Limitations](docs/KNOWN_LIMITATIONS.md#1-insertbuild-time-not-optimized) for workarounds.

| Scale | Float32 Build | Quantized (SQ8) Build |
|:------|:--------------|:----------------------|
| 10k vectors | 6.3s | 2.6s |
| 50k vectors | 55s | 24s |
| 100k vectors | 116s | 63s |

*Quantized mode is 2-3x faster for bulk building. Batch insert API planned for v0.3.0.*

### Known Limitations

See [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md) for detailed information.

1. **Insert Latency**: Build time is not optimized for this alpha. Batch insert API planned for v0.3.0.

2. **No Delete/Update**: Vectors are append-only. Tombstone-based deletion planned for v0.3.0.

3. **Single-Threaded WASM**: Browser execution is single-threaded. Use Web Workers for parallel searches on separate index instances.

4. **Compiler Flags Required**: For optimal performance, users must configure `.cargo/config.toml` with `-C target-cpu=native`. Without this, performance is 60-78% slower.

### Breaking Changes

None (initial alpha release).

### Security

- No external network calls
- No user data sent to servers
- All computation runs locally (browser, Node, edge)
- IndexedDB data stored locally in browser sandbox

### Migration Guide

N/A (initial release).

---

## Version History

| Version | Date | Highlights |
|:--------|:-----|:-----------|
| 0.2.0-alpha.1 | 2025-12-12 | Initial public alpha release |

---

## Links

- [GitHub Repository](https://github.com/anthropics/edgevec)
- [Documentation](docs/)
- [Performance Guide](docs/PERFORMANCE_GUIDE.md)
- [Known Limitations](docs/KNOWN_LIMITATIONS.md)

---

**Note:** This is an **alpha release**. APIs may change before v1.0.0 stable. Please report issues at the GitHub repository.
