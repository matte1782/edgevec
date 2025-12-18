/**
 * EdgeVec Index Wrapper - TypeScript API
 *
 * High-level wrapper for EdgeVec WASM module with filtered search support.
 *
 * @module edgevec-wrapper
 * @version 0.5.0
 */

import { EdgeVec, EdgeVecConfig, JsMetadataValue } from './edgevec.js';
import { Filter, FilterExpression, MetadataValue } from './filter.js';

// =============================================================================
// Types
// =============================================================================

/** Filter strategy for search */
export type FilterStrategy = 'auto' | 'pre' | 'post' | 'hybrid';

/** Metadata record for a vector */
export type Metadata = Record<string, MetadataValue>;

/**
 * Options for filtered search.
 */
export interface SearchOptions {
  /**
   * Filter expression (string or Filter object).
   */
  filter?: string | FilterExpression;

  /**
   * Filter strategy.
   * @default 'auto'
   */
  strategy?: FilterStrategy;

  /**
   * Oversample factor for post/hybrid strategy.
   * @default 3.0
   */
  oversampleFactor?: number;

  /**
   * Include metadata in results.
   * @default false
   */
  includeMetadata?: boolean;

  /**
   * Include vectors in results.
   * @default false
   */
  includeVectors?: boolean;

  /**
   * Override ef_search for this query.
   */
  efSearch?: number;
}

/**
 * Single search result.
 */
export interface SearchResult {
  /** Vector ID */
  id: number;
  /** Similarity score (lower is more similar for distance metrics) */
  score: number;
  /** Metadata (if includeMetadata=true) */
  metadata?: Metadata;
  /** Vector data (if includeVectors=true) */
  vector?: Float32Array;
}

/**
 * Filtered search result with diagnostics.
 */
export interface FilteredSearchResult {
  /** Search results */
  results: SearchResult[];
  /** Whether k results were found */
  complete: boolean;
  /** Observed selectivity (0.0 - 1.0, lower means more selective) */
  observedSelectivity: number;
  /** Strategy used */
  strategyUsed: FilterStrategy;
  /** Number of vectors evaluated */
  vectorsEvaluated: number;
  /** Filter evaluation time (ms) */
  filterTimeMs: number;
  /** Total search time (ms) */
  totalTimeMs: number;
}

/**
 * Index configuration.
 */
export interface IndexConfig {
  /** Vector dimensions */
  dimensions: number;
  /** HNSW M parameter (connections per node) */
  m?: number;
  /** HNSW ef_construction parameter */
  efConstruction?: number;
  /** Enable quantization for smaller memory footprint */
  quantized?: boolean;
}

/**
 * Source position in filter string.
 */
export interface SourcePosition {
  line: number;
  column: number;
  offset: number;
}

// =============================================================================
// FilterException
// =============================================================================

/**
 * Filter exception with rich error information.
 */
export class FilterException extends Error {
  readonly code: string;
  readonly position?: SourcePosition;
  readonly suggestion?: string;
  readonly filterString?: string;

  constructor(
    code: string,
    message: string,
    position?: SourcePosition,
    suggestion?: string,
    filterString?: string
  ) {
    super(message);
    this.name = 'FilterException';
    this.code = code;
    this.position = position;
    this.suggestion = suggestion;
    this.filterString = filterString;
  }

  /**
   * Format error with source context.
   */
  format(): string {
    let output = `FilterException [${this.code}]: ${this.message}`;

    if (this.position) {
      output += `\n  at line ${this.position.line}, column ${this.position.column}`;
    }

    if (this.filterString && this.position) {
      const lines = this.filterString.split('\n');
      const line = lines[this.position.line - 1];
      if (line) {
        output += `\n  ${line}`;
        output += `\n  ${' '.repeat(this.position.column - 1)}^`;
      }
    }

    if (this.suggestion) {
      output += `\n  Suggestion: ${this.suggestion}`;
    }

    return output;
  }

