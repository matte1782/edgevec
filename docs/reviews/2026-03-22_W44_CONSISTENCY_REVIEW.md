# HOSTILE_REVIEWER: Consistency Review (Round 2) -- Week 44

**Date:** 2026-03-22
**Artifact Scope:** 12 cross-referenced documents (full W44 artifact set)
**Review Type:** STRICT CONSISTENCY-FOCUSED (Second Round)
**Prior Review:** `docs/reviews/2026-03-21_W44_HOSTILE_REVIEW.md` (1 critical, 4 major, 10 minor)
**Status:** See verdict below

---

## Part 1: Fix Verification

All 5 must-fix and should-fix items from the Round 1 hostile review were verified against the current file contents.

### C1: ROADMAP title v6.1 -> v7.0

- **Location:** `docs/planning/ROADMAP.md` line 1
- **Current content:** `# EdgeVec Roadmap v7.0`
- **Verdict:** FIXED. Title matches revision history entry at line 696.

### M1: Limitations note added to WEBGPU_SPIKE.md Section 5

- **Location:** `docs/research/WEBGPU_SPIKE.md` lines 162-162
- **Current content:** "Methodological note: The WASM SIMD128 column uses first-party EdgeVec measurements. The WebGPU column uses estimates derived from published third-party benchmarks..."
- **Verdict:** FIXED. The paragraph explicitly acknowledges the evidence gap and explains why first-party benchmarks were not run.

### M2: FilterExpression import path unified

- **Location:** `pkg/langchain/src/store.ts` line 19 and `pkg/langchain/src/index.ts` line 19
- **store.ts:** `import type { Metadata, SearchOptions, FilterExpression } from "edgevec/edgevec-wrapper.js";`
- **index.ts:** `export { Filter, type FilterExpression } from "edgevec/edgevec-wrapper.js";`
- **Verdict:** FIXED. Both files now import from `edgevec/edgevec-wrapper.js`. The pre-fix `edgevec/filter.js` path no longer appears.

### M3: Test double-cast replaced with helper

- **Location:** `pkg/langchain/tests/store.test.ts` lines 1174-1191
- **Current content:** A `mockFilterExpression` helper function (lines 1181-1191) constructs structurally typed objects. Lines 1174-1179 document why the real `Filter` class cannot be imported in the mocked test context. No `as unknown as` double-casts remain in the FilterExpression tests.
- **Verdict:** FIXED. The helper returns a properly typed object and the rationale is documented.

### M4: Relaxed SIMD metric replaced

- **Location:** `docs/planning/ROADMAP.md` lines 493-498
- **Current content:** v0.10.0 success metrics table now reads:
  - `WebGPU decision | Go/No-Go documented`
  - `Relaxed SIMD decision | Go/No-Go documented`
  - `Research spikes | All exit criteria answered with data`
  - `Bundle size | <600KB (with PQ)`
- **Verdict:** FIXED. The unachievable "Relaxed SIMD speedup 1.5x+" metric is gone.

### Round 1 Minor Fixes

| ID | Claim | Verified |
|:---|:------|:---------|
| m6 | Test renamed from "gt, lt, ge, le" to "between" | CONFIRMED -- `store.test.ts` line 1262: `"accepts between FilterExpression"` |
| m9 | LangChain test count corrected | CONFIRMED -- `ROADMAP.md` line 340: `"128 tests at v0.9.0 release (41 metadata + 62 store + 25 integration); 134 after W44 FilterExpression addition"` |

**Fix verification: 7/7 items confirmed fixed.**

---

## Part 2: NEW Findings -- Cross-Document Consistency

### Critical Issues: 1

- [C1] **v0.9.0 release date conflict between ROADMAP and CHANGELOG**
  - `docs/planning/ROADMAP.md` line 6: `Current Version: v0.9.0 (released 2026-02-27)`
  - `docs/planning/ROADMAP.md` line 33: `v0.9.0 | RELEASED | ... | 2026-02-27`
  - `docs/planning/ROADMAP.md` line 189: `Status: RELEASED (2026-02-27)`
  - `docs/planning/ROADMAP.md` line 368: `v0.9.0 Status: RELEASED (2026-02-27)`
  - `docs/planning/ROADMAP.md` line 674: `v0.9.0 | 2026-02-27 | ...`
  - `CHANGELOG.md` line 25: `## [0.9.0] - 2026-03-07`
  - `CHANGELOG.md` line 1004: `0.9.0 | 2026-03-07 | ...`
  - Criterion violated: "Consistency between documents" -- the same release is dated 2026-02-27 in the ROADMAP (5 occurrences) and 2026-03-07 in the CHANGELOG (2 occurrences). These cannot both be correct.
  - Impact: Anyone checking release history gets contradictory dates. The CHANGELOG is the canonical release record per Keep a Changelog conventions; the ROADMAP appears to use the date the release was cut internally (before W41 documentation/publish sprint), while the CHANGELOG uses the actual publish date. Regardless of which is correct, they must agree.
  - Required Action: Determine the authoritative v0.9.0 release date and align both documents. The CHANGELOG date (2026-03-07) appears to be the actual publish date per the W41 plan (which targets 2026-03-07). If so, all 5 ROADMAP occurrences must be updated.

