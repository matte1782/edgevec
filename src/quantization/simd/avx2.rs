//! AVX2-accelerated Hamming distance for x86_64.
//!
//! Provides SIMD implementation using Intel AVX2 instructions (256-bit registers).
//!
//! # CPU Requirements
//!
//! - AVX2 support (Intel Haswell 2013+, AMD Excavator 2015+)
//! - Runtime detection via `is_x86_feature_detected!("avx2")`
//!
//! # Performance Target
//!
//! <50 CPU cycles per comparison (vs ~300 cycles portable)
//!
//! # Algorithm
//!
//! 1. Load 96 bytes in 3 × 256-bit YMM registers
//! 2. XOR corresponding registers to find differing bits
//! 3. Population count using native popcnt instruction (extract 4×u64, sum count_ones())
//! 4. Sum results across all registers
//!
//! # Safety
//!
//! All functions in this module are marked `unsafe` and require:
//! 1. AVX2 CPU feature verified by caller
//! 2. Input arrays are exactly 96 bytes
//! 3. No undefined behavior in pointer arithmetic

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{__m256i, _mm256_extract_epi64, _mm256_loadu_si256, _mm256_xor_si256};

/// AVX2-accelerated Hamming distance for 96-byte binary vectors.
///
/// # Safety
///
/// Caller MUST ensure:
/// 1. AVX2 is available (`is_x86_feature_detected!("avx2")` returned true)
/// 2. Both arrays are valid `[u8; 96]` arrays
/// 3. No aliasing violations (enforced by Rust's borrow checker)
///
/// These invariants are enforced by the public API in `simd/mod.rs`.
///
/// # Algorithm
///
/// 1. Load 96 bytes in 3 × 256-bit registers:
///    - Register 0: bytes [0..32)
///    - Register 1: bytes [32..64)
///    - Register 2: bytes [64..96)
/// 2. XOR: `vpxor ymm_a, ymm_b` → differing bits become 1
/// 3. Popcount: Lookup table method (AVX2 has no native popcount)
/// 4. Horizontal sum: Sum all partial popcounts
///
/// # Performance
///
/// Target: <50 CPU cycles
///
/// # Arguments
///
/// * `a` - First 96-byte array (768 bits), 64-byte aligned
/// * `b` - Second 96-byte array (768 bits), 64-byte aligned
///
/// # Returns
///
/// Number of differing bits (0..=768)
#[target_feature(enable = "avx2")]
#[cfg(target_arch = "x86_64")]
#[allow(clippy::cast_ptr_alignment)] // _mm256_loadu_si256 is designed for unaligned access
pub(crate) unsafe fn hamming_distance_avx2(a: &[u8; 96], b: &[u8; 96]) -> u32 {
    // SAFETY: Caller verified AVX2 is available.
    // Array size (96 bytes) allows loads at offsets 0, 32, 64.
    // QuantizedVector guarantees 64-byte alignment, which exceeds AVX2's 32-byte requirement.

    // Load 96 bytes in 3 × 256-bit registers
    // Using _mm256_loadu_si256 (unaligned load) for safety,
    // though QuantizedVector is 64-byte aligned
    let a0 = _mm256_loadu_si256(a.as_ptr().cast::<__m256i>());
    let a1 = _mm256_loadu_si256(a.as_ptr().add(32).cast::<__m256i>());
    let a2 = _mm256_loadu_si256(a.as_ptr().add(64).cast::<__m256i>());

    let b0 = _mm256_loadu_si256(b.as_ptr().cast::<__m256i>());
    let b1 = _mm256_loadu_si256(b.as_ptr().add(32).cast::<__m256i>());
    let b2 = _mm256_loadu_si256(b.as_ptr().add(64).cast::<__m256i>());

    // XOR to find differing bits
    let xor0 = _mm256_xor_si256(a0, b0);
    let xor1 = _mm256_xor_si256(a1, b1);
    let xor2 = _mm256_xor_si256(a2, b2);

    // Population count for each register
    // AVX2 doesn't have native popcount, so we use lookup table method
    let pop0 = popcount_avx2(xor0);
    let pop1 = popcount_avx2(xor1);
    let pop2 = popcount_avx2(xor2);

    // Sum all popcounts
    pop0 + pop1 + pop2
}

/// AVX2 population count using native popcnt instruction.
///
/// # Algorithm
///
/// Extract 4 × 64-bit values from the 256-bit register and use the native
/// hardware popcnt instruction via count_ones(). This is faster than the
/// PSHUFB lookup table method on modern CPUs (Haswell+) that have popcnt.
///
/// # Safety
///
/// Caller must ensure AVX2 is available via `is_x86_feature_detected!("avx2")`.
///
/// # Arguments
///
/// * `v` - 256-bit register containing XOR result
///
/// # Returns
///
/// Sum of all set bits in the register (0..=256)
#[target_feature(enable = "avx2")]
#[inline]
#[cfg(target_arch = "x86_64")]
#[allow(clippy::cast_sign_loss)] // Values are bit patterns, not signed integers
unsafe fn popcount_avx2(v: __m256i) -> u32 {
    // Extract 4 × 64-bit values and use native popcnt instruction.
    // count_ones() compiles to popcnt on x86_64 with hardware support.
    let a = _mm256_extract_epi64(v, 0) as u64;
    let b = _mm256_extract_epi64(v, 1) as u64;
    let c = _mm256_extract_epi64(v, 2) as u64;
    let d = _mm256_extract_epi64(v, 3) as u64;

    a.count_ones() + b.count_ones() + c.count_ones() + d.count_ones()
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod tests {
    use super::*;

    // Helper to check if AVX2 is available
    fn avx2_available() -> bool {
        is_x86_feature_detected!("avx2")
    }

    #[test]
    fn test_avx2_identical() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let a = [0xAA; 96];
        let b = [0xAA; 96];

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 0);
    }

    #[test]
    fn test_avx2_opposite() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let a = [0x00; 96];
        let b = [0xFF; 96];

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 768);
    }

    #[test]
    fn test_avx2_alternating() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let a = [0xAA; 96]; // 10101010...
        let b = [0x55; 96]; // 01010101...

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 768);
    }

    #[test]
    fn test_avx2_single_bit() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let mut a = [0x00; 96];
        let b = [0x00; 96];
        a[0] = 0x01; // Only bit 0 differs

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 1);
    }

    #[test]
    fn test_avx2_boundary_32() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let mut a = [0x00; 96];
        let b = [0x00; 96];
        a[31] = 0xFF; // Last byte of first register
        a[32] = 0xFF; // First byte of second register

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 16); // 8 bits × 2 bytes
    }

    #[test]
    fn test_avx2_boundary_64() {
        if !avx2_available() {
            eprintln!("Skipping AVX2 test: CPU does not support AVX2");
            return;
        }

        let mut a = [0x00; 96];
        let b = [0x00; 96];
        a[63] = 0xFF; // Last byte of second register
        a[64] = 0xFF; // First byte of third register

        let distance = unsafe { hamming_distance_avx2(&a, &b) };
        assert_eq!(distance, 16);
    }
}
