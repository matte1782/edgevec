# Week 13 — Day 1 Tasks (Monday, Dec 16)

**Date:** 2025-12-16
**Focus:** Unsafe Block Audit — Persistence Module
**Agent:** RUST_ENGINEER
**Status:** COMPLETE

---

## Day Objective

Audit ALL unsafe blocks in the persistence module and begin SIMD module audit. Document each unsafe block with SAFETY comments explaining the invariants that make it sound (or not). Identify the known UB issue in `snapshot.rs:223-227` and document other potential issues.

**Success Criteria:**
- `docs/audits/unsafe_audit_persistence.md` complete
- All unsafe blocks in `src/persistence/` have SAFETY comments
- W13.1b (SIMD audit) started
- No compilation regressions

---

## Theoretical Foundation

### Unsafe Pointer Cast Analysis

**The Known UB Issue (snapshot.rs:223-227):**

```rust
#[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
let nodes: &[HnswNode] = unsafe {
    let ptr = nodes_bytes.as_ptr() as *const HnswNode;
    std::slice::from_raw_parts(ptr, vec_count)
};
```

**Why This Is UB:**
1. `nodes_bytes.as_ptr()` returns a `*const u8` with byte alignment (1)
2. `HnswNode` requires 4-byte alignment (contains `u32` fields)
3. Cast `as *const HnswNode` does NOT verify alignment
4. If `nodes_bytes` happens to be misaligned, dereferencing is UB
5. `#[allow(clippy::cast_ptr_alignment)]` suppresses the warning—a red flag

**Architecture Impact:**
- x86_64: May "work" (lenient alignment) but still UB
- ARM: Will fault on misaligned access
- WASM: Depends on engine; Chrome lenient, strict mode faults

### Audit Methodology

For each unsafe block, document:
1. **Location:** File and line number
2. **Operation:** What the unsafe code does
3. **Invariants:** What conditions must be true for soundness
4. **Verification:** How invariants are enforced (or not)
5. **Classification:** SOUND, POTENTIALLY_UNSOUND, or UNSOUND
6. **Recommendation:** Keep, Fix, or Remove

---

## Tasks

### W13.1a: Unsafe Block Audit - Persistence Module

**Priority:** P0 (Critical Path)
**Estimate:** Raw: ~3h → **8h with 3x** (Day 1 complete)
**Agent:** RUST_ENGINEER
**Status:** COMPLETE on Day 1

#### Acceptance Criteria

- [ ] **AC1.1:** File `docs/audits/unsafe_audit_persistence.md` exists
- [ ] **AC1.2:** All unsafe blocks in `src/persistence/` listed with line numbers
- [ ] **AC1.3:** Each block has SAFETY comment in source code
- [ ] **AC1.4:** Known UB at snapshot.rs:223-227 documented as UNSOUND
- [ ] **AC1.5:** Secondary issue at chunking.rs:216-220 documented
- [ ] **AC1.6:** Classification provided for each block
- [ ] **AC1.7:** Recommendations provided for each block
- [ ] **AC1.8:** `cargo build` succeeds after adding SAFETY comments

#### Implementation Specification

**File to Create:** `docs/audits/unsafe_audit_persistence.md`

