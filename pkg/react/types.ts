/**
 * EdgeVec React Hooks - Type Definitions
 *
 * @module edgevec/react
 * @version 0.8.0
 */

import type { EdgeVecIndex, SearchResult } from '../edgevec-wrapper.js';
import type { FilterExpression } from '../filter.js';

// =============================================================================
// useEdgeVec Types
// =============================================================================

/**
 * Options for useEdgeVec hook.
 */
export interface UseEdgeVecOptions {
  /** Vector dimensions (required) */
  dimensions: number;

  /** IndexedDB store name for persistence (optional) */
  persistName?: string;

  /** HNSW ef_construction parameter (default: 200) */
  efConstruction?: number;

  /** HNSW M parameter (default: 16) */
  m?: number;
}

/**
 * Result of useEdgeVec hook.
 */
export interface UseEdgeVecResult {
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

  /** Save index to IndexedDB */
  save: (name?: string) => Promise<void>;
}

// =============================================================================
// useSearch Types
// =============================================================================

/**
 * Options for useSearch hook.
 */
export interface UseSearchOptions {
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

  /** Include metadata in results (default: false) */
  includeMetadata?: boolean;
}

/**
 * Result of useSearch hook.
 */
export interface UseSearchResult {
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
