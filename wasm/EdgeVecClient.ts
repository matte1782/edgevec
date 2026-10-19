// wasm/EdgeVecClient.ts

import init, {
  initSync,
  EdgeVec as WasmEdgeVec,
  EdgeVecConfig as WasmConfig,
  WasmCompactionResult
} from '../pkg/edgevec.js';
import type { CompactionResult, SoftDeleteStats } from './types.js';

export interface SearchResult {
  id: number;
  distance: number;
}

export interface EdgeVecClientConfig {
  dimensions: number;
  metric?: 'l2' | 'cosine' | 'dot';
  quantization?: 'none' | 'sq8';
}

/**
 * High-level TypeScript wrapper for EdgeVec WASM bindings.
 *
 * Provides auto-initialization, Promise-based API, and TypeScript ergonomics.
 *
 * @example
 * ```typescript
 * const client = await EdgeVecClient.create({ dimensions: 128 });
 *
 * const vector = new Float32Array(128).fill(0.1);
 * const id = client.insert(vector); // Synchronous
 *
 * const results = client.search(vector, 10); // Synchronous
 * console.log(results);
 *
 * await client.save('my-db');
 * ```
 */
export class EdgeVecClient {
  private static initPromise: Promise<void> | null = null;
  private inner: WasmEdgeVec;
  private constructor(inner: WasmEdgeVec) {
    this.inner = inner;
  }

  /**
   * Create a new EdgeVec instance with auto WASM initialization.
   *
   * @param config - Configuration options
   * @returns Promise resolving to initialized EdgeVecClient
   * @throws Error if dimensions <= 0 or metric is invalid
   */
  static async create(config: EdgeVecClientConfig): Promise<EdgeVecClient> {
    EdgeVecClient.validateConfig(config);
    await EdgeVecClient.ensureInitialized();
    const wasmConfig = new WasmConfig(config.dimensions);
    if (config.metric) wasmConfig.metric = config.metric;
    try { return new EdgeVecClient(new WasmEdgeVec(wasmConfig)); } finally { wasmConfig.free(); }
  }

  private static validateConfig(config: EdgeVecClientConfig): void {
    // C4 Fix: Input validation before WASM calls
    if (!config.dimensions || config.dimensions <= 0) {
      throw new Error(`Dimensions must be positive, got ${config.dimensions}`);
    }
    if (!Number.isInteger(config.dimensions)) {
      throw new Error(`Dimensions must be an integer, got ${config.dimensions}`);
    }
    if (config.metric && !['l2', 'cosine', 'dot'].includes(config.metric)) {
      throw new Error(`Invalid metric: ${config.metric}. Must be 'l2', 'cosine', or 'dot'`);
    }
    // M5 Fix: Explicitly reject quantization parameter
    if (config.quantization && config.quantization !== 'none') {
      throw new Error('Quantization not supported in current WASM API. Remove the quantization parameter from config.');
    }

  }

  /**
   * Load an existing database from IndexedDB.
   *
   * @param name - Database name
   * @param config - Optional configuration; dimensions must match the saved database
   * @returns Promise resolving to loaded EdgeVecClient
   * @throws Error if name is empty or config is invalid
   *
   * Dimensions and live vector count are restored from the saved database.
   */
  static async load(name: string, config?: EdgeVecClientConfig): Promise<EdgeVecClient> {
    // M1 Fix: Validate inputs
    if (!name || name.trim().length === 0) {
      throw new Error('Database name cannot be empty');
    }
    if (config) EdgeVecClient.validateConfig(config);

    await EdgeVecClient.ensureInitialized();

    const inner = await WasmEdgeVec.load(name);
    if (config && config.dimensions !== inner.dimensions()) {
      inner.free();
      throw new Error('Configuration dimensions do not match the saved database');
    }
    return new EdgeVecClient(inner);
  }

