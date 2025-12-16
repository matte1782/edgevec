//! Binary Quantization for vector compression.
//!
//! Compresses 768-dimensional f32 vectors into 96-byte binary representations
//! using sign-based quantization with Hamming distance computation.
//!
//! # Algorithm
//!
//! For each dimension `i` in the input vector:
//! - If `vector[i] > 0.0`, set bit `i` to 1
//! - Otherwise, set bit `i` to 0
//!
//! # Binary Format Specification
//!
//! The output is a fixed 96-byte array with the following layout:
//!
//! | Bytes | Bits | Description |
//! |-------|------|-------------|
//! | 0-95  | 0-767 | Packed sign bits |
//!
//! **Bit Ordering (Little-Endian):**
//! - Byte 0 contains bits [0..8] of the source vector
//! - Bit 0 (LSB) = dimension 0, Bit 7 (MSB) = dimension 7
//! - Byte 1 contains bits [8..16], and so on
//!
//! **Example:**
//! ```text
//! Source vector: [+, -, +, -, +, -, +, -, ...]
//! Bit pattern:   [1, 0, 1, 0, 1, 0, 1, 0, ...]
//! Byte 0:        0b01010101 = 0x55
//! ```
//!
//! # Complexity
//!
//! | Operation | Time | Space |
//! |-----------|------|-------|
//! | Quantize | O(n) | O(n/8) |
//! | Hamming Distance | O(n/8) | O(1) |
//! | Similarity | O(n/8) | O(1) |
//!
//! Where n = dimension (768 for standard embeddings).
//!
//! # Performance
//!
//! - Compression ratio: 8x (768 f32 = 3072 bytes → 96 bytes)
//! - Quantization: O(n) time, O(n/8) space
//! - Hamming distance: O(n/8) time, O(1) space
//! - Target: <50 CPU cycles per comparison (with SIMD in W9.3)
//!
//! # SIMD Alignment
//!
//! The `QuantizedVector` struct is 64-byte aligned (`#[repr(C, align(64))]`)
//! for compatibility with AVX-512 SIMD operations. This alignment is
//! guaranteed by the Rust compiler and applies to both stack and heap
//! allocations when using `Box<QuantizedVector>`.
//!
//! # Special Value Handling
//!
//! | Input Value | Bit Value | Reason |
//! |-------------|-----------|--------|
//! | NaN | 0 | `NaN > 0.0` is false (IEEE 754) |
//! | +Inf | 1 | `+Inf > 0.0` is true |
//! | -Inf | 0 | `-Inf > 0.0` is false |
//! | -0.0 | 0 | `-0.0 > 0.0` is false |
//! | Subnormal | depends | Compared normally |

// SPDX-License-Identifier: MIT
// Adapted from binary_semantic_cache v1.0 (MIT License)
// Copyright (c) 2024 Matteo Panzeri
// Original: https://github.com/MatteoPossamai/binary_semantic_cache

/// The expected dimension for binary quantization (768D embeddings).
pub const BINARY_QUANTIZATION_DIM: usize = 768;

/// The size of a quantized vector in bytes (768 bits = 96 bytes).
pub const QUANTIZED_VECTOR_SIZE: usize = 96;

/// Quantized binary vector (96 bytes, 768 bits).
///
/// Each bit represents the sign of the original f32 value:
/// - Bit = 1 if f32 > 0.0
/// - Bit = 0 if f32 <= 0.0
///
/// # Memory Layout
///
/// The struct is aligned to 64 bytes for SIMD compatibility.
///
/// # Example
///
/// ```
/// use edgevec::quantization::binary::{BinaryQuantizer, QuantizedVector};
///
/// let quantizer = BinaryQuantizer::new();
/// let vector = vec![1.0f32; 768];
/// let quantized = quantizer.quantize(&vector);
///
/// // All positive values result in all bits set
/// assert_eq!(quantized.data()[0], 0xFF);
/// ```
#[repr(C, align(64))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantizedVector {
    /// Packed binary data (768 bits = 96 bytes).
    data: [u8; QUANTIZED_VECTOR_SIZE],
}

impl QuantizedVector {
    /// Creates a new `QuantizedVector` from raw bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - Exactly 96 bytes of packed binary data.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::QuantizedVector;
    ///
    /// let data = [0u8; 96];
    /// let qv = QuantizedVector::from_bytes(data);
    /// assert_eq!(qv.data(), &[0u8; 96]);
    /// ```
    #[must_use]
    pub const fn from_bytes(data: [u8; QUANTIZED_VECTOR_SIZE]) -> Self {
        Self { data }
    }

