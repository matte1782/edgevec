# Week 10: Development Tool Infrastructure

**Version:** v3.0 [APPROVED]
**Status:** Ready for Implementation
**Total Effort:** 76h raw → 228h with 3x rule
**Critical Path:** 60h raw
**Start Date:** 2025-12-14 (estimated)
**End Date:** 2025-12-21 (estimated)

---

## Objectives

1. **Establish robust fuzz testing infrastructure** for all core modules
2. **Fix all broken fuzz targets** to enable CI integration
3. **Design batch insert API** to unblock Week 11 implementation
4. **Create benchmark validation suite** for performance regression detection

---

## Tasks

### W10.1: Restructure Fuzz Targets into Corpus Hierarchy

**Priority:** P0 (Critical Path)
**Dependencies:** None
**Raw Estimate:** 6h
**3x Estimate:** 18h

**Description:**
Reorganize `fuzz/fuzz_targets/` directory structure to support corpus-based fuzzing with proper input generation and seed files.

**Current Structure (Pre-W10.1):**
```
fuzz/fuzz_targets/
├── dummy_harness.rs
├── fuzz_quantization.rs
├── graph_ops.rs
├── header_parse.rs
├── hnsw_config_init.rs
├── hnsw_insert.rs
├── hnsw_search.rs
├── persistence_load.rs
├── search_robustness.rs
└── wal_replay.rs
```

**Target Structure (Post-W10.1):**
```
fuzz/
├── fuzz_targets/
│   ├── dummy_harness/target.rs
│   ├── quantization/target.rs
│   ├── graph_ops/target.rs
│   ├── header_parse/target.rs
│   ├── hnsw_config/target.rs
│   ├── hnsw_insert/target.rs
│   ├── hnsw_search/target.rs
│   ├── persistence/target.rs
│   ├── search_robustness/target.rs
│   └── wal_replay/target.rs
├── corpus/
│   └── <target>/
└── Cargo.toml (updated with new paths)
```

**Done When:**
- [ ] New directory structure created
- [ ] All fuzz targets moved to subdirectories
- [ ] `fuzz/Cargo.toml` updated with new paths
- [ ] All fuzz targets compile: `cargo +nightly fuzz build --all`
- [ ] Old flat structure removed

**Acceptance Criteria:**
- Binary: Directory structure matches specification (yes/no)
- Binary: Cargo.toml has correct paths (yes/no)
- Binary: All fuzz targets compile (yes/no)

**Risk:** R10.1 (Refactor takes longer than expected)

---

### W10.2a: Fix hnsw_insert Fuzz Target

**Priority:** P0 (Critical Path)
**Dependencies:** W10.1
**Raw Estimate:** 6h
**3x Estimate:** 18h

**Description:**
Fix API mismatches in `hnsw_insert/target.rs` to match current HNSW graph API.

**Known Issues:**
- `insert()` method signature changed
- `VectorProvider` trait implementation needs update
- Missing proper error handling for graph operations

**Done When:**
- [ ] hnsw_insert compiles without errors
- [ ] hnsw_insert runs for 60s without panics
- [ ] Proper assertions added (graph invariants maintained)
- [ ] Test passes in CI: `cargo +nightly fuzz run hnsw_insert -- -max_total_time=60`

**Acceptance Criteria:**
- Binary: Code compiles (yes/no)
- Binary: 60s run completes without panic (yes/no)
- Binary: Assertions present in code (yes/no)

---

### W10.2b: Fix hnsw_search Fuzz Target

**Priority:** P0 (Critical Path)
**Dependencies:** W10.2a
**Raw Estimate:** 6h
**3x Estimate:** 18h

**Description:**
Fix API mismatches in `hnsw_search/target.rs` to match current HNSW search API.

**Known Issues:**
- `Searcher` constructor signature changed
- `search_layer` method requires updated parameters
- `VectorProvider` trait needs matching implementation

**Done When:**
- [ ] hnsw_search compiles without errors
- [ ] hnsw_search runs for 60s without panics
- [ ] Search result validation added (proper distance ordering)
- [ ] Test passes in CI

**Acceptance Criteria:**
- Binary: Code compiles (yes/no)
- Binary: 60s run completes without panic (yes/no)
- Binary: Result validation present (yes/no)

---

### W10.2c: Fix graph_ops Fuzz Target

**Priority:** P0 (Critical Path)
**Dependencies:** W10.2b
**Raw Estimate:** 6h
**3x Estimate:** 18h

**Description:**
Fix API mismatches in `graph_ops/target.rs` to match current graph operations API.

**Known Issues:**
- `set_neighbors()` method signature changed
- Graph node operations need updated parameters
- Missing validation for neighbor list bounds

**Done When:**
- [ ] graph_ops compiles without errors
- [ ] graph_ops runs for 60s without panics
- [ ] Neighbor bounds validation added
- [ ] Test passes in CI

