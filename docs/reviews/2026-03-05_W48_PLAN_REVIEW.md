# HOSTILE_REVIEWER: Review -- Week 48 Plan

**Date:** 2026-03-05
**Artifact:** `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` + `DAY_1_TASKS.md` through `DAY_6_TASKS.md`
**Author:** PLANNER
**Status:** REJECTED

---

## Review Intake

Artifact: Week 48 Plan (Weekly + 6 Daily Files)
Author: PLANNER
Date Submitted: 2026-03-05
Type: Plan

---

## Summary

Week 48 proposes a "growth pivot" sprint delivering three deliverables: MetadataBoost API (weighted metadata scoring), in-browser demo with 1000 SQuAD docs, and a positioning blog post. The plan includes 6 daily files, two hostile review gates, and a critical path with three independent tracks.

---

## Findings

### Critical Issues (BLOCKING)

- **[C1] Score scale mismatch in `search_boosted()` formula -- distance and boost weights operate on incomparable scales**
  - Location: `WEEKLY_TASK_PLAN.md` line 114, `DAY_1_TASKS.md` lines 130-133
  - Evidence: The plan specifies `final_score = vector_similarity + sum(weight_i * match_i)` (line 114) but then DAY_1 correctly identifies that EdgeVec uses **distance** (lower = better) and proposes `search_result.distance -= boost`. The problem: `SearchResult.distance` is L2-squared distance (potentially 0 to unbounded), while boost weights are user-specified f32 values with no defined scale relationship. A boost weight of 0.3 means nothing relative to L2-squared distances that could be 0.001 or 50000 depending on vector normalization. For **cosine distance** (0 to 2), a weight of 0.3 is significant. For **L2-squared on unnormalized 384D vectors**, distances can easily be in the hundreds, making 0.3 invisible.
  - Impact: The feature will produce **no visible effect** on many real workloads, or produce **wildly different behavior** depending on the distance metric and vector scale. The blog post and demo will demonstrate a broken feature.
  - Criterion violated: "Correctness of core algorithm" -- boost must be scale-aware or documented as requiring normalized vectors.
  - Required Action: Define how boost weights relate to the distance scale. Options: (a) normalize distances to [0,1] before boosting, (b) multiply boost by mean distance as a scaling factor, (c) explicitly require normalized vectors and document the valid distance range, (d) use a reranking formula that is scale-independent (e.g., rank-based boosting like RRF). This is an **architectural decision** that must be made before implementation begins.

- **[C2] Oversample formula `k.max(50).min(500)` is wrong -- it does not use k*3 as specified in the weekly plan**
  - Location: `DAY_1_TASKS.md` line 121 vs `WEEKLY_TASK_PLAN.md` line 114
  - Evidence: The weekly plan says `oversample = max(k*3, 50)`. DAY_1 code snippet says `let oversample_k = k.max(50).min(500)`. These are completely different formulas. `k.max(50)` with k=10 gives 50. `max(k*3, 50)` with k=10 also gives 50. But `k.max(50)` with k=100 gives 100, while `max(k*3, 50)` with k=100 gives 300. The DAY_1 version also adds `.min(500)` which the weekly plan does not mention.
  - Impact: For k > 50, the DAY_1 code fetches far fewer candidates than the weekly plan specifies, degrading reranking quality. The plan and daily file contradict each other.
  - Criterion violated: "Consistency between documents" -- weekly plan and daily file must agree on the same formula.
  - Required Action: Reconcile the oversample formula. Choose ONE formula and propagate it to both the weekly plan and DAY_1 file. Document the rationale for the chosen bounds.

### Major Issues (MUST FIX)

- **[M1] Blog citations "Palmonari et al. 2025" and "Xu et al. 2024" are unverified and likely fabricated**
  - Location: `WEEKLY_TASK_PLAN.md` line 17, `DAY_5_TASKS.md` line 57
  - Evidence: The plan claims "entity-enhanced retrieval improves RAG accuracy from 0.52 to 0.71 (Palmonari et al. 2025, Xu et al. 2024)." No paper titles, venues, DOIs, or URLs are provided. "Palmonari et al. 2025" is a future date (it is currently 2026-03-05, so a 2025 paper could exist, but the claim is suspiciously specific without a reference). The hostile review for the blog post at Day 5 (W48.5d) lists "Research citations are real and correctly attributed" as an attack vector, but this verification should happen at **planning** stage, not after 3 hours are spent writing a blog post built on potentially fabricated citations.
  - Impact: Publishing a blog post with fabricated citations destroys project credibility. If citations are fake, the blog's central argument collapses.
  - Required Action: Provide full bibliographic references (title, venue, DOI or URL) for both cited papers before the plan is approved. If the papers do not exist, remove the specific accuracy claims and replace with real, verifiable citations, or remove citation claims entirely and rely on intuitive argument.

