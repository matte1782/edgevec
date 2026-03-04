# Hostile Review: W46 Day 2 — ADC Scan + Encode Pipeline

**Date:** 2026-03-28
**Reviewer:** HOSTILE_REVIEWER
**Artifact:** W46 Day 2 uncommitted diff (product.rs +342, mod.rs +16, ci.yml +25)
**Author:** RUST_ENGINEER
**Type:** Code

---

## Verdict: CONDITIONAL GO → GO (after M1 fix)

| Severity | Count | Details |
|:---------|:------|:--------|
| Critical | 0 | — |
| Major | 1 | M1: NaN sort doc → FIXED |
| Minor | 2 | m1: PartialEq only (correct), m2: encode_batch short-circuits (documented) |

---

## Findings

### Major (FIXED)

- **[M1] NaN sort stability in `scan_topk`** — `partial_cmp` fallback `unwrap_or(Equal)` means NaN sorts arbitrarily. Missing documentation. **Fix:** Added `# NaN Handling` section to `scan_topk` doc comment. **Status:** FIXED.

### Minor (ACCEPTED AS-IS)

- **[m1]** `PqSearchResult` derives `PartialEq` but not `Eq` — correct because `f32` is not `Eq`. No action needed.
- **[m2]** `encode_batch` short-circuits on first error — documented behavior ("first error encountered"). Acceptable.

---

## Verification Results

| Check | Result |
|:------|:-------|
| `cargo test --lib` | 1008 passed, 0 failed |
| `cargo clippy -- -D warnings` | 0 warnings |
| `cargo fmt --check` | Clean |
| Doc tests (quantization) | 23 passed |
| `unsafe` blocks | 0 |
| `unwrap()` in production | 0 (1 in test only) |
| `panic!` macros | 0 |

## New Code Summary

| Item | Location | Tests |
|:-----|:---------|:------|
| `PqSearchResult` struct | product.rs:135-146 | Used by scan_topk tests |
| `DistanceTable::scan_topk()` | product.rs:218-263 | 5 tests |
| `PqCodebook::encode_batch()` | product.rs:447-471 | 3 tests |
| Integration pipeline (1K vectors) | product.rs:1215-1270 | 1 test |
| mod.rs PQ doc example | mod.rs:38-49 | Compiles in doc test |
| `PqSearchResult` re-export | mod.rs:90 | — |
| CI Miri skip expansion | ci.yml | hybrid, product, hnsw |

## Plan Compliance

- W46.2a-2c: encode/distance_table/ADC completed in Day 1 (ahead of schedule)
- W46.2d: scan_topk — COMPLIANT
- W46.2e: error handling clean — COMPLIANT
- Bonus: encode_batch and integration pipeline test (Day 3 prep)

---

**HOSTILE_REVIEWER: GO — Proceed to Day 3.**
