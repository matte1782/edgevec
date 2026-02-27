# Week 42: Foundation & Safety — v1.0 "Polish & Ship"

**Status:** [PROPOSED]
**Sprint Goal:** Validate v1.0 readiness through security audit (Miri + fuzzing) and LangChain.js spike
**Dates:** 2026-02-28 to 2026-03-06

---

## Day-by-Day Plan

### Day 1 (2026-02-28): LangChain.js Spike + Miri Setup

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Research @langchain/core VectorStore interface | W42.1a | 2h | DONE |
| Create `docs/research/LANGCHAIN_SPIKE.md` with full analysis | W42.1b | 1h | DONE |
| GO/NO-GO decision for v1.0 inclusion | W42.1c | 0.5h | DONE: GO |
| Gate `wasm` module behind `cfg(target_arch = "wasm32")` | W42.2a | 1h | DONE |
| Move WASM-only deps to target-specific section in Cargo.toml | W42.2b | 0.5h | DONE |
| Run initial Miri audit | W42.2c | 1h | DONE |
| Fix Miri finding: unaligned bytemuck read in header.rs | W42.2d | 0.5h | DONE |

**Day 1 Artifacts:**
- `docs/research/LANGCHAIN_SPIKE.md` [DONE]
- Miri-compatible build configuration [DONE]
- UB fix in persistence header [DONE]

### Day 2 (2026-03-01): Miri Completion + Fuzz Target Fixes

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Verify Miri passes clean (0 UB) | W42.2e | 2h | PENDING |
| Run Miri with `--features sparse` | W42.2f | 1h | PENDING |
| Document Miri results in `docs/reviews/` | W42.2g | 1h | PENDING |
| Fix `graph_ops` fuzz target (delete API change) | W42.3a | 0.5h | DONE |
| Fix `sparse_storage` fuzz target (SparseId type) | W42.3b | 0.5h | DONE |
| Verify all 15 fuzz targets compile | W42.3c | 0.5h | DONE |

**Day 2 Artifacts:**
- `docs/reviews/2026-03-01_MIRI_AUDIT.md`
- All 15 fuzz targets compiling

### Day 3 (2026-03-02): Fuzzing Campaign (Critical Targets)

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Fuzz `filter_deep` (1h min runtime) | W42.3d | 1.5h | PENDING |
| Fuzz `persistence` (1h min runtime) | W42.3e | 1.5h | PENDING |
| Fuzz `hnsw_search` (1h min runtime) | W42.3f | 1.5h | PENDING |
| Fix any crashes found | W42.3g | 2h | PENDING |

### Day 4 (2026-03-03): Fuzzing Campaign (Remaining Targets)

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Fuzz `sparse_vector` (1h min runtime) | W42.3h | 1.5h | PENDING |
| Fuzz `flat_index` (1h min runtime) | W42.3i | 1.5h | PENDING |
| Fix any crashes found | W42.3j | 2h | PENDING |

### Day 5 (2026-03-04): CI Update + Documentation

| Task | ID | Hours | Status |
|:-----|:---|:------|:-------|
| Update CI: fuzz-check all targets (not just dummy) | W42.3k | 0.5h | DONE |
| Update CI: add Miri job | W42.2h | 0.5h | DONE |
| Document fuzz campaign in `docs/reviews/` | W42.3l | 1h | PENDING |
| Run full regression: `cargo test --all-features` | W42.3m | 0.5h | PENDING |
| Run clippy: `cargo clippy -- -D warnings` | W42.3n | 0.5h | PENDING |

**Day 5 Artifacts:**
- `.github/workflows/ci.yml` updated [DONE]
- `docs/reviews/2026-03-04_FUZZ_CAMPAIGN.md`

---

## Acceptance Criteria

### W42.1: LangChain.js Spike
- [x] `@langchain/core` installato e VectorStore interface analizzata
- [x] Documento `docs/research/LANGCHAIN_SPIKE.md` con: metodi richiesti, mapping API, stima effort, rischi
- [x] GO/NO-GO decision per inclusione in v1.0
- [x] Se NO-GO: alternativa documentata

### W42.2: Miri Audit
- [x] `cargo +nightly miri test` eseguito
- [ ] Risultati documentati in `docs/reviews/2026-XX-XX_MIRI_AUDIT.md`
- [x] Ogni finding categorizzato: REAL BUG / FALSE POSITIVE / WASM-ONLY
- [x] Zero REAL BUG rimasti non-fixati (1 found, 1 fixed)
- [x] CI job aggiunto: `.github/workflows/ci.yml` con step Miri

