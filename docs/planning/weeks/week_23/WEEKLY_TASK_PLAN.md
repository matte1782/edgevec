# Week 23: Filtering Implementation Sprint

**Week:** 23
**Sprint Theme:** Filtering System Implementation
**Version Target:** v0.5.0
**Status:** [REVISED]
**Planning Date:** 2025-12-17
**Revision Date:** 2025-12-17
**Revision:** Addresses HOSTILE_REVIEWER issues [C1-C5], [M1-M5]

---

## Executive Summary

Week 23 implements the complete filtering subsystem designed in Week 22. This is the core feature for v0.5.0: metadata-aware vector search with SQL-like query syntax.

**Scope:**
- Filter parser (pest-based, 27 operator types)
- Filter evaluator (recursive tree-walk, short-circuit optimization)
- Strategy selection (hybrid with auto-selection)
- HNSW integration (filtered search API)
- WASM bindings and TypeScript wrapper
- Comprehensive test suite (1856+ tests planned)

**Total Effort:** 92 hours (7 days × ~13h/day) [+7h from revision]
**Critical Path:** Baseline → Parser → Evaluator → Strategy → Integration → WASM → Compliance

---

## Prerequisites

| Prerequisite | Status | Evidence |
|:-------------|:-------|:---------|
| GATE_W22_COMPLETE.md | APPROVED | `.claude/GATE_W22_COMPLETE.md` |
| FILTERING_API.md | APPROVED | `docs/architecture/FILTERING_API.md` |
| FILTER_TEST_STRATEGY.md | APPROVED | `docs/architecture/FILTER_TEST_STRATEGY.md` |
| Metadata subsystem | COMPLETE | `src/metadata/` |
| 305 tests passing | VERIFIED | `cargo test` |

---

## Week 23 Task Overview

| Day | Focus | Tasks | Hours | Agent |
|:----|:------|:------|:------|:------|
| Day 1 | Parser Foundation | W23.0.1 + W23.1.1 - W23.1.4 | 13h | RUST_ENGINEER |
| Day 2 | Evaluator Core | W23.2.1 - W23.2.4 | 12h | RUST_ENGINEER |
| Day 3 | Strategy & Integration | W23.3.1 - W23.3.4 | 12h | RUST_ENGINEER |
| Day 4 | WASM Bindings | W23.4.1 - W23.4.4 | 12h | WASM_SPECIALIST |
| Day 5 | TypeScript Wrapper | W23.5.1 - W23.5.4 | 12h | WASM_SPECIALIST |
| Day 6 | Testing Sprint | W23.6.1 - W23.6.8 | 18h | TEST_ENGINEER |
| Day 7 | Polish & Gate | W23.7.1 - W23.7.6 | 13h | ALL |
| **TOTAL** | | **33 Tasks** | **92h** | |

**NOTE:** Task count increased from 28→33, hours from 85→92 to address HOSTILE_REVIEWER issues.

---

## Day-by-Day Breakdown

### Day 1: Parser Foundation (13 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.0.1 | **[M5]** Record baseline performance (pre-implementation) | 1h | P0 | BENCHMARK_SCIENTIST |
| W23.1.1 | Define FilterExpr AST enum (27 variants) | 3h | P0 | RUST_ENGINEER |
| W23.1.2 | Implement pest grammar file (filter.pest) | 4h | P0 | RUST_ENGINEER |
| W23.1.3 | Build AST from pest parse tree | 3h | P0 | RUST_ENGINEER |
| W23.1.4 | **[M1]** Parser error handling with position info | 3h | P0 | RUST_ENGINEER |

**[M5] Baseline Benchmark Task (W23.0.1):**
- Run current `cargo bench` to establish pre-filter baseline
- Record: search latency without filters
- Output: `docs/benchmarks/week_23_baseline.md`
- Required for: Regression detection in W23.7.1

**[M1] Parser Error Handling Estimate Adjustment:**
- Original: 2h → Revised: 3h (9 error variants in FILTER_PARSER.md)

