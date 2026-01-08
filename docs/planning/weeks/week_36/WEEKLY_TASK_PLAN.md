# Week 36: v0.9.0 Kickoff — Documentation Polish + Sparse Vectors Design

**Date Range:** 2026-02-03 to 2026-02-09
**Focus:** Fix v0.8.0 documentation gaps, then begin Sparse Vectors
**Hours:** 12h (2h/day)
**Status:** [ ] PROPOSED

---

## Context

Week 36 marks the start of v0.9.0 (Hybrid Search + Community Features). Before implementing new features, we must first:

1. **Fix v0.8.0 documentation gaps** — Vue/React guides, Functional Filters guide
2. **Clean up outdated docs** — Delete API_REFERENCE.md (v0.3.0), update version tags
3. **Begin Sparse Vectors design** — Core v0.9.0 feature, no external dependencies

**HOSTILE_REVIEWER Note:** Documentation gaps from v0.8.0 must be addressed before new feature code.

---

## Week 36 Tasks Overview

| Day | Task | Hours | Priority |
|:----|:-----|:------|:---------|
| 1 | Vue Integration Guide | 2h | P0 |
| 2 | React Integration Guide | 2h | P0 |
| 3 | Functional Filters Guide + Doc Cleanup | 2h | P0 |
| 4 | Sparse Vectors RFC | 2h | P0 |
| 5 | Sparse Vectors Design Review | 2h | P0 |
| 6 | Week 36 Review + Week 37 Planning | 2h | P1 |
| 7 | Buffer / Hostile Review | - | - |

**Total:** 12 hours

---

## Day 1: Vue Integration Guide (2h)

**File:** `docs/guides/VUE_INTEGRATION.md`
**Source:** Extract from `pkg/README.md` lines 259-320 + CHANGELOG v0.8.0

### Tasks

- [ ] **1.1** Create `docs/guides/VUE_INTEGRATION.md` (1h)
  - Copy Vue examples from pkg/README.md
  - Add table of contents
  - Include `useEdgeVec` API reference
  - Include `useSearch` API reference
  - Add TypeScript setup instructions

- [ ] **1.2** Add advanced patterns (30min)
  - Debounce configuration
  - Error handling
  - Cleanup on unmount
  - Combining with Vue Router

- [ ] **1.3** Cross-reference from docs/api/README.md (30min)
  - Add link to Vue guide
  - Update "Framework Support" section

### Acceptance Criteria

- [ ] Vue guide exists at `docs/guides/VUE_INTEGRATION.md`
- [ ] All code examples copy-paste runnable
- [ ] TypeScript types documented
- [ ] Linked from docs/api/README.md

---

## Day 2: React Integration Guide (2h)

**File:** `docs/guides/REACT_INTEGRATION.md`
**Source:** Extract from `pkg/README.md` lines 223-258 + CHANGELOG v0.8.0

### Tasks

- [ ] **2.1** Create `docs/guides/REACT_INTEGRATION.md` (1h)
  - Copy React examples from pkg/README.md
  - Add table of contents
  - Include `useEdgeVec` hook API
  - Include `useSearch` hook API
  - Add TypeScript setup instructions

- [ ] **2.2** Add advanced patterns (30min)
  - Debounce configuration
  - Error boundaries
  - Suspense integration (if applicable)
  - Context provider pattern

- [ ] **2.3** Cross-reference from docs/api/README.md (30min)
  - Add link to React guide
  - Update "Framework Support" section

### Acceptance Criteria

- [ ] React guide exists at `docs/guides/REACT_INTEGRATION.md`
- [ ] All code examples copy-paste runnable
- [ ] TypeScript types documented
- [ ] Linked from docs/api/README.md

---

## Day 3: Functional Filters Guide + Doc Cleanup (2h)

### Part A: Functional Filters Guide (1h)

**File:** `docs/api/FUNCTIONAL_FILTERS.md`
**Source:** Extract from `pkg/README.md` lines 213-257

#### Tasks