**Acceptance Criteria:**
- Binary: Code compiles (yes/no)
- Binary: 60s run completes without panic (yes/no)
- Binary: Bounds validation present (yes/no)

---

### W10.2d: Fix search_robustness Fuzz Target

**Priority:** P0 (Critical Path)
**Dependencies:** W10.2c
**Raw Estimate:** 6h
**3x Estimate:** 18h

**Description:**
Fix API mismatches in `search_robustness/target.rs` to match current HNSW search robustness testing.

**Known Issues:**
- Multiple API changes: `add_node()`, `set_neighbors()`, `Searcher` constructor
- `VectorProvider` trait implementation needs update
- Search layer parameter changes

**Done When:**
- [ ] search_robustness compiles without errors
- [ ] search_robustness runs for 60s without panics
- [ ] Random entry point handling validated
- [ ] Test passes in CI

**Acceptance Criteria:**
- Binary: Code compiles (yes/no)
- Binary: 60s run completes without panic (yes/no)
- Binary: Entry point handling validated (yes/no)

---

### W10.3: Generate Corpus Seeds for All HNSW Fuzz Targets

**Priority:** P0 (Critical Path)
**Dependencies:** W10.2d (all HNSW fuzz targets compile and run)
**Raw Estimate:** 12h
**3x Estimate:** 36h

**Description:**
Create corpus seed files for HNSW fuzz targets to improve fuzzing effectiveness and reproducibility.

**Targets:**
- `hnsw_insert/corpus/` — seed inputs for insert operations
- `hnsw_search/corpus/` — seed inputs for search operations
- `search_robustness/corpus/` — seed inputs for edge case entry points
- `graph_ops/corpus/` — seed inputs for graph operations

**Done When:**
- [ ] Each target has ≥10 corpus seed files
- [ ] Seeds cover edge cases (empty graphs, max neighbors, boundary values)
- [ ] Documentation added explaining corpus generation methodology
- [ ] CI can reproduce fuzzing with seed corpus

**Acceptance Criteria:**
- Binary: Each target has ≥10 seeds (file count)
- Binary: Edge cases covered (code review of seeds)
- Binary: Methodology documented (yes/no)
- Binary: CI reproduces fuzzing (workflow passes)

**Risk:** R10.2 (Corpus generation complexity)

---

### W10.4: Implement HNSW Property Tests

**Priority:** P1 (High Priority)
**Dependencies:** W10.3
**Raw Estimate:** 18h
**3x Estimate:** 54h

**Description:**
Create property-based tests using `proptest` to verify HNSW graph invariants.

**Properties to Test:**
1. **Connectivity:** All inserted vectors are reachable from entry point
2. **Level Distribution:** Level assignments follow exponential decay
3. **Neighbor Consistency:** If A is neighbor of B, distance(A,B) is reasonable
4. **Search Recall:** For exact search (k=1), result is provably within top-k
5. **No Orphans:** No vectors are unreachable from entry point

**Done When:**
- [ ] All 5 properties implemented as proptest tests
- [ ] Tests pass with 1000 random test cases
- [ ] Tests integrated into CI: `cargo test --test hnsw_properties`
- [ ] Documentation explains each property and why it matters

**Acceptance Criteria:**
- Binary: 5 properties implemented (code review)
- Binary: 1000 test cases pass (CI log)
- Binary: CI integration complete (workflow file updated)
- Binary: Documentation exists (file present)

**Risk:** R10.3 (Property test failures reveal bugs), R10.4 (Property tests are flaky)

---

### W10.5: Design Batch Insert API

**Priority:** P1 (High Priority)
**Dependencies:** None (runs in parallel with critical path)
**Raw Estimate:** 4h
**3x Estimate:** 12h

**Description:**
Design the public API for batch insert operations to enable Week 11 implementation.

**Deliverables:**
1. **RFC Document** (`docs/rfcs/0001-batch-insert-api.md`) with:
   - Trait signature for `BatchInsertable`
   - Error handling strategy (fail-fast vs. partial success)
   - WASM memory budget analysis
   - Progress reporting mechanism
   - Example usage code

2. **API Sketch** (not implemented, just designed):
```rust
pub trait BatchInsertable {
    fn insert_batch(&mut self, vectors: &[&[f32]]) -> Result<Vec<VectorId>, BatchError>;
    fn insert_batch_with_progress<F>(&mut self, vectors: &[&[f32]], progress: F)
        -> Result<Vec<VectorId>, BatchError>
    where F: FnMut(usize, usize);
}
```

**Scope Boundaries (Explicit):**
- Design ONLY — no implementation in Week 10
- Memory budget must be <100MB for 10k vector batch
- Error strategy must be decided (fail-fast OR partial)
- Must work in WASM (no `std::thread`)

**Done When:**
- [ ] RFC document created in `docs/rfcs/0001-batch-insert-api.md`
- [ ] Trait signatures documented
- [ ] Error handling strategy chosen and justified
- [ ] WASM memory budget analyzed
- [ ] Example usage code included
- [ ] Document reviewed and approved (can proceed to W11.1)

