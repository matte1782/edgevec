# Day 4: useEdgeVec Implementation

**Date:** 2026-01-16
**Focus:** W33.2.2 — Implement useEdgeVec Hook
**Hours:** 2h

---

## Objectives

1. Implement useEdgeVec hook
2. Handle WASM initialization
3. Handle persistence loading
4. Test React 18 Strict Mode compatibility

---

## Tasks

### Task 4.1: Implement useEdgeVec (1.5h)

**Create:** `pkg/react/useEdgeVec.ts`

```typescript
/**
 * useEdgeVec - React hook for EdgeVec database initialization
 *
 * @example
 * const { db, isReady, error } = useEdgeVec({
 *   dimensions: 384,
 *   persistName: 'my-vectors'
 * });
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { EdgeVecIndex, IndexConfig } from '../edgevec-wrapper.js';
import type { UseEdgeVecOptions, UseEdgeVecResult } from './types.js';

export function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecResult {
  const {
    dimensions,
    persistName,
    metric = 'cosine',
    efConstruction = 200,
    m = 16,
    enableBQ = false
  } = options;

  const [isLoading, setIsLoading] = useState(true);
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [stats, setStats] = useState<{ count: number; dimensions: number } | null>(null);

  const dbRef = useRef<EdgeVecIndex | null>(null);
  const mountedRef = useRef(true);

  const initialize = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      const config: IndexConfig = {
        dimensions,
        metric,
        efConstruction,
        m
      };

      const index = new EdgeVecIndex(config);

      // Load from persistence if specified
      if (persistName) {
        await index.load(persistName);
      }

      // Enable BQ if requested
      if (enableBQ) {
        index.enableBQ();
      }

      // Only update state if still mounted
      if (mountedRef.current) {
        dbRef.current = index;
        setStats({
          count: index.count(),
          dimensions: index.dimensions
        });
        setIsReady(true);
        setIsLoading(false);
      }
    } catch (err) {
      if (mountedRef.current) {
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      }
    }
  }, [dimensions, persistName, metric, efConstruction, m, enableBQ]);

  const reload = useCallback(async () => {
    dbRef.current = null;
    setIsReady(false);
    await initialize();
  }, [initialize]);

  useEffect(() => {
    mountedRef.current = true;
    initialize();

    return () => {
      mountedRef.current = false;
      // Cleanup: close index if needed
      // Note: EdgeVecIndex may not need explicit close
    };
  }, [initialize]);

  return {
    db: dbRef.current,
    isReady,
    isLoading,
    error,
    stats,
    reload
  };
}
```

---

### Task 4.2: Create Types File (15 min)

**Create:** `pkg/react/types.ts`

```typescript
import type { EdgeVecIndex } from '../edgevec-wrapper.js';
import type { FilterExpression } from '../filter.js';
import type { SearchResult } from '../edgevec-wrapper.js';

export interface UseEdgeVecOptions {
  dimensions: number;
  persistName?: string;
  metric?: 'cosine' | 'euclidean' | 'dot';
  efConstruction?: number;
  m?: number;
  enableBQ?: boolean;
}

export interface UseEdgeVecResult {
  db: EdgeVecIndex | null;
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
  stats: { count: number; dimensions: number } | null;
  reload: () => Promise<void>;
}

export interface UseSearchOptions {
  vector: Float32Array | number[] | null;
  k?: number;
  filter?: FilterExpression | string;
  enabled?: boolean;
  debounceMs?: number;
  includeVectors?: boolean;
}

export interface UseSearchResult {
  results: SearchResult[];
  isSearching: boolean;
  error: Error | null;
  searchTime: number | null;
  refetch: () => Promise<void>;
}
```

---

### Task 4.3: Create Index Export (15 min)

**Create:** `pkg/react/index.ts`

```typescript
/**
 * EdgeVec React Hooks
 *
 * @module edgevec/react
 * @version 0.8.0
 */

export { useEdgeVec } from './useEdgeVec.js';
export { useSearch } from './useSearch.js';
export type {
  UseEdgeVecOptions,
  UseEdgeVecResult,
  UseSearchOptions,
  UseSearchResult
} from './types.js';
```

---

## Verification

- [ ] `pkg/react/useEdgeVec.ts` created
- [ ] `pkg/react/types.ts` created
- [ ] `pkg/react/index.ts` created
- [ ] TypeScript compiles
- [ ] Hook handles mount/unmount correctly
- [ ] React 18 Strict Mode safe (no double initialization issues)

---

## Notes

_Fill during work:_

---

**Status:** PENDING
