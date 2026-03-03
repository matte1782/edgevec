# Week 45: LangChain.js v0.2.0 + Product Quantization Research + v1.0 Prep

**Status:** [APPROVED] (hostile review passed — all C1+M1-M4+m1-m5 fixed)
**Sprint Goal:** Ship edgevec-langchain@0.2.0 with polished FilterExpression; begin PQ research spike (pulled forward from W46); start v1.0 readiness groundwork
**Dates:** 2026-03-24 to 2026-03-29 (5 core days + 1 overflow)
**Prerequisites:** W44 [COMPLETE], both research spikes NO-GO, FilterExpression implemented

---

## Strategic Context

W44 produced two NO-GO decisions (WebGPU + Relaxed SIMD), freeing ~36h of roadmapped implementation time. W45 redirects this capacity toward:

1. **Ship what we have** — Polish and publish edgevec-langchain@0.2.0 (FilterExpression is implemented but not published)
2. **Pull forward PQ research** — Product Quantization was scheduled for W46; starting the literature review early gives W46 more implementation time if GO
3. **v1.0 groundwork** — Begin API surface audit and documentation gap analysis to de-risk Phase 11

### Scope Decision Rationale

| Candidate | Decision | Rationale |
|:----------|:---------|:----------|
| LangChain v0.2.0 | **IN** | FilterExpression done, needs edge cases + docs + publish |
| PQ Research (Phase 1) | **IN (pulled forward)** | NO-GO decisions freed capacity; early research gives W46 more implementation time |
| BM25 Integration | **SKIP** | Zero community requests (trigger: 3+) |
| WebGPU Implementation | **SKIP** | NO-GO from W44 spike |
| Relaxed SIMD Implementation | **SKIP** | NO-GO from W44 spike |
| API Stability Review | **IN (lightweight)** | Early identification of breaking changes before v1.0 freeze |
| Docs Polish | **IN** | README examples, embedding guide improvements |

---

## Estimation Notes

**Optimistic total:** 24h
**3x ceiling:** 72h (disaster scenario boundary — not a target)
**Planned:** ~33h across 6 days (~5.5h/day) — 1.4x multiplier on optimistic estimate
**Buffer strategy:** Day 6 is overflow (5.5h). Per-task estimates include ~30% padding. The 3x ceiling (72h) represents the absolute upper bound; 33h is the realistic target with padding built into individual tasks. Total sprint will not exceed 40h even with all contingencies.
**If surplus time:** Additional PQ research depth or early W46 prep

---

## Critical Path

```
Track A (LangChain):     Day 1 (Edge Cases) → Day 2 (Docs + v0.2.0 Prep)
Track B (Research/Audit): Day 3 (PQ Literature) → Day 4 (PQ Benchmark + API Audit)
                                                        ↓
                                          Day 5 (Hostile Review — all artifacts)
                                                        ↓
                                          Day 6 (Fix Findings + Publish)
```

**Note:** Track A (Days 1-2) and Track B (Days 3-4) are fully independent — no cross-dependencies. They are sequenced by calendar, not by dependency. Day 5 reviews all artifacts from both tracks.

**Key decision point:** End of Day 4 = PQ Research GO/NO-GO preliminary assessment (full decision in W46).

---

## Day-by-Day Summary

### Day 1 (2026-03-24): LangChain.js Edge Cases + Testing
- W45.1a: FilterExpression edge case tests (5 `it()` blocks) — PENDING
- W45.1b: Null/undefined filter handling tests (3 `it()` blocks) — PENDING
- W45.1c: Complex nested filter tests (4 `it()` blocks) — PENDING
- W45.1d: FilterExpression error handling (3 `it()` blocks) — PENDING
- W45.1e: Run full regression (Rust + LangChain) — PENDING
- **Target:** 15 new `it()` blocks → 149 total langchain tests (134 + 15)

### Day 2 (2026-03-25): LangChain.js Documentation + v0.2.0 Prep
- W45.2a: Write FilterExpression usage guide with real-world examples — PENDING
- W45.2b: Add code examples to README (semantic search + filtered retrieval) — PENDING
- W45.2c: Update CHANGELOG for v0.2.0 — PENDING
- W45.2d: Bump version to 0.2.0 in package.json + verify backward compatibility — PENDING
- W45.2e: Verify build output (ESM + CJS, size check) — PENDING
- W45.2f: Update ROADMAP.md Milestone 10.4 schedule (PQ research pulled forward to W45) — PENDING

