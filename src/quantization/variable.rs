//! Variable-dimension binary quantization.
//!
//! This module provides binary quantization for vectors of any dimension
//! divisible by 8, unlike the fixed 768-dimension `BinaryQuantizer`.
//!
//! # Memory Savings
//!
//! Binary quantization provides 32x compression:
//! - 768D F32 vector: 3072 bytes
//! - 768D binary vector: 96 bytes
//!
//! # Dimension Support
//!
//! Supports any dimension where `dimension % 8 == 0`:
//! - 128D (common for small models)
//! - 384D (sentence-transformers/all-MiniLM-L6-v2)
//! - 768D (OpenAI ada-002, BERT)
//! - 1024D (larger models)
//! - 1536D (OpenAI text-embedding-3-small)
//!
//! # Example
//!
//! ```
//! use edgevec::quantization::variable::BinaryVector;
//!
//! // Quantize a 128-dimensional vector
//! let vector = vec![1.0f32; 128];
//! let bv = BinaryVector::quantize(&vector).unwrap();
//!
//! assert_eq!(bv.dimension(), 128);
//! assert_eq!(bv.bytes(), 16);
//!
//! // Compute Hamming distance
//! let other = vec![-1.0f32; 128];
//! let bv2 = BinaryVector::quantize(&other).unwrap();
//! let distance = bv.hamming_distance(&bv2).unwrap();
//! assert_eq!(distance, 128); // All bits differ
//! ```

use std::fmt;

/// Error type for quantization operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantizationError {
    /// Dimension must be divisible by 8.
    InvalidDimension {
        /// The invalid dimension provided.
        dimension: usize,
    },
    /// Dimension mismatch between vectors.
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension provided.
        actual: usize,
    },
    /// Byte length doesn't match expected dimension.
    ByteLengthMismatch {
        /// Expected byte length (dimension / 8).
        expected: usize,
        /// Actual byte length provided.
        actual: usize,
    },
}

