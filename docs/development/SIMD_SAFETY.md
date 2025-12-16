# SIMD Safety Audit

**Document Version:** 1.0.0
**Date:** 2025-12-16
**Author:** EdgeVec Development Team
**Week:** 20 (ARM/NEON Implementation Sprint)

---

## Overview

This document provides a comprehensive safety audit of all `unsafe` code blocks in the EdgeVec SIMD implementation. Each unsafe block is documented with:

1. **Location** - File and line number
2. **Operation** - What the unsafe code does
3. **Justification** - Why the usage is safe
4. **Invariants** - What conditions must hold
5. **Verification** - How safety is verified

---

## Unsafe Code Inventory

### 1. NEON Hamming Distance (`src/simd/neon.rs`)

#### Location: `hamming_distance_neon_unchecked`

**File:** `src/simd/neon.rs`
**Lines:** 112-149

```rust
#[inline]
#[target_feature(enable = "neon")]
unsafe fn hamming_distance_neon_unchecked(a: &[u8], b: &[u8]) -> u32 {
    // ... implementation
}
```

**Operation:** Loads 16 bytes at a time using NEON intrinsics (`vld1q_u8`) for vectorized XOR and popcount operations.

**Unsafe Operations:**

| Operation | Intrinsic | Risk |
|:----------|:----------|:-----|
| Load 16 bytes from `a` | `vld1q_u8(a.as_ptr().add(offset))` | Out-of-bounds read |
| Load 16 bytes from `b` | `vld1q_u8(b.as_ptr().add(offset))` | Out-of-bounds read |
| XOR vectors | `veorq_u8(va, vb)` | None (pure computation) |
| Popcount | `vcntq_u8(xor)` | None (pure computation) |
| Horizontal sum | `vaddlvq_u8(bit_counts)` | None (pure computation) |

**Justification:**

1. **Bounds Safety:** The chunk calculation `chunks = len / 16` guarantees that for each iteration `i < chunks`, the read of 16 bytes starting at `offset = i * 16` stays within bounds:
   - `offset + 16 = i * 16 + 16 ≤ (chunks - 1) * 16 + 16 = chunks * 16 ≤ len`

2. **Alignment:** NEON `vld1q_u8` supports unaligned loads, so no alignment requirements.

3. **NEON Availability:** The `#[target_feature(enable = "neon")]` attribute ensures the function is only called when NEON is available. The public wrapper `hamming_distance_slice` is only compiled on `aarch64` where NEON is mandatory.

**Invariants:**

| Invariant | Enforcement |
|:----------|:------------|
| `a.len() == b.len()` | `assert_eq!` in public wrapper |
| NEON available | Compile-time (`#[target_feature(enable = "neon")]`) |
| Read within bounds | Chunk calculation guarantees `offset + 16 ≤ len` |

**Verification:**

1. **Property Tests:** 1000+ random test cases verify NEON matches portable exactly
2. **Edge Case Tests:** Empty, 1-byte, 15-byte, 16-byte, 17-byte inputs tested
3. **ARM CI:** Tests run on ARM64 via QEMU emulation

---

## Safe Wrapper Pattern

All unsafe NEON code follows the **Safe Wrapper Pattern**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│   PUBLIC API (Safe)                                                     │
│   hamming_distance_slice(a: &[u8], b: &[u8]) -> u32                    │
│   ├── Validates: a.len() == b.len() (panic if not)                     │
│   └── Calls: unsafe { hamming_distance_neon_unchecked(a, b) }          │
├─────────────────────────────────────────────────────────────────────────┤
│   INTERNAL (Unsafe)                                                     │
│   hamming_distance_neon_unchecked(a: &[u8], b: &[u8]) -> u32           │
│   ├── Requires: a.len() == b.len() (debug_assert)                      │
│   ├── Bounds: chunk calculation ensures in-bounds reads                │
│   └── NEON: #[target_feature(enable = "neon")] ensures availability    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Memory Safety Analysis

### Read-Only Operations

All NEON operations in EdgeVec are **read-only**:

- `vld1q_u8`: Loads 16 bytes from memory (read)
- `veorq_u8`: XOR operation on registers (no memory access)
- `vcntq_u8`: Popcount on registers (no memory access)
- `vaddlvq_u8`: Horizontal sum on registers (no memory access)

No write operations to input slices are performed.

### Pointer Arithmetic Safety

```rust
let offset = i * 16;
let va = vld1q_u8(a.as_ptr().add(offset));
```

**Safety Proof:**

Given:
- `chunks = len / 16` (integer division)
- Loop: `for i in 0..chunks`

For any iteration:
- `i ∈ [0, chunks)`
- `offset = i * 16 ∈ [0, (chunks-1) * 16]`
- `offset + 16 ≤ (chunks-1) * 16 + 16 = chunks * 16 ≤ len` ✓

---

## Undefined Behavior Prevention

### UB Vectors Considered

| UB Vector | Prevention |
|:----------|:-----------|
| Out-of-bounds read | Chunk calculation bounds proof |
| Misaligned read | NEON supports unaligned (vld1q) |
| Data race | Single-threaded, immutable refs |
| Invalid pointer | Slices guarantee valid ptrs |
| Type confusion | Strongly typed intrinsics |

---

## Test Coverage

### Property Tests (`tests/simd_neon_hamming.rs`)

| Property | Cases | Platform |
|:---------|:------|:---------|
| NEON == Portable | 1000 | ARM64 |
| Symmetric | 1000 | All |
| Self distance = 0 | 1000 | All |
| Bounded by bits | 1000 | All |
| Manual calculation match | 1000 | All |

### Edge Case Tests

| Case | Description | Platform |
|:-----|:------------|:---------|
| Empty slices | len = 0 | All |
| 1 byte | Scalar only | All |
| 15 bytes | Below NEON width | All |
| 16 bytes | Exact NEON width | All |
| 17 bytes | NEON + 1 tail | All |
| 1000 bytes | Large input | All |

---

## Clippy Compliance

The codebase passes clippy with the `undocumented_unsafe_blocks` lint:

```bash
cargo clippy -- -W clippy::undocumented_unsafe_blocks
```

All unsafe blocks have `// SAFETY:` comments explaining the justification.

---

## Future Work

### Day 4: NEON Dot Product

Will follow the same safety pattern:
- Safe public wrapper with validation
- Unsafe internal function with `#[target_feature(enable = "neon")]`
- Bounds checking via chunk calculation
- `// SAFETY:` comments on all unsafe blocks

### Day 5: NEON Euclidean Distance

Same pattern as dot product.

---

## Approval

This safety audit covers the NEON hamming distance implementation completed in Week 20 Day 3.

**Auditor:** EdgeVec Development Team
**Date:** 2025-12-16
**Status:** APPROVED (pending hostile review)

---

## References

1. [ARM NEON Intrinsics Reference](https://developer.arm.com/architectures/instruction-sets/intrinsics/)
2. [Rust `target_feature` Documentation](https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute)
3. [Rustonomicon - Unsafe Code Guidelines](https://doc.rust-lang.org/nomicon/)
