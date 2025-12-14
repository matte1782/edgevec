# Week 12 — Day 1 Tasks (Monday)

**Date:** 2025-12-16
**Focus:** Define TypeScript Type Definitions
**Agent:** WASM_SPECIALIST
**Status:** [REVISED]

---

## Context References

**Required Reading:**
- `docs/architecture/WASM_BOUNDARY.md` — FFI safety rules
- `src/batch.rs` — Rust BatchInsertable trait
- `docs/rfcs/0001-batch-insert-api.md` — Original specification

---

## Day Objective

Define TypeScript types for the batch insert API. These types will guide the Rust FFI implementation and ensure JavaScript developers have excellent IDE support.

**Success Criteria:**
- TypeScript types compile with `tsc --strict --noEmit`
- All 3 public types documented with JSDoc
- Types map 1:1 to Week 11 Rust API

---

## Tasks

### W12.1: Define TypeScript Types for Batch Insert API

**Priority:** P0 (Critical Path)
**Estimate:** Raw: 2h → **6h with 3x**
**Agent:** WASM_SPECIALIST

#### Inputs

- `src/batch.rs` — Rust BatchInsertable trait (reference)
- `wasm/pkg/edgevec.d.ts` — Existing TypeScript definitions (if present)
- `docs/architecture/WASM_BOUNDARY.md` — FFI constraints

#### Outputs

- `wasm/batch_types.ts` — TypeScript type definitions (follows project convention)
- Updated `wasm/pkg/edgevec.d.ts` — Extended with batch types (if needed)

#### Specification

**Required Types (3 total):**

```typescript
/**
 * Configuration options for batch insert operations.
 */
export interface BatchInsertConfig {
  /**
   * If true, validates all vectors have matching dimensions before insertion.
   * @default true
   */
  validateDimensions?: boolean;
}

/**
 * Result of a batch insert operation.
 */
export interface BatchInsertResult {
  /** Number of vectors successfully inserted (0 to total) */
  inserted: number;
  /** Total vectors attempted (same as input array length) */
  total: number;
  /** IDs of inserted vectors, in insertion order */
  ids: number[];
}

/**
 * Error thrown when batch insert fails.
 * Maps 1:1 to Rust BatchError variants.
 */
export interface BatchInsertError extends Error {
  /**
   * Error code for programmatic handling.
   * Maps to Rust: EmptyBatch, DimensionMismatch, DuplicateId, InvalidVector, CapacityExceeded, InternalError
   */
  code: 'EMPTY_BATCH' | 'DIMENSION_MISMATCH' | 'DUPLICATE_ID' | 'INVALID_VECTOR' | 'CAPACITY_EXCEEDED' | 'INTERNAL_ERROR';
  /** Additional context from Rust error message */
  details?: string;
}
```

**Extended EdgeVecIndex Method:**

```typescript
export class EdgeVecIndex {
  /**
   * Insert multiple vectors in a single batch operation.
   *
   * @param vectors - Array of Float32Array vectors to insert (1 to 100,000)
   * @param config - Optional configuration (default: { validateDimensions: true })
   * @returns Promise resolving to BatchInsertResult
   * @throws BatchInsertError if insertion fails
   *
   * @example
   * const vectors = [new Float32Array([0.1, 0.2]), new Float32Array([0.3, 0.4])];
   * const result = await index.insertBatch(vectors);
   * console.log(`Inserted ${result.inserted} of ${result.total} vectors`);
   */
  insertBatch(
    vectors: Float32Array[],
    config?: BatchInsertConfig
  ): Promise<BatchInsertResult>;
}
```

#### Acceptance Criteria (Binary Pass/Fail)

- [ ] **AC1.1:** Types compile without errors when running `tsc --strict --noEmit` (exit code 0)
- [ ] **AC1.2:** Exactly 3 types defined: `BatchInsertConfig`, `BatchInsertResult`, `BatchInsertError`
- [ ] **AC1.3:** All 3 types have JSDoc comments (verified by `grep -c "@" batch_types.ts` returns ≥9)
- [ ] **AC1.4:** `BatchInsertConfig.validateDimensions` is optional with `@default true` documented
- [ ] **AC1.5:** `BatchInsertResult` has exactly 3 fields: `inserted` (number), `total` (number), `ids` (number[])
- [ ] **AC1.6:** `BatchInsertError.code` has exactly 6 values matching Rust BatchError variants (1:1 mapping)

#### Verification Commands

```bash
# AC1.1: Types compile
cd wasm && npx tsc --strict --noEmit batch_types.ts
echo "Exit code: $?"  # Must be 0

# AC1.2: Count exported interfaces
grep -c "export interface" wasm/batch_types.ts  # Must be 3

# AC1.3: Count JSDoc annotations
grep -c "@" wasm/batch_types.ts  # Must be ≥9

# AC1.6: Verify error codes match Rust (6 codes)
grep "code:" wasm/batch_types.ts | grep -q "EMPTY_BATCH.*DIMENSION_MISMATCH.*DUPLICATE_ID.*INVALID_VECTOR.*CAPACITY_EXCEEDED.*INTERNAL_ERROR"
```

---

## Commit Strategy

```
[W12.1] AC1.1 Types compile - BatchInsertConfig, BatchInsertResult, BatchInsertError
[W12.1] AC1.2-AC1.6 All criteria verified - Ready for Day 2 review
```

---

## Day 1 Summary

**Total Effort:** 6h (2h raw × 3x multiplier)

**Deliverables:**
1. `wasm/batch_types.ts` — 3 TypeScript type definitions with JSDoc (6 error codes)

**Exit Criteria:**
- [ ] All 6 acceptance criteria for W12.1 met
- [ ] Types reviewed by peer (informal)
- [ ] Ready for Gate 1 review (Day 2)

---

**Next:** Day 2 — Create API design document and Gate 1 review
