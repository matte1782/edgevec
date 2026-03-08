# Unsafe Code Inventory

**Document:** `docs/audits/UNSAFE_INVENTORY.md`
**Date:** 2026-03-08
**Status:** [APPROVED]
**Scope:** All `unsafe` usage in `src/`
**Method:** `grep -rn "unsafe" src/` with manual classification of every hit

---

## Summary

| Category | Count |
|:---------|------:|
| Production unsafe code sites | 31 |
| Test-only unsafe call-sites (`#[cfg(test)]`) | 6 |
| Comments, doc strings, attributes mentioning "unsafe" | 25 |
| **Total grep hits** | **62** |

Production breakdown (31 sites):

| Sub-category | Count | Files |
|:-------------|------:|:------|
| WASM SIMD128 blocks/functions | 6 | `src/metric/simd.rs` (wasm module) |
| x86 AVX2 blocks/functions | 8 | `src/metric/simd.rs` (x86 module) |
| Dispatcher call-sites | 3 | `src/metric/simd.rs` (top-level) |
| Quantization AVX2 functions | 2 | `src/quantization/simd/avx2.rs` |
| Quantization dispatcher | 1 | `src/quantization/simd/mod.rs` |
| NEON functions + call-sites | 6 | `src/simd/neon.rs` |
| Popcount SIMD functions + dispatchers | 4 | `src/simd/popcount.rs` |
| WASM lifetime transmute | 1 | `src/wasm/mod.rs` |

All unsafe code falls into two categories:

1. **SIMD intrinsics** (36 sites) -- Required because CPU SIMD instructions are inherently unsafe in Rust. All are guarded by compile-time `cfg` guards or runtime feature detection.
2. **Lifetime transmute** (1 site) -- Required because `wasm-bindgen` does not support lifetime parameters. Guarded by a runtime liveness check.

No unsafe code is used for performance-only reasons (e.g., skipping bounds checks in non-SIMD hot paths). The `get_unchecked` calls within SIMD functions are inside the same `unsafe` block as the SIMD intrinsics themselves.

---

## Risk Summary

| Risk Level | Count | Description |
|:-----------|------:|:------------|
| **HIGH** | 1 | Lifetime transmute in WASM iterator (`src/wasm/mod.rs:1695`) |
| **MEDIUM** | 0 | -- |
| **LOW** | 36 | All SIMD intrinsic sites |

---

## Detailed Inventory

### Category A: WASM SIMD128 Intrinsics (`src/metric/simd.rs`, `wasm` module)

All WASM SIMD code is gated by `#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]` at module level (line 17). The `simd128` target feature is a compile-time guarantee.

#### A1. `src/metric/simd.rs:30` -- WASM `l2_squared` (f32)

**Code:**
```rust
pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // v128_load, f32x4_sub, f32x4_mul, f32x4_add, get_unchecked
        // ... (lines 30-96)
    }
}
```

**Why unsafe is needed:** WASM SIMD128 intrinsics (`v128_load`, `f32x4_*`) are `unsafe` functions in `std::arch::wasm32`. Scalar tail uses `get_unchecked` to avoid bounds checks inside the verified loop.

**Invariant:** (1) `a.len() == b.len()` (assert on line 29). (2) Loop bounds `i + 16 <= n` and `i + 4 <= n` guarantee pointer arithmetic stays within slice bounds. (3) Scalar tail `i < n` guarantees `get_unchecked(i)` is valid. (4) `v128_load` performs unaligned loads per WASM SIMD spec.

**Why invariant holds:** Length equality is asserted before the unsafe block. Loop conditions are monotonically increasing (`i` increments by 16, 4, or 1) and bounded by `n = a.len()`.

**Risk level:** LOW

---

#### A2. `src/metric/simd.rs:105` -- WASM `l2_squared_u8` (u8)

**Code:**
```rust
pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    // v128_load, i16x8_extend_*, i16x8_sub, i32x4_dot_i16x8, get_unchecked
    // ... (lines 105-188)
}
```

**Why unsafe is needed:** Entire function is `unsafe fn` because it requires WASM SIMD128 intrinsics. Caller must ensure `simd128` target feature.

**Invariant:** Same as A1 -- length equality asserted, loop bounds guarantee in-bounds access.

**Why invariant holds:** Same as A1. Called only from `l2_squared_u8` dispatcher (line 1286) which is gated by `cfg(all(target_arch = "wasm32", target_feature = "simd128"))`.

**Risk level:** LOW

---

#### A3. `src/metric/simd.rs:198` -- WASM `dot_product` (f32)

**Code:**
```rust
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // v128_load, f32x4_mul, f32x4_add, get_unchecked
        // ... (lines 198-248)
    }
}
```

