# Day 3: React Hook Design

**Date:** 2026-01-15
**Focus:** W33.2.1 — Design React Hooks API
**Hours:** 2h

---

## Objectives

1. Design useEdgeVec hook API
2. Design useSearch hook API
3. Set up pkg/react/ structure

---

## Tasks

### Task 3.1: Create React Directory Structure (15 min)

```
pkg/
├── react/
│   ├── index.ts          # Main exports
│   ├── types.ts          # Shared types
│   ├── useEdgeVec.ts     # Database hook
│   └── useSearch.ts      # Search hook
```

---

### Task 3.2: Design useEdgeVec Hook (45 min)

**Create:** `docs/planning/weeks/week_33/REACT_HOOKS_DESIGN.md`

```typescript
// =============================================================================
// useEdgeVec - Database Initialization Hook
// =============================================================================

interface UseEdgeVecOptions {
  /** Vector dimensions (required) */
  dimensions: number;

  /** IndexedDB store name for persistence (optional) */
  persistName?: string;

  /** Distance metric (default: 'cosine') */
  metric?: 'cosine' | 'euclidean' | 'dot';

  /** HNSW ef_construction parameter (default: 200) */
  efConstruction?: number;

  /** HNSW M parameter (default: 16) */
  m?: number;

  /** Enable binary quantization (default: false) */
  enableBQ?: boolean;
}

interface UseEdgeVecResult {
  /** The EdgeVec index instance (null until ready) */
  db: EdgeVecIndex | null;

  /** True when WASM is loaded and index is ready */
  isReady: boolean;

  /** True during WASM initialization */
  isLoading: boolean;

  /** Error if initialization failed */
  error: Error | null;

  /** Index statistics (null until ready) */
  stats: {
    count: number;
    dimensions: number;
  } | null;

  /** Force reload the index */
  reload: () => Promise<void>;
}

// Usage
const { db, isReady, error } = useEdgeVec({
  dimensions: 384,
  persistName: 'my-vectors'
});
```

**Implementation Notes:**
- Use `useEffect` for WASM initialization
- Use `useRef` to hold index reference (stable across renders)
- Use `useState` for loading/ready/error state
- Handle React 18 Strict Mode (double-mount cleanup)
- Cleanup: call db.close() on unmount if needed

---

### Task 3.3: Design useSearch Hook (45 min)

```typescript
// =============================================================================
// useSearch - Reactive Search Hook
// =============================================================================

interface UseSearchOptions {
  /** Query vector (null disables search) */
  vector: Float32Array | number[] | null;

  /** Number of results (default: 10) */
  k?: number;

  /** Filter expression or string */
  filter?: FilterExpression | string;

  /** Enable/disable search (default: true) */
  enabled?: boolean;

  /** Debounce delay in ms (default: 0) */
  debounceMs?: number;

  /** Include vector data in results (default: false) */
  includeVectors?: boolean;
}

interface UseSearchResult {
  /** Search results (empty array until search completes) */
  results: SearchResult[];

  /** True during search execution */
  isSearching: boolean;

  /** Error if search failed */
  error: Error | null;

  /** Search execution time in ms (null until search completes) */
  searchTime: number | null;

  /** Manually trigger search */
  refetch: () => Promise<void>;
}

// Usage
const { results, isSearching } = useSearch(db, {
  vector: queryEmbedding,
  k: 10,
  filter: eq('category', 'docs'),
  enabled: isReady && queryEmbedding !== null,
  debounceMs: 300
});
```

**Implementation Notes:**
- Use `useEffect` with vector/filter dependencies
- Implement debounce with `setTimeout` + cleanup
- Return stable empty array when disabled
- Track search timing with `performance.now()`
- Handle race conditions (abort stale searches)

---

### Task 3.4: Create Type Definitions (15 min)

**Create:** `pkg/react/types.ts`

Export all interfaces for external use.

---

## Verification

- [ ] Design document created
- [ ] Directory structure created
- [ ] Type definitions complete
- [ ] API covers common use cases
- [ ] Ready for Day 4 implementation

---

## Notes

_Fill during work:_

---

**Status:** PENDING
