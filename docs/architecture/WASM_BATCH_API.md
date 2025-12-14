# EdgeVec WASM Batch Insert API Design

**Date:** 2025-12-13
**Author:** WASM_SPECIALIST
**Status:** [PROPOSED]
**Version:** 1.0.0

---

## 1. Function Signature

The batch insert API exposes a single method on the `EdgeVecIndex` class:

```typescript
class EdgeVecIndex {
  /**
   * Insert multiple vectors in a single batch operation.
   *
   * @param vectors - Array of Float32Array vectors to insert (1 to 100,000)
   * @param config - Optional configuration (default: { validateDimensions: true })
   * @returns Promise resolving to BatchInsertResult
   * @throws BatchInsertError if insertion fails
   */
  insertBatch(
    vectors: Float32Array[],
    config?: BatchInsertConfig
  ): Promise<BatchInsertResult>;
}
```

### Rust FFI Binding

```rust
#[wasm_bindgen]
impl EdgeVecIndex {
    #[wasm_bindgen(js_name = insertBatch)]
    pub async fn insert_batch(
        &mut self,
        vectors: js_sys::Array,
        config: Option<JsBatchInsertConfig>,
    ) -> Result<JsBatchInsertResult, JsValue>;
}
```

---

## 2. Type Mapping

| TypeScript Type | Rust Type | Notes |
|:----------------|:----------|:------|
| `Float32Array[]` | `js_sys::Array` of `js_sys::Float32Array` | Each array is one vector |
| `Float32Array` | `js_sys::Float32Array` | Single vector data |
| `BatchInsertConfig` | `JsBatchInsertConfig` | WASM-bindgen wrapper struct |
| `BatchInsertResult` | `JsBatchInsertResult` | WASM-bindgen wrapper struct |
| `BatchInsertError` | `JsValue` from `BatchError` | Via `From<BatchError> for JsValue` |
| `number` | `u32` / `usize` | JavaScript safe integer |
| `number[]` (ids) | `Vec<u64>` → `BigInt64Array` | Vector IDs (u64 for capacity) |
| `boolean` | `bool` | Direct mapping |
| `string` | `String` / `JsString` | UTF-8 |
| `Promise<T>` | `js_sys::Promise` | Async boundary |

### Type Definitions (TypeScript)

```typescript
export interface BatchInsertConfig {
  validateDimensions?: boolean;  // @default true
}

export interface BatchInsertResult {
  inserted: number;  // Successfully inserted count
  total: number;     // Input array length
  ids: number[];     // IDs of inserted vectors
}

export interface BatchInsertError extends Error {
  code: 'EMPTY_BATCH' | 'DIMENSION_MISMATCH' | 'DUPLICATE_ID' |
        'INVALID_VECTOR' | 'CAPACITY_EXCEEDED' | 'INTERNAL_ERROR';
  details?: string;
}
```

---

## 3. Error Mapping

All 6 error codes map 1:1 between Rust `BatchError` and TypeScript `BatchInsertError.code`:

| Rust Variant | JS Error Code | Trigger Condition | Fatal? |
|:-------------|:--------------|:------------------|:-------|
| `BatchError::EmptyBatch` | `EMPTY_BATCH` | `vectors.length === 0` | Yes |
| `BatchError::DimensionMismatch` | `DIMENSION_MISMATCH` | First vector: `vector.length !== index.dimensions` | Yes |
| `BatchError::DimensionMismatch` | `DIMENSION_MISMATCH` | Later vectors: skipped (non-fatal) | No |
| `BatchError::DuplicateId` | `DUPLICATE_ID` | Vector ID already exists in index | No* |
| `BatchError::InvalidVector` | `INVALID_VECTOR` | Vector contains `NaN` or `Infinity` values | No* |
| `BatchError::CapacityExceeded` | `CAPACITY_EXCEEDED` | Batch would exceed `index.maxCapacity` | Yes |
| `BatchError::InternalError` | `INTERNAL_ERROR` | HNSW graph invariant violated (bug) | Yes |

