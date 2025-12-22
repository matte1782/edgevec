/* tslint:disable */
/* eslint-disable */

export class BatchInsertConfig {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Creates a new `BatchInsertConfig` with default settings.
   *
   * Default: `validate_dimensions = true`
   */
  constructor();
  /**
   * Returns whether dimension validation is enabled.
   */
  validateDimensions: boolean;
}

export class BatchInsertResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Returns a copy of the IDs of successfully inserted vectors.
   */
  readonly ids: BigUint64Array;
  /**
   * Returns the total number of vectors attempted (input array length).
   */
  readonly total: number;
  /**
   * Returns the number of vectors successfully inserted.
   */
  readonly inserted: number;
}

export class EdgeVec {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Loads the database from IndexedDB.
   *
   * # Arguments
   *
   * * `name` - The name of the database file in IndexedDB.
   *
   * # Returns
   *
   * A Promise that resolves to the loaded EdgeVec instance.
   *
   * # Errors
   *
   * Returns an error if loading fails, deserialization fails, or data is corrupted.
   */
  static load(name: string): Promise<EdgeVec>;
  /**
   * Saves the database to IndexedDB.
   *
   * # Arguments
   *
   * * `name` - The name of the database file in IndexedDB.
   *
   * # Returns
   *
   * A Promise that resolves when saving is complete.
   *
   * # Errors
   *
   * Returns an error if serialization fails or if the backend write fails.
   */
  save(name: string): Promise<void>;
  /**
   * Check if a vector is deleted (tombstoned).
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector to check.
   *
   * # Returns
   *
   * * `true` if the vector is deleted
   * * `false` if the vector is live
   *
   * # Errors
   *
   * Returns an error if the vector ID doesn't exist.
   */
  isDeleted(vector_id: number): boolean;
  /**
   * Get the count of live (non-deleted) vectors.
   *
   * # Returns
   *
   * The number of vectors that are currently searchable.
   */
  liveCount(): number;
  /**
   * Creates an iterator to save the database in chunks.
   *
   * # Arguments
   *
   * * `chunk_size` - Maximum size of each chunk in bytes (default: 10MB).
   *
   * # Returns
   *
   * A `PersistenceIterator` that yields `Uint8Array` chunks.
   *
   * # Safety
   *
   * The returned iterator holds a reference to this `EdgeVec` instance.
   * You MUST ensure `EdgeVec` is not garbage collected or freed while using the iterator.
   */
  save_stream(chunk_size?: number | null): PersistenceIterator;
  /**
   * Soft delete a vector by marking it as a tombstone.
   *
   * The vector remains in the index but is excluded from search results.
   * Space is reclaimed via `compact()` when tombstone ratio exceeds threshold.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector to delete (returned from `insert`).
   *
   * # Returns
   *
   * * `true` if the vector was deleted
   * * `false` if the vector was already deleted (idempotent)
   *
   * # Errors
   *
   * Returns an error if the vector ID doesn't exist.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const id = index.insert(new Float32Array(128).fill(1.0));
   * const wasDeleted = index.softDelete(id);
   * console.log(`Deleted: ${wasDeleted}`); // true
   * console.log(`Is deleted: ${index.isDeleted(id)}`); // true
   * ```
   */
  softDelete(vector_id: number): boolean;
  /**
   * Gets metadata for a vector.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   * * `key` - The metadata key to retrieve
   *
   * # Returns
   *
   * The metadata value, or `undefined` if the key or vector doesn't exist.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const title = index.getMetadata(id, 'title');
   * if (title) {
   *     console.log('Title:', title.asString());
   *     console.log('Type:', title.getType());
   * }
   * ```
   */
  getMetadata(vector_id: number, key: string): JsMetadataValue | undefined;
  /**
   * Checks if a metadata key exists for a vector.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   * * `key` - The metadata key to check
   *
   * # Returns
   *
   * `true` if the key exists, `false` otherwise.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * if (index.hasMetadata(id, 'title')) {
   *     console.log('Vector has title metadata');
   * }
   * ```
   */
  hasMetadata(vector_id: number, key: string): boolean;
  /**
   * Sets metadata for a vector (upsert operation).
   *
   * If the key already exists, its value is overwritten. If the key is new,
   * it is added (subject to the 64-key-per-vector limit).
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector to attach metadata to
   * * `key` - The metadata key (alphanumeric + underscore, max 256 chars)
   * * `value` - The metadata value (created via JsMetadataValue.fromX methods)
   *
   * # Errors
   *
   * Returns an error if:
   * - Key is empty or contains invalid characters
   * - Key exceeds 256 characters
   * - Value validation fails (e.g., NaN float, string too long)
   * - Vector already has 64 keys and this is a new key
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const id = index.insert(vector);
   * index.setMetadata(id, 'title', JsMetadataValue.fromString('My Document'));
   * index.setMetadata(id, 'page_count', JsMetadataValue.fromInteger(42));
   * index.setMetadata(id, 'score', JsMetadataValue.fromFloat(0.95));
   * index.setMetadata(id, 'verified', JsMetadataValue.fromBoolean(true));
   * ```
   */
  setMetadata(vector_id: number, key: string, value: JsMetadataValue): void;
  /**
   * Get the count of deleted (tombstoned) vectors.
   *
   * # Returns
   *
   * The number of vectors that have been soft-deleted but not yet compacted.
   */
  deletedCount(): number;
  /**
   * Hybrid search combining BQ speed with metadata filtering.
   *
   * This is the most flexible search method, combining:
   * - Binary quantization for speed
   * - Metadata filtering for precision
   * - Optional F32 rescoring for accuracy
   *
   * # Arguments
   *
   * * `query` - A Float32Array containing the query vector
   * * `options` - A JavaScript object with search options:
   *   - `k` (required): Number of results to return
   *   - `filter` (optional): Filter expression string
   *   - `useBQ` (optional, default true): Use binary quantization
   *   - `rescoreFactor` (optional, default 3): Overfetch multiplier
   *
   * # Returns
   *
   * An array of search result objects: `[{ id: number, distance: number }, ...]`
   *
   * # Errors
   *
   * Returns error if:
   * - Options is not a valid object
   * - k is 0 or missing
   * - Filter expression is invalid
   * - Query dimensions mismatch
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const results = index.searchHybrid(
   *     new Float32Array([0.1, 0.2, ...]),
   *     {
   *         k: 10,
   *         filter: 'category == "news" AND score > 0.5',
   *         useBQ: true,
   *         rescoreFactor: 3
   *     }
   * );
   * ```
   */
  searchHybrid(query: Float32Array, options: any): any;
  /**
   * Deletes a metadata key for a vector.
   *
   * This operation is idempotent - deleting a non-existent key is not an error.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   * * `key` - The metadata key to delete
   *
   * # Returns
   *
   * `true` if the key existed and was deleted, `false` otherwise.
   *
   * # Errors
   *
   * Returns an error if the key is invalid (empty or contains invalid characters).
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const wasDeleted = index.deleteMetadata(id, 'title');
   * console.log(wasDeleted); // true if key existed
   * ```
   */
  deleteMetadata(vector_id: number, key: string): boolean;
  /**
   * Inserts multiple vectors using the new batch API (W12.3).
   *
   * This method follows the API design from `WASM_BATCH_API.md`:
   * - Input: Array of Float32Array (each array is one vector)
   * - Output: BatchInsertResult with inserted count, total, and IDs
   * - Error codes: EMPTY_BATCH, DIMENSION_MISMATCH, DUPLICATE_ID, etc.
   *
   * # Arguments
   *
   * * `vectors` - JS Array of Float32Array vectors to insert (1 to 100,000)
   * * `config` - Optional BatchInsertConfig (default: validateDimensions = true)
   *
   * # Returns
   *
   * `BatchInsertResult` containing:
   * - `inserted`: Number of vectors successfully inserted
   * - `total`: Total vectors attempted (input array length)
   * - `ids`: Array of IDs for inserted vectors
   *
   * # Performance Note
   *
   * Batch insert optimizes **JavaScript↔WASM boundary overhead**, not HNSW graph
   * construction. At smaller batch sizes (100-1K vectors), expect 1.2-1.5x speedup
   * vs sequential insertion due to reduced FFI calls. At larger scales (5K+), both
   * methods converge as HNSW graph construction becomes the dominant cost.
   *
   * The batch API still provides value at all scales through:
   * - Simpler API (single call vs loop)
   * - Atomic operation semantics
   * - Progress callback support (via `insertBatchWithProgress`)
   *
   * # Errors
   *
   * Returns a JS error object with `code` property:
   * - `EMPTY_BATCH`: Input array is empty
   * - `DIMENSION_MISMATCH`: Vector dimensions don't match index
   * - `DUPLICATE_ID`: Vector ID already exists
   * - `INVALID_VECTOR`: Vector contains NaN or Infinity
   * - `CAPACITY_EXCEEDED`: Batch exceeds max capacity
   * - `INTERNAL_ERROR`: Internal HNSW error
   */
  insertBatch(vectors: Array<any>, config?: BatchInsertConfig | null): BatchInsertResult;
  /**
   * Execute a filtered search on the index.
   *
   * Combines HNSW vector search with metadata filtering using configurable
   * strategies (pre-filter, post-filter, hybrid, auto).
   *
   * # Arguments
   *
   * * `query` - A Float32Array containing the query vector
   * * `k` - Number of results to return
   * * `options_json` - JSON object with search options:
   *   ```json
   *   {
   *     "filter": "category = \"gpu\"",  // optional filter expression
   *     "strategy": "auto",              // "auto" | "pre" | "post" | "hybrid"
   *     "oversampleFactor": 3.0,         // for post/hybrid strategies
   *     "includeMetadata": true,         // include metadata in results
   *     "includeVectors": false          // include vectors in results
   *   }
   *   ```
   *
   * # Returns
   *
   * JSON string with search results:
   * ```json
   * {
   *   "results": [{ "id": 42, "score": 0.95, "metadata": {...}, "vector": [...] }],
   *   "complete": true,
   *   "observedSelectivity": 0.15,
   *   "strategyUsed": "hybrid",
   *   "vectorsEvaluated": 150,
   *   "filterTimeMs": 2.5,
   *   "totalTimeMs": 8.3
   * }
   * ```
   *
   * # Errors
   *
   * Returns an error if:
   * - Query dimensions don't match index
   * - Filter expression is invalid
   * - Options JSON is malformed
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const query = new Float32Array([0.1, 0.2, ...]);
   * const result = JSON.parse(index.searchFiltered(query, 10, JSON.stringify({
   *     filter: 'category = "gpu" AND price < 500',
   *     strategy: 'auto'
   * })));
   * console.log(`Found ${result.results.length} results`);
   * ```
   */
  searchFiltered(query: Float32Array, k: number, options_json: string): string;
  /**
   * Get the ratio of deleted to total vectors.
   *
   * # Returns
   *
   * A value between 0.0 and 1.0 representing the tombstone ratio.
   * 0.0 means no deletions, 1.0 means all vectors deleted.
   */
  tombstoneRatio(): number;
  /**
   * Gets all metadata for a vector as a JavaScript object.
   *
   * Returns a plain JavaScript object where keys are metadata keys and
   * values are JavaScript-native types (string, number, boolean, string[]).
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   *
   * # Returns
   *
   * A JavaScript object mapping keys to values, or `undefined` if the vector
   * has no metadata.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const metadata = index.getAllMetadata(id);
   * if (metadata) {
   *     console.log(metadata.title);     // 'My Document'
   *     console.log(metadata.page_count); // 42
   *     console.log(Object.keys(metadata)); // ['title', 'page_count', ...]
   * }
   * ```
   */
  getAllMetadata(vector_id: number): any;
  /**
   * Check if compaction is recommended.
   *
   * Returns `true` when `tombstoneRatio()` exceeds the compaction threshold
   * (default: 30%). Use `compact()` to reclaim space from deleted vectors.
   *
   * # Returns
   *
   * * `true` if compaction is recommended
   * * `false` if tombstone ratio is below threshold
   */
  needsCompaction(): boolean;
  /**
   * Inserts a batch of vectors into the index (flat array format).
   *
   * **Note:** This is the legacy API. For the new API, use `insertBatch` which
   * accepts an Array of Float32Array.
   *
   * # Arguments
   *
   * * `vectors` - Flat Float32Array containing `count * dimensions` elements.
   * * `count` - Number of vectors in the batch.
   *
   * # Returns
   *
   * A Uint32Array containing the assigned Vector IDs.
   *
   * # Errors
   *
   * Returns error if dimensions mismatch, vector contains NaNs, or ID overflows.
   */
  insertBatchFlat(vectors: Float32Array, count: number): Uint32Array;
  /**
   * Soft-delete multiple vectors using BigUint64Array (modern browsers).
   *
   * Efficiently deletes multiple vectors in a single operation. More efficient
   * than calling `softDelete()` N times due to reduced FFI overhead and
   * deduplication of input IDs.
   *
   * **Browser Compatibility:** Requires BigUint64Array support (Chrome 67+,
   * Firefox 68+, Safari 15+). For Safari 14 compatibility, use
   * `softDeleteBatchCompat()` instead.
   *
   * # Arguments
   *
   * * `ids` - A Uint32Array of vector IDs to delete
   *
   * # Returns
   *
   * A `WasmBatchDeleteResult` object containing:
   * * `deleted` - Number of vectors successfully deleted
   * * `alreadyDeleted` - Number of vectors that were already deleted
   * * `invalidIds` - Number of IDs not found in the index
   * * `total` - Total IDs in input (including duplicates)
   * * `uniqueCount` - Number of unique IDs after deduplication
   *
   * # Behavior
   *
   * * **Deduplication:** Duplicate IDs in input are processed only once
   * * **Idempotent:** Re-deleting an already-deleted vector returns `alreadyDeleted`
   * * **Atomic:** Two-phase validation ensures all-or-nothing semantics
   *
   * # Errors
   *
   * Returns an error if the batch size exceeds the maximum (10M IDs).
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const ids = new Uint32Array([1, 3, 5, 7, 9, 11]);
   * const result = index.softDeleteBatch(ids);
   *
   * console.log(`Deleted: ${result.deleted}`);
   * console.log(`Already deleted: ${result.alreadyDeleted}`);
   * console.log(`Invalid IDs: ${result.invalidIds}`);
   * console.log(`All valid: ${result.allValid()}`);
   * ```
   */
  softDeleteBatch(ids: Uint32Array): WasmBatchDeleteResult;
  /**
   * Get a warning message if compaction is recommended.
   *
   * # Returns
   *
   * * A warning string if `needsCompaction()` is true
   * * `null` if compaction is not needed
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const warning = index.compactionWarning();
   * if (warning) {
   *     console.warn(warning);
   *     index.compact();
   * }
   * ```
   */
  compactionWarning(): string | undefined;
  /**
   * Returns the number of metadata keys for a vector.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   *
   * # Returns
   *
   * The number of metadata keys, or 0 if the vector has no metadata.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const count = index.metadataKeyCount(id);
   * console.log(`Vector has ${count} metadata keys`);
   * ```
   */
  metadataKeyCount(vector_id: number): number;
  /**
   * Search using BQ with F32 rescoring (fast + accurate).
   *
   * This method combines BQ speed with F32 accuracy:
   * 1. Uses BQ to quickly find `k * rescoreFactor` candidates
   * 2. Rescores candidates using exact F32 distance
   * 3. Returns the final top-k results
   *
   * This provides near-F32 recall (~95%) with most of the BQ speedup.
   *
   * # Arguments
   *
   * * `query` - A Float32Array containing the query vector
   * * `k` - Number of results to return
   * * `rescore_factor` - Overfetch multiplier (3-10 recommended)
   *
   * # Returns
   *
   * An array of search result objects: `[{ id: number, distance: number }, ...]`
   * where distance is a similarity score (higher is more similar).
   *
   * # Errors
   *
   * Returns error if:
   * - Binary quantization is not enabled on this index
   * - Query dimensions mismatch
   * - k or rescore_factor is 0
   *
   * # Rescore Factor Guide
   *
   * | Factor | Recall | Relative Speed |
   * |--------|--------|----------------|
   * | 1      | ~70%   | 5x             |
   * | 3      | ~90%   | 3x             |
   * | 5      | ~95%   | 2.5x           |
   * | 10     | ~98%   | 2x             |
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * // Fast search with high recall (~95%)
   * const results = index.searchBQRescored(
   *     new Float32Array([0.1, 0.2, ...]),
   *     10,  // k
   *     5    // rescore factor
   * );
   * ```
   */
  searchBQRescored(query: Float32Array, k: number, rescore_factor: number): any;
  /**
   * Search with metadata filter expression (simplified API).
   *
   * This is a simplified version of `searchFiltered()` that takes the filter
   * expression directly as a string instead of JSON options.
   *
   * # Arguments
   *
   * * `query` - A Float32Array containing the query vector
   * * `filter` - Filter expression string (e.g., 'category == "news" AND score > 0.5')
   * * `k` - Number of results to return
   *
   * # Returns
   *
   * An array of search result objects: `[{ id: number, distance: number }, ...]`
   *
   * # Filter Syntax
   *
   * - Comparison: `field == value`, `field != value`, `field > value`, etc.
   * - Logical: `expr AND expr`, `expr OR expr`, `NOT expr`
   * - Grouping: `(expr)`
   * - Array contains: `field CONTAINS value`
   *
   * # Errors
   *
   * Returns error if:
   * - Query dimensions mismatch
   * - Filter expression is invalid
   * - k is 0
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const results = index.searchWithFilter(
   *     new Float32Array([0.1, 0.2, ...]),
   *     'category == "news" AND score > 0.5',
   *     10
   * );
   * for (const r of results) {
   *     console.log(`ID: ${r.id}, Distance: ${r.distance}`);
   * }
   * ```
   */
  searchWithFilter(query: Float32Array, filter: string, k: number): any;
  /**
   * Deletes all metadata for a vector.
   *
   * This operation is idempotent - deleting metadata for a vector without
   * metadata is not an error.
   *
   * # Arguments
   *
   * * `vector_id` - The ID of the vector
   *
   * # Returns
   *
   * `true` if the vector had metadata that was deleted, `false` otherwise.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const hadMetadata = index.deleteAllMetadata(id);
   * console.log(hadMetadata); // true if vector had any metadata
   * ```
   */
  deleteAllMetadata(vector_id: number): boolean;
  /**
   * Get all metadata for a vector by ID (alias for getAllMetadata).
   *
   * This is an alias for `getAllMetadata()` provided for API consistency
   * with the new RFC-002 metadata API.
   *
   * # Arguments
   *
   * * `id` - The vector ID to look up
   *
   * # Returns
   *
   * A JavaScript object with all metadata key-value pairs, or `undefined`
   * if the vector has no metadata or doesn't exist.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const id = index.insertWithMetadata(vector, { category: 'news' });
   * const meta = index.getVectorMetadata(id);
   * console.log(meta.category); // 'news'
   * ```
   */
  getVectorMetadata(id: number): any;
  /**
   * Get the current compaction threshold.
   *
   * # Returns
   *
   * The threshold ratio (0.0 to 1.0) above which `needsCompaction()` returns true.
   * Default is 0.3 (30%).
   */
  compactionThreshold(): number;
  /**
   * Insert a vector with associated metadata in a single operation.
   *
   * This is a convenience method that combines `insert()` and `setMetadata()`
   * into a single atomic operation. The vector is inserted first, then all
   * metadata key-value pairs are attached to it.
   *
   * # Arguments
   *
   * * `vector` - A Float32Array containing the vector data
   * * `metadata` - A JavaScript object with string keys and metadata values
   *   - Supported value types: `string`, `number`, `boolean`, `string[]`
   *   - Numbers are automatically detected as integer or float
   *
   * # Returns
   *
   * The assigned Vector ID (u32).
   *
   * # Errors
   *
   * Returns error if:
   * - Vector dimensions mismatch the index configuration
   * - Vector contains NaN or Infinity values
   * - Metadata key is invalid (empty, too long, or contains invalid characters)
   * - Metadata value is invalid (NaN float, string too long, etc.)
   * - Too many metadata keys (>64 per vector)
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const id = index.insertWithMetadata(
   *     new Float32Array([0.1, 0.2, 0.3, ...]),
   *     {
   *         category: "news",
   *         score: 0.95,
   *         active: true,
   *         tags: ["featured", "trending"]
   *     }
   * );
   * console.log(`Inserted vector with ID: ${id}`);
   * ```
   */
  insertWithMetadata(vector: Float32Array, metadata_js: any): number;
  /**
   * Returns the total number of metadata key-value pairs across all vectors.
   *
   * # Returns
   *
   * The total count of metadata entries.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const total = index.totalMetadataCount();
   * console.log(`${total} total metadata entries`);
   * ```
   */
  totalMetadataCount(): number;
  /**
   * Returns the total number of vectors with metadata.
   *
   * # Returns
   *
   * The count of vectors that have at least one metadata key.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const count = index.metadataVectorCount();
   * console.log(`${count} vectors have metadata`);
   * ```
   */
  metadataVectorCount(): number;
  /**
   * Set the compaction threshold.
   *
   * # Arguments
   *
   * * `ratio` - The new threshold (clamped to 0.01 - 0.99).
   */
  setCompactionThreshold(ratio: number): void;
  /**
   * Soft-delete multiple vectors using number array (Safari 14 compatible).
   *
   * This method provides Safari 14 compatibility by accepting a regular JavaScript
   * Array of numbers instead of BigUint64Array. IDs must be less than 2^53
   * (Number.MAX_SAFE_INTEGER) to avoid precision loss.
   *
   * **Note:** For modern browsers, prefer `softDeleteBatch()` which uses typed arrays.
   *
   * # Arguments
   *
   * * `ids` - A JavaScript Array or Float64Array of vector IDs
   *
   * # Returns
   *
   * Same as `softDeleteBatch()` - see that method for details.
   *
   * # Errors
   *
   * Returns an error if the batch size exceeds the maximum (10M IDs) or if
   * any ID exceeds Number.MAX_SAFE_INTEGER.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * // Safari 14 compatible
   * const ids = [1, 3, 5, 7, 9, 11];
   * const result = index.softDeleteBatchCompat(ids);
   * console.log(`Deleted: ${result.deleted}`);
   * ```
   */
  softDeleteBatchCompat(ids: Float64Array): WasmBatchDeleteResult;
  /**
   * Batch insert with progress callback (W14.1).
   *
   * Inserts multiple vectors while reporting progress to a JavaScript callback.
   * The callback is invoked at the **start (0%)** and **end (100%)** of the batch
   * insertion. Intermediate progress during insertion is not currently reported.
   *
   * # Arguments
   *
   * * `vectors` - JS Array of Float32Array vectors to insert
   * * `on_progress` - JS function called with (inserted: number, total: number)
   *
   * # Returns
   *
   * `BatchInsertResult` containing inserted count, total, and IDs.
   *
   * # Performance Note
   *
   * See [`Self::insert_batch_v2`] for performance characteristics. Batch insert optimizes
   * JS↔WASM boundary overhead (1.2-1.5x at small scales), but converges with
   * sequential insertion at larger scales as HNSW graph construction dominates.
   *
   * # Callback Behavior
   *
   * - The callback is called exactly **twice**: once with `(0, total)` before
   *   insertion begins, and once with `(total, total)` after completion.
   * - **Errors in the callback are intentionally ignored** — the batch insert
   *   will succeed even if the progress callback throws an exception. This
   *   ensures that UI errors don't break data operations.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const result = index.insertBatchWithProgress(vectors, (done, total) => {
   *     console.log(`Progress: ${Math.round(done/total*100)}%`);
   * });
   * console.log(`Inserted ${result.inserted} vectors`);
   * ```
   *
   * # Errors
   *
   * Returns a JS error object with `code` property on failure.
   * Note: Callback exceptions do NOT cause this function to return an error.
   */
  insertBatchWithProgress(vectors: Array<any>, on_progress: Function): BatchInsertResult;
  /**
   * Creates a new EdgeVec database.
   *
   * # Errors
   *
   * Returns an error if the configuration is invalid (e.g., unknown metric).
   */
  constructor(config: EdgeVecConfig);
  /**
   * Check if binary quantization is enabled on this index.
   *
   * # Returns
   *
   * `true` if BQ is enabled and ready for use, `false` otherwise.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * if (index.hasBQ()) {
   *     const results = index.searchBQ(query, 10);
   * } else {
   *     const results = index.search(query, 10);
   * }
   * ```
   */
  hasBQ(): boolean;
  /**
   * Inserts a vector into the index.
   *
   * # Arguments
   *
   * * `vector` - A Float32Array containing the vector data.
   *
   * # Returns
   *
   * The assigned Vector ID (u32).
   *
   * # Errors
   *
   * Returns error if dimensions mismatch, vector contains NaNs, or ID overflows.
   */
  insert(vector: Float32Array): number;
  /**
   * Searches for nearest neighbors.
   *
   * # Arguments
   *
   * * `query` - The query vector.
   * * `k` - The number of neighbors to return.
   *
   * # Returns
   *
   * An array of objects: `[{ id: u32, score: f32 }, ...]`.
   *
   * # Errors
   *
   * Returns error if dimensions mismatch or vector contains NaNs.
   */
  search(query: Float32Array, k: number): any;
  /**
   * Compact the index by rebuilding without tombstones.
   *
   * This operation:
   * 1. Creates a new index with only live vectors
   * 2. Re-inserts vectors preserving IDs
   * 3. Replaces the current index
   *
   * **WARNING:** This is a blocking operation. For indices with >10k vectors,
   * consider running during idle time or warning the user about potential delays.
   *
   * # Returns
   *
   * A `CompactionResult` object containing:
   * * `tombstonesRemoved` - Number of deleted vectors removed
   * * `newSize` - Size of the index after compaction
   * * `durationMs` - Time taken in milliseconds
   *
   * # Errors
   *
   * Returns an error if compaction fails (e.g., memory allocation error).
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * if (index.needsCompaction()) {
   *     const result = index.compact();
   *     console.log(`Removed ${result.tombstonesRemoved} tombstones`);
   *     console.log(`New size: ${result.newSize}`);
   *     console.log(`Took ${result.durationMs}ms`);
   * }
   * ```
   */
  compact(): WasmCompactionResult;
  /**
   * Search using binary quantization (fast, approximate).
   *
   * Binary quantization converts vectors to bit arrays (1 bit per dimension)
   * and uses Hamming distance for comparison. This provides:
   * - ~32x memory reduction
   * - ~3-5x faster search
   * - ~70-85% recall (use `searchBQRescored` for higher recall)
   *
   * # Arguments
   *
   * * `query` - A Float32Array containing the query vector
   * * `k` - Number of results to return
   *
   * # Returns
   *
   * An array of search result objects: `[{ id: number, distance: number }, ...]`
   * where distance is a similarity score (higher is more similar).
   *
   * # Errors
   *
   * Returns error if:
   * - Binary quantization is not enabled on this index
   * - Query dimensions mismatch
   * - k is 0
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * // Fast search, lower recall
   * const results = index.searchBQ(new Float32Array([0.1, 0.2, ...]), 10);
   * for (const r of results) {
   *     console.log(`ID: ${r.id}, Similarity: ${r.distance}`);
   * }
   * ```
   */
  searchBQ(query: Float32Array, k: number): any;
}

