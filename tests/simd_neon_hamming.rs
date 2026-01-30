//! NEON Hamming Distance Property Tests
//!
//! Tests verifying correctness of NEON hamming distance implementation.
//! On x86, tests verify the portable implementation.
//! On ARM64, tests verify NEON matches portable exactly.

use edgevec::quantization::simd::portable::hamming_distance_slice;
use proptest::prelude::*;

// =============================================================================
// Portable Implementation Tests (Run on all platforms)
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property: Hamming distance is symmetric
    #[test]
    fn prop_hamming_symmetric(
        a in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let b: Vec<u8> = a.iter().map(|x| x.wrapping_add(1)).collect();

        let ab = hamming_distance_slice(&a, &b);
        let ba = hamming_distance_slice(&b, &a);

        prop_assert_eq!(ab, ba, "Hamming distance should be symmetric");
    }

    /// Property: Distance to self is zero
    #[test]
    fn prop_hamming_self_zero(
        a in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let distance = hamming_distance_slice(&a, &a);
        prop_assert_eq!(distance, 0, "Distance to self should be zero");
    }

    /// Property: Distance is bounded by bits available
    #[test]
    fn prop_hamming_bounded(
        a in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let b: Vec<u8> = a.iter().map(|x| x.wrapping_add(42)).collect();
        let distance = hamming_distance_slice(&a, &b);
        let max_bits = (a.len() * 8) as u32;

        prop_assert!(distance <= max_bits, "Distance {} exceeds max bits {}", distance, max_bits);
    }

    /// Property: All bits differ when XOR is all ones
    #[test]
    fn prop_hamming_all_different(
        len in 0usize..256,
    ) {
        let a = vec![0x00u8; len];
        let b = vec![0xFFu8; len];
        let distance = hamming_distance_slice(&a, &b);
        let expected = (len * 8) as u32;

        prop_assert_eq!(distance, expected, "All bits should differ for 0x00 vs 0xFF");
    }

    /// Property: Arbitrary pairs match manual calculation
    #[test]
    fn prop_hamming_matches_manual(
        a in prop::collection::vec(any::<u8>(), 1..64),
    ) {
        let b: Vec<u8> = a.iter().map(|x| x.wrapping_add(7)).collect();

        let computed = hamming_distance_slice(&a, &b);
        let manual: u32 = a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum();

        prop_assert_eq!(computed, manual, "Computed should match manual calculation");
    }
}

// =============================================================================
// Edge Case Tests (All platforms)
// =============================================================================

#[test]
fn test_portable_empty_slices() {
    let a: Vec<u8> = vec![];
    let b: Vec<u8> = vec![];
    assert_eq!(hamming_distance_slice(&a, &b), 0);
}

#[test]
fn test_portable_single_byte_all_bits() {
    let a = vec![0b11111111u8];
    let b = vec![0b00000000u8];
    assert_eq!(hamming_distance_slice(&a, &b), 8);
}

#[test]
fn test_portable_single_byte_half_bits() {
    let a = vec![0b11110000u8];
    let b = vec![0b00001111u8];
    assert_eq!(hamming_distance_slice(&a, &b), 8);
}

#[test]
fn test_portable_exactly_16_bytes() {
    // Exactly one NEON vector width
    let a = vec![0xFFu8; 16];
    let b = vec![0x00u8; 16];
    assert_eq!(hamming_distance_slice(&a, &b), 128); // 16 * 8 bits
}

#[test]
fn test_portable_exactly_15_bytes() {
    // Just under one NEON vector (tail only)
    let a = vec![0xFFu8; 15];
    let b = vec![0x00u8; 15];
    assert_eq!(hamming_distance_slice(&a, &b), 120); // 15 * 8 bits
}

#[test]
fn test_portable_exactly_17_bytes() {
    // One NEON vector + 1 tail byte
    let a = vec![0xFFu8; 17];
    let b = vec![0x00u8; 17];
    assert_eq!(hamming_distance_slice(&a, &b), 136); // 17 * 8 bits
}

