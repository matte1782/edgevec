// wasm/types.ts

/**
 * Vector identifier returned from insert operations.
 */
export type VectorId = number;

/**
 * Distance metrics supported by EdgeVec.
 */
export type DistanceMetric = 'l2' | 'cosine' | 'dot';

/**
 * Quantization modes for memory optimization.
 */
export type QuantizationMode = 'none' | 'sq8';

/**
 * Search result containing vector ID and distance.
 */
export interface SearchResult {
  /** Unique identifier of the matched vector */
  id: VectorId;
  /** Distance from query (lower is more similar for L2/cosine) */
  distance: number;
}

/**
 * Configuration options for EdgeVecClient.
 */
export interface EdgeVecClientConfig {
  /** Vector dimensions (must match all inserted vectors) */
  dimensions: number;
  /** Distance metric for similarity calculation */
  metric?: DistanceMetric;
  /** Quantization mode for memory optimization */
  quantization?: QuantizationMode;
}

/**
 * Statistics about the EdgeVec instance.
 */
export interface EdgeVecStats {
  /** Number of vectors in the index */
  vectorCount: number;
  /** Configured dimensions */
  dimensions: number;
  /** Memory usage in bytes (approximate) */
  memoryBytes: number;
}

/**
 * Result of a compaction operation (v0.3.0).
 *
 * Returned by `compact()` to provide metrics about the operation.
 */
export interface CompactionResult {
  /** Number of tombstones (deleted vectors) removed during compaction */
  tombstonesRemoved: number;
  /** New index size after compaction (live vectors only) */
  newSize: number;
  /** Time taken for the compaction operation in milliseconds */
  durationMs: number;
}

/**
 * Soft delete statistics for the index.
 */
export interface SoftDeleteStats {
  /** Number of vectors that have been soft-deleted */
  deletedCount: number;
  /** Number of vectors that are currently searchable */
  liveCount: number;
  /** Ratio of deleted to total vectors (0.0 to 1.0) */
  tombstoneRatio: number;
  /** Whether compaction is recommended (ratio > threshold) */
  needsCompaction: boolean;
}

// =============================================================================
// METADATA TYPES (v0.6.0 — Week 28 RFC-002)
// =============================================================================

/**
 * Supported metadata value types.
 *
 * When passing metadata to `insertWithMetadata()`, JavaScript values are
 * automatically converted to these types:
 * - `string` → MetadataValue.String
 * - `number` (integer) → MetadataValue.Integer
 * - `number` (float) → MetadataValue.Float
 * - `boolean` → MetadataValue.Boolean
 * - `string[]` → MetadataValue.StringArray
 */
export type MetadataValue = string | number | boolean | string[];

/**
 * Metadata object for a vector.
 *
 * Keys must be:
 * - Non-empty strings
 * - ASCII alphanumeric with underscores (a-z, A-Z, 0-9, _)
 * - Max 64 characters
 *
 * Values can be any of the supported `MetadataValue` types.
 */
export type VectorMetadata = Record<string, MetadataValue>;

/**
 * Search result with optional metadata.
 *
 * Used by `searchWithFilter()` when metadata is requested.
 */
export interface FilteredSearchResult {
  /** Unique identifier of the matched vector */
  id: VectorId;
  /** Distance from query (lower is more similar for L2/cosine) */
  distance: number;
  /** Optional metadata if requested */
  metadata?: VectorMetadata;
}

/**
 * Options for filtered search operations.
 */
export interface FilteredSearchOptions {
  /** Filter expression string (e.g., 'category == "news" AND score > 0.5') */
  filter: string;
  /** Number of results to return */
  k: number;
  /** Whether to include metadata in results */
  includeMetadata?: boolean;
  /** Whether to include vector data in results */
  includeVectors?: boolean;
}

/**
 * Filter expression syntax reference.
 *
 * Comparison operators:
 * - `==`, `!=` — Equality
 * - `<`, `<=`, `>`, `>=` — Numeric comparison
 *
 * Logical operators:
 * - `AND`, `&&` — Logical AND
 * - `OR`, `||` — Logical OR
 * - `NOT`, `!` — Logical NOT
 *
 * String operators:
 * - `STARTS_WITH` — String prefix match
 * - `ENDS_WITH` — String suffix match
 * - `CONTAINS` — Substring match
 * - `LIKE` — Pattern match (% = any chars, _ = single char)
 *
 * Array operators:
 * - `IN` — Value in array
 * - `NOT IN` — Value not in array
 * - `ANY` — Any element matches
 * - `ALL` — All elements match
 *
 * Null checks:
 * - `IS NULL` — Field is missing
 * - `IS NOT NULL` — Field exists
 *
 * Range:
 * - `BETWEEN low AND high` — Inclusive range
 *
 * Examples:
 * - `category == "news"`
 * - `score > 0.5 AND active == true`
 * - `tags CONTAINS "featured"`
 * - `price BETWEEN 10 AND 100`
 * - `NOT (status == "archived")`
 */
export type FilterExpression = string;

// =============================================================================
// BINARY QUANTIZATION TYPES (v0.6.0 — Week 28 RFC-002)
// =============================================================================

/**
 * Search result from Binary Quantization search.
 *
 * BQ search uses Hamming distance on binary-quantized vectors for fast
 * approximate search. Lower distance means more similar.
 */
export interface BQSearchResult {
  /** Unique identifier of the matched vector */
  id: VectorId;
  /** Hamming distance from query (lower is more similar) */
  distance: number;
}

/**
 * Options for hybrid search combining BQ and metadata filtering.
 *
 * @example
 * ```typescript
 * const results = client.searchHybrid(queryVector, {
 *   k: 10,
 *   filter: 'category == "news"',
 *   useBQ: true,
 *   rescoreFactor: 4
 * });
 * ```
 */
export interface HybridSearchOptions {
  /** Number of results to return */
  k: number;
  /**
   * Optional filter expression string.
   * If provided, only vectors matching the filter are returned.
   */
  filter?: string;
  /**
   * Whether to use Binary Quantization for initial candidate generation.
   * When true, BQ is used for fast approximate search, then results are
   * rescored using full-precision vectors.
   * @default false
   */
  useBQ?: boolean;
  /**
   * Rescore factor for BQ search.
   * Generates k * rescoreFactor candidates with BQ, then rescores with F32.
   * Higher values improve recall but increase latency.
   * Only used when useBQ is true.
   * @default 4
   */
  rescoreFactor?: number;
}

/**
 * Binary Quantization configuration.
 *
 * BQ provides 32x memory reduction by encoding each vector dimension as a
 * single bit (positive = 1, non-positive = 0).
 *
 * Trade-offs:
 * - Memory: 32x reduction (128 bytes → 4 bytes for 1024d vectors)
 * - Speed: 3-5x faster search using Hamming distance
 * - Recall: ~70-85% without rescoring, ~95% with rescoring
 */
export interface BinaryQuantizationInfo {
  /** Whether BQ is enabled for this index */
  enabled: boolean;
  /** Number of bytes per quantized vector (dimensions / 8) */
  bytesPerVector: number;
  /** Estimated memory savings compared to F32 storage */
  memorySavingsRatio: number;
}