  private static async ensureInitialized(): Promise<void> {
    if (!EdgeVecClient.initPromise) {
      EdgeVecClient.initPromise = (async () => {
        if (typeof process !== 'undefined' && process.versions?.node) {
          const nodeFs = 'node:fs/promises';
          const { readFile } = await import(/* webpackIgnore: true */ /* @vite-ignore */ nodeFs);
          const bytes = await readFile(new URL('../pkg/edgevec_bg.wasm', import.meta.url));
          initSync({ module: bytes });
        } else {
          await init();
        }
      })().catch(error => {
        EdgeVecClient.initPromise = null;
        throw error;
      });
    }
    await EdgeVecClient.initPromise;
  }

  /**
   * Insert a vector into the index.
   *
   * @param vector - Float32Array of dimension matching config
   * @returns Assigned vector ID
   * @throws Error if vector dimension doesn't match config
   */
  insert(vector: Float32Array): number {
    this.validateDimension(vector);
    const id = this.inner.insert(vector);
    return id;
  }

  /**
   * Search for k nearest neighbors.
   *
   * @param query - Query vector (Float32Array)
   * @param k - Number of neighbors to return
   * @returns Search results sorted by distance
   * @throws Error if query dimension doesn't match config or k <= 0
   */
  search(query: Float32Array, k: number): SearchResult[] {
    this.validateDimension(query);
    if (k <= 0) {
      throw new Error(`k must be positive, got ${k}`);
    }
    if (!Number.isInteger(k)) {
      throw new Error(`k must be an integer, got ${k}`);
    }
    const results = this.inner.search(query, k);
    // M4 Fix: Type guard for WASM results
    return Array.from(results).map((r) => {
      if (!this.isWasmSearchResult(r)) {
        throw new Error('Invalid search result from WASM');
      }
      // Transform WASM results to typed interface
      // WASM API returns { id, score } but we normalize to { id, distance }
      return {
        id: r.id,
        distance: r.score
      };
    });
  }

  private isWasmSearchResult(value: unknown): value is { id: number; score: number } {
    return (
      typeof value === 'object' &&
      value !== null &&
      'id' in value &&
      'score' in value &&
      typeof (value as { id: unknown }).id === 'number' &&
      typeof (value as { score: unknown }).score === 'number'
    );
  }

  /**
   * Save database to IndexedDB.
   *
   * @param name - Database name for storage
   * @returns Promise resolving when save is complete
   * @throws Error if name is empty
   */
  async save(name: string): Promise<void> {
    // M3 Fix: Validate database name
    if (!name || name.trim().length === 0) {
      throw new Error('Database name cannot be empty');
    }
    await this.inner.save(name);
  }

  /**
   * Get the current live vector count, including restored vectors.
   */
  get length(): number {
    return this.inner.liveCount();
  }

  /**
   * Get the configured dimensions.
   */
  get dimensions(): number {
    return this.inner.dimensions();
  }

  private validateDimension(vector: Float32Array): void {
    if (vector.length !== this.dimensions) {
      throw new Error(
        `Dimension mismatch: expected ${this.dimensions}, got ${vector.length}`
      );
    }
  }

  // =========================================================================
  // SOFT DELETE API (v0.3.0 — RFC-001)
  // =========================================================================

  /**
   * Soft delete a vector by marking it as a tombstone.
   *
   * The vector remains in the index but is excluded from search results.
   * Space is reclaimed via `compact()` when tombstone ratio exceeds threshold.
   *
   * @param vectorId - The ID of the vector to delete (returned from `insert`)
   * @returns `true` if the vector was deleted, `false` if already deleted (idempotent)
   * @throws Error if the vector ID doesn't exist
   *
   * @example
   * ```typescript
   * const id = client.insert(vector);
   * const wasDeleted = client.softDelete(id);
   * console.log(client.isDeleted(id)); // true
   * ```
   */
  softDelete(vectorId: number): boolean {
    return this.inner.softDelete(vectorId);
  }

