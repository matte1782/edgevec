//! SIMD primitives for distance metrics.
//!
//! This module provides platform-specific SIMD implementations.
//! They are gated by `cfg` flags and feature detection.
//!
//! # Safety
//!
//! This module uses intentional pointer casts for SIMD operations.
//! The `_mm_loadu_*` intrinsics handle unaligned loads safely.

// SIMD code requires intentional pointer casts and alignment handling.
// These lints are disabled at module level as they are false positives for SIMD code.
#![allow(clippy::cast_ptr_alignment)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::missing_panics_doc)]

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
/// WASM SIMD implementations.
pub mod wasm {
    use std::arch::wasm32::*;

    /// L2 Squared distance using WASM SIMD128.
    ///
    /// # Safety
    ///
    /// Requires `simd128` target feature.
    #[inline]
    pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;

            // 4 accumulators to break dependency chains and increase ILP
            let mut sum0 = f32x4_splat(0.0);
            let mut sum1 = f32x4_splat(0.0);
            let mut sum2 = f32x4_splat(0.0);
            let mut sum3 = f32x4_splat(0.0);

            // Process 16 floats (4 vectors) per iteration
            while i + 16 <= n {
                let ptr_a = a.as_ptr().add(i) as *const v128;
                let ptr_b = b.as_ptr().add(i) as *const v128;

                // Load 4 vectors from each array
                let a0 = v128_load(ptr_a);
                let b0 = v128_load(ptr_b);
                let a1 = v128_load(ptr_a.add(1));
                let b1 = v128_load(ptr_b.add(1));
                let a2 = v128_load(ptr_a.add(2));
                let b2 = v128_load(ptr_b.add(2));
                let a3 = v128_load(ptr_a.add(3));
                let b3 = v128_load(ptr_b.add(3));

                // Compute diffs
                let d0 = f32x4_sub(a0, b0);
                let d1 = f32x4_sub(a1, b1);
                let d2 = f32x4_sub(a2, b2);
                let d3 = f32x4_sub(a3, b3);

                // Accumulate squares
                sum0 = f32x4_add(sum0, f32x4_mul(d0, d0));
                sum1 = f32x4_add(sum1, f32x4_mul(d1, d1));
                sum2 = f32x4_add(sum2, f32x4_mul(d2, d2));
                sum3 = f32x4_add(sum3, f32x4_mul(d3, d3));

                i += 16;
            }

            // Reduce accumulators to single vector
            let sum_mid = f32x4_add(f32x4_add(sum0, sum1), f32x4_add(sum2, sum3));
            let mut sum_v = sum_mid;

            // Handle remaining chunks of 4
            while i + 4 <= n {
                let va = v128_load(a.as_ptr().add(i) as *const v128);
                let vb = v128_load(b.as_ptr().add(i) as *const v128);
                let diff = f32x4_sub(va, vb);
                sum_v = f32x4_add(sum_v, f32x4_mul(diff, diff));
                i += 4;
            }

            // Reduce vector to scalar
            let mut sum = f32x4_extract_lane::<0>(sum_v)
                + f32x4_extract_lane::<1>(sum_v)
                + f32x4_extract_lane::<2>(sum_v)
                + f32x4_extract_lane::<3>(sum_v);

