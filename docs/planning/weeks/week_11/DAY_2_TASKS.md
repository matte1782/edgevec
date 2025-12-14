# Week 11 — Day 2 Tasks (Tuesday)

**Date:** 2025-01-14
**Focus:** Core Batch Logic Implementation
**Agent:** RUST_ENGINEER
**Status:** DRAFT

---

## Day Objective

Complete the full implementation of the BatchInsertable trait. Transform the Day 1 stub into a production-ready batch insertion function with proper error handling, progress callbacks, and efficient batching logic.

**Success Criteria:**
- batch_insert() implements full logic (no stubs)
- All 5 error types properly handled
- Progress callbacks functional
- Internal batching optimizes for locality
- Unit tests pass (happy path)

---

## Theoretical Foundation

### Batch Processing Strategy

**Naive Approach (Sequential):**
```rust,ignore
for (id, vec) in vectors {
    index.insert(id, vec)?;  // O(log n) per insert
}
```

**Optimized Approach (Batched):**
```rust,ignore
// 1. Pre-validate all vectors (fail fast)
// 2. Sort by insertion order for cache locality
// 3. Batch insert with progress tracking
// 4. Return partial results on error
```

### Performance Optimization

**Locality Principle:**
- Consecutive inserts likely access similar graph regions
- Sorting by vector similarity improves cache hits
- But: Sorting overhead must not exceed insertion cost

**Decision:** No sorting for v0.1 (complexity > benefit). Revisit in Week 14.

### Progress Callback Design

**Callback Signature:**
```rust,ignore
F: FnMut(usize, usize)  // (inserted_count, total_count)
```

**Invocation Strategy:**
- Call at 0% (start)
- Call every 10% progress
- Call at 100% (end)
- Never call more than 11 times (avoid overhead)

### Error Recovery Strategy

**Atomicity:** Best-effort (no rollback)

| Error | Action | Return |
|:------|:-------|:-------|
| DimensionMismatch (first vector) | Abort immediately | Err(BatchError::DimensionMismatch) |
| DuplicateId | Skip, continue | Ok(partial_ids) |
| InvalidVector | Skip, log, continue | Ok(partial_ids) |
| CapacityExceeded | Abort | Err(BatchError::CapacityExceeded) |
| InternalError | Abort, panic in debug | Err(BatchError::InternalError) |

---

## Tasks

### W11.1: Implement BatchInsertable Trait (COMPLETE)

**Priority:** P0 (Critical Path)
**Estimate:** Raw: 8h total → **24h with 3x** (6h Day 1 + 18h Day 2)
**Agent:** RUST_ENGINEER
**Status:** DAY 2: Full implementation (18h)

**Calculation:** Raw: 6h (Day 2 portion) → 6h × 3 = 18h

#### Acceptance Criteria (Day 2 Completion)

- [ ] **AC1.7:** batch_insert() implements full logic (no TODOs)
- [ ] **AC1.8:** Pre-validates first vector dimensionality
- [ ] **AC1.9:** Handles all 5 error types correctly
- [ ] **AC1.10:** Progress callback invoked at 0%, 10%, 20%, ..., 100%
- [ ] **AC1.11:** Returns Vec<VectorId> for successful inserts
- [ ] **AC1.12:** Partial success on non-fatal errors
- [ ] **AC1.13:** All unit tests pass (from Day 3 prep)
- [ ] **AC1.14:** `cargo clippy -- -D warnings` passes
- [ ] **AC1.15:** No unsafe code without justification

#### Files to Modify

- `src/hnsw.rs` (replace batch_insert stub, add helper methods)

#### Dependencies

**Blocks:**
- W11.3 (Unit tests) — can now run real tests
- W11.4 (Integration test) — can test 10k vectors
- W11.5 (Benchmark) — can measure throughput

**Requires:**
- ✅ W11.2 (BatchError type) — completed Day 1

#### Verification Commands

```bash
# Must pass before marking complete
cargo build --release
cargo clippy -- -D warnings
cargo test batch --lib

# Manual smoke test
cargo run --example batch_insert_demo
```

---

## Day 2 Summary

**Total Effort:** Raw: 6h → **18h with 3x**

**Deliverables:**
1. ✅ Complete batch_insert() implementation
2. ✅ Error handling for all 5 types
3. ✅ Progress callback logic
4. ✅ Helper methods (contains_id, insert_internal)
5. ✅ Example usage in examples/

**Carryover to Day 3:**
- None (W11.1 complete)

**Blockers Removed:**
- W11.3 (Unit tests can now run against real implementation)
- W11.4 (Integration test can validate large batches)
- W11.5 (Benchmark can measure throughput)

**Status Validation:**
```bash
# Run before end of day
cargo build --release
cargo test batch --lib
cargo run --example batch_insert_demo
```

---

**PLANNER Notes:**
- Day 2 is the most complex implementation day
- 18h estimate (with 3x) accounts for implementation complexity
- Best-effort semantics simplifies rollback logic
- Progress callback design prevents excessive overhead

**Status:** DRAFT
**Next:** Day 3 unit testing validates all branches
