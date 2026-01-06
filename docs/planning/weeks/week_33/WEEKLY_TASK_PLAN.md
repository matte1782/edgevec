# Week 33: TypeScript SDK Improvements Phase 1

**Date Range:** 2026-01-13 to 2026-01-19
**Version Target:** v0.8.0 (Milestone 8.2)
**Author:** PLANNER
**Status:** [PROPOSED]

---

## Executive Summary

Week 33 focuses on **TypeScript SDK Improvements** — the second milestone of v0.8.0. This week delivers:

1. Typed Filter Functions (functional composition API)
2. React Hooks (`useEdgeVec`, `useSearch`)
3. Hook documentation and examples

**Total Hours:** 12 hours
**Buffer:** 3 hours (25%)
**Working Hours:** 15 hours across 7 days (~2h/day)

---

## Week 33 Objectives

| ID | Objective | Hours | Deliverable |
|:---|:----------|:------|:------------|
| W33.1 | Typed Filter Functions | 4h | `pkg/filter-functions.ts` |
| W33.2 | React Hooks | 6h | `pkg/react/index.ts` |
| W33.3 | Documentation & Examples | 2h | README updates, examples |

---

## Daily Breakdown

| Day | Date | Focus | Hours | Tasks |
|:----|:-----|:------|:------|:------|
| 1 | 2026-01-13 | Research & Design | 2h | W33.1.1: Design typed filter API |
| 2 | 2026-01-14 | Filter Functions | 2h | W33.1.2: Implement filter functions |
| 3 | 2026-01-15 | React Hook Design | 2h | W33.2.1: Design hooks API, setup |
| 4 | 2026-01-16 | useEdgeVec Hook | 2h | W33.2.2: Implement useEdgeVec |
| 5 | 2026-01-17 | useSearch Hook | 2h | W33.2.3: Implement useSearch |
| 6 | 2026-01-18 | Documentation | 2h | W33.3.1: README, examples |
| 7 | 2026-01-19 | Testing & Review | 3h | W33.T: Tests, hostile review |

---

## Task Details

### W33.1: Typed Filter Functions (4 hours)

**Objective:** Add functional composition API for type-safe filter building.

**Current State:**
- `FilterBuilder` class exists (fluent/chainable API)
- `Filter` class exists (static methods)
- No standalone composable functions

**Target State:**
```typescript
// Functional composition (new)
import { filter, and, or, eq, gt, lt, between } from 'edgevec';

const query = filter(
  and(
    eq('category', 'electronics'),
    gt('price', 100),
    or(
      eq('brand', 'Apple'),
      eq('brand', 'Samsung')
    )
  )
);

// Type-safe: TypeScript catches errors at compile time
const bad = eq('price', 'not-a-number'); // Type error for numeric field
```

**Subtasks:**

| ID | Task | Hours | Verification |
|:---|:-----|:------|:-------------|
| W33.1.1 | Design API & type signatures | 1h | RFC-style doc in DAY_1 |
| W33.1.2 | Implement core functions (eq, ne, gt, lt, ge, le) | 1.5h | Unit tests pass |
| W33.1.3 | Implement combinators (and, or, not, filter) | 1h | Unit tests pass |
| W33.1.4 | Export from index.ts, update types | 0.5h | TypeScript compiles |

**Files to Create/Modify:**
- `pkg/filter-functions.ts` (NEW)
- `pkg/index.ts` (add exports)

**Acceptance Criteria:**
- [ ] All comparison functions implemented (eq, ne, gt, lt, ge, le, between)
- [ ] Logical combinators implemented (and, or, not)
- [ ] `filter()` wrapper returns FilterExpression
- [ ] TypeScript strict mode passes
- [ ] Unit tests cover all functions
- [ ] Integrates with existing Filter/FilterBuilder

---

### W33.2: React Hooks (6 hours)

**Objective:** Create React hooks for modern React 18+ applications.

**Target API:**
```typescript
import { useEdgeVec, useSearch } from 'edgevec/react';

function SearchComponent() {
  // Initialize database
  const { db, isReady, error } = useEdgeVec({
    dimensions: 384,
    persistName: 'my-vectors'
  });

  // Reactive search
  const {
    results,
    isSearching,
    error: searchError
  } = useSearch(db, {
    vector: queryEmbedding,
    k: 10,
    filter: eq('category', 'docs'),
    enabled: isReady && queryEmbedding !== null
  });

  if (!isReady) return <div>Loading EdgeVec...</div>;
  if (isSearching) return <div>Searching...</div>;

  return (
    <ul>
      {results.map(r => <li key={r.id}>{r.score}</li>)}
    </ul>
  );
}
```

**Subtasks:**

| ID | Task | Hours | Verification |
|:---|:-----|:------|:-------------|
| W33.2.1 | Design hooks API, create pkg/react/ structure | 1h | Design doc |
| W33.2.2 | Implement `useEdgeVec` hook | 2h | Unit tests pass |
| W33.2.3 | Implement `useSearch` hook | 2h | Unit tests pass |
| W33.2.4 | Add TypeScript types, exports | 1h | Types compile |