export class EdgeVecConfig {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new configuration with required dimensions.
   */
  constructor(dimensions: number);
  /**
   * Vector dimensionality.
   */
  dimensions: number;
  /**
   * Set distance metric ("l2", "cosine", "dot").
   */
  set metric(value: string);
  /**
   * Set ef_search parameter.
   */
  set ef_search(value: number);
  /**
   * Set ef_construction parameter.
   */
  set ef_construction(value: number);
  /**
   * Set M parameter (max connections per node in layers > 0).
   */
  set m(value: number);
  /**
   * Set M0 parameter (max connections per node in layer 0).
   */
  set m0(value: number);
}

export class JsMetadataValue {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Gets the value as a boolean.
   *
   * @returns The boolean value, or undefined if not a boolean
   */
  asBoolean(): boolean | undefined;
  /**
   * Gets the value as an integer.
   *
   * Note: Returns as f64 for JavaScript compatibility. Safe for integers up to ±2^53.
   *
   * @returns The integer value as a number, or undefined if not an integer
   */
  asInteger(): number | undefined;
  /**
   * Creates a float metadata value.
   *
   * @param value - The float value (must not be NaN or Infinity)
   * @returns A new JsMetadataValue containing a float
   */
  static fromFloat(value: number): JsMetadataValue;
  /**
   * Checks if this value is a boolean.
   */
  isBoolean(): boolean;
  /**
   * Checks if this value is an integer.
   */
  isInteger(): boolean;
  /**
   * Creates a string metadata value.
   *
   * @param value - The string value
   * @returns A new JsMetadataValue containing a string
   */
  static fromString(value: string): JsMetadataValue;
  /**
   * Creates a boolean metadata value.
   *
   * @param value - The boolean value
   * @returns A new JsMetadataValue containing a boolean
   */
  static fromBoolean(value: boolean): JsMetadataValue;
  /**
   * Creates an integer metadata value.
   *
   * JavaScript numbers are always f64, so this method validates the input
   * to ensure it's a valid integer within JavaScript's safe integer range.
   *
   * @param value - The integer value (must be within ±(2^53 - 1))
   * @returns A new JsMetadataValue containing an integer
   * @throws {Error} If value is outside safe integer range or has fractional part
   *
   * # Errors
   *
   * Returns an error if:
   * - Value exceeds JavaScript's safe integer range (±9007199254740991)
   * - Value has a fractional part (e.g., 3.14)
   * - Value is NaN or Infinity
   */
  static fromInteger(value: number): JsMetadataValue;
  /**
   * Gets the value as a string array.
   *
   * @returns The string array, or undefined if not a string array
   */
  asStringArray(): any;
  /**
   * Checks if this value is a string array.
   */
  isStringArray(): boolean;
  /**
   * Creates a string array metadata value.
   *
   * @param value - An array of strings
   * @returns A new JsMetadataValue containing a string array
   *
   * # Errors
   *
   * Returns an error if any array element is not a string.
   */
  static fromStringArray(value: Array<any>): JsMetadataValue;
  /**
   * Converts to a JavaScript-native value.
   *
   * Returns:
   * - `string` for String values
   * - `number` for Integer and Float values
   * - `boolean` for Boolean values
   * - `string[]` for StringArray values
   *
   * @returns The JavaScript-native value
   */
  toJS(): any;
  /**
   * Gets the value as a float.
   *
   * @returns The float value, or undefined if not a float
   */
  asFloat(): number | undefined;
  /**
   * Returns the type of this value.
   *
   * @returns One of: 'string', 'integer', 'float', 'boolean', 'string_array'
   */
  getType(): string;
  /**
   * Checks if this value is a float.
   */
  isFloat(): boolean;
  /**
   * Gets the value as a string.
   *
   * @returns The string value, or undefined if not a string
   */
  asString(): string | undefined;
  /**
   * Checks if this value is a string.
   */
  isString(): boolean;
}

