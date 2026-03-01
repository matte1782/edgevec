/**
 * Type definitions for edgevec-langchain
 *
 * @module types
 */

// Import from edgevec-wrapper directly since edgevec's package.json
// points `types` to edgevec.d.ts (low-level), not the wrapper index.
import type { IndexConfig } from "edgevec/edgevec-wrapper.js";

/**
 * Supported distance metrics for score normalization.
 *
 * - `"cosine"` — Cosine distance (0 = identical). Normalized via `1 - distance`.
 * - `"l2"` — Euclidean distance (0 = identical). Normalized via `1 / (1 + distance)`.
 * - `"dotproduct"` — Negative inner product. Normalized via `1 / (1 + |distance|)`.
 */
export type EdgeVecMetric = "cosine" | "l2" | "dotproduct";

/**
 * Configuration for EdgeVecStore.
 *
 * Extends EdgeVec's `IndexConfig` with adapter-specific fields.
 * The `metric` field is NOT passed to EdgeVec's `IndexConfig` —
 * it is stored internally by the adapter for score normalization at query time.
 *
 * [HOSTILE REVIEW FIX C1]: IndexConfig does NOT have a `metric` field.
 * It only has: `dimensions`, `m`, `efConstruction`, `quantized`.
 * The metric is an adapter-level concern for LangChain score normalization.
 */
export interface EdgeVecStoreConfig extends IndexConfig {
  /**
   * Distance metric used by the index.
   * Used for score normalization (distance → similarity).
   * NOT passed to EdgeVec IndexConfig.
   *
   * @default "cosine"
   */
  metric?: EdgeVecMetric;
}

/**
 * Error thrown when metadata serialization fails.
 */
export class MetadataSerializationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MetadataSerializationError";
  }
}

/**
 * Error thrown when persistence operations fail.
 */
export class EdgeVecPersistenceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "EdgeVecPersistenceError";
  }
}
