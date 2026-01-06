/**
 * useEdgeVec - React hook for EdgeVec database initialization
 *
 * Initializes WASM module and creates/loads an EdgeVec index.
 * Handles persistence loading from IndexedDB when persistName is provided.
 *
 * @module edgevec/react
 * @version 0.8.0
 *
 * @example
 * ```tsx
 * import { useEdgeVec } from 'edgevec/react';
 *
 * function App() {
 *   const { db, isReady, isLoading, error, stats } = useEdgeVec({
 *     dimensions: 384,
 *     persistName: 'my-vectors'
 *   });
 *
 *   if (isLoading) return <div>Loading EdgeVec...</div>;
 *   if (error) return <div>Error: {error.message}</div>;
 *   if (!isReady) return null;
 *
 *   return <div>Loaded {stats?.count} vectors</div>;
 * }
 * ```
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { EdgeVecIndex, IndexConfig } from '../edgevec-wrapper.js';
import type { UseEdgeVecOptions, UseEdgeVecResult } from './types.js';

/**
 * React hook for EdgeVec database initialization.
 *
 * @param options - Configuration options
 * @returns Hook result with db instance and status
 */
export function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecResult {
  const {
    dimensions,
    persistName,
    efConstruction = 200,
    m = 16,
  } = options;

  const [isLoading, setIsLoading] = useState(true);
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [stats, setStats] = useState<{ count: number; dimensions: number } | null>(null);

  // Use ref for db to avoid re-renders and ensure stable reference
  const dbRef = useRef<EdgeVecIndex | null>(null);
  const mountedRef = useRef(true);
  const persistNameRef = useRef(persistName);

  // Update persistName ref when it changes
  persistNameRef.current = persistName;

  const initialize = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      setIsReady(false);

      let index: EdgeVecIndex;

      // Try to load from IndexedDB if persistName is provided
      if (persistName) {
        try {
          index = await EdgeVecIndex.load(persistName);
        } catch {
          // If load fails, create new index
          const config: IndexConfig = {
            dimensions,
            efConstruction,
            m,
          };
          index = new EdgeVecIndex(config);
        }
      } else {
        // Create new index
        const config: IndexConfig = {
          dimensions,
          efConstruction,
          m,
        };
        index = new EdgeVecIndex(config);
      }

      // Only update state if still mounted
      if (mountedRef.current) {
        dbRef.current = index;
        setStats({
          count: index.size,
          dimensions: index.dimensions || dimensions,
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
  }, [dimensions, persistName, efConstruction, m]);

  const reload = useCallback(async () => {
    dbRef.current = null;
    setIsReady(false);
    setStats(null);
    await initialize();
  }, [initialize]);

  const save = useCallback(async (name?: string) => {
    const db = dbRef.current;
    if (!db) {
      throw new Error('Database not initialized');
    }
    const saveName = name || persistNameRef.current;
    if (!saveName) {
      throw new Error('No persist name provided');
    }
    await db.save(saveName);
    // Update stats after save
    if (mountedRef.current) {
      setStats({
        count: db.size,
        dimensions: db.dimensions,
      });
    }
  }, []);

  // Initialize on mount
  useEffect(() => {
    mountedRef.current = true;
    initialize();

    return () => {
      mountedRef.current = false;
    };
  }, [initialize]);

  return {
    db: dbRef.current,
    isReady,
    isLoading,
    error,
    stats,
    reload,
    save,
  };
}