    /// Returns a reference to the underlying byte data.
    ///
    /// # Returns
    ///
    /// A reference to the 96-byte array containing the packed binary representation.
    /// Each byte contains 8 bits of quantized data in little-endian bit order.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::QuantizedVector;
    ///
    /// let qv = QuantizedVector::from_bytes([0xAA; 96]);
    /// let data = qv.data();
    /// assert_eq!(data.len(), 96);
    /// assert_eq!(data[0], 0xAA);
    /// ```
    #[must_use]
    pub const fn data(&self) -> &[u8; QUANTIZED_VECTOR_SIZE] {
        &self.data
    }

    /// Computes the Hamming distance to another quantized vector.
    ///
    /// Hamming distance is the number of differing bits between two vectors.
    ///
    /// # Algorithm
    ///
    /// 1. XOR the two binary vectors (differing bits become 1)
    /// 2. Count the number of 1 bits (popcount)
    ///
    /// # Performance
    ///
    /// Automatically uses the fastest available implementation:
    /// - AVX2 (x86_64 with AVX2): ~47 CPU cycles
    /// - Portable fallback: ~300 CPU cycles
    ///
    /// Implementation uses runtime CPU feature detection for optimal performance
    /// across all platforms.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::QuantizedVector;
    ///
    /// let v1 = QuantizedVector::from_bytes([0x00; 96]);
    /// let v2 = QuantizedVector::from_bytes([0xFF; 96]);
    ///
    /// // All 768 bits differ
    /// assert_eq!(v1.hamming_distance(&v2), 768);
    /// ```
    #[must_use]
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        // Delegate to SIMD dispatcher
        // This automatically selects AVX2 on capable hardware or portable fallback
        crate::quantization::simd::hamming_distance(&self.data, &other.data)
    }

    /// Returns the Hamming distance as a normalized similarity score [0, 1].
    ///
    /// - 1.0 = identical vectors (distance = 0)
    /// - 0.0 = completely opposite vectors (distance = 768)
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::QuantizedVector;
    ///
    /// let v1 = QuantizedVector::from_bytes([0xAA; 96]); // 10101010...
    /// let v2 = QuantizedVector::from_bytes([0xAA; 96]); // same pattern
    ///
    /// assert!((v1.similarity(&v2) - 1.0).abs() < f32::EPSILON);
    /// ```
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // 768 and distances up to 768 fit easily in f32's 23-bit mantissa
    pub fn similarity(&self, other: &Self) -> f32 {
        let distance = self.hamming_distance(other);
        1.0 - (distance as f32 / BINARY_QUANTIZATION_DIM as f32)
    }
}

impl Default for QuantizedVector {
    fn default() -> Self {
        Self {
            data: [0u8; QUANTIZED_VECTOR_SIZE],
        }
    }
}

/// Binary quantizer compressing f32 vectors to binary (u8) representation.
///
/// # Memory Layout
///
/// - Input: `[f32; 768]` (3072 bytes)
/// - Output: `[u8; 96]` (96 bytes, 64-byte aligned)
/// - Compression: 8x
///
/// # Algorithm
///
/// Sign-based quantization: each bit represents whether the corresponding
/// f32 value is positive (1) or non-positive (0).
///
/// # Example
///
/// ```
/// use edgevec::quantization::binary::BinaryQuantizer;
///
/// let quantizer = BinaryQuantizer::new();
/// let vector = vec![0.5f32; 768];
/// let quantized = quantizer.quantize(&vector);
///
/// // 768 dimensions compressed to 96 bytes
/// assert_eq!(quantized.data().len(), 96);
/// ```
#[derive(Clone, Debug, Default)]
pub struct BinaryQuantizer {
    // Stateless quantizer - no configuration needed for sign-based quantization
    _marker: std::marker::PhantomData<()>,
}