### Major Issues: 3

- [M1] **ROADMAP Milestone 10.0 task statuses are stale -- show "IN PROGRESS" for completed work**
  - Location: `docs/planning/ROADMAP.md` lines 382, 389, 390
  - Line 382: `Status: IN PROGRESS`
  - Line 389: `WebGPU PoC + Benchmark | 12h | ... | IN PROGRESS`
  - Line 390: `Relaxed SIMD Feasibility | 4h | ... | IN PROGRESS`
  - Evidence: Both spikes are documented as COMPLETE in `WEEKLY_TASK_PLAN.md` (lines 72-73), `GATE_W44_COMPLETE.md` (lines 26-27), and the spikes themselves have full GO/NO-GO verdicts.
  - Criterion violated: "Status tags must accurately reflect current state"
  - Impact: A reader of the ROADMAP would believe the research spikes are still underway. The ROADMAP was updated to v7.0 on 2026-03-17 (start of W44) and the task statuses were never updated as work completed during the week.
  - Required Action: Update the three "IN PROGRESS" statuses to reflect actual completion. The Milestone 10.0 status should indicate that W44 deliverables are complete; remaining milestones (10.1-10.5) are still CONDITIONAL/PLANNED.

- [M2] **WEBGPU_SPIKE.md status tag `[PROPOSED]` not updated to `[APPROVED]`**
  - Location: `docs/research/WEBGPU_SPIKE.md` line 6
  - Current content: `**Status:** [PROPOSED]`
  - Evidence: The hostile review (`docs/reviews/2026-03-21_W44_HOSTILE_REVIEW.md` line 56) verdict is "APPROVED with minor observations." `GATE_W44_COMPLETE.md` line 26 lists the spike as COMPLETE.
  - Criterion violated: Project convention Section 8.2 of `.claude/CLAUDE.md`: `[PROPOSED]` means "Ready for hostile review", `[APPROVED]` means "Passed hostile review". The document passed review but still carries the pre-review tag.
  - Required Action: Update line 6 to `**Status:** [APPROVED]`.

- [M3] **RELAXED_SIMD_SPIKE.md status tag `[PROPOSED]` not updated to `[APPROVED]`**
  - Location: `docs/research/RELAXED_SIMD_SPIKE.md` line 6
  - Current content: `**Status:** [PROPOSED]`
  - Evidence: The hostile review (`docs/reviews/2026-03-21_W44_HOSTILE_REVIEW.md` line 88) verdict is "APPROVED." `GATE_W44_COMPLETE.md` line 27 lists the spike as COMPLETE.
  - Criterion violated: Same as M2 -- status tag convention.
  - Required Action: Update line 6 to `**Status:** [APPROVED]`.

### Minor Issues: 4

- [m1] **WEEKLY_TASK_PLAN Day 5/6 items still marked PENDING despite GATE_W44 marking sprint as COMPLETE**
  - Location: `docs/planning/weeks/week_44/WEEKLY_TASK_PLAN.md` lines 45-50
  - Line 45: `W44.5a-d: Review all artifacts -- **PENDING**`
  - Line 46: `W44.5e: Triage findings -- **PENDING**`
  - Line 47: `W44.5f: Update CHANGELOG -- **PENDING**`
  - Line 49: `W44.6a-d: Fix findings, commit -- **PENDING**`
  - Evidence: The hostile review was completed (`docs/reviews/2026-03-21_W44_HOSTILE_REVIEW.md`), the CHANGELOG was updated (`CHANGELOG.md` [Unreleased] section), and `GATE_W44_COMPLETE.md` exists and lists all deliverables. The Deliverables table at lines 79-80 also shows "Hostile review of all artifacts | PENDING" and "Updated CHANGELOG.md | PENDING".
  - Impact: Low. The plan is internally contradictory -- the header says `[APPROVED]` and GATE_W44 says COMPLETE, but 6 line items and 2 deliverable rows still say PENDING. A reader would not know whether these tasks were actually done.

