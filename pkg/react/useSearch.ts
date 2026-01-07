/**
 * useSearch - React hook for reactive vector search
 *
 * Performs searches that automatically update when the query vector or filter changes.
 * Supports debouncing, filtering, and handles race conditions.
 *
 * @module edgevec/react
 * @version 0.8.0
 *
 * @example
 * ```tsx
 * import { useEdgeVec, useSearch } from 'edgevec/react';
 * import { eq } from 'edgevec';
 *
 * function SearchComponent() {
 *   const { db, isReady } = useEdgeVec({ dimensions: 384 });
 *   const [queryVector, setQueryVector] = useState<number[] | null>(null);
 *
 *   const { results, isSearching, searchTime } = useSearch(db, {
 *     vector: queryVector,
 *     k: 10,
 *     filter: eq('category', 'docs'),
 *     enabled: isReady && queryVector !== null,
 *     debounceMs: 300,
 *   });
 *
 *   return (
 *     <ul>
 *       {results.map(r => (
 *         <li key={r.id}>Score: {r.score.toFixed(4)}</li>
 *       ))}
 *     </ul>
 *   );
 * }
 * ```
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import type { EdgeVecIndex, SearchResult } from '../edgevec-wrapper.js';
import type { UseSearchOptions, UseSearchResult } from './types.js';

// Stable empty array reference to prevent unnecessary re-renders
const EMPTY_RESULTS: SearchResult[] = [];

/**
 * React hook for reactive vector search.
 *
 * @param db - EdgeVec index instance (from useEdgeVec)
 * @param options - Search options
 * @returns Hook result with search results and status
 */
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
    includeVectors = false,
    includeMetadata = false,
  } = options;

  const [results, setResults] = useState<SearchResult[]>(EMPTY_RESULTS);
  const [isSearching, setIsSearching] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [searchTime, setSearchTime] = useState<number | null>(null);

  // Track the current search to handle race conditions
  const searchIdRef = useRef(0);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  // Store filter as string for dependency tracking
  const filterString = filter
    ? typeof filter === 'string'
      ? filter
      : filter.toString()
    : undefined;

  const executeSearch = useCallback(async () => {
    if (!db || !vector || !enabled) {
      setResults(EMPTY_RESULTS);
      setIsSearching(false);
      setSearchTime(null);
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
        includeVectors,
        includeMetadata,
      });

      const endTime = performance.now();

      // Only update if this is still the latest search and component is mounted
      if (currentSearchId === searchIdRef.current && mountedRef.current) {
        setResults(searchResults);
        setSearchTime(endTime - startTime);
        setIsSearching(false);
      }
    } catch (err) {
      if (currentSearchId === searchIdRef.current && mountedRef.current) {
        setError(err instanceof Error ? err : new Error(String(err)));
        setResults(EMPTY_RESULTS);
        setIsSearching(false);
      }
    }
  }, [db, vector, k, filterString, enabled, includeVectors, includeMetadata]);

  const refetch = useCallback(async () => {
    await executeSearch();
  }, [executeSearch]);

  // Effect to trigger search with debouncing
  useEffect(() => {
    mountedRef.current = true;

    // Clear any existing debounce timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = null;
    }

    // Don't search if disabled or no db/vector
    if (!enabled || !db || !vector) {
      setResults(EMPTY_RESULTS);
      setIsSearching(false);
      setSearchTime(null);
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
      mountedRef.current = false;
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
        debounceTimerRef.current = null;
      }
    };
  }, [db, vector, k, filterString, enabled, debounceMs, executeSearch]);

  return {
    results,
    isSearching,
    error,
    searchTime,
    refetch,
  };
}