impl BinaryQuantizer {
    /// Creates a new `BinaryQuantizer`.
    ///
    /// The quantizer is stateless - no training or configuration required
    /// for simple sign-based quantization.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::BinaryQuantizer;
    ///
    /// let quantizer = BinaryQuantizer::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Quantizes a 768-dimensional f32 vector to binary representation.
    ///
    /// # Arguments
    ///
    /// * `vector` - A 768-dimensional f32 vector.
    ///
    /// # Panics
    ///
    /// Panics if the input vector length is not exactly 768.
    ///
    /// # Algorithm
    ///
    /// For each dimension `i` in `[0, 768)`:
    /// - If `vector[i] > 0.0`, set bit `i` to 1
    /// - Else, set bit `i` to 0
    ///
    /// Bits are packed into bytes in little-endian order:
    /// - Byte 0 contains bits `[0..8]`
    /// - Byte 1 contains bits `[8..16]`
    /// - ...
    /// - Byte 95 contains bits `[760..768]`
    ///
    /// # Performance
    ///
    /// Target: <1ms per vector.
    ///
    /// # Example
    ///
    /// ```
    /// use edgevec::quantization::binary::BinaryQuantizer;
    ///
    /// let quantizer = BinaryQuantizer::new();
    ///
    /// // All positive -> all bits 1
    /// let positive = vec![1.0f32; 768];
    /// let q_pos = quantizer.quantize(&positive);
    /// assert_eq!(q_pos.data()[0], 0xFF);
    ///
    /// // All negative -> all bits 0
    /// let negative = vec![-1.0f32; 768];
    /// let q_neg = quantizer.quantize(&negative);
    /// assert_eq!(q_neg.data()[0], 0x00);
    /// ```
    #[must_use]
    pub fn quantize(&self, vector: &[f32]) -> QuantizedVector {
        assert_eq!(
            vector.len(),
            BINARY_QUANTIZATION_DIM,
            "Input must be {BINARY_QUANTIZATION_DIM}-dimensional, got {}",
            vector.len()
        );

        let mut data = [0u8; QUANTIZED_VECTOR_SIZE];

        // Adapted from binary_semantic_cache v1.0 (MIT License)
        // Copyright (c) 2024 Matteo Panzeri
        for (i, &value) in vector.iter().enumerate() {
            if value > 0.0 {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                data[byte_idx] |= 1 << bit_idx;
            }
        }

        QuantizedVector { data }
    }

    /// Quantizes a vector of arbitrary dimension.
    ///
    /// Unlike `quantize`, this method accepts vectors of any length and
    /// pads or truncates as needed. Primarily for testing and flexibility.
    ///
    /// # Arguments
    ///
    /// * `vector` - An f32 vector of any length.
    ///
    /// # Returns
    ///
    /// A `QuantizedVector` where:
    /// - If input is shorter than 768, remaining bits are 0
    /// - If input is longer than 768, extra values are ignored
    ///
    /// # Examples
    ///
    /// ```
    /// use edgevec::quantization::binary::BinaryQuantizer;
    ///
    /// let quantizer = BinaryQuantizer::new();
    ///
    /// // Short vector (16 elements) - remaining bits are 0
    /// let short = vec![1.0f32; 16];
    /// let q_short = quantizer.quantize_flexible(&short);
    /// assert_eq!(q_short.data()[0], 0xFF); // First 8 bits set
    /// assert_eq!(q_short.data()[1], 0xFF); // Next 8 bits set
    /// assert_eq!(q_short.data()[2], 0x00); // Rest are 0
    ///
    /// // Long vector (1000 elements) - truncated to 768
    /// let long = vec![1.0f32; 1000];
    /// let q_long = quantizer.quantize_flexible(&long);
    /// assert_eq!(q_long.data().len(), 96); // Always 96 bytes
    /// ```
    #[must_use]
    pub fn quantize_flexible(&self, vector: &[f32]) -> QuantizedVector {
        let mut data = [0u8; QUANTIZED_VECTOR_SIZE];
        let len = vector.len().min(BINARY_QUANTIZATION_DIM);

        for (i, &value) in vector.iter().take(len).enumerate() {
            if value > 0.0 {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                data[byte_idx] |= 1 << bit_idx;
            }
        }

        QuantizedVector { data }
    }
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_zero_vector() {
        let quantizer = BinaryQuantizer::new();
        let zero = vec![0.0f32; BINARY_QUANTIZATION_DIM];
        let quantized = quantizer.quantize(&zero);

        // All bits should be 0 (0.0 <= 0.0)
        assert_eq!(quantized.data, [0u8; QUANTIZED_VECTOR_SIZE]);
    }

