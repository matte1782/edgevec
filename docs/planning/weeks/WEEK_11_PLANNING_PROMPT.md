# PROMPT_MAKER: Week 11 Planning Generation

**Target Agent:** PLANNER
**Task:** Generate Week 11 Planning Documents
**Output:** Complete week_11/ folder structure with daily task specifications
**Quality Standard:** Matches Week 1-10 rigor and format

---

## CONTEXT

You are the PLANNER agent tasked with creating Week 11 planning documents for EdgeVec. Week 10 has been **APPROVED** (NVIDIA-grade review complete). Week 11 focuses on **implementing the Batch Insert API** designed in W10.5 (RFC 0001).

**Prerequisites Verified:**
- ✅ Week 10 complete (all 9 tasks approved)
- ✅ RFC 0001: Batch Insert API designed and approved
- ✅ Gate 2 complete (planning → implementation unlocked)
- ✅ All dependencies met for Week 11 start

---

## PROMPT FOR PLANNER AGENT

Execute the PLANNER workflow to create Week 11 planning documents. Generate a complete week_11/ directory structure following the exact format established in weeks 1-10.

### Required Deliverables

Create the following directory structure:

```
docs/planning/weeks/week_11/
├── WEEK_11_OVERVIEW.md          # Week summary, goals, dependencies
├── DAY_1_TASKS.md                # Monday: Implementation setup
├── DAY_2_TASKS.md                # Tuesday: Core implementation
├── DAY_3_TASKS.md                # Wednesday: Testing
├── DAY_4_TASKS.md                # Thursday: Benchmarking
├── DAY_5_TASKS.md                # Friday: Documentation & review
└── RISK_REGISTER.md              # Week 11 specific risks
```

### Week 11 Scope

**Primary Goal:** Implement Batch Insert API (RFC 0001)

**Tasks to Plan:**

| Task ID | Description | Priority | Estimated |
|:--------|:------------|:---------|:----------|
| W11.1 | Implement BatchInsertable trait | P0 | 8h → 24h |
| W11.2 | Add BatchError type | P0 | 2h → 6h |
| W11.3 | Unit tests for batch insert | P0 | 4h → 12h |
| W11.4 | Integration test (10k vectors) | P1 | 4h → 12h |
| W11.5 | Benchmark batch vs sequential | P1 | 4h → 12h |
| W11.6 | Error handling tests | P1 | 3h → 9h |
| W11.7 | Progress callback tests | P2 | 2h → 6h |
| W11.8 | Update API documentation | P2 | 3h → 9h |

**Total Estimate:** 30h raw → 90h with 3x multiplier

### Template Requirements

Each DAY_X_TASKS.md file MUST contain:

#### 1. Day Header
```markdown
# Week 11 Day X — [Title]

**Date:** 2025-12-16 (Monday) [adjust for actual day]
**Focus:** [Primary goal for this day]
**Agent:** RUST_ENGINEER / TEST_ENGINEER / BENCHMARK_SCIENTIST / DOCWRITER
**Dependencies:** [List all dependencies from previous weeks/days]
```

#### 2. Theoretical Context
```markdown
## Theoretical Foundation

### Core Concept
[Explain the computer science / algorithm theory behind today's work]

### Why This Implementation
[Justify why this specific approach over alternatives]

### Related Work
- [Reference to RFC 0001 sections]
- [Reference to similar implementations in literature]
- [Reference to ARCHITECTURE.md sections]
```

#### 3. Task Breakdown
```markdown
## Task List

### W11.X: [Task Name]

**Priority:** P0 | P1 | P2
**Estimate:** Xh raw → Yh with 3x
**Agent:** [Responsible agent]

**Description:**
[2-3 sentence description of what needs to be done]

**Acceptance Criteria:**
- [ ] Criterion 1 (specific, measurable, binary)
- [ ] Criterion 2
- [ ] Criterion 3

**Implementation Notes:**
- [Technical detail 1]
- [Technical detail 2]
- [Reference to RFC section or architecture doc]

**Dependencies:**
- [Task ID or external dependency]

**Verification:**
```bash
# Command to verify completion
cargo test [specific_test]
cargo clippy -- -D warnings
```

**Files to Create/Modify:**
- `src/hnsw/batch.rs` (create)
- `src/hnsw/mod.rs` (modify - export BatchInsertable)
- `tests/batch_insert.rs` (create)
```

#### 4. Day Summary
```markdown
## Day X Summary

**Total Tasks:** X
**Priority Breakdown:**
- P0 (Blocking): X tasks
- P1 (Important): X tasks
- P2 (Nice-to-have): X tasks

**Expected Deliverables:**
1. [Deliverable 1]
2. [Deliverable 2]

**Handoff to Next Day:**
[What must be complete before Day X+1 can start]
```

### WEEK_11_OVERVIEW.md Requirements

Must contain:

