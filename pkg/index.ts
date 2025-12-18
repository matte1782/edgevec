/**
 * EdgeVec - High-Performance Embedded Vector Database
 *
 * A WebAssembly-powered vector database for Browser, Node, and Edge environments.
 *
 * @module edgevec
 * @version 0.5.0
 *
 * @example
 * ```typescript
 * import { EdgeVecIndex, Filter, FilterBuilder } from 'edgevec';
 *
 * // Create index
 * const index = new EdgeVecIndex({ dimensions: 384 });
 *
 * // Add vectors with metadata
 * index.add(embedding, { category: 'gpu', price: 499 });
 *
 * // Search with filter (string syntax)
 * const results = await index.search(query, 10, {
 *   filter: 'category = "gpu" AND price < 500'
 * });
 *
 * // Search with filter (builder syntax)
 * const filter = new FilterBuilder()
 *   .where('category').eq('gpu')
 *   .and('price').lt(500)
 *   .build();
 * const results2 = await index.search(query, 10, { filter });
 * ```
 */

// Re-export everything from modules
export {
  Filter,
  FilterExpression,
  FilterValidation,
  FilterValidationError,
  FilterValidationWarning,
  MetadataValue,
} from './filter.js';

export { FilterBuilder, FieldCondition } from './filter-builder.js';

export {
  EdgeVecIndex,
  FilterException,
  FilterStrategy,
  Metadata,
  SearchOptions,
  SearchResult,
  FilteredSearchResult,
  IndexConfig,
  SourcePosition,
} from './edgevec-wrapper.js';

// Default export is the index class
export { EdgeVecIndex as default } from './edgevec-wrapper.js';
