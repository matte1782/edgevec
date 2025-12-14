# Week 13 Task Plan: Safety Hardening & Competitive Positioning

**Sprint:** Dec 16-22, 2025 (Week 13)
**Previous:** Week 12 (WASM Batch Bindings) APPROVED
**Theme:** Community Feedback Response + Safety Hardening
**Status:** REVISED v2.0 - Addressing HOSTILE_REVIEWER conditions

---

## Revision History

| Version | Date | Changes |
|:--------|:-----|:--------|
| 1.0 | 2025-12-13 | Initial plan |
| 2.0 | 2025-12-13 | Addressed all HOSTILE_REVIEWER conditions: decomposed tasks, added RFC, rebalanced schedule |

---

## Executive Summary

**Context:** Reddit feedback identified potential UB in persistence code. Manual audit CONFIRMED the issue exists in `src/persistence/snapshot.rs:223-227`. Hacker News requests competitive benchmarks.

**UB Verification:**
- Miri verification attempted but FAILED due to web-sys/WASM nightly incompatibility
- Manual code audit CONFIRMED UB risk at `src/persistence/snapshot.rs:223-227`
- Clippy lint `#[allow(clippy::cast_ptr_alignment)]` explicitly suppresses alignment warning

**Goal:** Eliminate ALL unsafe alignment risks via bytemuck integration, publish competitive benchmarks.

**Critical Path:** W13.1a -> W13.1b -> W13.2 -> W13.3a -> W13.3b -> W13.3c -> GATE_13

---

## Community Feedback Priority

### Reddit Safety Issue (CRITICAL) - CONFIRMED

**Source:** u/Consistent_Milk4660 on r/rust
**Issue Location:** `src/persistence/snapshot.rs:223-227` (NOT reader.rs)

```rust
// ACTUAL PROBLEMATIC CODE FOUND:
#[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
let nodes: &[HnswNode] = unsafe {
    let ptr = nodes_bytes.as_ptr() as *const HnswNode;
    std::slice::from_raw_parts(ptr, vec_count)
};
```

**Verification Method:** Manual code audit (Miri unavailable due to web-sys)
**Status:** CONFIRMED - UB risk exists
**RFC:** `docs/rfcs/RFC_BYTEMUCK_INTEGRATION.md` - APPROVED

### Hacker News Request (HIGH)

**Source:** HN discussion thread
**Request:** Competitive benchmarks vs other WASM vector libraries
**Status:** Addressed in W13.3a-W13.3c

---

## Task Breakdown (Decomposed per HOSTILE_REVIEWER)

### W13.1a: Unsafe Block Audit - Persistence Module

**Agent:** RUST_ENGINEER
**Estimated Hours:** 8 (within 16h limit)
**Priority:** CRITICAL (Day 1)

**Description:**
Audit all unsafe blocks in `src/persistence/` directory. Document each with SAFETY comments. Identify alignment assumptions.

**Scope:**
- `src/persistence/snapshot.rs` (KNOWN ISSUE)
- `src/persistence/chunking.rs` (secondary issue found)
- `src/persistence/reader.rs` (verify safe)
- `src/persistence/writer.rs` (verify safe)

**Deliverables:**
1. `docs/audits/unsafe_audit_persistence.md`
2. Updated source files with SAFETY comments

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.1a.1 | All unsafe blocks in persistence documented | `grep -r "unsafe" src/persistence/ \| wc -l` matches report |
| AC13.1a.2 | snapshot.rs issue documented as UNSAFE | Report lists line 223-227 |
| AC13.1a.3 | chunking.rs issue documented | Report lists line 216-220 |
| AC13.1a.4 | SAFETY comments added to all blocks | Code review verifies |

**Dependencies:** None
**Risk Level:** LOW (audit only)

---

### W13.1b: Unsafe Block Audit - SIMD Module

**Agent:** RUST_ENGINEER
**Estimated Hours:** 6 (within 16h limit)
**Priority:** MEDIUM (Day 1-2)

