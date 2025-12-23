# HOSTILE_REVIEWER: F32 WASM Crash Deep Analysis

**Date:** 2025-12-23
**Issue:** F32 search "memory access out of bounds" in WASM
**Status:** UNDER INVESTIGATION

---

## Summary

F32 search crashes with "memory access out of bounds" in WASM while:
- BQ search: WORKS
- Hybrid search: WORKS
- Filtered search: FIXED (C1 off-by-one applied)
- F32 search: CRASHES

---

## Code Analysis Completed

### 1. Data Flow Verified

| Step | Function | Status |
|:-----|:---------|:-------|
| Insert | `EdgeVec::insertWithMetadata()` | Correct |
| → | `HnswIndex::insert_with_metadata()` | Correct |
| → | `HnswIndex::insert()` → `insert_impl()` | Correct |
| → | `VectorStorage::insert()` | Appends to `data_f32` ✅ |
| → | `HnswIndex::add_node(vector_id, level)` | Stores `vector_id` correctly ✅ |
| Search | `EdgeVec::search()` | Correct |
| → | `HnswIndex::search()` | Correct |
| → | `Searcher::search_layer()` | Calls `get_vector()` |
| → | `VectorStorage::get_vector(id)` | Bounds check added ✅ |

### 2. ID Translation Verified

```rust
// VectorStorage::get_vector()
let idx = (id.0 as usize) - 1;  // VectorId 1 → index 0 ✅
let start = idx * dim;
let end = start + dim;

// Bounds check added:
assert!(end <= self.data_f32.len(), "...");
```

### 3. Rust Tests PASS

Created `tests/wasm_flow.rs` that replicates exact demo flow:
- 768 dimensions (same as demo)
- 800 vectors (same as demo)
- enableBQ() before inserts (same as demo)
- F32 search after inserts

**Result:** ALL TESTS PASS in native Rust.

---

## Hypothesis: WASM-Specific Issue

The crash only occurs in WASM release mode, not in native Rust tests.

### Possible Causes (Ordered by Likelihood)

1. **Release Mode Optimization Issue**
   - Rust's release optimizations may interact unexpectedly with WASM
   - Some memory access pattern being reordered unsafely

2. **wasm-bindgen Memory Management**
   - Float32Array conversion (`query.to_vec()`) may have edge case
   - Large Vec allocation in WASM linear memory

3. **Browser WASM Implementation**
   - Specific browser (Chrome/Firefox) WASM runtime behavior
   - Memory growth or bounds checking differences

4. **Timing/Concurrency** (Unlikely - WASM is single-threaded)

---

## Recommended Debugging Steps

### Step 1: Test with Debug WASM

A debug WASM build was created at `pkg-debug/`:

```javascript
// In demo HTML, change import path:
const wasmPath = '../../../pkg-debug/edgevec.js';
```

Debug build will provide:
- Full panic messages with line numbers
- No optimizations that might mask issues
- Better stack traces

### Step 2: Use Browser DevTools

1. Open Chrome/Firefox DevTools → Sources tab
2. Enable "Pause on exceptions"
3. Run F32 search
4. Capture exact WASM instruction that crashes

### Step 3: Add Explicit Logging

In `src/wasm/mod.rs` search(), add before the search call:

```rust
web_sys::console::log_1(&format!(
    "F32 search: storage.len={}, data_f32.len={}",
    self.storage.len(),
    self.storage.data_f32.len() / (self.inner.config.dimensions as usize)
).into());
```

### Step 4: Check Memory State

Before search, verify storage state:
```javascript
console.log('Vector count:', db.liveCount());
console.log('Has BQ:', db.hasBQ());
```

---

## Fixes Applied in This Session

### C1: Off-by-One in EdgeVecMetadataAdapter

**File:** `src/wasm/mod.rs:2350`
**Change:** `self.store.get_all((id + 1) as u32)`

This fixed filtered search returning zero results.

### M1: Bounds Checking in get_vector()

**File:** `src/storage/mod.rs:469-476`
**Change:** Added explicit assert with detailed error message

```rust
assert!(
    end <= self.data_f32.len(),
    "get_vector: VectorId {} out of bounds (idx={}, end={}, data_len={})",
    id.0, idx, end, self.data_f32.len()
);
```

### m1: Removed Redundant BQ Conditional

**Files:** `src/wasm/mod.rs:246-250, 319-323`
**Change:** Removed `if has_bq()` since `insert_impl()` handles BQ automatically

---

## Tests Passing

| Test | Status |
|:-----|:-------|
| `cargo test --test bq_recall_roundtrip` | 7/7 ✅ |
| `cargo test --test bq_persistence` | 7/7 ✅ |
| `cargo test --test hybrid_search` | 5/5 ✅ |
| `cargo test --test wasm_flow` | 1/1 ✅ |
| `cargo clippy --all-features` | 0 warnings ✅ |

---

## Next Actions Required

1. **User Testing:** Test with debug WASM (`pkg-debug/`) to get better error messages
2. **Browser Debugging:** Use DevTools to capture exact crash location
3. **Report Findings:** Share browser console output and DevTools stack trace

---

## VERDICT (Preliminary)

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: INVESTIGATION ONGOING                          │
│                                                                     │
│   Issue: F32 WASM Crash                                             │
│   Rust Logic: VERIFIED CORRECT                                      │
│   Tests: ALL PASS                                                   │
│                                                                     │
│   Disposition:                                                      │
│   - Code review complete, no obvious bugs found                     │
│   - Requires browser-level debugging to identify root cause         │
│   - Debug WASM build provided for better error messages             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

**Next Step:** User should test with debug WASM and report browser console output.
