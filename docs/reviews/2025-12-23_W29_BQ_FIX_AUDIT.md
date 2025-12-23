# HOSTILE_REVIEWER: W29 BQ Integration Fix Audit

**Date:** 2025-12-23
**Artifact:** BQ Integration Fix (W29.1 - Engineer Implementation)
**Author:** RUST_ENGINEER
**Reviewer:** HOSTILE_REVIEWER

---

## Review Intake

### User-Reported Issues
1. BQ search: **WORKING** (after fix)
2. Hybrid search: **WORKING** (after fix)
3. Filtered search: **RETURNS ZERO RESULTS** for category filters like "tech"
4. F32 (Accurate) search: **CRASHES** with "memory access out of bounds"

---

## Findings

### CRITICAL [C1]: Off-by-One Bug in EdgeVecMetadataAdapter

**Location:** `src/wasm/mod.rs:2349`

**Evidence:**
```rust
impl crate::filter::MetadataStore for EdgeVecMetadataAdapter<'_> {
    fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
        // BUG: Filter uses 0-indexed iteration, but MetadataStore uses 1-indexed VectorId
        self.store.get_all(id as u32)  // <-- Wrong! Should be (id + 1)
    }
}
```

**Root Cause:**
- Filter code (`filtered_search.rs:405`) iterates `for idx in 0..total`
- Metadata is stored with 1-indexed VectorId keys (1, 2, 3, ...)
- Lookup `get_all(0)` returns `None` (no vector 0 exists)
- Lookup `get_all(1)` returns metadata for vector 1, but filter expects it at idx 0
- Result: ALL filter lookups are offset by 1, returning wrong or no metadata

**Impact:** Filtered search returns zero results for ANY filter expression

**Fix Required:**
```rust
fn get_metadata(&self, id: usize) -> Option<&HashMap<String, MetadataValue>> {
    // Filter uses 0-indexed iteration, but VectorId is 1-indexed
    self.store.get_all((id + 1) as u32)
}
```

---

### MAJOR [M1]: F32 Search "memory access out of bounds" - Unconfirmed Root Cause

**Location:** WASM runtime (exact location unknown)

**Evidence:**
- Rust unit tests pass: `cargo test --test bq_persistence test_bq_index_f32_search_after_load` ✅
- Rust integration tests pass: `cargo test --test hybrid_search` ✅
- WASM demo fails with "memory access out of bounds"

**Investigation Summary:**
- Traced full code path from WASM `search()` to `VectorStorage::get_vector()`
- `get_vector()` at `storage/mod.rs:468` accesses `data_f32[start..start + dim]`
- Could panic if `start + dim > data_f32.len()` but this shouldn't happen
- VectorId mapping appears correct (1-indexed → 0-indexed via `idx = id.0 - 1`)

**Hypothesis:**
The issue may be related to:
1. WASM memory growth/allocation during vector insertion
2. Serialization/deserialization state corruption
3. A second off-by-one bug in ID translation
4. Release mode optimization removing safety checks

**Status:** CANNOT REPRODUCE IN RUST TESTS - Requires WASM debugging

**Recommended Actions:**
1. Add explicit bounds check before `data_f32[start..start + dim]` access
2. Add WASM-specific integration test
3. Use browser DevTools to get exact crash location
4. Consider adding `#[track_caller]` for better panic traces

---

### MINOR [m1]: Redundant BQ Check in WASM insert()

**Location:** `src/wasm/mod.rs:246-255`

**Evidence:**
```rust
// Use insert_bq() when BQ is enabled to ensure BQ storage is updated
let id = if self.inner.has_bq() {
    self.inner.insert_bq(&vec, &mut self.storage)  // <-- Redundant
} else {
    self.inner.insert(&vec, &mut self.storage)
}
```

**Explanation:**
Since `insert_impl()` now handles BQ insertion automatically (Step 6), the conditional here is redundant. `insert()` already calls `insert_impl()` which handles BQ when enabled.

**Impact:** Code duplication, potential confusion
**Fix:** Remove conditional, always use `self.inner.insert()`

---

## BQ Fix Verification

### What Was Fixed (Correctly)
1. **`insert_impl()` Step 6:** Automatically inserts into BQ storage when BQ enabled ✅
2. **`insert_bq()` simplified:** Now just calls `insert()` since `insert_impl` handles BQ ✅
3. **`enableBQ()` WASM method added:** Properly exposed to JavaScript ✅

### What Was Verified
- `cargo test --test bq_recall_roundtrip`: 7/7 tests pass ✅
- `cargo test --test bq_persistence`: 7/7 tests pass ✅
- `cargo test --test hybrid_search`: 5/5 tests pass ✅
- WASM builds without errors: 494 KB optimized ✅

---

## VERDICT

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: REJECT                                          │
│                                                                     │
│   Artifact: W29 BQ Integration Fix                                  │
│   Author: RUST_ENGINEER                                             │
│                                                                     │
│   Critical Issues: 1                                                │
│   Major Issues: 1                                                   │
│   Minor Issues: 1                                                   │
│                                                                     │
│   Disposition:                                                      │
│   BLOCK: Cannot proceed until C1 is fixed                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Required Actions Before Resubmission

### Mandatory (Blocking)

1. **[C1] Fix EdgeVecMetadataAdapter off-by-one bug**
   - File: `src/wasm/mod.rs`
   - Line: 2349
   - Change: `self.store.get_all(id as u32)` → `self.store.get_all((id + 1) as u32)`

2. **[M1] Investigate F32 search WASM crash**
   - Add bounds checking before array access in `get_vector()`
   - Create WASM-specific test to reproduce
   - If cannot fix, document as known issue with workaround

### Recommended (Non-Blocking)

3. **[m1] Remove redundant BQ conditional in WASM insert()**
   - File: `src/wasm/mod.rs`
   - Lines: 246-255
   - Change: Remove `if self.inner.has_bq()` conditional

---

## Resubmission Checklist

- [ ] C1 fix applied and tested
- [ ] M1 investigated and either fixed or documented
- [ ] WASM rebuilt and optimized
- [ ] Browser demo tested with all 4 search modes
- [ ] `cargo test` passes all tests

---

**Resubmit via:** `/review W29_BQ_FIX_v2`