#[test]
fn test_portable_large_1000_bytes() {
    let a = vec![0xAAu8; 1000];
    let b = vec![0x55u8; 1000];
    assert_eq!(hamming_distance_slice(&a, &b), 8000); // 1000 * 8 bits all differ
}

#[test]
fn test_portable_identical_slices() {
    let a = vec![42u8; 100];
    let b = a.clone();
    assert_eq!(hamming_distance_slice(&a, &b), 0);
}

#[test]
fn test_portable_96_bytes_standard() {
    // Standard 768-bit binary embedding size
    let a = [0xAAu8; 96];
    let b = [0x55u8; 96];
    assert_eq!(hamming_distance_slice(&a, &b), 768);
}

// =============================================================================
// NEON-specific tests (Only run on ARM64)
// =============================================================================

#[cfg(target_arch = "aarch64")]
mod neon_tests {
    use super::*;
    use edgevec::simd::neon;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property: NEON matches portable for all inputs
        #[test]
        fn prop_neon_matches_portable(
            a in prop::collection::vec(any::<u8>(), 0..1024),
        ) {
            let b: Vec<u8> = a.iter().map(|x| x.wrapping_add(1)).collect();

            let neon_result = neon::hamming_distance_slice(&a, &b);
            let portable_result = hamming_distance_slice(&a, &b);

            prop_assert_eq!(
                neon_result, portable_result,
                "NEON ({}) != Portable ({}) for len={}",
                neon_result, portable_result, a.len()
            );
        }

        /// Property: NEON is symmetric
        #[test]
        fn prop_neon_symmetric(
            a in prop::collection::vec(any::<u8>(), 0..256),
        ) {
            let b: Vec<u8> = a.iter().map(|x| x.wrapping_add(1)).collect();

            let ab = neon::hamming_distance_slice(&a, &b);
            let ba = neon::hamming_distance_slice(&b, &a);

            prop_assert_eq!(ab, ba, "NEON hamming distance should be symmetric");
        }
    }

    #[test]
    fn test_neon_empty_slices() {
        let a: Vec<u8> = vec![];
        let b: Vec<u8> = vec![];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 0);
    }

    #[test]
    fn test_neon_single_byte() {
        let a = vec![0b11111111u8];
        let b = vec![0b00000000u8];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 8);
    }

    #[test]
    fn test_neon_exactly_16_bytes() {
        let a = vec![0xFFu8; 16];
        let b = vec![0x00u8; 16];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 128);
    }

    #[test]
    fn test_neon_exactly_17_bytes_with_tail() {
        let a = vec![0xFFu8; 17];
        let b = vec![0x00u8; 17];
        assert_eq!(neon::hamming_distance_slice(&a, &b), 136);
    }

    #[test]
    fn test_neon_identical_slices() {
        let a = vec![42u8; 100];
        let b = a.clone();
        assert_eq!(neon::hamming_distance_slice(&a, &b), 0);
    }

    #[test]
    fn test_neon_96_bytes_matches_fixed() {
        // Verify slice and fixed-size produce same result
        let a = [0xAAu8; 96];
        let b = [0x55u8; 96];
        assert_eq!(
            neon::hamming_distance(&a, &b),
            neon::hamming_distance_slice(&a, &b)
        );
    }

    #[test]
    fn test_neon_matches_portable_various_sizes() {
        // Test specific sizes that exercise NEON vector boundaries
        for size in [0, 1, 15, 16, 17, 31, 32, 33, 64, 96, 100, 128, 1000] {
            let a: Vec<u8> = (0..size).map(|i| i as u8).collect();
            let b: Vec<u8> = (0..size).map(|i| (i + 1) as u8).collect();

            let neon_result = neon::hamming_distance_slice(&a, &b);
            let portable_result = hamming_distance_slice(&a, &b);

            assert_eq!(
                neon_result, portable_result,
                "NEON != Portable for size={}: {} != {}",
                size, neon_result, portable_result
            );
        }
    }
}