### W42.3: Fuzzing Campaign
- [x] Tutti i 15 fuzz target compilano con cargo +nightly
- [ ] Fuzz target critici eseguiti per minimo 1h ciascuno
- [ ] Zero crash non-fixati
- [x] CI aggiornato: fuzz-check compila TUTTI i target
- [ ] Risultati in `docs/reviews/2026-XX-XX_FUZZ_CAMPAIGN.md`

---

## Key Changes Made (Day 1)

### Architecture Change: WASM Module Gating
- `pub mod wasm` → `#[cfg(target_arch = "wasm32")] pub mod wasm` in `src/lib.rs`
- WASM deps moved to `[target.'cfg(target_arch = "wasm32")'.dependencies]` in `Cargo.toml`
- Error impls for JsValue gated behind `#[cfg(target_arch = "wasm32")]` in `src/error.rs`
- 11 WASM integration tests gated with `#![cfg(target_arch = "wasm32")]`
- **Rationale:** Required for Miri to test non-WASM code paths; also more correct semantically

### Bug Fix: Unaligned bytemuck Read (Miri Finding)
- `persistence/header.rs`: Replaced `bytemuck::from_bytes` with `bytemuck::pod_read_unaligned`
- `persistence/header.rs`: Replaced `bytemuck::try_from_bytes` with `bytemuck::pod_read_unaligned`
- `persistence/chunking.rs`: Replaced `bytemuck::cast_slice` with manual `f32::from_le_bytes` in test
- **Category:** REAL BUG (undefined behavior on unaligned reads)
- **Risk:** Low in practice (WASM always uses aligned allocations) but UB is UB

### Fuzz Target Fixes
- `graph_ops/target.rs`: `index.delete()` → `index.soft_delete()` (API changed)
- `sparse_storage/target.rs`: `storage.get(id)` → `storage.get(SparseId::from(id))` (type changed)

---

## Dependencies

| This Week | Blocks |
|:----------|:-------|
| W42.1 (LangChain spike) | W43.1 (LangChain implementation) |
| W42.2 (Miri audit) | W46.3 (GATE_5 — security audit complete) |
| W42.3 (Fuzzing campaign) | W46.3 (GATE_5 — fuzzing complete) |

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| Miri finds more UB | LOW | MEDIUM | Budget 2h extra fix time in Day 2 |
| Fuzz targets crash | MEDIUM | LOW | Each crash is a bug fix opportunity |
| CI Miri job too slow | LOW | LOW | Reduce PROPTEST_CASES to 4 in CI |

---

## Session Log

### Session 2026-02-28 (Day 1 actual work)

**Commit:** `18724d1` — `feat(w42): Week 42 Days 1-2 — WASM gating, Miri UB fix, fuzz target fixes, CI updates`

**What was done today:**
- All Day 1 tasks DONE (LangChain spike, WASM gating, Miri setup, UB fix)
- Day 2 fuzz target fixes DONE (W42.3a, W42.3b, W42.3c)
- Day 5 CI tasks done early (W42.3k, W42.2h)
- Fixed clippy `redundant_closure` in `snapshot.rs` (from pod_read_unaligned conversion)
- Fixed deprecated `UnalignedBuffer` variant in `persistence_corruption.rs` test
- **Full regression passed:** 980 lib tests + all 32 integration test binaries, 0 failures
- **Clippy clean:** `cargo clippy --all-features -- -D warnings` passes

**What was NOT done (carry forward):**
- W42.2e: Verify Miri passes clean (0 UB) — needs `cargo +nightly miri test`
- W42.2f: Run Miri with `--features sparse`
- W42.2g: Document Miri results in `docs/reviews/`
- W42.3d-g: Fuzzing campaign critical targets (filter_deep, persistence, hnsw_search)
- W42.3h-j: Fuzzing campaign remaining targets (sparse_vector, flat_index)
- W42.3l-n: Fuzz documentation + final regression

**EXACT PICKUP POINT:**
> Start with Day 2 Miri completion (W42.2e-g), then proceed to Day 3 fuzzing campaign (W42.3d-g).
> All fuzz targets already compile. CI already has Miri job and fuzz-check.
> Working tree is clean. All previous work committed.

---

**END OF WEEKLY TASK PLAN**
