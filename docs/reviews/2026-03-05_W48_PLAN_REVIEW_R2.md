# HOSTILE_REVIEWER: Review Round 2 -- Week 48 Plan [REVISED]

**Date:** 2026-03-05
**Artifact:** `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` + `DAY_1_TASKS.md` through `DAY_6_TASKS.md`
**Author:** PLANNER
**Round:** 2 (following R1 REJECTION: 2C/5M/7m)
**Status:** CONDITIONAL GO

---

## Review Intake

Artifact: Week 48 Plan [REVISED] (Weekly + 6 Daily Files)
Author: PLANNER
Date Submitted: 2026-03-05
Type: Plan
Round: 2 (resubmission after R1 REJECT)

---

## R1 Fix Verification

Every R1 finding is verified below. Status: FIXED, STILL BROKEN, or PARTIALLY FIXED.

### Critical Issues

| ID | Finding | Status | Evidence |
|:---|:--------|:-------|:---------|
| C1 | Score scale mismatch -- additive boost on incomparable distance scales | **FIXED** | All files now use multiplicative formula: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)`. Zero instances of `vector_similarity`, `final_score`, or additive `distance -= boost` remain across all 7 files. The formula is documented in WEEKLY_TASK_PLAN.md lines 116, 262, 371; DAY_1_TASKS.md lines 51, 152, 163, 182, 201; DAY_2_TASKS.md line 142; DAY_5_TASKS.md line 62. Scale-independence rationale is explicit. |
| C2 | Oversample formula contradiction (`k.max(50)` vs `k*3`) | **FIXED** | Unified to `(k * 3).max(50).min(500)` in WEEKLY_TASK_PLAN.md line 116, DAY_1_TASKS.md lines 142, 146, 200. No instance of bare `k.max(50)` without `* 3` survives. Rationale documented in DAY_1 lines 143-145. |

### Major Issues

| ID | Finding | Status | Evidence |
|:---|:--------|:-------|:---------|
| M1 | Blog citations unverified | **FIXED** | Full bibliographic references provided in WEEKLY_TASK_PLAN.md lines 18-19 (Xu et al. 2024, arXiv:2404.17723; Palmonari 2025, UNIMIB/Expert.AI) and DAY_5_TASKS.md lines 58-59. Acceptance criteria at DAY_5 line 132 include full reference verification. |
| M2 | MetadataValue serde mismatch for WASM | **FIXED** | DAY_2_TASKS.md lines 61-66 explicitly document BoostConfig as a SEPARATE struct with bare JSON values, not MetadataValue adjacently-tagged format. Acceptance criteria at DAY_2 lines 82-83 and WEEKLY_TASK_PLAN.md line 297 both include BoostConfig format check. |
| M3 | data.json size -- no precision control | **FIXED** | DAY_3_TASKS.md line 120 uses `round(float(x), 6)` for document embeddings and line 132 for query embeddings. DAY_3 acceptance criteria at line 227 include "All embeddings rounded to 6 decimal places." |
| M4 | No index build time criterion | **FIXED** | DAY_4_TASKS.md lines 16-17 specify loading indicator + "< 3 seconds in Chrome" build time. WEEKLY_TASK_PLAN.md line 300 includes "Index build time for 1000 vectors at 384D < 3 seconds in Chrome" in sprint acceptance criteria. DAY_4 smoke test at line 174 also verifies. |
| M5 | Distance vs similarity language inconsistency | **FIXED** | Zero instances of `vector_similarity` or `final_score` found in any W48 file. All formulas use distance language (`final_distance`, `raw_distance`, `search_result.distance *= ...`). Anti-error checklist at WEEKLY_TASK_PLAN.md line 371 uses correct distance language. |

### Minor Issues

| ID | Finding | Status | Evidence |
|:---|:--------|:-------|:---------|
| m1 | StringArray vs String matching | **FIXED** | DAY_1_TASKS.md lines 88 and 111 document StringArray.contains() matching. Test list includes `test_boost_string_array_contains` at line 223 with explicit description of the demo use case. |
| m2 | Hours total wrong (~42h claimed, ~35h actual) | **FIXED** | WEEKLY_TASK_PLAN.md line 47 now says "~35h across 6 days (~5.9h/day)" with per-day breakdown matching the daily files (8.5+5+3.6+7+6.5+4.5=35.1h). |
| m3 | `import os` misplaced inside `__main__` | **FIXED** | DAY_3_TASKS.md line 73 has `import os` at module level, before `def main():`. The second `import os` at line 215 is in a separate standalone verification script (W48.3b), which is correct. |
| m4 | "(if modified)" on WASM export in Day 6 | **FIXED** | DAY_6_TASKS.md line 149 now reads `git add src/wasm/mod.rs` with no "(if modified)" qualifier. |
| m5 | NaN validation at construction | **FIXED** | DAY_1_TASKS.md lines 74-78 define `BoostError::NonFiniteWeight` enum. Line 82-83 show `MetadataBoost::new()` returns `Result<Self, BoostError>` with explicit `!weight.is_finite()` check. Acceptance criteria at line 109 confirm. |
| m6 | Blog comparison table latency | **FIXED** | DAY_5_TASKS.md line 104 now says `<1ms (1K docs)` instead of `<10ms (1K docs)`. |
| m7 | .gitignore verification | **FIXED** | DAY_6_TASKS.md line 162 includes `grep "data.json" .gitignore || echo "ERROR: data.json not in .gitignore!"` in the commit sequence. |

**R1 Verification Summary: 14/14 findings FIXED.**

---

## New Findings (R2 Attack)

### Critical Issues: 0

### Major Issues: 0

### Minor Issues: 2

- **[m1] Comparison table Qdrant/Weaviate latency numbers are apples-to-oranges and should be flagged as such**
  - Location: DAY_5_TASKS.md line 104
  - Evidence: The table compares EdgeVec at `<1ms (1K docs)` with Qdrant at `~5ms (remote)` and Weaviate at `~10ms (remote)`. The EdgeVec number is local in-process latency for 1K docs. The Qdrant/Weaviate numbers include network round-trip time (`(remote)`), which adds 1-10ms regardless of engine speed. Qdrant's in-process search at 1K docs would also be sub-millisecond. The `(remote)` annotation partially mitigates this, but a savvy reader will still note the comparison is structurally unfair.
  - Impact: Low -- the `(remote)` tag is present, and the table's main point (in-browser vs server) is valid. But the hostile review at Day 5 (W48.5d) should specifically attack this row.
  - Required Action: Add a footnote or note in the blog comparison section acknowledging that Qdrant/Weaviate latency includes network overhead and the comparison is about deployment model, not raw engine speed.

- **[m2] Day 1 code snippet has `unwrap_or(std::cmp::Ordering::Equal)` in sort_by -- this is NaN handling, but it silently treats NaN distances as equal rather than pushing them to the end**
  - Location: DAY_1_TASKS.md line 169
  - Evidence: `result.results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))`. If a distance becomes NaN (which should be impossible after the clamp, but defense-in-depth matters), NaN items will be interleaved arbitrarily among real results. A more defensive approach would push NaN to the end (`Ordering::Greater`). This is a code snippet in a plan, not production code, so the impact is purely advisory.
  - Impact: Low -- the `.clamp(-1.0, 0.99)` on boost_factor and the constructor NaN rejection make NaN distances practically impossible. This is defense-in-depth territory.
  - Required Action: Note for the Day 2 hostile review (W48.2d) to check the actual implementation's NaN sort behavior. If NaN distances are possible, push them to end with `Ordering::Greater`.

---

## Cross-Document Consistency Check

| Check | Result |
|:------|:-------|
| Oversample formula identical in WEEKLY + DAY_1 | PASS -- `(k * 3).max(50).min(500)` in both |
| Boost formula identical in WEEKLY + DAY_1 + DAY_2 + DAY_5 | PASS -- multiplicative formula consistent |
| 11 test names match between WEEKLY (line 117) and DAY_1 (lines 214-224) | PASS -- all 11 names identical |
| BoostConfig serde documented in WEEKLY + DAY_2 | PASS -- both say bare JSON values, not adjacently-tagged |
| Hours sum (35.1h) matches WEEKLY claim (~35h) | PASS -- 0.1h rounding difference is acceptable |
| Day 4 hours: 0.5+5+1+0.5 = 7h | PASS -- matches Day 4 total |
| Index build time criterion in WEEKLY + DAY_4 | PASS -- both say < 3 seconds in Chrome |
| Blog citations in WEEKLY + DAY_5 | PASS -- both have identical full references |
| NaN/BoostError documented in WEEKLY + DAY_1 | PASS -- both specify `new() -> Result<Self, BoostError>` |
| Track independence (A/B/C) documented | PASS -- WEEKLY lines 93-94, critical path diagram |
| Halt condition for hostile review NO-GO | PASS -- WEEKLY line 133, DAY_2 line 198 |
| `.gitignore` verification in commit sequence | PASS -- DAY_6 line 162 |

---

## Mathematical Sanity Check: Boost Formula

The formula `final_distance = raw_distance * (1.0 - boost_factor)` with `boost_factor = sum(boosts).clamp(-1.0, 0.99)`:

| Scenario | boost_factor | Multiplier | Effect |
|:---------|:-------------|:-----------|:-------|
| k=10, oversample | `(10*3).max(50).min(500)` = 50 | N/A | Correct |
| k=100, oversample | `(100*3).max(50).min(500)` = 300 | N/A | Correct |
| k=200, oversample | `(200*3).max(50).min(500)` = 500 | N/A | Capped correctly |
| Single match, weight=0.3 | 0.3 | 0.7 | 30% distance reduction -- reasonable |
| Two matches, weights=0.3+0.3 | 0.6 | 0.4 | 60% reduction -- aggressive but clamped |
| Negative weight=-0.2, match | -0.2 | 1.2 | 20% distance increase (penalty) -- correct |
| Extreme: weights sum to 2.0 | clamped to 0.99 | 0.01 | Distance reduced to 1% -- this is very aggressive but the clamp prevents zero/negative |
| Extreme: negative sum to -3.0 | clamped to -1.0 | 2.0 | Distance doubled -- correct upper bound |

The formula is mathematically sound. The 0.99 upper clamp prevents distance from reaching zero, which would make a result infinitely "good." The -1.0 lower clamp prevents distance from more than doubling. Both bounds are reasonable.

---

## Verdict

```
+---------------------------------------------------------------------+
|   HOSTILE_REVIEWER: CONDITIONAL GO                                   |
|                                                                     |
|   Artifact: Week 48 Plan [REVISED] (Weekly + 6 Daily Files)        |
|   Author: PLANNER                                                   |
|   Round: 2                                                          |
|                                                                     |
|   R1 Findings Verified: 14/14 FIXED                                |
|                                                                     |
|   New Critical Issues: 0                                            |
|   New Major Issues: 0                                               |
|   New Minor Issues: 2                                               |
|                                                                     |
|   Disposition:                                                      |
|   - CONDITIONAL GO: All R1 findings resolved. 2 new minors are     |
|     advisory and do not block implementation. Both can be addressed |
|     during execution (m1 at blog writing, m2 at mid-week review).  |
|                                                                     |
+---------------------------------------------------------------------+
```

**CONDITIONAL GO**

All 14 R1 findings (2 Critical, 5 Major, 7 Minor) have been verified as FIXED. The revised plan demonstrates thorough, consistent corrections across all 7 files. The multiplicative boosting formula is mathematically sound and scale-independent. Cross-document consistency is solid.

Two new minor findings are noted for action during execution, not as blocking items:
1. Blog comparison table should acknowledge latency measurement difference (network vs local)
2. NaN sort behavior in code snippet should use `Ordering::Greater` not `Ordering::Equal`

---

## Required Actions (Non-Blocking)

1. [m1] When writing the blog post (Day 5, W48.5a): add a note to the comparison table acknowledging Qdrant/Weaviate latency includes network overhead
2. [m2] During mid-week hostile review (Day 2, W48.2d): verify the actual `sort_by` implementation uses `Ordering::Greater` for NaN distances as defense-in-depth

---

## UNLOCK

Week 48 implementation may proceed. Day 1 execution is unlocked.

Gate status: Plan [APPROVED] (Conditional -- 2 minor advisory items tracked for execution phase).

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-03-05*
*Round: 2*
*Verdict: CONDITIONAL GO*