```markdown
# Week 11 Overview — Batch Insert Implementation

**Date:** 2025-12-16 to 2025-12-20
**Phase:** Implementation (Phase 3)
**Gate Status:** Gate 2 complete, Gate 3 target
**Primary Goal:** Implement and validate Batch Insert API

## Week Objective

Implement the BatchInsertable trait and associated infrastructure designed in RFC 0001, enabling efficient bulk loading of vectors with progress reporting.

## Success Criteria

By end of week:
- [ ] BatchInsertable trait implemented for HnswIndex
- [ ] All unit tests pass (100% coverage for new code)
- [ ] Integration test validates 10k vector batch insert
- [ ] Benchmark shows performance vs sequential insert
- [ ] Documentation updated
- [ ] All code passes hostile review

## Dependencies

**From Previous Weeks:**
- W10.5: RFC 0001 approved (Batch Insert API design)
- W8: HnswIndex core implementation
- W6: VectorStorage implementation

**External:**
- None

## Task Distribution

| Day | Focus | Tasks | Agent |
|:----|:------|:------|:------|
| Monday | Setup & Error Types | W11.1, W11.2 | RUST_ENGINEER |
| Tuesday | Core Implementation | W11.1 (cont.) | RUST_ENGINEER |
| Wednesday | Unit Testing | W11.3, W11.6, W11.7 | TEST_ENGINEER |
| Thursday | Integration & Bench | W11.4, W11.5 | TEST_ENGINEER, BENCHMARK_SCIENTIST |
| Friday | Documentation & Review | W11.8, hostile review | DOCWRITER, HOSTILE_REVIEWER |

## Risk Register

See `RISK_REGISTER.md` for detailed risk analysis.

**Top 3 Risks:**
1. Memory budget exceeded during batch insert
2. Progress callback performance overhead
3. Error handling edge cases

## Week 11 Milestone

**Milestone:** Batch Insert API production-ready
**Deliverable:** Fully tested BatchInsertable implementation
**Next Week:** Week 12 (TBD - depends on roadmap)
```

### RISK_REGISTER.md Requirements

```markdown
# Week 11 Risk Register

| Risk ID | Description | Probability | Impact | Mitigation | Owner |
|:--------|:------------|:------------|:-------|:-----------|:------|
| W11.R1 | Memory exceeds 100MB budget | Medium | High | Monitor during integration test | RUST_ENGINEER |
| W11.R2 | Progress callback slows insert | Low | Medium | Benchmark with/without callback | BENCHMARK_SCIENTIST |
| W11.R3 | Error recovery leaves index inconsistent | Low | High | Property tests for error paths | TEST_ENGINEER |
| W11.R4 | WASM compatibility breaks | Low | Critical | Test in wasm32 target | WASM_SPECIALIST |

## Mitigation Strategies

### W11.R1: Memory Budget
- Action: Profile memory usage during W11.4 integration test
- Success: Peak memory < 100 MB for 10k vectors
- Fallback: Implement chunked batch insert

### W11.R2: Callback Overhead
- Action: Benchmark in W11.5 with callback disabled vs enabled
- Success: Overhead < 5% of total insert time
- Fallback: Make callback optional (already in design)

### W11.R3: Index Consistency
- Action: Add property test in W11.6 that inserts batch, fails mid-way, verifies state
- Success: Index remains searchable after partial failure
- Fallback: Document fail-fast behavior clearly

### W11.R4: WASM Compatibility
- Action: Compile with wasm32-unknown-unknown target
- Success: No compilation errors, no thread usage
- Fallback: Conditional compilation for native vs WASM
```

---

## FORMAT CONSISTENCY REQUIREMENTS

### 1. Markdown Structure
- All files use ATX-style headers (`#`, `##`, `###`)
- Code blocks use triple backticks with language hints
- Tables use GitHub-flavored markdown alignment (`:---`, `:---:`, `---:`)
- Task lists use `- [ ]` format

### 2. Task ID Format
- Always `W[week].[task]` (e.g., W11.1, W11.2)
- Sequential numbering within week
- Reference format: `W11.1` in text, full description in task section

### 3. Estimate Format
- Always: `Xh raw → Yh with 3x` where Y = X * 3
- Use realistic raw estimates (minimum 2h for any task)

### 4. Priority Levels
- **P0 (Blocking):** Must complete for week success
- **P1 (Important):** Should complete, week incomplete without
- **P2 (Nice-to-have):** Bonus, can defer to next week

### 5. Agent Assignment
- Use exact agent names: RUST_ENGINEER, TEST_ENGINEER, BENCHMARK_SCIENTIST, DOCWRITER, HOSTILE_REVIEWER
- One primary agent per task (can reference secondary agents in notes)

### 6. File Path Format
- Always absolute from repo root or relative from `src/`
- Use backticks: `src/hnsw/batch.rs`
- Specify (create) vs (modify) in parentheses

---

## REFERENCE MATERIALS

### RFC 0001 Sections to Reference