export class PersistenceIterator {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Returns the next chunk of data.
   *
   * # Returns
   *
   * * `Some(Uint8Array)` - The next chunk of data.
   * * `None` - If iteration is complete.
   *
   * # Panics
   *
   * Panics if the parent `EdgeVec` instance has been freed.
   */
  next_chunk(): Uint8Array | undefined;
}

export class WasmBatchDeleteResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Check if any deletions occurred in this operation.
   *
   * Returns `true` if at least one vector was newly deleted.
   */
  anyDeleted(): boolean;
  /**
   * Check if all operations succeeded (no invalid IDs).
   *
   * Returns `true` if every ID was valid (either deleted or already deleted).
   */
  allValid(): boolean;
  /**
   * Number of invalid IDs (not found in the index).
   */
  readonly invalidIds: number;
  /**
   * Number of unique vector IDs after deduplication.
   */
  readonly uniqueCount: number;
  /**
   * Number of vectors that were already deleted (tombstoned).
   */
  readonly alreadyDeleted: number;
  /**
   * Total number of vector IDs provided in the input (including duplicates).
   */
  readonly total: number;
  /**
   * Number of vectors successfully deleted in this operation.
   */
  readonly deleted: number;
}

export class WasmCompactionResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Number of tombstones (deleted vectors) removed during compaction.
   */
  readonly tombstones_removed: number;
  /**
   * New index size after compaction (live vectors only).
   */
  readonly new_size: number;
  /**
   * Time taken for the compaction operation in milliseconds.
   */
  readonly duration_ms: number;
}