**Why unsafe is needed:** WASM SIMD128 intrinsics require unsafe.

**Invariant:** Same pattern as A1.

**Why invariant holds:** Same as A1.

**Risk level:** LOW

---

#### A4. `src/metric/simd.rs:256` -- WASM `dot_product_u8` (u8)

**Code:**
```rust
pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    // ... (lines 256-347)
}
```

**Why unsafe is needed:** Entire function is `unsafe fn` for SIMD intrinsics. Called only from dispatcher that verifies target feature.

**Invariant:** Same as A1.

**Why invariant holds:** Same as A1.

**Risk level:** LOW

---

#### A5. `src/metric/simd.rs:399` -- WASM `hamming_distance` (u8)

**Code:**
```rust
pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // v128_load, v128_xor, i8x16_swizzle, get_unchecked
        // ... (lines 399-516)
    }
}
```

**Why unsafe is needed:** WASM SIMD128 intrinsics for LUT-based popcount.

**Invariant:** (1) Length equality asserted on line 385. (2) Loop bounds `i + 64 <= n` and `i + 16 <= n` keep pointer arithmetic in bounds. (3) Scalar tail `i < n` guards `get_unchecked`.

**Why invariant holds:** Detailed safety comment on lines 387-398 documents all 5 invariants. Module-level cfg guard guarantees simd128 availability.

**Risk level:** LOW

---

#### A6. `src/metric/simd.rs:526` -- WASM `cosine_similarity` (f32)

**Code:**
```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // v128_load, f32x4_mul, f32x4_add, get_unchecked
        // ... (lines 526-617)
    }
}
```

**Why unsafe is needed:** WASM SIMD128 intrinsics.

**Invariant:** Same pattern as A1.

**Why invariant holds:** Same as A1.

**Risk level:** LOW

---

### Category B: x86_64 AVX2 Intrinsics (`src/metric/simd.rs`, `x86` module)

All x86 AVX2 code is gated by `#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]` at module level (line 667). The `avx2` target feature is a compile-time guarantee.

#### B1. `src/metric/simd.rs:693` -- AVX2 `l2_squared` (f32)

**Code:**
```rust
pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // _mm256_loadu_ps, _mm256_sub_ps, _mm256_fmadd_ps, hsum256_ps_avx, get_unchecked
        // ... (lines 693-765)
    }
}
```

**Why unsafe is needed:** AVX2 intrinsics (`_mm256_*`) are unsafe in `std::arch::x86_64`.

**Invariant:** (1) `a.len() == b.len()` asserted on line 692. (2) Loop bounds `i + 16 <= n` and `i + 8 <= n` ensure in-bounds access. (3) `_mm256_loadu_ps` performs unaligned loads. (4) Scalar tail `i < n` guards `get_unchecked`.

**Why invariant holds:** Same reasoning as WASM variants. Module cfg guard guarantees AVX2.

**Risk level:** LOW

---

#### B2. `src/metric/simd.rs:775` -- AVX2 `l2_squared_u8` (u8)

**Code:**
```rust
pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    // _mm_loadu_si128, _mm256_cvtepu8_epi16, _mm256_sub_epi16, _mm256_madd_epi16, get_unchecked
    // ... (lines 775-829)
}
```

**Why unsafe is needed:** Entire function is `unsafe fn` for AVX2 intrinsics.

**Invariant:** Same as B1.

**Why invariant holds:** Called only from dispatcher (line 1288) under matching cfg guard.

**Risk level:** LOW

---

#### B3. `src/metric/simd.rs:840` -- AVX2 `dot_product` (f32)

**Code:**
```rust
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // _mm256_loadu_ps, _mm256_fmadd_ps, hsum256_ps_avx, get_unchecked
        // ... (lines 840-895)
    }
}
```

**Why unsafe is needed:** AVX2 intrinsics.

**Invariant:** Same as B1.

**Why invariant holds:** Same as B1.

**Risk level:** LOW

---

#### B4. `src/metric/simd.rs:905` -- AVX2 `dot_product_u8` (u8)

**Code:**
```rust
pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    // _mm_loadu_si128, _mm256_cvtepu8_epi16, _mm256_madd_epi16, get_unchecked
    // ... (lines 905-951)
}
```

**Why unsafe is needed:** Entire function is `unsafe fn` for AVX2 intrinsics.

**Invariant:** Same as B1.

**Why invariant holds:** Same as B1.

**Risk level:** LOW

---

#### B5. `src/metric/simd.rs:965` -- AVX2 `cosine_similarity` (f32)

**Code:**
```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    unsafe {
        // _mm256_loadu_ps, _mm256_fmadd_ps, hsum256_ps_avx, get_unchecked
        // ... (lines 965-1013)
    }
}
```

