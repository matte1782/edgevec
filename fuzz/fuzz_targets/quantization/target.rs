//! Fuzz target for binary quantization.
//!
//! Tests:
//! 1. Quantization stability (same input -> same output)
//! 2. Hamming distance bounds (always <= 768)
//! 3. No panics on arbitrary input
//! 4. Edge cases (NaN, Inf, subnormal values)

#![no_main]

use edgevec::quantization::binary::{BinaryQuantizer, BINARY_QUANTIZATION_DIM};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need at least 768 * 4 = 3072 bytes for a full vector
    if data.len() < BINARY_QUANTIZATION_DIM * 4 {
        return;
    }

    // Parse fuzzer input into f32 vector
    let mut vector = Vec::with_capacity(BINARY_QUANTIZATION_DIM);
    for chunk in data.chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        vector.push(value);
        if vector.len() == BINARY_QUANTIZATION_DIM {
            break;
        }
    }

    // Ensure we have exactly 768 dimensions
    if vector.len() != BINARY_QUANTIZATION_DIM {
        return;
    }

    // Fuzz quantization
    let quantizer = BinaryQuantizer::new();
    let quantized1 = quantizer.quantize(&vector);

    // Invariant 1: Determinism - same input must produce same output
    let quantized2 = quantizer.quantize(&vector);
    assert_eq!(quantized1, quantized2, "Quantization must be deterministic");

    // Invariant 2: Self-distance is always 0
    let distance_self = quantized1.hamming_distance(&quantized1);
    assert_eq!(distance_self, 0, "Hamming distance to self must be 0");

    // Invariant 3: Hamming distance is bounded
    let distance = quantized1.hamming_distance(&quantized2);
    assert!(
        distance <= BINARY_QUANTIZATION_DIM as u32,
        "Hamming distance {} exceeds maximum {}",
        distance,
        BINARY_QUANTIZATION_DIM
    );

    // Invariant 4: Symmetry
    assert_eq!(
        quantized1.hamming_distance(&quantized2),
        quantized2.hamming_distance(&quantized1),
        "Hamming distance must be symmetric"
    );

    // Invariant 5: Output size is correct
    assert_eq!(
        quantized1.data().len(),
        96,
        "Quantized vector must be 96 bytes"
    );

    // Invariant 6: Similarity is in valid range [0, 1]
    let similarity = quantized1.similarity(&quantized2);
    assert!(
        (0.0..=1.0).contains(&similarity),
        "Similarity {} out of range [0, 1]",
        similarity
    );
});
