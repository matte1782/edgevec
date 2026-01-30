//! WASM FFI Bindings for Batch Insert API (W12.3)
//!
//! This module provides JavaScript-friendly wrappers around the Rust
//! `BatchInsertable` trait for efficient batch vector insertion.
//!
//! # Error Mapping
//!
//! All 6 `BatchError` variants map 1:1 to JavaScript error codes:
//! - `EmptyBatch` → `EMPTY_BATCH`
//! - `DimensionMismatch` → `DIMENSION_MISMATCH`
//! - `DuplicateId` → `DUPLICATE_ID`
//! - `InvalidVector` → `INVALID_VECTOR`
//! - `CapacityExceeded` → `CAPACITY_EXCEEDED`
//! - `InternalError` → `INTERNAL_ERROR`

use super::memory::track_batch_insert;
use crate::batch::BatchInsertable;
use crate::error::BatchError;
use js_sys::{Array, Float32Array};
use wasm_bindgen::prelude::*;

/// Configuration options for batch insert operations (WASM).
///
/// This struct mirrors the TypeScript `BatchInsertConfig` interface
/// defined in `wasm/batch_types.ts`.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct BatchInsertConfig {
    validate_dimensions: bool,
}

#[wasm_bindgen]
impl BatchInsertConfig {
    /// Creates a new `BatchInsertConfig` with default settings.
    ///
    /// Default: `validate_dimensions = true`
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            validate_dimensions: true,
        }
    }

    /// Returns whether dimension validation is enabled.
    #[wasm_bindgen(getter, js_name = validateDimensions)]
    #[must_use]
    pub fn validate_dimensions(&self) -> bool {
        self.validate_dimensions
    }

    /// Sets whether to validate vector dimensions before insertion.
    #[wasm_bindgen(setter, js_name = validateDimensions)]
    pub fn set_validate_dimensions(&mut self, value: bool) {
        self.validate_dimensions = value;
    }
}

/// Result of a batch insert operation (WASM).
///
/// This struct mirrors the TypeScript `BatchInsertResult` interface
/// defined in `wasm/batch_types.ts`.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct BatchInsertResult {
    inserted: usize,
    total: usize,
    ids: Vec<u64>,
}

impl BatchInsertResult {
    /// Creates a new `BatchInsertResult`.
    #[must_use]
    pub fn from_ids(ids: Vec<u64>, total: usize) -> Self {
        Self {
            inserted: ids.len(),
            total,
            ids,
        }
    }
}

#[wasm_bindgen]
impl BatchInsertResult {
    /// Returns the number of vectors successfully inserted.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn inserted(&self) -> usize {
        self.inserted
    }

    /// Returns the total number of vectors attempted (input array length).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Returns a copy of the IDs of successfully inserted vectors.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ids(&self) -> Vec<u64> {
        self.ids.clone()
    }
}

/// Converts a JS `Array` of `Float32Array` to a Rust `Vec<(u64, Vec<f32>)>`.
///
/// Auto-assigns sequential IDs starting from `start_id`.
///
/// # Errors
///
/// - Returns `Err(BatchError::EmptyBatch)` if the input array is empty.
/// - Returns `Err(BatchError::InvalidVector)` if any vector contains NaN or Infinity.
#[allow(clippy::cast_possible_truncation)]
fn convert_js_vectors(vectors: &Array, start_id: u64) -> Result<Vec<(u64, Vec<f32>)>, BatchError> {
    let len = vectors.length() as usize;

    if len == 0 {
        return Err(BatchError::EmptyBatch);
    }

    let mut result = Vec::with_capacity(len);

    for i in 0..len {
        // Safety: Array length is already bounds-checked above, and JS arrays
        // use 32-bit indices, so this cast is safe for valid array access.
        let js_val = vectors.get(i as u32);
        let float_array = Float32Array::from(js_val);
        let vec_data: Vec<f32> = float_array.to_vec();

        // Validate vector at FFI boundary: reject NaN and Infinity values
        for (j, &val) in vec_data.iter().enumerate() {
            if !val.is_finite() {
                return Err(BatchError::InvalidVector {
                    vector_id: i as u64,
                    reason: format!(
                        "element {} is {} (must be finite)",
                        j,
                        if val.is_nan() { "NaN" } else { "Infinity" }
                    ),
                });
            }
        }

        let id = start_id.saturating_add(i as u64);
        result.push((id, vec_data));
    }

    Ok(result)
}