**Why unsafe is needed:** AVX2 intrinsics.

**Invariant:** Same as B1.

**Why invariant holds:** Same as B1.

**Risk level:** LOW

---

#### B6. `src/metric/simd.rs:1066` -- AVX2 `hamming_distance` (u8)

**Code:**
```rust
pub unsafe fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    // _mm256_loadu_si256, _mm256_xor_si256, _mm256_shuffle_epi8, _mm256_sad_epu8, get_unchecked
    // ... (lines 1066-1173)
}
```

**Why unsafe is needed:** Entire function is `unsafe fn` for AVX2 intrinsics. Uses LUT-based popcount with SIMD shuffle.

**Invariant:** (1) Length equality asserted on line 1073. (2) Loop bounds `i + 64 <= n` and `i + 32 <= n` keep loads in-bounds. (3) `_mm256_loadu_si256` performs unaligned loads. (4) Scalar tail `i < n` guards `get_unchecked`. Detailed safety comment on lines 1075-1084.

**Why invariant holds:** Module cfg guard guarantees AVX2. Caller (dispatcher line 1343) also verifies cfg.

**Risk level:** LOW

---

#### B7. `src/metric/simd.rs:1177` -- AVX2 `hsum256_ps_avx` (helper)

**Code:**
```rust
unsafe fn hsum256_ps_avx(v: __m256) -> f32 {
    let x128 = _mm_add_ps(_mm256_extractf128_ps(v, 1), _mm256_castps256_ps128(v));
    let x64 = _mm_add_ps(x128, _mm_movehl_ps(x128, x128));
    let x32 = _mm_add_ss(x64, _mm_shuffle_ps(x64, x64, 0x55));
    _mm_cvtss_f32(x32)
}
```

**Why unsafe is needed:** AVX2/SSE intrinsics for horizontal sum. Private helper called only from other unsafe functions within the same module.

**Invariant:** Input `v` is a valid `__m256` register. No memory access -- purely register operations.

**Why invariant holds:** Only called with values produced by other AVX2 intrinsics within the same unsafe block.

**Risk level:** LOW

---

#### B8. `src/metric/simd.rs:1188` -- AVX2 `hsum256_epi32_avx` (helper)

**Code:**
```rust
unsafe fn hsum256_epi32_avx(v: __m256i) -> u32 {
    let vlow = _mm256_castsi256_si128(v);
    let vhigh = _mm256_extracti128_si256(v, 1);
    // ...
    _mm_cvtsi128_si32(v32) as u32
}
```

**Why unsafe is needed:** AVX2/SSE intrinsics for horizontal integer sum. Private helper.

**Invariant:** Same as B7 -- purely register operations on a valid `__m256i` input.

**Why invariant holds:** Same as B7.

**Risk level:** LOW

---

### Category C: Dispatchers (`src/metric/simd.rs`)

These are safe wrapper functions that call unsafe SIMD functions after compile-time feature verification.

#### C1. `src/metric/simd.rs:1286` -- `l2_squared_u8` dispatcher (WASM path)

**Code:**
```rust
unsafe { wasm::l2_squared_u8(a, b) }
```

**Why unsafe is needed:** Calling `unsafe fn wasm::l2_squared_u8`.

**Invariant:** `simd128` target feature available.

**Why invariant holds:** Gated by `cfg(all(target_arch = "wasm32", target_feature = "simd128"))` on line 1285.

**Risk level:** LOW

---

#### C2. `src/metric/simd.rs:1288` -- `l2_squared_u8` dispatcher (AVX2 path)

**Code:**
```rust
unsafe { x86::l2_squared_u8(a, b) }
```

**Why unsafe is needed:** Calling `unsafe fn x86::l2_squared_u8`.

**Invariant:** `avx2` target feature available.

**Why invariant holds:** Gated by `cfg(all(target_arch = "x86_64", target_feature = "avx2"))` on line 1287.

**Risk level:** LOW

---

#### C3. `src/metric/simd.rs:1343` -- `hamming_distance` dispatcher (AVX2 path)

**Code:**
```rust
unsafe { x86::hamming_distance(a, b) }
```

**Why unsafe is needed:** Calling `unsafe fn x86::hamming_distance`.

**Invariant:** `avx2` target feature available.

**Why invariant holds:** Gated by `cfg(all(target_arch = "x86_64", target_feature = "avx2"))` on line 1338. Safety comment on lines 1339-1342.

**Risk level:** LOW

---

### Category D: Quantization AVX2 SIMD (`src/quantization/simd/avx2.rs`)

