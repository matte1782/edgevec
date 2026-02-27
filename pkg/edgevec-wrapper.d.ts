/**
 * EdgeVec Index Wrapper - TypeScript API
 *
 * High-level wrapper for EdgeVec WASM module with filtered search support.
 *
 * @module edgevec-wrapper
 * @version 0.9.0
 */
import { FilterExpression, MetadataValue } from './filter.js';
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
/**
 * Filter exception with rich error information.
 */
export declare class FilterException extends Error {
    readonly code: string;
    readonly position?: SourcePosition;
    readonly suggestion?: string;
    readonly filterString?: string;
    constructor(code: string, message: string, position?: SourcePosition, suggestion?: string, filterString?: string);
    /**
     * Format error with source context.
     */
    format(): string;
    /**
     * Create from JSON error response.
     */
    static fromJson(json: string, filterString?: string): FilterException;
}
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
export declare class EdgeVecIndex {
    private inner;
    private readonly dims;
    /**
     * Create a new EdgeVec index.
     *
     * @param config - Index configuration
     */
    constructor(config: IndexConfig);
    /**
     * Number of vectors in index.
     */
    get size(): number;
    /**
     * Vector dimensions.
     */
    get dimensions(): number;
    /**
     * Add a vector with optional metadata.
     *
     * @param vector - Vector data
     * @param metadata - Optional metadata
     * @returns Vector ID
     */
    add(vector: Float32Array | number[], metadata?: Metadata): number;
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
    search(query: Float32Array | number[], k: number, options?: SearchOptions): Promise<SearchResult[]>;
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
    searchFiltered(query: Float32Array | number[], k: number, options?: SearchOptions): Promise<FilteredSearchResult>;
    /**
     * Count vectors matching a filter.
     *
     * @param filter - Filter expression
     * @returns Count of matching vectors
     */
    count(filter?: string | FilterExpression): Promise<number>;
    /**
     * Get metadata for a vector.
     *
     * @param id - Vector ID
     * @returns Metadata or undefined
     */
    getMetadata(id: number): Metadata | undefined;
    /**
     * Set metadata for a vector.
     *
     * @param id - Vector ID
     * @param key - Metadata key
     * @param value - Metadata value
     */
    setMetadata(id: number, key: string, value: MetadataValue): void;
    /**
     * Delete a vector (soft delete).
     *
     * @param id - Vector ID
     * @returns True if deleted, false if not found or already deleted
     */
    delete(id: number): boolean;
    /**
     * Save index to IndexedDB.
     *
     * @param name - Database name
     */
    save(name: string): Promise<void>;
    /**
     * Load index from IndexedDB.
     *
     * @param name - Database name
     * @returns Loaded index
     */
    static load(name: string): Promise<EdgeVecIndex>;
    /**
     * Initialize sparse storage for hybrid search.
     * Must be called before using sparse or hybrid search.
     */
    initSparseStorage(): void;
    /**
     * Check if sparse storage is initialized.
     */
    hasSparseStorage(): boolean;
    /**
     * Get the number of sparse vectors stored.
     */
    sparseCount(): number;
    /**
     * Insert a sparse vector (e.g., BM25 scores).
     *
     * @param indices - Sorted indices of non-zero elements
     * @param values - Values corresponding to indices
     * @param dim - Dimension of sparse space (vocabulary size)
     * @returns The assigned sparse vector ID
     */
    insertSparse(indices: Uint32Array, values: Float32Array, dim: number): number;
    /**
     * Search sparse vectors by query.
     *
     * @param indices - Query sparse indices (sorted)
     * @param values - Query sparse values
     * @param dim - Dimension of sparse space
     * @param k - Number of results
     * @returns Parsed sparse search results
     */
    searchSparse(indices: Uint32Array, values: Float32Array, dim: number, k: number): SparseSearchResult[];
    /**
     * Perform hybrid search combining dense and sparse results.
     *
     * @param denseQuery - Dense embedding vector
     * @param sparseIndices - Sparse query indices (sorted)
     * @param sparseValues - Sparse query values
     * @param sparseDim - Dimension of sparse space
     * @param options - Hybrid search options
     * @returns Hybrid search results with fusion scores
     */
    hybridSearch(denseQuery: Float32Array, sparseIndices: Uint32Array, sparseValues: Float32Array, sparseDim: number, options: HybridSearchOptions): HybridSearchResult[];
    /**
     * Insert a binary vector into the index.
     *
     * @param vector - Binary vector as packed bytes
     * @returns Vector ID
     */
    insertBinary(vector: Uint8Array): number;
    /**
     * Search with a binary query vector.
     *
     * @param query - Binary query vector as packed bytes
     * @param k - Number of results
     * @returns Search results sorted by Hamming distance
     */
    searchBinary(query: Uint8Array, k: number): SearchResult[];
    /**
     * Search binary with custom ef parameter.
     *
     * @param query - Binary query vector
     * @param k - Number of results
     * @param efSearch - ef_search parameter for accuracy/speed tradeoff
     * @returns Search results
     */
    searchBinaryWithEf(query: Uint8Array, k: number, efSearch: number): SearchResult[];
    /**
     * Insert a batch of flat vectors.
     *
     * @param vectors - Flat array of vectors (concatenated Float32)
     * @param count - Number of vectors
     * @returns Array of assigned vector IDs
     */
    insertBatchFlat(vectors: Float32Array, count: number): Uint32Array;
    private buildOptionsJson;
    private wrapError;
    private toJsMetadataValue;
}
export { Filter, FilterExpression, MetadataValue } from './filter.js';
export { FilterBuilder, FieldCondition } from './filter-builder.js';
export type { SparseSearchResult, HybridSearchResult, HybridSearchOptions } from './edgevec-types.js';
export default EdgeVecIndex;
//# sourceMappingURL=edgevec-wrapper.d.ts.map