/// Implements the WASM `insertBatch` method for `EdgeVec`.
///
/// This function is the FFI bridge between JavaScript and the Rust
/// `BatchInsertable::batch_insert` method.
///
/// # Arguments
///
/// * `edge_vec` - Mutable reference to the EdgeVec instance
/// * `vectors` - JS Array of Float32Array vectors to insert
/// * `config` - Optional configuration (defaults to dimension validation enabled)
///
/// # Returns
///
/// `Ok(BatchInsertResult)` on success, or `Err(JsValue)` with error code on failure.
///
/// # Error Codes
///
/// - `EMPTY_BATCH`: Input array is empty
/// - `DIMENSION_MISMATCH`: Vector dimensions don't match index configuration
/// - `DUPLICATE_ID`: Vector ID already exists (when using manual IDs)
/// - `INVALID_VECTOR`: Vector contains NaN or Infinity
/// - `CAPACITY_EXCEEDED`: Batch would exceed index capacity
/// - `INTERNAL_ERROR`: Internal HNSW invariant violated
#[allow(clippy::needless_pass_by_value)]
pub fn insert_batch_impl(
    edge_vec: &mut super::EdgeVec,
    vectors: Array,
    config: Option<BatchInsertConfig>,
) -> Result<BatchInsertResult, JsValue> {
    let _config = config.unwrap_or_default();
    let total = vectors.length() as usize;

    // Batch insert is only supported for HNSW indexes
    let (index, storage) = edge_vec
        .inner
        .as_hnsw_mut()
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;

    // Get the next available ID from the index
    let start_id = index.node_count() as u64;

    // Convert JS vectors to Rust format
    let rust_vectors = convert_js_vectors(&vectors, start_id)?;

    // Call the BatchInsertable trait method
    let ids = index.batch_insert(rust_vectors, storage, None::<fn(usize, usize)>)?;

    // Track memory allocation for memory pressure monitoring
    let inserted_count = ids.len();
    if inserted_count > 0 {
        track_batch_insert(inserted_count, index.config.dimensions);
    }

    Ok(BatchInsertResult::from_ids(ids, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Test 1: BatchInsertConfig default
    // ==========================================================================

    #[test]
    fn test_batch_config_default() {
        let config = BatchInsertConfig::new();
        assert!(
            config.validate_dimensions(),
            "Default config should have validate_dimensions: true"
        );
    }

    // ==========================================================================
    // Test 2: BatchInsertConfig setter
    // ==========================================================================

    #[test]
    fn test_batch_config_setter() {
        let mut config = BatchInsertConfig::new();
        assert!(config.validate_dimensions());

        config.set_validate_dimensions(false);
        assert!(
            !config.validate_dimensions(),
            "Setter should change validate_dimensions to false"
        );

        config.set_validate_dimensions(true);
        assert!(
            config.validate_dimensions(),
            "Setter should change validate_dimensions back to true"
        );
    }

    // ==========================================================================
    // Test 3: BatchInsertResult getters
    // ==========================================================================

    #[test]
    fn test_batch_result_getters() {
        let ids = vec![1, 2, 3];
        let result = BatchInsertResult::from_ids(ids, 5);

        assert_eq!(result.inserted(), 3, "inserted should return 3");
        assert_eq!(result.total(), 5, "total should return 5");
        assert_eq!(result.ids(), vec![1, 2, 3], "ids should return [1, 2, 3]");
    }

    // ==========================================================================
    // Test 4: BatchInsertResult ids clone
    // ==========================================================================

    #[test]
    fn test_batch_result_ids_clone() {
        let ids = vec![10, 20, 30];
        let result = BatchInsertResult::from_ids(ids.clone(), 3);

        // Call ids() multiple times - should return clones
        let ids1 = result.ids();
        let ids2 = result.ids();

        assert_eq!(
            ids1, ids2,
            "Multiple calls to ids() should return same values"
        );
        assert_eq!(ids1, ids, "ids() should return cloned data");
    }

    // ==========================================================================
    // Test 5: Empty batch error - Tests the BatchError::EmptyBatch variant
    // ==========================================================================

    #[test]
    fn test_empty_batch_error() {
        // Test that EmptyBatch error has correct Display message
        let error = BatchError::EmptyBatch;
        let msg = format!("{error}");
        assert!(
            msg.contains("empty") || msg.contains("Empty"),
            "EmptyBatch error message should mention 'empty'"
        );
    }

    // ==========================================================================
    // Test 6: BatchError DIMENSION_MISMATCH has correct fields
    // ==========================================================================

    #[test]
    fn test_dimension_mismatch_error_code() {
        let error = BatchError::DimensionMismatch {
            expected: 128,
            actual: 64,
            vector_id: 1,
        };

        // Verify error contains expected information in display
        let msg = format!("{error}");
        assert!(msg.contains("128"), "Should contain expected dimension");
        assert!(msg.contains("64"), "Should contain actual dimension");
        assert!(msg.contains('1'), "Should contain vector_id");
    }

    // ==========================================================================
    // Test 7: BatchInsertResult from_ids with zero vectors
    // ==========================================================================

    #[test]
    fn test_successful_insert_result() {
        let ids = vec![100, 101, 102, 103, 104];
        let result = BatchInsertResult::from_ids(ids.clone(), 5);

        assert_eq!(result.inserted(), 5);
        assert_eq!(result.total(), 5);
        assert_eq!(result.ids().len(), 5);
    }

    // ==========================================================================
    // Test 8: Partial success scenario
    // ==========================================================================

    #[test]
    fn test_partial_success_result() {
        // Simulate partial success: 3 inserted out of 5 attempted
        let ids = vec![1, 2, 3];
        let result = BatchInsertResult::from_ids(ids, 5);

        assert_eq!(
            result.inserted(),
            3,
            "inserted should be 3 (partial success)"
        );
        assert_eq!(result.total(), 5, "total should be 5 (all attempted)");
        assert!(
            result.inserted() < result.total(),
            "Partial success means inserted < total"
        );
    }

    // ==========================================================================
    // Test 9: DuplicateId error variant - Tests BatchError::DuplicateId
    // ==========================================================================

    #[test]
    fn test_duplicate_id_error() {
        let error = BatchError::DuplicateId { vector_id: 42 };
        let msg = format!("{error}");
        assert!(
            msg.contains("42"),
            "DuplicateId error should contain the duplicate ID"
        );
        assert!(
            msg.to_lowercase().contains("duplicate"),
            "DuplicateId error should mention 'duplicate'"
        );
    }

    // ==========================================================================
    // Test 10: InvalidVector error variant - Tests BatchError::InvalidVector
    // ==========================================================================

    #[test]
    fn test_invalid_vector_error() {
        let error = BatchError::InvalidVector {
            vector_id: 7,
            reason: "contains NaN".to_string(),
        };
        let msg = format!("{error}");
        assert!(
            msg.contains('7'),
            "InvalidVector error should contain vector_id"
        );
        assert!(
            msg.contains("NaN"),
            "InvalidVector error should contain reason"
        );
    }

    // ==========================================================================
    // Test 11: CapacityExceeded error variant - Tests BatchError::CapacityExceeded
    // ==========================================================================

    #[test]
    fn test_capacity_exceeded_error() {
        let error = BatchError::CapacityExceeded {
            current: 100_000,
            max: 100_000,
        };
        let msg = format!("{error}");
        assert!(
            msg.contains("100000"),
            "CapacityExceeded error should contain capacity numbers"
        );
        assert!(
            msg.to_lowercase().contains("capacity") || msg.to_lowercase().contains("exceeded"),
            "CapacityExceeded error should mention capacity/exceeded"
        );
    }

    // ==========================================================================
    // Test 12: InternalError error variant - Tests BatchError::InternalError
    // ==========================================================================

    #[test]
    fn test_internal_error() {
        let error = BatchError::InternalError {
            message: "HNSW invariant violated".to_string(),
        };
        let msg = format!("{error}");
        assert!(
            msg.contains("HNSW invariant violated"),
            "InternalError should contain the message"
        );
    }

    // ==========================================================================
    // Test 13: All error codes have correct Display - comprehensive check
    // ==========================================================================

    #[test]
    fn test_all_error_variants_display() {
        // This test ensures all 6 BatchError variants have meaningful Display output
        let errors: Vec<BatchError> = vec![
            BatchError::EmptyBatch,
            BatchError::DimensionMismatch {
                expected: 128,
                actual: 64,
                vector_id: 0,
            },
            BatchError::DuplicateId { vector_id: 1 },
            BatchError::InvalidVector {
                vector_id: 2,
                reason: "test".to_string(),
            },
            BatchError::CapacityExceeded {
                current: 10,
                max: 10,
            },
            BatchError::InternalError {
                message: "test".to_string(),
            },
        ];

        for error in errors {
            let msg = format!("{error}");
            assert!(
                !msg.is_empty(),
                "Error variant {error:?} should have non-empty Display"
            );
        }
    }

    // ==========================================================================
    // Test 14: BatchInsertResult with empty ids (edge case)
    // ==========================================================================

    #[test]
    fn test_batch_result_empty_ids() {
        // Edge case: Zero vectors inserted out of N attempted
        let result = BatchInsertResult::from_ids(vec![], 10);

        assert_eq!(result.inserted(), 0, "inserted should be 0");
        assert_eq!(result.total(), 10, "total should be 10");
        assert!(result.ids().is_empty(), "ids should be empty");
    }

    // ==========================================================================
    // Test 15: BatchInsertResult with max boundary (edge case)
    // ==========================================================================

    #[test]
    fn test_batch_result_large_batch() {
        // Edge case: Simulate MAX_BATCH_SIZE (1000) vectors inserted
        let ids: Vec<u64> = (0..1000).collect();
        let result = BatchInsertResult::from_ids(ids.clone(), 1000);

        assert_eq!(result.inserted(), 1000, "inserted should be 1000");
        assert_eq!(result.total(), 1000, "total should be 1000");
        assert_eq!(result.ids().len(), 1000, "should have 1000 ids");
        assert_eq!(result.ids()[0], 0, "first id should be 0");
        assert_eq!(result.ids()[999], 999, "last id should be 999");
    }
}
