//! Dot Product distance metric.

use super::Metric;

/// Dot Product metric.
///
/// Calculates `sum(a_i * b_i)`.
/// Used for Cosine Similarity when vectors are normalized.
#[derive(Debug, Clone, Copy, Default)]
pub struct DotProduct;

impl Metric<f32> for DotProduct {
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
                if a.len() < 16 {
                     let mut sum = 0.0;
                     for (x, y) in a.iter().zip(b.iter()) {
                         assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                         sum += x * y;
                     }
                     return sum;
                }
                let result = super::simd::wasm::dot_product(a, b);
                assert!(!result.is_nan(), "NaN detected in input");
                result
            } else if #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))] {
                 if a.len() < 256 {
                     let mut sum = 0.0;
                     for (x, y) in a.iter().zip(b.iter()) {
                         assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                         sum += x * y;
                     }
                     return sum;
                }
                let result = super::simd::x86::dot_product(a, b);
                assert!(!result.is_nan(), "NaN detected in input");
                result
            } else {
                let mut sum = 0.0;
                for (x, y) in a.iter().zip(b.iter()) {
                    assert!(!(x.is_nan() || y.is_nan()), "NaN detected in input");
                    sum += x * y;
                }
                sum
            }
        }
    }
}