    #[test]
    fn test_quantize_positive_vector() {
        let quantizer = BinaryQuantizer::new();
        let positive = vec![1.0f32; BINARY_QUANTIZATION_DIM];
        let quantized = quantizer.quantize(&positive);

        // All bits should be 1 (1.0 > 0.0)
        assert_eq!(quantized.data, [0xFFu8; QUANTIZED_VECTOR_SIZE]);
    }

    #[test]
    fn test_quantize_negative_vector() {
        let quantizer = BinaryQuantizer::new();
        let negative = vec![-1.0f32; BINARY_QUANTIZATION_DIM];
        let quantized = quantizer.quantize(&negative);

        // All bits should be 0 (-1.0 <= 0.0)
        assert_eq!(quantized.data, [0u8; QUANTIZED_VECTOR_SIZE]);
    }

    #[test]
    fn test_quantize_mixed_vector() {
        let quantizer = BinaryQuantizer::new();
        let mut mixed = vec![-1.0f32; BINARY_QUANTIZATION_DIM];
        mixed[0] = 1.0; // First bit should be 1
        mixed[8] = 1.0; // Ninth bit should be 1 (second byte, bit 0)

        let quantized = quantizer.quantize(&mixed);

        assert_eq!(quantized.data[0], 0b0000_0001); // Bit 0 set
        assert_eq!(quantized.data[1], 0b0000_0001); // Bit 8 set
                                                    // Rest should be 0
        for i in 2..QUANTIZED_VECTOR_SIZE {
            assert_eq!(quantized.data[i], 0);
        }
    }

    #[test]
    fn test_quantize_alternating() {
        let quantizer = BinaryQuantizer::new();
        let alternating: Vec<f32> = (0..BINARY_QUANTIZATION_DIM)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();

        let quantized = quantizer.quantize(&alternating);

        // Even indices are positive -> bits 0, 2, 4, 6 set in each byte
        // Binary: 01010101 = 0x55
        for byte in &quantized.data {
            assert_eq!(*byte, 0x55);
        }
    }

