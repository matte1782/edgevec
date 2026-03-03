# HOSTILE_REVIEWER: Review -- Week 45 Plan

**Date:** 2026-03-24
**Artifact:** Week 45 WEEKLY_TASK_PLAN.md + DAY_1 through DAY_6 task files
**Author:** PLANNER
**Status:** APPROVED (all conditions met — fixes applied 2026-03-24)

---

## HOSTILE_REVIEWER: Review Intake

Artifact: W45 Weekly Task Plan (7 files)
Author: PLANNER
Date Submitted: 2026-03-24
Type: Plan

---

## Executive Summary

The Week 45 plan is a well-structured sprint with three parallel workstreams (LangChain v0.2.0 polish, PQ research pull-forward, API stability audit). The plan correctly capitalizes on the W44 NO-GO decisions to redirect freed capacity. However, the review identifies **1 critical issue** (ROADMAP scheduling conflict), **4 major issues** (hour math discrepancy, missing rollback plan, test count assumptions, semver justification gap), and **5 minor issues**. The plan is approvable after the critical and major items are addressed.

---

## Findings

### Critical Issues (BLOCKING)

- **[C1] ROADMAP Milestone 10.4 schedules PQ research for Week 46, not Week 45 -- plan pulls forward without updating ROADMAP**
  - Description: ROADMAP.md Milestone 10.4 (line 453-468) explicitly states "Phase 1: Research (8h, Week 46)". The W45 plan pulls this forward to Days 3-4 but does not include a task to update ROADMAP.md to reflect this schedule change.
  - Evidence: `docs/planning/ROADMAP.md` line 458: "Phase 1: Research (8h, Week 46)". `WEEKLY_TASK_PLAN.md` line 15: "Pull forward PQ research -- Product Quantization was scheduled for W46".
  - Impact: ROADMAP becomes stale and inconsistent with actual execution. This is the exact kind of document drift that caused 2 of the 6 fixes in the W44 Round 2 remediation.
  - Criterion violated: "Consistency between documents" (HOSTILE_GATE_CHECKLIST)
  - Required Action: Add a task (suggest W45.2c.1 or W45.5e scope expansion) to update ROADMAP.md Milestone 10.4 to reflect the W45/W46 split. Milestone 10.4 Phase 1 should say "Week 45-46" or "Week 45 (pulled forward)".

---

### Major Issues (MUST FIX)

- **[M1] Hour totals do not add up -- WEEKLY_TASK_PLAN claims ~34h but day files sum to ~33h, and the 3x ceiling is incorrectly applied**
  - Description: The WEEKLY_TASK_PLAN states "Optimistic total: 24h", "3x ceiling: 72h", "Planned: ~34h". The day-by-day totals are: Day 1 = 6h, Day 2 = 4.5h, Day 3 = 6h, Day 4 = 6h, Day 5 = 5h, Day 6 = 5.5h = **33h total**, not 34h. More importantly, the 3x rule states estimates should be multiplied by 3x to get realistic ceiling. If 24h is the optimistic total, planning 34h (1.4x) violates the spirit of the 3x rule -- where is the remaining buffer accounted for?
  - Evidence: WEEKLY_TASK_PLAN.md line 34-36 vs Day 1-6 totals.
  - Criterion violated: "3x rule applied to all estimates" (CLAUDE.md Section 3.3)
  - Required Action: (a) Fix the arithmetic: planned total is 33h, not 34h. (b) Clarify the 3x rule application -- the 72h ceiling is the "disaster scenario" boundary, the 34h planned is the "realistic" target. State this explicitly so there is no ambiguity about whether the sprint is overloaded.

- **[M2] No rollback plan if v0.2.0 npm publish introduces a regression discovered post-publish**
  - Description: The risk register (line 147) states "v0.2.0 breaks existing users -- LOW -- Additive changes only; union types are backward-compatible". The mitigation says nothing about what to do IF it happens. There is no `npm unpublish` or `npm deprecate` procedure, no v0.2.1 hotfix plan, and no post-publish smoke test defined.
  - Evidence: WEEKLY_TASK_PLAN.md risk register line 147; DAY_6_TASKS.md W45.6c has pre-publish checklist but no post-publish verification beyond "verify install works".
  - Criterion violated: "Are risks identified? Are mitigations defined?" (HOSTILE_REVIEWER attack vector)
  - Required Action: Add a post-publish smoke test to W45.6c (e.g., create a scratch project, install edgevec-langchain@0.2.0, import and verify types compile). Add a rollback procedure to the risk register: if regression found, publish 0.2.1 patch or `npm deprecate edgevec-langchain@0.2.0`.

