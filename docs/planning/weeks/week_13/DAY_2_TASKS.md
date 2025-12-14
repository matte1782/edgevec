# Week 13 — Day 2 Tasks (Tuesday, Dec 17)

**Date:** 2025-12-17
**Focus:** Complete SIMD Audit + Start bytemuck Integration
**Agent:** RUST_ENGINEER
**Status:** DRAFT

---

## Day Objective

Complete the SIMD unsafe block audit and begin bytemuck integration. By end of day, bytemuck should be added to Cargo.toml and HnswNode should derive Pod/Zeroable.

**Success Criteria:**
- `docs/audits/unsafe_audit_simd.md` complete
- All SIMD unsafe blocks classified
- bytemuck added to Cargo.toml
- HnswNode derives Pod and Zeroable
- `cargo build` succeeds

---

## Tasks

### W13.1b: Unsafe Block Audit - SIMD Module (COMPLETE)

**Priority:** P1
**Estimate:** 4h remaining (6h total - 2h Day 1)
**Agent:** RUST_ENGINEER
**Status:** COMPLETE on Day 2

#### Acceptance Criteria

- [ ] **AC1b.1:** File `docs/audits/unsafe_audit_simd.md` exists
- [ ] **AC1b.2:** All unsafe blocks in `src/metric/simd.rs` listed
- [ ] **AC1b.3:** Each block has target_feature guard verified
- [ ] **AC1b.4:** SIMD blocks classified (expected: SOUND when guarded)
- [ ] **AC1b.5:** Any UNSOUND blocks documented with recommendations
- [ ] **AC1b.6:** Summary includes total block count

#### Implementation Specification

**File to Create:** `docs/audits/unsafe_audit_simd.md`

```markdown
# Unsafe Block Audit: SIMD Module

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-17
**Scope:** `src/metric/simd.rs`

---

## Summary

| Feature | Block Count | Classification |
|:--------|:------------|:---------------|
| wasm_simd128 | X | SOUND (guarded) |
| avx2 | Y | SOUND (guarded) |
| sse2 | Z | SOUND (guarded) |
| Unguarded | ? | NEEDS_REVIEW |

**Total:** XX unsafe blocks

---

## SIMD Safety Model

SIMD intrinsics are `unsafe` because:
1. They require specific CPU features to be present
2. Calling without the feature causes illegal instruction (SIGILL)

**Rust's solution:** `#[target_feature(enable = "...")]` + runtime feature detection

**Our implementation pattern:**
```rust
#[target_feature(enable = "avx2")]
unsafe fn dot_avx2(a: &[f32], b: &[f32]) -> f32 {
    // Safe to call intrinsics here because guard ensures AVX2 is present
    ...
}

// Caller uses runtime detection:
if is_x86_feature_detected!("avx2") {
    unsafe { dot_avx2(a, b) }  // SOUND: guard verified
}
```

---

## Detailed Analysis

### Block 1: [file:line]
...
```

#### Files to Create

- `docs/audits/unsafe_audit_simd.md` (new)

#### Verification Commands

```bash
# Verify audit file exists
test -f docs/audits/unsafe_audit_simd.md && echo "OK"

# Check total unsafe count matches audit
ACTUAL=$(grep -c "unsafe" src/metric/simd.rs || echo 0)
echo "Total unsafe blocks in simd.rs: $ACTUAL"
```

---

### W13.2: Integrate bytemuck Crate (START)

**Priority:** P0 (Critical Path)
**Estimate:** 6h on Day 2 (14h total - 6h remaining for Day 3)
**Agent:** RUST_ENGINEER
**Status:** START Day 2 (6h), COMPLETE Day 3 (8h)

#### Day 2 Scope

- [ ] **AC2.1:** Add bytemuck to Cargo.toml
- [ ] **AC2.2:** Derive Pod and Zeroable for HnswNode
- [ ] **AC2.3:** Verify HnswNode is repr(C)
- [ ] **AC2.4:** Address any padding issues
- [ ] **AC2.5:** `cargo build` succeeds with new derives

#### Implementation Specification

**Step 1: Add Dependency**

```toml
# Cargo.toml
[dependencies]
bytemuck = { version = "1.14", features = ["derive"] }
```

**Step 2: Derive Traits for HnswNode**

```rust
// src/hnsw/graph.rs
use bytemuck::{Pod, Zeroable};

/// HNSW graph node.
///
/// # Safety
/// This struct derives `Pod` and `Zeroable` for safe byte casting.
/// The `#[repr(C)]` ensures deterministic layout for serialization.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct HnswNode {
    pub id: u32,
    pub max_layer: u8,
    pub _padding: [u8; 3],  // Explicit padding for alignment
    pub neighbor_start: u32,
    pub neighbor_counts: [u16; MAX_LAYERS],
}
```

**Pod Requirements:**
1. All fields must implement Pod
2. No padding bytes (or explicit padding)
3. `#[repr(C)]` for deterministic layout
4. No interior mutability (no Cell, RefCell)

**Common Issues:**
- Implicit padding → Add explicit `_padding` field
- Non-Pod fields → Use primitive types or nested Pod structs

#### Verification Commands

```bash
# Verify dependency added
cargo tree | grep bytemuck

# Verify derives compile
cargo build

# Check Pod constraint is satisfied
cargo check 2>&1 | grep -i "pod\|zeroable" || echo "No derive errors"
```

---

## Day 2 Summary

**Total Effort:** 4h (W13.1b complete) + 6h (W13.2 start) = **8h scheduled** (within budget)

**Note:** We budget 2h extra since Day 2 is critical for unblocking W13.3.

**Deliverables:**
1. ✅ `docs/audits/unsafe_audit_simd.md`
2. ✅ bytemuck in Cargo.toml
3. ✅ HnswNode derives Pod/Zeroable
4. ✅ Build succeeds

**Carryover to Day 3:**
- W13.2: Replace unsafe casts (8h remaining)
- W13.2: Add alignment tests
- W13.2: Benchmark overhead

**Blockers Removed:**
- W13.3a can start planning once bytemuck is in (end of Day 2)

**Status Validation:**
```bash
# Run before end of day
cargo build
cargo tree | grep bytemuck
test -f docs/audits/unsafe_audit_simd.md && echo "SIMD audit exists"
grep -c "derive.*Pod" src/hnsw/graph.rs
```

---

## HOSTILE_REVIEWER Pre-Flight Checklist

Before submitting Day 2 work:

- [ ] W13.1b complete (SIMD audit)
- [ ] SIMD blocks classified correctly
- [ ] bytemuck dependency added
- [ ] HnswNode derives compile without error
- [ ] No changes to persistence code yet (Day 3)
- [ ] Build succeeds

---

**PLANNER Notes:**
- Day 2 is the "bridge" between audit and fix
- Getting Pod derive working is critical for Day 3
- If Pod derive fails, investigate HnswNode layout ASAP
- SIMD audit should be straightforward (most blocks are guarded)

**Status:** DRAFT
**Next:** Execute W13.1b completion, W13.2 start
