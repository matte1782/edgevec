# Proptest Simulation Report — EdgeVec v0.9.0

**Date:** 2026-03-01
**Status:** [PROPOSED]
**Auditor:** TEST_ENGINEER

---

## Executive Summary

Cross-platform proptest-based random testing covering 5 critical subsystems with **8,000 iterations** across 8 proptest simulations. **Zero panics found.**

**Important:** This is NOT a coverage-guided fuzzing campaign. Proptest uses strategy-based random generation, not coverage-guided mutation (libFuzzer). See [Limitations](#limitations) for details.

---

## Platform Note

`cargo fuzz run` (libFuzzer) is not available on Windows MSVC (linker error: `LNK2001: simbolo esterno main non risolto`). Two approaches used instead:

1. **`cargo +nightly fuzz check`** — Verifies all 15 fuzz targets compile correctly
2. **Proptest-based simulations** (`tests/proptest_fuzz.rs`) — Structured random testing that runs on any platform

---

## Fuzz Target Compilation Check

All 15 cargo-fuzz targets compile successfully via `cargo +nightly fuzz check`:

| Target | Status |
|:-------|:-------|
| `dummy_harness` | COMPILES |
| `filter_deep` | COMPILES |
| `filter_simple` | COMPILES |
| `flat_index` | COMPILES |
| `graph_ops` | COMPILES |
| `header_parse` | COMPILES |
| `hnsw_config` | COMPILES |
| `hnsw_insert` | COMPILES |
| `hnsw_search` | COMPILES |
| `persistence` | COMPILES |
| `quantization` | COMPILES |
| `search_robustness` | COMPILES |
| `sparse_storage` | COMPILES |
| `sparse_vector` | COMPILES |
| `wal_replay` | COMPILES |

---

## Proptest Simulation Results

### Configuration

- **Cases per test:** 1,000 (default, configurable via `PROPTEST_CASES`)
- **Mode:** Release (optimized)
- **Total iterations:** 8,000 (8 tests x 1,000 cases)

### Results

| Test | Target Module | Invariant | Cases | Result |
|:-----|:-------------|:----------|:------|:-------|
| `proptest_fuzz_filter_deep` | filter parser | Parse never panics on deeply nested expressions | 1,000 | PASS |
| `proptest_fuzz_filter_arbitrary_string` | filter parser | Parse never panics on arbitrary UTF-8 strings | 1,000 | PASS |
| `proptest_fuzz_persistence` | persistence | Deserialize never panics on arbitrary bytes | 1,000 | PASS |
| `proptest_fuzz_persistence_roundtrip` | persistence | Save/load roundtrip preserves count + searchability | 1,000 | PASS |
| `proptest_fuzz_header_parse` | header parser | FileHeader::from_bytes never panics on arbitrary bytes | 1,000 | PASS |
| `proptest_fuzz_hnsw_search` | HNSW graph | Search never panics with random topology + full f32 range | 1,000 | PASS |
| `proptest_fuzz_hnsw_insert_search` | HNSW graph | Insert+search roundtrip: results <= k, full f32 range | 1,000 | PASS |
| `proptest_fuzz_graph_ops` | HNSW graph | Interleaved insert/delete/search never panics | 1,000 | PASS |

### Coverage Mapping: Cargo-Fuzz Targets vs Proptest Equivalents

| Cargo-Fuzz Target | Proptest Equivalent | Coverage Level |
|:------------------|:-------------------|:---------------|
| `filter_deep` | `proptest_fuzz_filter_deep` + `proptest_fuzz_filter_arbitrary_string` | Equivalent |
| `persistence` | `proptest_fuzz_persistence` + `proptest_fuzz_persistence_roundtrip` | Equivalent |
| `hnsw_search` | `proptest_fuzz_hnsw_search` + `proptest_fuzz_hnsw_insert_search` | Equivalent |
| `header_parse` | `proptest_fuzz_header_parse` | Equivalent |
| `graph_ops` | `proptest_fuzz_graph_ops` | Substantial (no SaveLoad interleave) |
| `filter_simple` | — | **NOT COVERED** |
| `flat_index` | — | **NOT COVERED** |
| `sparse_storage` | — | **NOT COVERED** |
| `sparse_vector` | — | **NOT COVERED** |
| `hnsw_config` | — | **NOT COVERED** |
| `hnsw_insert` | — | **NOT COVERED** |
| `quantization` | — | **NOT COVERED** |
| `search_robustness` | — | **NOT COVERED** |
| `wal_replay` | — | **NOT COVERED** |
| `dummy_harness` | — | N/A (test harness) |

**Coverage: 5 of 14 real targets (36%)** have proptest equivalents. The remaining 9 targets require libFuzzer on Linux CI.

---

## Panics Found

**None.** Zero panics across 8,000 iterations.

---

## Limitations

1. **Proptest is NOT coverage-guided fuzzing.** It generates random inputs from defined strategies. It cannot discover inputs outside the strategy space and does not use code coverage feedback to guide mutation.
2. **libFuzzer (cargo-fuzz) could not run on Windows MSVC.** The 15 cargo-fuzz targets are verified to compile but have NOT been executed. Running them on Linux CI is essential for security assurance.
3. **9 of 14 cargo-fuzz targets have no proptest equivalent.** These modules (`filter_simple`, `flat_index`, `sparse_storage`, `sparse_vector`, `hnsw_config`, `hnsw_insert`, `quantization`, `search_robustness`, `wal_replay`) are only covered by their regular unit/integration tests, not by randomized testing.
4. **SafeMockVectorProvider returns zero vectors for missing IDs** instead of panicking. This is intentional to isolate EdgeVec panics from test infrastructure, but means the provider itself cannot reveal bugs via panic.

---

## Recommendations

1. **Run libFuzzer targets in CI** — GitHub Actions runs on Ubuntu where `cargo fuzz run` works. Add a scheduled nightly job for extended fuzzing of all 15 targets.
2. **Add proptest equivalents for uncovered targets** — Priority: `quantization`, `flat_index`, `sparse_vector`.
3. **Increase cases for deeper local testing** — `PROPTEST_CASES=50000 cargo test --test proptest_fuzz --release`.
4. **Keep proptest_fuzz.rs in sync** — When adding new fuzz targets, add corresponding proptest simulations.

---

## Verdict

**PENDING REVIEW** — All tested subsystems (filter parser, persistence, header, HNSW search, graph ops) pass proptest simulation with zero panics. 9 of 14 cargo-fuzz targets remain untested due to Windows platform limitation and missing proptest equivalents. libFuzzer execution on Linux CI is recommended before v1.0 release.

---

**END OF PROPTEST SIMULATION REPORT**
