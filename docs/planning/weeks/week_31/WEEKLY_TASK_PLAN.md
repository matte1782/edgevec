# Week 31: v0.7.0 Release — SIMD Acceleration + First External Contribution

**Date:** 2025-12-27 to 2026-01-02
**Focus:** v0.7.0 Release with SIMD Acceleration and Community Celebration
**Phase:** Release Execution
**Status:** [PROPOSED] — Ready for HOSTILE_REVIEWER approval

---

## Executive Summary

Week 31 focuses on **releasing v0.7.0**, which includes:

1. **SIMD Acceleration** — 2x+ faster vector operations (Week 30 complete)
2. **First External Contribution** — @jsonMartin's WASM SIMD128 Hamming distance (8.75x speedup)
3. **Filter Playground Enhancement** — Interactive demo for metadata filtering
4. **GitHub Pages Deployment** — All demos live
5. **Release Announcement** — Community outreach

**Critical Highlight:** This release includes EdgeVec's **FIRST EXTERNAL CONTRIBUTION** from @jsonMartin!

---

## Week 30 Verification (Day 1 Morning)

Before proceeding, verify Week 30 completion:

| Task | Expected Status | Verification |
|:-----|:----------------|:-------------|
| W30.0 Code Quality | COMPLETE | No rambling comments in chunking.rs |
| W30.1 SIMD Build Enablement | COMPLETE | 285 SIMD instructions in WASM |
| W30.2 SIMD Benchmarking | COMPLETE | 2.0+ Gelem/s documented |
| W30.3-5 Filter Playground | PARTIAL | Check enhancement status |
| W30.6 Documentation | PARTIAL | README SIMD section exists |
| W30.7 Review | PENDING | Run clippy + tests |

**Action:** Day 1 begins with 30-minute verification. If gaps found, address in Day 1.

---

## Day Files (7 Days)

| Day | File | Focus | Hours |
|:----|:-----|:------|:------|
| **Day 1** | [DAY_1_TASKS.md](DAY_1_TASKS.md) | W30 Verification + CHANGELOG Update | 4 |
| **Day 2** | [DAY_2_TASKS.md](DAY_2_TASKS.md) | Filter Playground Completion | 4 |
| **Day 3** | [DAY_3_TASKS.md](DAY_3_TASKS.md) | Documentation Finalization | 3 |
| **Day 4** | [DAY_4_TASKS.md](DAY_4_TASKS.md) | Pre-Release Testing | 3 |
| **Day 5** | [DAY_5_TASKS.md](DAY_5_TASKS.md) | Release Execution | 3 |
| **Day 6** | [DAY_6_TASKS.md](DAY_6_TASKS.md) | GitHub Pages Deployment | 3 |
| **Day 7** | [DAY_7_TASKS.md](DAY_7_TASKS.md) | Release Announcement + Monitoring | 3 |
| **Total** | | | **23** |

---

## v0.7.0 Feature Summary

### Core Features

| Feature | Source | Status |
|:--------|:-------|:-------|
| WASM SIMD128 Distance Metrics | Week 30 | ✅ Complete |
| WASM SIMD128 Hamming Distance | PR #4 (@jsonMartin) | ✅ Merged |
| AVX2 Native Hamming Distance | PR #4 (@jsonMartin) | ✅ Merged |
| Filter Playground Enhancement | Week 30 | ⏳ Verify |
| SIMD Benchmarks | Week 30 | ✅ Complete |

### Performance Improvements

| Metric | v0.6.0 | v0.7.0 | Improvement |
|:-------|:-------|:-------|:------------|
| Dot Product (768D) | ~500ns | ~200ns | 2.5x |
| L2 Distance (768D) | ~600ns | ~250ns | 2.4x |
| Hamming Distance | ~350ns | ~40ns | **8.75x** |
| Search (10k, k=10) | ~2ms | ~1ms | 2x |

*Source: [docs/benchmarks/2025-12-24_simd_benchmark.md](../../benchmarks/2025-12-24_simd_benchmark.md)*

### Community Milestone

**FIRST EXTERNAL CONTRIBUTION:**
- Contributor: **@jsonMartin**
- PR: #4 - feat(simd): add WASM SIMD128 Hamming distance
- Impact: 8.75x faster binary distance calculations
- Quality: 10 comprehensive tests, LUT-based popcount algorithm

---

## Task Breakdown

### W31.1: Week 30 Verification + CHANGELOG (Day 1)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.1.1 | Verify W30 completion status | 0.5 | PLANNER |
| W31.1.2 | Address any W30 gaps | 1 | RUST_ENGINEER |
| W31.1.3 | Update CHANGELOG with v0.7.0 features | 1.5 | DOCWRITER |
| W31.1.4 | Add PR #4 contribution credit | 0.5 | DOCWRITER |
| W31.1.5 | Run full test suite | 0.5 | TEST_ENGINEER |

### W31.2: Filter Playground Completion (Day 2)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.2.1 | Verify existing filter-playground.html | 0.5 | WASM_SPECIALIST |
| W31.2.2 | Add LiveSandbox class with WASM execution | 1.5 | WASM_SPECIALIST |
| W31.2.3 | Add performance timing display | 0.5 | WASM_SPECIALIST |
| W31.2.4 | Update version references to v0.7.0 | 0.5 | WASM_SPECIALIST |
| W31.2.5 | Test in Chrome, Firefox, Safari | 1 | TEST_ENGINEER |

### W31.3: Documentation Finalization (Day 3)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.3.1 | Update README Performance section | 1 | DOCWRITER |
| W31.3.2 | Add @jsonMartin to Contributors | 0.5 | DOCWRITER |
| W31.3.3 | Update API documentation | 1 | DOCWRITER |
| W31.3.4 | Review all demo pages for v0.7.0 | 0.5 | DOCWRITER |

