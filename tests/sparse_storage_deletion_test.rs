//! Property tests for SparseStorage deletion operations.
//!
//! Week 38 Day 5: Deletion Support
//!
//! These tests verify the correctness of soft delete operations
//! using property-based testing with proptest.

use proptest::prelude::*;
use std::collections::HashSet;

use edgevec::sparse::{SparseError, SparseId, SparseStorage, SparseVector};

// =============================================================================
// TEST STRATEGIES
// =============================================================================

/// Strategy to generate a valid SparseVector
fn arb_sparse_vector() -> impl Strategy<Value = SparseVector> {
    // Use a simpler strategy that always produces valid vectors
    (100u32..=1000, 1usize..=20)
        .prop_flat_map(|(dim, nnz)| {
            // Generate unique indices by shuffling and taking first nnz
            let indices = proptest::collection::vec(0u32..dim, nnz..=nnz).prop_map(|mut v| {
                v.sort();
                v.dedup();
                v
            });
            let values = proptest::collection::vec(-100.0f32..100.0, nnz..=nnz)
                .prop_filter("nonzero", |v| v.iter().all(|x| *x != 0.0 && x.is_finite()));
            (Just(dim), indices, values)
        })
        .prop_filter_map("valid vector", |(dim, indices, values)| {
            if indices.is_empty() {
                return None;
            }
            // Trim values to match indices if needed
            let len = indices.len().min(values.len());
            if len == 0 {
                return None;
            }
            SparseVector::new(indices[..len].to_vec(), values[..len].to_vec(), dim).ok()
        })
}