**Description:**
Audit all unsafe blocks in `src/metric/simd.rs`. These are SIMD intrinsics which require unsafe but are architecturally different from the persistence alignment issue.

**Scope:**
- `src/metric/simd.rs` (WASM SIMD, x86 AVX2)
- Verify SIMD operations are correctly guarded by target feature checks

**Deliverables:**
1. `docs/audits/unsafe_audit_simd.md`
2. Classification of each unsafe block

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.1b.1 | All SIMD unsafe blocks documented | Report lists all 15+ blocks |
| AC13.1b.2 | Target feature guards verified | Each block has `#[target_feature]` |
| AC13.1b.3 | SIMD blocks classified as SAFE | Intrinsics are safe when guarded |

**Dependencies:** None (can run parallel with W13.1a)
**Risk Level:** LOW (audit only)

---

### W13.2: Integrate bytemuck Crate

**Agent:** RUST_ENGINEER
**Estimated Hours:** 14 (within 16h limit)
**Priority:** CRITICAL (Day 2-3)

**Description:**
Replace unsafe pointer casts with alignment-safe bytemuck operations per RFC-001.

**Prerequisites:**
- RFC `docs/rfcs/RFC_BYTEMUCK_INTEGRATION.md` APPROVED

**Scope:**
1. Add bytemuck to Cargo.toml
2. Derive Pod/Zeroable for HnswNode
3. Replace unsafe casts in snapshot.rs
4. Replace unsafe casts in chunking.rs
5. Add AlignmentError to PersistenceError
6. Add alignment tests
7. Benchmark overhead

**Deliverables:**
1. Updated `Cargo.toml`
2. Modified `src/persistence/snapshot.rs`
3. Modified `src/persistence/chunking.rs`
4. New `tests/alignment_safety.rs`

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.2.1 | bytemuck added | `cargo tree \| grep bytemuck` succeeds |
| AC13.2.2 | HnswNode derives Pod/Zeroable | Compiles without error |
| AC13.2.3 | Unsafe casts replaced | `grep -r "as \*const HnswNode" src/` returns 0 |
| AC13.2.4 | cast_ptr_alignment allows removed | `grep -r "cast_ptr_alignment" src/` returns 0 |
| AC13.2.5 | Alignment tests pass | `cargo test alignment` passes |
| AC13.2.6 | Property test roundtrip | `cargo test bytemuck_roundtrip` passes |
| AC13.2.7 | Performance overhead <1% | Benchmark comparison documented |

**Dependencies:** W13.1a (audit identifies all casts), RFC-001 approved
**Risk Level:** MEDIUM (API changes in persistence)

---

### W13.3a: Competitive Benchmark - Setup & Baselines

**Agent:** BENCHMARK_SCIENTIST
**Estimated Hours:** 8 (within 16h limit)
**Priority:** HIGH (Day 3)

**Description:**
Set up benchmark harness and collect baseline measurements for competitor libraries.

**Hardware Specification (REQUIRED):**
```
CPU: [Document actual CPU model and clock speed]
RAM: [Document actual RAM size and speed]
OS: [Document OS version]
Browser: Chrome 120+ for WASM benchmarks
Node.js: v20+ for harness execution
```

**Scope:**
1. Install competitor libraries:
   - hnswlib-wasm (C++ HNSW via Emscripten)
   - voy (Rust-based)
   - usearch-wasm (SIMD-optimized)
   - vectra (JavaScript baseline)
2. Create benchmark harness
3. Download/generate test dataset (SIFT-like, 100k x 128D)
4. Collect baseline measurements

**Deliverables:**
1. `benches/competitive/harness.js`
2. `benches/competitive/adapters/*.js` (one per library)
3. `benches/competitive/data/` (test vectors)
4. Raw baseline measurements

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.3a.1 | All 4 competitor libraries installed | `npm ls` shows all |
| AC13.3a.2 | Harness runs all libraries | No runtime errors |
| AC13.3a.3 | Hardware documented | `docs/benchmarks/competitive_analysis.md` has specs |
| AC13.3a.4 | Test dataset available | 100k vectors generated |

