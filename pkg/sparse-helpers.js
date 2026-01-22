/**
 * Sparse/Hybrid Search Helper Functions (v0.9.0 — Week 39 RFC-007)
 *
 * Convenience functions for working with sparse vectors and parsing
 * hybrid search results.
 *
 * @module edgevec/sparse-helpers
 */

/**
 * Create a sparse vector from term-score pairs.
 *
 * @param {Record<number, number>} termScores - Object mapping term IDs to scores
 * @param {number} dim - Vocabulary size (dimension of sparse space)
 * @returns {SparseVector} SparseVector ready for insertion/search
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
export function createSparseVector(termScores, dim) {
  // Extract and sort indices
  const entries = Object.entries(termScores)
    .map(([key, value]) => [parseInt(key, 10), value])
    .filter(([idx, val]) => !isNaN(idx) && val !== 0) // Filter out NaN keys and zero values
    .sort((a, b) => a[0] - b[0]); // Sort by index ascending

  const indices = new Uint32Array(entries.map(([idx]) => idx));
  const values = new Float32Array(entries.map(([, val]) => val));

  return {
    indices,
    values,
    dim,
  };
}

/**
 * Parse hybrid search results from JSON string.
 *
 * @param {string} json - JSON string from hybridSearch()
 * @returns {HybridSearchResult[]} Typed array of HybridSearchResult
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
export function parseHybridResults(json) {
  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) {
      throw new Error('Expected array of results');
    }
    return parsed;
  } catch (e) {
    throw new Error(`Failed to parse hybrid search results: ${e.message}`);
  }
}

/**
 * Parse sparse search results from JSON string.
 *
 * @param {string} json - JSON string from searchSparse()
 * @returns {SparseSearchResult[]} Typed array of SparseSearchResult
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
export function parseSparseResults(json) {
  try {
    const parsed = JSON.parse(json);
    if (!Array.isArray(parsed)) {
      throw new Error('Expected array of results');
    }
    return parsed;
  } catch (e) {
    throw new Error(`Failed to parse sparse search results: ${e.message}`);
  }
}

/**
 * Create hybrid search options JSON string.
 *
 * Convenience function for building the options JSON required by hybridSearch().
 *
 * @param {Object} options - Hybrid search options
 * @param {number} options.k - Final number of results to return (required)
 * @param {number} [options.dense_k=20] - Number of results from dense search
 * @param {number} [options.sparse_k=20] - Number of results from sparse search
 * @param {'rrf' | { type: 'linear', alpha: number }} [options.fusion='rrf'] - Fusion method
 * @returns {string} JSON string ready for hybridSearch()
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
export function createHybridOptions(options) {
  if (typeof options.k !== 'number' || options.k <= 0) {
    throw new Error('options.k must be a positive number');
  }

  const opts = {
    k: options.k,
  };

  if (options.dense_k !== undefined) {
    opts.dense_k = options.dense_k;
  }
  if (options.sparse_k !== undefined) {
    opts.sparse_k = options.sparse_k;
  }
  if (options.fusion !== undefined) {
    opts.fusion = options.fusion;
  }

  return JSON.stringify(opts);
}