**Acceptance Criteria:**
- [ ] `parse("category = \"gpu\"")` returns `FilterExpr::Eq(...)`
- [ ] All 27 operators parse correctly
- [ ] Error messages include line, column, and suggestion
- [ ] `cargo test filter::parser` passes

**Dependencies:** FILTERING_SYNTAX.md (grammar specification)

---

### Day 2: Evaluator Core (12 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.2.1 | Core recursive evaluate() function | 4h | P0 | RUST_ENGINEER |
| W23.2.2 | Comparison operators (=, !=, <, <=, >, >=) | 3h | P0 | RUST_ENGINEER |
| W23.2.3 | String operators (CONTAINS, LIKE, etc.) | 3h | P0 | RUST_ENGINEER |
| W23.2.4 | Array operators (IN, ANY, ALL, NONE) | 2h | P0 | RUST_ENGINEER |

**Acceptance Criteria:**
- [ ] `evaluate(expr, metadata)` returns `Result<bool, FilterError>`
- [ ] Short-circuit optimization for AND/OR
- [ ] Type coercion handles int/float promotion
- [ ] `cargo test filter::evaluator` passes

**Dependencies:** W23.1.1 (FilterExpr AST)

**[M2] Added Dependency:** W23.2.4 (Array operators) depends on W23.2.3 (String operators) for pattern matching foundation.

---

### Day 3: Strategy & Integration (12 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.3.1 | FilterStrategy enum and configuration | 2h | P0 | RUST_ENGINEER |
| W23.3.2 | Selectivity estimation via sampling | 3h | P0 | RUST_ENGINEER |
| W23.3.3 | search_filtered() public API | 4h | P0 | RUST_ENGINEER |
| W23.3.4 | Edge case handlers (contradiction, tautology) | 3h | P0 | RUST_ENGINEER |

**Acceptance Criteria:**
- [ ] Auto-selection chooses correct strategy based on selectivity
- [ ] Pre-filter path works for high selectivity (>80%)
- [ ] Post-filter path works for low selectivity (<5%)
- [ ] `cargo test filter::strategy` passes

**Dependencies:** W23.2.1 (evaluator)

---

### Day 4: WASM Bindings (12 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.4.1 | parse_filter_js() WASM export | 3h | P0 | WASM_SPECIALIST |
| W23.4.2 | search_filtered_js() WASM export | 4h | P0 | WASM_SPECIALIST |
| W23.4.3 | FilterError → JsValue serialization | 2h | P0 | WASM_SPECIALIST |
| W23.4.4 | JSON serialization for FilterExpr | 3h | P0 | WASM_SPECIALIST |

**Acceptance Criteria:**
- [ ] `parse_filter_js("category = \"gpu\"")` callable from JS
- [ ] `search_filtered_js(...)` returns JSON results
- [ ] Error includes code, message, position, suggestion
- [ ] `wasm-pack test --node` passes

**Dependencies:** W23.3.3 (search_filtered API)

---

### Day 5: TypeScript Wrapper (12 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.5.1 | Filter static class (eq, ne, lt, etc.) | 3h | P0 | WASM_SPECIALIST |
| W23.5.2 | FilterBuilder fluent API | 4h | P0 | WASM_SPECIALIST |
| W23.5.3 | EdgeVecIndex.searchFiltered() method | 3h | P0 | WASM_SPECIALIST |
| W23.5.4 | TypeScript type definitions (.d.ts) | 2h | P0 | WASM_SPECIALIST |

**Acceptance Criteria:**
- [ ] `Filter.eq('category', 'gpu')` works
- [ ] `new FilterBuilder().where('price').lt(500).build()` works
- [ ] `index.searchFiltered(query, 10, { filter })` works
- [ ] All examples from FILTERING_WASM_API.md pass

**Dependencies:** W23.4.1-4.4 (WASM exports)

---