- **[M3] LangChain test count target is assumed, not derived -- "142+" is unverified**
  - Description: WEEKLY_TASK_PLAN line 133 and DAY_1_TASKS line 17 both claim "142+" as the post-Day-1 test count. The current count is 134 (per GATE_W44_COMPLETE.md line 63). Day 1 adds 8+ new tests. 134 + 8 = 142. But the "8+" is a minimum -- some edge case tests in W45.1a list 5 tests, W45.1b lists 3, W45.1c lists 4, W45.1d lists 3 = **15 test cases total**. The plan says "8+ new tests" which implies some test cases share a single `it()` block, but this is nowhere documented.
  - Evidence: DAY_1_TASKS.md lists 15 test cases across W45.1a-1d. Sprint acceptance says "8+ new tests" (line 128). GATE_W44_COMPLETE.md says current count is 134.
  - Criterion violated: "Every task has binary pass/fail criteria" -- the "8+" threshold is ambiguous when 15 test cases are defined.
  - Required Action: Clarify whether "8+ new tests" means 8+ `it()` blocks (with some test cases grouped) or 8+ test cases. If grouping is intended, document which cases are grouped. Alternatively, raise the target to match the 15 test cases listed, or reduce the test case list to match 8.

- **[M4] v0.2.0 semver bump justification is incomplete -- the plan does not verify that FilterExpression is a purely additive (minor) change**
  - Description: DAY_2_TASKS W45.2d states the version bump is 0.1.0 to 0.2.0 (semver minor). The plan asserts "Additive changes only" but does not verify this claim. The `declare FilterType: string | FilterExpression` in `store.ts` line 118 changes the type signature of the filter parameter. For TypeScript users who previously narrowed `FilterType` to `string`, the union type `string | FilterExpression` is additive at the value level but potentially breaking at the type level (e.g., a user who does `const f: EdgeVecStore["FilterType"] = getFilter()` where `getFilter()` returns `string` -- this would now require accepting `FilterExpression` too). The plan should explicitly verify backward compatibility.
  - Evidence: `pkg/langchain/src/store.ts` line 118: `declare FilterType: string | FilterExpression;`. DAY_2_TASKS.md line 88: version bump described as minor.
  - Criterion violated: "Is there any backward-incompatible change hidden in v0.2.0?" (DAY_5_TASKS hostile question #3)
  - Required Action: Add a verification step to W45.2d or W45.2e: confirm that existing code using `similaritySearchVectorWithScore(query, k, "string filter")` compiles without changes against v0.2.0 types. Document the type widening as a known non-breaking change in the CHANGELOG.

---

### Minor Issues (SHOULD FIX)

- **[m1] DAY_1_TASKS W45.1c test case #4 references `Filter.not()` with conditional "if `Filter.not` exists"**
  - Description: The plan includes a test case for `Filter.not(Filter.eq("deleted", true))` with a hedge: "if `Filter.not` exists". `Filter.not` DOES exist in `pkg/filter.js` (line 439). The conditional language suggests the author did not verify this. The test should be stated unconditionally.
  - Evidence: DAY_1_TASKS.md line 66: "if `Filter.not` exists". `pkg/filter.js` line 439 confirms `Filter.not` exists.
  - Required Action: Remove the conditional hedge. State the test unconditionally.

- **[m2] Day 5 hostile review is self-reviewing PQ research written by META_ARCHITECT + BENCHMARK_SCIENTIST -- conflict of interest is not addressed**
  - Description: Day 3 and Day 4 artifacts are produced by META_ARCHITECT and BENCHMARK_SCIENTIST. Day 5 review is by HOSTILE_REVIEWER. This is correct and proper. However, the Day 5 review document is named `2026-03-28_W45_HOSTILE_REVIEW.md` (line 89) while the current review (this document) is `2026-03-24_W45_PLAN_HOSTILE_REVIEW.md`. The plan should ensure the Day 5 review file name does not collide with this plan review file name.
  - Evidence: DAY_5_TASKS.md line 89 vs this document's filename.
  - Required Action: No collision exists -- `2026-03-28` vs `2026-03-24`. This is a false alarm on closer inspection. However, the Day 5 review covers multiple artifact types in a single review document; consider whether separate review files per artifact category would be more traceable.

- **[m3] DAY_3_TASKS codebook storage calculation (line 112) appears incorrect**
  - Description: Line 112 states "256 centroids * 8 subspaces * 96D * 4 bytes = 768KB per trained codebook". The math: 256 * 8 * 96 * 4 = 786,432 bytes = 768KB. The math is correct. However, the text says "per trained codebook" -- a single PQ configuration has ONE codebook set. The concern is that this could be misread as "each of the 8 subspaces has its own 768KB codebook" (which would be 6MB total). Clarify that 768KB is the total for all 8 subspaces combined.
  - Evidence: DAY_3_TASKS.md line 112-113.
  - Required Action: Rephrase to "Total codebook storage: 256 centroids * 8 subspaces * 96 floats/centroid * 4 bytes/float = 768KB (all subspaces combined)".

- **[m4] WEEKLY_TASK_PLAN critical path diagram (lines 43-53) shows Day 2 and Day 3 as parallel but Day 3 has no dependency on Day 1**
  - Description: The critical path shows Day 1 flows to Day 2, and Day 2 is parallel with Day 3. But Day 3 (PQ research) has zero dependency on Days 1-2 (LangChain work). It could theoretically start on Day 1. The diagram implies a false dependency: that Day 3 must wait until Day 2 starts. This does not affect execution (days are sequential by calendar) but creates misleading critical path analysis.
  - Evidence: WEEKLY_TASK_PLAN.md lines 43-53.
  - Required Action: Acknowledge in the critical path that Day 3 is independent of Days 1-2 and could be reordered without impact. Or simplify: "Days 1-2: LangChain track. Days 3-4: Research track. Day 5: Review. Day 6: Ship."

- **[m5] DAY_6_TASKS commit message (line 82-88) references "142+ langchain tests" but this number should reflect post-Day-1 actual count**
  - Description: The pre-written commit message says "142+ langchain tests". Since this is a template to be used on Day 6, the actual count should be inserted at execution time, not pre-committed in the plan. If Day 1 produces 15 new tests (not 8), the count would be 149, not 142.
  - Evidence: DAY_6_TASKS.md lines 84.
  - Required Action: Change "142+" to "[ACTUAL_COUNT]" or "150+" to account for the actual additions. Or document that the commit message is a template to be updated at execution time.

---

## Cross-Reference Validation

### ROADMAP Alignment

| ROADMAP Item | W45 Plan Coverage | Status |
|:-------------|:------------------|:-------|
| Milestone 10.4: PQ Research Phase 1 (W46) | Pulled forward to W45 Days 3-4 | **CONFLICT** (see C1) |
| Milestone 10.5: LangChain v0.2.0 (W45) | Covered by Days 1-2, 6 | ALIGNED |
| Milestone 10.1: WebGPU (IF GO) | Correctly excluded (NO-GO from W44) | ALIGNED |
| Milestone 10.2: Relaxed SIMD (IF GO) | Correctly excluded (NO-GO from W44) | ALIGNED |
| Milestone 10.3: BM25 | Correctly excluded (no community demand) | ALIGNED |

### Task ID Convention

All task IDs follow the W45.Xa pattern (W45.1a through W45.6e). Consistent and correct.

### Date Alignment

| Day | Planned Date | Day of Week | Correct? |
|:----|:-------------|:------------|:---------|
| 1 | 2026-03-24 | Tuesday | NO -- March 24, 2026 is a Tuesday, not Monday |
| 2 | 2026-03-25 | Wednesday | NO -- plan says Tuesday |
| 3 | 2026-03-26 | Thursday | NO -- plan says Wednesday |
| 4 | 2026-03-27 | Friday | NO -- plan says Thursday |
| 5 | 2026-03-28 | Saturday | NO -- plan says Friday |
| 6 | 2026-03-29 | Sunday | NO -- plan says Saturday |

**NOTE:** March 24, 2026 is actually a **Tuesday** (verified: March 1, 2026 is Sunday, so March 24 = Tuesday). The day-of-week labels in DAY_1_TASKS through DAY_6_TASKS are all shifted by one day. Day 1 says "Monday, Mar 24" but March 24, 2026 is a Tuesday. This is a cosmetic inconsistency -- the dates themselves are self-consistent (sequential from 24 to 29), and execution follows dates not day-of-week labels. **Promoted from minor to informational** since it does not affect execution.

**UPDATE on day-of-week:** I cannot verify the exact day-of-week for March 24, 2026 with certainty from my training data. If March 24, 2026 IS a Monday, then the labels are correct. If it is a Tuesday, then all day-of-week labels are wrong. The planner should verify. Leaving this as informational rather than a finding.

### Deliverable Count Consistency

| WEEKLY_TASK_PLAN Deliverables | Day Files | Match? |
|:------------------------------|:----------|:-------|
| FilterExpression edge case tests | Day 1 | YES |
| FilterExpression usage guide | Day 2 (FILTER_GUIDE.md) | YES |
| PQ Literature Review doc | Day 3 | YES |
| API Stability Audit | Day 4 | YES -- but Day 4 produces 3 files (PQ_BENCHMARK_PLAN, API_SURFACE_INVENTORY, API_STABILITY_AUDIT) while WEEKLY_TASK_PLAN lists 2 |
| Hostile review | Day 5 | YES |
| Gate file | Day 6 | YES |

**Minor discrepancy:** Day 4 produces `PQ_BENCHMARK_PLAN.md` which is not listed in the WEEKLY_TASK_PLAN deliverables table (line 109-119). The WEEKLY_TASK_PLAN lists "docs/research/PRODUCT_QUANTIZATION_LITERATURE.md" (Day 3-4) and "docs/audits/API_STABILITY_AUDIT.md" (Day 4) but omits `PQ_BENCHMARK_PLAN.md` and `API_SURFACE_INVENTORY.md`. This is tracked as part of [m5] pattern but not a separate finding.

---

## Verdict

```
+---------------------------------------------------------------------+
|   HOSTILE_REVIEWER: APPROVED WITH CONDITIONS                        |
|                                                                     |
|   Artifact: Week 45 WEEKLY_TASK_PLAN + DAY_1 through DAY_6         |
|   Author: PLANNER                                                   |
|                                                                     |
|   Critical Issues: 1                                                |
|   Major Issues: 4                                                   |
|   Minor Issues: 5                                                   |
|                                                                     |
|   Disposition:                                                      |
|   - Fix C1 and all M-items before execution begins                  |
|   - Minor items may be fixed during execution                       |
|   - Re-review NOT required if fixes are mechanical                  |
|                                                                     |
+---------------------------------------------------------------------+
```

---

## Required Actions Before Execution

1. [x] **[C1]** [FIXED] Added W45.2f task to update ROADMAP.md Milestone 10.4 schedule; added to deliverables table
2. [x] **[M1]** [FIXED] Corrected hour total to ~33h; clarified 3x rule as disaster ceiling vs realistic target with per-task padding
3. [x] **[M2]** [FIXED] Added post-publish smoke test (scratch project import) + rollback procedure (npm deprecate + 0.2.1 hotfix) to W45.6c and risk register
4. [x] **[M3]** [FIXED] Clarified: 15 new `it()` blocks (5+3+4+3), 149 total tests. Updated all references from "8+" to "15"
5. [x] **[M4]** [FIXED] Added backward compatibility verification step to W45.2d with concrete test (existing string filter must compile)
6. [x] **[m1]** [FIXED] Removed "if Filter.not exists" conditional — confirmed in pkg/filter.js
7. [x] **[m3]** [FIXED] Clarified codebook storage as "all subspaces combined, single codebook set"
8. [x] **[m4]** [FIXED] Redesigned critical path as two independent tracks (LangChain + Research/Audit) with explicit note about no cross-dependencies
9. [x] **[m5]** [FIXED] Changed hardcoded "142+" to "[ACTUAL_COUNT]" placeholder in commit message template

---

## Resubmission Process

Since the findings are mechanical fixes (no structural redesign needed), the plan may proceed after addressing C1 + M1-M4. A full re-review is NOT required -- the author should mark fixes with `[FIXED]` annotations and execution can begin.

If structural changes are needed (e.g., day reordering, scope change), resubmit for hostile review via `/review WEEKLY_TASK_PLAN.md`.

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-03-24*
*Verdict: APPROVED WITH CONDITIONS*
*Agent Version: 2.0.0 (Claude Code)*
