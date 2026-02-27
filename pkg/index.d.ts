/**
 * EdgeVec - High-Performance Embedded Vector Database
 *
 * A WebAssembly-powered vector database for Browser, Node, and Edge environments.
 *
 * @module edgevec
 * @version 0.9.0
 *
 * @example
 * ```typescript
 * // High-level API (recommended)
 * import { EdgeVecIndex, Filter, FilterBuilder } from 'edgevec';
 *
 * const index = new EdgeVecIndex({ dimensions: 384 });
 * index.add(embedding, { category: 'gpu', price: 499 });
 * const results = await index.search(query, 10, {
 *   filter: 'category = "gpu" AND price < 500'
 * });
 *
 * // Low-level API (direct WASM access)
 * import init, { EdgeVec, EdgeVecConfig, Filter } from 'edgevec';
 *
 * await init();
 * const config = new EdgeVecConfig(128);
 * const db = new EdgeVec(config);
 * const id = db.insert(new Float32Array(128).fill(0.1));
 * const results = db.search(new Float32Array(128).fill(0.1), 10);
 * ```
 */
export { default, EdgeVec, EdgeVecConfig, JsMetadataValue, BatchInsertConfig, BatchInsertResult, } from './edgevec.js';
export { Filter, FilterExpression, FilterValidation, FilterValidationError, FilterValidationWarning, MetadataValue, } from './filter.js';
export { FilterBuilder, FieldCondition } from './filter-builder.js';
export { EdgeVecIndex, FilterException, FilterStrategy, Metadata, SearchOptions, SearchResult, FilteredSearchResult, IndexConfig, SourcePosition, } from './edgevec-wrapper.js';
export type { SparseSearchResult, HybridSearchResult, HybridSearchOptions } from './edgevec-wrapper.js';
export { createSparseVector, parseHybridResults, parseSparseResults, createHybridOptions } from './sparse-helpers.js';
//# sourceMappingURL=index.d.ts.map