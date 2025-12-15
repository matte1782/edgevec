//! Batch Delete API Tests

use edgevec::hnsw::{BatchDeleteResult, HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;

fn create_test_index(count: usize) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for i in 0..count {
        index.insert(&vec![i as f32; 4], &mut storage).unwrap();
    }

    (index, storage)
}

#[test]
fn test_batch_delete_all_valid() {
    let (mut index, _storage) = create_test_index(10);

    let ids: Vec<VectorId> = vec![VectorId(1), VectorId(3), VectorId(5)];
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.deleted, 3);
    assert_eq!(result.already_deleted, 0);
    assert_eq!(result.invalid_ids, 0);
    assert_eq!(result.total, 3);
    assert_eq!(result.unique_count, 3);
    assert!(result.all_valid());
    assert!(result.any_deleted());
}

#[test]
fn test_batch_delete_mixed_results() {
    let (mut index, _storage) = create_test_index(10);

    // Delete one first
    index.soft_delete(VectorId(1)).unwrap();

    // Batch with: already deleted, valid, invalid
    let ids = vec![VectorId(1), VectorId(2), VectorId(999)];
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.deleted, 1); // VectorId(2)
    assert_eq!(result.already_deleted, 1); // VectorId(1)
    assert_eq!(result.invalid_ids, 1); // VectorId(999)
    assert_eq!(result.total, 3);
    assert!(!result.all_valid());
}

#[test]
fn test_batch_delete_empty() {
    let (mut index, _storage) = create_test_index(10);

    let result = index.soft_delete_batch(&[]);

    assert_eq!(result.deleted, 0);
    assert_eq!(result.total, 0);
    assert!(result.all_valid());
    assert!(!result.any_deleted());
}

#[test]
fn test_batch_delete_idempotent() {
    let (mut index, _storage) = create_test_index(10);

    let ids: Vec<VectorId> = (1..=5).map(VectorId).collect();

    // First batch delete
    let result1 = index.soft_delete_batch(&ids);
    assert_eq!(result1.deleted, 5);

    // Second batch delete (same IDs)
    let result2 = index.soft_delete_batch(&ids);
    assert_eq!(result2.deleted, 0);
    assert_eq!(result2.already_deleted, 5);
}

#[test]
fn test_batch_delete_with_progress() {
    let (mut index, _storage) = create_test_index(100);

    let ids: Vec<VectorId> = (1..=50).map(VectorId).collect();
    let mut progress_calls = 0;
    let mut last_processed = 0;

    let result = index.soft_delete_batch_with_progress(&ids, |processed, total| {
        progress_calls += 1;
        assert!(processed > last_processed);
        assert_eq!(total, 50);
        last_processed = processed;
    });

    assert_eq!(result.deleted, 50);
    assert!(progress_calls >= 5); // At least 5 callbacks for 50 items at ~10% intervals
}

#[test]
fn test_batch_delete_updates_counts() {
    let (mut index, _storage) = create_test_index(100);

    assert_eq!(index.deleted_count(), 0);
    assert_eq!(index.live_count(), 100);

    let ids: Vec<VectorId> = (1..=30).map(VectorId).collect();
    index.soft_delete_batch(&ids);

    assert_eq!(index.deleted_count(), 30);
    assert_eq!(index.live_count(), 70);
}

#[test]
fn test_batch_delete_result_default() {
    let result = BatchDeleteResult::default();

    assert_eq!(result.deleted, 0);
    assert_eq!(result.already_deleted, 0);
    assert_eq!(result.invalid_ids, 0);
    assert_eq!(result.total, 0);
    assert_eq!(result.unique_count, 0);
}

// [C2 FIX] Test duplicate ID handling
#[test]
fn test_batch_delete_duplicate_ids() {
    let (mut index, _storage) = create_test_index(10);

    // Same ID appears 3 times in batch
    let ids = vec![
        VectorId(1),
        VectorId(2),
        VectorId(1),
        VectorId(3),
        VectorId(1),
    ];
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.total, 5); // 5 IDs in input
    assert_eq!(result.unique_count, 3); // Only 3 unique IDs
    assert_eq!(result.deleted, 3); // All 3 unique IDs deleted
    assert_eq!(result.already_deleted, 0);
    assert_eq!(result.invalid_ids, 0);
}

// [M2 FIX] Test memory bounds check
#[test]
fn test_batch_delete_exceeds_max_size() {
    let (mut index, _storage) = create_test_index(10);

    // Create a vector larger than MAX_BATCH_SIZE (10M)
    let large_count = 10_000_001;
    let ids = vec![VectorId(1); large_count];
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.total, large_count);
    assert_eq!(result.invalid_ids, large_count);
    assert!(!result.errors.is_empty());
}

// [M4 FIX] Test large batch size for scalability
#[test]
fn test_batch_delete_large_batch() {
    let (mut index, _storage) = create_test_index(10000);

    // Delete 5000 vectors
    let ids: Vec<VectorId> = (1..=5000).map(VectorId).collect();
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.deleted, 5000);
    assert_eq!(result.unique_count, 5000);
    assert_eq!(result.total, 5000);
    assert_eq!(index.deleted_count(), 5000);
}

// [m4 FIX] Test VectorId(0) edge case
#[test]
fn test_batch_delete_vector_id_zero() {
    let (mut index, _storage) = create_test_index(10);

    // VectorId(0) should be treated as not found (IDs start at 1)
    let ids = vec![VectorId(0), VectorId(1), VectorId(2)];
    let result = index.soft_delete_batch(&ids);

    assert_eq!(result.deleted, 2); // VectorId(1) and VectorId(2)
    assert_eq!(result.invalid_ids, 1); // VectorId(0)
    assert!(!result.all_valid());
}