### W31.4: Pre-Release Testing (Day 4)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.4.1 | Run cargo test --all | 0.5 | TEST_ENGINEER |
| W31.4.2 | Run cargo clippy -- -D warnings | 0.5 | TEST_ENGINEER |
| W31.4.3 | Run wasm-pack test --headless | 0.5 | TEST_ENGINEER |
| W31.4.4 | Build release WASM bundle | 0.5 | WASM_SPECIALIST |
| W31.4.5 | Verify bundle size <500KB | 0.5 | WASM_SPECIALIST |
| W31.4.6 | HOSTILE_REVIEWER pre-release check | 0.5 | HOSTILE_REVIEWER |

### W31.5: Release Execution (Day 5)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.5.1 | Update Cargo.toml version to 0.7.0 | 0.25 | RUST_ENGINEER |
| W31.5.2 | Update pkg/package.json version | 0.25 | WASM_SPECIALIST |
| W31.5.3 | cargo publish --dry-run | 0.5 | RUST_ENGINEER |
| W31.5.4 | cargo publish | 0.5 | RUST_ENGINEER |
| W31.5.5 | npm publish | 0.5 | WASM_SPECIALIST |
| W31.5.6 | Create GitHub release + tag v0.7.0 | 0.5 | RUST_ENGINEER |
| W31.5.7 | Verify crates.io + npm listings | 0.5 | TEST_ENGINEER |

### W31.6: GitHub Pages Deployment (Day 6)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.6.1 | Deploy filter-playground.html | 0.5 | WASM_SPECIALIST |
| W31.6.2 | Deploy simd_benchmark.html | 0.5 | WASM_SPECIALIST |
| W31.6.3 | Deploy v070_demo.html | 0.5 | WASM_SPECIALIST |
| W31.6.4 | Update demo index page | 0.5 | DOCWRITER |
| W31.6.5 | Verify all demos work | 0.5 | TEST_ENGINEER |
| W31.6.6 | Test mobile responsiveness | 0.5 | TEST_ENGINEER |

### W31.7: Release Announcement (Day 7)

| ID | Task | Hours | Agent |
|:---|:-----|:------|:------|
| W31.7.1 | Draft Reddit announcement | 1 | DOCWRITER |
| W31.7.2 | Highlight @jsonMartin contribution | 0.25 | DOCWRITER |
| W31.7.3 | Post to r/rust | 0.25 | DOCWRITER |
| W31.7.4 | Post to r/MachineLearning | 0.25 | DOCWRITER |
| W31.7.5 | Post to r/LocalLLaMA | 0.25 | DOCWRITER |
| W31.7.6 | Monitor feedback | 0.5 | DOCWRITER |
| W31.7.7 | Respond to comments | 0.5 | DOCWRITER |

---

## Exit Criteria

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| **Release** | | |
| v0.7.0 on crates.io | `cargo search edgevec` shows 0.7.0 | [ ] |
| v0.7.0 on npm | `npm info edgevec` shows 0.7.0 | [ ] |
| GitHub release exists | Tag v0.7.0 visible | [ ] |
| **Quality** | | |
| All tests pass | 677+ tests green | [ ] |
| Clippy clean | 0 warnings | [ ] |
| Bundle size <500KB | wasm-opt output verified | [ ] |
| **Documentation** | | |
| CHANGELOG complete | v0.7.0 section with all features | [ ] |
| @jsonMartin credited | Listed in CHANGELOG + README | [ ] |
| README updated | SIMD + Hamming performance | [ ] |
| **Deployment** | | |
| GitHub Pages live | All demos accessible | [ ] |
| Filter playground works | Live sandbox functional | [ ] |
| **Community** | | |
| Reddit posted | r/rust announcement live | [ ] |
| First contributor celebrated | @jsonMartin highlighted | [ ] |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| crates.io publish failure | Low | High | cargo publish --dry-run first |
| npm publish issues | Low | Medium | Test locally with npm pack |
| GitHub Pages deploy fail | Low | Low | Manual upload fallback |
| Broken demos after deploy | Medium | Medium | Test all demos before announce |
| Community silence | Medium | Low | Cross-post to multiple subreddits |

---

## PR #4 Credit Template

**For CHANGELOG:**
```markdown
#### Community Contribution 🎉

- **WASM SIMD128 Hamming Distance** — 8.75x faster binary distance calculations
  - LUT-based popcount algorithm (Warren, "Hacker's Delight", 2nd ed.)
  - Comprehensive test coverage (10 tests including edge cases)
  - Thanks to **[@jsonMartin](https://github.com/jsonMartin)** for this excellent first contribution!

- **AVX2 Native Hamming Distance** — Native popcount for x86_64
  - 4-way ILP optimization with separate accumulators
  - Also contributed by **@jsonMartin**
```

**For README Contributors Section:**
```markdown
## Contributors

Thank you to everyone who has contributed to EdgeVec!

- [@jsonMartin](https://github.com/jsonMartin) — SIMD Hamming distance implementation
```

---

## Approval Request

This weekly plan is submitted for HOSTILE_REVIEWER approval.

**Plan Statistics:**
- Total Hours: 23
- Days: 7
- Tasks: 35
- Agents Involved: PLANNER, RUST_ENGINEER, WASM_SPECIALIST, TEST_ENGINEER, DOCWRITER, HOSTILE_REVIEWER

**Key Deliverables:**
1. v0.7.0 released on crates.io + npm
2. GitHub Pages with live demos
3. First external contributor (@jsonMartin) properly credited
4. Community announcement with celebration

---

**Status:** [PROPOSED]
**Agent:** PLANNER
**Date:** 2025-12-26
**Next:** `/review WEEKLY_TASK_PLAN.md`