- **[M2] MetadataValue serialization format is adjacently-tagged (`{"type":"string","value":"ORG"}`) but the WASM boost JSON format expects bare values (`"value": "ORG"`)**
  - Location: `DAY_2_TASKS.md` lines 61-65, `src/metadata/types.rs` lines 83-84
  - Evidence: `MetadataValue` uses `#[serde(tag = "type", content = "value", rename_all = "snake_case")]` which serializes as `{"type": "string", "value": "ORG"}`. The WASM boost JSON format specified in W48.2a is `[{"field": "entity_type", "value": "ORG", "weight": 0.3}]` where `"value": "ORG"` is a bare JSON string, NOT the adjacently-tagged `{"type": "string", "value": "ORG"}` object. The plan proposes converting `serde_json::Value` to `MetadataValue` manually (line 62-65), which is correct but means the `BoostConfig` struct needs its own deserialization -- it cannot use `MetadataValue`'s serde impl directly. This is not explicitly called out as a design risk.
  - Impact: If the implementer naively uses `MetadataValue`'s serde format for the JSON API, the demo JS will need to send `{"field": "entity_type", "value": {"type": "string", "value": "ORG"}, "weight": 0.3}` which is ugly and error-prone. The manual conversion approach is correct but adds complexity that is not reflected in the 3h estimate.
  - Required Action: Explicitly document in the plan that `BoostConfig` is a **separate** struct with its own serde deserialization, not a reuse of `MetadataValue`'s tagged format. Add acceptance criterion: "BoostConfig JSON format uses bare values, not adjacently-tagged MetadataValue format."

- **[M3] `data.json` size estimate of < 5MB is wrong for the proposed format**
  - Location: `WEEKLY_TASK_PLAN.md` line 138, `DAY_3_TASKS.md` line 17
  - Evidence: 1000 docs * 384 dimensions * ~7 bytes per float in JSON (e.g., `0.1234,` averages 7 chars) = ~2.7MB for document embeddings alone. Add 10 query embeddings * 384 * 7 = ~27KB. Add text (SQuAD paragraphs are ~500 chars after truncation: 1000 * 500 = 500KB). Add entity metadata (~100 bytes per doc = 100KB). Add JSON structural overhead (~200KB). Total estimate: ~3.5MB. This is within the < 5MB budget. HOWEVER: `normalize_embeddings=True` in the script (line 86) produces values like `0.012345678` (10+ chars per float), not 4-char floats. Realistic per-float JSON size with normalized 384D embeddings is ~10-12 bytes, giving: 1000 * 384 * 11 = ~4.2MB for embeddings alone, pushing total to ~5.0MB or above.
  - Impact: The 5MB budget may be exceeded, requiring the fallback (500 docs or float16) which is not pre-tested and would invalidate the "1000 docs" demo claim.
  - Required Action: Add a precision control step in `prepare_data.py`: round embeddings to 6 decimal places before JSON serialization (e.g., `[round(x, 6) for x in emb.tolist()]`). This keeps precision adequate for demo purposes while controlling file size. Add this to DAY_3 acceptance criteria.

- **[M4] Demo JS logic flow assumes EdgeVec WASM index can be built client-side from 1000 * 384D vectors -- no performance estimate provided**
  - Location: `DAY_4_TASKS.md` lines 87-88
  - Evidence: The demo JS loads data.json, then "Create EdgeVecIndex (dim=384)" and "Insert all 1000 embeddings." HNSW index construction for 1000 vectors at 384D involves 1000 insertions, each requiring distance computations against existing neighbors. In WASM, this could take several seconds. The plan specifies "search latency < 100ms" but provides **no acceptance criterion for index build time**. Users will stare at a blank page while the index builds.
  - Impact: If index build takes > 5s, the demo feels broken. If > 10s, users leave.
  - Required Action: Add acceptance criterion for index build time (e.g., "< 5s in Chrome"). Add a loading indicator to the demo spec. Consider pre-building the index and shipping it as a binary blob if build time is too high.

