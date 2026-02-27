/**
 * EdgeVec Index Wrapper - TypeScript API
 *
 * High-level wrapper for EdgeVec WASM module with filtered search support.
 *
 * @module edgevec-wrapper
 * @version 0.9.0
 */
import { EdgeVec, EdgeVecConfig, JsMetadataValue } from './edgevec.js';
// =============================================================================
// FilterException
// =============================================================================
/**
 * Filter exception with rich error information.
 */
export class FilterException extends Error {
    constructor(code, message, position, suggestion, filterString) {
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
    format() {
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
    static fromJson(json, filterString) {
        try {
            const parsed = JSON.parse(json);
            return new FilterException(parsed.code || 'E000', parsed.message || 'Unknown error', parsed.position, parsed.suggestion, filterString);
        }
        catch {
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
    /**
     * Create a new EdgeVec index.
     *
     * @param config - Index configuration
     */
    constructor(config) {
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
    get size() {
        return this.inner.liveCount();
    }
    /**
     * Vector dimensions.
     */
    get dimensions() {
        return this.dims;
    }
    /**
     * Add a vector with optional metadata.
     *
     * @param vector - Vector data
     * @param metadata - Optional metadata
     * @returns Vector ID
     */
    add(vector, metadata) {
        const vec = vector instanceof Float32Array ? vector : new Float32Array(vector);
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
    async search(query, k, options) {
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
    async searchFiltered(query, k, options) {
        const queryVec = query instanceof Float32Array ? query : new Float32Array(query);
        // Build options JSON
        const optionsJson = this.buildOptionsJson(options);
        try {
            // Call WASM searchFiltered
            const resultJson = this.inner.searchFiltered(queryVec, k, optionsJson);
            // Parse result
            const result = JSON.parse(resultJson);
            // Convert results
            return {
                results: result.results.map((r) => ({
                    id: r.id,
                    score: r.score,
                    metadata: r.metadata ?? undefined,
                    vector: r.vector ? new Float32Array(r.vector) : undefined,
                })),
                complete: result.complete,
                observedSelectivity: result.observedSelectivity,
                strategyUsed: result.strategyUsed,
                vectorsEvaluated: result.vectorsEvaluated,
                filterTimeMs: result.filterTimeMs,
                totalTimeMs: result.totalTimeMs,
            };
        }
        catch (e) {
            throw this.wrapError(e, options?.filter);
        }
    }
    /**
     * Count vectors matching a filter.
     *
     * @param filter - Filter expression
     * @returns Count of matching vectors
     */
    async count(filter) {
        if (!filter) {
            return this.size;
        }
        // Use pre-filter strategy to count all matches
        const result = await this.searchFiltered(new Float32Array(this.dimensions).fill(0), this.size, { filter, strategy: 'pre' });
        return result.results.length;
    }
    /**
     * Get metadata for a vector.
     *
     * @param id - Vector ID
     * @returns Metadata or undefined
     */
    getMetadata(id) {
        const obj = this.inner.getAllMetadata(id);
        return obj;
    }
    /**
     * Set metadata for a vector.
     *
     * @param id - Vector ID
     * @param key - Metadata key
     * @param value - Metadata value
     */
    setMetadata(id, key, value) {
        this.inner.setMetadata(id, key, this.toJsMetadataValue(value));
    }
    /**
     * Delete a vector (soft delete).
     *
     * @param id - Vector ID
     * @returns True if deleted, false if not found or already deleted
     */
    delete(id) {
        return this.inner.softDelete(id);
    }
    /**
     * Save index to IndexedDB.
     *
     * @param name - Database name
     */
    async save(name) {
        await this.inner.save(name);
    }
    /**
     * Load index from IndexedDB.
     *
     * @param name - Database name
     * @returns Loaded index
     */
    static async load(name) {
        const inner = await EdgeVec.load(name);
        const index = Object.create(EdgeVecIndex.prototype);
        index.inner = inner;
        // Get dimensions from config
        index.dims = 0; // Will be inferred from first vector
        return index;
    }
    // ===========================================================================
    // Private Helpers
    // ===========================================================================
    buildOptionsJson(options) {
        if (!options) {
            return '{}';
        }
        const opts = {};
        // Handle filter (string or FilterExpression)
        if (options.filter) {
            if (typeof options.filter === 'string') {
                opts.filter = options.filter;
            }
            else {
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
    wrapError(e, filter) {
        const filterString = typeof filter === 'string' ? filter : filter?.toString();
        if (typeof e === 'string') {
            return FilterException.fromJson(e, filterString);
        }
        if (e instanceof Error) {
            // Check if it's a WASM error with JSON message
            try {
                return FilterException.fromJson(e.message, filterString);
            }
            catch {
                return e;
            }
        }
        return new Error(String(e));
    }
    toJsMetadataValue(value) {
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
export { Filter } from './filter.js';
export { FilterBuilder, FieldCondition } from './filter-builder.js';
export default EdgeVecIndex;
//# sourceMappingURL=edgevec-wrapper.js.map