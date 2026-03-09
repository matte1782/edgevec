# Fuzzing Campaign Plan

**Document:** `docs/audits/FUZZ_CAMPAIGN_PLAN.md`
**Date:** 2026-03-08
**Status:** [PROPOSED]
**Author:** TEST_ENGINEER agent

---

## 1. Existing Fuzz Targets

EdgeVec has 15 fuzz targets registered in `fuzz/Cargo.toml` (plus 1 `dummy_harness` for link verification). Each lives in `fuzz/fuzz_targets/<name>/target.rs`.

| # | Target | What It Fuzzes | Corpus Seeds | Last Known Run |
|:--|:-------|:---------------|:-------------|:---------------|
| 1 | `dummy_harness` | Nothing (link verification only) | 0 | N/A |
| 2 | `quantization` | `BinaryQuantizer::quantize()` with arbitrary f32 vectors; verifies determinism, self-distance=0, Hamming bounds, symmetry, output size, similarity range | 103 | W8 (Dec 2025) |
| 3 | `header_parse` | `FileHeader::from_bytes()` with arbitrary bytes; verifies no panics | 0 | W8 (Dec 2025) |
| 4 | `hnsw_config` | `HnswConfig` construction + `HnswIndex::new()` with arbitrary u32 parameters | 0 | W8 (Dec 2025) |
| 5 | `hnsw_insert` | Interleaved insert/search operations on 4D HNSW index from raw byte commands | 12 | W10 (Dec 2025) |
| 6 | `hnsw_search` | `Searcher::search_layer()` with arbitrary graph topology, random entry points, fuzz-generated vectors via `MockVectorProvider` | 12 | W10 (Dec 2025) |
| 7 | `graph_ops` | Sequence of `Insert`/`Delete`/`Search`/`SaveLoad` operations on HNSW; verifies sorted results and connectivity | 12 | W10 (Dec 2025) |
| 8 | `search_robustness` | `search_layer()` with arbitrary `NodeId` entry points on a fixed diamond graph; verifies no panic | 20 | W10 (Dec 2025) |
| 9 | `persistence` | `read_snapshot()` from arbitrary bytes via `MemoryBackend`; verifies no panic | 0 | W9 (Dec 2025) |
| 10 | `wal_replay` | `WalIterator` over arbitrary byte stream; verifies no panic during iteration | 0 | W8 (Dec 2025) |
| 11 | `filter_simple` | `filter::parse()` with arbitrary UTF-8 strings up to 10KB; verifies no panic | 2130 | W24 (Dec 2025) |
| 12 | `filter_deep` | `filter::parse()` with structured deeply-nested AND/OR/NOT expressions (max depth 50) | 159 | W24 (Dec 2025) |
| 13 | `flat_index` | `FlatIndex::from_snapshot()` with arbitrary bytes; exercises loaded index methods and roundtrip serialization | 0 | W38 (Feb 2026) |
| 14 | `sparse_vector` | `SparseVector::new()` and `from_pairs()` with arbitrary indices/values/dimension; exercises dot, cosine, normalize | 0 | W38 (Feb 2026) |
| 15 | `sparse_storage` | `SparseStorage::from_bytes()` with arbitrary bytes; exercises loaded storage and roundtrip | 0 | W38 (Feb 2026) |

**Note:** Target #7 `graph_ops` was updated in W38 to a comprehensive `Arbitrary`-derived lifecycle test (insert, delete, search, save/load roundtrip), superseding the original W10 version.

### Assessment of Existing Targets

**Strong points:**
- Filter parser is well-covered (both raw strings and structured nesting).
- HNSW graph operations are covered from multiple angles (insert, search, config, graph ops, robustness).
- Persistence deserialization paths (snapshot, WAL, FlatIndex, SparseStorage) are covered.
- Binary quantization has the richest corpus (103 seeds with edge cases).

**Weak points:**
- 7 targets have 0 corpus seeds (header_parse, hnsw_config, persistence, wal_replay, flat_index, sparse_vector, sparse_storage).
- `dummy_harness` serves no purpose in a production campaign.
- No targets for Product Quantization (PQ), MetadataStore deserialization, WASM boundary inputs, or neighbor encoding/decoding.

