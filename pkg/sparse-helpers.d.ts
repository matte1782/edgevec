/**
 * Sparse/Hybrid Search Helper Functions (v0.9.0 -- Week 39 RFC-007)
 *
 * Convenience functions for working with sparse vectors and parsing
 * hybrid search results.
 *
 * @module edgevec/sparse-helpers
 * @version 0.9.0
 */

import type { SparseVector, SparseSearchResult, HybridSearchResult, HybridSearchOptions } from './edgevec-types.js';

/**
 * Create a sparse vector from term-score pairs.
 *
 * Converts a plain object mapping term IDs to scores into a SparseVector
 * with sorted Uint32Array indices and Float32Array values.
 *
 * @param termScores - Object mapping term IDs (as string keys) to scores.
 *   Zero values and non-numeric keys are filtered out.
 * @param dim - Vocabulary size (dimension of sparse space)
 * @returns SparseVector ready for insertion/search
 *
 * @example
 * ```typescript
 * // From BM25 scores
 * const bm25Scores = { 42: 2.5, 100: 1.8, 500: 3.2 };
 * const sparse = createSparseVector(bm25Scores, 10000);
 *
 * // Use with EdgeVec
 * const id = db.insertSparse(sparse.indices, sparse.values, sparse.dim);
 * ```
 */
export function createSparseVector(
  termScores: Record<number, number>,
  dim: number
): SparseVector;

/**
 * Parse hybrid search results from JSON string.
 *
 * @param json - JSON string from hybridSearch() (raw WASM interface)
 * @returns Typed array of HybridSearchResult
 * @throws Error if JSON is malformed or not an array
 *
 * @example
 * ```typescript
 * const resultsJson = db.hybridSearch(dense, sparseIdx, sparseVal, dim, opts);
 * const results = parseHybridResults(resultsJson);
 *
 * for (const r of results) {
 *   console.log(`ID: ${r.id}, Score: ${r.score}`);
 *   if (r.dense_rank) console.log(`  Dense rank: ${r.dense_rank}`);
 *   if (r.sparse_rank) console.log(`  Sparse rank: ${r.sparse_rank}`);
 * }
 * ```
 */
export function parseHybridResults(json: string): HybridSearchResult[];

/**
 * Parse sparse search results from JSON string.
 *
 * @param json - JSON string from searchSparse() (raw WASM interface)
 * @returns Typed array of SparseSearchResult
 * @throws Error if JSON is malformed or not an array
 *
 * @example
 * ```typescript
 * const resultsJson = db.searchSparse(indices, values, dim, k);
 * const results = parseSparseResults(resultsJson);
 *
 * for (const r of results) {
 *   console.log(`ID: ${r.id}, Score: ${r.score}`);
 * }
 * ```
 */
export function parseSparseResults(json: string): SparseSearchResult[];

/**
 * Create hybrid search options JSON string.
 *
 * Convenience function for building the options JSON required by the
 * raw WASM hybridSearch() interface.
 *
 * @param options - Hybrid search options
 * @returns JSON string ready for hybridSearch()
 * @throws Error if options.k is not a positive number
 *
 * @example
 * ```typescript
 * // RRF fusion (default, recommended)
 * const opts1 = createHybridOptions({ k: 10 });
 *
 * // Linear fusion with 70% dense, 30% sparse
 * const opts2 = createHybridOptions({
 *   k: 10,
 *   dense_k: 50,
 *   sparse_k: 50,
 *   fusion: { type: 'linear', alpha: 0.7 }
 * });
 *
 * const results = db.hybridSearch(dense, idx, val, dim, opts2);
 * ```
 */
export function createHybridOptions(options: HybridSearchOptions): string;
