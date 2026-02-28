# Fuzzing Campaign Report — EdgeVec v0.9.0

**Date:** 2026-03-01
**Status:** [APPROVED]
**Auditor:** TEST_ENGINEER + HOSTILE_REVIEWER

---

## Executive Summary

Fuzzing campaign covering 5 critical subsystems with **30,000+ iterations** across 6 proptest-based fuzz simulations. **Zero crashes, zero panics found.**

---

## Platform Note

`cargo fuzz run` (libFuzzer) is not available on Windows MSVC (linker error: `LNK2001: simbolo esterno main non risolto`). Two approaches used instead:

1. **`cargo +nightly fuzz check`** — Verifies all 15 fuzz targets compile correctly
2. **Proptest-based fuzz simulations** (`tests/proptest_fuzz.rs`) — Structured random testing that runs on any platform

---

## Fuzz Target Compilation Check

All 15 cargo-fuzz targets compile successfully:

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

## Proptest Fuzz Simulations

### Configuration

- **Cases per test:** 5,000 (via `PROPTEST_CASES=5000`)
- **Mode:** Release (optimized)
- **Total iterations:** 30,000 (6 tests x 5,000 cases)
- **Runtime:** 0.32s (release mode)

### Results

| Test | Target Module | Invariant | Cases | Result |
|:-----|:-------------|:----------|:------|:-------|
| `proptest_fuzz_filter_deep` | filter parser | Parse never panics on deeply nested expressions | 5,000 | PASS |
| `proptest_fuzz_filter_arbitrary_string` | filter parser | Parse never panics on arbitrary UTF-8 strings | 5,000 | PASS |
| `proptest_fuzz_persistence` | persistence | Deserialize never panics on arbitrary bytes | 5,000 | PASS |
| `proptest_fuzz_persistence_roundtrip` | persistence | Save/load roundtrip preserves vector count | 5,000 | PASS |
| `proptest_fuzz_hnsw_search` | HNSW graph | Search never panics with random graph topology | 5,000 | PASS |
| `proptest_fuzz_hnsw_insert_search` | HNSW graph | Insert+search roundtrip: results <= k | 5,000 | PASS |

### Coverage Analysis

| Cargo-Fuzz Target | Proptest Equivalent | Coverage |
|:------------------|:-------------------|:---------|
| `filter_deep` | `proptest_fuzz_filter_deep` + `proptest_fuzz_filter_arbitrary_string` | Full |
| `persistence` | `proptest_fuzz_persistence` + `proptest_fuzz_persistence_roundtrip` | Full |
| `hnsw_search` | `proptest_fuzz_hnsw_search` + `proptest_fuzz_hnsw_insert_search` | Full |
| `sparse_vector` | (covered by existing proptest in `src/sparse/vector.rs`) | Partial |
| `flat_index` | (covered by existing proptest in `src/index/flat.rs`) | Partial |

---

## Crashes Found

**None.** Zero crashes across 30,000 iterations.

---

## Recommendations

1. **Run libFuzzer targets in CI** — The GitHub Actions CI runs on Ubuntu where `cargo fuzz run` works. Add a scheduled nightly job for extended fuzzing.
2. **Increase local proptest cases** — Developers can run `PROPTEST_CASES=50000 cargo test --test proptest_fuzz --release` for deeper local coverage.
3. **Keep proptest_fuzz.rs in sync** — When adding new fuzz targets, add corresponding proptest simulations for cross-platform coverage.

---

## Verdict

**APPROVED** — All critical subsystems (filter parser, persistence, HNSW search) pass fuzzing with zero crashes. The proptest-based approach provides cross-platform coverage while cargo-fuzz targets remain available for Linux CI.

---

**END OF FUZZ CAMPAIGN REPORT**