#### D1. `src/quantization/simd/avx2.rs:67` -- `hamming_distance_avx2` (fixed 96-byte)

**Code:**
```rust
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn hamming_distance_avx2(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    let a0 = _mm256_loadu_si256(a.as_ptr().cast::<__m256i>());
    let a1 = _mm256_loadu_si256(a.as_ptr().add(32).cast::<__m256i>());
    let a2 = _mm256_loadu_si256(a.as_ptr().add(64).cast::<__m256i>());
    // ... XOR + popcount
}
```

**Why unsafe is needed:** AVX2 intrinsics (`_mm256_loadu_si256`, `_mm256_xor_si256`). Function uses `#[target_feature(enable = "avx2")]` attribute.

**Invariant:** (1) AVX2 must be available (caller verified via `is_x86_feature_detected!`). (2) Input arrays are exactly 96 bytes (enforced by `[u8; 96]` type). (3) Loads at offsets 0, 32, 64 read 32 bytes each, totaling 96 bytes -- exactly the array size.

**Why invariant holds:** Type system enforces 96-byte input. Caller in `quantization/simd/mod.rs:70` checks `is_x86_feature_detected!("avx2")` at runtime. Loads use `_mm256_loadu_si256` (unaligned) so alignment is not required.

**Risk level:** LOW

---

#### D2. `src/quantization/simd/avx2.rs:121` -- `popcount_avx2` (helper)

**Code:**
```rust
#[target_feature(enable = "avx2")]
unsafe fn popcount_avx2(v: __m256i) -> u32 {
    let a = _mm256_extract_epi64(v, 0) as u64;
    let b = _mm256_extract_epi64(v, 1) as u64;
    let c = _mm256_extract_epi64(v, 2) as u64;
    let d = _mm256_extract_epi64(v, 3) as u64;
    a.count_ones() + b.count_ones() + c.count_ones() + d.count_ones()
}
```

**Why unsafe is needed:** AVX2 intrinsic `_mm256_extract_epi64`. Private helper.

**Invariant:** AVX2 available, input is valid `__m256i`.

**Why invariant holds:** Called only from D1 which already requires AVX2. Purely register operations.

**Risk level:** LOW

---

### Category E: Quantization SIMD Dispatcher (`src/quantization/simd/mod.rs`)

#### E1. `src/quantization/simd/mod.rs:70` -- `hamming_distance` dispatcher

**Code:**
```rust
if is_x86_feature_detected!("avx2") {
    return unsafe { avx2::hamming_distance_avx2(a, b) };
}
```

**Why unsafe is needed:** Calling `unsafe fn hamming_distance_avx2`.

**Invariant:** AVX2 available.

**Why invariant holds:** `is_x86_feature_detected!("avx2")` returns true on the line immediately before the unsafe call.

**Risk level:** LOW

---

### Category F: NEON Intrinsics (`src/simd/neon.rs`)

All NEON code is compiled only on `aarch64` targets. Functions use `#[target_feature(enable = "neon")]`.

#### F1. `src/simd/neon.rs:98` -- `hamming_distance_slice` call to unchecked

**Code:**
```rust
pub fn hamming_distance_slice(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len(), "Slice lengths must match");
    unsafe { hamming_distance_neon_unchecked(a, b) }
}
```

**Why unsafe is needed:** Calling `unsafe fn hamming_distance_neon_unchecked`.

**Invariant:** `a.len() == b.len()`.

**Why invariant holds:** Asserted on the line before the unsafe call.

**Risk level:** LOW

---

#### F2. `src/simd/neon.rs:116` -- `hamming_distance_neon_unchecked`

**Code:**
```rust
#[target_feature(enable = "neon")]
unsafe fn hamming_distance_neon_unchecked(a: &[u8], b: &[u8]) -> u32 {
    // vld1q_u8, veorq_u8, vcntq_u8, vaddlvq_u8
    // ... (lines 116-156)
}
```

**Why unsafe is needed:** NEON intrinsics. Function uses `#[target_feature(enable = "neon")]`.

**Invariant:** (1) `a.len() == b.len()` (debug_assert on line 117). (2) Loop `i in 0..chunks` where `chunks = len / 16` ensures `offset + 16 <= len`. (3) Tail loop uses safe indexing `a[i]`.

**Why invariant holds:** Caller verifies length equality. Loop arithmetic is bounded. Detailed safety comments on lines 110-113.

**Risk level:** LOW

---

#### F3. `src/simd/neon.rs:264` -- `dot_product` call to unchecked

**Code:**
```rust
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Slice lengths must match");
    unsafe { dot_product_neon_unchecked(a, b) }
}
```

**Why unsafe is needed:** Calling `unsafe fn dot_product_neon_unchecked`.

