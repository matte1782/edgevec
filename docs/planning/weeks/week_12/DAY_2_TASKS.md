# Week 12 — Day 2 Tasks (Tuesday)

**Date:** 2025-12-17
**Focus:** Create API Design Document + Gate 1 Review
**Agent:** WASM_SPECIALIST, HOSTILE_REVIEWER
**Status:** [REVISED]

---

## Context References

**Required Reading:**
- `docs/architecture/WASM_BOUNDARY.md` — FFI safety rules
- `src/batch.rs` — Rust BatchInsertable trait
- `wasm/src/batch_types.ts` — Day 1 TypeScript types

---

## Day Objective

Create the API design document and obtain Gate 1 approval before implementation begins. This is a **hard gate** — Day 3 cannot start without approval.

**Success Criteria:**
- API design document created with all required sections
- HOSTILE_REVIEWER approval received (Gate 1)
- All edge cases and error codes documented

---

## Tasks

### W12.2: Create API Design Document + Gate 1 Review

**Priority:** P0 (Gate 1 Blocker)
**Estimate:** Raw: 2h → **6h with 3x**
**Agent:** WASM_SPECIALIST

#### Inputs

- `wasm/src/batch_types.ts` — Day 1 TypeScript types
- `src/batch.rs` — Rust BatchInsertable trait
- `src/error.rs` — Rust BatchError enum
- `docs/architecture/WASM_BOUNDARY.md` — FFI constraints

#### Outputs

- `docs/architecture/WASM_BATCH_API.md` — Complete API design document

#### Specification

**Required Sections (7 total):**

1. **Function Signature** — TypeScript method signature
2. **Type Mapping** — TypeScript ↔ Rust type correspondence
3. **Error Mapping** — All 5 error codes with trigger conditions
4. **Performance Contract** — FFI overhead target (<5%)
5. **Batch Size Limits** — Maximum vectors per dimension
6. **JavaScript Conventions** — camelCase, Promise, Config object checklist
7. **Example Usage** — Minimal working example (<10 lines)

**Error Mapping Table:**

| Rust BatchError | JS Error Code | Trigger Condition |
|:----------------|:--------------|:------------------|
| `EmptyBatch` | `EMPTY_BATCH` | `vectors.length === 0` |
| `DimensionMismatch` | `DIMENSION_MISMATCH` | `vector.length !== index.dimensions` |
| `DuplicateId` | `DUPLICATE_ID` | ID already exists in index |
| `CapacityExceeded` | `CAPACITY_EXCEEDED` | `vectors.length > 100000` |
| `InternalError` | `INTERNAL_ERROR` | HNSW graph invariant violated |

**Batch Size Limits:**

| Dimension | Max Vectors | Memory (Float32) | Rationale |
|:----------|:------------|:-----------------|:----------|
| 128 | 100,000 | ~51 MB | WASM heap < 4GB |
| 512 | 50,000 | ~102 MB | WASM heap < 4GB |
| 768 | 30,000 | ~92 MB | WASM heap < 4GB |
| 1536 | 15,000 | ~92 MB | WASM heap < 4GB |

#### Acceptance Criteria (Binary Pass/Fail)

- [ ] **AC2.1:** Document created at `docs/architecture/WASM_BATCH_API.md` (file exists)
- [ ] **AC2.2:** Document has exactly 7 required sections (verified by header count)
- [ ] **AC2.3:** All 5 error codes documented with exact trigger conditions
- [ ] **AC2.4:** Performance contract states "FFI overhead <5% of total insertion time"
- [ ] **AC2.5:** Batch size limits documented for 4 dimension values (128, 512, 768, 1536)
- [ ] **AC2.6:** JavaScript conventions checklist has 4 items checked (camelCase, Promise, Config, Error)
- [ ] **AC2.7:** Example usage is ≤10 lines of code
- [ ] **AC2.8:** HOSTILE_REVIEWER approval received (Gate 1 passed)

#### Verification Commands

```bash
# AC2.1: File exists
test -f docs/architecture/WASM_BATCH_API.md && echo "PASS" || echo "FAIL"

# AC2.2: Count ## headers (should be ≥7)
grep -c "^## " docs/architecture/WASM_BATCH_API.md

# AC2.3: All error codes present
grep -c "EMPTY_BATCH\|DIMENSION_MISMATCH\|DUPLICATE_ID\|CAPACITY_EXCEEDED\|INTERNAL_ERROR" docs/architecture/WASM_BATCH_API.md
# Should return 5

# AC2.5: Dimension limits present
grep -c "128\|512\|768\|1536" docs/architecture/WASM_BATCH_API.md
# Should return ≥4
```

---

## Gate 1: Design Review

**Reviewer:** HOSTILE_REVIEWER
**Command:** `/review docs/architecture/WASM_BATCH_API.md`

**Gate Criteria (All Must Pass):**
- [ ] API design is complete (7 sections)
- [ ] All 5 error cases documented with codes
- [ ] Performance target is measurable (FFI overhead <5%)
- [ ] No violations of WASM_BOUNDARY.md constraints
- [ ] JavaScript conventions followed (4/4 checked)

**Gate Status:** PENDING

**Blocker:** Day 3 (Rust FFI implementation) **CANNOT START** until Gate 1 passes.

---

## Commit Strategy

```
[W12.2] AC2.1-AC2.7 API design complete - WASM_BATCH_API.md created
[W12.2] AC2.8 Gate 1 passed - HOSTILE_REVIEWER approval received
```

---

## Day 2 Summary

**Total Effort:** 6h (2h raw × 3x multiplier)

**Deliverables:**
1. `docs/architecture/WASM_BATCH_API.md` — Complete API design document
2. Gate 1 approval from HOSTILE_REVIEWER

**Exit Criteria:**
- [ ] All 8 acceptance criteria for W12.2 met
- [ ] Gate 1 passed (HOSTILE_REVIEWER approval)
- [ ] Day 3 implementation unlocked

---

**Next:** Day 3 — Implement Rust FFI for insertBatch