### Day 6: Testing Sprint (18 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.6.1 | Parser unit tests (344 tests) | 3h | P0 | TEST_ENGINEER |
| W23.6.2 | Evaluator unit tests (804 tests) | 4h | P0 | TEST_ENGINEER |
| W23.6.3 | Strategy unit tests (408 tests) | 2h | P0 | TEST_ENGINEER |
| W23.6.4 | Property tests (17 invariants) | 2h | P0 | TEST_ENGINEER |
| W23.6.5a | **[C4]** Fuzz target - simple expressions | 2h | P0 | TEST_ENGINEER |
| W23.6.5b | **[C4]** Fuzz target - deeply nested (max depth 50) | 2h | P0 | TEST_ENGINEER |
| W23.6.6 | Integration tests (HNSW + filter) | 1h | P0 | TEST_ENGINEER |
| W23.6.7 | **[C5]** Multi-field filter integration tests | 1h | P0 | TEST_ENGINEER |
| W23.6.8 | **[M3]** Test count verification (≥1856) | 1h | P0 | TEST_ENGINEER |

**[C4] Fuzz Target Split (W23.6.5a + W23.6.5b):**
- W23.6.5a: Simple expressions, basic patterns (10M iterations)
- W23.6.5b: Deeply nested expressions (max AST depth 50), adversarial patterns
- Acceptance: Must survive 10M iterations without crash/panic
- Example: `A AND (B OR (C AND (D OR ...)))` (depth 50)

**[C5] Multi-Field Filter Test (W23.6.7):**
- Test case: Multi-field filter with search ranking
- Index vectors with metadata: `{category: str, price: f32, tags: Vec<str>}`
- Filter: `category = "electronics" AND price < 100 AND tags ANY ["sale"]`
- Verify: Results are both filtered AND ranked by similarity

**[M3] Test Count Verification (W23.6.8):**
- Run: `cargo test --lib -- --list | wc -l` → Output: ≥1856 tests
- Generate coverage: `cargo tarpaulin` → Coverage: ≥90%

**Acceptance Criteria:**
- [ ] 1856+ tests passing (verified via W23.6.8)
- [ ] 17 property invariants verified
- [ ] 6 fuzz targets run clean (W23.6.5a + W23.6.5b + original 3)
- [ ] `cargo tarpaulin` shows >90% coverage
- [ ] Multi-field filters work correctly with search (W23.6.7)

**Dependencies:** W23.1-5 (all implementation)

---

### Day 7: Polish & Gate (13 hours)

| Task ID | Description | Hours | Priority | Agent |
|:--------|:------------|:------|:---------|:------|
| W23.7.1 | **[C2]** Performance benchmarks validation | 3h | P0 | BENCHMARK_SCIENTIST |
| W23.7.2 | WASM bundle size verification (<500KB) | 1h | P0 | WASM_SPECIALIST |
| W23.7.3 | Documentation and examples | 3h | P1 | DOCWRITER |
| W23.7.4 | **[C1]** Architecture compliance audit | 2h | P0 | RUST_ENGINEER |
| W23.7.5 | **[M4]** Hostile review and gate approval | 4h | P0 | HOSTILE_REVIEWER |

**[C1] Architecture Compliance Audit (W23.7.4):**
Verify implementation matches ALL architecture specifications:
- [ ] FILTER_PARSER.md compliance verified
- [ ] FILTER_EVALUATOR.md compliance verified
- [ ] FILTER_STRATEGY.md compliance verified
- [ ] FILTERING_WASM_API.md compliance verified
- [ ] FILTERING_API.md compliance verified
Output: Architecture compliance report with signed statements

**[C2] Performance Validation (W23.7.1):**
Validate against architecture targets:
- [ ] Filter parsing: <100μs (P99)
- [ ] Filter evaluation: <1ms for 1000-record set (P99)
- [ ] Memory overhead: <5KB per FilterState (measured)
- [ ] Search + filter: <10ms P99 at 100k vectors
- [ ] No regression vs W23.0.1 baseline

**Acceptance Criteria:**
- [ ] <10ms P99 search latency with filter (100k vectors)
- [ ] WASM bundle <500KB gzipped
- [ ] README updated with filter examples
- [ ] All 5 architecture docs compliance verified (W23.7.4)
- [ ] GATE_W23_COMPLETE.md created

**Dependencies:** W23.6 (all tests passing)

---

## Critical Path