            // Scalar tail
            while i < n {
                let diff = *a.get_unchecked(i) - *b.get_unchecked(i);
                sum += diff * diff;
                i += 1;
            }
            sum
        }
    }

    /// L2 Squared distance for u8 vectors using WASM SIMD128.
    ///
    /// # Safety
    ///
    /// Requires `simd128` target feature.
    #[inline]
    pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
        assert_eq!(a.len(), b.len());
        let n = a.len();
        let mut i = 0;

        let mut sum0 = i32x4_splat(0);
        let mut sum1 = i32x4_splat(0);
        let mut sum2 = i32x4_splat(0);
        let mut sum3 = i32x4_splat(0);

        // Process 64 bytes (16 * 4) per iteration
        while i + 64 <= n {
            let ptr_a = a.as_ptr().add(i) as *const v128;
            let ptr_b = b.as_ptr().add(i) as *const v128;

            let a0 = v128_load(ptr_a);
            let b0 = v128_load(ptr_b);
            let a1 = v128_load(ptr_a.add(1));
            let b1 = v128_load(ptr_b.add(1));
            let a2 = v128_load(ptr_a.add(2));
            let b2 = v128_load(ptr_b.add(2));
            let a3 = v128_load(ptr_a.add(3));
            let b3 = v128_load(ptr_b.add(3));

            // Helper to process diff
            macro_rules! process {
                ($va:expr, $vb:expr, $sum:expr) => {
                    let a_lo = i16x8_extend_low_u8x16($va);
                    let a_hi = i16x8_extend_high_u8x16($va);
                    let b_lo = i16x8_extend_low_u8x16($vb);
                    let b_hi = i16x8_extend_high_u8x16($vb);
                    let d_lo = i16x8_sub(a_lo, b_lo);
                    let d_hi = i16x8_sub(a_hi, b_hi);
                    $sum = i32x4_add($sum, i32x4_dot_i16x8(d_lo, d_lo));
                    $sum = i32x4_add($sum, i32x4_dot_i16x8(d_hi, d_hi));
                };
            }

            process!(a0, b0, sum0);
            process!(a1, b1, sum1);
            process!(a2, b2, sum2);
            process!(a3, b3, sum3);

            i += 64;
        }

        let sum_v = i32x4_add(i32x4_add(sum0, sum1), i32x4_add(sum2, sum3));
        let mut sum = i32x4_extract_lane::<0>(sum_v) as u32
            + i32x4_extract_lane::<1>(sum_v) as u32
            + i32x4_extract_lane::<2>(sum_v) as u32
            + i32x4_extract_lane::<3>(sum_v) as u32;

        // Tail loop 16
        while i + 16 <= n {
            let va = v128_load(a.as_ptr().add(i) as *const v128);
            let vb = v128_load(b.as_ptr().add(i) as *const v128);

            let a_lo = i16x8_extend_low_u8x16(va);
            let a_hi = i16x8_extend_high_u8x16(va);
            let b_lo = i16x8_extend_low_u8x16(vb);
            let b_hi = i16x8_extend_high_u8x16(vb);

            let d_lo = i16x8_sub(a_lo, b_lo);
            let d_hi = i16x8_sub(a_hi, b_hi);

            let s_lo = i32x4_dot_i16x8(d_lo, d_lo);
            let s_hi = i32x4_dot_i16x8(d_hi, d_hi);
            let s = i32x4_add(s_lo, s_hi);

            sum += i32x4_extract_lane::<0>(s) as u32
                + i32x4_extract_lane::<1>(s) as u32
                + i32x4_extract_lane::<2>(s) as u32
                + i32x4_extract_lane::<3>(s) as u32;
            i += 16;
        }

        // Scalar tail
        while i < n {
            let diff = (*a.get_unchecked(i) as i32) - (*b.get_unchecked(i) as i32);
            sum += (diff * diff) as u32;
            i += 1;
        }
        sum
    }

    /// Dot Product using WASM SIMD128.
    ///
    /// # Safety
    ///
    /// Requires `simd128` target feature.
    #[inline]
    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;

            let mut sum0 = f32x4_splat(0.0);
            let mut sum1 = f32x4_splat(0.0);
            let mut sum2 = f32x4_splat(0.0);
            let mut sum3 = f32x4_splat(0.0);

            while i + 16 <= n {
                let ptr_a = a.as_ptr().add(i) as *const v128;
                let ptr_b = b.as_ptr().add(i) as *const v128;

                let a0 = v128_load(ptr_a);
                let b0 = v128_load(ptr_b);
                let a1 = v128_load(ptr_a.add(1));
                let b1 = v128_load(ptr_b.add(1));
                let a2 = v128_load(ptr_a.add(2));
                let b2 = v128_load(ptr_b.add(2));
                let a3 = v128_load(ptr_a.add(3));
                let b3 = v128_load(ptr_b.add(3));

                sum0 = f32x4_add(sum0, f32x4_mul(a0, b0));
                sum1 = f32x4_add(sum1, f32x4_mul(a1, b1));
                sum2 = f32x4_add(sum2, f32x4_mul(a2, b2));
                sum3 = f32x4_add(sum3, f32x4_mul(a3, b3));

                i += 16;
            }

            let mut sum_v = f32x4_add(f32x4_add(sum0, sum1), f32x4_add(sum2, sum3));

            while i + 4 <= n {
                let va = v128_load(a.as_ptr().add(i) as *const v128);
                let vb = v128_load(b.as_ptr().add(i) as *const v128);
                sum_v = f32x4_add(sum_v, f32x4_mul(va, vb));
                i += 4;
            }

            let mut sum = f32x4_extract_lane::<0>(sum_v)
                + f32x4_extract_lane::<1>(sum_v)
                + f32x4_extract_lane::<2>(sum_v)
                + f32x4_extract_lane::<3>(sum_v);

            while i < n {
                sum += *a.get_unchecked(i) * *b.get_unchecked(i);
                i += 1;
            }
            sum
        }
    }

    /// Dot Product for u8 vectors using WASM SIMD128.
    ///
    /// # Safety
    ///
    /// Requires `simd128` target feature.
    #[inline]
    pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
        assert_eq!(a.len(), b.len());
        let n = a.len();
        let mut i = 0;

        let mut sum0 = i32x4_splat(0);
        let mut sum1 = i32x4_splat(0);
        let mut sum2 = i32x4_splat(0);
        let mut sum3 = i32x4_splat(0);

        while i + 64 <= n {
            let ptr_a = a.as_ptr().add(i) as *const v128;
            let ptr_b = b.as_ptr().add(i) as *const v128;

            let a0 = v128_load(ptr_a);
            let b0 = v128_load(ptr_b);
            let a1 = v128_load(ptr_a.add(1));
            let b1 = v128_load(ptr_b.add(1));
            let a2 = v128_load(ptr_a.add(2));
            let b2 = v128_load(ptr_b.add(2));
            let a3 = v128_load(ptr_a.add(3));
            let b3 = v128_load(ptr_b.add(3));

            macro_rules! process_dot {
                ($va:expr, $vb:expr, $sum:expr) => {
                    let a_lo = i16x8_extend_low_u8x16($va);
                    let a_hi = i16x8_extend_high_u8x16($va);
                    let b_lo = i16x8_extend_low_u8x16($vb);
                    let b_hi = i16x8_extend_high_u8x16($vb);
                    $sum = i32x4_add($sum, i32x4_dot_i16x8(a_lo, b_lo));
                    $sum = i32x4_add($sum, i32x4_dot_i16x8(a_hi, b_hi));
                };
            }

            process_dot!(a0, b0, sum0);
            process_dot!(a1, b1, sum1);
            process_dot!(a2, b2, sum2);
            process_dot!(a3, b3, sum3);

            i += 64;
        }

        let sum_v = i32x4_add(i32x4_add(sum0, sum1), i32x4_add(sum2, sum3));
        let mut sum = i32x4_extract_lane::<0>(sum_v) as u32
            + i32x4_extract_lane::<1>(sum_v) as u32
            + i32x4_extract_lane::<2>(sum_v) as u32
            + i32x4_extract_lane::<3>(sum_v) as u32;

        while i + 16 <= n {
            let va = v128_load(a.as_ptr().add(i) as *const v128);
            let vb = v128_load(b.as_ptr().add(i) as *const v128);

            let a_lo = i16x8_extend_low_u8x16(va);
            let a_hi = i16x8_extend_high_u8x16(va);
            let b_lo = i16x8_extend_low_u8x16(vb);
            let b_hi = i16x8_extend_high_u8x16(vb);

            let s_lo = i32x4_dot_i16x8(a_lo, b_lo);
            let s_hi = i32x4_dot_i16x8(a_hi, b_hi);
            let s = i32x4_add(s_lo, s_hi);

            sum += i32x4_extract_lane::<0>(s) as u32
                + i32x4_extract_lane::<1>(s) as u32
                + i32x4_extract_lane::<2>(s) as u32
                + i32x4_extract_lane::<3>(s) as u32;
            i += 16;
        }

        while i < n {
            sum += (*a.get_unchecked(i) as u32) * (*b.get_unchecked(i) as u32);
            i += 1;
        }
        sum
    }

    // =========================================================================
    // Constants for Hamming distance SIMD implementation
    // =========================================================================

    /// WASM SIMD128 vector width in bytes (128 bits / 8 bits per byte).
    const WASM_U8_VECTOR_WIDTH: usize = 16;

    /// Optimal unroll factor for WASM u8 operations (4 vectors = 64 bytes).
    /// Reduces dependency chains and increases instruction-level parallelism.
    const WASM_U8_UNROLL_BYTES: usize = 64;

    /// Mask to extract low nibble (4 bits) from a byte.
    /// 0x0F = 0b00001111 - masks out high 4 bits.
    const LOW_NIBBLE_MASK: u8 = 0x0F;

    /// Hamming distance using WASM SIMD128.
    ///
    /// Counts the number of differing bits between two byte slices using SIMD acceleration.
    /// This implementation uses a 4-bit lookup table (LUT) approach with SIMD shuffle
    /// for efficient popcount computation.
    ///
    /// # Algorithm
    ///
    /// 1. XOR input vectors to find differing bits
    /// 2. Split each byte into low/high nibbles (4 bits each)
    /// 3. Use SIMD shuffle (`i8x16_swizzle`) to lookup popcount for each nibble
    /// 4. Sum popcounts using horizontal reduction (i8 → i16 → i32)
    ///
    /// # Arguments
    ///
    /// * `a` - First byte slice (binary vector)
    /// * `b` - Second byte slice (must have same length as `a`)
    ///
    /// # Returns
    ///
    /// Number of differing bits (0 to `a.len() * 8`).
    ///
    /// # Panics
    ///
    /// Panics if `a.len() != b.len()`. This matches the existing SIMD function
    /// patterns in this module. A future PR should convert all SIMD functions
    /// to return `Result` for consistency with CONTRIBUTING.md guidelines.
    ///
    /// # Performance
    ///
    /// - **Speedup:** 2-12x faster than scalar depending on vector size
    /// - **Optimal:** Vectors ≥64 bytes benefit from 4-wide unrolling
    /// - **Complexity:** O(n) where n = byte count
    ///
    /// # Target Feature Requirement
    ///
    /// Requires `simd128` target feature enabled at compile time.
    #[inline]
    pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
        assert_eq!(a.len(), b.len());

        // SAFETY: This unsafe block is required for WASM SIMD128 intrinsics.
        //
        // Safety invariants maintained:
        // 1. The `simd128` target feature is statically verified by the cfg guard
        //    on the parent module (line 17).
        // 2. Slice length equality is verified by assert_eq above.
        // 3. Loop bounds (`i + 64 <= n`, `i + 16 <= n`) guarantee all pointer
        //    arithmetic stays within slice bounds.
        // 4. `v128_load` performs unaligned loads, which are safe per WASM SIMD spec.
        // 5. Scalar tail uses `get_unchecked(i)` only when `i < n` is verified.
        //
        // Reference: WASM SIMD128 specification §5.2 (unaligned memory operations).
        unsafe {
            let n = a.len();
            let mut i = 0;

            // 4-bit popcount lookup table: maps nibble value (0-15) to bit count.
            // Source: Warren, "Hacker's Delight", 2nd ed., Section 5-1.
            // Values: [0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4]
            let lut = i8x16(0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4);
            let low_mask = u8x16_splat(LOW_NIBBLE_MASK);

            // 4 accumulators to increase ILP
            let mut sum0 = i32x4_splat(0);
            let mut sum1 = i32x4_splat(0);
            let mut sum2 = i32x4_splat(0);
            let mut sum3 = i32x4_splat(0);

            // Process 64 bytes (4 x 16-byte vectors) per iteration
            while i + 64 <= n {
                let ptr_a = a.as_ptr().add(i) as *const v128;
                let ptr_b = b.as_ptr().add(i) as *const v128;

                // Load 4 vectors from each array
                let a0 = v128_load(ptr_a);
                let b0 = v128_load(ptr_b);
                let a1 = v128_load(ptr_a.add(1));
                let b1 = v128_load(ptr_b.add(1));
                let a2 = v128_load(ptr_a.add(2));
                let b2 = v128_load(ptr_b.add(2));
                let a3 = v128_load(ptr_a.add(3));
                let b3 = v128_load(ptr_b.add(3));

                // XOR to find differing bits
                let xor0 = v128_xor(a0, b0);
                let xor1 = v128_xor(a1, b1);
                let xor2 = v128_xor(a2, b2);
                let xor3 = v128_xor(a3, b3);

                // Popcount using LUT: count bits in each byte
                // For each byte: popcount(byte) = lut[low_nibble] + lut[high_nibble]
                macro_rules! popcount_bytes {
                    ($xor:expr) => {{
                        let lo = v128_and($xor, low_mask);
                        let hi = u8x16_shr($xor, 4);
                        let cnt_lo = i8x16_swizzle(lut, lo);
                        let cnt_hi = i8x16_swizzle(lut, hi);
                        i8x16_add(cnt_lo, cnt_hi)
                    }};
                }

                let cnt0 = popcount_bytes!(xor0);
                let cnt1 = popcount_bytes!(xor1);
                let cnt2 = popcount_bytes!(xor2);
                let cnt3 = popcount_bytes!(xor3);

                // Sum bytes horizontally into i32 accumulators
                // Use pairwise widening: i8 -> i16 -> i32
                macro_rules! hsum_to_i32 {
                    ($cnt:expr) => {{
                        // First, sum pairs of u8 -> u16
                        let lo = i16x8_extend_low_u8x16($cnt);
                        let hi = i16x8_extend_high_u8x16($cnt);
                        let sum16 = i16x8_add(lo, hi);
                        // Then sum pairs of i16 -> i32
                        let lo32 = i32x4_extend_low_i16x8(sum16);
                        let hi32 = i32x4_extend_high_i16x8(sum16);
                        i32x4_add(lo32, hi32)
                    }};
                }

                sum0 = i32x4_add(sum0, hsum_to_i32!(cnt0));
                sum1 = i32x4_add(sum1, hsum_to_i32!(cnt1));
                sum2 = i32x4_add(sum2, hsum_to_i32!(cnt2));
                sum3 = i32x4_add(sum3, hsum_to_i32!(cnt3));

                i += 64;
            }

            // Reduce 4 accumulators to one
            let sum_mid = i32x4_add(i32x4_add(sum0, sum1), i32x4_add(sum2, sum3));
            let mut sum_v = sum_mid;

            // Handle remaining chunks of 16 bytes
            while i + 16 <= n {
                let va = v128_load(a.as_ptr().add(i) as *const v128);
                let vb = v128_load(b.as_ptr().add(i) as *const v128);
                let xor = v128_xor(va, vb);

                let lo = v128_and(xor, low_mask);
                let hi = u8x16_shr(xor, 4);
                let cnt_lo = i8x16_swizzle(lut, lo);
                let cnt_hi = i8x16_swizzle(lut, hi);
                let cnt = i8x16_add(cnt_lo, cnt_hi);

                let lo16 = i16x8_extend_low_u8x16(cnt);
                let hi16 = i16x8_extend_high_u8x16(cnt);
                let sum16 = i16x8_add(lo16, hi16);
                let lo32 = i32x4_extend_low_i16x8(sum16);
                let hi32 = i32x4_extend_high_i16x8(sum16);
                sum_v = i32x4_add(sum_v, i32x4_add(lo32, hi32));

                i += 16;
            }

            // Reduce vector to scalar
            let mut sum = (i32x4_extract_lane::<0>(sum_v)
                + i32x4_extract_lane::<1>(sum_v)
                + i32x4_extract_lane::<2>(sum_v)
                + i32x4_extract_lane::<3>(sum_v)) as u32;

            // Scalar tail for remaining bytes
            while i < n {
                sum += (*a.get_unchecked(i) ^ *b.get_unchecked(i)).count_ones();
                i += 1;
            }

            sum
        }
    }

    /// Cosine Similarity using WASM SIMD128.
    ///
    /// # Safety
    ///
    /// Requires `simd128` target feature.
    #[inline]
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;

            let mut dot0 = f32x4_splat(0.0);
            let mut dot1 = f32x4_splat(0.0);
            let mut dot2 = f32x4_splat(0.0);
            let mut dot3 = f32x4_splat(0.0);

            let mut aa0 = f32x4_splat(0.0);
            let mut aa1 = f32x4_splat(0.0);
            let mut aa2 = f32x4_splat(0.0);
            let mut aa3 = f32x4_splat(0.0);

            let mut bb0 = f32x4_splat(0.0);
            let mut bb1 = f32x4_splat(0.0);
            let mut bb2 = f32x4_splat(0.0);
            let mut bb3 = f32x4_splat(0.0);

            while i + 16 <= n {
                let ptr_a = a.as_ptr().add(i) as *const v128;
                let ptr_b = b.as_ptr().add(i) as *const v128;

                let a0 = v128_load(ptr_a);
                let b0 = v128_load(ptr_b);
                let a1 = v128_load(ptr_a.add(1));
                let b1 = v128_load(ptr_b.add(1));
                let a2 = v128_load(ptr_a.add(2));
                let b2 = v128_load(ptr_b.add(2));
                let a3 = v128_load(ptr_a.add(3));
                let b3 = v128_load(ptr_b.add(3));

                dot0 = f32x4_add(dot0, f32x4_mul(a0, b0));
                aa0 = f32x4_add(aa0, f32x4_mul(a0, a0));
                bb0 = f32x4_add(bb0, f32x4_mul(b0, b0));

                dot1 = f32x4_add(dot1, f32x4_mul(a1, b1));
                aa1 = f32x4_add(aa1, f32x4_mul(a1, a1));
                bb1 = f32x4_add(bb1, f32x4_mul(b1, b1));

                dot2 = f32x4_add(dot2, f32x4_mul(a2, b2));
                aa2 = f32x4_add(aa2, f32x4_mul(a2, a2));
                bb2 = f32x4_add(bb2, f32x4_mul(b2, b2));

                dot3 = f32x4_add(dot3, f32x4_mul(a3, b3));
                aa3 = f32x4_add(aa3, f32x4_mul(a3, a3));
                bb3 = f32x4_add(bb3, f32x4_mul(b3, b3));

                i += 16;
            }

            let mut dot_v = f32x4_add(f32x4_add(dot0, dot1), f32x4_add(dot2, dot3));
            let mut aa_v = f32x4_add(f32x4_add(aa0, aa1), f32x4_add(aa2, aa3));
            let mut bb_v = f32x4_add(f32x4_add(bb0, bb1), f32x4_add(bb2, bb3));

            while i + 4 <= n {
                let va = v128_load(a.as_ptr().add(i) as *const v128);
                let vb = v128_load(b.as_ptr().add(i) as *const v128);
                dot_v = f32x4_add(dot_v, f32x4_mul(va, vb));
                aa_v = f32x4_add(aa_v, f32x4_mul(va, va));
                bb_v = f32x4_add(bb_v, f32x4_mul(vb, vb));
                i += 4;
            }

            let mut dot = f32x4_extract_lane::<0>(dot_v)
                + f32x4_extract_lane::<1>(dot_v)
                + f32x4_extract_lane::<2>(dot_v)
                + f32x4_extract_lane::<3>(dot_v);
            let mut aa = f32x4_extract_lane::<0>(aa_v)
                + f32x4_extract_lane::<1>(aa_v)
                + f32x4_extract_lane::<2>(aa_v)
                + f32x4_extract_lane::<3>(aa_v);
            let mut bb = f32x4_extract_lane::<0>(bb_v)
                + f32x4_extract_lane::<1>(bb_v)
                + f32x4_extract_lane::<2>(bb_v)
                + f32x4_extract_lane::<3>(bb_v);

            while i < n {
                let va = *a.get_unchecked(i);
                let vb = *b.get_unchecked(i);
                dot += va * vb;
                aa += va * va;
                bb += vb * vb;
                i += 1;
            }

            if aa > 0.0 && bb > 0.0 {
                dot / (aa.sqrt() * bb.sqrt())
            } else {
                0.0
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
/// x86 AVX2 implementations.
pub mod x86 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        __m128i, __m256, __m256i, _mm256_add_epi32, _mm256_castps256_ps128, _mm256_castsi256_si128,
        _mm256_cvtepu8_epi16, _mm256_extractf128_ps, _mm256_extracti128_si256, _mm256_loadu_ps,
        _mm256_madd_epi16, _mm256_setzero_ps, _mm256_setzero_si256, _mm256_sub_epi16,
        _mm256_sub_ps, _mm_add_epi32, _mm_add_ps, _mm_add_ss, _mm_cvtsi128_si32, _mm_cvtss_f32,
        _mm_loadu_si128, _mm_movehl_ps, _mm_shuffle_epi32, _mm_shuffle_ps,
    };

    #[cfg(all(target_arch = "x86_64", target_feature = "fma"))]
    use std::arch::x86_64::_mm256_fmadd_ps;

    /// L2 Squared distance using AVX2.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature to be enabled at compile time.
    #[inline]
    #[must_use]
    pub fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;
            let mut sum256 = _mm256_setzero_ps();

            // Process 8 floats per iteration (32 bytes)
            // Unrolling to 4 registers (32 floats) helps pipelining
            // but simple 8-float loop is already much faster than scalar.
            // Let's implement moderate unrolling (2x = 16 floats).
            while i + 16 <= n {
                let va1 = _mm256_loadu_ps(a.as_ptr().add(i));
                let vb1 = _mm256_loadu_ps(b.as_ptr().add(i));
                let diff1 = _mm256_sub_ps(va1, vb1);

                // FMA: sum += diff * diff
                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(diff1, diff1, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    let sq1 = _mm256_mul_ps(diff1, diff1);
                    sum256 = _mm256_add_ps(sum256, sq1);
                }

                let va2 = _mm256_loadu_ps(a.as_ptr().add(i + 8));
                let vb2 = _mm256_loadu_ps(b.as_ptr().add(i + 8));
                let diff2 = _mm256_sub_ps(va2, vb2);

                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(diff2, diff2, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    let sq2 = _mm256_mul_ps(diff2, diff2);
                    sum256 = _mm256_add_ps(sum256, sq2);
                }

                i += 16;
            }

            // Single block loop for remaining chunks of 8
            while i + 8 <= n {
                let va = _mm256_loadu_ps(a.as_ptr().add(i));
                let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                let diff = _mm256_sub_ps(va, vb);

                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(diff, diff, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    let sq = _mm256_mul_ps(diff, diff);
                    sum256 = _mm256_add_ps(sum256, sq);
                }
                i += 8;
            }

            // Horizontal sum of 256-bit register
            // [h, g, f, e, d, c, b, a] -> sum
            let mut sum = hsum256_ps_avx(sum256);

            // Scalar tail
            while i < n {
                let diff = *a.get_unchecked(i) - *b.get_unchecked(i);
                sum += diff * diff;
                i += 1;
            }

            sum
        }
    }

    /// L2 Squared distance for u8 vectors using AVX2.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature.
    #[inline]
    #[must_use]
    pub unsafe fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
        assert_eq!(a.len(), b.len());
        let n = a.len();
        let mut i = 0;
        let mut sum256 = _mm256_setzero_si256();

        // Process 64 bytes per loop (4 * 16)
        while i + 64 <= n {
            let pa = a.as_ptr().add(i);
            let pb = b.as_ptr().add(i);

            // Helper macro
            macro_rules! process {
                ($offset:expr) => {
                    let a_chunk = _mm_loadu_si128(pa.add($offset) as *const __m128i);
                    let b_chunk = _mm_loadu_si128(pb.add($offset) as *const __m128i);
                    let a_ext = _mm256_cvtepu8_epi16(a_chunk);
                    let b_ext = _mm256_cvtepu8_epi16(b_chunk);
                    let diff = _mm256_sub_epi16(a_ext, b_ext);
                    let sq = _mm256_madd_epi16(diff, diff);
                    sum256 = _mm256_add_epi32(sum256, sq);
                };
            }

            process!(0);
            process!(16);
            process!(32);
            process!(48);

            i += 64;
        }

        while i + 16 <= n {
            let a_chunk = _mm_loadu_si128(a.as_ptr().add(i) as *const __m128i);
            let b_chunk = _mm_loadu_si128(b.as_ptr().add(i) as *const __m128i);
            let a_ext = _mm256_cvtepu8_epi16(a_chunk);
            let b_ext = _mm256_cvtepu8_epi16(b_chunk);
            let diff = _mm256_sub_epi16(a_ext, b_ext);
            let sq = _mm256_madd_epi16(diff, diff);
            sum256 = _mm256_add_epi32(sum256, sq);
            i += 16;
        }

        let mut sum = hsum256_epi32_avx(sum256);

        while i < n {
            let diff = i32::from(*a.get_unchecked(i)) - i32::from(*b.get_unchecked(i));
            // SAFETY: diff*diff is always non-negative
            #[allow(clippy::cast_sign_loss)]
            let sq = (diff * diff) as u32;
            sum += sq;
            i += 1;
        }
        sum
    }

    /// Dot Product using AVX2.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature to be enabled at compile time.
    #[inline]
    #[must_use]
    pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;
            let mut sum256 = _mm256_setzero_ps();

            while i + 16 <= n {
                let va1 = _mm256_loadu_ps(a.as_ptr().add(i));
                let vb1 = _mm256_loadu_ps(b.as_ptr().add(i));

                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(va1, vb1, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    sum256 = _mm256_add_ps(sum256, _mm256_mul_ps(va1, vb1));
                }

                let va2 = _mm256_loadu_ps(a.as_ptr().add(i + 8));
                let vb2 = _mm256_loadu_ps(b.as_ptr().add(i + 8));

                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(va2, vb2, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    sum256 = _mm256_add_ps(sum256, _mm256_mul_ps(va2, vb2));
                }
                i += 16;
            }

            while i + 8 <= n {
                let va = _mm256_loadu_ps(a.as_ptr().add(i));
                let vb = _mm256_loadu_ps(b.as_ptr().add(i));

                #[cfg(target_feature = "fma")]
                {
                    sum256 = _mm256_fmadd_ps(va, vb, sum256);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    sum256 = _mm256_add_ps(sum256, _mm256_mul_ps(va, vb));
                }
                i += 8;
            }

            let mut sum = hsum256_ps_avx(sum256);

            while i < n {
                sum += *a.get_unchecked(i) * *b.get_unchecked(i);
                i += 1;
            }

            sum
        }
    }

    /// Dot Product for u8 vectors using AVX2.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature.
    #[inline]
    #[must_use]
    pub unsafe fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
        assert_eq!(a.len(), b.len());
        let n = a.len();
        let mut i = 0;
        let mut sum256 = _mm256_setzero_si256();

        while i + 64 <= n {
            let pa = a.as_ptr().add(i);
            let pb = b.as_ptr().add(i);

            macro_rules! process {
                ($offset:expr) => {
                    let a_chunk = _mm_loadu_si128(pa.add($offset) as *const __m128i);
                    let b_chunk = _mm_loadu_si128(pb.add($offset) as *const __m128i);
                    let a_ext = _mm256_cvtepu8_epi16(a_chunk);
                    let b_ext = _mm256_cvtepu8_epi16(b_chunk);
                    let prod = _mm256_madd_epi16(a_ext, b_ext);
                    sum256 = _mm256_add_epi32(sum256, prod);
                };
            }

            process!(0);
            process!(16);
            process!(32);
            process!(48);

            i += 64;
        }

        while i + 16 <= n {
            let a_chunk = _mm_loadu_si128(a.as_ptr().add(i) as *const __m128i);
            let b_chunk = _mm_loadu_si128(b.as_ptr().add(i) as *const __m128i);
            let a_ext = _mm256_cvtepu8_epi16(a_chunk);
            let b_ext = _mm256_cvtepu8_epi16(b_chunk);
            let prod = _mm256_madd_epi16(a_ext, b_ext);
            sum256 = _mm256_add_epi32(sum256, prod);
            i += 16;
        }

        let mut sum = hsum256_epi32_avx(sum256);

        while i < n {
            sum += u32::from(*a.get_unchecked(i)) * u32::from(*b.get_unchecked(i));
            i += 1;
        }
        sum
    }

    /// Cosine Similarity using AVX2.
    ///
    /// Computes dot product and L2 norms of both vectors in parallel.
    /// Returns `dot(a, b) / (norm(a) * norm(b))`.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature to be enabled at compile time.
    #[inline]
    #[must_use]
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        unsafe {
            let n = a.len();
            let mut i = 0;

            let mut sum_dot = _mm256_setzero_ps();
            let mut sum_aa = _mm256_setzero_ps();
            let mut sum_bb = _mm256_setzero_ps();

            while i + 8 <= n {
                let va = _mm256_loadu_ps(a.as_ptr().add(i));
                let vb = _mm256_loadu_ps(b.as_ptr().add(i));

                // Dot: a * b
                #[cfg(target_feature = "fma")]
                {
                    sum_dot = _mm256_fmadd_ps(va, vb, sum_dot);
                    sum_aa = _mm256_fmadd_ps(va, va, sum_aa);
                    sum_bb = _mm256_fmadd_ps(vb, vb, sum_bb);
                }
                #[cfg(not(target_feature = "fma"))]
                {
                    sum_dot = _mm256_add_ps(sum_dot, _mm256_mul_ps(va, vb));
                    sum_aa = _mm256_add_ps(sum_aa, _mm256_mul_ps(va, va));
                    sum_bb = _mm256_add_ps(sum_bb, _mm256_mul_ps(vb, vb));
                }

                i += 8;
            }

            let mut dot = hsum256_ps_avx(sum_dot);
            let mut aa = hsum256_ps_avx(sum_aa);
            let mut bb = hsum256_ps_avx(sum_bb);

            while i < n {
                let va = *a.get_unchecked(i);
                let vb = *b.get_unchecked(i);
                dot += va * vb;
                aa += va * va;
                bb += vb * vb;
                i += 1;
            }

            if aa > 0.0 && bb > 0.0 {
                dot / (aa.sqrt() * bb.sqrt())
            } else {
                0.0
            }
        }
    }

    // =========================================================================
    // Constants for Hamming distance AVX2 implementation
    // =========================================================================

    /// AVX2 vector width in bytes (256 bits / 8 bits per byte).
    const AVX2_U8_VECTOR_WIDTH: usize = 32;

    /// Optimal unroll factor for AVX2 u8 operations (2 vectors = 64 bytes).
    const AVX2_U8_UNROLL_BYTES: usize = 64;

    /// Mask to extract low nibble (4 bits) from a byte.
    const LOW_NIBBLE_MASK_I8: i8 = 0x0F;

    /// Hamming distance using AVX2.
    ///
    /// Counts the number of differing bits between two byte slices using SIMD acceleration.
    /// This implementation uses a 4-bit lookup table (LUT) approach with SIMD shuffle
    /// for efficient popcount computation.
    ///
    /// # Algorithm
    ///
    /// 1. XOR input vectors to find differing bits
    /// 2. Split each byte into low/high nibbles (4 bits each)
    /// 3. Use SIMD shuffle (`_mm256_shuffle_epi8`) to lookup popcount for each nibble
    /// 4. Use SAD (`_mm256_sad_epu8`) for efficient horizontal byte summation
    ///
    /// # Arguments
    ///
    /// * `a` - First byte slice (binary vector)
    /// * `b` - Second byte slice (must have same length as `a`)
    ///
    /// # Returns
    ///
    /// Number of differing bits (0 to `a.len() * 8`).
    ///
    /// # Panics
    ///
    /// Panics if `a.len() != b.len()`. This matches the existing SIMD function
    /// patterns in this module.
    ///
    /// # Safety
    ///
    /// Requires `avx2` target feature enabled at compile time.
    ///
    /// # Performance
    ///
    /// - **Speedup:** 4-10x faster than scalar depending on vector size
    /// - **Optimal:** Vectors ≥64 bytes benefit from 2-wide unrolling
    /// - **Complexity:** O(n) where n = byte count
    #[inline]
    #[must_use]
    pub unsafe fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
        use std::arch::x86_64::{
            __m256i, _mm256_add_epi64, _mm256_add_epi8, _mm256_and_si256, _mm256_loadu_si256,
            _mm256_sad_epu8, _mm256_set1_epi8, _mm256_setzero_si256, _mm256_shuffle_epi8,
            _mm256_srli_epi16, _mm256_xor_si256,
        };

        assert_eq!(a.len(), b.len());

        // SAFETY: This unsafe block is required for AVX2 SIMD intrinsics.
        //
        // Safety invariants maintained:
        // 1. The `avx2` target feature is statically verified by the cfg guard
        //    on the parent module (line 563).
        // 2. Slice length equality is verified by assert_eq above.
        // 3. Loop bounds (`i + 64 <= n`, `i + 32 <= n`) guarantee all pointer
        //    arithmetic stays within slice bounds.
        // 4. `_mm256_loadu_si256` performs unaligned loads, which are safe.
        // 5. Scalar tail uses `get_unchecked(i)` only when `i < n` is verified.

        let n = a.len();
        let mut i = 0;

        // 4-bit popcount lookup table (duplicated for both 128-bit lanes).
        // Maps nibble value (0-15) to number of set bits.
        // Source: Warren, "Hacker's Delight", 2nd ed., Section 5-1.
        let lut = {
            use std::arch::x86_64::_mm256_setr_epi8;
            _mm256_setr_epi8(
                0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, // Low 128 bits
                0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3,
                4, // High 128 bits (duplicated for AVX2)
            )
        };
        let low_mask = _mm256_set1_epi8(LOW_NIBBLE_MASK_I8);

        let mut sum256 = _mm256_setzero_si256();

        // Process 64 bytes (2 x 32-byte vectors) per iteration
        while i + 64 <= n {
            let ptr_a = a.as_ptr().add(i) as *const __m256i;
            let ptr_b = b.as_ptr().add(i) as *const __m256i;

            let a0 = _mm256_loadu_si256(ptr_a);
            let b0 = _mm256_loadu_si256(ptr_b);
            let a1 = _mm256_loadu_si256(ptr_a.add(1));
            let b1 = _mm256_loadu_si256(ptr_b.add(1));

            // XOR to find differing bits
            let xor0 = _mm256_xor_si256(a0, b0);
            let xor1 = _mm256_xor_si256(a1, b1);

            // Popcount using LUT
            macro_rules! popcount_bytes {
                ($xor:expr) => {{
                    let lo = _mm256_and_si256($xor, low_mask);
                    let hi = _mm256_and_si256(_mm256_srli_epi16($xor, 4), low_mask);
                    let cnt_lo = _mm256_shuffle_epi8(lut, lo);
                    let cnt_hi = _mm256_shuffle_epi8(lut, hi);
                    _mm256_add_epi8(cnt_lo, cnt_hi)
                }};
            }

            let cnt0 = popcount_bytes!(xor0);
            let cnt1 = popcount_bytes!(xor1);

            // Use SAD (sum of absolute differences vs zero) to horizontally sum bytes
            // This efficiently sums all bytes in each 64-bit lane
            let zero = _mm256_setzero_si256();
            sum256 = _mm256_add_epi64(sum256, _mm256_sad_epu8(cnt0, zero));
            sum256 = _mm256_add_epi64(sum256, _mm256_sad_epu8(cnt1, zero));

            i += 64;
        }

        // Handle remaining chunks of 32 bytes
        while i + 32 <= n {
            let va = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let vb = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);
            let xor = _mm256_xor_si256(va, vb);

            let lo = _mm256_and_si256(xor, low_mask);
            let hi = _mm256_and_si256(_mm256_srli_epi16(xor, 4), low_mask);
            let cnt_lo = _mm256_shuffle_epi8(lut, lo);
            let cnt_hi = _mm256_shuffle_epi8(lut, hi);
            let cnt = _mm256_add_epi8(cnt_lo, cnt_hi);

            let zero = _mm256_setzero_si256();
            sum256 = _mm256_add_epi64(sum256, _mm256_sad_epu8(cnt, zero));

            i += 32;
        }

        // Extract and sum the 4 u64 lanes
        use std::arch::x86_64::_mm256_extract_epi64;
        let mut sum = (_mm256_extract_epi64(sum256, 0) as u64
            + _mm256_extract_epi64(sum256, 1) as u64
            + _mm256_extract_epi64(sum256, 2) as u64
            + _mm256_extract_epi64(sum256, 3) as u64) as u32;

        // Scalar tail
        while i < n {
            sum += (*a.get_unchecked(i) ^ *b.get_unchecked(i)).count_ones();
            i += 1;
        }

        sum
    }

    /// Horizontal sum of f32x8
    #[inline]
    unsafe fn hsum256_ps_avx(v: __m256) -> f32 {
        // High 128 + Low 128
        let x128 = _mm_add_ps(_mm256_extractf128_ps(v, 1), _mm256_castps256_ps128(v));
        // Shuffle and add
        let x64 = _mm_add_ps(x128, _mm_movehl_ps(x128, x128));
        let x32 = _mm_add_ss(x64, _mm_shuffle_ps(x64, x64, 0x55));
        _mm_cvtss_f32(x32)
    }

    /// Horizontal sum of i32x8
    #[inline]
    unsafe fn hsum256_epi32_avx(v: __m256i) -> u32 {
        // Split into two 128-bit vectors and add
        let vlow = _mm256_castsi256_si128(v);
        let vhigh = _mm256_extracti128_si256(v, 1);
        let v128 = _mm_add_epi32(vlow, vhigh);

        // Horizontal sum of 128-bit vector
        // [A, B, C, D]
        let v64 = _mm_add_epi32(v128, _mm_shuffle_epi32(v128, 0x4E)); // Swap high/low 64
        let v32 = _mm_add_epi32(v64, _mm_shuffle_epi32(v64, 0xB1)); // Swap adjacent 32

        // SAFETY: Sum of u32 values, result is always positive.
        // _mm_cvtsi128_si32 returns i32 but our data is logically u32.
        #[allow(clippy::cast_sign_loss)]
        let result = _mm_cvtsi128_si32(v32) as u32;
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_cosine_similarity_avx2() {
            // This test only runs if AVX2 is enabled at compile time.
            let a = vec![1.0, 0.0, 1.0, 0.0];
            let b = vec![1.0, 0.0, 0.0, 0.0];
            // dot = 1.0
            // norm_a = sqrt(2)
            // norm_b = 1
            // sim = 1 / sqrt(2) = FRAC_1_SQRT_2

            let sim = cosine_similarity(&a, &b);
            assert!((sim - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);

            // Orthogonal
            let c = vec![0.0, 1.0, 0.0, 1.0];
            let sim_orth = cosine_similarity(&b, &c);
            assert!(sim_orth.abs() < 1e-5);
        }
    }
}