```markdown
# Unsafe Block Audit: Persistence Module

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-16
**Scope:** `src/persistence/*.rs`

---

## Summary

| File | Line | Classification | Recommendation |
|:-----|:-----|:---------------|:---------------|
| snapshot.rs | 223-227 | UNSOUND | FIX (bytemuck) |
| chunking.rs | 216-220 | UNSOUND | FIX (bytemuck) |
| ... | ... | ... | ... |

---

## Detailed Analysis

### snapshot.rs:223-227 (UNSOUND)

**Code:**
[include code block]

**Invariants Required:**
1. `nodes_bytes.as_ptr()` must be aligned to `align_of::<HnswNode>()`
2. `nodes_bytes.len() >= vec_count * size_of::<HnswNode>()`
3. Bytes must be valid HnswNode representations

**Verification:**
- Invariant 1: NOT VERIFIED (source is arbitrary bytes)
- Invariant 2: Partially verified (length check exists)
- Invariant 3: NOT VERIFIED (no validation of byte content)

**Impact:** Misaligned read on ARM/WASM strict mode causes undefined behavior.

**Recommendation:** Replace with `bytemuck::try_cast_slice()` which verifies alignment at runtime.

...
```

**Files to Modify:** Add SAFETY comments to each unsafe block:

```rust
// src/persistence/snapshot.rs:223-227
// SAFETY: UNSOUND — This block assumes alignment without verification.
// The ptr cast from u8 to HnswNode requires 4-byte alignment which is
// not guaranteed for arbitrary byte slices. This will be replaced with
// bytemuck::try_cast_slice() in W13.2.
// See: docs/audits/unsafe_audit_persistence.md
#[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
let nodes: &[HnswNode] = unsafe {
    // ...
};
```

#### Files to Create

- `docs/audits/unsafe_audit_persistence.md` (new)

#### Files to Modify

- `src/persistence/snapshot.rs` (add SAFETY comments)
- `src/persistence/chunking.rs` (add SAFETY comments)

#### Verification Commands

```bash
# Find all unsafe blocks in persistence module
grep -rn "unsafe" src/persistence/

# Verify build still works
cargo build

# Check clippy passes
cargo clippy -- -D warnings

# Verify audit file exists
test -f docs/audits/unsafe_audit_persistence.md && echo "OK"
```

---

### W13.1b: Unsafe Block Audit - SIMD Module (START)

**Priority:** P1 (Parallel with W13.1a)
**Estimate:** Raw: ~2h → **6h with 3x** (Day 1: 2h start, Day 2: 4h complete)
**Agent:** RUST_ENGINEER
**Status:** START Day 1 (2h), COMPLETE Day 2 (4h)

#### Day 1 Scope (Partial)

- [ ] **AC1b.1:** Identify all unsafe blocks in `src/metric/simd.rs`
- [ ] **AC1b.2:** Count total blocks for Day 2 planning
- [ ] **AC1b.3:** Begin classification of first 5 blocks

#### Implementation Notes

SIMD unsafe blocks are architecturally different:
- They use `#[target_feature(enable = "...")]` guards
- Intrinsics are memory-safe when target feature is present
- No alignment issues (SIMD intrinsics handle alignment internally)

Expected classification: Mostly SOUND (when properly guarded)

#### Verification Commands

```bash
# Count SIMD unsafe blocks
grep -cn "unsafe" src/metric/simd.rs

# Verify target_feature guards exist
grep -n "target_feature" src/metric/simd.rs
```

---

## Day 1 Summary

**Total Effort:** 8h (W13.1a: 8h) + 2h (W13.1b: start) = **8h scheduled**

**Note:** W13.1b hours are counted in Day 2 total. Day 1 just starts the SIMD review.

**Deliverables:**
1. ✅ `docs/audits/unsafe_audit_persistence.md`
2. ✅ SAFETY comments in snapshot.rs
3. ✅ SAFETY comments in chunking.rs
4. ✅ W13.1b SIMD audit started
5. ✅ Build and clippy pass

**Carryover to Day 2:**
- W13.1b SIMD audit (4h remaining)

**Blockers Removed:**
- W13.2 can start once W13.1a completes (identifies all casts to replace)

**Status Validation:**
```bash
# Run before end of day
cargo build
cargo clippy -- -D warnings
test -f docs/audits/unsafe_audit_persistence.md && echo "Audit exists"
grep -c "SAFETY:" src/persistence/snapshot.rs
```

---

## HOSTILE_REVIEWER Pre-Flight Checklist

Before submitting Day 1 work for review:

- [ ] All acceptance criteria met for W13.1a (complete)
- [ ] Partial criteria met for W13.1b (start only)
- [ ] Audit document lists all unsafe blocks
- [ ] Each block has SAFETY comment in source
- [ ] Known UB clearly marked as UNSOUND
- [ ] Build succeeds with no regressions
- [ ] No new clippy warnings

---

**PLANNER Notes:**
- Day 1 focuses on "understand the problem" before "fix the problem"
- Audit creates evidence trail for HOSTILE_REVIEWER
- SAFETY comments serve as documentation AND developer warning
- Classification helps prioritize W13.2 implementation order

**Status:** COMPLETE
**Next:** Proceed to W13.2 (bytemuck integration)