- [m2] **GATE_W44_COMPLETE.md does not acknowledge unfixed minor items from Round 1**
  - Location: `.claude/GATE_W44_COMPLETE.md` lines 13-21
  - Description: The "Fixed Issues" section lists 7 items (C1, M1-M4, m6, m9). The Round 1 review had 10 minor issues total (m1-m10). Of the remaining 8 unfixed minors (m1, m2, m3, m4, m5, m7, m8, m10), none are acknowledged as "accepted/deferred" in the gate file.
  - Impact: Very low. These were all "NICE TO FIX (Optional)" items. But best practice is for the gate file to explicitly state which optional items were accepted as-is.

- [m3] **GATE_W43_COMPLETE.md test breakdown uses "62 store" but Milestone 9.4 in ROADMAP also says "62 store"**
  - Location: `.claude/GATE_W43_COMPLETE.md` line 15 vs `docs/planning/ROADMAP.md` line 340
  - Description: Both correctly say 62 store tests at v0.9.0 release. This is internally consistent. No issue -- confirmed clean.
  - **Retracted.** This is actually consistent. Removing from findings.

- [m3] **Competitive analysis table in ROADMAP line 652 still shows "Hybrid Search" with a future emoji for v0.9.0**
  - Location: `docs/planning/ROADMAP.md` line 652
  - Content: `| Hybrid Search | (coming soon emoji) v0.9.0 |`
  - Evidence: v0.9.0 is released with hybrid search implemented (ROADMAP line 186 says COMPLETE). The competitive analysis table was not updated to show Hybrid Search as shipped.
  - Impact: Low. Cosmetic inconsistency in a reference table.

- [m4] **index.ts version comment says `@version 0.1.0` -- no inconsistency currently but will drift**
  - Location: `pkg/langchain/src/index.ts` line 5
  - Content: `* @version 0.1.0`
  - Evidence: `pkg/langchain/package.json` also says `"version": "0.1.0"`. Currently consistent. No issue.
  - **Retracted.** This is consistent. Removing from findings.

---

## Part 3: Test Count Verification

### Claimed: 41 + 68 + 25 = 134

| File | `it()` count | Expected |
|:-----|:-------------|:---------|
| `pkg/langchain/tests/metadata.test.ts` | 41 | 41 |
| `pkg/langchain/tests/store.test.ts` | 68 | 68 |
| `pkg/langchain/tests/integration.test.ts` | 25 | 25 |
| **Total** | **134** | **134** |

**Verified.** All test counts match across GATE_W44, ROADMAP, and CHANGELOG.

### Pre-W44 count: 41 + 62 + 25 = 128

The 6 new FilterExpression tests (lines 1193-1315 of `store.test.ts`) account for the 62 -> 68 increase. GATE_W43 correctly shows 128. ROADMAP correctly shows "128 at v0.9.0 release... 134 after W44." Consistent.

---

## Part 4: GO/NO-GO Criteria Cross-Check

### WebGPU GO/NO-GO

| Source | Verdict | Consistent? |
|:-------|:--------|:------------|
| `WEBGPU_SPIKE.md` Section 6 | NO-GO | -- |
| `WEEKLY_TASK_PLAN.md` line 56 | NO-GO | YES |
| `GATE_W44_COMPLETE.md` line 48 | NO-GO | YES |
| `ROADMAP.md` line 396 (criteria listed) | Criteria match spike | YES |
| `CHANGELOG.md` line 20 | NO-GO | YES |

### Relaxed SIMD GO/NO-GO

| Source | Verdict | Consistent? |
|:-------|:--------|:------------|
| `RELAXED_SIMD_SPIKE.md` Section 7 | NO-GO | -- |
| `WEEKLY_TASK_PLAN.md` line 61 | NO-GO | YES |
| `GATE_W44_COMPLETE.md` line 49 | NO-GO | YES |
| `ROADMAP.md` line 401 (criteria listed) | Criteria match spike | YES |
| `CHANGELOG.md` line 21 | NO-GO | YES |

**All GO/NO-GO verdicts are consistent across all 5 reference points for each spike.**

---

## Part 5: Date Consistency