// =============================================================================
// PROPERTY TESTS
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Property: get(id) returns None after delete(id)
    #[test]
    fn prop_get_none_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        // Before delete: get returns Some
        prop_assert!(storage.get(id).is_some());

        // After delete: get returns None
        storage.delete(id).unwrap();
        prop_assert!(storage.get(id).is_none());
    }

    /// Property: delete(id) twice -> second returns false
    #[test]
    fn prop_double_delete_returns_false(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        let first = storage.delete(id).unwrap();
        let second = storage.delete(id).unwrap();

        prop_assert!(first, "First delete should return true");
        prop_assert!(!second, "Second delete should return false");
    }

    /// Property: is_deleted(id) == true after delete(id)
    #[test]
    fn prop_is_deleted_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        prop_assert!(!storage.is_deleted(id), "Before delete");
        storage.delete(id).unwrap();
        prop_assert!(storage.is_deleted(id), "After delete");
    }

    /// Property: exists(id) remains true after delete(id)
    #[test]
    fn prop_exists_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        prop_assert!(storage.exists(id), "Before delete");
        storage.delete(id).unwrap();
        prop_assert!(storage.exists(id), "After delete - should still exist");
        prop_assert!(storage.is_deleted(id), "After delete - should be marked deleted");
    }

    /// Property: active_count + deleted_count == total_count
    #[test]
    fn prop_count_invariant(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        // Insert all vectors
        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        // Delete some (ignore out of bounds)
        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        // Invariant check
        prop_assert_eq!(
            storage.active_count() + storage.deleted_count(),
            storage.total_count(),
            "active + deleted must equal total"
        );
    }

    /// Property: iter().count() == active_count()
    #[test]
    fn prop_iter_count_equals_active(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        let iter_count = storage.iter().count();
        let active = storage.active_count();

        prop_assert_eq!(iter_count, active,
            "iter().count() must equal active_count()");
    }

    /// Property: ids() yields exactly the non-deleted IDs
    #[test]
    fn prop_ids_matches_non_deleted(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();
        let mut deleted_set = HashSet::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                if storage.delete(ids[idx]).unwrap_or(false) {
                    deleted_set.insert(ids[idx]);
                }
            }
        }

        let iter_ids: HashSet<_> = storage.ids().collect();
        let expected: HashSet<_> = ids.iter()
            .copied()
            .filter(|id| !deleted_set.contains(id))
            .collect();

        prop_assert_eq!(iter_ids, expected);
    }

    /// Property: delete(nonexistent) returns IdNotFound
    #[test]
    fn prop_delete_nonexistent_fails(
        vecs in proptest::collection::vec(arb_sparse_vector(), 0..=5)
    ) {
        let mut storage = SparseStorage::new();

        for vec in &vecs {
            storage.insert(vec).unwrap();
        }

        // ID beyond total count should fail
        let fake_id = SparseId::new(storage.total_count() as u64 + 100);
        let result = storage.delete(fake_id);

        prop_assert!(matches!(result, Err(SparseError::IdNotFound(_))));
    }

    /// Property: deletion_ratio in [0.0, 1.0]
    #[test]
    fn prop_deletion_ratio_bounded(
        vecs in proptest::collection::vec(arb_sparse_vector(), 1..=20),
        delete_indices in proptest::collection::vec(0usize..20, 0..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids = Vec::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        for idx in delete_indices {
            if idx < ids.len() {
                let _ = storage.delete(ids[idx]);
            }
        }

        let ratio = storage.deletion_ratio();
        prop_assert!(ratio >= 0.0 && ratio <= 1.0,
            "deletion_ratio must be in [0.0, 1.0], got {}", ratio);
    }

    /// Property: is_deleted returns true for non-existent IDs
    #[test]
    fn prop_is_deleted_nonexistent(
        vecs in proptest::collection::vec(arb_sparse_vector(), 0..=10)
    ) {
        let mut storage = SparseStorage::new();

        for vec in &vecs {
            storage.insert(vec).unwrap();
        }

        // Non-existent ID
        let fake_id = SparseId::new(storage.total_count() as u64 + 1000);
        prop_assert!(storage.is_deleted(fake_id), "Non-existent ID should return true for is_deleted");
        prop_assert!(!storage.exists(fake_id), "Non-existent ID should return false for exists");
    }

    /// Property: contains(id) == exists(id) && !is_deleted(id)
    #[test]
    fn prop_contains_equivalence(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        // Before delete
        prop_assert_eq!(
            storage.contains(id),
            storage.exists(id) && !storage.is_deleted(id)
        );

        // After delete
        storage.delete(id).unwrap();
        prop_assert_eq!(
            storage.contains(id),
            storage.exists(id) && !storage.is_deleted(id)
        );

        // For non-existent
        let fake = SparseId::new(999);
        prop_assert_eq!(
            storage.contains(fake),
            storage.exists(fake) && !storage.is_deleted(fake)
        );
    }

    /// Property: restored vectors are accessible again
    #[test]
    fn prop_restore_makes_accessible(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        // Delete
        storage.delete(id).unwrap();
        prop_assert!(storage.get(id).is_none());

        // Restore
        storage.restore(id).unwrap();
        prop_assert!(storage.get(id).is_some());
        prop_assert!(!storage.is_deleted(id));
    }

    /// Property: zero-copy accessors return None for deleted
    #[test]
    fn prop_zero_copy_none_after_delete(vec in arb_sparse_vector()) {
        let mut storage = SparseStorage::new();
        let id = storage.insert(&vec).unwrap();

        // Before delete: all accessors work
        prop_assert!(storage.get_indices(id).is_some());
        prop_assert!(storage.get_values(id).is_some());
        prop_assert!(storage.get_dim(id).is_some());

        // After delete: all return None
        storage.delete(id).unwrap();
        prop_assert!(storage.get_indices(id).is_none());
        prop_assert!(storage.get_values(id).is_none());
        prop_assert!(storage.get_dim(id).is_none());
    }

    /// Property: delete_batch is atomic on invalid ID
    #[test]
    fn prop_delete_batch_atomic(
        vecs in proptest::collection::vec(arb_sparse_vector(), 2..=10)
    ) {
        let mut storage = SparseStorage::new();
        let mut ids: Vec<SparseId> = Vec::new();

        for vec in &vecs {
            ids.push(storage.insert(vec).unwrap());
        }

        let initial_active = storage.active_count();

        // Try to delete with an invalid ID
        let mut batch = vec![ids[0], ids[1]];
        batch.push(SparseId::new(9999)); // Invalid

        let result = storage.delete_batch(&batch);
        prop_assert!(result.is_err());

        // All vectors should still be active (atomicity)
        prop_assert_eq!(storage.active_count(), initial_active);
    }
}