- **[M5] Weekly plan formula says `final_score = vector_similarity + sum(weight_i * match_i)` but DAY_1 correctly uses distance subtraction -- terminology inconsistency creates implementation risk**
  - Location: `WEEKLY_TASK_PLAN.md` line 114 says "vector_similarity" (higher = better), line 366 says same formula. `DAY_1_TASKS.md` line 132 says "search_result.distance -= boost" (lower = better).
  - Evidence: `SearchResult.distance` is a distance metric (lower = better), confirmed by `src/hnsw/search.rs` line 17-18. The weekly plan uses "vector_similarity" which implies higher = better. These are opposite conventions. The DAY_1 file correctly uses distance subtraction, but the weekly plan formula is wrong.
  - Impact: An implementer reading only the weekly plan (as designed for independence) will implement the boost incorrectly (adding to similarity instead of subtracting from distance).
  - Required Action: Fix the weekly plan formula to use distance-based language: `final_distance = raw_distance - sum(weight_i * match_i)`. Update all occurrences including the anti-error checklist (line 366) and hostile review attack vectors (lines 260, 263).

### Minor Issues (SHOULD FIX)

- **[m1] `compute_boost()` uses equality comparison on MetadataValue, but entities in demo data are stored as `StringArray` not `String`**
  - Location: `DAY_1_TASKS.md` line 69, `DAY_3_TASKS.md` line 116-125
  - Evidence: `compute_boost()` checks `metadata[field] == value` using `MetadataValue::PartialEq`. Demo data stores entities as `"entities": ["NASA", ...]` which is `MetadataValue::StringArray`. A boost on `field: "entities", value: MetadataValue::String("NASA")` would compare `StringArray(["NASA",...])` with `String("NASA")` -- this will NEVER match because the variants differ. The demo boost configs use `entity_type` (e.g., "PERSON") not individual entity names, which maps to a `StringArray` of types, not a single `String`.
  - Evidence: DAY_4 line 97-98 says `boost entity_type=GPE with weight=0.5` but demo data has `"entity_types": ["ORG", ...]` as an array, not a scalar.
  - Impact: If boost checks equality on a StringArray field against a String value, the boost will silently return 0.0 for ALL documents, making the feature appear broken in the demo.
  - Required Action: Either (a) change `compute_boost` to support `StringArray.contains(value)` matching, or (b) change demo data schema to store `primary_entity_type` as a `String` scalar, or (c) add a `has_entity_type` boolean field per type. Document the chosen approach.

- **[m2] Day 1 total hours are 8.5h but the plan says ~7h/day average**
  - Location: `DAY_1_TASKS.md` line 238, `WEEKLY_TASK_PLAN.md` line 45
  - Evidence: Day 1 estimates sum to 8.5h (3 + 3 + 2 + 0.5). The weekly plan says "~42h across 6 days (~7h/day)." Day 1 exceeds by 21%. Day 4 is 7h, Day 5 is 6.5h, Day 6 is 4.5h, Day 2 is 5h, Day 3 is 3.6h. Total = 35.1h, not ~42h as claimed.
  - Impact: The "42h planned" figure is wrong. Actual planned total is ~35.1h. This is misleading for estimation purposes but actually makes the plan more conservative (35h is below the 42h stated), so it does not create a feasibility risk. But the number should be accurate.
  - Required Action: Fix the total to match the actual sum across all daily files.

- **[m3] `import os` is inside `__main__` block but `os.path.getsize` is called inside `main()` function**
  - Location: `DAY_3_TASKS.md` lines 142, 156-157
  - Evidence: The script template has `import os` at line 156 (inside `if __name__ == "__main__":`) but `os.path.getsize("data.json")` is called at line 142 inside `main()`. This will raise `NameError: name 'os' is not defined`.
  - Impact: Script will crash on the verification step. Minor because this is a code snippet in a plan, not production code, and the actual implementation will presumably fix it.
  - Required Action: Move `import os` to the top of the script template.

- **[m4] Day 6 task W48.6d stages `src/wasm/mod.rs` with comment "(if modified)" but the WASM export is a required deliverable**
  - Location: `DAY_6_TASKS.md` line 149
  - Evidence: `git add src/wasm/mod.rs  # search_with_boost export (if modified)` -- the WASM export (W48.2a) IS a required deliverable. This should be staged unconditionally, not conditionally.
  - Impact: Low risk, but the "(if modified)" comment creates doubt about whether the WASM export was actually implemented.
  - Required Action: Remove "(if modified)" -- the WASM export is a mandatory deliverable.