### Day 3 (2026-03-26): Product Quantization Literature Review
- W45.3a: Survey PQ implementations (Faiss, ScaNN, Qdrant) — PENDING
- W45.3b: Analyze PQ vs BQ trade-offs (compression, recall, latency) — PENDING
- W45.3c: Assess WASM feasibility (codebook training in browser, memory) — PENDING
- W45.3d: Draft research questions and exit criteria — PENDING

### Day 4 (2026-03-27): PQ Benchmark Design + API Stability Audit
- W45.4a: Design PQ benchmark methodology — PENDING
- W45.4b: API surface inventory (public Rust + WASM + TS types) — PENDING
- W45.4c: Identify breaking change candidates for v1.0 — PENDING
- W45.4d: Document API stability recommendations — PENDING

### Day 5 (2026-03-28): Hostile Review
- W45.5a: `/review` LangChain v0.2.0 changes — PENDING
- W45.5b: `/review` PQ research document — PENDING
- W45.5c: `/review` API audit document — PENDING
- W45.5d: Triage findings (critical/major/minor) — PENDING
- W45.5e: Update CHANGELOG with W45 summary — PENDING

### Day 6 (2026-03-29): Overflow / Fix Findings + Publish
- W45.6a: Fix all critical hostile reviewer findings — PENDING
- W45.6b: Fix all major hostile reviewer findings — PENDING
- W45.6c: npm publish `edgevec-langchain@0.2.0` (user handles OTP) — PENDING
- W45.6d: Commit and push all work — PENDING
- W45.6e: Create `.claude/GATE_W45_COMPLETE.md` — PENDING

**If surplus time available**, prioritize:
1. Additional PQ research depth (implementation feasibility assessment)
2. Begin W46 PQ implementation planning
3. README main package improvements
4. Blog post draft about edgevec-langchain

---

## Deliverables

| Deliverable | Target |
|:------------|:-------|
| `edgevec-langchain@0.2.0` npm publish | Day 6 |
| Additional FilterExpression edge case tests | Day 1 |
| FilterExpression usage guide | Day 2 |
| `docs/research/PRODUCT_QUANTIZATION_LITERATURE.md` | Day 3 |
| `docs/research/PQ_BENCHMARK_PLAN.md` | Day 4 |
| `docs/audits/API_SURFACE_INVENTORY.md` | Day 4 |
| `docs/audits/API_STABILITY_AUDIT.md` | Day 4 |
| Updated `docs/planning/ROADMAP.md` (PQ schedule) | Day 2 |
| Hostile review of all artifacts | Day 5 |
| `.claude/GATE_W45_COMPLETE.md` | Day 6 |

---

## Sprint-Level Acceptance Criteria

| Criterion | Pass/Fail |
|:----------|:----------|
| edgevec-langchain@0.2.0 published to npm | [ ] |
| FilterExpression edge cases covered (15 new `it()` blocks, 149 total) | [ ] |
| Usage guide with 3+ real-world examples | [ ] |
| PQ literature review answers all 3 research questions | [ ] |
| API stability audit identifies breaking change candidates | [ ] |
| All Rust tests pass (980+) | [ ] |
| All LangChain tests pass (149, up from 134) | [ ] |
| Clippy clean | [ ] |
| Hostile review: all critical + major fixed | [ ] |

---

## Risk Register

| Risk | Prob | Impact | Mitigation |
|:-----|:-----|:-------|:-----------|
| FilterExpression edge case reveals WASM bug | LOW | HIGH | Full regression before publish; rollback plan |
| PQ research inconclusive at literature stage | LOW | LOW | Expected — full research continues in W46 |
| npm publish blocked by auth/OTP | LOW | MEDIUM | User handles OTP; tested in W43 |
| API audit reveals many breaking changes | MEDIUM | LOW | Document only — no changes until v1.0 |
| v0.2.0 breaks existing users | LOW | HIGH | Additive union type (verified backward-compatible); post-publish smoke test; rollback: `npm deprecate edgevec-langchain@0.2.0` + publish 0.2.1 hotfix |

---

## Dependencies

| This Week Blocks | What |
|:-----------------|:-----|
| W45.1-2 LangChain v0.2.0 | npm publish, community adoption |
| W45.3-4 PQ Research | W46 PQ implementation (if GO) |
| W45.4b-d API Audit | W48-51 API stability work (v1.0) |

---

## W44 Context (Informs W45 Decisions)

- **WebGPU:** NO-GO — GPU-CPU transfer overhead dominates under 500K vectors
- **Relaxed SIMD:** NO-GO — Safari behind flag, dual bundles not justified
- **FilterExpression:** Implemented and tested (134 total tests), not yet published as v0.2.0
- **v0.10.0 pivot:** Focus shifts from browser acceleration research to practical features (PQ, LangChain polish, v1.0 prep)

---

**END OF WEEKLY TASK PLAN**