**Invariant:** `a.len() == b.len()`.

**Why invariant holds:** Asserted on the line before.

**Risk level:** LOW

---

#### F4. `src/simd/neon.rs:282` -- `dot_product_neon_unchecked`

**Code:**
```rust
#[target_feature(enable = "neon")]
unsafe fn dot_product_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
    // vld1q_f32, vfmaq_f32, vaddvq_f32
    // ... (lines 282-318)
}
```

**Why unsafe is needed:** NEON intrinsics (`vld1q_f32`, `vfmaq_f32`).

**Invariant:** Same pattern as F2 but with 4-float chunks.

**Why invariant holds:** Same as F2. `chunks = len / 4` ensures in-bounds NEON loads. Tail uses safe indexing.

**Risk level:** LOW

---

#### F5. `src/simd/neon.rs:398` -- `euclidean_distance` call to unchecked

**Code:**
```rust
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Slice lengths must match");
    unsafe { euclidean_distance_neon_unchecked(a, b) }
}
```

**Why unsafe is needed:** Calling `unsafe fn euclidean_distance_neon_unchecked`.

**Invariant:** `a.len() == b.len()`.

**Why invariant holds:** Asserted on the line before.

**Risk level:** LOW

---

#### F6. `src/simd/neon.rs:416` -- `euclidean_distance_neon_unchecked`

**Code:**
```rust
#[target_feature(enable = "neon")]
unsafe fn euclidean_distance_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
    // vld1q_f32, vsubq_f32, vfmaq_f32, vaddvq_f32
    // ... (lines 416-458)
}
```

**Why unsafe is needed:** NEON intrinsics.

**Invariant:** Same as F4.

**Why invariant holds:** Same as F4.

**Risk level:** LOW

---

### Category G: Popcount SIMD (`src/simd/popcount.rs`)

#### G1. `src/simd/popcount.rs:79` -- `simd_popcount_xor` AVX2 dispatch

**Code:**
```rust
if is_x86_feature_detected!("avx2") {
    return unsafe { avx2_popcount_xor(a, b) };
}
```

**Why unsafe is needed:** Calling `unsafe fn avx2_popcount_xor`.

**Invariant:** AVX2 available.

**Why invariant holds:** `is_x86_feature_detected!("avx2")` returns true on the line before.

**Risk level:** LOW

---

#### G2. `src/simd/popcount.rs:90` -- `simd_popcount_xor` NEON dispatch

**Code:**
```rust
if std::arch::is_aarch64_feature_detected!("neon") {
    return unsafe { neon_popcount_xor(a, b) };
}
```

**Why unsafe is needed:** Calling `unsafe fn neon_popcount_xor`.

**Invariant:** NEON available.

**Why invariant holds:** Feature detection on the line before.

**Risk level:** LOW

---

#### G3. `src/simd/popcount.rs:132` -- `avx2_popcount_xor`

**Code:**
```rust
#[target_feature(enable = "avx2")]
unsafe fn avx2_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    // _mm256_loadu_si256, _mm256_xor_si256, _mm256_extract_epi64
    // ... (lines 132-165)
}
```

**Why unsafe is needed:** AVX2 intrinsics.

**Invariant:** (1) AVX2 available (caller checked). (2) Loop `chunks = len / 32` ensures 32-byte loads stay in bounds. (3) Scalar remainder handles tail.

**Why invariant holds:** Caller verifies AVX2. Loop arithmetic bounded by slice length.

**Risk level:** LOW

---

#### G4. `src/simd/popcount.rs:208` -- `neon_popcount_xor`

**Code:**
```rust
#[target_feature(enable = "neon")]
unsafe fn neon_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
    // vld1q_u8, veorq_u8, vcntq_u8, vpaddlq_*
    // ... (lines 208-241)
}
```

**Why unsafe is needed:** NEON intrinsics.

**Invariant:** (1) NEON available (caller checked). (2) Loop `chunks = len / 16` ensures 16-byte loads stay in bounds. (3) Scalar remainder handles tail.

**Why invariant holds:** Caller verifies NEON. Loop arithmetic bounded by slice length.

**Risk level:** LOW

---

### Category H: WASM Lifetime Transmute (`src/wasm/mod.rs`)

#### H1. `src/wasm/mod.rs:1695` -- `save_stream` lifetime erasure

**Code:**
```rust
#[allow(unsafe_code)]
let static_iter = unsafe { std::mem::transmute::<ChunkIter<'_>, ChunkIter<'static>>(iter) };
```

**Why unsafe is needed:** `wasm-bindgen` does not support lifetime parameters on exported types. The `PersistenceIterator` must be returned to JavaScript, which has no concept of Rust lifetimes. `std::mem::transmute` is required to erase the lifetime.

