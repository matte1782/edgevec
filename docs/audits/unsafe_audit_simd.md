# Unsafe Block Audit: SIMD Module

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-13
**Scope:** `src/metric/simd.rs`
**Task:** W13.1b
**Status:** COMPLETE

---

## Executive Summary

The SIMD module contains **6 unsafe blocks** within safe public functions and **6 unsafe functions**. Additionally, there are **2 dispatcher unsafe calls** that invoke unsafe functions from safe contexts. All unsafe code is properly guarded by compile-time `#[cfg(target_feature)]` attributes, ensuring the code is only compiled when the required CPU features are available.

| File | Line | Function | Classification | Risk Level |
|:-----|:-----|:---------|:---------------|:-----------|
| `simd.rs` | 30-96 | `wasm::l2_squared` | **SOUND** | LOW |
| `simd.rs` | 198-247 | `wasm::dot_product` | **SOUND** | LOW |
| `simd.rs` | 339-430 | `wasm::cosine_similarity` | **SOUND** | LOW |
| `simd.rs` | 460-532 | `x86::l2_squared` | **SOUND** | LOW |
| `simd.rs` | 607-662 | `x86::dot_product` | **SOUND** | LOW |
| `simd.rs` | 732-779 | `x86::cosine_similarity` | **SOUND** | LOW |

| Unsafe Functions | Line | Classification | Risk Level |
|:-----------------|:-----|:---------------|:-----------|
| `wasm::l2_squared_u8` | 105-188 | **SOUND** | LOW |
| `wasm::dot_product_u8` | 256-329 | **SOUND** | LOW |
| `x86::l2_squared_u8` | 542-596 | **SOUND** | LOW |
| `x86::dot_product_u8` | 672-718 | **SOUND** | LOW |
| `x86::hsum256_ps_avx` | 784-791 | **SOUND** (helper) | LOW |
| `x86::hsum256_epi32_avx` | 795-811 | **SOUND** (helper) | LOW |

**Recommendation:** No changes required. All SIMD code follows best practices for platform-specific intrinsics.

---

## Module-Level Configuration

**Location:** `src/metric/simd.rs` lines 11-15

```rust
// SIMD code requires intentional pointer casts and alignment handling.
// These lints are disabled at module level as they are false positives for SIMD code.
#![allow(clippy::cast_ptr_alignment)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::missing_panics_doc)]
```

**Justification:** These lint suppressions are **appropriate** for SIMD code:

1. **`cast_ptr_alignment`**: SIMD intrinsics like `v128_load`, `_mm256_loadu_ps` are specifically designed to handle unaligned loads. The "loadu" suffix means "load unaligned."

2. **`ptr_as_ptr`**: SIMD code frequently casts `*const f32` to `*const v128` or `*const __m256`. This is the correct pattern for SIMD operations.

3. **`missing_panics_doc`**: The `assert_eq!` statements in SIMD functions can panic but documenting these is low value since they are standard slice length assertions.

---

## Detailed Analysis

### WASM SIMD128 Module

**Location:** `src/metric/simd.rs` lines 17-432

**Platform Guard:**
```rust
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub mod wasm { ... }
```

This ensures the entire module is only compiled when:
1. Target is `wasm32`
2. SIMD128 feature is enabled

---

### Block 1: `wasm::l2_squared` (SOUND)

**Location:** `src/metric/simd.rs` lines 30-96

**Code Pattern:**
```rust
pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // ... SIMD operations using v128_load, f32x4_* intrinsics ...
    }
}
```

**Operations Performed:**
1. Pointer arithmetic: `a.as_ptr().add(i)` - Safe because `i` is bounded by loop condition `i + 16 <= n`
2. Pointer cast: `as *const v128` - Safe because `v128_load` handles unaligned loads
3. Intrinsic calls: `v128_load`, `f32x4_sub`, `f32x4_mul`, `f32x4_add` - Safe compiler intrinsics
4. `get_unchecked(i)` in scalar tail - Safe because `i < n` is loop condition

**Invariants Required:**
1. `a.len() == b.len()` - Verified by `assert_eq!` at function start
2. Loop index `i` never exceeds `n` - Verified by loop conditions

**Classification:** **SOUND** - All operations are correctly bounded and use appropriate SIMD intrinsics.

---

### Block 2: `wasm::l2_squared_u8` (SOUND)

**Location:** `src/metric/simd.rs` lines 105-188

**Note:** This is an `unsafe fn`, not an unsafe block in a safe function.

```rust
pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 { ... }
```

**Rationale for unsafe fn:** The function performs u8 → i16 extension and uses integer dot products. The caller must ensure slices are valid. The unsafe propagates to callers.

**Operations Performed:** Same pattern as `l2_squared` but with integer types.

**Classification:** **SOUND** - Properly marked `unsafe fn`, operations are correctly bounded.

---

### Block 3: `wasm::dot_product` (SOUND)

**Location:** `src/metric/simd.rs` lines 198-247

**Pattern:** Identical to `l2_squared` but computes `a * b` instead of `(a - b)^2`.