/**
 * Get filter information (complexity, fields, operators).
 *
 * # Arguments
 *
 * * `filter_str` - Filter expression to analyze
 *
 * # Returns
 *
 * JSON string with filter info:
 * ```json
 * {
 *   "nodeCount": 5,
 *   "depth": 3,
 *   "fields": ["category", "price"],
 *   "operators": ["eq", "lt", "and"],
 *   "complexity": 3
 * }
 * ```
 *
 * # Errors
 *
 * Returns error if filter parsing fails.
 */
export function get_filter_info_js(filter_str: string): string;

/**
 * Initialize logging hooks.
 */
export function init_logging(): void;

/**
 * Parse a filter expression string into a compiled filter.
 *
 * # Arguments
 *
 * * `filter_str` - Filter expression in EdgeVec syntax
 *
 * # Returns
 *
 * JSON string representation of the parsed filter AST.
 *
 * # Errors
 *
 * Returns a JsValue error with structured JSON containing:
 * - `code`: Error code (e.g., "E001")
 * - `message`: Human-readable error message
 * - `position`: Position information (if available)
 * - `suggestion`: Fix suggestion (if available)
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * try {
 *     const filterJson = parse_filter_js('category = "gpu" AND price < 500');
 *     console.log(JSON.parse(filterJson));
 * } catch (e) {
 *     console.error('Parse error:', JSON.parse(e).message);
 * }
 * ```
 */