    #[test]
    fn test_hamming_distance_identical() {
        let quantizer = BinaryQuantizer::new();
        let vec = vec![0.5f32; BINARY_QUANTIZATION_DIM];
        let q1 = quantizer.quantize(&vec);
        let q2 = quantizer.quantize(&vec);

        assert_eq!(q1.hamming_distance(&q2), 0);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_hamming_distance_opposite() {
        let q1 = QuantizedVector::from_bytes([0x00u8; QUANTIZED_VECTOR_SIZE]);
        let q2 = QuantizedVector::from_bytes([0xFFu8; QUANTIZED_VECTOR_SIZE]);

        assert_eq!(q1.hamming_distance(&q2), BINARY_QUANTIZATION_DIM as u32);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_hamming_distance_symmetric() {
        let q1 = QuantizedVector::from_bytes([0xAAu8; QUANTIZED_VECTOR_SIZE]); // 10101010...
        let q2 = QuantizedVector::from_bytes([0x55u8; QUANTIZED_VECTOR_SIZE]); // 01010101...

        assert_eq!(q1.hamming_distance(&q2), q2.hamming_distance(&q1));
        // Each byte has all 8 bits different
        assert_eq!(q1.hamming_distance(&q2), BINARY_QUANTIZATION_DIM as u32);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_hamming_distance_partial() {
        let q1 = QuantizedVector::from_bytes([0xF0u8; QUANTIZED_VECTOR_SIZE]); // 11110000
        let q2 = QuantizedVector::from_bytes([0x0Fu8; QUANTIZED_VECTOR_SIZE]); // 00001111

        // Each byte differs in all 8 bits
        assert_eq!(q1.hamming_distance(&q2), BINARY_QUANTIZATION_DIM as u32);
    }

    #[test]
    fn test_quantize_deterministic() {
        let quantizer = BinaryQuantizer::new();
        let vec = vec![0.123f32; BINARY_QUANTIZATION_DIM];

        let q1 = quantizer.quantize(&vec);
        let q2 = quantizer.quantize(&vec);

        assert_eq!(q1, q2); // Must be deterministic
    }

    #[test]
    fn test_alignment() {
        let q = QuantizedVector::default();
        let ptr = std::ptr::addr_of!(q) as usize;

        assert_eq!(
            ptr % 64,
            0,
            "QuantizedVector must be 64-byte aligned, got alignment {}",
            ptr % 64
        );
    }

    #[test]
    fn test_struct_size() {
        assert_eq!(
            std::mem::size_of::<QuantizedVector>(),
            128, // 96 bytes + padding for 64-byte alignment
            "QuantizedVector size should be 128 bytes (96 data + padding)"
        );
    }

    #[test]
    fn test_struct_alignment() {
        assert_eq!(
            std::mem::align_of::<QuantizedVector>(),
            64,
            "QuantizedVector must have 64-byte alignment"
        );
    }

    #[test]
    fn test_similarity_identical() {
        let q1 = QuantizedVector::from_bytes([0xAAu8; QUANTIZED_VECTOR_SIZE]);
        let q2 = QuantizedVector::from_bytes([0xAAu8; QUANTIZED_VECTOR_SIZE]);

        assert!((q1.similarity(&q2) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_opposite() {
        let q1 = QuantizedVector::from_bytes([0x00u8; QUANTIZED_VECTOR_SIZE]);
        let q2 = QuantizedVector::from_bytes([0xFFu8; QUANTIZED_VECTOR_SIZE]);

        assert!((q1.similarity(&q2) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quantize_flexible_short() {
        let quantizer = BinaryQuantizer::new();
        let short = vec![1.0f32; 16]; // Only 16 elements

        let quantized = quantizer.quantize_flexible(&short);

        // First 2 bytes should be 0xFF (16 bits set)
        assert_eq!(quantized.data[0], 0xFF);
        assert_eq!(quantized.data[1], 0xFF);
        // Rest should be 0
        for i in 2..QUANTIZED_VECTOR_SIZE {
            assert_eq!(quantized.data[i], 0);
        }
    }

    #[test]
    #[should_panic(expected = "Input must be 768-dimensional")]
    fn test_quantize_wrong_dimension() {
        let quantizer = BinaryQuantizer::new();
        let wrong_dim = vec![1.0f32; 100];
        let _ = quantizer.quantize(&wrong_dim);
    }

    #[test]
    fn test_edge_case_nan() {
        let quantizer = BinaryQuantizer::new();
        let mut vec = vec![1.0f32; BINARY_QUANTIZATION_DIM];
        vec[0] = f32::NAN; // NaN is not > 0.0, so bit should be 0

        let quantized = quantizer.quantize(&vec);

        // First bit should be 0 (NaN is not positive)
        assert_eq!(quantized.data[0] & 1, 0);
    }

    #[test]
    fn test_edge_case_infinity() {
        let quantizer = BinaryQuantizer::new();
        let mut vec = vec![1.0f32; BINARY_QUANTIZATION_DIM];
        vec[0] = f32::INFINITY; // +Inf > 0.0, so bit should be 1
        vec[1] = f32::NEG_INFINITY; // -Inf < 0.0, so bit should be 0

        let quantized = quantizer.quantize(&vec);

        // Bit 0 should be 1 (positive infinity)
        assert_eq!(quantized.data[0] & 0b01, 0b01);
        // Bit 1 should be 0 (negative infinity)
        assert_eq!(quantized.data[0] & 0b10, 0b00);
    }

    #[test]
    fn test_edge_case_negative_zero() {
        let quantizer = BinaryQuantizer::new();
        let mut vec = vec![1.0f32; BINARY_QUANTIZATION_DIM];
        vec[0] = -0.0f32; // -0.0 is not > 0.0, so bit should be 0

        let quantized = quantizer.quantize(&vec);

        // First bit should be 0 (-0.0 is not positive)
        assert_eq!(quantized.data[0] & 1, 0);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_hamming_bounds() {
        let q1 = QuantizedVector::default();
        let q2 = QuantizedVector::from_bytes([0xFFu8; QUANTIZED_VECTOR_SIZE]);

        let distance = q1.hamming_distance(&q2);

        // Distance must be in valid range [0, 768]
        assert!(distance <= BINARY_QUANTIZATION_DIM as u32);
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a valid 768-dimensional vector with values in [-1, 1].
    fn valid_vector_strategy() -> impl Strategy<Value = Vec<f32>> {
        proptest::collection::vec(-1.0f32..=1.0f32, BINARY_QUANTIZATION_DIM)
    }

    proptest! {
        /// Property: Quantization is deterministic.
        /// quantize(v) == quantize(v) for all valid vectors.
        #[test]
        fn prop_quantize_deterministic(v in valid_vector_strategy()) {
            let quantizer = BinaryQuantizer::new();
            let q1 = quantizer.quantize(&v);
            let q2 = quantizer.quantize(&v);
            prop_assert_eq!(q1, q2);
        }

        /// Property: Self-distance is always zero.
        /// hamming_distance(q, q) == 0 for all quantized vectors.
        #[test]
        fn prop_self_distance_zero(v in valid_vector_strategy()) {
            let quantizer = BinaryQuantizer::new();
            let q = quantizer.quantize(&v);
            prop_assert_eq!(q.hamming_distance(&q), 0);
        }

        /// Property: Hamming distance is symmetric.
        /// hamming_distance(a, b) == hamming_distance(b, a).
        #[test]
        fn prop_hamming_symmetric(
            v1 in valid_vector_strategy(),
            v2 in valid_vector_strategy()
        ) {
            let quantizer = BinaryQuantizer::new();
            let q1 = quantizer.quantize(&v1);
            let q2 = quantizer.quantize(&v2);
            prop_assert_eq!(q1.hamming_distance(&q2), q2.hamming_distance(&q1));
        }

        /// Property: Hamming distance is bounded by dimension.
        /// 0 <= hamming_distance(a, b) <= 768.
        #[test]
        fn prop_hamming_bounded(
            v1 in valid_vector_strategy(),
            v2 in valid_vector_strategy()
        ) {
            let quantizer = BinaryQuantizer::new();
            let q1 = quantizer.quantize(&v1);
            let q2 = quantizer.quantize(&v2);
            let dist = q1.hamming_distance(&q2);
            prop_assert!(dist <= BINARY_QUANTIZATION_DIM as u32);
        }

        /// Property: Similarity is in valid range [0, 1].
        #[test]
        fn prop_similarity_bounded(
            v1 in valid_vector_strategy(),
            v2 in valid_vector_strategy()
        ) {
            let quantizer = BinaryQuantizer::new();
            let q1 = quantizer.quantize(&v1);
            let q2 = quantizer.quantize(&v2);
            let sim = q1.similarity(&q2);
            prop_assert!(sim >= 0.0);
            prop_assert!(sim <= 1.0);
        }

        /// Property: Triangle inequality for Hamming distance.
        /// hamming_distance(a, c) <= hamming_distance(a, b) + hamming_distance(b, c).
        #[test]
        fn prop_triangle_inequality(
            v1 in valid_vector_strategy(),
            v2 in valid_vector_strategy(),
            v3 in valid_vector_strategy()
        ) {
            let quantizer = BinaryQuantizer::new();
            let q1 = quantizer.quantize(&v1);
            let q2 = quantizer.quantize(&v2);
            let q3 = quantizer.quantize(&v3);

            let d12 = q1.hamming_distance(&q2);
            let d23 = q2.hamming_distance(&q3);
            let d13 = q1.hamming_distance(&q3);

            prop_assert!(d13 <= d12 + d23,
                "Triangle inequality violated: {} > {} + {}", d13, d12, d23);
        }

        /// Property: Output size is always exactly 96 bytes.
        #[test]
        fn prop_output_size_constant(v in valid_vector_strategy()) {
            let quantizer = BinaryQuantizer::new();
            let q = quantizer.quantize(&v);
            prop_assert_eq!(q.data().len(), QUANTIZED_VECTOR_SIZE);
        }

        /// Property: Sign preservation - if all values are positive, all bits are 1.
        #[test]
        fn prop_all_positive_all_ones(scale in 0.001f32..10.0f32) {
            let v: Vec<f32> = (0..BINARY_QUANTIZATION_DIM).map(|_| scale).collect();
            let quantizer = BinaryQuantizer::new();
            let q = quantizer.quantize(&v);
            for byte in q.data() {
                prop_assert_eq!(*byte, 0xFF);
            }
        }

        /// Property: Sign preservation - if all values are negative, all bits are 0.
        #[test]
        fn prop_all_negative_all_zeros(scale in 0.001f32..10.0f32) {
            let v: Vec<f32> = (0..BINARY_QUANTIZATION_DIM).map(|_| -scale).collect();
            let quantizer = BinaryQuantizer::new();
            let q = quantizer.quantize(&v);
            for byte in q.data() {
                prop_assert_eq!(*byte, 0x00);
            }
        }
    }
}
