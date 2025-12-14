//! Progress callback tests for BatchInsertable trait (W11.7)
//!
//! This module validates progress callback behavior during batch insertions.

use edgevec::batch::BatchInsertable;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::cell::RefCell;
use std::rc::Rc;

/// Helper to create a test index and storage with given dimensions
fn create_test_env(dimensions: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dimensions);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).expect("Failed to create index");
    (index, storage)
}

/// Generate test vectors with sequential IDs
fn generate_vectors(count: usize, dimensions: usize) -> Vec<(u64, Vec<f32>)> {
    (1..=count)
        .map(|i| {
            let vector: Vec<f32> = (0..dimensions)
                .map(|j| ((i * dimensions + j) as f32) / 1000.0)
                .collect();
            (i as u64, vector)
        })
        .collect()
}

// =============================================================================
// PROGRESS CALLBACK INVOCATION TESTS
// =============================================================================

#[test]
fn test_progress_callback_invoked_at_zero_percent() {
    // AC7.2: Callback invoked at 0%
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(10, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();
    assert!(!recorded.is_empty(), "Callback should be called");
    assert_eq!(recorded[0], (0, 10), "First call should be (0, total)");
}

#[test]
fn test_progress_callback_invoked_at_100_percent() {
    // AC7.3: Callback invoked at 100%
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(10, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();
    assert!(!recorded.is_empty(), "Callback should be called");
    let last = recorded.last().unwrap();
    assert_eq!(*last, (10, 10), "Last call should be (total, total)");
}

#[test]
fn test_progress_callback_invoked_at_intermediate_percentages() {
    // AC7.4: Callback invoked at intermediate percentages
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(100, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();

    // Should have calls at approximately 0%, 10%, 20%, ..., 100%
    // With 100 vectors and interval of 10, we expect calls at 0, 10, 20, ..., 100
    assert!(
        recorded.len() >= 3,
        "Should have at least 3 calls (0%, middle, 100%)"
    );

    // First should be 0%
    assert_eq!(recorded[0].0, 0, "First should be at 0%");

    // Last should be 100%
    assert_eq!(recorded.last().unwrap().0, 100, "Last should be at 100%");

    // Check intermediate calls exist
    let has_intermediate = recorded.iter().any(|(current, _)| *current > 0 && *current < 100);
    assert!(has_intermediate, "Should have intermediate progress calls");
}

#[test]
fn test_progress_callback_not_invoked_if_none() {
    // AC7.5: Callback not invoked if None
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(10, 4);

    // Pass None - this should work and not panic
    let result = index.batch_insert(vectors, &mut storage, None::<fn(usize, usize)>);

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 10);
    // No way to verify callback wasn't called since we passed None,
    // but if this test runs without panic, None is handled correctly
}

#[test]
fn test_progress_callback_state_captured() {
    // AC7.6: Callback state properly captured
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(20, 4);

    // Use mutable state that the callback captures
    let mut sum = 0usize;
    let mut call_count = 0usize;

    let result = index.batch_insert(vectors, &mut storage, Some(|current, _total| {
        sum += current;
        call_count += 1;
    }));

    assert!(result.is_ok());
    assert!(call_count > 0, "Callback should have been called");
    // Sum should be non-zero since we add current progress values
    // First call is (0, 20), so sum includes 0
    // But subsequent calls add non-zero values
}

// =============================================================================
// PROGRESS CALLBACK EDGE CASES
// =============================================================================

#[test]
fn test_progress_callback_single_vector() {
    // Single vector should still trigger 0% and 100%
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![(1u64, vec![1.0, 2.0, 3.0, 4.0])];

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();

    // Should have at least 2 calls: 0% and 100%
    assert!(recorded.len() >= 2, "Single vector should have at least 2 calls");
    assert!(recorded.contains(&(0, 1)), "Should have (0, 1) call");
    assert!(recorded.contains(&(1, 1)), "Should have (1, 1) call");
}

#[test]
fn test_progress_callback_empty_batch() {
    // Empty batch should not trigger any callbacks
    let (mut index, mut storage) = create_test_env(4);
    let vectors: Vec<(u64, Vec<f32>)> = vec![];

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();
    assert!(recorded.is_empty(), "Empty batch should not trigger callbacks");
}

#[test]
fn test_progress_callback_with_skipped_vectors() {
    // Progress should reflect attempted vectors, not just successful ones
    let (mut index, mut storage) = create_test_env(4);
    let vectors = vec![
        (1u64, vec![1.0, 2.0, 3.0, 4.0]),       // Valid
        (2u64, vec![f32::NAN, 2.0, 3.0, 4.0]),  // Invalid - skipped
        (3u64, vec![3.0, 3.0, 3.0, 3.0]),       // Valid
    ];

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let ids = result.unwrap();
    assert_eq!(ids.len(), 2, "Should have 2 successful inserts");

    let recorded = calls.borrow();
    // Total should be 3 (input count)
    assert!(recorded.iter().all(|(_, total)| *total == 3));

    // Final callback should report inserted count (2), not processed count
    let last = recorded.last().unwrap();
    assert_eq!(last.0, 2, "Final callback should report 2 inserted");
}

#[test]
fn test_progress_callback_large_batch() {
    // Large batch should have bounded callback calls
    let (mut index, mut storage) = create_test_env(16);
    let vectors = generate_vectors(1000, 16);

    let mut call_count = 0usize;

    let result = index.batch_insert(vectors, &mut storage, Some(|_current, _total| {
        call_count += 1;
    }));

    assert!(result.is_ok());
    assert_eq!(index.node_count(), 1000);

    // Should not be called 1000 times - should be limited to ~11 times (0% through 100% in 10% increments)
    // Plus first inserted (1) callback
    assert!(
        call_count <= 20,
        "Callback count {} should be bounded (not 1000)",
        call_count
    );
    assert!(
        call_count >= 2,
        "Callback should be called at least twice (0% and 100%)"
    );
}

// =============================================================================
// PROGRESS CALLBACK DATA VALIDATION
// =============================================================================

#[test]
fn test_progress_current_never_exceeds_total() {
    // Current should never exceed total
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(50, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();

    for (current, total) in recorded.iter() {
        assert!(
            current <= total,
            "Current {} should not exceed total {}",
            current,
            total
        );
    }
}

#[test]
fn test_progress_total_is_consistent() {
    // Total should be the same in all callbacks
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(30, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();

    if !recorded.is_empty() {
        let expected_total = 30; // Input vector count
        for (_, total) in recorded.iter() {
            assert_eq!(*total, expected_total, "Total should be consistent");
        }
    }
}

#[test]
fn test_progress_current_is_monotonic_for_successful() {
    // When all vectors succeed, current should be non-decreasing
    let (mut index, mut storage) = create_test_env(4);
    let vectors = generate_vectors(25, 4);

    let calls: Rc<RefCell<Vec<(usize, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_clone = calls.clone();

    let result = index.batch_insert(vectors, &mut storage, Some(move |current, total| {
        calls_clone.borrow_mut().push((current, total));
    }));

    assert!(result.is_ok());
    let recorded = calls.borrow();

    let mut prev_current = 0;
    for (current, _) in recorded.iter() {
        assert!(
            *current >= prev_current,
            "Current should be non-decreasing: {} < {}",
            *current,
            prev_current
        );
        prev_current = *current;
    }
}