---

## 2. Input Boundary Map

Every function that accepts external/untrusted data is an input boundary. The following table maps all such boundaries in `src/`.

| # | Module | Entry Point | Input Type | Accepts Untrusted Data | Current Fuzz Coverage |
|:--|:-------|:------------|:-----------|:----------------------|:---------------------|
| B1 | `filter::parser` | `parse(input: &str)` | Arbitrary string | YES (user-supplied filter expressions) | YES (`filter_simple`, `filter_deep`) |
| B2 | `persistence::header` | `FileHeader::from_bytes(bytes: &[u8])` | Raw bytes | YES (file on disk / IndexedDB) | YES (`header_parse`) |
| B3 | `persistence::header` | `MetadataHeader::from_bytes(bytes: &[u8])` | Raw bytes | YES (file on disk) | NO |
| B4 | `persistence::snapshot` | `read_snapshot(backend)` | Raw bytes via backend | YES (snapshot file) | YES (`persistence`) |
| B5 | `persistence::storage` | `load_snapshot(backend)` | Raw bytes via backend | YES (snapshot file) | PARTIAL (covered indirectly by `persistence`) |
| B6 | `persistence::wal` | `WalIterator::new(reader)` | Raw bytes | YES (WAL file) | YES (`wal_replay`) |
| B7 | `index::flat` | `FlatIndex::from_snapshot(data: &[u8])` | Raw bytes | YES (snapshot file) | YES (`flat_index`) |
| B8 | `sparse::storage` | `SparseStorage::from_bytes(bytes: &[u8])` | Raw bytes | YES (serialized file) | YES (`sparse_storage`) |
| B9 | `sparse::vector` | `SparseVector::new(indices, values, dim)` | Arbitrary indices/values | YES (user API) | YES (`sparse_vector`) |
| B10 | `quantization::product` | `PqCode::from_codes(codes: Vec<u8>)` | Arbitrary byte vector | YES (WASM boundary, lesson #75) | **NO** |
| B11 | `quantization::product` | `PqCodebook::train(vectors, m, ksub, ...)` | Arbitrary f32 vectors | YES (user API) | **NO** |
| B12 | `quantization::product` | `PqCodebook::compute_distance_table(query)` | Arbitrary f32 vector | YES (user API) | **NO** |
| B13 | `quantization::product` | `PqCodebook::encode(vector)` | Arbitrary f32 vector | YES (user API) | **NO** |
| B14 | `quantization::variable` | `BinaryVector::from_bytes(data, dim)` | Raw bytes | YES (serialized data) | **NO** |
| B15 | `metadata::serialize` | `MetadataStore::from_postcard(bytes)` | Raw bytes | YES (serialized metadata) | **NO** |
| B16 | `metadata::serialize` | `MetadataStore::from_json(bytes)` | Raw bytes (JSON) | YES (WASM boundary) | **NO** |
| B17 | `metadata::store` | `MetadataStore::insert(id, key, value)` | User-supplied key/value pairs | YES (validated by `validation.rs`) | **NO** |
| B18 | `hnsw::neighbor` | `NeighborCodec::decode_neighbors(data: &[u8])` | Raw bytes | YES (from persistence) | **NO** |
| B19 | `hnsw::neighbor` | `NeighborCodec::decode_layer(data, level)` | Raw bytes + level | YES (from persistence) | **NO** |
| B20 | `wasm::mod` | `EdgeVec::insert_binary(vector: Uint8Array)` | Arbitrary bytes from JS | YES (WASM boundary) | **NO** (WASM-only, not fuzzable natively) |
| B21 | `wasm::mod` | `EdgeVec::search_binary(query: Uint8Array, k)` | Arbitrary bytes from JS | YES (WASM boundary) | **NO** (WASM-only) |
| B22 | `wasm::filter` | `parse_filter_js(filter_str: &str)` | Arbitrary string from JS | YES (WASM boundary) | INDIRECT (same parser as B1) |
| B23 | `filter::boost` | `MetadataBoost::new(field, op, value, weight)` | User-supplied config | YES (WASM boundary) | **NO** |
| B24 | `filter::evaluator` | `evaluate(expr, metadata)` | Parsed AST + metadata | YES (internal, but AST from B1) | **NO** |

---

## 3. Gap Analysis

Ordered by risk (highest first). Risk considers: (a) whether the boundary accepts untrusted bytes, (b) whether a crash could cause memory unsafety, (c) attack surface breadth.

| Priority | Gap | Boundary | Risk Rationale |
|:---------|:----|:---------|:---------------|
| **P0 CRITICAL** | PQ `from_codes()` has no validation | B10 | Lesson #75: code values >= Ksub cause OOB reads during `scan_topk`. WASM boundary passes user-supplied bytes directly. This is the single highest-risk unfuzzed boundary. |
| **P1 HIGH** | PQ train/encode/distance_table pipeline | B11-B13 | NaN validation was added (lesson #63) but never fuzz-verified. Edge cases in k-means convergence with adversarial float patterns could trigger panics or infinite loops. |
| **P1 HIGH** | MetadataStore deserialization | B15-B16 | `from_postcard()` and `from_json()` accept raw bytes from persistence/WASM. No fuzz target exists. Postcard deserialization of untrusted data is a known attack surface. |
| **P1 HIGH** | Neighbor codec decode | B18-B19 | `decode_neighbors()` and `decode_layer()` parse raw bytes from snapshot files. Malformed data could cause incorrect graph reconstruction. Currently only fuzzed indirectly through `persistence` target (which may not reach these paths). |
| **P2 MEDIUM** | BinaryVector deserialization | B14 | `from_bytes()` parses raw bytes with dimension parameter. No fuzz target. |
| **P2 MEDIUM** | MetadataHeader parsing | B3 | Separate from `FileHeader` (B2 which is fuzzed). `MetadataHeader::from_bytes()` has its own parsing logic. |
| **P2 MEDIUM** | Filter evaluator | B24 | Parser is fuzzed, but evaluator is not directly fuzzed with adversarial AST+metadata combinations. Could have edge cases in type coercion. |
| **P2 MEDIUM** | MetadataBoost construction | B23 | `MetadataBoost::new()` validates weight (NaN/Inf), but combined boost pipeline with adversarial configs is untested. |
| **P3 LOW** | Corpus gaps for existing targets | B2,B4,B6,B7,B8,B9 | Seven targets have 0 corpus seeds. The fuzzer must discover all interesting inputs from scratch, reducing coverage efficiency. |

---

## 4. Proposed New Targets

### Target N1: `pq_from_codes` (P0 CRITICAL)

**What to fuzz:** `PqCodebook::train()` followed by `PqCode::from_codes()` with arbitrary byte vectors, then `scan_topk()` to trigger OOB reads if codes >= Ksub.

**Approach:** Use `Arbitrary` to generate (a) a small training set of 4D vectors, (b) a PqCodebook via `train()`, (c) arbitrary `PqCode` from random bytes, (d) call `compute_distance_table()` + `scan_topk()`.

**Expected complexity:** Medium. Must handle the training step (slow for large M/Ksub), so constrain to small dimensions (D=4-8, M=2-4, Ksub=4-16).

### Target N2: `pq_pipeline` (P1 HIGH)

**What to fuzz:** Full PQ pipeline: `train()` -> `encode()` -> `compute_distance_table()` -> `scan_topk()` with arbitrary f32 vectors including NaN/Inf/subnormal.

**Approach:** Generate random f32 training vectors and queries. Verify no panic and that NaN validation catches bad inputs.

**Expected complexity:** Medium-high. Training is CPU-intensive. Constrain dimensions and vector count.

### Target N3: `metadata_deserialize` (P1 HIGH)

**What to fuzz:** `MetadataStore::from_postcard(bytes)` and `MetadataStore::from_json(bytes)` with arbitrary bytes.

**Approach:** Feed raw bytes to both deserializers. If successful, exercise `get()`, `keys()`, iteration.

**Expected complexity:** Low. Pure deserialization fuzzing.

### Target N4: `neighbor_codec` (P1 HIGH)

**What to fuzz:** `NeighborCodec::decode_neighbors(data)` and `NeighborCodec::decode_layer(data, level)` with arbitrary bytes and level values.

**Approach:** Feed random bytes and random u8 level values.

**Expected complexity:** Low. Simple byte parsing.

### Target N5: `variable_quantizer` (P2 MEDIUM)

**What to fuzz:** `BinaryVector::from_bytes(data, dimension)` with arbitrary bytes and dimension values.

**Approach:** Feed random bytes with random dimension values (0, 1, 768, u32::MAX, etc.).

**Expected complexity:** Low.

### Target N6: `metadata_header` (P2 MEDIUM)

**What to fuzz:** `MetadataHeader::from_bytes(bytes)` with arbitrary bytes.

**Approach:** Same pattern as `header_parse` target.

**Expected complexity:** Low.

### Target N7: `filter_evaluate` (P2 MEDIUM)

**What to fuzz:** Generate random filter strings via `filter_simple` approach, parse them, then evaluate against random metadata maps.

**Approach:** Use `Arbitrary` to generate metadata maps with random types (String, Integer, Float, Boolean, Null). Parse a fuzz-generated filter string, then `evaluate()` it against the metadata.

**Expected complexity:** Medium. Must generate valid metadata maps.

---

## 5. 48-Hour Campaign Plan

### 5.1 Schedule

The campaign runs 15 targets across 48 hours. Time allocation is weighted by priority and complexity. Targets with 0 corpus seeds get extra time for initial exploration.

| Phase | Hours | Targets | Rationale |
|:------|:------|:--------|:----------|
| **Phase 1: Critical (0-12h)** | 12 | `pq_from_codes` (6h), `pq_pipeline` (6h) | P0/P1 targets. PQ is the highest-risk unfuzzed code. |
| **Phase 2: High Priority (12-24h)** | 12 | `metadata_deserialize` (4h), `neighbor_codec` (4h), `persistence` (2h), `flat_index` (2h) | P1 targets + re-running existing targets with empty corpus. |
| **Phase 3: Existing Core (24-36h)** | 12 | `graph_ops` (3h), `hnsw_insert` (3h), `hnsw_search` (3h), `filter_simple` (1.5h), `filter_deep` (1.5h) | Re-validate core targets with extended runtime. |
| **Phase 4: Coverage Fill (36-48h)** | 12 | `variable_quantizer` (2h), `metadata_header` (2h), `filter_evaluate` (2h), `quantization` (2h), `wal_replay` (2h), `sparse_storage` (1h), `sparse_vector` (1h) | P2 targets + remaining corpus gaps. |

**Total: 48 hours, 23 target-runs across 15 distinct targets (7 new + 8 existing re-runs).** Note: some targets run in multiple phases, accounting for the 23 total runs.

### 5.2 Infrastructure Requirements

| Resource | Minimum | Recommended |
|:---------|:--------|:------------|
| **Platform** | Linux (required for `cargo-fuzz` / libFuzzer) | Ubuntu 22.04+ |
| **CPU** | 4 cores (1 target at a time) | 8+ cores (2-4 targets in parallel) |
| **RAM** | 8 GB | 16 GB (PQ training targets are memory-hungry) |
| **Disk** | 10 GB free (corpus + artifacts) | 20 GB (headroom for corpus growth) |
| **Toolchain** | Rust nightly + `cargo-fuzz` | Latest nightly + LLVM 16+ for coverage |
| **Runtime** | Serial: 48h wall clock | Parallel (4 cores): ~12-16h wall clock |

**CI Option:** GitHub Actions `ubuntu-latest` runner with `timeout-minutes: 2880` (48h). Free tier runners have 2 cores / 7 GB RAM, which is sufficient for serial execution. For parallel execution, use a self-hosted runner or paid plan with 8+ cores.

**Windows Note:** The development machine runs Windows 11. `cargo-fuzz` has limited Windows support. The campaign MUST run on Linux. Options:
1. WSL2 with Ubuntu (available on the dev machine)
2. GitHub Actions Linux runner
3. Dedicated Linux VM/container

### 5.3 Corpus Seeding Strategy

| Target | Seeding Strategy | Estimated Initial Seeds |
|:-------|:-----------------|:----------------------|
| `pq_from_codes` | Generate valid PqCodes from a pre-trained small codebook (D=8, M=2, Ksub=8). Include codes with values 0, Ksub-1, Ksub (boundary), 255. | 20 |
| `pq_pipeline` | 768D real embedding subsets from SQuAD data (if available) downsampled to 8D. Include all-zero, all-NaN, all-Inf vectors. | 30 |
| `metadata_deserialize` | Valid postcard-serialized MetadataStore with 0, 1, 100 entries. Valid JSON with various types. Truncated versions of each. | 15 |
| `neighbor_codec` | Valid encoded neighbor lists (0 neighbors, 1, M, M0). Truncated and padded versions. | 10 |
| `variable_quantizer` | Valid serialized BinaryVector blobs. Truncated at various offsets. | 10 |
| `metadata_header` | Valid MetadataHeader bytes. Corrupted magic, version, checksum variants. | 10 |
| `filter_evaluate` | Reuse `filter_simple` corpus (2130 seeds) as base; prepend metadata map bytes. | 50 |
| Existing targets with 0 seeds | Generate minimal valid inputs programmatically (similar to `fuzz/generate_corpus.py`). | 10-20 each |

**Total estimated seed corpus: ~250 new seeds across all targets.**

### 5.4 Crash Triage Process

1. **Immediate (automated):** `cargo-fuzz` writes crash inputs to `fuzz/artifacts/<target>/`. Each crash file contains the exact bytes that triggered it.

2. **Minimize (within 1h of discovery):**
   ```bash
   cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/crash-<hash>
   ```

3. **Classify severity:**
   - **P0 Critical:** Memory unsafety (ASAN report, buffer overflow, use-after-free). Requires immediate fix.
   - **P1 High:** Panic in production code path (unwrap, index out of bounds, assertion failure). Fix before merge.
   - **P2 Medium:** Invariant violation (wrong result, not sorted, bounds exceeded). Fix in next sprint.
   - **P3 Low:** Timeout or excessive memory (algorithmic issue). Track for optimization.

4. **Reproduce:** Create a standalone unit test from minimized input:
   ```rust
   #[test]
   fn regression_crash_<hash>() {
       let data = include_bytes!("../../fuzz/artifacts/<target>/crash-<hash>-minimized");
       // ... exercise the same code path as the fuzz target
   }
   ```

5. **Fix:** Implement fix, verify regression test passes, add minimized crash to permanent corpus.

6. **Document:** Log crash in `fuzz/artifacts/CRASH_LOG.md` with: date, target, severity, root cause, fix commit.

---

## 6. Resource Estimates

### 6.1 Corpus Size Projections

After 48 hours of coverage-guided fuzzing, corpus growth depends on code complexity and path count.

| Target | Initial Seeds | Estimated 48h Corpus | Avg Seed Size | Estimated Disk |
|:-------|:-------------|:--------------------|:-------------|:---------------|
| `pq_from_codes` | 20 | 500-1000 | 64 B | 64 KB |
| `pq_pipeline` | 30 | 200-500 | 256 B | 128 KB |
| `metadata_deserialize` | 15 | 500-1500 | 128 B | 192 KB |
| `neighbor_codec` | 10 | 300-800 | 32 B | 25 KB |
| `filter_simple` | 2130 | 3000-5000 | 64 B | 320 KB |
| `filter_deep` | 159 | 500-1000 | 128 B | 128 KB |
| `graph_ops` | 12 | 200-500 | 512 B | 256 KB |
| `hnsw_insert` | 12 | 200-500 | 256 B | 128 KB |
| `hnsw_search` | 12 | 200-500 | 256 B | 128 KB |
| `persistence` | 0 | 300-800 | 256 B | 204 KB |
| `flat_index` | 0 | 300-800 | 256 B | 204 KB |
| `quantization` | 103 | 500-1000 | 3072 B | 3 MB |
| Others (7 targets) | ~70 | 200-500 each | 64-256 B | ~500 KB |
| **TOTAL** | **~2500** | **~7000-14000** | -- | **~5-6 MB** |

### 6.2 Memory Usage Per Target

| Target | Peak RSS (estimated) | Rationale |
|:-------|:--------------------|:----------|
| `pq_pipeline` | 200-500 MB | K-means training allocates codebook + distance matrices |
| `graph_ops` | 100-200 MB | HNSW graph + storage for up to ~1000 vectors |
| `filter_evaluate` | 50-100 MB | Metadata maps + AST allocation |
| `quantization` | 50-100 MB | 768D float vectors |
| All others | 10-50 MB | Small inputs, simple data structures |

### 6.3 Total Infrastructure Budget

| Resource | Estimate |
|:---------|:---------|
| Disk for corpus | 6 MB |
| Disk for artifacts | 1 MB (assuming few crashes) |
| Disk for fuzz binaries | 500 MB (debug builds with ASAN) |
| Peak RAM (single target) | 500 MB (`pq_pipeline`) |
| Peak RAM (4 parallel) | 1.5 GB |
| CPU-hours | 48h serial / 12-16h parallel on 4 cores |

---

## 7. Success Criteria

### Primary Gate

**0 crashes after 48 hours of continuous fuzzing across all targets = PASS.**

This means:
- Zero ASAN violations (buffer overflow, use-after-free, stack overflow)
- Zero panics in production code paths
- Zero assertion failures
- Zero timeouts exceeding 60 seconds per input

### Secondary Metrics

| Metric | Target | How Measured |
|:-------|:-------|:-------------|
| Corpus coverage | >80% line coverage on fuzzed functions | `cargo fuzz coverage` + `llvm-cov` |
| Executions per second | >1000 exec/s average across targets | libFuzzer stats output |
| New corpus entries discovered | >100 per target | Corpus directory file count |
| Total executions | >100M across all targets | libFuzzer stats sum |

### Failure Response

| Result | Action |
|:-------|:-------|
| **0 crashes** | Campaign PASS. Document results, archive corpus, update FUZZING_STRATEGY.md. |
| **P3 only (timeouts)** | Campaign CONDITIONAL PASS. Log timeouts, create optimization tasks. |
| **P2 crashes (invariant violations)** | Campaign CONDITIONAL PASS. Create fix tasks, add regression tests. |
| **P1 crashes (panics)** | Campaign FAIL. Fix all panics, re-run affected targets for 12h. |
| **P0 crashes (memory unsafety)** | Campaign CRITICAL FAIL. Stop campaign, fix immediately, full 48h re-run required. |

---

## 8. Summary

EdgeVec has a solid foundation of 14 functional fuzz targets (excluding `dummy_harness`) covering HNSW, persistence, filter parsing, binary quantization, FlatIndex, and sparse vectors. However, significant gaps exist:

**Critical gap:** Product Quantization `from_codes()` accepts arbitrary bytes with no validation, and is reachable from the WASM boundary. This is a known footgun (lesson #75) with zero fuzz coverage.

**High-priority gaps:** MetadataStore deserialization (postcard/JSON), neighbor codec decoding, and the full PQ train/encode pipeline have no dedicated fuzz targets.

**Campaign plan:** 48 hours of continuous fuzzing across 15 targets (7 new + 8 existing re-runs), prioritized by risk. Infrastructure requires a Linux environment with 8+ cores for parallel execution. Success criterion is zero crashes.

**Estimated effort to implement new targets:** 4-6 hours of development time for 7 new targets, plus 1-2 hours for corpus generation scripts.

---

**END OF DOCUMENT**