export function parse_filter_js(filter_str: string): string;

/**
 * Try to parse a filter string, returning null on error.
 *
 * # Arguments
 *
 * * `filter_str` - Filter expression to parse
 *
 * # Returns
 *
 * JSON string of parsed filter, or null if invalid.
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const filter = try_parse_filter_js(userInput);
 * if (filter !== null) {
 *     // Valid filter
 * }
 * ```
 */
export function try_parse_filter_js(filter_str: string): any;

/**
 * Validate a filter string without fully returning the AST.
 *
 * # Arguments
 *
 * * `filter_str` - Filter expression to validate
 *
 * # Returns
 *
 * JSON string with validation result:
 * ```json
 * {
 *   "valid": true,
 *   "errors": [],
 *   "warnings": []
 * }
 * ```
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const result = JSON.parse(validate_filter_js('price <'));
 * if (!result.valid) {
 *     console.log('Errors:', result.errors);
 * }
 * ```
 */
export function validate_filter_js(filter_str: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_batchinsertconfig_free: (a: number, b: number) => void;
  readonly __wbg_batchinsertresult_free: (a: number, b: number) => void;
  readonly __wbg_edgevec_free: (a: number, b: number) => void;
  readonly __wbg_edgevecconfig_free: (a: number, b: number) => void;
  readonly __wbg_get_edgevecconfig_dimensions: (a: number) => number;
  readonly __wbg_get_wasmcompactionresult_duration_ms: (a: number) => number;
  readonly __wbg_get_wasmcompactionresult_new_size: (a: number) => number;
  readonly __wbg_get_wasmcompactionresult_tombstones_removed: (a: number) => number;
  readonly __wbg_jsmetadatavalue_free: (a: number, b: number) => void;
  readonly __wbg_persistenceiterator_free: (a: number, b: number) => void;
  readonly __wbg_set_edgevecconfig_dimensions: (a: number, b: number) => void;
  readonly __wbg_wasmbatchdeleteresult_free: (a: number, b: number) => void;
  readonly __wbg_wasmcompactionresult_free: (a: number, b: number) => void;
  readonly batchinsertconfig_new: () => number;
  readonly batchinsertconfig_set_validateDimensions: (a: number, b: number) => void;
  readonly batchinsertconfig_validateDimensions: (a: number) => number;
  readonly batchinsertresult_ids: (a: number, b: number) => void;
  readonly batchinsertresult_inserted: (a: number) => number;
  readonly batchinsertresult_total: (a: number) => number;
  readonly edgevec_compact: (a: number, b: number) => void;
  readonly edgevec_compactionThreshold: (a: number) => number;
  readonly edgevec_compactionWarning: (a: number, b: number) => void;
  readonly edgevec_deleteAllMetadata: (a: number, b: number) => number;
  readonly edgevec_deleteMetadata: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly edgevec_deletedCount: (a: number) => number;
  readonly edgevec_getAllMetadata: (a: number, b: number) => number;
  readonly edgevec_getMetadata: (a: number, b: number, c: number, d: number) => number;
  readonly edgevec_hasBQ: (a: number) => number;
  readonly edgevec_hasMetadata: (a: number, b: number, c: number, d: number) => number;
  readonly edgevec_insert: (a: number, b: number, c: number) => void;
  readonly edgevec_insertBatch: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_insertBatchFlat: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_insertBatchWithProgress: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_insertWithMetadata: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_isDeleted: (a: number, b: number, c: number) => void;
  readonly edgevec_liveCount: (a: number) => number;
  readonly edgevec_load: (a: number, b: number) => number;
  readonly edgevec_metadataKeyCount: (a: number, b: number) => number;
  readonly edgevec_metadataVectorCount: (a: number) => number;
  readonly edgevec_needsCompaction: (a: number) => number;
  readonly edgevec_new: (a: number, b: number) => void;
  readonly edgevec_save: (a: number, b: number, c: number) => number;
  readonly edgevec_save_stream: (a: number, b: number) => number;
  readonly edgevec_search: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_searchBQ: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_searchBQRescored: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly edgevec_searchFiltered: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly edgevec_searchHybrid: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_searchWithFilter: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly edgevec_setCompactionThreshold: (a: number, b: number) => void;
  readonly edgevec_setMetadata: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly edgevec_softDelete: (a: number, b: number, c: number) => void;
  readonly edgevec_softDeleteBatch: (a: number, b: number, c: number) => void;
  readonly edgevec_softDeleteBatchCompat: (a: number, b: number, c: number) => void;
  readonly edgevec_tombstoneRatio: (a: number) => number;
  readonly edgevec_totalMetadataCount: (a: number) => number;
  readonly edgevecconfig_new: (a: number) => number;
  readonly edgevecconfig_set_ef_construction: (a: number, b: number) => void;
  readonly edgevecconfig_set_ef_search: (a: number, b: number) => void;
  readonly edgevecconfig_set_m: (a: number, b: number) => void;
  readonly edgevecconfig_set_m0: (a: number, b: number) => void;
  readonly edgevecconfig_set_metric: (a: number, b: number, c: number) => void;
  readonly get_filter_info_js: (a: number, b: number, c: number) => void;
  readonly init_logging: () => void;
  readonly jsmetadatavalue_asBoolean: (a: number) => number;
  readonly jsmetadatavalue_asFloat: (a: number, b: number) => void;
  readonly jsmetadatavalue_asInteger: (a: number, b: number) => void;
  readonly jsmetadatavalue_asString: (a: number, b: number) => void;
  readonly jsmetadatavalue_asStringArray: (a: number) => number;
  readonly jsmetadatavalue_fromBoolean: (a: number) => number;
  readonly jsmetadatavalue_fromFloat: (a: number) => number;
  readonly jsmetadatavalue_fromInteger: (a: number, b: number) => void;
  readonly jsmetadatavalue_fromString: (a: number, b: number) => number;
  readonly jsmetadatavalue_fromStringArray: (a: number, b: number) => void;
  readonly jsmetadatavalue_getType: (a: number, b: number) => void;
  readonly jsmetadatavalue_isBoolean: (a: number) => number;
  readonly jsmetadatavalue_isFloat: (a: number) => number;
  readonly jsmetadatavalue_isInteger: (a: number) => number;
  readonly jsmetadatavalue_isString: (a: number) => number;
  readonly jsmetadatavalue_isStringArray: (a: number) => number;
  readonly jsmetadatavalue_toJS: (a: number) => number;
  readonly parse_filter_js: (a: number, b: number, c: number) => void;
  readonly persistenceiterator_next_chunk: (a: number) => number;
  readonly try_parse_filter_js: (a: number, b: number) => number;
  readonly validate_filter_js: (a: number, b: number, c: number) => void;
  readonly wasmbatchdeleteresult_allValid: (a: number) => number;
  readonly wasmbatchdeleteresult_alreadyDeleted: (a: number) => number;
  readonly wasmbatchdeleteresult_anyDeleted: (a: number) => number;
  readonly wasmbatchdeleteresult_deleted: (a: number) => number;
  readonly wasmbatchdeleteresult_invalidIds: (a: number) => number;
  readonly wasmbatchdeleteresult_total: (a: number) => number;
  readonly wasmbatchdeleteresult_uniqueCount: (a: number) => number;
  readonly edgevec_getVectorMetadata: (a: number, b: number) => number;
  readonly __wasm_bindgen_func_elem_1608: (a: number, b: number, c: number) => void;
  readonly __wasm_bindgen_func_elem_1592: (a: number, b: number) => void;
  readonly __wasm_bindgen_func_elem_2123: (a: number, b: number, c: number, d: number) => void;
  readonly __wbindgen_export: (a: number, b: number) => number;
  readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export3: (a: number) => void;
  readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