- **[m5] No NaN validation on boost weight at construction time**
  - Location: `DAY_1_TASKS.md` line 67
  - Evidence: `MetadataBoost::new()` takes `weight: f32` but the plan only guards against NaN in `compute_boost()` (line 71). Per project rules (CLAUDE.md Section 3.1), "All public entry points validate finiteness." The constructor is a public entry point that accepts a weight parameter. If NaN is passed, it should be caught at construction, not at compute time.
  - Impact: NaN weights silently become 0.0 at compute time, which is correct but defers the error. A constructor-time rejection (returning `Result`) is cleaner.
  - Required Action: Either make `new()` return `Result<Self, _>` and reject NaN/Inf weights, or explicitly document in the plan that NaN is accepted at construction but neutralized at compute time (with justification for this tradeoff).

- **[m6] Blog comparison table claims EdgeVec search latency "<10ms (1K docs)" but this is the latency for 100K vectors per CLAUDE.md constraints**
  - Location: `DAY_5_TASKS.md` line 102
  - Evidence: `| Search latency | <10ms (1K docs) |` -- this claims 10ms for 1000 docs. The project constraint (CLAUDE.md Section 6.3) is `<10ms` for `100k vectors`. 1K docs should be orders of magnitude faster (sub-millisecond). Claiming "<10ms for 1K" is technically true but misleadingly slow.
  - Impact: Low -- the table is understatting EdgeVec performance, not overstating it. But a savvy reader might think "10ms for 1K docs is slow."
  - Required Action: Use a more accurate estimate for the blog table (e.g., "<1ms (1K docs)") or use the established 100K target.

- **[m7] No `.gitignore` entry for `demo/entity-rag/data.json` in Day 3 -- Day 3 creates it but Day 6 commit does not verify it**
  - Location: `DAY_3_TASKS.md` line 239-240 adds to `.gitignore`, but `DAY_6_TASKS.md` line 159 stages `.gitignore` without verifying the entry exists.
  - Evidence: The gitignore addition is in Day 3 (W48.3c), but Day 6's commit sequence (W48.6d) just runs `git add .gitignore` without checking the entry is there. If Day 3 is skipped or the entry is lost, a 3-5MB JSON file gets committed.
  - Impact: Low probability, but the mitigation is trivial (add a verification step).
  - Required Action: Add verification in Day 6 pre-commit: `grep "data.json" .gitignore` succeeds.

---

## Verdict

```
+---------------------------------------------------------------------+
|   HOSTILE_REVIEWER: REJECT                                          |
|                                                                     |
|   Artifact: Week 48 Plan (Weekly + 6 Daily Files)                   |
|   Author: PLANNER                                                   |
|                                                                     |
|   Critical Issues: 2                                                |
|   Major Issues: 5                                                   |
|   Minor Issues: 7                                                   |
|                                                                     |
|   Disposition:                                                      |
|   - REJECT: 2 Critical issues must be resolved before approval.     |
|   - C1 is an architectural flaw in the core feature.                |
|   - C2 is an internal contradiction between plan documents.         |
|                                                                     |
+---------------------------------------------------------------------+
```

**REJECTED**

This plan fails 2 critical quality gates and cannot proceed.

---

## Required Actions Before Resubmission

1. [ ] **[C1]** Resolve the score scale mismatch. Define how boost weights interact with the distance metric. Choose a scale-aware approach and document it in the MetadataBoost design section. Propagate the decision to all daily files.
2. [ ] **[C2]** Reconcile the oversample formula between the weekly plan and DAY_1. Choose ONE formula with documented rationale.
3. [ ] **[M1]** Provide full bibliographic references for Palmonari et al. 2025 and Xu et al. 2024, or remove fabricated citations.
4. [ ] **[M2]** Explicitly document that `BoostConfig` uses a separate deserialization format from `MetadataValue`. Add acceptance criterion.
5. [ ] **[M3]** Add float precision control to `prepare_data.py` spec (round to 6 decimals) and add to Day 3 acceptance criteria.
6. [ ] **[M4]** Add index build time acceptance criterion for the demo. Specify loading indicator.
7. [ ] **[M5]** Fix all "vector_similarity" references to use "distance" language. Update the formula in the weekly plan, anti-error checklist, and hostile review attack vectors.

## Resubmission Process

1. Address ALL 2 critical issues
2. Address ALL 5 major issues
3. Address minor issues m1 through m7 or document them as accepted-as-is with justification
4. Update all artifacts with `[REVISED]` tag
5. Resubmit for hostile review via `/review W48_PLAN`

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-03-05*
*Verdict: REJECTED*