**Blocking Artifact for Week 11:**
This RFC document is the **blocking artifact** that unlocks:
- W11.1: Batch insert implementation
- W11.2: Batch insert benchmarks

**Acceptance Criteria:**
- Binary: RFC file exists (yes/no)
- Binary: All required sections present (checklist)
- Binary: Memory budget <100MB (calculation present)
- Binary: Example code compiles (syntax-valid, type-checks)

**Risk:** R10.6 (Batch implementation complexity discovered during design)

---

### W10.8: Create Benchmark Validation Suite

**Priority:** P1 (High Priority)
**Dependencies:** W10.3 (needs working fuzz infrastructure)
**Raw Estimate:** 12h
**3x Estimate:** 36h

**Description:**
Create automated benchmark validation suite to detect performance regressions in CI.

**Components:**
1. **Baseline Storage** — Store P50/P99 latencies for each benchmark in git
2. **Regression Detection** — Compare current run to baseline, fail if >10% slower
3. **CI Integration** — Run on every PR, post results as comment
4. **Benchmark Suite:**
   - `insert_1k` — Insert 1000 vectors
   - `search_100k` — Search in 100k vector index
   - `quantization_encode` — SQ8 encoding speed
   - `hamming_distance` — Distance calculation speed

**Done When:**
- [ ] 4 benchmarks implemented in `benches/validation/`
- [ ] Baseline values stored in `benches/baselines.json`
- [ ] Regression detection script created
- [ ] CI workflow file updated to run validation
- [ ] Example PR with benchmark results posted

**Acceptance Criteria:**
- Binary: 4 benchmarks exist (file count)
- Binary: baselines.json has all 4 entries (JSON validation)
- Binary: Script detects regression (manual test with degraded code)
- Binary: CI runs validation (workflow log shows execution)

**Risk:** R10.8 (Benchmark suite has bugs or false positives)

---

## Deferred Tasks (Week 11)

### W11.1: Implement Batch Insert (formerly W10.6)

**Priority:** P1
**Dependencies:** W10.5 (RFC document)
**Raw Estimate:** 16h
**3x Estimate:** 48h
**Reason for Deferral:** Implementation requires completed W10.5 API design

**Risk:** R10.6 (may exceed 16h if complexity discovered during W10.5)

---

### W11.2: Benchmark Batch Insert (formerly W10.7)

**Priority:** P2
**Dependencies:** W11.1
**Raw Estimate:** 12h
**3x Estimate:** 36h
**Reason for Deferral:** Cannot benchmark until implementation exists

---

## Time Budget

| Category | Raw Hours | With 3x | Percentage |
|:---------|----------:|--------:|-----------:|
| **Fuzz Infrastructure** (W10.1-W10.4) | 60h | 180h | 79% |
| **API Design** (W10.5) | 4h | 12h | 5% |
| **Benchmark Suite** (W10.8) | 12h | 36h | 16% |
| **Total** | **76h** | **228h** | **100%** |

**Critical Path:** 60h (W10.1 → W10.2a → W10.2b → W10.2c → W10.2d → W10.3 → W10.4)

**Parallel Work:** 16h (W10.5 + W10.8 can run alongside critical path)

**Week Capacity:** 40h (1 week @ 40h/week)

**Buffer Multiplier:** 228h / 40h = 5.7x (acceptable, as actual critical path is 60h × 3 = 180h = 4.5x, which is within tolerance for complex infrastructure work)

---

## Handoff Protocol

**After Week 10 Completion:**

1. **Verify Deliverables:**
   - [ ] All fuzz targets passing in CI
   - [ ] Property tests integrated
   - [ ] Batch insert RFC approved
   - [ ] Benchmark validation suite operational

2. **Unlock Week 11:**
   - W11.1 can start immediately (RFC is complete)
   - W11.2 waits for W11.1 completion

3. **Status Report:**
   - Document any deviations from plan
   - Update risk register with lessons learned
   - Submit retrospective document

---

## Approval Chain

- [x] v1.0 created (2025-12-13) — REJECTED (5 critical issues)
- [x] v2.0 created (2025-12-13) — REJECTED (3 issues: N1, N2, N3)
- [x] v3.0 created (2025-12-13) — APPROVED (0 issues)
- [x] v3.1 updated (2025-12-13) — Minor fix: Updated task descriptions to reflect actual fuzz targets
  - W10.2a: `fuzz_hamming` → `hnsw_insert`
  - W10.2b: `fuzz_encoder` → `hnsw_search`
  - W10.2c: `fuzz_quantizer` → `graph_ops`
  - W10.2d: Assertions → `search_robustness`
  - W10.3: Updated to corpus generation focus (structure already done in W10.1)

**Status:** ✅ APPROVED — Implementation phase unlocked

---

**Version:** v3.1
**Last Updated:** 2025-12-13
**Next Review:** End of Week 10
