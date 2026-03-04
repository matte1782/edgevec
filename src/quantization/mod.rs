//! Quantization logic for vector compression.
//!
//! This module provides vector quantization implementations for memory-efficient
//! storage and fast similarity computation.
//!
//! # Available Quantizers
//!
//! - [`ScalarQuantizer`]: SQ8 quantization (f32 -> u8), 4x compression
//! - [`BinaryQuantizer`]: Binary quantization (f32 -> bit), 32x compression (fixed 768D)
//! - `BinaryVector`: Variable-dimension binary quantization (any dimension divisible by 8)
//! - [`PqCodebook`]: Product Quantization (f32 -> u8 per subspace), up to 384x compression
//!
//! # Example
//!
//! ```
//! use edgevec::quantization::binary::BinaryQuantizer;
//!
//! let quantizer = BinaryQuantizer::new();
//! let vector = vec![0.5f32; 768];
//! let quantized = quantizer.quantize(&vector);
//!
//! // 768 f32 values (3072 bytes) -> 96 bytes
//! assert_eq!(quantized.data().len(), 96);
//! ```
//!
//! # Variable Dimension Example
//!
//! ```
//! use edgevec::quantization::variable::BinaryVector;
//!
//! // Works with any dimension divisible by 8
//! let vector = vec![1.0f32; 128];
//! let bv = BinaryVector::quantize(&vector).unwrap();
//! assert_eq!(bv.dimension(), 128);
//! assert_eq!(bv.bytes(), 16);
//! ```
//!
//! # Product Quantization Example
//!
//! ```
//! use edgevec::quantization::product::PqCodebook;
//!
//! let vectors: Vec<Vec<f32>> = (0..100).map(|i| vec![(i as f32) * 0.01; 32]).collect();
//! let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();
//! let codebook = PqCodebook::train(&refs, 4, 16, 10).unwrap();
//!
//! let code = codebook.encode(&vectors[0]).unwrap();
//! assert_eq!(code.num_subquantizers(), 4);
//! ```

/// Binary quantization (sign-based) implementation.
pub mod binary;

/// Scalar quantization (SQ8) implementation.
pub mod scalar;

/// SIMD-accelerated operations for quantized vectors.
///
/// This module provides high-performance SIMD implementations for operations
/// on quantized vectors. It automatically selects the best implementation
/// based on CPU capabilities at runtime.
///
/// # Public API
///
/// While this module is public, most users will access SIMD functionality
/// indirectly through the [`BinaryQuantizer`] and [`ScalarQuantizer`] APIs.
///
/// Advanced users can use this module directly for:
/// - Benchmarking SIMD vs portable implementations
/// - Custom quantized vector operations
/// - Performance analysis
pub mod simd;

/// Product Quantization (PQ) for high-compression vector encoding.
///
/// Compresses vectors into M-byte codes using learned subspace codebooks.
/// Achieves 384× compression at M=8 (768D → 8 bytes) with codebook-based
/// Asymmetric Distance Computation (ADC) for approximate search.
pub mod product;

/// Variable-dimension binary quantization.
///
/// This module provides `BinaryVector` which supports any dimension
/// divisible by 8, unlike the fixed 768D [`BinaryQuantizer`].
pub mod variable;

pub use binary::{
    BinaryQuantizer, QuantizedVector, BINARY_QUANTIZATION_DIM, QUANTIZED_VECTOR_SIZE,
};
pub use product::{DistanceTable, PqCode, PqCodebook, PqError, PqSearchResult};
pub use scalar::{QuantizerConfig, ScalarQuantizer};
pub use variable::{BinaryVector, QuantizationError};