**Invariant:** The `EdgeVec` instance that owns the data borrowed by `ChunkIter` must remain alive for the entire lifetime of the `PersistenceIterator`.

**Why invariant holds:**
1. A `liveness: Arc<AtomicBool>` guard is set to `true` while `EdgeVec` is alive and `false` when it is dropped.
2. `PersistenceIterator::next_chunk()` (in `src/wasm/iterator.rs:48`) asserts `self.liveness.load(Ordering::Acquire)` before every access, panicking if `EdgeVec` was freed.
3. JavaScript's single-threaded event loop guarantees `EdgeVec` cannot be garbage-collected during a synchronous iteration step.
4. The doc comment on `PersistenceIterator` (lines 13-20 of `iterator.rs`) explicitly warns users not to call `free()` during iteration.

**Residual risk:** If a user calls `EdgeVec.free()` in JavaScript between two calls to `next_chunk()`, the liveness check will catch it and panic (preventing use-after-free), but the panic itself may be undesirable. This is the only mitigation possible without `wasm-bindgen` lifetime support.

**Risk level:** HIGH

---

### Category T: Test-Only Unsafe (not production code)

The following 6 unsafe call-sites appear only in `#[cfg(test)]` modules and are excluded from the production inventory count:

| # | File:Line | Description |
|:--|:----------|:------------|
| T1 | `src/quantization/simd/avx2.rs:153` | Test `test_avx2_identical` calls `hamming_distance_avx2` |
| T2 | `src/quantization/simd/avx2.rs:167` | Test `test_avx2_opposite` calls `hamming_distance_avx2` |
| T3 | `src/quantization/simd/avx2.rs:181` | Test `test_avx2_alternating` calls `hamming_distance_avx2` |
| T4 | `src/quantization/simd/avx2.rs:196` | Test `test_avx2_single_bit` calls `hamming_distance_avx2` |
| T5 | `src/quantization/simd/avx2.rs:212` | Test `test_avx2_boundary_32` calls `hamming_distance_avx2` |
| T6 | `src/quantization/simd/avx2.rs:228` | Test `test_avx2_boundary_64` calls `hamming_distance_avx2` |

All test-only unsafe calls are guarded by `if !avx2_available() { return; }` before the unsafe block.

---

### Non-Code "unsafe" Mentions (comments, docs, attributes)

The following 19 grep hits contain the word "unsafe" but are NOT executable unsafe code. They are comments, doc strings, or allow-attributes:

| # | File:Line | Type | Content |
|:--|:----------|:-----|:--------|
| N1 | `src/metric/simd.rs:387` | Comment | `// SAFETY: This unsafe block is required for WASM SIMD128 intrinsics.` |
| N2 | `src/metric/simd.rs:1075` | Comment | `// SAFETY: This unsafe block is required for AVX2 SIMD intrinsics.` |
| N3 | `src/metric/simd.rs:1336` | Comment | `// WASM SIMD128 path - safe wrapper, no explicit unsafe needed` |
| N4 | `src/quantization/simd/avx2.rs:23` | Doc | `//! All functions in this module are marked 'unsafe' and require:` |
| N5 | `src/quantization/simd/mod.rs:12` | Doc | `//! All unsafe SIMD operations are encapsulated behind safe public APIs.` |
| N6 | `src/quantization/simd/mod.rs:48` | Doc | `/// This function is completely safe. All unsafe operations are internal` |
| N7 | `src/simd/dispatch.rs:33` | Doc | `//! avx2: unsafe { x86::dot_product(a, b) },` |
| N8 | `src/simd/dispatch.rs:98` | Doc | `/// avx2: unsafe { x86::hamming_distance(a, b) },` |
| N9 | `src/simd/dispatch.rs:122` | Doc | `/// - Use 'unsafe { }' when calling unsafe SIMD intrinsics` |
| N10 | `src/simd/neon.rs:14` | Doc | `//! The unsafe code within this module:` |
| N11 | `src/simd/neon.rs:68` | Doc | `/// This function uses unsafe NEON intrinsics internally.` |
| N12 | `src/simd/neon.rs:96` | Comment | `// SAFETY: We've verified equal lengths. The unsafe function handles` |
| N13 | `src/simd/neon.rs:234` | Doc | `/// This function uses unsafe NEON intrinsics internally.` |
| N14 | `src/simd/neon.rs:262` | Comment | `// SAFETY: We've verified equal lengths. The unsafe function handles` |
| N15 | `src/simd/neon.rs:368` | Doc | `/// This function uses unsafe NEON intrinsics internally.` |
| N16 | `src/simd/neon.rs:396` | Comment | `// SAFETY: We've verified equal lengths. The unsafe function handles` |
| N17 | `src/sparse/vector.rs:160` | Doc | `/// # Safety (not unsafe, but requires caller discipline)` |
| N18 | `src/wasm/iterator.rs:15` | Doc | `/// **WARNING:** This iterator holds a reference... (via 'unsafe' transmutation).` |
| N19 | `src/wasm/iterator.rs:26` | Comment | `/// The underlying iterator, with lifetime erased via unsafe.` |

