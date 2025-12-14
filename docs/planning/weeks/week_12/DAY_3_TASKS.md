# Week 12 — Day 3 Tasks (Wednesday)

**Date:** 2025-12-18
**Focus:** Implement Rust FFI for insertBatch
**Agent:** RUST_ENGINEER, WASM_SPECIALIST
**Status:** [REVISED]

---

## Context References

**Required Reading:**
- `docs/architecture/WASM_BATCH_API.md` — Approved API design (Gate 1)
- `docs/architecture/WASM_BOUNDARY.md` — FFI safety rules
- `src/batch.rs` — Rust BatchInsertable trait

---

## Day Objective

Implement the Rust FFI layer that exposes `BatchInsertable` to JavaScript via `#[wasm_bindgen]`. This is the core implementation day.

**Prerequisites:**
- ✅ Gate 1 passed (API design approved)

**Success Criteria:**
- Rust FFI compiles for `wasm32-unknown-unknown` target
- All 8 unit tests pass
- No `unsafe` or `unwrap()` in FFI code

---

## Tasks

### W12.3: Implement Rust FFI for insertBatch

**Priority:** P0 (Critical Path)
**Estimate:** Raw: 3h → **9h with 3x**
**Agent:** RUST_ENGINEER

#### Inputs

- `docs/architecture/WASM_BATCH_API.md` — Approved API design
- `src/batch.rs` — BatchInsertable trait implementation
- `wasm/src/lib.rs` — Existing WASM bindings (if present)

#### Outputs

- `wasm/src/batch.rs` — New FFI module for batch operations
- Updated `wasm/src/lib.rs` — Module declaration and exports
- 8 unit tests for FFI layer

#### Specification

**File: `wasm/src/batch.rs`**

```rust
use wasm_bindgen::prelude::*;
use js_sys::{Array, Float32Array};

/// Configuration for batch insert operations (WASM).
#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct BatchInsertConfig {
    validate_dimensions: bool,
}

#[wasm_bindgen]
impl BatchInsertConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { validate_dimensions: true }
    }

    #[wasm_bindgen(getter)]
    pub fn validate_dimensions(&self) -> bool {
        self.validate_dimensions
    }

    #[wasm_bindgen(setter)]
    pub fn set_validate_dimensions(&mut self, value: bool) {
        self.validate_dimensions = value;
    }
}

/// Result of a batch insert operation (WASM).
#[wasm_bindgen]
pub struct BatchInsertResult {
    inserted: usize,
    total: usize,
    ids: Vec<u64>,
}

#[wasm_bindgen]
impl BatchInsertResult {
    #[wasm_bindgen(getter)]
    pub fn inserted(&self) -> usize { self.inserted }

    #[wasm_bindgen(getter)]
    pub fn total(&self) -> usize { self.total }

    #[wasm_bindgen(getter)]
    pub fn ids(&self) -> Vec<u64> { self.ids.clone() }
}

// Implementation on EdgeVecIndex (in wasm/src/lib.rs or batch.rs)
#[wasm_bindgen]
impl EdgeVecIndex {
    #[wasm_bindgen(js_name = insertBatch)]
    pub fn insert_batch_js(
        &mut self,
        vectors: Array,
        config: Option<BatchInsertConfig>,
    ) -> Result<BatchInsertResult, JsValue> {
        // Implementation per WASM_BATCH_API.md
    }
}
```

**Required Unit Tests (8 total):**

1. `test_batch_config_default` — Default config has `validate_dimensions: true`
2. `test_batch_config_setter` — Setter changes value
3. `test_batch_result_getters` — All 3 getters return correct values
4. `test_batch_result_ids_clone` — IDs are cloned (not moved)
5. `test_empty_batch_error` — Empty array returns `EMPTY_BATCH` error
6. `test_dimension_mismatch_error` — Wrong dimension returns `DIMENSION_MISMATCH` error
7. `test_successful_insert` — Valid batch returns correct result
8. `test_partial_success` — Batch with duplicates returns partial result

#### Acceptance Criteria (Binary Pass/Fail)

- [ ] **AC3.1:** File `wasm/src/batch.rs` exists with ≥100 lines of code
- [ ] **AC3.2:** Compiles for WASM: `cargo build --target wasm32-unknown-unknown` exits with code 0
- [ ] **AC3.3:** `#[wasm_bindgen(js_name = insertBatch)]` attribute present
- [ ] **AC3.4:** All 5 error codes return correct error strings (EMPTY_BATCH, DIMENSION_MISMATCH, etc.)
- [ ] **AC3.5:** Unit tests pass: `cargo test --lib` with 8 tests passing
- [ ] **AC3.6:** Zero `unsafe` blocks: `grep -c "unsafe" wasm/src/batch.rs` returns 0
- [ ] **AC3.7:** Zero `unwrap()` or `expect()`: `grep -c "unwrap\|expect" wasm/src/batch.rs` returns 0
- [ ] **AC3.8:** Module exported in `wasm/src/lib.rs`: `mod batch;` present

#### Verification Commands

```bash
# AC3.1: File exists with ≥100 lines
wc -l wasm/src/batch.rs | awk '{print ($1 >= 100 ? "PASS" : "FAIL")}'

# AC3.2: WASM build
cargo build --target wasm32-unknown-unknown
echo "Exit code: $?"

# AC3.3: wasm_bindgen attribute
grep -q 'js_name = insertBatch' wasm/src/batch.rs && echo "PASS" || echo "FAIL"

# AC3.5: Run tests
cargo test --lib 2>&1 | grep -E "test result.*8 passed"

# AC3.6: No unsafe
grep -c "unsafe" wasm/src/batch.rs  # Must be 0

# AC3.7: No unwrap/expect
grep -c "unwrap\|expect" wasm/src/batch.rs  # Must be 0

# AC3.8: Module declared
grep -q "mod batch" wasm/src/lib.rs && echo "PASS" || echo "FAIL"
```

---

## Commit Strategy

```
[W12.3] AC3.1 FFI module created - BatchInsertConfig, BatchInsertResult structs
[W12.3] AC3.2-AC3.4 insertBatch implemented - All error codes mapped
[W12.3] AC3.5 Unit tests pass - 8 tests covering all paths
[W12.3] AC3.6-AC3.8 Quality checks pass - No unsafe, no unwrap, module exported
```

---

## Day 3 Summary

**Total Effort:** 9h (3h raw × 3x multiplier)

**Deliverables:**
1. `wasm/src/batch.rs` — Complete FFI implementation (≥100 lines)
2. 8 unit tests passing
3. Module exported in `wasm/src/lib.rs`

**Exit Criteria:**
- [ ] All 8 acceptance criteria for W12.3 met
- [ ] WASM package builds successfully
- [ ] Ready for Day 4 JavaScript integration

---

**Next:** Day 4 — Create JavaScript integration examples and run browser benchmarks
