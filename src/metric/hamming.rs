//! Hamming distance metric.

use super::Metric;

/// Hamming distance metric.
///
/// Calculates the number of differing bits between two binary vectors.
/// Uses SIMD acceleration (WASM SIMD128 or AVX2) when available.
///
/// # Attribution
///
/// Adapted from `binary_semantic_cache` v1.0 (MIT License)
/// Copyright (c) 2024 Matteo Panzeri
/// Original: <https://github.com/mp-monitor/binary_semantic_cache>
#[derive(Debug, Clone, Copy, Default)]
pub struct Hamming;

impl Metric<u8> for Hamming {
    #[inline]
    fn distance(a: &[u8], b: &[u8]) -> f32 {
        assert_eq!(
            a.len(),
            b.len(),
            "dimension mismatch: {} != {}",
            a.len(),
            b.len()
        );

        // Use SIMD-accelerated Hamming distance when available
        // Dispatcher in simd.rs selects WASM SIMD128, AVX2, or scalar fallback
        let distance = super::simd::hamming_distance(a, b);

        // Precision loss is acceptable because max distance for expected vector sizes
        // (< 1MB) fits within f32 mantissa (2^24).
        #[allow(clippy::cast_precision_loss)]
        {
            distance as f32
        }
    }
}