**Classification:** **SOUND** - Same reasoning as Block 1.

---

### Block 4: `wasm::dot_product_u8` (SOUND)

**Location:** `src/metric/simd.rs` lines 256-329

**Pattern:** Identical to `l2_squared_u8` but computes `a * b`.

**Classification:** **SOUND** - Same reasoning as Block 2.

---

### Block 5: `wasm::cosine_similarity` (SOUND)

**Location:** `src/metric/simd.rs` lines 339-430

**Pattern:** Computes three quantities in parallel (dot product, ||a||^2, ||b||^2) then divides.

**Additional Safety:** Division by zero is handled:
```rust
if aa > 0.0 && bb > 0.0 {
    dot / (aa.sqrt() * bb.sqrt())
} else {
    0.0
}
```

**Classification:** **SOUND** - Same reasoning as Block 1, plus safe division handling.

---

### x86 AVX2 Module

**Location:** `src/metric/simd.rs` lines 434-836

**Platform Guard:**
```rust
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub mod x86 { ... }
```

This ensures the entire module is only compiled when:
1. Target is `x86_64`
2. AVX2 feature is enabled

---

### Block 6: `x86::l2_squared` (SOUND)

**Location:** `src/metric/simd.rs` lines 460-532

**Code Pattern:**
```rust
pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // ... AVX2 operations using _mm256_loadu_ps, _mm256_sub_ps, etc ...
    }
}
```

**Operations Performed:**
1. `_mm256_loadu_ps` - Unaligned 256-bit load (safe for any alignment)
2. `_mm256_sub_ps`, `_mm256_mul_ps`, `_mm256_add_ps` - AVX2 float operations
3. `_mm256_fmadd_ps` (optional with FMA feature) - Fused multiply-add
4. `hsum256_ps_avx` - Horizontal sum helper
5. `get_unchecked(i)` in scalar tail - Safe, bounded by loop

**FMA Conditional Compilation:**
```rust
#[cfg(target_feature = "fma")]
{
    sum256 = _mm256_fmadd_ps(diff1, diff1, sum256);
}
#[cfg(not(target_feature = "fma"))]
{
    let sq1 = _mm256_mul_ps(diff1, diff1);
    sum256 = _mm256_add_ps(sum256, sq1);
}
```

This is excellent: FMA is used when available, fallback otherwise.

**Classification:** **SOUND** - Correctly uses unaligned load intrinsics, properly bounded.

---

### Block 7: `x86::l2_squared_u8` (SOUND)

**Location:** `src/metric/simd.rs` lines 542-596

**Pattern:** Uses `_mm_loadu_si128` for unaligned 128-bit loads, then `_mm256_cvtepu8_epi16` to extend to 16-bit integers.

**Classification:** **SOUND** - Properly marked `unsafe fn`, operations correctly bounded.

---

### Block 8: `x86::dot_product` (SOUND)

**Location:** `src/metric/simd.rs` lines 607-662

**Pattern:** Same as `l2_squared` but computes `a * b`.

**Classification:** **SOUND** - Same reasoning as Block 6.

---

### Block 9: `x86::dot_product_u8` (SOUND)

**Location:** `src/metric/simd.rs` lines 672-718

**Pattern:** Same as `l2_squared_u8` but computes `a * b`.

**Classification:** **SOUND** - Same reasoning as Block 7.

---

### Block 10: `x86::cosine_similarity` (SOUND)

**Location:** `src/metric/simd.rs` lines 732-779

**Pattern:** Same as `wasm::cosine_similarity` but with AVX2 intrinsics.

**Classification:** **SOUND** - Same reasoning as Block 5.

---

### Helper Functions

**`hsum256_ps_avx`** (Line 784-791)
```rust
unsafe fn hsum256_ps_avx(v: __m256) -> f32 {
    let x128 = _mm_add_ps(_mm256_extractf128_ps(v, 1), _mm256_castps256_ps128(v));
    let x64 = _mm_add_ps(x128, _mm_movehl_ps(x128, x128));
    let x32 = _mm_add_ss(x64, _mm_shuffle_ps(x64, x64, 0x55));
    _mm_cvtss_f32(x32)
}
```

**Classification:** **SOUND** - Standard horizontal sum pattern for AVX registers.

**`hsum256_epi32_avx`** (Line 795-811)
```rust
unsafe fn hsum256_epi32_avx(v: __m256i) -> u32 {
    let vlow = _mm256_castsi256_si128(v);
    let vhigh = _mm256_extracti128_si256(v, 1);
    let v128 = _mm_add_epi32(vlow, vhigh);
    let v64 = _mm_add_epi32(v128, _mm_shuffle_epi32(v128, 0x4E));
    let v32 = _mm_add_epi32(v64, _mm_shuffle_epi32(v64, 0xB1));
    #[allow(clippy::cast_sign_loss)]
    let result = _mm_cvtsi128_si32(v32) as u32;
    result
}
```

