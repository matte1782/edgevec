//! Dense and sparse storage use different ID namespaces.
#![cfg(feature = "sparse")]
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::hybrid::HybridError;
use edgevec::hybrid::{linear_fusion, HybridSearchConfig, HybridSearcher};
use edgevec::sparse::{SparseStorage, SparseVector};
use edgevec::storage::VectorStorage;

#[test]
fn mapping_fuses_documents_and_filters_before_sparse_top_k() {
    let config = HnswConfig::new(2);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();
    index.insert(&[1.0, 0.0], &mut storage).unwrap();
    index.insert(&[0.0, 1.0], &mut storage).unwrap();
    let mut sparse = SparseStorage::new();
    sparse
        .insert(&SparseVector::singleton(0, 10.0, 10).unwrap())
        .unwrap();
    sparse
        .insert(&SparseVector::singleton(0, 1.0, 10).unwrap())
        .unwrap();
    let query = SparseVector::singleton(0, 1.0, 10).unwrap();
    let hits = HybridSearcher::new(&index, &storage, &sparse)
        .search_with_mapping(
            &[0.0, 1.0],
            &query,
            &HybridSearchConfig::rrf(2, 2, 2),
            |id| Some(id + 1),
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits
        .iter()
        .all(|hit| hit.dense_rank.is_some() && hit.sparse_rank.is_some()));
    for (alpha, expected) in [(1.0, 2), (0.0, 1)] {
        let hits = HybridSearcher::new(&index, &storage, &sparse)
            .search_with_mapping(
                &[0.0, 1.0],
                &query,
                &HybridSearchConfig::linear(2, 2, 1, alpha).unwrap(),
                |id| Some(id + 1),
            )
            .unwrap();
        assert_eq!(hits[0].id, VectorId(expected));
    }
    index.soft_delete(VectorId(1)).unwrap();
    let hits = HybridSearcher::new(&index, &storage, &sparse)
        .search_with_mapping(
            &[0.0, 1.0],
            &query,
            &HybridSearchConfig::rrf(2, 1, 1),
            |id| (id == 1).then_some(2),
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, VectorId(2));
    assert_eq!(hits[0].sparse_score, Some(1.0));
    assert!(hits[0].dense_rank.is_some());
}

#[test]
fn signed_extreme_similarity_scores_normalize_without_overflow() {
    let hits = linear_fusion(&[(1, -f32::MAX), (2, 0.0), (3, f32::MAX)], &[], 1.0, 3);
    assert_eq!(
        hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert!(hits.iter().all(|hit| hit.score.is_finite()));
    assert!((hits[1].score - 0.5).abs() < f32::EPSILON);
}

#[test]
fn hybrid_rejects_overflowing_sparse_dot_products() {
    let config = HnswConfig::new(2);
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).unwrap();
    let mut sparse = SparseStorage::new();
    let vector = SparseVector::singleton(0, f32::MAX, 10).unwrap();
    sparse.insert(&vector).unwrap();
    let result = HybridSearcher::new(&index, &storage, &sparse).search_with_mapping(
        &[0.0, 0.0],
        &vector,
        &HybridSearchConfig::rrf(0, 1, 1),
        Some,
    );
    assert!(matches!(result, Err(HybridError::SparseSearchError(_))));
}