```
W23.1 (Parser, 12h)
    │
    └──> W23.2 (Evaluator, 12h)
              │
              └──> W23.3 (Strategy, 12h)
                        │
                        ├──> W23.4 (WASM, 12h) ──> W23.5 (TS, 12h)
                        │
                        └──> W23.6 (Tests, 14h)
                                   │
                                   └──> W23.7 (Polish, 11h)

Critical Path Length: 73 hours (sequential)
With Parallelization (Days 4-5 || Days 6): ~61 hours
```

---

## Performance Targets

| Metric | Target | Measurement |
|:-------|:-------|:------------|
| Parse time | <1ms | Typical 3-clause filter |
| Evaluate time | <5μs | Per vector |
| Search latency (filtered) | <10ms P99 | 100k vectors, any selectivity |
| Selectivity estimation | <100μs | 100 sample points |
| WASM boundary overhead | <200μs | Round-trip |
| Bundle size | <500KB | gzipped WASM |

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|:-----|:------------|:-------|:-----------|
| Parser complexity | Medium | High | Use pest (well-documented), start simple |
| LIKE pattern ReDoS | Medium | High | Iterative impl, max pattern length |
| Performance miss | Low | High | Benchmark early, optimize hot paths |
| WASM size bloat | Low | Medium | Feature flags, dead code elimination |
| Test coverage gaps | Medium | Medium | Property tests + fuzzing |
| **[C3]** WASM breaking changes | Medium | High | Rollback procedure (see below) |
| Integration test flakiness | Medium | Low | Deterministic harness, retry logic |

---

## [C3] WASM Breaking Changes Rollback Plan

**Purpose:** Provide recovery path if v0.5.0 filtering breaks existing users.

### Branch Strategy
1. Tag current `main` as `v0.4.x-stable` before Week 23 begins
2. Create branch `week23-filtering` for all W23 implementation
3. Gate 3 merge requires migration guide complete

### Rollback Procedure
```bash
# If v0.5.0 breaks production:
git checkout v0.4.x-stable
npm publish --tag rollback
```

### Migration Requirements
- [ ] Migration guide (v0.4 → v0.5) in `docs/MIGRATION_v05.md`
- [ ] Deprecation notices (not removal) for 2 releases
- [ ] TypeScript types backward compatible where possible
- [ ] Tested fallback path documented

### Gate 3 Rollback Criteria
Gate 3 approval requires:
- [ ] Migration guide reviewed and complete
- [ ] No silent breaking changes (all must be documented)
- [ ] Rollback tested on sample project

---

## Agent Assignments

| Agent | Tasks | Total Hours |
|:------|:------|:------------|
| RUST_ENGINEER | W23.1, W23.2, W23.3 | 36h |
| WASM_SPECIALIST | W23.4, W23.5 | 24h |
| TEST_ENGINEER | W23.6 | 14h |
| BENCHMARK_SCIENTIST | W23.7.1 | 3h |
| DOCWRITER | W23.7.3 | 3h |
| HOSTILE_REVIEWER | W23.7.4 | 4h |
| **TOTAL** | | **84h** |

---

## Deliverables

### Code Artifacts
- `src/filter/mod.rs` - Module root
- `src/filter/parser.rs` - Filter parser (pest)
- `src/filter/filter.pest` - Grammar file
- `src/filter/ast.rs` - FilterExpr AST
- `src/filter/evaluator.rs` - Evaluation logic
- `src/filter/strategy.rs` - Strategy selection
- `src/filter/error.rs` - Error types
- `src/wasm/filter.rs` - WASM bindings
- `wasm/src/filter.ts` - TypeScript wrapper
- `wasm/src/filter-builder.ts` - Builder pattern

### Test Artifacts
- `tests/filter_parser_tests.rs`
- `tests/filter_evaluator_tests.rs`
- `tests/filter_strategy_tests.rs`
- `tests/filter_integration_tests.rs`
- `fuzz/fuzz_targets/fuzz_parser.rs`
- `fuzz/fuzz_targets/fuzz_evaluator.rs`

### Documentation
- README.md filter section
- TypeScript JSDoc comments
- Inline Rust doc comments

---

## Gate Requirements [M4 EXPANDED]

**GATE_W23_COMPLETE.md requires:**