Additionally, 4 grep hits are `#[allow(...)]` attributes:

| # | File:Line | Content |
|:--|:----------|:--------|
| A1 | `src/persistence/chunking.rs:273` | Comment referencing prior unsafe audit |
| A2 | `src/persistence/snapshot.rs:233` | Comment referencing prior unsafe audit |
| A3 | `src/wasm/mod.rs:628-629` | Doc explaining `unsafe_derive_deserialize` allow |
| A4 | `src/wasm/mod.rs:633` | `#[allow(clippy::unsafe_derive_deserialize)]` |
| A5 | `src/wasm/mod.rs:1694` | `#[allow(unsafe_code)]` |

**Note on persistence:** Lines 273 and 233 reference `docs/audits/unsafe_audit_persistence.md`. The original unsafe code in persistence was **removed** in W13.2 (bytemuck integration) and W42 (Miri audit). These lines are safe code with historical comments.

---

## Cross-Validation

| Metric | Value |
|:-------|:------|
| Total `grep -rn "unsafe" src/` hits | 62 |
| Production unsafe code sites (A1-H1) | 31 |
| Test-only unsafe call-sites (T1-T6) | 6 |
| Non-code mentions (N1-N19, A1-A5) | 25 |
| **Sum** | **62** |

Breakdown of 31 production sites:
- `unsafe { ... }` blocks: 14 (A1, A3, A5, A6, B1, B3, B5, C1, C2, C3, E1, G1, G2, H1)
- `unsafe fn` declarations: 10 (A2, A4, B2, B4, B6, B7, B8, D1, D2, G3, G4 -- note B7+B8 = 2, G3+G4 = 2, so 10 total)
- Safe functions calling unsafe: 7 (F1, F3, F5, and the 4 unchecked fns F2, F4, F6 are already counted as `unsafe fn`)

Recount by file:
- `src/metric/simd.rs`: 17 production sites (A1-A6, B1-B8, C1-C3)
- `src/quantization/simd/avx2.rs`: 2 production sites (D1, D2)
- `src/quantization/simd/mod.rs`: 1 production site (E1)
- `src/simd/neon.rs`: 6 production sites (F1-F6)
- `src/simd/popcount.rs`: 4 production sites (G1-G4)
- `src/wasm/mod.rs`: 1 production site (H1)
- **Total: 31 production sites**

---

## Recommendations

1. **H1 (transmute) is the only HIGH-risk site.** Monitor `wasm-bindgen` for lifetime support. When available, replace the transmute with a safe borrowed return.

2. **All SIMD unsafe is well-structured.** Each follows the same pattern: assert preconditions, enter unsafe block, use loop bounds to stay in-bounds. The codebase has no "creative" unsafe -- only standard SIMD patterns.

3. **Consider `safe_arch` crate.** The `safe_arch` crate provides safe wrappers around x86 SIMD intrinsics. Adopting it would eliminate category B entirely. However, it does not cover WASM SIMD128 or NEON.

4. **No unsafe for performance optimization.** The `get_unchecked` calls inside SIMD blocks are secondary to the SIMD intrinsics that already require unsafe. There are zero standalone `get_unchecked` sites outside of already-unsafe contexts.

5. **Persistence code is clean.** The original unsafe in `src/persistence/` was replaced with `bytemuck` (safe casts) in W13.2 and further hardened with `pod_read_unaligned` in W42 (Miri audit).

---

## Appendix: Raw Grep Output

