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
   * Creates a new EdgeVec database.
   *
   * # Errors
   *
   * Returns an error if the configuration is invalid (e.g., unknown metric).
   */
  constructor(config: EdgeVecConfig);
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

/**
 * Initialize logging hooks.
 */
export function init_logging(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_batchinsertconfig_free: (a: number, b: number) => void;
  readonly __wbg_batchinsertresult_free: (a: number, b: number) => void;
  readonly __wbg_edgevec_free: (a: number, b: number) => void;
  readonly __wbg_edgevecconfig_free: (a: number, b: number) => void;
  readonly __wbg_get_edgevecconfig_dimensions: (a: number) => number;
  readonly __wbg_persistenceiterator_free: (a: number, b: number) => void;
  readonly __wbg_set_edgevecconfig_dimensions: (a: number, b: number) => void;
  readonly batchinsertconfig_new: () => number;
  readonly batchinsertconfig_set_validateDimensions: (a: number, b: number) => void;
  readonly batchinsertconfig_validateDimensions: (a: number) => number;
  readonly batchinsertresult_ids: (a: number, b: number) => void;
  readonly batchinsertresult_inserted: (a: number) => number;
  readonly batchinsertresult_total: (a: number) => number;
  readonly edgevec_insert: (a: number, b: number, c: number) => void;
  readonly edgevec_insertBatch: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_insertBatchFlat: (a: number, b: number, c: number, d: number) => void;
  readonly edgevec_load: (a: number, b: number) => number;
  readonly edgevec_new: (a: number, b: number) => void;
  readonly edgevec_save: (a: number, b: number, c: number) => number;
  readonly edgevec_save_stream: (a: number, b: number) => number;
  readonly edgevec_search: (a: number, b: number, c: number, d: number) => void;
  readonly edgevecconfig_new: (a: number) => number;
  readonly edgevecconfig_set_ef_construction: (a: number, b: number) => void;
  readonly edgevecconfig_set_ef_search: (a: number, b: number) => void;
  readonly edgevecconfig_set_m: (a: number, b: number) => void;
  readonly edgevecconfig_set_m0: (a: number, b: number) => void;
  readonly edgevecconfig_set_metric: (a: number, b: number, c: number) => void;
  readonly persistenceiterator_next_chunk: (a: number) => number;
  readonly init_logging: () => void;
  readonly __wasm_bindgen_func_elem_270: (a: number, b: number, c: number) => void;
  readonly __wasm_bindgen_func_elem_263: (a: number, b: number) => void;
  readonly __wasm_bindgen_func_elem_470: (a: number, b: number, c: number, d: number) => void;
  readonly __wbindgen_export: (a: number) => void;
  readonly __wbindgen_export2: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export3: (a: number, b: number) => number;
  readonly __wbindgen_export4: (a: number, b: number, c: number, d: number) => number;
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