  /**
   * Create from JSON error response.
   */
  static fromJson(json: string, filterString?: string): FilterException {
    try {
      const parsed = JSON.parse(json);
      return new FilterException(
        parsed.code || 'E000',
        parsed.message || 'Unknown error',
        parsed.position,
        parsed.suggestion,
        filterString
      );
    } catch {
      return new FilterException('E000', json, undefined, undefined, filterString);
    }
  }
}

// =============================================================================
// EdgeVecIndex Class
// =============================================================================

/**
 * EdgeVec vector index with filtering support.
 *
 * @example
 * const index = new EdgeVecIndex({ dimensions: 384 });
 * await index.add(vector, { category: 'gpu', price: 499 });
 * const results = await index.search(query, 10, {
 *   filter: 'category = "gpu" AND price < 500'
 * });
 */
export class EdgeVecIndex {
  private inner: EdgeVec;
  private readonly dims: number;

  /**
   * Create a new EdgeVec index.
   *
   * @param config - Index configuration
   */
  constructor(config: IndexConfig) {
    this.dims = config.dimensions;
    const wasmConfig = new EdgeVecConfig(config.dimensions);
    if (config.efConstruction !== undefined) {
      wasmConfig.ef_construction = config.efConstruction;
    }
    this.inner = new EdgeVec(wasmConfig);
  }

  /**
   * Number of vectors in index.
   */
  get size(): number {
    return this.inner.liveCount();
  }

  /**
   * Vector dimensions.
   */
  get dimensions(): number {
    return this.dims;
  }

  /**
   * Add a vector with optional metadata.
   *
   * @param vector - Vector data
   * @param metadata - Optional metadata
   * @returns Vector ID
   */
  add(vector: Float32Array | number[], metadata?: Metadata): number {
    const vec =
      vector instanceof Float32Array ? vector : new Float32Array(vector);
    const id = this.inner.insert(vec);

    if (metadata) {
      for (const [key, value] of Object.entries(metadata)) {
        this.inner.setMetadata(id, key, this.toJsMetadataValue(value));
      }
    }

    return id;
  }