impl fmt::Display for QuantizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension { dimension } => {
                write!(
                    f,
                    "dimension must be divisible by 8, got {dimension}. \
                     Try using a dimension like 128, 384, 768, 1024, or 1536."
                )
            }
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "dimension mismatch: expected {expected}, got {actual}")
            }
            Self::ByteLengthMismatch { expected, actual } => {
                write!(f, "byte length mismatch: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for QuantizationError {}

/// Variable-dimension binary quantized vector.
///
/// Each bit represents the sign of the original f32 value:
/// - Bit = 1 if f32 > 0.0
/// - Bit = 0 if f32 <= 0.0
///
/// # Memory Layout
///
/// Bits are packed in little-endian order:
/// - Byte 0 contains bits [0..8]
/// - Byte 1 contains bits [8..16]
/// - etc.
///
/// # Example
///
/// ```
/// use edgevec::quantization::variable::BinaryVector;
///
/// let v = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0]; // 8D
/// let bv = BinaryVector::quantize(&v).unwrap();
///
/// assert_eq!(bv.dimension(), 8);
/// assert_eq!(bv.bytes(), 1);
/// assert_eq!(bv.data()[0], 0b01010101); // 0x55
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct BinaryVector {
    /// Packed binary data.
    data: Vec<u8>,
    /// Dimension of the original vector (in bits).
    dimension: usize,
}

impl BinaryVector {
    /// Quantizes a vector to binary representation.
    ///
    /// # Arguments
    ///
    /// * `vector` - The f32 vector to quantize.
    ///
    /// # Returns
    ///
    /// A `BinaryVector` where each bit represents the sign of the
    /// corresponding f32 value.
    ///
    /// # Errors
    ///
    /// Returns `QuantizationError::InvalidDimension` if `vector.len() % 8 != 0`.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::variable::BinaryVector;
    ///
    /// let v = vec![1.0f32; 768];
    /// let bv = BinaryVector::quantize(&v).unwrap();
    ///
    /// // All positive -> all bits set
    /// assert!(bv.data().iter().all(|&b| b == 0xFF));
    /// ```
    pub fn quantize(vector: &[f32]) -> Result<Self, QuantizationError> {
        let dimension = vector.len();

        if dimension == 0 || dimension % 8 != 0 {
            return Err(QuantizationError::InvalidDimension { dimension });
        }

        let bytes = dimension / 8;
        let mut data = vec![0u8; bytes];

        // Build each packed byte once instead of repeatedly updating it for
        // individual components. This also avoids a branch for every sign bit.
        for (byte, values) in data.iter_mut().zip(vector.chunks_exact(8)) {
            *byte = u8::from(values[0] > 0.0)
                | (u8::from(values[1] > 0.0) << 1)
                | (u8::from(values[2] > 0.0) << 2)
                | (u8::from(values[3] > 0.0) << 3)
                | (u8::from(values[4] > 0.0) << 4)
                | (u8::from(values[5] > 0.0) << 5)
                | (u8::from(values[6] > 0.0) << 6)
                | (u8::from(values[7] > 0.0) << 7);
        }

        Ok(Self { data, dimension })
    }

    /// Creates a `BinaryVector` from raw bytes with known dimension.
    ///
    /// # Arguments
    ///
    /// * `data` - The packed binary data.
    /// * `dimension` - The dimension of the vector (must equal `data.len() * 8`).
    ///
    /// # Errors
    ///
    /// - `QuantizationError::InvalidDimension` if `dimension % 8 != 0`.
    /// - `QuantizationError::ByteLengthMismatch` if `data.len() != dimension / 8`.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::variable::BinaryVector;
    ///
    /// let data = vec![0xFF; 16]; // 128 bits
    /// let bv = BinaryVector::from_bytes(data, 128).unwrap();
    ///
    /// assert_eq!(bv.dimension(), 128);
    /// ```
    pub fn from_bytes(data: Vec<u8>, dimension: usize) -> Result<Self, QuantizationError> {
        if dimension == 0 || dimension % 8 != 0 {
            return Err(QuantizationError::InvalidDimension { dimension });
        }

        let expected_bytes = dimension / 8;
        if data.len() != expected_bytes {
            return Err(QuantizationError::ByteLengthMismatch {
                expected: expected_bytes,
                actual: data.len(),
            });
        }

        Ok(Self { data, dimension })
    }

    /// Returns the dimension of this vector (in bits).
    #[must_use]
    #[inline]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Returns the size in bytes.
    #[must_use]
    #[inline]
    pub fn bytes(&self) -> usize {
        self.data.len()
    }

    /// Returns the underlying binary data.
    #[must_use]
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Computes the Hamming distance to another vector.
    ///
    /// Hamming distance is the number of differing bits between two vectors.
    ///
    /// # Arguments
    ///
    /// * `other` - Another `BinaryVector` of the same dimension.
    ///
    /// # Returns
    ///
    /// The number of differing bits (0..=dimension).
    ///
    /// # Errors
    ///
    /// Returns `QuantizationError::DimensionMismatch` if dimensions differ.
    ///
    /// # Performance
    ///
    /// Uses SIMD-accelerated popcount when available.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::variable::BinaryVector;
    ///
    /// let v1 = vec![1.0f32; 128];
    /// let v2 = vec![-1.0f32; 128];
    ///
    /// let bv1 = BinaryVector::quantize(&v1).unwrap();
    /// let bv2 = BinaryVector::quantize(&v2).unwrap();
    ///
    /// let distance = bv1.hamming_distance(&bv2).unwrap();
    /// assert_eq!(distance, 128); // All bits differ
    /// ```
    pub fn hamming_distance(&self, other: &Self) -> Result<u32, QuantizationError> {
        if self.dimension != other.dimension {
            return Err(QuantizationError::DimensionMismatch {
                expected: self.dimension,
                actual: other.dimension,
            });
        }

        // Use SIMD popcount for variable-length data
        Ok(crate::simd::popcount::simd_popcount_xor(
            &self.data,
            &other.data,
        ))
    }

    /// Returns similarity score [0, 1] based on Hamming distance.
    ///
    /// - 1.0 = identical vectors (distance = 0)
    /// - 0.0 = completely opposite vectors (distance = dimension)
    ///
    /// # Arguments
    ///
    /// * `other` - Another `BinaryVector` of the same dimension.
    ///
    /// # Errors
    ///
    /// Returns `QuantizationError::DimensionMismatch` if dimensions differ.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::variable::BinaryVector;
    ///
    /// let v1 = vec![1.0f32; 128];
    /// let v2 = vec![1.0f32; 128];
    ///
    /// let bv1 = BinaryVector::quantize(&v1).unwrap();
    /// let bv2 = BinaryVector::quantize(&v2).unwrap();
    ///
    /// let sim = bv1.similarity(&bv2).unwrap();
    /// assert!((sim - 1.0).abs() < f32::EPSILON);
    /// ```
    #[allow(clippy::cast_precision_loss)] // dimension fits easily in f32
    pub fn similarity(&self, other: &Self) -> Result<f32, QuantizationError> {
        let distance = self.hamming_distance(other)?;
        Ok(1.0 - (distance as f32 / self.dimension as f32))
    }
}

impl fmt::Debug for BinaryVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BinaryVector")
            .field("dimension", &self.dimension)
            .field("bytes", &self.data.len())
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_128d() {
        let v = vec![1.0f32; 128];
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.dimension(), 128);
        assert_eq!(bv.bytes(), 16);
        assert!(bv.data().iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_quantize_384d() {
        let v = vec![-1.0f32; 384];
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.dimension(), 384);
        assert_eq!(bv.bytes(), 48);
        assert!(bv.data().iter().all(|&b| b == 0x00));
    }

    #[test]
    fn test_quantize_768d() {
        let v: Vec<f32> = (0..768)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.dimension(), 768);
        assert_eq!(bv.bytes(), 96);
        // 0b01010101 = 0x55
        assert!(bv.data().iter().all(|&b| b == 0x55));
    }

    #[test]
    fn test_quantize_all_sign_patterns_and_zero() {
        // All 256 possible packed bytes, including both signed zeros and
        // non-finite values whose comparison semantics must stay unchanged.
        let nonpositive = [-1.0, 0.0, -0.0, f32::NEG_INFINITY, f32::NAN];
        for &off in &nonpositive {
            let vector: Vec<f32> = (0u16..256)
                .flat_map(|pattern| {
                    (0..8).map(move |bit| {
                        if pattern & (1 << bit) == 0 {
                            off
                        } else {
                            f32::INFINITY
                        }
                    })
                })
                .collect();
            let packed = BinaryVector::quantize(&vector).unwrap();
            let expected: Vec<u8> = (0u8..=255).collect();
            assert_eq!(packed.data(), expected);
        }
    }

    #[test]
    fn test_quantize_1024d() {
        let v = vec![1.0f32; 1024];
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.dimension(), 1024);
        assert_eq!(bv.bytes(), 128);
    }

    #[test]
    fn test_quantize_1536d() {
        let v = vec![1.0f32; 1536];
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.dimension(), 1536);
        assert_eq!(bv.bytes(), 192);
    }

    #[test]
    fn test_invalid_dimension_not_divisible_by_8() {
        let v = vec![1.0f32; 100];
        let result = BinaryVector::quantize(&v);
        assert!(matches!(
            result,
            Err(QuantizationError::InvalidDimension { dimension: 100 })
        ));
    }

    #[test]
    fn test_invalid_dimension_zero() {
        let v: Vec<f32> = vec![];
        let result = BinaryVector::quantize(&v);
        assert!(matches!(
            result,
            Err(QuantizationError::InvalidDimension { dimension: 0 })
        ));
    }

    #[test]
    fn test_from_bytes_valid() {
        let data = vec![0xFF; 16];
        let bv = BinaryVector::from_bytes(data, 128).unwrap();
        assert_eq!(bv.dimension(), 128);
        assert_eq!(bv.bytes(), 16);
    }

    #[test]
    fn test_from_bytes_length_mismatch() {
        let data = vec![0xFF; 16];
        let result = BinaryVector::from_bytes(data, 256);
        assert!(matches!(
            result,
            Err(QuantizationError::ByteLengthMismatch {
                expected: 32,
                actual: 16
            })
        ));
    }

    #[test]
    fn test_hamming_distance_zero() {
        let v = vec![1.0f32; 128];
        let bv = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv.hamming_distance(&bv).unwrap(), 0);
    }

    #[test]
    fn test_hamming_distance_max() {
        let v1 = vec![1.0f32; 128];
        let v2 = vec![-1.0f32; 128];
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();
        assert_eq!(bv1.hamming_distance(&bv2).unwrap(), 128);
    }

    #[test]
    fn test_hamming_distance_symmetric() {
        let v1: Vec<f32> = (0..128).map(|i| (i as f32).sin()).collect();
        let v2: Vec<f32> = (0..128).map(|i| (i as f32).cos()).collect();
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();
        assert_eq!(
            bv1.hamming_distance(&bv2).unwrap(),
            bv2.hamming_distance(&bv1).unwrap()
        );
    }

    #[test]
    fn test_hamming_distance_dimension_mismatch() {
        let v1 = vec![1.0f32; 128];
        let v2 = vec![1.0f32; 256];
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();
        let result = bv1.hamming_distance(&bv2);
        assert!(matches!(
            result,
            Err(QuantizationError::DimensionMismatch {
                expected: 128,
                actual: 256
            })
        ));
    }

    #[test]
    fn test_similarity_identical() {
        let v = vec![1.0f32; 128];
        let bv = BinaryVector::quantize(&v).unwrap();
        let sim = bv.similarity(&bv).unwrap();
        assert!((sim - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_opposite() {
        let v1 = vec![1.0f32; 128];
        let v2 = vec![-1.0f32; 128];
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();
        let sim = bv1.similarity(&bv2).unwrap();
        assert!(sim.abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_half() {
        let v1 = vec![1.0f32; 128];
        let v2: Vec<f32> = (0..128).map(|i| if i < 64 { 1.0 } else { -1.0 }).collect();
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();
        let sim = bv1.similarity(&bv2).unwrap();
        assert!((sim - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_edge_case_nan() {
        let mut v = vec![1.0f32; 128];
        v[0] = f32::NAN;
        let bv = BinaryVector::quantize(&v).unwrap();
        // NaN > 0.0 is false, so bit 0 should be 0
        assert_eq!(bv.data()[0] & 0x01, 0);
    }

    #[test]
    fn test_edge_case_infinity() {
        let mut v = vec![0.0f32; 128];
        v[0] = f32::INFINITY;
        v[1] = f32::NEG_INFINITY;
        let bv = BinaryVector::quantize(&v).unwrap();
        // +Inf > 0.0 is true (bit 0 = 1)
        // -Inf > 0.0 is false (bit 1 = 0)
        assert_eq!(bv.data()[0] & 0x01, 1);
        assert_eq!(bv.data()[0] & 0x02, 0);
    }

    #[test]
    fn test_quantize_deterministic() {
        let v: Vec<f32> = (0..128).map(|i| (i as f32).sin()).collect();
        let bv1 = BinaryVector::quantize(&v).unwrap();
        let bv2 = BinaryVector::quantize(&v).unwrap();
        assert_eq!(bv1, bv2);
    }

    #[test]
    fn test_debug_format() {
        let v = vec![1.0f32; 128];
        let bv = BinaryVector::quantize(&v).unwrap();
        let debug = format!("{bv:?}");
        assert!(debug.contains("BinaryVector"));
        assert!(debug.contains("dimension: 128"));
        assert!(debug.contains("bytes: 16"));
    }

    #[test]
    fn test_error_display() {
        let err = QuantizationError::InvalidDimension { dimension: 100 };
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("divisible by 8"));
    }
}