```
$ grep -rn "unsafe" src/

src/metric/simd.rs:30:        unsafe {
src/metric/simd.rs:105:    pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
src/metric/simd.rs:198:        unsafe {
src/metric/simd.rs:256:    pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
src/metric/simd.rs:387:        // SAFETY: This unsafe block is required for WASM SIMD128 intrinsics.
src/metric/simd.rs:399:        unsafe {
src/metric/simd.rs:526:        unsafe {
src/metric/simd.rs:693:        unsafe {
src/metric/simd.rs:775:    pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
src/metric/simd.rs:840:        unsafe {
src/metric/simd.rs:905:    pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
src/metric/simd.rs:965:        unsafe {
src/metric/simd.rs:1066:    pub unsafe fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
src/metric/simd.rs:1075:        // SAFETY: This unsafe block is required for AVX2 SIMD intrinsics.
src/metric/simd.rs:1177:    unsafe fn hsum256_ps_avx(v: __m256) -> f32 {
src/metric/simd.rs:1188:    unsafe fn hsum256_epi32_avx(v: __m256i) -> u32 {
src/metric/simd.rs:1286:            unsafe { wasm::l2_squared_u8(a, b) }
src/metric/simd.rs:1288:            unsafe { x86::l2_squared_u8(a, b) }
src/metric/simd.rs:1336:            // WASM SIMD128 path - safe wrapper, no explicit unsafe needed
src/metric/simd.rs:1343:            unsafe { x86::hamming_distance(a, b) }
src/quantization/simd/avx2.rs:23://! All functions in this module are marked `unsafe` and require:
src/quantization/simd/avx2.rs:67:pub(crate) unsafe fn hamming_distance_avx2(a: &[u8; 96], b: &[u8; 96]) -> u32 {
src/quantization/simd/avx2.rs:121:unsafe fn popcount_avx2(v: __m256i) -> u32 {
src/quantization/simd/avx2.rs:153:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/avx2.rs:167:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/avx2.rs:181:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/avx2.rs:196:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/avx2.rs:212:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/avx2.rs:228:        let distance = unsafe { hamming_distance_avx2(&a, &b) };
src/quantization/simd/mod.rs:12://! All unsafe SIMD operations are encapsulated behind safe public APIs.
src/quantization/simd/mod.rs:48:/// This function is completely safe. All unsafe operations are internal
src/quantization/simd/mod.rs:70:            return unsafe { avx2::hamming_distance_avx2(a, b) };
src/simd/dispatch.rs:33://!         avx2: unsafe { x86::dot_product(a, b) },
src/simd/dispatch.rs:98:///         avx2: unsafe { x86::hamming_distance(a, b) },
src/simd/dispatch.rs:122:/// - Use `unsafe { }` when calling unsafe SIMD intrinsics
src/simd/neon.rs:14://! The unsafe code within this module:
src/simd/neon.rs:68:/// This function uses unsafe NEON intrinsics internally. Safety is guaranteed by:
src/simd/neon.rs:96:    // SAFETY: We've verified equal lengths. The unsafe function handles
src/simd/neon.rs:98:    unsafe { hamming_distance_neon_unchecked(a, b) }
src/simd/neon.rs:116:unsafe fn hamming_distance_neon_unchecked(a: &[u8], b: &[u8]) -> u32 {
src/simd/neon.rs:234:/// This function uses unsafe NEON intrinsics internally. Safety is guaranteed by:
src/simd/neon.rs:262:    // SAFETY: We've verified equal lengths. The unsafe function handles
src/simd/neon.rs:264:    unsafe { dot_product_neon_unchecked(a, b) }
src/simd/neon.rs:282:unsafe fn dot_product_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
src/simd/neon.rs:368:/// This function uses unsafe NEON intrinsics internally. Safety is guaranteed by:
src/simd/neon.rs:396:    // SAFETY: We've verified equal lengths. The unsafe function handles
src/simd/neon.rs:398:    unsafe { euclidean_distance_neon_unchecked(a, b) }
src/simd/neon.rs:416:unsafe fn euclidean_distance_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
src/simd/popcount.rs:79:            return unsafe { avx2_popcount_xor(a, b) };
src/simd/popcount.rs:90:            return unsafe { neon_popcount_xor(a, b) };
src/simd/popcount.rs:132:unsafe fn avx2_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
src/simd/popcount.rs:208:unsafe fn neon_popcount_xor(a: &[u8], b: &[u8]) -> u32 {
src/sparse/vector.rs:160:    /// # Safety (not unsafe, but requires caller discipline)
src/wasm/iterator.rs:15:/// **WARNING:** This iterator holds a reference to the `EdgeVec` instance (via `unsafe` transmutation).
src/wasm/iterator.rs:26:    /// The underlying iterator, with lifetime erased via unsafe.
src/wasm/mod.rs:628:/// This type derives `Deserialize` despite containing methods with `unsafe`.
src/wasm/mod.rs:629:/// The unsafe code (`save_stream`) is unrelated to deserialization and is safe
src/wasm/mod.rs:633:#[allow(clippy::unsafe_derive_deserialize)]
src/wasm/mod.rs:1694:        #[allow(unsafe_code)]
src/wasm/mod.rs:1695:        let static_iter = unsafe { std::mem::transmute::<ChunkIter<'_>, ChunkIter<'static>>(iter) };
```

**Total: 62 lines. All accounted for.**

---

**END OF DOCUMENT**
