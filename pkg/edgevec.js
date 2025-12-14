import { IndexedDbBackend } from './snippets/edgevec-3900ce8b80cc807b/src/js/storage.js';

let wasm;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => state.dtor(state.a, state.b));

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getBigUint64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedBigUint64ArrayMemory0 = null;
function getBigUint64ArrayMemory0() {
    if (cachedBigUint64ArrayMemory0 === null || cachedBigUint64ArrayMemory0.byteLength === 0) {
        cachedBigUint64ArrayMemory0 = new BigUint64Array(wasm.memory.buffer);
    }
    return cachedBigUint64ArrayMemory0;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export(addHeapObject(e));
    }
}

let heap = new Array(128).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    }
}

let WASM_VECTOR_LEN = 0;

function __wasm_bindgen_func_elem_270(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_270(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_470(arg0, arg1, arg2, arg3) {
    wasm.__wasm_bindgen_func_elem_470(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const BatchInsertConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_batchinsertconfig_free(ptr >>> 0, 1));

const BatchInsertResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_batchinsertresult_free(ptr >>> 0, 1));

const EdgeVecFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_edgevec_free(ptr >>> 0, 1));

const EdgeVecConfigFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_edgevecconfig_free(ptr >>> 0, 1));

const PersistenceIteratorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_persistenceiterator_free(ptr >>> 0, 1));

/**
 * Configuration options for batch insert operations (WASM).
 *
 * This struct mirrors the TypeScript `BatchInsertConfig` interface
 * defined in `wasm/batch_types.ts`.
 */