**Files to Create:**
- `pkg/react/index.ts` (main exports)
- `pkg/react/useEdgeVec.ts`
- `pkg/react/useSearch.ts`
- `pkg/react/types.ts`

**Hook Specifications:**

#### useEdgeVec

```typescript
interface UseEdgeVecOptions {
  dimensions: number;
  persistName?: string;
  metric?: 'cosine' | 'euclidean' | 'dot';
  efConstruction?: number;
  m?: number;
}

interface UseEdgeVecResult {
  db: EdgeVecIndex | null;
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
  stats: { count: number; dimensions: number } | null;
}

function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecResult;
```

#### useSearch

```typescript
interface UseSearchOptions {
  vector: Float32Array | number[] | null;
  k?: number;
  filter?: FilterExpression | string;
  enabled?: boolean;
  debounceMs?: number;
}

interface UseSearchResult {
  results: SearchResult[];
  isSearching: boolean;
  error: Error | null;
  searchTime: number | null;
}

function useSearch(
  db: EdgeVecIndex | null,
  options: UseSearchOptions
): UseSearchResult;
```

**Acceptance Criteria:**
- [ ] `useEdgeVec` initializes WASM and creates index
- [ ] `useEdgeVec` handles persistence loading
- [ ] `useSearch` performs reactive search when vector changes
- [ ] `useSearch` respects `enabled` flag
- [ ] `useSearch` supports debouncing
- [ ] Proper cleanup on unmount
- [ ] Works with React 18 Strict Mode (double-mount safe)
- [ ] TypeScript types are complete
- [ ] No memory leaks

---

### W33.3: Documentation & Examples (2 hours)

**Objective:** Document new APIs and provide copy-paste examples.

**Subtasks:**

| ID | Task | Hours | Verification |
|:---|:-----|:------|:-------------|
| W33.3.1 | Add filter functions to README | 0.5h | Examples render |
| W33.3.2 | Add React hooks section to README | 1h | Examples render |
| W33.3.3 | Create example React component | 0.5h | Component works |

**Documentation Sections:**

1. **Filter Functions** — Functional composition examples
2. **React Integration** — useEdgeVec, useSearch usage
3. **Complete Example** — Full React component with search

**Acceptance Criteria:**
- [ ] README has filter functions section with 3+ examples
- [ ] README has React hooks section with complete example
- [ ] Example component demonstrates full flow
- [ ] All code examples are tested/verified

---

## Testing & Review (W33.T)

| Test Type | Target | Command |
|:----------|:-------|:--------|
| TypeScript | Strict mode | `npx tsc --noEmit` |
| Unit Tests | Filter functions | `npm test` |
| Unit Tests | React hooks | `npm test` |
| Build | Package compiles | `npm run build` |

---

## Success Metrics

| Metric | Target | Verification |
|:-------|:-------|:-------------|
| Filter functions | 10+ functions | Code inspection |
| React hooks | 2 hooks | Code inspection |
| TypeScript strict | 0 errors | `tsc --noEmit` |
| Test coverage | All new code | `npm test` |
| Documentation | 2 new sections | README review |

---

## Dependencies

| Dependency | Status | Notes |
|:-----------|:-------|:------|
| Week 32 (SIMD) | COMPLETE | No blockers |
| React peer dep | REQUIRED | React 18+ |
| v0.7.0 Release | COMPLETE | Base SDK ready |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| React 18 Strict Mode issues | MEDIUM | MEDIUM | Test double-mount early |
| WASM async initialization | LOW | MEDIUM | Proven pattern from wrapper |
| Bundle size increase | LOW | LOW | React is peer dep, not bundled |
| Type complexity | MEDIUM | LOW | Start simple, expand later |

---

## Non-Goals for Week 33

Explicitly **out of scope**:
- Vue composables (Week 34)
- Server-side rendering (SSR) support
- React Native support
- Breaking changes to existing API

---

## Daily Task Files

| Day | File | Focus |
|:----|:-----|:------|
| 1 | `DAY_1_TASKS.md` | Research & Design |
| 2 | `DAY_2_TASKS.md` | Filter Functions Implementation |
| 3 | `DAY_3_TASKS.md` | React Hook Design |
| 4 | `DAY_4_TASKS.md` | useEdgeVec Implementation |
| 5 | `DAY_5_TASKS.md` | useSearch Implementation |
| 6 | `DAY_6_TASKS.md` | Documentation |
| 7 | `DAY_7_TASKS.md` | Testing & Review |

---

## Exit Criteria

Week 33 is complete when:

- [ ] All filter functions implemented and tested
- [ ] `useEdgeVec` hook working with persistence
- [ ] `useSearch` hook working with debounce
- [ ] TypeScript compiles with strict mode
- [ ] README updated with new sections
- [ ] HOSTILE_REVIEWER approves all deliverables
- [ ] `.claude/GATE_W33_COMPLETE.md` created

---

## Approval Status

| Reviewer | Verdict | Date |
|:---------|:--------|:-----|
| HOSTILE_REVIEWER | PENDING | - |

---

**Author:** PLANNER
**Date:** 2026-01-05
**Version:** 1.0