**Note:** The `#[allow(clippy::cast_sign_loss)]` is documented in the code:
```rust
// SAFETY: Sum of u32 values, result is always positive.
// _mm_cvtsi128_si32 returns i32 but our data is logically u32.
```

**Classification:** **SOUND** - Standard horizontal sum for integer registers, cast is correct.

---

## Dispatcher Function

**Location:** `src/metric/simd.rs` lines 843-853

```rust
pub fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
    cfg_if::cfg_if! {
        if #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))] {
            unsafe { wasm::l2_squared_u8(a, b) }
        } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
            unsafe { x86::l2_squared_u8(a, b) }
        } else {
            crate::metric::scalar::l2_squared_u8(a, b)
        }
    }
}
```

**Safety Analysis:**
- Calls `unsafe fn` from safe context
- Safe because `cfg_if` ensures correct platform at compile time
- Falls back to scalar implementation if no SIMD available

**Classification:** **SOUND** - Correct use of compile-time dispatch.

---

## Target Feature Guard Verification

| Module | Guard | Verified |
|:-------|:------|:---------|
| `wasm` | `#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]` | ✅ |
| `x86` | `#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]` | ✅ |
| Dispatcher | `cfg_if!` with same guards | ✅ |

All modules are properly gated to prevent compilation on incorrect platforms.

---

## Intrinsic Safety Analysis

### WASM Intrinsics Used

| Intrinsic | Purpose | Alignment Requirement |
|:----------|:--------|:---------------------|
| `v128_load` | Load 128 bits | **Unaligned** (safe) |
| `f32x4_*` | f32x4 operations | N/A (register ops) |
| `i32x4_*` | i32x4 operations | N/A (register ops) |
| `i16x8_*` | i16x8 operations | N/A (register ops) |
| `*_extract_lane` | Extract scalar | N/A (register ops) |

### x86 Intrinsics Used

| Intrinsic | Purpose | Alignment Requirement |
|:----------|:--------|:---------------------|
| `_mm256_loadu_ps` | Load 8 floats | **Unaligned** (u = unaligned) |
| `_mm_loadu_si128` | Load 128 bits | **Unaligned** (u = unaligned) |
| `_mm256_setzero_*` | Zero vector | N/A (immediate) |
| `_mm256_sub_ps` | Subtract floats | N/A (register ops) |
| `_mm256_mul_ps` | Multiply floats | N/A (register ops) |
| `_mm256_add_ps` | Add floats | N/A (register ops) |
| `_mm256_fmadd_ps` | Fused multiply-add | N/A (register ops) |
| `_mm256_cvtepu8_epi16` | u8 → i16 extend | N/A (register ops) |
| `_mm256_madd_epi16` | Multiply-add i16 | N/A (register ops) |

**Key Observation:** All load operations use the **unaligned** variants (`loadu`). This is deliberate and correct since slice pointers have no alignment guarantees.

---

## Recommendations

### No Changes Required

All unsafe code in the SIMD module is:
1. **Correctly bounded** - Loop indices respect slice lengths
2. **Properly guarded** - `#[cfg(target_feature)]` ensures correct platform
3. **Using correct intrinsics** - Unaligned loads for arbitrary slice data
4. **Well documented** - Module-level safety comment explains lint suppressions

### Optional Enhancements (Future Work)

1. **Runtime feature detection**: Consider `is_x86_feature_detected!()` for runtime dispatch instead of compile-time only (would allow single binary to use AVX2 when available)

2. **NEON support**: ARM NEON implementation could be added for mobile/M1 platforms

---

## Verification Commands

```bash
# Verify unsafe block count (should find blocks in expected locations)
grep -n "unsafe {" src/metric/simd.rs

# Verify unsafe fn count
grep -n "pub unsafe fn" src/metric/simd.rs

# Verify target_feature guards
grep -n "target_feature" src/metric/simd.rs

# Build and test (native)
cargo build
cargo test --lib

# Check WASM compilation (requires wasm32 target)
# cargo build --target wasm32-unknown-unknown
```

---

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|:----------|:-------|:---------|
| AC13.1b.1: All unsafe blocks identified | ✅ | 6 unsafe blocks, 6 unsafe functions documented |
| AC13.1b.2: target_feature guards verified | ✅ | Both modules have correct `#[cfg]` attributes |
| AC13.1b.3: Each block classified | ✅ | All classified as SOUND |
| AC13.1b.4: Intrinsic alignment verified | ✅ | All loads use unaligned variants |

---

## Audit Conclusion

The SIMD module contains **6 unsafe blocks** and **6 unsafe functions**, all classified as **SOUND**:

1. **Proper platform guards** - All code is `#[cfg]`-gated
2. **Correct intrinsics** - Unaligned loads used throughout
3. **Bounded operations** - All index access is within slice bounds
4. **Good documentation** - Module-level comments explain lint suppressions

**No remediation required.** The SIMD code follows Rust best practices for platform-specific intrinsics.

---

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-13
**Status:** AUDIT COMPLETE — All SIMD unsafe code is SOUND
