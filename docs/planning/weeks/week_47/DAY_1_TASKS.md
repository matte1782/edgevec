# Week 47 — Day 1 Tasks (Monday, Apr 7)

**Date:** 2026-04-07
**Focus:** Prerequisites — Git Push, Real Embeddings, WASM PQ Exports
**Agents:** PLANNER, BENCHMARK_SCIENTIST, WASM_SPECIALIST, TEST_ENGINEER
**Status:** COMPLETE

---

## Day Objective

Unblock all downstream work by completing three prerequisites: push pending commits, generate real embedding data for G3 validation, and implement WASM PQ exports for G2 validation.

**Success Criteria:**
- 6 pending commits pushed to origin/main
- `tests/data/embeddings_768d_50k.bin` exists with 50K x 768D verified embeddings
- 3 WASM PQ functions (`train_pq`, `encode_pq`, `pq_search`) compile for `wasm32-unknown-unknown`
- All regression tests pass (1013+ lib, 0 clippy warnings, WASM build)

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `src/wasm/mod.rs` — existing WASM handle/opaque-pointer patterns
- [ ] `src/quantization/product.rs` — PQ public API (7 PqError variants, lines 49-107)
- [ ] `src/quantization/mod.rs` — PQ re-exports
- [ ] `.claude/GATE_W46_COMPLETE.md` — carry-forward items
- [ ] `docs/benchmarks/PQ_GO_NOGO_DECISION.md` — baseline numbers

---

## Tasks

### W47.0a: Push Pending Commits (0.25h) — PLANNER

**Dependency:** None
**Commands:**
```bash
git log --oneline origin/main..HEAD    # Verify 6 commits
git push origin main                    # Push
git log --oneline origin/main..HEAD    # Should return empty
```

**Expected Output:** `git push` succeeds, remote is up to date.

**Acceptance:**
- [ ] `git push` exits 0
- [ ] `git log --oneline origin/main..HEAD` returns empty

---

### W47.1a: Generate Real Embedding Dataset (2h) — BENCHMARK_SCIENTIST

**Dependency:** None (can run in parallel with W47.0a and W47.1b)
**Pre-requisite check:**
```bash
python --version          # Python 3.8+ required
pip install sentence-transformers numpy   # If not installed
```

**Decision Tree:**
- If `sentence-transformers` install fails on Windows → use WSL2 or conda
- If MTEB STS has < 50K sentences → use English Wikipedia (first 50K sentences)
- If model download fails → use `all-MiniLM-L6-v2` (384D) as fallback, adjust all downstream D references

**Commands:**
```bash
# Create data directory
mkdir -p tests/data

# Run generation script
python tests/data/generate_embeddings.py

# Verify output
python -c "import numpy as np; d=np.fromfile('tests/data/embeddings_768d_50k.bin', dtype='float32').reshape(-1, 768); assert d.shape == (50000, 768); assert np.all(np.isfinite(d)); print(f'OK: {d.shape}, range=[{d.min():.3f}, {d.max():.3f}]')"

# Add to gitignore
echo "tests/data/embeddings_768d_50k.bin" >> .gitignore
```

**Expected Output:**
- `tests/data/embeddings_768d_50k.bin` — exactly 153,600,000 bytes
- `tests/data/generate_embeddings.py` — reproducible script (committed to git)
- Verification prints: `OK: (50000, 768), range=[...]`

**Acceptance:**
- [ ] Binary file exists at correct size (153,600,000 bytes)
- [ ] Numpy verification passes (shape + finite check)
- [ ] Generation script is committed, binary is gitignored
- [ ] Model used: `all-mpnet-base-v2` (768D)

---

### W47.1b: Implement WASM PQ Exports (4h) — WASM_SPECIALIST

**Dependency:** None (can run in parallel with W47.0a and W47.1a)

**Context:** Existing WASM pattern in `src/wasm/mod.rs` uses `#[wasm_bindgen]` with `JsValue` returns. PQ codebook handles should follow the same opaque-pointer pattern as `EdgeVecIndex`.

**Functions to implement:**
1. `train_pq(data: &[f32], dims: u32, m: u32, ksub: u32, max_iters: u32) -> Result<JsValue, JsValue>`
2. `encode_pq(codebook_handle: &JsValue, vector: &[f32]) -> Result<Uint8Array, JsValue>`
3. `pq_search(codebook_handle: &JsValue, codes: &[u8], num_codes: u32, query: &[f32], k: u32) -> Result<JsValue, JsValue>`

**Commands:**
```bash
# After implementation
cargo check --target wasm32-unknown-unknown    # WASM compilation
cargo build                                      # Native compilation
cargo clippy -- -D warnings                      # Lint check
```

**Expected Output:** Three new `#[wasm_bindgen]` functions in `src/wasm/mod.rs`. No `unwrap()` in any WASM code. All errors converted to `JsValue` via `.map_err()`.

**Acceptance:**
- [ ] 3 functions added with `#[wasm_bindgen]` attribute
- [ ] `cargo check --target wasm32-unknown-unknown` succeeds
- [ ] No `unwrap()` in WASM PQ code
- [ ] All functions return `Result<_, JsValue>`
- [ ] Follow existing handle pattern from `EdgeVecIndex`

---

### W47.1c: Add PQ WASM Integration Tests (1.5h) — TEST_ENGINEER

**Dependency:** W47.1b complete

**Test Strategy:**
- **Native-side tests** (`cargo test`): verify WASM wrapper logic calls through to PQ correctly
- **WASM-side smoke test**: deferred to W47.2c (Playwright verification)

**Commands:**
```bash
cargo test test_wasm_pq    # Run PQ WASM-related tests
cargo test --lib            # Full regression
```

**Expected Output:** 3+ new tests passing.

**Acceptance:**
- [ ] `test_wasm_pq_train_returns_handle` passes
- [ ] `test_wasm_pq_encode_returns_codes` passes
- [ ] `test_wasm_pq_search_returns_results` passes
- [ ] All pass with `cargo test`

---

### W47.1d: Full Regression (0.5h) — TEST_ENGINEER

**Dependency:** W47.1b, W47.1c complete

**Commands:**
```bash
cargo test --lib                                  # 1013+ tests
cargo clippy -- -D warnings                       # 0 warnings
cargo check --target wasm32-unknown-unknown        # WASM build
```

**Expected Output:** All green. No regressions.

**Acceptance:**
- [ ] `cargo test --lib` — 1013+ passed, 0 failed
- [ ] `cargo clippy -- -D warnings` — 0 warnings
- [ ] `cargo check --target wasm32-unknown-unknown` — success

---

## Day 1 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~8.25h |
| New WASM functions | 3 (train_pq, encode_pq, pq_search) |
| New tests | 3+ |
| New files | `generate_embeddings.py`, `embeddings_768d_50k.bin` |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W47.0a | 0.25h | | |
| W47.1a | 2h | | |
| W47.1b | 4h | | |
| W47.1c | 1.5h | | |
| W47.1d | 0.5h | | |
| **Total** | **8.25h** | | |

---

## Handoff to Day 2

**Codebase state at EOD:**
- origin/main up to date (6 commits pushed)
- `tests/data/embeddings_768d_50k.bin` exists and verified
- `src/wasm/mod.rs` has 3 PQ WASM exports
- All tests green, WASM builds

**Day 2 prerequisites satisfied:**
- [ ] WASM exports exist (needed for benchmark harness)
- [ ] `wasm-pack build --release` needs to be run (Day 2 first task)

**Day 2 focus:** WASM benchmark harness + Playwright integration

---

**END OF DAY 1 TASKS**