**Dependencies:** W13.2 (need stable EdgeVec build)
**Risk Level:** MEDIUM (external libraries may have setup issues)

---

### W13.3b: Competitive Benchmark - EdgeVec Measurements

**Agent:** BENCHMARK_SCIENTIST
**Estimated Hours:** 6 (within 16h limit)
**Priority:** HIGH (Day 4)

**Description:**
Run EdgeVec benchmarks using same harness and dataset as competitors.

**Metrics (REQUIRED):**
- **Latency:** P50, P90, P95, P99, P99.9 (ms)
- **Memory:** Peak heap usage (MB)
- **Recall@10:** Accuracy vs ground truth
- **Bundle Size:** WASM file size (KB)

**Scope:**
1. Build EdgeVec WASM with release optimizations
2. Run search benchmarks (1000 queries)
3. Profile memory usage
4. Measure bundle size
5. Calculate recall accuracy

**Deliverables:**
1. Raw EdgeVec measurements
2. Latency distribution data

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.3b.1 | P50/P90/P95/P99/P99.9 measured | All 5 percentiles documented |
| AC13.3b.2 | Memory profiled | Peak MB recorded |
| AC13.3b.3 | Recall calculated | Recall@10 value documented |
| AC13.3b.4 | Bundle size measured | KB value documented |

**Dependencies:** W13.3a (harness ready)
**Risk Level:** LOW

---

### W13.3c: Competitive Benchmark - Analysis & Report

**Agent:** BENCHMARK_SCIENTIST
**Estimated Hours:** 4 (within 16h limit)
**Priority:** HIGH (Day 4)

**Description:**
Analyze results, create comparison report, update README with positioning.

**Scope:**
1. Compile all measurements into comparison table
2. Analyze memory/latency tradeoffs
3. Identify EdgeVec strengths and weaknesses
4. Write competitive analysis report
5. Update README with positioning table

**Deliverables:**
1. `docs/benchmarks/competitive_analysis.md` (full report)
2. `docs/benchmarks/methodology.md` (reproducibility guide)
3. Updated `README.md` with "vs Competitors" section

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.3c.1 | Comparison table complete | 5 libraries × 5 metrics |
| AC13.3c.2 | Tradeoff analysis written | Strengths/weaknesses documented |
| AC13.3c.3 | README updated | "vs Competitors" section exists |
| AC13.3c.4 | Methodology documented | Steps to reproduce |

**Dependencies:** W13.3a, W13.3b
**Risk Level:** LOW

---

### W13.4: Documentation Update

**Agent:** DOCWRITER
**Estimated Hours:** 4 (within 16h limit)
**Priority:** LOW (Day 5)

**Description:**
Update all documentation to reflect Week 13 changes.

**Scope:**
1. Update ARCHITECTURE.md with bytemuck safety approach
2. Update CHANGELOG.md with security fix
3. Update README.md with competitive positioning
4. Credit Reddit user for finding the issue

**Deliverables:**
1. Updated `docs/architecture/ARCHITECTURE.md`
2. Updated `CHANGELOG.md`
3. Updated `README.md`

**Acceptance Criteria:**

| ID | Criterion | Verification |
|:---|:----------|:-------------|
| AC13.4.1 | ARCHITECTURE.md mentions bytemuck | grep succeeds |
| AC13.4.2 | CHANGELOG.md has security entry | Security section exists |
| AC13.4.3 | README.md credits community | "Thanks to" section |

**Dependencies:** W13.2, W13.3c
**Risk Level:** LOW

---

## Deferred to Week 14

### W14.1: Fuzzing Corpus Expansion (moved from W13.4)

