//! Error handling tests for BatchInsertable trait (W11.6)
//!
//! This module validates all 5 BatchError variants and their context fields.

use edgevec::batch::BatchInsertable;
use edgevec::error::BatchError;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

/// Helper to create a test index and storage with given dimensions
fn create_test_env(dimensions: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dimensions);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).expect("Failed to create index");
    (index, storage)
}

// =============================================================================
// ERROR TYPE TESTS (AC6.2: All 5 error types have dedicated tests)
// =============================================================================

#[test]
fn test_error_dimension_mismatch_first_vector() {
    // BatchError::DimensionMismatch on first vector (fatal)
    let (mut index, mut storage) = create_test_env(128);
    let vectors = vec![
        (1u64, vec![1.0f32; 64]), // Wrong dimension (64 vs 128)
        (2u64, vec![2.0f32; 128]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_err(), "Should fail on dimension mismatch");
    match result.unwrap_err() {
        BatchError::DimensionMismatch {
            expected,
            actual,
            vector_id,
        } => {
            assert_eq!(expected, 128, "Expected dimension should be 128");
            assert_eq!(actual, 64, "Actual dimension should be 64");
            assert_eq!(vector_id, 1, "Vector ID should be 1");
        }
        other => panic!("Expected DimensionMismatch, got {:?}", other),
    }
    // Index should be unchanged
    assert_eq!(index.node_count(), 0);
}

#[test]
fn test_error_dimension_mismatch_context_fields() {
    // AC6.4: Error context fields validated for DimensionMismatch
    let (mut index, mut storage) = create_test_env(256);
    let vectors = vec![(42u64, vec![1.0f32; 100])]; // 100 vs 256

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    match result.unwrap_err() {
        BatchError::DimensionMismatch {
            expected,
            actual,
            vector_id,
        } => {
            assert_eq!(expected, 256);
            assert_eq!(actual, 100);
            assert_eq!(vector_id, 42);
        }
        _ => panic!("Expected DimensionMismatch"),
    }
}

#[test]
fn test_error_duplicate_id_skipped_not_error() {
    // BatchError::DuplicateId exists but duplicates are skipped, not errors
    // Verify the error type exists and can be constructed
    let error = BatchError::DuplicateId { vector_id: 123 };
    assert_eq!(format!("{}", error), "Duplicate vector ID: 123");
}

#[test]
fn test_error_invalid_vector_skipped_not_error() {
    // BatchError::InvalidVector exists but invalid vectors are skipped
    let error = BatchError::InvalidVector {
        vector_id: 456,
        reason: "contains NaN".to_string(),
    };
    assert_eq!(format!("{}", error), "Invalid vector 456: contains NaN");
}

#[test]
fn test_error_capacity_exceeded() {
    // BatchError::CapacityExceeded when batch exceeds capacity
    // Note: Default capacity is u32::MAX, so we can't easily trigger this
    // in practice. We test the error type exists and can be constructed.
    let error = BatchError::CapacityExceeded {
        current: 1000000,
        max: 1000000,
    };
    assert_eq!(
        format!("{}", error),
        "Capacity exceeded: current=1000000, max=1000000"
    );
}

#[test]
fn test_error_internal_error() {
    // BatchError::InternalError for HNSW invariant violations
    let error = BatchError::InternalError {
        message: "HNSW insert failed: graph corruption detected".to_string(),
    };
    assert!(format!("{}", error).contains("Internal error"));
    assert!(format!("{}", error).contains("graph corruption"));
}

// =============================================================================
// ERROR MESSAGE TESTS (AC6.3: Error messages validated)
// =============================================================================

#[test]
fn test_error_messages_human_readable() {
    // All error messages should be human-readable
    let errors = vec![
        BatchError::EmptyBatch,
        BatchError::DimensionMismatch {
            expected: 128,
            actual: 64,
            vector_id: 1,
        },
        BatchError::DuplicateId { vector_id: 42 },
        BatchError::InvalidVector {
            vector_id: 100,
            reason: "contains NaN at index 5".to_string(),
        },
        BatchError::CapacityExceeded {
            current: 1000,
            max: 1000,
        },
        BatchError::InternalError {
            message: "Unexpected state".to_string(),
        },
    ];

    for error in errors {
        let msg = format!("{}", error);
        assert!(!msg.is_empty(), "Error message should not be empty");
        // Should contain relevant information
        match &error {
            BatchError::EmptyBatch => {
                assert!(msg.contains("empty") || msg.contains("Empty"));
            }
            BatchError::DimensionMismatch {
                expected, actual, ..
            } => {
                assert!(msg.contains(&expected.to_string()));
                assert!(msg.contains(&actual.to_string()));
            }
            BatchError::DuplicateId { vector_id } => {
                assert!(msg.contains(&vector_id.to_string()));
            }
            BatchError::InvalidVector { vector_id, reason } => {
                assert!(msg.contains(&vector_id.to_string()));
                assert!(msg.contains(reason));
            }
            BatchError::CapacityExceeded { current, max } => {
                assert!(msg.contains(&current.to_string()));
                assert!(msg.contains(&max.to_string()));
            }
            BatchError::InternalError { message } => {
                assert!(msg.contains(message));
            }
        }
    }
}

// =============================================================================
// ERROR BEHAVIOR TESTS
// =============================================================================

#[test]
fn test_dimension_mismatch_first_is_fatal() {
    // Dimension mismatch on FIRST vector aborts immediately
    let (mut index, mut storage) = create_test_env(10);
    let vectors = vec![
        (1u64, vec![1.0f32; 5]),  // Wrong dim - FATAL
        (2u64, vec![2.0f32; 10]), // Never reached
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_err());
    assert_eq!(index.node_count(), 0, "No vectors should be inserted");
}

#[test]
fn test_dimension_mismatch_later_is_skipped() {
    // Dimension mismatch on LATER vectors is skipped (non-fatal)
    let (mut index, mut storage) = create_test_env(10);
    let vectors = vec![
        (1u64, vec![1.0f32; 10]), // Valid
        (2u64, vec![2.0f32; 5]),  // Wrong dim - skipped
        (3u64, vec![3.0f32; 10]), // Valid
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2, "Should insert 2, skip 1");
    assert_eq!(index.node_count(), 2);
}

#[test]
fn test_nan_vector_is_skipped() {
    // NaN vectors are skipped (non-fatal)
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),
        (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]), // NaN - skipped
        (3u64, vec![3.0, 3.0, 3.0, 3.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(index.node_count(), 2);
}

#[test]
fn test_infinity_vector_is_skipped() {
    // Infinity vectors are skipped (non-fatal)
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),
        (2u64, vec![f32::INFINITY, 2.0, 3.0, 4.0]), // Inf - skipped
        (3u64, vec![f32::NEG_INFINITY, 2.0, 3.0, 4.0]), // -Inf - skipped
        (4u64, vec![4.0, 4.0, 4.0, 4.0]),
    ];

    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(index.node_count(), 2);
}

// =============================================================================
// ERROR TRAIT TESTS
// =============================================================================

#[test]
fn test_batch_error_implements_std_error() {
    // BatchError implements std::error::Error
    fn assert_error<T: std::error::Error>() {}
    assert_error::<BatchError>();
}

#[test]
fn test_batch_error_implements_display() {
    // BatchError implements Display
    fn assert_display<T: std::fmt::Display>() {}
    assert_display::<BatchError>();
}

#[test]
fn test_batch_error_implements_debug() {
    // BatchError implements Debug
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<BatchError>();
}

#[test]
fn test_batch_error_implements_clone() {
    // BatchError implements Clone
    let original = BatchError::DimensionMismatch {
        expected: 128,
        actual: 64,
        vector_id: 1,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_batch_error_implements_partial_eq() {
    // BatchError implements PartialEq
    let e1 = BatchError::DuplicateId { vector_id: 42 };
    let e2 = BatchError::DuplicateId { vector_id: 42 };
    let e3 = BatchError::DuplicateId { vector_id: 99 };

    assert_eq!(e1, e2);
    assert_ne!(e1, e3);
}
