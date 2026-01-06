# Day 5: useSearch Implementation

**Date:** 2026-01-17
**Focus:** W33.2.3 — Implement useSearch Hook
**Hours:** 2h

---

## Objectives

1. Implement useSearch hook
2. Implement debouncing
3. Handle race conditions
4. Test with useEdgeVec

---

## Tasks

### Task 5.1: Implement useSearch (1.5h)

**Create:** `pkg/react/useSearch.ts`

```typescript
/**
 * useSearch - React hook for reactive vector search
 *
 * @example
 * const { results, isSearching } = useSearch(db, {
 *   vector: queryEmbedding,
 *   k: 10,
 *   filter: eq('category', 'docs'),
 *   debounceMs: 300
 * });
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import type { EdgeVecIndex, SearchResult } from '../edgevec-wrapper.js';
import type { FilterExpression } from '../filter.js';
import type { UseSearchOptions, UseSearchResult } from './types.js';

const EMPTY_RESULTS: SearchResult[] = [];

export function useSearch(
  db: EdgeVecIndex | null,
  options: UseSearchOptions
): UseSearchResult {
  const {
    vector,
    k = 10,
    filter,
    enabled = true,
    debounceMs = 0,
    includeVectors = false
  } = options;

  const [results, setResults] = useState<SearchResult[]>(EMPTY_RESULTS);
  const [isSearching, setIsSearching] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [searchTime, setSearchTime] = useState<number | null>(null);

  // Track the current search to handle race conditions
  const searchIdRef = useRef(0);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const executeSearch = useCallback(async () => {
    if (!db || !vector || !enabled) {
      setResults(EMPTY_RESULTS);
      setIsSearching(false);
      return;
    }

    const currentSearchId = ++searchIdRef.current;
    setIsSearching(true);
    setError(null);

    try {
      const startTime = performance.now();

      // Convert number[] to Float32Array if needed
      const queryVector = vector instanceof Float32Array
        ? vector
        : new Float32Array(vector);

      // Execute search
      const searchResults = await db.search(queryVector, k, {
        filter: filter,
        includeVectors
      });

      const endTime = performance.now();

      // Only update if this is still the latest search
      if (currentSearchId === searchIdRef.current) {
        setResults(searchResults);
        setSearchTime(endTime - startTime);
        setIsSearching(false);
      }
    } catch (err) {
      if (currentSearchId === searchIdRef.current) {
        setError(err instanceof Error ? err : new Error(String(err)));
        setResults(EMPTY_RESULTS);
        setIsSearching(false);
      }
    }
  }, [db, vector, k, filter, enabled, includeVectors]);

  const refetch = useCallback(async () => {
    await executeSearch();
  }, [executeSearch]);

  // Effect to trigger search with debouncing
  useEffect(() => {
    // Clear any existing debounce timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = null;
    }

    // Don't search if disabled or no db/vector
    if (!enabled || !db || !vector) {
      setResults(EMPTY_RESULTS);
      setIsSearching(false);
      return;
    }

    // Apply debounce if specified
    if (debounceMs > 0) {
      setIsSearching(true); // Show searching state during debounce
      debounceTimerRef.current = setTimeout(() => {
        executeSearch();
      }, debounceMs);
    } else {
      executeSearch();
    }

    // Cleanup
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
        debounceTimerRef.current = null;
      }
    };
  }, [db, vector, k, filter, enabled, debounceMs, executeSearch]);

  return {
    results,
    isSearching,
    error,
    searchTime,
    refetch
  };
}
```

---

### Task 5.2: Add React Peer Dependency (15 min)

**Update:** `pkg/package.json`

```json
{
  "peerDependencies": {
    "react": ">=18.0.0"
  },
  "peerDependenciesMeta": {
    "react": {
      "optional": true
    }
  }
}
```

---

### Task 5.3: Integration Test (15 min)

**Manual Test:** Create a simple test component to verify hooks work together.

```tsx
// Test component (not committed, just for verification)
function TestSearch() {
  const { db, isReady } = useEdgeVec({ dimensions: 4 });
  const [query, setQuery] = useState<number[] | null>(null);

  const { results, isSearching } = useSearch(db, {
    vector: query,
    k: 5,
    enabled: isReady,
    debounceMs: 200
  });

  // Test: add vectors, set query, verify results appear
}
```

---

## Verification

- [ ] `pkg/react/useSearch.ts` created
- [ ] Debouncing works correctly
- [ ] Race conditions handled (stale searches ignored)
- [ ] TypeScript compiles
- [ ] Hooks work together in test component
- [ ] Empty results returned when disabled

---

## Notes

_Fill during work:_

---

**Status:** PENDING