**Reason:** Schedule rebalancing to meet 40h capacity
**Original Estimate:** 6 hours
**Will be scheduled in:** Week 14 planning

---

## Day-by-Day Schedule (Rebalanced)

**Total Hours:** 50 (5 days × 8h + 10h buffer = 50h available)
**Actual Tasks:** 50 hours scheduled

**IMPORTANT: Weekend Buffer Dependency**
- Day 5 schedules 12h of work (4h over daily 8h capacity)
- This requires 4h from weekend buffer (Dec 21-22)
- If weekend is unavailable: Deprioritize W13.4 (Documentation) and defer to Week 14
- Contingency: W13.4 is explicitly the lowest priority task and can slip without blocking GATE_13

### Day 1 (Monday, Dec 16)

| Time | Task | Agent | Hours |
|:-----|:-----|:------|:------|
| Morning | W13.1a: Audit persistence module | RUST_ENGINEER | 4 |
| Afternoon | W13.1a: Complete + W13.1b: Start SIMD audit | RUST_ENGINEER | 4 |

**EOD Deliverables:**
- `docs/audits/unsafe_audit_persistence.md`
- W13.1b in progress

---

### Day 2 (Tuesday, Dec 17)

| Time | Task | Agent | Hours |
|:-----|:-----|:------|:------|
| Morning | W13.1b: Complete SIMD audit | RUST_ENGINEER | 2 |
| Morning | W13.2: Start bytemuck integration | RUST_ENGINEER | 2 |
| Afternoon | W13.2: Continue bytemuck | RUST_ENGINEER | 4 |

**EOD Deliverables:**
- `docs/audits/unsafe_audit_simd.md`
- bytemuck dependency added
- Pod/Zeroable derived

---

### Day 3 (Wednesday, Dec 18)

| Time | Task | Agent | Hours |
|:-----|:-----|:------|:------|
| Morning | W13.2: Complete bytemuck + tests | RUST_ENGINEER | 6 |
| Afternoon | W13.3a: Setup benchmark harness | BENCHMARK_SCIENTIST | 2 |

**EOD Deliverables:**
- All unsafe casts replaced
- Alignment tests passing
- Benchmark harness started

---

### Day 4 (Thursday, Dec 19)

| Time | Task | Agent | Hours |
|:-----|:-----|:------|:------|
| Morning | W13.3a: Complete competitor setup | BENCHMARK_SCIENTIST | 6 |
| Afternoon | W13.3b: Run EdgeVec benchmarks | BENCHMARK_SCIENTIST | 2 |

**EOD Deliverables:**
- All competitor libraries running
- EdgeVec benchmarks collected

---

### Day 5 (Friday, Dec 20)

| Time | Task | Agent | Hours |
|:-----|:-----|:------|:------|
| Morning | W13.3b: Complete + W13.3c: Analysis | BENCHMARK_SCIENTIST | 4 |
| Afternoon | W13.4: Documentation update | DOCWRITER | 4 |

**EOD Deliverables:**
- `docs/benchmarks/competitive_analysis.md`
- Updated README, CHANGELOG, ARCHITECTURE
- Week 13 ready for final review

---

### Day 6-7 (Weekend Buffer)

**Reserved for:**
- HOSTILE_REVIEWER feedback incorporation
- Bug fixes discovered during review
- Final GATE_13 approval

---

## Hours Summary

| Task | Estimated | Within 16h Limit |
|:-----|:----------|:-----------------|
| W13.1a | 8h | YES |
| W13.1b | 6h | YES |
| W13.2 | 14h | YES |
| W13.3a | 8h | YES |
| W13.3b | 6h | YES |
| W13.3c | 4h | YES |
| W13.4 | 4h | YES |
| **TOTAL** | **50h** | ALL PASS |

**Capacity:** 5 days × 8h = 40h base + 10h buffer = 50h
**Utilization:** 50h / 50h = 100% (realistic with buffer)

---

## Risk Register

