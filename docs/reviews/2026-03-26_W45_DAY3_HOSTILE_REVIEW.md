# HOSTILE REVIEW: W45 Day 3 — PQ Literature Review

**Verdict:** GO (after fixes)
**Date:** 2026-03-26
**Reviewer:** hostile-reviewer agent
**Artifact:** `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md`

---

## Critical (blocks release)

**[C1] Comparison table showed wrong codebook sizes for M=16 (1.5MB) and M=32 (3MB) — all are 768KB** [FIXED]
- Corrected table values. Removed the after-the-fact "Correction" paragraph. Table is now correct on first read.

**[C2] Section 5.1 and Section 3.2 used inconsistent memory figures (codes-only vs system-level)** [FIXED]
- All sections now distinguish codes-only from system-level. Recommendation uses conservative ~50% figure (including HNSW overhead), not the misleading 83% codes-only figure.

## Major (must fix before merge)

**[M1] Section 5.1 crossover analysis did not specify what costs were included** [FIXED]
- Explicit "(codes + codebook only, excluding HNSW overhead)" label added.

**[M2] G3 recall threshold has zero empirical support for 768D vectors** [FIXED]
- Note added to G3 explicitly acknowledging this is extrapolated from 128D data with HIGH uncertainty.

**[M3] File name did not match ROADMAP exit criteria** [FIXED]
- Updated ROADMAP line 468 to reference `PRODUCT_QUANTIZATION_LITERATURE.md`.

**[M4] G2 relaxation from 100ns to 150ns was silent ROADMAP amendment** [FIXED]
- Added explicit ROADMAP amendment note with rationale (HNSW visits ~200-400 candidates, not full scan).

**[M5] Recall benchmark B3 only at 10K (worst case for PQ)** [FIXED]
- Added B3b benchmark at 50K vectors where PQ is expected to perform better.

## Minor (addressed)

- [m1] Section 3 status [DRAFT] → [PROPOSED] [FIXED]
- [m2] Stale handoff block referencing temp file [FIXED — removed]
- [m3] "256 * 39" → "30-256x centroids (8K-65K)" [FIXED]
- [m4] "48 bytes per vector" → "8 bytes per vector" [FIXED]
- [m5] all-MiniLM-L6-v2 (384D) → all-mpnet-base-v2 (768D) [FIXED]
- [m7] Safari "10 seconds" → [HYPOTHESIS] with qualified language [FIXED]
- [m6] Dataset generation (Rust code for WASM benchmarks) [ACCEPTED — will be addressed in W46 benchmark setup]
- [m8] No Table of Contents [ACCEPTED — will add if document grows further]

## Acceptance Criteria Verification

- [x] Literature review complete with cited sources (4 systems, tagged [FACT]/[HYPOTHESIS])
- [x] All 3 research questions addressed with data
- [x] WASM feasibility assessment with concrete constraints
- [x] Preliminary GO/NO-GO recommendation (LEAN GO, MEDIUM confidence)
- [x] Internal consistency between sections (memory figures reconciled)
- [x] ROADMAP exit criteria aligned

## VERDICT: GO

All critical and major findings fixed. Document is internally consistent and ready for merge.
