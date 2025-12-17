import { IndexedDbBackend } from './snippets/edgevec-e9d90602e6bb6c2c/src/js/storage.js';

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

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
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

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
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
        wasm.__wbindgen_export3(addHeapObject(e));
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

function __wasm_bindgen_func_elem_401(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_401(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_596(arg0, arg1, arg2, arg3) {
    wasm.__wasm_bindgen_func_elem_596(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
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

const JsMetadataValueFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsmetadatavalue_free(ptr >>> 0, 1));

const PersistenceIteratorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_persistenceiterator_free(ptr >>> 0, 1));

const WasmBatchDeleteResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmbatchdeleteresult_free(ptr >>> 0, 1));

const WasmCompactionResultFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcompactionresult_free(ptr >>> 0, 1));

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
            wasm.__wbindgen_export4(r0, r1 * 8, 8);
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
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_export, wasm.__wbindgen_export2);
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
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.edgevec_save(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
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
     * @param {number} vector_id
     * @returns {boolean}
     */
    isDeleted(vector_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_isDeleted(retptr, this.__wbg_ptr, vector_id);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Get the count of live (non-deleted) vectors.
     *
     * # Returns
     *
     * The number of vectors that are currently searchable.
     * @returns {number}
     */
    liveCount() {
        const ret = wasm.edgevec_liveCount(this.__wbg_ptr);
        return ret >>> 0;
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
     * @param {number} vector_id
     * @returns {boolean}
     */
    softDelete(vector_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_softDelete(retptr, this.__wbg_ptr, vector_id);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {number} vector_id
     * @param {string} key
     * @returns {JsMetadataValue | undefined}
     */
    getMetadata(vector_id, key) {
        const ptr0 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.edgevec_getMetadata(this.__wbg_ptr, vector_id, ptr0, len0);
        return ret === 0 ? undefined : JsMetadataValue.__wrap(ret);
    }
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
     * @param {number} vector_id
     * @param {string} key
     * @returns {boolean}
     */
    hasMetadata(vector_id, key) {
        const ptr0 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.edgevec_hasMetadata(this.__wbg_ptr, vector_id, ptr0, len0);
        return ret !== 0;
    }
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
     * @param {number} vector_id
     * @param {string} key
     * @param {JsMetadataValue} value
     */
    setMetadata(vector_id, key, value) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len0 = WASM_VECTOR_LEN;
            _assertClass(value, JsMetadataValue);
            wasm.edgevec_setMetadata(retptr, this.__wbg_ptr, vector_id, ptr0, len0, value.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Get the count of deleted (tombstoned) vectors.
     *
     * # Returns
     *
     * The number of vectors that have been soft-deleted but not yet compacted.
     * @returns {number}
     */
    deletedCount() {
        const ret = wasm.edgevec_deletedCount(this.__wbg_ptr);
        return ret >>> 0;
    }
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
     * @param {number} vector_id
     * @param {string} key
     * @returns {boolean}
     */
    deleteMetadata(vector_id, key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len0 = WASM_VECTOR_LEN;
            wasm.edgevec_deleteMetadata(retptr, this.__wbg_ptr, vector_id, ptr0, len0);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
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
     * Get the ratio of deleted to total vectors.
     *
     * # Returns
     *
     * A value between 0.0 and 1.0 representing the tombstone ratio.
     * 0.0 means no deletions, 1.0 means all vectors deleted.
     * @returns {number}
     */
    tombstoneRatio() {
        const ret = wasm.edgevec_tombstoneRatio(this.__wbg_ptr);
        return ret;
    }
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
     * @param {number} vector_id
     * @returns {any}
     */
    getAllMetadata(vector_id) {
        const ret = wasm.edgevec_getAllMetadata(this.__wbg_ptr, vector_id);
        return takeObject(ret);
    }
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
     * @returns {boolean}
     */
    needsCompaction() {
        const ret = wasm.edgevec_needsCompaction(this.__wbg_ptr);
        return ret !== 0;
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
     * @param {Uint32Array} ids
     * @returns {WasmBatchDeleteResult}
     */
    softDeleteBatch(ids) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_softDeleteBatch(retptr, this.__wbg_ptr, addHeapObject(ids));
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return WasmBatchDeleteResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @returns {string | undefined}
     */
    compactionWarning() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_compactionWarning(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            let v1;
            if (r0 !== 0) {
                v1 = getStringFromWasm0(r0, r1).slice();
                wasm.__wbindgen_export4(r0, r1 * 1, 1);
            }
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {number} vector_id
     * @returns {number}
     */
    metadataKeyCount(vector_id) {
        const ret = wasm.edgevec_metadataKeyCount(this.__wbg_ptr, vector_id);
        return ret >>> 0;
    }
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
     * @param {number} vector_id
     * @returns {boolean}
     */
    deleteAllMetadata(vector_id) {
        const ret = wasm.edgevec_deleteAllMetadata(this.__wbg_ptr, vector_id);
        return ret !== 0;
    }
    /**
     * Get the current compaction threshold.
     *
     * # Returns
     *
     * The threshold ratio (0.0 to 1.0) above which `needsCompaction()` returns true.
     * Default is 0.3 (30%).
     * @returns {number}
     */
    compactionThreshold() {
        const ret = wasm.edgevec_compactionThreshold(this.__wbg_ptr);
        return ret;
    }
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
     * @returns {number}
     */
    totalMetadataCount() {
        const ret = wasm.edgevec_totalMetadataCount(this.__wbg_ptr);
        return ret >>> 0;
    }
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
     * @returns {number}
     */
    metadataVectorCount() {
        const ret = wasm.edgevec_metadataVectorCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Set the compaction threshold.
     *
     * # Arguments
     *
     * * `ratio` - The new threshold (clamped to 0.01 - 0.99).
     * @param {number} ratio
     */
    setCompactionThreshold(ratio) {
        wasm.edgevec_setCompactionThreshold(this.__wbg_ptr, ratio);
    }
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
     * @param {Float64Array} ids
     * @returns {WasmBatchDeleteResult}
     */
    softDeleteBatchCompat(ids) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_softDeleteBatchCompat(retptr, this.__wbg_ptr, addHeapObject(ids));
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return WasmBatchDeleteResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @param {Array<any>} vectors
     * @param {Function} on_progress
     * @returns {BatchInsertResult}
     */
    insertBatchWithProgress(vectors, on_progress) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_insertBatchWithProgress(retptr, this.__wbg_ptr, addHeapObject(vectors), addHeapObject(on_progress));
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
     * @returns {WasmCompactionResult}
     */
    compact() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.edgevec_compact(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return WasmCompactionResult.__wrap(r0);
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
        const ptr0 = passStringToWasm0(metric, wasm.__wbindgen_export, wasm.__wbindgen_export2);
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
 * JavaScript-friendly metadata value representation.
 *
 * This type bridges Rust's `MetadataValue` enum to JavaScript objects.
 * Use the static factory methods (`fromString`, `fromInteger`, etc.) to create
 * values from JavaScript.
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const strValue = JsMetadataValue.fromString('hello');
 * const intValue = JsMetadataValue.fromInteger(42);
 * const floatValue = JsMetadataValue.fromFloat(3.14);
 * const boolValue = JsMetadataValue.fromBoolean(true);
 * const arrValue = JsMetadataValue.fromStringArray(['a', 'b', 'c']);
 *
 * console.log(strValue.getType()); // 'string'
 * console.log(intValue.toJS());    // 42
 * ```
 */
export class JsMetadataValue {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsMetadataValue.prototype);
        obj.__wbg_ptr = ptr;
        JsMetadataValueFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsMetadataValueFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsmetadatavalue_free(ptr, 0);
    }
    /**
     * Gets the value as a boolean.
     *
     * @returns The boolean value, or undefined if not a boolean
     * @returns {boolean | undefined}
     */
    asBoolean() {
        const ret = wasm.jsmetadatavalue_asBoolean(this.__wbg_ptr);
        return ret === 0xFFFFFF ? undefined : ret !== 0;
    }
    /**
     * Gets the value as an integer.
     *
     * Note: Returns as f64 for JavaScript compatibility. Safe for integers up to ±2^53.
     *
     * @returns The integer value as a number, or undefined if not an integer
     * @returns {number | undefined}
     */
    asInteger() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_asInteger(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r2 = getDataViewMemory0().getFloat64(retptr + 8 * 1, true);
            return r0 === 0 ? undefined : r2;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Creates a float metadata value.
     *
     * @param value - The float value (must not be NaN or Infinity)
     * @returns A new JsMetadataValue containing a float
     * @param {number} value
     * @returns {JsMetadataValue}
     */
    static fromFloat(value) {
        const ret = wasm.jsmetadatavalue_fromFloat(value);
        return JsMetadataValue.__wrap(ret);
    }
    /**
     * Checks if this value is a boolean.
     * @returns {boolean}
     */
    isBoolean() {
        const ret = wasm.jsmetadatavalue_isBoolean(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Checks if this value is an integer.
     * @returns {boolean}
     */
    isInteger() {
        const ret = wasm.jsmetadatavalue_isInteger(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Creates a string metadata value.
     *
     * @param value - The string value
     * @returns A new JsMetadataValue containing a string
     * @param {string} value
     * @returns {JsMetadataValue}
     */
    static fromString(value) {
        const ptr0 = passStringToWasm0(value, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jsmetadatavalue_fromString(ptr0, len0);
        return JsMetadataValue.__wrap(ret);
    }
    /**
     * Creates a boolean metadata value.
     *
     * @param value - The boolean value
     * @returns A new JsMetadataValue containing a boolean
     * @param {boolean} value
     * @returns {JsMetadataValue}
     */
    static fromBoolean(value) {
        const ret = wasm.jsmetadatavalue_fromBoolean(value);
        return JsMetadataValue.__wrap(ret);
    }
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
     * @param {number} value
     * @returns {JsMetadataValue}
     */
    static fromInteger(value) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_fromInteger(retptr, value);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return JsMetadataValue.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Gets the value as a string array.
     *
     * @returns The string array, or undefined if not a string array
     * @returns {any}
     */
    asStringArray() {
        const ret = wasm.jsmetadatavalue_asStringArray(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Checks if this value is a string array.
     * @returns {boolean}
     */
    isStringArray() {
        const ret = wasm.jsmetadatavalue_isStringArray(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Creates a string array metadata value.
     *
     * @param value - An array of strings
     * @returns A new JsMetadataValue containing a string array
     *
     * # Errors
     *
     * Returns an error if any array element is not a string.
     * @param {Array<any>} value
     * @returns {JsMetadataValue}
     */
    static fromStringArray(value) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_fromStringArray(retptr, addHeapObject(value));
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
            if (r2) {
                throw takeObject(r1);
            }
            return JsMetadataValue.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
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
     * @returns {any}
     */
    toJS() {
        const ret = wasm.jsmetadatavalue_toJS(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Gets the value as a float.
     *
     * @returns The float value, or undefined if not a float
     * @returns {number | undefined}
     */
    asFloat() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_asFloat(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r2 = getDataViewMemory0().getFloat64(retptr + 8 * 1, true);
            return r0 === 0 ? undefined : r2;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Returns the type of this value.
     *
     * @returns One of: 'string', 'integer', 'float', 'boolean', 'string_array'
     * @returns {string}
     */
    getType() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_getType(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export4(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Checks if this value is a float.
     * @returns {boolean}
     */
    isFloat() {
        const ret = wasm.jsmetadatavalue_isFloat(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Gets the value as a string.
     *
     * @returns The string value, or undefined if not a string
     * @returns {string | undefined}
     */
    asString() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsmetadatavalue_asString(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            let v1;
            if (r0 !== 0) {
                v1 = getStringFromWasm0(r0, r1).slice();
                wasm.__wbindgen_export4(r0, r1 * 1, 1);
            }
            return v1;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
     * Checks if this value is a string.
     * @returns {boolean}
     */
    isString() {
        const ret = wasm.jsmetadatavalue_isString(this.__wbg_ptr);
        return ret !== 0;
    }
}
if (Symbol.dispose) JsMetadataValue.prototype[Symbol.dispose] = JsMetadataValue.prototype.free;

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
 * Result of a batch delete operation (W18.4/W18.5).
 *
 * Returned by `EdgeVec.softDeleteBatch()` and `EdgeVec.softDeleteBatchCompat()`
 * to provide detailed metrics about the batch deletion.
 */
export class WasmBatchDeleteResult {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmBatchDeleteResult.prototype);
        obj.__wbg_ptr = ptr;
        WasmBatchDeleteResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmBatchDeleteResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmbatchdeleteresult_free(ptr, 0);
    }
    /**
     * Check if any deletions occurred in this operation.
     *
     * Returns `true` if at least one vector was newly deleted.
     * @returns {boolean}
     */
    anyDeleted() {
        const ret = wasm.wasmbatchdeleteresult_anyDeleted(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Number of invalid IDs (not found in the index).
     * @returns {number}
     */
    get invalidIds() {
        const ret = wasm.wasmbatchdeleteresult_invalidIds(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of unique vector IDs after deduplication.
     * @returns {number}
     */
    get uniqueCount() {
        const ret = wasm.batchinsertresult_total(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of vectors that were already deleted (tombstoned).
     * @returns {number}
     */
    get alreadyDeleted() {
        const ret = wasm.wasmbatchdeleteresult_alreadyDeleted(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Total number of vector IDs provided in the input (including duplicates).
     * @returns {number}
     */
    get total() {
        const ret = wasm.batchinsertresult_inserted(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of vectors successfully deleted in this operation.
     * @returns {number}
     */
    get deleted() {
        const ret = wasm.wasmbatchdeleteresult_deleted(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Check if all operations succeeded (no invalid IDs).
     *
     * Returns `true` if every ID was valid (either deleted or already deleted).
     * @returns {boolean}
     */
    allValid() {
        const ret = wasm.wasmbatchdeleteresult_allValid(this.__wbg_ptr);
        return ret !== 0;
    }
}
if (Symbol.dispose) WasmBatchDeleteResult.prototype[Symbol.dispose] = WasmBatchDeleteResult.prototype.free;

/**
 * Result of a compaction operation (v0.3.0).
 *
 * Returned by `EdgeVec.compact()` to provide metrics about the operation.
 */
export class WasmCompactionResult {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmCompactionResult.prototype);
        obj.__wbg_ptr = ptr;
        WasmCompactionResultFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmCompactionResultFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcompactionresult_free(ptr, 0);
    }
    /**
     * Number of tombstones (deleted vectors) removed during compaction.
     * @returns {number}
     */
    get tombstones_removed() {
        const ret = wasm.__wbg_get_wasmcompactionresult_tombstones_removed(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * New index size after compaction (live vectors only).
     * @returns {number}
     */
    get new_size() {
        const ret = wasm.__wbg_get_wasmcompactionresult_new_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Time taken for the compaction operation in milliseconds.
     * @returns {number}
     */
    get duration_ms() {
        const ret = wasm.__wbg_get_wasmcompactionresult_duration_ms(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) WasmCompactionResult.prototype[Symbol.dispose] = WasmCompactionResult.prototype.free;

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
    imports.wbg.__wbg_Error_52673b7de5a0ca89 = function(arg0, arg1) {
        const ret = Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg___wbindgen_is_function_8d400b8b1af978cd = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbg___wbindgen_is_undefined_f6b95eab589e0269 = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbg___wbindgen_string_get_a2a31e16edf96e42 = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
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
    imports.wbg.__wbg_call_c8baa5c5e72d274e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
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
            wasm.__wbindgen_export4(deferred0_0, deferred0_1, 1);
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
    imports.wbg.__wbg_length_406f6daaaa453057 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_86ce4877baf913bb = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_89c3414ed7f0594d = function(arg0) {
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
    imports.wbg.__wbg_new_25f239778d6112b9 = function() {
        const ret = new Array();
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
                    return __wasm_bindgen_func_elem_596(a, state0.b, arg0, arg1);
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
    imports.wbg.__wbg_prototypesetcall_6a0ca140cebe5ef8 = function(arg0, arg1, arg2) {
        Uint32Array.prototype.set.call(getArrayU32FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_prototypesetcall_96cc7097487b926d = function(arg0, arg1, arg2) {
        Float32Array.prototype.set.call(getArrayF32FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_prototypesetcall_d3c4edbb4ef96ca1 = function(arg0, arg1, arg2) {
        Float64Array.prototype.set.call(getArrayF64FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_prototypesetcall_dfe9b766cdc1f1fd = function(arg0, arg1, arg2) {
        Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
    };
    imports.wbg.__wbg_push_7d9be8f38fc13975 = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_queueMicrotask_9b549dfce8865860 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_queueMicrotask_fca69f5bfad613a5 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_read_fae8abcd008c321a = function() { return handleError(function (arg0, arg1) {
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
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
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
    imports.wbg.__wbg_write_2eb8c26f002dc6fe = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = IndexedDbBackend.write(getStringFromWasm0(arg0, arg1), getArrayU8FromWasm0(arg2, arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(String) -> Externref`.
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cast_9802243c858d266f = function(arg0, arg1) {
        // Cast intrinsic for `Closure(Closure { dtor_idx: 71, function: Function { arguments: [Externref], shim_idx: 72, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
        const ret = makeMutClosure(arg0, arg1, wasm.__wasm_bindgen_func_elem_394, __wasm_bindgen_func_elem_401);
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
    cachedFloat64ArrayMemory0 = null;
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