| Section | Content | Relevant to |
|:--------|:--------|:------------|
| Lines 82-135 | Trait definition | W11.1, W11.2 |
| Lines 138-182 | Implementation | W11.1 |
| Lines 186-240 | Error handling | W11.6 |
| Lines 244-329 | Memory budget | W11.4 |
| Lines 333-376 | Progress reporting | W11.7 |
| Lines 380-466 | Example usage | W11.8 |

### Related Architecture Documents

- `docs/architecture/ARCHITECTURE.md` - System overview
- `docs/architecture/DATA_LAYOUT.md` - Memory layout
- `docs/planning/ROADMAP.md` - Overall timeline

### Week 10 Reference

- `docs/planning/weeks/week_10/WEEK_10_OVERVIEW.md` - For format consistency
- `docs/planning/weeks/week_10/DAY_1_TASKS.md` - Example day structure

---

## VALIDATION CHECKLIST

Before considering planning complete, verify:

### Structure
- [ ] All 7 required files created
- [ ] All files follow naming convention (UPPERCASE, underscores)
- [ ] Directory structure matches template

### Content Completeness
- [ ] Each day has: Header, Theoretical Foundation, Task List, Summary
- [ ] Each task has: Priority, Estimate, Agent, Description, Acceptance Criteria, Dependencies, Verification, Files
- [ ] WEEK_11_OVERVIEW.md has all required sections
- [ ] RISK_REGISTER.md has all identified risks with mitigations

### Cross-References
- [ ] All task IDs match between days and overview
- [ ] All dependencies reference valid tasks or artifacts
- [ ] All file paths reference actual or planned files
- [ ] All RFC sections referenced are correct

### Estimates
- [ ] All estimates use 3x multiplier
- [ ] Total week estimate is reasonable (70-100h with 3x)
- [ ] No task < 2h raw estimate (too granular)
- [ ] No task > 16h raw estimate (needs decomposition)

### Quality
- [ ] No typos or grammatical errors
- [ ] Consistent terminology throughout
- [ ] Professional tone maintained
- [ ] Technical accuracy verified against RFC

---

## EXECUTION INSTRUCTIONS

1. **Read Context:**
   - Read `docs/rfcs/0001-batch-insert-api.md` completely
   - Read `docs/planning/weeks/week_10/WEEK_10_OVERVIEW.md` for format
   - Read `docs/planning/ROADMAP.md` for overall context

2. **Generate Files:**
   - Create `docs/planning/weeks/week_11/` directory
   - Generate all 7 required files
   - Follow templates exactly

3. **Distribute Tasks:**
   - Monday: W11.1 (start), W11.2 (complete)
   - Tuesday: W11.1 (complete), begin testing setup
   - Wednesday: W11.3, W11.6, W11.7 (all testing)
   - Thursday: W11.4, W11.5 (integration + benchmarks)
   - Friday: W11.8 (docs), hostile review

4. **Cross-Link:**
   - Reference RFC 0001 sections in task notes
   - Reference architecture docs where applicable
   - Link to Week 10 dependencies

5. **Validate:**
   - Run through validation checklist
   - Verify all 8 tasks are covered
   - Ensure consistency across files

---

## EXPECTED OUTPUT

After execution, the PLANNER should have created:

```
docs/planning/weeks/week_11/
├── WEEK_11_OVERVIEW.md          # 200-300 lines
├── DAY_1_TASKS.md                # 150-200 lines
├── DAY_2_TASKS.md                # 150-200 lines
├── DAY_3_TASKS.md                # 200-250 lines
├── DAY_4_TASKS.md                # 150-200 lines
├── DAY_5_TASKS.md                # 100-150 lines
└── RISK_REGISTER.md              # 50-100 lines
```

Total: ~1000-1400 lines of planning documentation

---

## HOSTILE REVIEW PREPARATION

After planning is complete, the following will be reviewed by HOSTILE_REVIEWER:

**Review Criteria:**
1. **Completeness:** All 8 tasks from scope are planned
2. **Estimates:** All tasks have 3x multiplier applied
3. **Dependencies:** All dependencies are valid and complete
4. **Acceptance Criteria:** All criteria are measurable and binary
5. **Consistency:** Format matches Week 1-10 exactly
6. **Feasibility:** Plan is achievable within 5 days
7. **Risk Coverage:** All major risks identified with mitigations
8. **Cross-References:** All RFC references are accurate

**Review Mode:** NVIDIA-GRADE HOSTILE REVIEW
**Expectation:** Zero critical or major issues

---

## BEGIN PLANNING

You are now ready to generate Week 11 planning documents.

**Command to execute:**
```
/planner-weekly 11
```

**Expected behavior:**
1. Read RFC 0001
2. Read Week 10 overview for format
3. Create week_11/ directory
4. Generate all 7 files following templates
5. Validate against checklist
6. Report completion

**Success criteria:**
- All files created
- All tasks distributed across 5 days
- All validation checks pass
- Ready for hostile review

---

**PROMPT_MAKER:** This prompt is now ready for PLANNER agent execution.
**Next Step:** Execute `/planner-weekly 11` and await hostile review.