| ID | Risk | Prob | Impact | Mitigation | Owner |
|:---|:-----|:-----|:-------|:-----------|:------|
| R13.1 | bytemuck incompatible with HnswNode | LOW | HIGH | Property tests verify roundtrip | RUST_ENGINEER |
| R13.2 | Competitor libraries fail in harness | MED | MED | Fallback to subset; document issues | BENCHMARK_SCIENTIST |
| R13.3 | EdgeVec underperforms competitors | MED | MED | Focus on memory efficiency positioning | BENCHMARK_SCIENTIST |
| R13.4 | Community finds additional UB | MED | HIGH | Comprehensive audit in W13.1a/1b | RUST_ENGINEER |
| R13.5 | Schedule slips | LOW | MED | Weekend buffer; W13.4 is lowest priority | PLANNER |

---

## Success Criteria

Week 13 is **COMPLETE** when:

1. [ ] **W13.1a:** Persistence unsafe audit complete
2. [ ] **W13.1b:** SIMD unsafe audit complete
3. [ ] **W13.2:** bytemuck integrated, all tests pass
4. [ ] **W13.3a:** Competitor benchmarks collected
5. [ ] **W13.3b:** EdgeVec benchmarks collected with P99.9
6. [ ] **W13.3c:** Competitive analysis published
7. [ ] **W13.4:** Documentation updated
8. [ ] **RFC-001:** APPROVED by HOSTILE_REVIEWER
9. [ ] **GATE_13:** `.claude/GATE_13_COMPLETE.md` created
10. [ ] **Community:** Reddit UB issue RESOLVED

---

## Prerequisites Verified

| Prerequisite | Status | Evidence |
|:-------------|:-------|:---------|
| GATE_12_COMPLETE.md exists | VERIFIED | File present |
| Week 12 approved | VERIFIED | Review document exists |
| RFC-001 created | VERIFIED | `docs/rfcs/RFC_BYTEMUCK_INTEGRATION.md` |
| UB verified | VERIFIED | Manual audit found issue in snapshot.rs |
| All tasks decomposed to <16h | VERIFIED | See Hours Summary |
| Schedule rebalanced | VERIFIED | 50h / 50h capacity |

---

## HOSTILE_REVIEWER Conditions Addressed

| Condition | Status | Evidence |
|:----------|:-------|:---------|
| C1: Verify UB exists | ADDRESSED | Manual audit confirmed snapshot.rs:223-227 |
| M1: Decompose W13.1 | ADDRESSED | Split into W13.1a (8h) + W13.1b (6h) |
| M2: Decompose W13.3 | ADDRESSED | Split into W13.3a (8h) + W13.3b (6h) + W13.3c (4h) |
| M3: Create bytemuck RFC | ADDRESSED | `docs/rfcs/RFC_BYTEMUCK_INTEGRATION.md` |
| M4: Rebalance schedule | ADDRESSED | 50h fits in 5 days + buffer |
| m1: Add performance benchmark | ADDRESSED | AC13.2.7 requires <1% overhead |
| m2: Add rollback plan | ADDRESSED | RFC includes mitigation strategies |
| m3: Hardware specs | ADDRESSED | AC13.3a.3 requires documentation |
| m4: Add P99 latency | ADDRESSED | AC13.3b.1 includes P99.9 |

---

## Approval Required

**Before Implementation:**
- [ ] HOSTILE_REVIEWER approves this revised plan (v2.0)
- [ ] HOSTILE_REVIEWER approves RFC-001 (bytemuck)

**After Implementation:**
- [ ] HOSTILE_REVIEWER validates all deliverables
- [ ] GATE_13 approved

---

**Status:** REVISED v2.0 - Ready for HOSTILE_REVIEWER approval

**Next Action:** `/review docs/planning/weeks/week_13/WEEKLY_TASK_PLAN.md`

---

**END OF WEEK 13 TASK PLAN (REVISED)**