| Event | WEEKLY_TASK_PLAN | GATE Files | ROADMAP | CHANGELOG |
|:------|:-----------------|:-----------|:--------|:----------|
| W44 start | 2026-03-17 | -- | 2026-03-17 (ROADMAP date field) | -- |
| W44 end | 2026-03-22 | GATE_W44: 2026-03-22 | -- | -- |
| W43 gate | -- | GATE_W43: 2026-03-17 | -- | -- |
| WebGPU spike date | 2026-03-17 (Day 1) | -- | -- | -- |
| Relaxed SIMD spike date | 2026-03-19 (Day 3) | -- | -- | -- |
| Hostile review date | 2026-03-21 (Day 5) | -- | -- | -- |
| v0.9.0 release | -- | -- | **2026-02-27** | **2026-03-07** |

The v0.9.0 date conflict (C1) is the only date inconsistency found.

---

## Consolidated Findings

### Critical Issues: 1

| ID | Description | Files Affected |
|:---|:------------|:---------------|
| C1 | v0.9.0 release date: ROADMAP says 2026-02-27, CHANGELOG says 2026-03-07 | `ROADMAP.md` (5 occurrences), `CHANGELOG.md` (2 occurrences) |

### Major Issues: 3

| ID | Description | Files Affected |
|:---|:------------|:---------------|
| M1 | ROADMAP Milestone 10.0 task statuses stale ("IN PROGRESS" for completed work) | `ROADMAP.md` lines 382, 389, 390 |
| M2 | WEBGPU_SPIKE.md status tag still `[PROPOSED]`, should be `[APPROVED]` | `WEBGPU_SPIKE.md` line 6 |
| M3 | RELAXED_SIMD_SPIKE.md status tag still `[PROPOSED]`, should be `[APPROVED]` | `RELAXED_SIMD_SPIKE.md` line 6 |

### Minor Issues: 2

| ID | Description | Files Affected |
|:---|:------------|:---------------|
| m1 | WEEKLY_TASK_PLAN Day 5/6 items + 2 deliverable rows still say PENDING | `WEEKLY_TASK_PLAN.md` lines 45-50, 79-80 |
| m2 | GATE_W44 does not acknowledge disposition of 8 unfixed optional items from R1 | `GATE_W44_COMPLETE.md` |

---

## VERDICT

```
+---------------------------------------------------------------------+
|   HOSTILE_REVIEWER: REJECT                                          |
|                                                                     |
|   Review: W44 Consistency (Round 2)                                 |
|   Artifacts Reviewed: 12                                            |
|                                                                     |
|   Round 1 Fix Verification: 7/7 CONFIRMED                          |
|                                                                     |
|   NEW Critical Issues: 1                                            |
|   NEW Major Issues: 3                                               |
|   NEW Minor Issues: 2                                               |
|                                                                     |
|   Disposition: REJECTED -- 1 new critical issue (date conflict)     |
|   blocks approval. 3 major issues (stale status tags) compound.     |
|                                                                     |
+---------------------------------------------------------------------+
```

---

## Required Actions Before Resubmission

### MUST FIX (Blocking)

1. [ ] **C1:** Determine the authoritative v0.9.0 release date. If the CHANGELOG date (2026-03-07) is correct, update all 5 occurrences in `docs/planning/ROADMAP.md` (lines 6, 33, 189, 368, 674). If 2026-02-27 is correct, update `CHANGELOG.md` lines 25 and 1004.

### SHOULD FIX (Non-blocking but expected)

2. [ ] **M1:** Update `docs/planning/ROADMAP.md` Milestone 10.0 status (line 382) and task statuses (lines 389-390) from "IN PROGRESS" to reflect completion of the W44 research spikes.
3. [ ] **M2:** Update `docs/research/WEBGPU_SPIKE.md` line 6 from `[PROPOSED]` to `[APPROVED]`.
4. [ ] **M3:** Update `docs/research/RELAXED_SIMD_SPIKE.md` line 6 from `[PROPOSED]` to `[APPROVED]`.

### NICE TO FIX (Optional)

5. [ ] **m1:** Update `WEEKLY_TASK_PLAN.md` Day 5/6 items and deliverable table rows from PENDING to DONE/COMPLETE.
6. [ ] **m2:** Add a "Deferred Items" section to `GATE_W44_COMPLETE.md` listing the 8 unfixed optional items from R1 as accepted-as-is.

---

## Resubmission Process

1. Address ALL items in "MUST FIX"
2. Address ALL items in "SHOULD FIX"
3. Tag updated artifacts with `[REVISED]`
4. Resubmit via `/review W44-consistency-revised`

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-03-22*
*Verdict: REJECTED*
*Review Type: Consistency (Round 2)*