  /**
   * Search for similar vectors.
   *
   * @param query - Query vector
   * @param k - Number of results
   * @param options - Search options (filter, strategy, etc.)
   * @returns Search results
   *
   * @example
   * const results = await index.search(query, 10, {
   *   filter: 'category = "gpu" AND price < 500'
   * });
   */
  async search(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<SearchResult[]> {
    const result = await this.searchFiltered(query, k, options);
    return result.results;
  }

  /**
   * Search with filter and full diagnostics.
   *
   * @param query - Query vector
   * @param k - Number of results
   * @param options - Search options
   * @returns Filtered search result with diagnostics
   *
   * @example
   * const result = await index.searchFiltered(query, 10, {
   *   filter: Filter.eq('category', 'gpu'),
   *   strategy: 'auto'
   * });
   * console.log('Strategy used:', result.strategyUsed);
   * console.log('Selectivity:', result.observedSelectivity);
   */
  async searchFiltered(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<FilteredSearchResult> {
    const queryVec =
      query instanceof Float32Array ? query : new Float32Array(query);

    // Build options JSON
    const optionsJson = this.buildOptionsJson(options);

    try {
      // Call WASM searchFiltered
      const resultJson = this.inner.searchFiltered(queryVec, k, optionsJson);

      // Parse result
      const result = JSON.parse(resultJson);

      // Convert results
      return {
        results: result.results.map(
          (r: { id: number; score: number; metadata?: Metadata; vector?: number[] }) => ({
            id: r.id,
            score: r.score,
            metadata: r.metadata ?? undefined,
            vector: r.vector ? new Float32Array(r.vector) : undefined,
          })
        ),
        complete: result.complete,
        observedSelectivity: result.observedSelectivity,
        strategyUsed: result.strategyUsed as FilterStrategy,
        vectorsEvaluated: result.vectorsEvaluated,
        filterTimeMs: result.filterTimeMs,
        totalTimeMs: result.totalTimeMs,
      };
    } catch (e) {
      throw this.wrapError(e, options?.filter);
    }
  }

  /**
   * Count vectors matching a filter.
   *
   * @param filter - Filter expression
   * @returns Count of matching vectors
   */
  async count(filter?: string | FilterExpression): Promise<number> {
    if (!filter) {
      return this.size;
    }

    // Use pre-filter strategy to count all matches
    const result = await this.searchFiltered(
      new Float32Array(this.dimensions).fill(0),
      this.size,
      { filter, strategy: 'pre' }
    );

    return result.results.length;
  }

  /**
   * Get metadata for a vector.
   *
   * @param id - Vector ID
   * @returns Metadata or undefined
   */
  getMetadata(id: number): Metadata | undefined {
    const obj = this.inner.getAllMetadata(id);
    return obj as Metadata | undefined;
  }

  /**
   * Set metadata for a vector.
   *
   * @param id - Vector ID
   * @param key - Metadata key
   * @param value - Metadata value
   */
  setMetadata(id: number, key: string, value: MetadataValue): void {
    this.inner.setMetadata(id, key, this.toJsMetadataValue(value));
  }

  /**
   * Delete a vector (soft delete).
   *
   * @param id - Vector ID
   * @returns True if deleted, false if not found or already deleted
   */
  delete(id: number): boolean {
    return this.inner.softDelete(id);
  }

  /**
   * Save index to IndexedDB.
   *
   * @param name - Database name
   */
  async save(name: string): Promise<void> {
    await this.inner.save(name);
  }

  /**
   * Load index from IndexedDB.
   *
   * @param name - Database name
   * @returns Loaded index
   */
  static async load(name: string): Promise<EdgeVecIndex> {
    const inner = await EdgeVec.load(name);
    const index = Object.create(EdgeVecIndex.prototype) as EdgeVecIndex;
    (index as unknown as { inner: EdgeVec }).inner = inner;
    // Get dimensions from config
    (index as unknown as { dims: number }).dims = 0; // Will be inferred from first vector
    return index;
  }

  // ===========================================================================
  // Private Helpers
  // ===========================================================================

  private buildOptionsJson(options?: SearchOptions): string {
    if (!options) {
      return '{}';
    }

    const opts: Record<string, unknown> = {};

    // Handle filter (string or FilterExpression)
    if (options.filter) {
      if (typeof options.filter === 'string') {
        opts.filter = options.filter;
      } else {
        opts.filter = options.filter.toString();
      }
    }

    if (options.strategy) {
      opts.strategy = options.strategy;
    }

    if (options.oversampleFactor !== undefined) {
      opts.oversampleFactor = options.oversampleFactor;
    }

    if (options.includeMetadata !== undefined) {
      opts.includeMetadata = options.includeMetadata;
    }

    if (options.includeVectors !== undefined) {
      opts.includeVectors = options.includeVectors;
    }

    if (options.efSearch !== undefined) {
      opts.efSearch = options.efSearch;
    }

    return JSON.stringify(opts);
  }

  private wrapError(e: unknown, filter?: string | FilterExpression): Error {
    const filterString =
      typeof filter === 'string' ? filter : filter?.toString();

    if (typeof e === 'string') {
      return FilterException.fromJson(e, filterString);
    }

    if (e instanceof Error) {
      // Check if it's a WASM error with JSON message
      try {
        return FilterException.fromJson(e.message, filterString);
      } catch {
        return e;
      }
    }

    return new Error(String(e));
  }

  private toJsMetadataValue(value: MetadataValue): JsMetadataValue {
    if (typeof value === 'string') {
      return JsMetadataValue.fromString(value);
    }
    if (typeof value === 'number') {
      if (Number.isInteger(value)) {
        return JsMetadataValue.fromInteger(value);
      }
      return JsMetadataValue.fromFloat(value);
    }
    if (typeof value === 'boolean') {
      return JsMetadataValue.fromBoolean(value);
    }
    if (Array.isArray(value)) {
      return JsMetadataValue.fromStringArray(value);
    }
    throw new Error(`Unsupported metadata value type: ${typeof value}`);
  }
}

// Re-export commonly used types and classes
export { Filter, FilterExpression, MetadataValue } from './filter.js';
export { FilterBuilder, FieldCondition } from './filter-builder.js';

export default EdgeVecIndex;