### 1. Code Quality (All must pass)
- [ ] All 33 tasks marked DONE (updated from 28)
- [ ] Zero compiler warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No `unsafe` blocks without safety proof
- [ ] No `unwrap()` in library code (only tests)
- [ ] `cargo build --release` succeeds
- [ ] `wasm-pack build --target web` succeeds

### 2. Test Quality (All must pass)
- [ ] All tests pass: `cargo test --all-features`
- [ ] Test count ≥1856: `cargo test --lib -- --list | wc -l`
- [ ] Coverage ≥90%: `cargo tarpaulin --out Stdout`
- [ ] Property tests pass 10k iterations: `cargo test --release -- --ignored`
- [ ] 6 fuzz targets survive 10M iterations each
- [ ] `wasm-pack test --node` - WASM tests pass

### 3. Performance Quality (All must pass)
- [ ] Benchmark report generated: `docs/benchmarks/week_23_report.md`
- [ ] Filter parsing <100μs (P99) validated
- [ ] Filter evaluation <1ms for 1000 records (P99) validated
- [ ] Memory overhead <5KB per FilterState measured
- [ ] Search + filter <10ms P99 at 100k vectors
- [ ] No regression vs W23.0.1 baseline
- [ ] Bundle size <500KB gzipped verified

### 4. Architecture Compliance (All must pass)
- [ ] FILTER_PARSER.md compliance verified
- [ ] FILTER_EVALUATOR.md compliance verified
- [ ] FILTER_STRATEGY.md compliance verified
- [ ] FILTERING_WASM_API.md compliance verified
- [ ] FILTERING_API.md compliance verified
- [ ] Compliance report signed by RUST_ENGINEER

### 5. Documentation Quality (All must pass)
- [ ] All public APIs documented with examples
- [ ] All error types documented with mitigation
- [ ] Migration guide (v0.4 → v0.5) complete
- [ ] CHANGELOG.md updated with breaking changes
- [ ] README filter section complete

### 6. WASM Quality (All must pass)
- [ ] WASM bundle builds: `wasm-pack build --target web`
- [ ] WASM bundle <500KB gzipped
- [ ] TypeScript types match Rust API
- [ ] Browser tests pass (Chrome, Firefox, Safari)

### 7. Rollback Ready (All must pass)
- [ ] v0.4.x-stable tag exists
- [ ] Migration guide reviewed
- [ ] Rollback procedure documented and tested

### 8. Hostile Review (Final gate)
- [ ] All critical issues resolved
- [ ] All major issues resolved
- [ ] Minor issues documented in backlog
- [ ] HOSTILE_REVIEWER signs off

---

## Revision History

| Version | Date | Author | Change |
|:--------|:-----|:-------|:-------|
| 1.0.0 | 2025-12-17 | PLANNER | Initial Week 23 plan |
| 1.1.0 | 2025-12-17 | PLANNER | Address HOSTILE_REVIEWER issues [C1-C5], [M1-M5] |

### Revision 1.1.0 Changes
- **[C1]** Added W23.7.4: Architecture Compliance Audit task
- **[C2]** Expanded W23.7.1 with explicit performance validation targets
- **[C3]** Added WASM Breaking Changes Rollback Plan section
- **[C4]** Split W23.6.5 into W23.6.5a (simple) + W23.6.5b (deep nesting)
- **[C5]** Added W23.6.7: Multi-field filter integration tests
- **[M1]** Adjusted W23.1.4 estimate from 2h → 3h
- **[M2]** Added dependency W23.2.4 → W23.2.3 (pattern matching)
- **[M3]** Added W23.6.8: Test count verification task
- **[M4]** Expanded Gate Requirements with detailed 8-section checklist
- **[M5]** Added W23.0.1: Baseline benchmark task (pre-implementation)

**Net Impact:** +5 tasks, +7 hours (85h → 92h)

---

**PLANNER: Week 23 Plan Revised**

Status: [REVISED] - Addresses all HOSTILE_REVIEWER critical and major issues

Next: `/review WEEKLY_TASK_PLAN.md`

---

*"Implementation is the crucible where architecture meets reality."*
