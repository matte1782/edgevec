use serde::{Deserialize, Serialize};
use std::vec::Vec;

/// Configuration for Scalar Quantization (SQ8).
///
/// # Algorithm: Min-Max Normalization
/// Formula: `u8 = (f32 - min) / (max - min) * 255`
///
/// # Storage
/// Global min/max per index for v1. Simplicity over per-segment optimization.
///
/// # Zero Mapping
/// If min=0.0 and max=1.0, then 0.0 maps strictly to 0.
#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct QuantizerConfig {
    /// Global minimum value observed/configured
    pub min: f32, // offset 0
    /// Global maximum value observed/configured
    pub max: f32, // offset 4
}

/// Scalar Quantizer implementation using global min/max normalization.
///
/// Maps f32 values to u8 in the range [0, 255].
/// Values outside [min, max] are clamped.
#[derive(Clone, Debug)]
pub struct ScalarQuantizer {
    config: QuantizerConfig,
}

impl ScalarQuantizer {
    /// Create a new quantizer with specific config.
    #[must_use]
    pub fn new(config: QuantizerConfig) -> Self {
        Self { config }
    }

    /// Train the quantizer on a batch of vectors to find global min/max.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Slice of vectors (slices of f32)
    ///
    /// # Returns
    ///
    /// A new `ScalarQuantizer` initialized with the observed range.
    ///
    /// # Edge Cases
    ///
    /// - If `vectors` is empty, returns default 0.0..1.0 range.
    /// - If all values are equal (min == max), returns that value as both min and max.
    #[must_use]
    pub fn train(vectors: &[&[f32]]) -> Self {
        if vectors.is_empty() {
            return Self {
                config: QuantizerConfig { min: 0.0, max: 1.0 },
            };
        }

        let mut min = f32::MAX;
        let mut max = f32::MIN;

        for vec in vectors {
            for &val in *vec {
                if val < min {
                    min = val;
                }
                if val > max {
                    max = val;
                }
            }
        }

        // Handle case where no valid values were seen (e.g. all NaNs or empty slices)
        if min > max {
            return Self {
                config: QuantizerConfig { min: 0.0, max: 1.0 },
            };
        }

        Self {
            config: QuantizerConfig { min, max },
        }
    }

    /// Quantize a vector from f32 to u8.
    ///
    /// # Formula
    /// `u8 = (val - min) / (max - min) * 255`
    ///
    /// # Behavior
    /// - Outliers (val < min or val > max) are clamped.
    /// - NaN values are treated as min (0).
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn quantize(&self, vector: &[f32]) -> Vec<u8> {
        let range = self.config.max - self.config.min;

        if range.abs() < f32::EPSILON {
            return vec![0u8; vector.len()];
        }

        let scale = 255.0 / range;
        let min = self.config.min;

        let mut out = Vec::with_capacity(vector.len());
        for &val in vector {
            let norm = (val - min) * scale;
            let quantized = norm.round().clamp(0.0, 255.0);
            out.push(quantized as u8);
        }
        out
    }

    /// Reconstruct f32 vector from quantized u8.
    ///
    /// # Formula
    /// `f32 = min + (u8 / 255) * (max - min)`
    #[must_use]
    #[allow(clippy::cast_lossless)]
    pub fn dequantize(&self, quantized: &[u8]) -> Vec<f32> {
        let range = self.config.max - self.config.min;
        let min = self.config.min;
        let scale = range / 255.0;

        let mut out = Vec::with_capacity(quantized.len());
        for &q in quantized {
            let val = min + (f32::from(q) * scale);
            out.push(val);
        }
        out
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> QuantizerConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_train_finds_min_max() {
        let v1 = vec![1.0, 5.0, -2.0];
        let v2 = vec![0.0, 10.0, 3.0];
        let vectors = vec![v1.as_slice(), v2.as_slice()];

        let q = ScalarQuantizer::train(&vectors);

        assert_eq!(q.config.min, -2.0);
        assert_eq!(q.config.max, 10.0);
    }

    #[test]
    fn test_quantize_dequantize_roundtrip() {
        let config = QuantizerConfig {
            min: 0.0,
            max: 10.0,
        };
        let q = ScalarQuantizer::new(config);

        let original = vec![0.0, 2.5, 5.0, 7.5, 10.0];
        let encoded = q.quantize(&original);

        let decoded = q.dequantize(&encoded);

        for (orig, dec) in original.iter().zip(decoded.iter()) {
            let diff = (*orig - *dec).abs();
            assert!(diff < 0.05, "Diff too large: {orig} vs {dec}");
        }
    }

    #[test]
    fn test_outliers_clamped() {
        let config = QuantizerConfig {
            min: 0.0,
            max: 10.0,
        };
        let q = ScalarQuantizer::new(config);

        let input = vec![-5.0, 15.0];
        let encoded = q.quantize(&input);

        assert_eq!(encoded[0], 0); // -5.0 clamped to min -> 0
        assert_eq!(encoded[1], 255); // 15.0 clamped to max -> 255
    }

    #[test]
    fn test_zero_range_safeguard() {
        let vectors = vec![&[5.0, 5.0][..]];
        let q = ScalarQuantizer::train(&vectors);

        // Use approximate comparison for floats
        assert!((q.config.min - 5.0).abs() < f32::EPSILON);
        assert!((q.config.max - 5.0).abs() < f32::EPSILON);

        let input = vec![5.0, 10.0];
        let encoded = q.quantize(&input);

        assert_eq!(encoded, vec![0, 0]);

        let decoded = q.dequantize(&encoded);
        assert!((decoded[0] - 5.0).abs() < f32::EPSILON);
        assert!((decoded[1] - 5.0).abs() < f32::EPSILON);
    }
}