/// Dispatcher for L2 Squared distance (u8).
///
/// Automatically selects the best implementation based on available features.
#[inline]
#[must_use]
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

/// Dispatcher for Hamming distance (u8 binary vectors).
///
/// Automatically selects the best SIMD implementation based on available features:
/// - WASM SIMD128 for WebAssembly targets
/// - AVX2 for x86_64 targets
/// - Scalar fallback for other platforms
///
/// # Arguments
///
/// * `a` - First byte slice (binary vector)
/// * `b` - Second byte slice (must have same length as `a`)
///
/// # Returns
///
/// Number of differing bits between the two vectors (0 to `a.len() * 8`).
///
/// # Panics
///
/// Panics if `a.len() != b.len()`.
///
/// # Performance
///
/// Typical speedups over scalar:
/// - WASM SIMD128: 2-12x (browser-dependent)
/// - AVX2: 4-10x
///
/// # Example
///
/// ```
/// use edgevec::metric::simd::hamming_distance;
///
/// let a = vec![0b11110000u8; 96]; // 768-bit binary vector
/// let b = vec![0b00001111u8; 96];
/// let distance = hamming_distance(&a, &b);
/// assert_eq!(distance, 768); // All bits differ
/// ```
#[inline]
#[must_use]
pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    cfg_if::cfg_if! {
        if #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))] {
            // WASM SIMD128 path - safe wrapper, no explicit unsafe needed
            wasm::hamming_distance(a, b)
        } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
            // SAFETY: The avx2 target feature is statically checked at compile time.
            // If this code path is compiled, AVX2 is guaranteed available.
            // The x86::hamming_distance function's safety contract requires avx2,
            // which is satisfied by the cfg guard.
            unsafe { x86::hamming_distance(a, b) }
        } else {
            // Scalar fallback - no SIMD available
            assert_eq!(a.len(), b.len());
            let mut distance: u32 = 0;
            for (x, y) in a.iter().zip(b.iter()) {
                distance += (x ^ y).count_ones();
            }
            distance
        }
    }
}