export class BatchInsertConfig {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BatchInsertConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_batchinsertconfig_free(ptr, 0);
    }
    /**
     * Returns whether dimension validation is enabled.
     * @returns {boolean}
     */
    get validateDimensions() {
        const ret = wasm.batchinsertconfig_validateDimensions(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Sets whether to validate vector dimensions before insertion.
     * @param {boolean} value
     */
    set validateDimensions(value) {
        wasm.batchinsertconfig_set_validateDimensions(this.__wbg_ptr, value);
    }
    /**
     * Creates a new `BatchInsertConfig` with default settings.
     *
     * Default: `validate_dimensions = true`
     */
    constructor() {
        const ret = wasm.batchinsertconfig_new();
        this.__wbg_ptr = ret >>> 0;
        BatchInsertConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) BatchInsertConfig.prototype[Symbol.dispose] = BatchInsertConfig.prototype.free;

/**
 * Result of a batch insert operation (WASM).
 *
 * This struct mirrors the TypeScript `BatchInsertResult` interface
 * defined in `wasm/batch_types.ts`.
 */
export class BatchInsertResult {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(BatchInsertResult.prototype);
        obj.__wbg_ptr = ptr;
        BatchInsertResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BatchInsertResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_batchinsertresult_free(ptr, 0);
    }
    /**
     * Returns a copy of the IDs of successfully inserted vectors.
     * @returns {BigUint64Array}
     */
    get ids() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.batchinsertresult_ids(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var v1 = getArrayU64FromWasm0(r0, r1).slice();
            wasm.__wbindgen_export2(r0, r1 * 8, 8);
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Returns the total number of vectors attempted (input array length).
     * @returns {number}
     */
    get total() {
        const ret = wasm.batchinsertresult_total(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Returns the number of vectors successfully inserted.
     * @returns {number}
     */
    get inserted() {
        const ret = wasm.batchinsertresult_inserted(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) BatchInsertResult.prototype[Symbol.dispose] = BatchInsertResult.prototype.free;

/**
 * The main EdgeVec database handle.
 *
 * This struct is serializable for persistence via `postcard`.
 * The `liveness` field is skipped as it is runtime state.
 *
 * # Safety Note
 *
 * This type derives `Deserialize` despite containing methods with `unsafe`.
 * The unsafe code (`save_stream`) is unrelated to deserialization and is safe
 * because it only extends lifetimes for iterator borrowing, controlled by the
 * `liveness` guard.
 */
export class EdgeVec {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(EdgeVec.prototype);
        obj.__wbg_ptr = ptr;
        EdgeVecFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        EdgeVecFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_edgevec_free(ptr, 0);
    }
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
     * @param {string} name
     * @returns {Promise<EdgeVec>}
     */
    static load(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.edgevec_load(ptr0, len0);
        return takeObject(ret);
    }
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
     * @param {string} name
     * @returns {Promise<void>}
     */
    save(name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.edgevec_save(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
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
     * @param {number | null} [chunk_size]
     * @returns {PersistenceIterator}
     */
    save_stream(chunk_size) {
        const ret = wasm.edgevec_save_stream(this.__wbg_ptr, isLikeNone(chunk_size) ? 0x100000001 : (chunk_size) >>> 0);
        return PersistenceIterator.__wrap(ret);
    }
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
     * @param {Array<any>} vectors
     * @param {BatchInsertConfig | null} [config]
     * @returns {BatchInsertResult}
     */
    insertBatch(vectors, config) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            let ptr0 = 0;
            if (!isLikeNone(config)) {
                _assertClass(config, BatchInsertConfig);
                ptr0 = config.__destroy_into_raw();
            }
            wasm.edgevec_insertBatch(retptr, this.__wbg_ptr, addHeapObject(vectors), ptr0);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return BatchInsertResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {Float32Array} vectors
     * @param {number} count
     * @returns {Uint32Array}
     */
    insertBatchFlat(vectors, count) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_insertBatchFlat(retptr, this.__wbg_ptr, addHeapObject(vectors), count);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Creates a new EdgeVec database.
     *
     * # Errors
     *
     * Returns an error if the configuration is invalid (e.g., unknown metric).
     * @param {EdgeVecConfig} config
     */
    constructor(config) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(config, EdgeVecConfig);
            wasm.edgevec_new(retptr, config.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            this.__wbg_ptr = r0 >>> 0;
            EdgeVecFinalization.register(this, this.__wbg_ptr, this);
            return this;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {Float32Array} vector
     * @returns {number}
     */
    insert(vector) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_insert(retptr, this.__wbg_ptr, addHeapObject(vector));
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return r0 >>> 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {Float32Array} query
     * @param {number} k
     * @returns {any}
     */
    search(query, k) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_search(retptr, this.__wbg_ptr, addHeapObject(query), k);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
if (Symbol.dispose) EdgeVec.prototype[Symbol.dispose] = EdgeVec.prototype.free;

/**
 * Configuration for EdgeVec database.
 */
export class EdgeVecConfig {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        EdgeVecConfigFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_edgevecconfig_free(ptr, 0);
    }
    /**
     * Vector dimensionality.
     * @returns {number}
     */
    get dimensions() {
        const ret = wasm.__wbg_get_edgevecconfig_dimensions(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Vector dimensionality.
     * @param {number} arg0
     */
    set dimensions(arg0) {
        wasm.__wbg_set_edgevecconfig_dimensions(this.__wbg_ptr, arg0);
    }
    /**
     * Set distance metric ("l2", "cosine", "dot").
     * @param {string} metric
     */
    set metric(metric) {
        const ptr0 = passStringToWasm0(metric, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
        const len0 = WASM_VECTOR_LEN;
        wasm.edgevecconfig_set_metric(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Set ef_search parameter.
     * @param {number} ef
     */
    set ef_search(ef) {
        wasm.edgevecconfig_set_ef_search(this.__wbg_ptr, ef);
    }
    /**
     * Set ef_construction parameter.
     * @param {number} ef
     */
    set ef_construction(ef) {
        wasm.edgevecconfig_set_ef_construction(this.__wbg_ptr, ef);
    }
    /**
     * Create a new configuration with required dimensions.
     * @param {number} dimensions
     */
    constructor(dimensions) {
        const ret = wasm.edgevecconfig_new(dimensions);
        this.__wbg_ptr = ret >>> 0;
        EdgeVecConfigFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Set M parameter (max connections per node in layers > 0).
     * @param {number} m
     */
    set m(m) {
        wasm.edgevecconfig_set_m(this.__wbg_ptr, m);
    }
    /**
     * Set M0 parameter (max connections per node in layer 0).
     * @param {number} m0
     */
    set m0(m0) {
        wasm.edgevecconfig_set_m0(this.__wbg_ptr, m0);
    }
}
if (Symbol.dispose) EdgeVecConfig.prototype[Symbol.dispose] = EdgeVecConfig.prototype.free;

/**
 * Iterator for saving the database in chunks.
 *
 * This avoids large allocations by serializing the database incrementally.
 *
 * # Safety Warning
 *
 * **WARNING:** This iterator holds a reference to the `EdgeVec` instance (via `unsafe` transmutation).
 *
 * - You **MUST NOT** call `free()` on the `EdgeVec` instance while this iterator is in use.
 * - If `EdgeVec` is garbage collected or explicitly freed, usage of this iterator will panic
 *   to prevent Use-After-Free (UAF) vulnerabilities.
 * - Ensure the `EdgeVec` instance remains in scope in JavaScript for the duration of the iteration.
 */
export class PersistenceIterator {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(PersistenceIterator.prototype);
        obj.__wbg_ptr = ptr;
        PersistenceIteratorFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        PersistenceIteratorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_persistenceiterator_free(ptr, 0);
    }
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
     * @returns {Uint8Array | undefined}
     */
    next_chunk() {
        const ret = wasm.persistenceiterator_next_chunk(this.__wbg_ptr);
        return takeObject(ret);
    }
}
if (Symbol.dispose) PersistenceIterator.prototype[Symbol.dispose] = PersistenceIterator.prototype.free;

/**
 * Initialize logging hooks.
 */
export function init_logging() {
    wasm.init_logging();
}

const EXPECTED_RESPONSE_TYPES = new Set(['basic', 'cors', 'default']);

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && EXPECTED_RESPONSE_TYPES.has(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg___wbindgen_is_function_8d400b8b1af978cd = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbg___wbindgen_is_undefined_f6b95eab589e0269 = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbg___wbindgen_throw_dd24417ed36fc46e = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg__wbg_cb_unref_87dfb5aaa0cbcea7 = function(arg0) {
        getObject(arg0)._wbg_cb_unref();
    };
    imports.wbg.__wbg_call_3020136f7a2d6e44 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_abb4ff46ce38be40 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_debug_9d0c87ddda3dc485 = function(arg0) {
        console.debug(getObject(arg0));
    };
    imports.wbg.__wbg_edgevec_new = function(arg0) {
        const ret = EdgeVec.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_error_7534b8e9a36f1ab4 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_export2(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_error_7bc7d576a6aaf855 = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_get_6b7bd52aca3f9671 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_info_ce6bcc489c22f6f0 = function(arg0) {
        console.info(getObject(arg0));
    };
    imports.wbg.__wbg_length_22ac23eaec9d8053 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_86ce4877baf913bb = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_d45040a40c570362 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_log_1d990106d99dacb7 = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_new_1ba21ce319a06297 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_6421f6084cc5bc5a = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_8a6f238a6ece86ea = function() {
        const ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_ff12d2b041fb48f1 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wasm_bindgen_func_elem_470(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return addHeapObject(ret);
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_from_slice_db0691b69e9d3891 = function(arg0, arg1) {
        const ret = new Uint32Array(getArrayU32FromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_from_slice_f9c22b9153b26992 = function(arg0, arg1) {
        const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_no_args_cb138f77cf6151ee = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_with_length_12c6de4fac33117a = function(arg0) {
        const ret = new Array(arg0 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_prototypesetcall_96cc7097487b926d = function(arg0, arg1, arg2) {
        Float32Array.prototype.set.call(getArrayF32FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_prototypesetcall_dfe9b766cdc1f1fd = function(arg0, arg1, arg2) {
        Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_queueMicrotask_9b549dfce8865860 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_fca69f5bfad613a5 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_read_4016394bb14db0bb = function() { return handleError(function (arg0, arg1) {
        const ret = IndexedDbBackend.read(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_resolve_fd5bfbaa4ce36e1e = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_781438a03c0c3c81 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_7df433eea03a5c14 = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_stack_0ed75d68575b0f3c = function(arg0, arg1) {
        const ret = getObject(arg1).stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export3, wasm.__wbindgen_export4);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_769e6b65d6557335 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_60cf02db4de8e1c1 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_08f5a74c69739274 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_a8924b26aa92d024 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_then_429f7caf1026411d = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_4f95312d68691235 = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_warn_6e567d0d926ff881 = function(arg0) {
        console.warn(getObject(arg0));
    };
    imports.wbg.__wbg_write_5dbdffa9542cb272 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = IndexedDbBackend.write(getStringFromWasm0(arg0, arg1), getArrayU8FromWasm0(arg2, arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(String) -> Externref`.
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cast_8eb6fd44e7238d11 = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 62, function: Function { arguments: [Externref], shim_idx: 63, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.__wasm_bindgen_func_elem_263, __wasm_bindgen_func_elem_270);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cast_d6cd19b81560fd6e = function(arg0) {
        // Cast intrinsic for `F64 -> Externref`.
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };

    return imports;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedBigUint64ArrayMemory0 = null;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('edgevec_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
