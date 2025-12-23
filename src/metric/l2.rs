//! L2 Squared distance metric.

use super::Metric;

/// L2 Squared (Euclidean Squared) distance metric.
///
/// Calculates `sum((a_i - b_i)^2)`.
/// Does not perform the square root operation, as squared distances preserve ordering
/// and are computationally cheaper.
#[derive(Debug, Clone, Copy, Default)]
pub struct L2Squared;

impl Metric<f32> for L2Squared {
    #[inline]
    fn distance(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(
            a.len(),
            b.len(),
            "dimension mismatch: {} != {}",
            a.len(),
            b.len()
        );

        cfg_if::cfg_if! {
            if #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))] {
                // W30.1: WASM SIMD128 threshold lowered from 256 to 16.
                // WASM SIMD processes 16 floats per iteration, so 16+ dims benefit.
                // The original 256 threshold was from x86_64 AVX2 analysis, not WASM.
                if a.len() < 16 {
                     let mut sum = 0.0;
                     for (x, y) in a.iter().zip(b.iter()) {
                         assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                         let diff = x - y;
                         sum += diff * diff;
                     }
                     return sum;
                }
                let result = super::simd::wasm::l2_squared(a, b);
                assert!(!result.is_nan(), "NaN detected in input");
                result
            } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
                 // W5.5 Regression Fix: For small dimensions, SIMD overhead > scalar.
                 if a.len() < 256 {
                     let mut sum = 0.0;
                     for (x, y) in a.iter().zip(b.iter()) {
                         assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                         let diff = x - y;
                         sum += diff * diff;
                     }
                     return sum;
                }
                let result = super::simd::x86::l2_squared(a, b);
                // We check result for NaN to satisfy safety contract without O(N) scan
                assert!(!result.is_nan(), "NaN detected in input");
                result
            } else {
                let mut sum = 0.0;
                for (x, y) in a.iter().zip(b.iter()) {
                    assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                    let diff = x - y;
                    sum += diff * diff;
                }
                sum
            }
        }
    }
}
