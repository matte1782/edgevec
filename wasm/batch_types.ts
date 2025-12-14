// wasm/batch_types.ts
// TypeScript type definitions for EdgeVec batch insert API
// Week 12, Day 1 (W12.1)

/**
 * Configuration options for batch insert operations.
 *
 * @remarks
 * These options control how batch insertions are validated and processed.
 * All options are optional with sensible defaults.
 *
 * @example
 * ```typescript
 * const config: BatchInsertConfig = {
 *   validateDimensions: true, // Enable dimension validation (default)
 * };
 * ```
 */
export interface BatchInsertConfig {
  /**
   * If true, validates all vectors have matching dimensions before insertion.
   * When false, dimension validation is skipped for better performance,
   * but mismatched vectors may cause runtime errors.
   *
   * @default true
   */
  validateDimensions?: boolean;
}

/**
 * Result of a batch insert operation.
 *
 * @remarks
 * Contains information about how many vectors were successfully inserted
 * and their assigned IDs. The `inserted` count may be less than `total`
 * if some vectors were skipped due to non-fatal errors (e.g., duplicates).
 *
 * @example
 * ```typescript
 * const result = await index.insertBatch(vectors);
 * console.log(`Inserted ${result.inserted} of ${result.total} vectors`);
 * console.log(`IDs: ${result.ids.join(', ')}`);
 * ```
 */
export interface BatchInsertResult {
  /**
   * Number of vectors successfully inserted.
   * May be less than `total` if some vectors were skipped (best-effort semantics).
   *
   * @minimum 0
   */
  inserted: number;

  /**
   * Total number of vectors attempted (same as input array length).
   *
   * @minimum 0
   */
  total: number;

  /**
   * IDs of inserted vectors, in insertion order.
   * Array length equals `inserted`.
   */
  ids: number[];
}

/**
 * Error thrown when batch insert fails.
 *
 * @remarks
 * Maps to Rust BatchError variants. Extends the standard Error type
 * with a `code` property for programmatic error handling and an
 * optional `details` property for additional context.
 *
 * @example
 * ```typescript
 * try {
 *   await index.insertBatch(vectors);
 * } catch (error) {
 *   const batchError = error as BatchInsertError;
 *   switch (batchError.code) {
 *     case 'EMPTY_BATCH':
 *       console.error('Cannot insert empty batch');
 *       break;
 *     case 'DIMENSION_MISMATCH':
 *       console.error('Vector dimensions do not match index');
 *       break;
 *     default:
 *       console.error(`Batch error: ${batchError.message}`);
 *   }
 * }
 * ```
 */
export interface BatchInsertError extends Error {
  /**
   * Error code for programmatic handling.
   *
   * @description
   * Maps 1:1 to Rust BatchError variants:
   * - `EMPTY_BATCH` - Empty array provided (vectors.length === 0)
   * - `DIMENSION_MISMATCH` - Vector dimensions do not match index configuration
   * - `DUPLICATE_ID` - Vector ID already exists in the index
   * - `INVALID_VECTOR` - Vector contains NaN or Infinity values
   * - `CAPACITY_EXCEEDED` - Batch would exceed maximum index capacity
   * - `INTERNAL_ERROR` - HNSW graph invariant violated (bug)
   */
  code: 'EMPTY_BATCH' | 'DIMENSION_MISMATCH' | 'DUPLICATE_ID' | 'INVALID_VECTOR' | 'CAPACITY_EXCEEDED' | 'INTERNAL_ERROR';

  /**
   * Additional context from Rust error message.
   * May include details like expected vs actual dimensions,
   * duplicate ID value, or capacity limits.
   */
  details?: string;
}