- [ ] **3.1** Create `docs/api/FUNCTIONAL_FILTERS.md` (45min)
  - Document all filter functions:
    - Comparison: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`
    - String: `contains`, `startsWith`, `endsWith`
    - Logical: `and`, `or`, `not`, `all`, `any`
  - Show composition patterns
  - Compare string syntax vs functional API

- [ ] **3.2** Cross-reference from FILTER_SYNTAX.md (15min)
  - Add link to functional API guide
  - Note when to use each approach

### Part B: Documentation Cleanup (1h)

#### Tasks

- [ ] **3.3** DELETE `docs/API_REFERENCE.md` (5min)
  - File is v0.3.0 — 3 major versions behind
  - docs/api/ already provides better organization
  - Update any links pointing to it

- [ ] **3.4** Update version tags in API docs (40min)
  - Change v0.7.0 → v0.8.0 in headers:
    - `docs/api/WASM_INDEX.md`
    - `docs/api/TYPESCRIPT_API.md`
    - `docs/api/DATABASE_OPERATIONS.md`
    - `docs/api/ERROR_REFERENCE.md`
    - `docs/api/FILTER_SYNTAX.md`
    - `docs/api/README.md`

- [ ] **3.5** Archive old checklist (15min)
  - Move `docs/RELEASE_CHECKLIST_v0.4.md` to `docs/release/v0.4.0/`
  - Add "archived" note

### Acceptance Criteria

- [ ] Functional filters guide exists
- [ ] API_REFERENCE.md deleted
- [ ] All API docs show v0.8.0
- [ ] No broken links

---

## Day 4: Sparse Vectors RFC (2h)

**File:** `docs/rfcs/RFC_SPARSE_VECTORS.md`
**Goal:** Define Sparse Vector specification for v0.9.0

### Tasks

- [ ] **4.1** Create RFC document (1.5h)
  - Problem statement (why sparse vectors?)
  - Use cases (BM25 hybrid search, keyword matching)
  - Design:
    - CSR format (indices + values)
    - Distance metrics (dot product, cosine)
    - Storage strategy
    - WASM bindings
  - API design (Rust + TypeScript)
  - Performance targets (<1μs dot product)
  - Memory budget

- [ ] **4.2** Define acceptance criteria (30min)
  - Test coverage requirements
  - Benchmark thresholds
  - Browser compatibility
  - Bundle size impact (<100KB added)

### Acceptance Criteria

- [ ] RFC exists at `docs/rfcs/RFC_SPARSE_VECTORS.md`
- [ ] Design is complete and specific
- [ ] Performance targets defined
- [ ] Ready for hostile review

---

## Day 5: Sparse Vectors Design Review (2h)

### Tasks

- [ ] **5.1** Submit RFC for hostile review (30min)
  - Run `/review docs/rfcs/RFC_SPARSE_VECTORS.md`
  - Address any critical issues

- [ ] **5.2** Review existing inverted index patterns (1h)
  - Check `src/filter/` for relevant patterns
  - Review `src/metadata/` storage patterns
  - Identify reusable components

- [ ] **5.3** Create module structure plan (30min)
  - Define files to create:
    - `src/sparse/mod.rs`
    - `src/sparse/vector.rs`
    - `src/sparse/metrics.rs`
    - `src/sparse/index.rs`
  - Define test files

### Acceptance Criteria

- [ ] RFC approved or issues addressed
- [ ] Module structure defined
- [ ] Ready to begin implementation in Week 37

---

## Day 6: Week 36 Review + Week 37 Planning (2h)

### Tasks

- [ ] **6.1** Week 36 completion check (30min)
  - All documentation guides created
  - All API docs updated to v0.8.0
  - Sparse RFC approved
  - No orphan links

- [ ] **6.2** Create Week 37 task plan (1h)
  - Sparse Vector implementation (core type)
  - Distance metrics implementation
  - Property tests
  - 12h estimated

- [ ] **6.3** Check @jsonMartin RFC status (30min)
  - No RFC yet? Note in Week 37 plan
  - RFC received? Begin planning Flat Index
  - Update ROADMAP conditional status

### Acceptance Criteria

- [ ] Week 36 100% complete
- [ ] Week 37 plan created
- [ ] @jsonMartin status documented

---

## Day 7: Buffer / Hostile Review

Reserved for:
- Addressing hostile review feedback
- Spillover from Days 1-6
- Emergency fixes

---

## Dependencies

| Dependency | Status | Impact |
|:-----------|:-------|:-------|
| pkg/README.md (source) | EXISTS | Extract framework examples |
| CHANGELOG v0.8.0 | EXISTS | Reference for feature docs |
| @jsonMartin RFC | AWAITED | Week 35 cutoff not yet reached |

---

## Success Metrics

| Metric | Target |
|:-------|:-------|
| Documentation guides created | 3 (Vue, React, Filters) |
| Outdated docs removed | 1 (API_REFERENCE.md) |
| Version tags updated | 6 files |
| Sparse RFC approved | Yes |
| Tests passing | 700+ |

---

## Risk Mitigation

| Risk | Likelihood | Mitigation |
|:-----|:-----------|:-----------|
| Documentation takes longer | MEDIUM | Day 7 buffer |
| Sparse RFC rejected | LOW | Clear design from ROADMAP |
| @jsonMartin RFC arrives early | LOW | Good problem — accelerate Flat Index |

---

## Commit Message Template

```
docs(v0.8.0): add Vue/React integration guides

- Create docs/guides/VUE_INTEGRATION.md
- Create docs/guides/REACT_INTEGRATION.md
- Create docs/api/FUNCTIONAL_FILTERS.md
- Delete outdated docs/API_REFERENCE.md (v0.3.0)
- Update version tags v0.7.0 → v0.8.0 in API docs

Addresses v0.8.0 documentation gaps before v0.9.0 feature work.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

## Week 36 Exit Criteria

Week 36 is complete when:
- [ ] Vue integration guide exists and is linked
- [ ] React integration guide exists and is linked
- [ ] Functional filters guide exists and is linked
- [ ] API_REFERENCE.md deleted
- [ ] All API docs show v0.8.0
- [ ] Sparse Vectors RFC approved
- [ ] Week 37 plan created
- [ ] @jsonMartin RFC status documented

---

**Agent:** DOCWRITER + PLANNER
**Hours:** 12h total
**Priority:** P0 (Documentation debt must clear before v0.9.0 features)
