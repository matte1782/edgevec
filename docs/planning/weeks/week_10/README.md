# Week 10 Planning — Package Index

**Version:** v3.0
**Status:** [APPROVED]
**Date:** 2025-12-13
**Total Effort:** 67h raw → 201h with 3x rule
**Critical Path:** 60h raw

---

## Overview

Week 10 focuses on **development tool infrastructure** to enable safe, high-velocity feature development in future weeks. The primary deliverable is a robust fuzzing and testing pipeline.

### Scope Change from Original Plan

**Original Estimate:** 174h (10 tasks)
**Revised Estimate:** 67h (8 tasks, batch insert deferred)
**Reduction:** 62% scope reduction via strategic deferral

**Justification:**
1. Fuzz infrastructure is **blocking** Week 11+ development (all new features need fuzz coverage)
2. Batch insert is feature work, not infrastructure
3. No external deadline for batch insert (can defer one week without impact)
4. Fuzz fixes unblock 3+ features in backlog
5. Decision Authority: Strategic prioritization (infrastructure before features)

---

## Week 10 Deliverables

### Core Tasks (8)

| ID | Task | Raw | 3x | Priority | Status |
|:---|:-----|----:|---:|:---------|:-------|
| **W10.1** | Restructure fuzz targets into corpus/ hierarchy | 6h | 18h | P0 | Pending |
| **W10.2a** | Fix fuzz_hamming test | 6h | 18h | P0 | Pending |
| **W10.2b** | Fix fuzz_encoder test | 6h | 18h | P0 | Pending |
| **W10.2c** | Fix fuzz_quantizer test | 6h | 18h | P0 | Pending |
| **W10.2d** | Add proper assertions to fuzz targets | 6h | 18h | P0 | Pending |
| **W10.3** | Refactor fuzz_hnsw for new corpus structure | 12h | 36h | P0 | Pending |
| **W10.4** | Implement HNSW property tests | 18h | 54h | P1 | Pending |
| **W10.5** | Design batch insert API | 4h | 12h | P1 | Pending |
| **W10.8** | Create benchmark validation suite | 12h | 36h | P1 | Pending |

**Subtotal:** 76h raw → 228h with 3x

### Deferred to Week 11

| Original ID | New ID | Task | Raw | Reason |
|:------------|:-------|:-----|----:|:-------|
| W10.6 | W11.1 | Implement batch insert | 16h | W10.5 design must complete first |
| W10.7 | W11.2 | Benchmark batch insert | 12h | Depends on W11.1 implementation |

**Deferred Total:** 28h raw

---

## CRITICAL PATH (REVISED — Fixed N1)

**Longest Sequential Chain:** W10.1 → W10.2a → W10.2b → W10.2c → W10.2d → W10.3 → W10.4 → W10.8

**Total Duration:** 60h raw

**Calculation:** 6h (W10.1) + 6h (W10.2a) + 6h (W10.2b) + 6h (W10.2c) + 6h (W10.2d) + 12h (W10.3) + 18h (W10.4) = 60h

**Note:** W10.5 (batch API design) and W10.8 (benchmark validation) can run in parallel with critical path work, as W10.5 has no dependencies and W10.8 only depends on completion of fuzz infrastructure.

**Slack Time:** 40h (67h total - 60h critical path = 7h natural slack, plus 33h from parallel tasks)

---

## Artifacts

| File | Purpose |
|:-----|:--------|
| **WEEKLY_TASK_PLAN.md** | Detailed task breakdown with acceptance criteria |
| **RISK_REGISTER.md** | 7 identified risks with mitigation strategies |
| **TASK_DEPENDENCIES.dot** | Graphviz dependency graph |
| **PLANNER_HANDOFF.md** | Handoff document for HOSTILE_REVIEWER |
| **REVISION_SUMMARY.md** | Track changes across v1.0, v2.0, v3.0 |
| **README.md** | This file |

---

## Week 11 Preparation

**Blocking Artifact from W10.5:**
- RFC document with batch insert API design
- Trait signatures for `BatchInsertable`
- Error handling strategy
- WASM memory budget analysis

**This artifact unblocks:**
- W11.1: Batch insert implementation
- W11.2: Batch insert benchmarks

---

## Approval Chain

- [x] PLANNER created v1.0 (2025-12-13)
- [x] HOSTILE_REVIEWER rejected v1.0 (5 critical issues)
- [x] PLANNER created v2.0 (2025-12-13)
- [x] HOSTILE_REVIEWER rejected v2.0 (3 issues: N1, N2, N3)
- [x] PLANNER created v3.0 (2025-12-13)
- [x] HOSTILE_REVIEWER approved v3.0 (0 issues)
- [x] GATE 10 COMPLETE (2025-12-13)

**Status:** ✅ APPROVED — Ready for implementation

---

**Last Updated:** 2025-12-13 (v3.0)
**Next Review:** End of Week 10 (retrospective)