// =============================================================================
// Unit Tests for Hamming Distance
// =============================================================================

#[cfg(test)]
mod hamming_tests {
    use super::hamming_distance;

    /// Reference scalar implementation for correctness verification.
    fn scalar_hamming(a: &[u8], b: &[u8]) -> u32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum()
    }

    #[test]
    fn test_hamming_empty_vectors() {
        let a: Vec<u8> = vec![];
        let b: Vec<u8> = vec![];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn test_hamming_identical_vectors() {
        // Identical vectors should have distance 0
        let a = vec![0xABu8; 96];
        let b = vec![0xABu8; 96];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn test_hamming_all_bits_differ() {
        // 0x00 XOR 0xFF = 0xFF (8 bits set per byte)
        let a = vec![0x00u8; 96];
        let b = vec![0xFFu8; 96];
        assert_eq!(hamming_distance(&a, &b), 96 * 8); // 768 bits
    }

    #[test]
    fn test_hamming_single_bit_diff() {
        // Only one bit differs
        let a = vec![0b00000000u8];
        let b = vec![0b00000001u8];
        assert_eq!(hamming_distance(&a, &b), 1);
    }

    #[test]
    fn test_hamming_known_values() {
        // 0xF0 XOR 0x0F = 0xFF (8 bits)
        let a = vec![0xF0u8; 10];
        let b = vec![0x0Fu8; 10];
        assert_eq!(hamming_distance(&a, &b), 10 * 8);

        // 0xAA (10101010) XOR 0x55 (01010101) = 0xFF (8 bits)
        let c = vec![0xAAu8; 20];
        let d = vec![0x55u8; 20];
        assert_eq!(hamming_distance(&c, &d), 20 * 8);
    }

    #[test]
    fn test_hamming_matches_scalar() {
        // Test various sizes to exercise SIMD paths
        for size in [1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 96, 128, 192] {
            let a: Vec<u8> = (0..size).map(|i| i as u8).collect();
            let b: Vec<u8> = (0..size).map(|i| (i + 1) as u8).collect();

            let simd_result = hamming_distance(&a, &b);
            let scalar_result = scalar_hamming(&a, &b);

            assert_eq!(
                simd_result, scalar_result,
                "Mismatch at size {}: SIMD={}, scalar={}",
                size, simd_result, scalar_result
            );
        }
    }

    #[test]
    fn test_hamming_768bit_binary_vector() {
        // Common binary vector size: 768 bits = 96 bytes
        let a = vec![0b11110000u8; 96];
        let b = vec![0b00001111u8; 96];
        // Each byte has 8 bits different
        assert_eq!(hamming_distance(&a, &b), 96 * 8);
    }

    #[test]
    fn test_hamming_1024bit_binary_vector() {
        // 1024 bits = 128 bytes
        let a = vec![0x00u8; 128];
        let b = vec![0x01u8; 128]; // 1 bit per byte
        assert_eq!(hamming_distance(&a, &b), 128);
    }

    #[test]
    fn test_hamming_partial_bit_diffs() {
        // 0b10101010 XOR 0b10100000 = 0b00001010 (2 bits)
        let a = vec![0b10101010u8; 50];
        let b = vec![0b10100000u8; 50];
        assert_eq!(hamming_distance(&a, &b), 50 * 2);
    }

    #[test]
    #[should_panic(expected = "assertion")]
    fn test_hamming_mismatched_lengths_panics() {
        let a = vec![0u8; 10];
        let b = vec![0u8; 20];
        let _ = hamming_distance(&a, &b);
    }
}