**\*** Non-fatal errors are skipped in best-effort mode; the problematic vector is not inserted but processing continues.

### Error Code Descriptions

1. **`EMPTY_BATCH`**: The input array has zero elements. Minimum batch size is 1.

2. **`DIMENSION_MISMATCH`**: Vector dimensionality does not match index configuration.
   - First vector mismatch: Fatal, entire batch aborted
   - Later vector mismatch: Skipped, batch continues
   - Details include: `expected`, `actual`, `vector_id`

3. **`DUPLICATE_ID`**: Vector ID already exists in the index. In auto-ID mode, this cannot occur. In manual ID mode, duplicates are skipped.
   - Details include: `vector_id`

4. **`INVALID_VECTOR`**: Vector contains non-finite floating-point values (`NaN`, `Infinity`, `-Infinity`).
   - These vectors are skipped (non-fatal)
   - Details include: `vector_id`, `reason`

5. **`CAPACITY_EXCEEDED`**: Adding the batch would exceed maximum index capacity.
   - Checked before insertion begins
   - Details include: `current`, `max`

6. **`INTERNAL_ERROR`**: Unexpected HNSW invariant violation.
   - Indicates a bug in EdgeVec itself
   - Details include: `message`

### JavaScript Error Handling Pattern

```javascript
try {
  const result = await index.insertBatch(vectors);
} catch (error) {
  if (error.code === 'DIMENSION_MISMATCH') {
    console.error(`Dimension error: ${error.details}`);
  } else if (error.code === 'CAPACITY_EXCEEDED') {
    console.error('Index is full');
  } else {
    console.error(`Batch error [${error.code}]: ${error.message}`);
  }
}
```

---

## 4. Performance Contract

### FFI Overhead Target

> **FFI overhead MUST be <5% of total insertion time.**

This means:
- 95% of execution time is spent in Rust HNSW algorithm
- ≤5% of execution time is spent on JS↔WASM boundary crossing

### Measurement Protocol

```
FFI Overhead = (Total Time - Rust Core Time) / Total Time × 100%
```

Where:
- **Total Time**: Wall-clock time from JS `insertBatch()` call to Promise resolution
- **Rust Core Time**: Time spent in `HnswIndex::batch_insert()` (excludes marshaling)

### Overhead Budget Breakdown

| Component | Budget | Notes |
|:----------|:-------|:------|
| Input validation (JS) | <1% | Array type checks |
| Array marshaling (JS→WASM) | <2% | `Float32Array[]` to Rust `Vec` |
| Result marshaling (WASM→JS) | <1% | `Vec<u64>` to `number[]` |
| Error conversion | <1% | `BatchError` to `JsValue` |
| **Total FFI Overhead** | **<5%** | Combined budget |

### Benchmark Validation

FFI overhead will be validated via `benches/wasm_ffi_overhead.rs`:
- Compare WASM batch insert vs native Rust batch insert
- 10,000 vectors × 128 dimensions
- P50 and P99 latencies compared
- CI gate: Fail if overhead exceeds 5%

---

## 5. Batch Size Limits

Maximum vectors per batch depends on dimension and available WASM memory:

| Dimension | Max Vectors | Memory (Float32) | Memory (Total*) | Rationale |
|:----------|:------------|:-----------------|:----------------|:----------|
| 128 | 100,000 | ~51 MB | ~76 MB | WASM heap budget |
| 512 | 50,000 | ~102 MB | ~153 MB | WASM heap budget |
| 768 | 30,000 | ~92 MB | ~138 MB | WASM heap budget |
| 1536 | 15,000 | ~92 MB | ~138 MB | WASM heap budget |

**\*Total** includes vector data + HNSW graph overhead (~50% additional)

### Memory Calculation

```
Vector Memory = count × dimensions × 4 bytes (f32)
Graph Memory  ≈ count × (M × 2 × 8 bytes) × avg_layers
Total Memory  ≈ Vector Memory × 1.5
```

### Runtime Enforcement