  /**
   * Check if a vector is deleted (tombstoned).
   *
   * @param vectorId - The ID of the vector to check
   * @returns `true` if the vector is deleted, `false` if live
   * @throws Error if the vector ID doesn't exist
   */
  isDeleted(vectorId: number): boolean {
    return this.inner.isDeleted(vectorId);
  }

  /**
   * Get the count of deleted (tombstoned) vectors.
   *
   * @returns The number of vectors that have been soft-deleted but not yet compacted
   */
  get deletedCount(): number {
    return this.inner.deletedCount();
  }

  /**
   * Get the count of live (non-deleted) vectors.
   *
   * @returns The number of vectors that are currently searchable
   */
  get liveCount(): number {
    return this.inner.liveCount();
  }

  /**
   * Get the ratio of deleted to total vectors.
   *
   * @returns A value between 0.0 and 1.0 representing the tombstone ratio
   */
  get tombstoneRatio(): number {
    return this.inner.tombstoneRatio();
  }

  /**
   * Check if compaction is recommended.
   *
   * Returns `true` when `tombstoneRatio` exceeds the compaction threshold
   * (default: 30%). Use `compact()` to reclaim space from deleted vectors.
   *
   * @returns `true` if compaction is recommended, `false` otherwise
   */
  get needsCompaction(): boolean {
    return this.inner.needsCompaction();
  }

  /**
   * Get the current compaction threshold.
   *
   * @returns The threshold ratio (0.0 to 1.0) above which `needsCompaction` returns true
   */
  get compactionThreshold(): number {
    return this.inner.compactionThreshold();
  }

  /**
   * Set the compaction threshold.
   *
   * @param ratio - The new threshold (clamped to 0.01 - 0.99)
   */
  set compactionThreshold(ratio: number) {
    this.inner.setCompactionThreshold(ratio);
  }

  /**
   * Get a warning message if compaction is recommended.
   *
   * @returns A warning string if `needsCompaction` is true, `null` otherwise
   *
   * @example
   * ```typescript
   * const warning = client.compactionWarning;
   * if (warning) {
   *   console.warn(warning);
   *   client.compact();
   * }
   * ```
   */
  get compactionWarning(): string | null {
    return this.inner.compactionWarning() ?? null;
  }

  /**
   * Compact the index by rebuilding without tombstones.
   *
   * This operation:
   * 1. Creates a new index with only live vectors
   * 2. Re-inserts surviving vectors with sequential IDs
   * 3. Replaces the current index
   *
   * **WARNING:** This is a blocking operation. For indices with >10k vectors,
   * consider running during idle time or warning the user about potential delays.
   *
   * @returns CompactionResult containing metrics about the operation
   * @throws Error if compaction fails (e.g., memory allocation error)
   *
   * @example
   * ```typescript
   * if (client.needsCompaction) {
   *   const result = client.compact();
   *   console.log(`Removed ${result.tombstonesRemoved} tombstones`);
   *   console.log(`New size: ${result.newSize}`);
   *   console.log(`Took ${result.durationMs}ms`);
   * }
   * ```
   */
  compact(): CompactionResult {
    const wasmResult: WasmCompactionResult = this.inner.compact();
    try {
      return {
        tombstonesRemoved: wasmResult.tombstones_removed,
        newSize: wasmResult.new_size,
        durationMs: wasmResult.duration_ms
      };
    } finally { wasmResult.free(); }
  }

  /**
   * Get soft delete statistics for the index.
   *
   * @returns Object containing deletedCount, liveCount, tombstoneRatio, and needsCompaction
   */
  getSoftDeleteStats(): SoftDeleteStats {
    return {
      deletedCount: this.deletedCount,
      liveCount: this.liveCount,
      tombstoneRatio: this.tombstoneRatio,
      needsCompaction: this.needsCompaction
    };
  }
}