```rust
const MAX_BATCH_VECTORS: usize = 100_000;

fn validate_batch_size(count: usize, dimensions: u32) -> Result<(), BatchError> {
    let max_for_dim = match dimensions {
        0..=128 => 100_000,
        129..=512 => 50_000,
        513..=768 => 30_000,
        769..=1536 => 15_000,
        _ => 10_000,  // Conservative for larger dims
    };

    if count > max_for_dim {
        return Err(BatchError::CapacityExceeded {
            current: count,
            max: max_for_dim
        });
    }
    Ok(())
}
```

---

## 6. JavaScript Conventions

The API follows these JavaScript conventions for idiomatic usage:

### Checklist (All Required)

- [x] **camelCase**: All method and property names use camelCase
  - `insertBatch` (not `insert_batch`)
  - `validateDimensions` (not `validate_dimensions`)

- [x] **Promise**: Async operations return native `Promise<T>`
  - `insertBatch()` returns `Promise<BatchInsertResult>`
  - Errors are thrown as rejected promises

- [x] **Config Object**: Optional parameters use a single config object
  - `insertBatch(vectors, config?)` not `insertBatch(vectors, validateDimensions?)`
  - Config properties are optional with documented defaults

- [x] **Error**: Errors extend native `Error` with typed `code` property
  - `catch (e) { if (e.code === 'EMPTY_BATCH') ... }`
  - Compatible with standard JavaScript error handling

### Additional Conventions

| Convention | Implementation |
|:-----------|:---------------|
| TypeScript-first | Full `.d.ts` type definitions shipped |
| JSDoc comments | All public APIs documented |
| Tree-shakeable | ES module exports only |
| Zero dependencies | No runtime dependencies in JS layer |

---

## 7. Example Usage

Minimal working example (10 lines):

```typescript
import { EdgeVecIndex } from 'edgevec';

const index = new EdgeVecIndex({ dimensions: 128 });
const vectors = [
  new Float32Array([0.1, 0.2, /* ...126 more */]),
  new Float32Array([0.3, 0.4, /* ...126 more */]),
];

const result = await index.insertBatch(vectors);
console.log(`Inserted ${result.inserted}/${result.total} vectors`);
// Output: Inserted 2/2 vectors
```

### Extended Example with Error Handling

```typescript
import { EdgeVecIndex, BatchInsertError } from 'edgevec';

async function batchInsert(index: EdgeVecIndex, vectors: Float32Array[]) {
  try {
    const result = await index.insertBatch(vectors, {
      validateDimensions: true
    });

    if (result.inserted < result.total) {
      console.warn(`${result.total - result.inserted} vectors skipped`);
    }

    return result.ids;
  } catch (error) {
    const e = error as BatchInsertError;
    switch (e.code) {
      case 'EMPTY_BATCH':
        throw new Error('Cannot insert empty batch');
      case 'DIMENSION_MISMATCH':
        throw new Error(`Vector dimension mismatch: ${e.details}`);
      case 'CAPACITY_EXCEEDED':
        throw new Error('Index capacity exceeded');
      default:
        throw error;
    }
  }
}
```

---

## Appendix A: FFI Safety Compliance

This API complies with `WASM_BOUNDARY.md` v1.1:

| Rule | Compliance |
|:-----|:-----------|
| No panics across boundary | ✅ All paths return `Result<T, JsValue>` |
| No `dyn Trait` | ✅ Only concrete types |
| No raw pointers | ✅ Uses `Box`, `Vec`, `TypedArray` |
| Explicit String handling | ✅ String in structs, `&str` in signatures |
| All types `#[wasm_bindgen]` | ✅ Explicit boundary crossing |

---

## Appendix B: References

- `wasm/batch_types.ts` — TypeScript type definitions (W12.1)
- `src/batch.rs` — Rust BatchInsertable trait
- `src/error.rs` — Rust BatchError enum with JsValue conversion
- `docs/architecture/WASM_BOUNDARY.md` — FFI safety rules

---

**Document Status:** [PROPOSED] — Awaiting HOSTILE_REVIEWER approval (Gate 1)
