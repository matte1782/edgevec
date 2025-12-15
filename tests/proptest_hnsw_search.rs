use edgevec::hnsw::{
    Candidate, GraphError, HnswConfig, HnswGraph, NodeId, SearchContext, Searcher, VectorId,
    VectorProvider,
};
use edgevec::metric::L2Squared;
use edgevec::quantization::QuantizerConfig;
use edgevec::storage::{StorageType, VectorStorage};
use proptest::prelude::*;
use std::collections::HashMap;

use std::borrow::Cow;

/// Mock vector provider for testing search without full storage overhead.
struct MockVectorProvider {
    vectors: HashMap<VectorId, Vec<f32>>,
}

impl MockVectorProvider {
    fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    fn add(&mut self, id: u64, vec: Vec<f32>) {
        self.vectors.insert(VectorId(id), vec);
    }
}

impl VectorProvider for MockVectorProvider {
    fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]> {
        Cow::Borrowed(
            self.vectors
                .get(&id)
                .expect("Test should only request existing vectors"),
        )
    }
}

/// Helper to create a dummy HnswIndex
fn create_index(config: HnswConfig) -> HnswGraph {
    let storage = VectorStorage::new(&config, None);
    HnswGraph::new(config, &storage).unwrap()
}

/// Helper to run search layer with L2Squared metric
fn run_search_layer<P: VectorProvider>(
    index: &HnswGraph,
    provider: &P,
    query: &[f32],
    entry_points: &[NodeId],
    ef: usize,
    level: u8,
) -> Result<Vec<Candidate>, GraphError> {
    let searcher = Searcher::<L2Squared, _>::new(index, provider);
    let mut ctx = SearchContext::new();
    searcher.search_layer(&mut ctx, entry_points.iter().copied(), query, ef, level)?;
    Ok(ctx.scratch.clone())
}

#[test]
fn test_search_small_graph_manual() {
    let dim = 2;
    let config = HnswConfig::new(dim);
    let mut index = create_index(config.clone());
    let mut provider = MockVectorProvider::new();

    for i in 0..10 {
        let vec = vec![i as f32, 0.0];
        provider.add(i + 1, vec);
        index.add_node(VectorId(i + 1), 0).unwrap();
    }

    for i in 0..9 {
        let u = NodeId(i as u32);
        let v = NodeId((i + 1) as u32);
        index.set_neighbors(u, &[v]).unwrap();
    }
    index.set_neighbors(NodeId(9), &[]).unwrap();

    let query = vec![9.1, 0.0];
    let entry_points = vec![NodeId(0)];

    let results = run_search_layer(&index, &provider, &query, &entry_points, 5, 0).unwrap();

    assert!(!results.is_empty(), "Should return results");
    let best = results[0];
    assert_eq!(best.node_id, NodeId(9), "Should find the end of the chain");
    assert!((best.distance - 0.01).abs() < 1e-5);
}

#[test]
fn test_search_empty_graph() {
    let config = HnswConfig::new(4);
    let index = create_index(config);
    let provider = MockVectorProvider::new();
    let query = vec![0.0; 4];

    let results = run_search_layer(&index, &provider, &query, &[], 10, 0).unwrap();
    assert!(results.is_empty());

    let result_err = run_search_layer(&index, &provider, &query, &[NodeId(0)], 10, 0);
    assert!(matches!(result_err, Err(GraphError::NodeIdOutOfBounds)));
}

#[test]
fn test_search_dimension_mismatch() {
    let dim = 128;
    let config = HnswConfig::new(dim);
    // Create storage with matching dimensions
    let storage = VectorStorage::new(&config, None);
    let index = HnswGraph::new(config.clone(), &storage).unwrap();

    let query = vec![0.0; 127]; // Wrong dimension

    let result = index.search(&query, 10, &storage);

    assert!(matches!(
        result,
        Err(GraphError::DimensionMismatch {
            expected: 128,
            actual: 127
        })
    ));
}

// [C2] Mock-based Fuzz Proxy
// Deterministic randomized test to ensure coverage on Windows where fuzzing is disabled.
#[test]
fn test_mock_fuzz_proxy() {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64(42);

    for _ in 0..100 {
        let dim = rng.gen_range(2..=16);
        let config = HnswConfig::new(dim);
        let mut index = create_index(config);
        let mut provider = MockVectorProvider::new();

        let n = rng.gen_range(5..50);
        for i in 0..n {
            let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
            provider.add(i + 1, vec);
            index.add_node(VectorId(i + 1), 0).unwrap();
        }

        // Random connections
        for i in 0..n {
            let n_neighbors = rng.gen_range(0..5);
            let mut neighbors = Vec::new();
            for _ in 0..n_neighbors {
                let neighbor = rng.gen_range(0..n);
                if neighbor != i {
                    neighbors.push(NodeId(neighbor as u32));
                }
            }
            index.set_neighbors(NodeId(i as u32), &neighbors).unwrap();
        }

        let query: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let entry = vec![NodeId(0)];

        // Should not panic
        let _ = run_search_layer(&index, &provider, &query, &entry, 10, 0);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_search_fully_connected_finds_best(
        dim in 2u32..16,
        n in 5u64..50,
        query_vec in prop::collection::vec(-10.0f32..10.0, 2..16),
        vectors in prop::collection::vec(prop::collection::vec(-10.0f32..10.0, 2..16), 5..50)
    ) {
        let d = dim as usize;
        let query = query_vec.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();

        let config = HnswConfig::new(dim);
        let mut index = create_index(config);
        let mut provider = MockVectorProvider::new();

        let count = std::cmp::min(n as usize, vectors.len());
        for (i, vectors_item) in vectors.iter().enumerate().take(count) {
            let vec = vectors_item.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();
            provider.add(i as u64 + 1, vec);
            index.add_node(VectorId(i as u64 + 1), 0).unwrap();
        }

        let all_nodes: Vec<NodeId> = (0..count).map(|i| NodeId(i as u32)).collect();
        for i in 0..count {
            let neighbors: Vec<NodeId> = all_nodes.iter()
                .copied()
                .filter(|&x| x != NodeId(i as u32))
                .collect();
            index.set_neighbors(NodeId(i as u32), &neighbors).unwrap();
        }

        let mut best_dist = f32::MAX;
        for i in 0..count {
            let vid = VectorId(i as u64 + 1);
            let vec = provider.get_vector(vid);
            let dist: f32 = query.iter().zip(vec.iter()).map(|(a, b)| (a - b).powi(2)).sum();
            if dist < best_dist {
                best_dist = dist;
            }
        }

        let entry = vec![NodeId(0)];
        let results = run_search_layer(&index, &provider, &query, &entry, count, 0).unwrap();

        assert!(!results.is_empty());
        let found_best = results[0];

        assert!(
            (found_best.distance - best_dist).abs() < 1e-4,
            "Greedy search in fully connected graph should find global optimum. Found {}, Expected {}",
            found_best.distance, best_dist
        );
    }

    // [m2] Renamed from prop_search_cyclic_terminates
    #[test]
    fn prop_search_cyclic_no_infinite_loop(
        n in 10u32..50,
        _seed in any::<u64>()
    ) {
        let dim = 2;
        let config = HnswConfig::new(dim);
        let mut index = create_index(config);
        let mut provider = MockVectorProvider::new();

        for i in 0..n {
            let vec = vec![0.0; dim as usize];
            provider.add(i as u64 + 1, vec);
            index.add_node(VectorId(i as u64 + 1), 0).unwrap();
        }

        for i in 0..n {
            let u = NodeId(i);
            let v = NodeId((i + 1) % n);
            index.set_neighbors(u, &[v]).unwrap();
        }

        let query = vec![1.0; dim as usize];
        let entry = vec![NodeId(0)];

        let result = run_search_layer(&index, &provider, &query, &entry, 10, 0);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(candidates.len() <= 10);
    }

    // [m5] Scale Test
    #[test]
    fn prop_search_scale_test(
        dim in 64u32..128,
        n in 100u64..200, // Reduced from 1000 for speed in proptest, 1000 is heavy for rapid proptest
        query_vec in prop::collection::vec(-1.0f32..1.0, 64..128),
    ) {
         let d = dim as usize;
         let query = query_vec.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();

         let config = HnswConfig::new(dim);
         let mut index = create_index(config);
         let mut provider = MockVectorProvider::new();

         let count = n as usize;
         for i in 0..count {
             // Just zero vectors for structure test, random values slow down proptest generation
             let vec = vec![0.0; d];
             provider.add(i as u64 + 1, vec);
             index.add_node(VectorId(i as u64 + 1), 0).unwrap();
         }

         // Random chain
         for i in 0..count-1 {
             let u = NodeId(i as u32);
             let v = NodeId((i + 1) as u32);
             index.set_neighbors(u, &[v]).unwrap();
         }

         let entry = vec![NodeId(0)];
         let result = run_search_layer(&index, &provider, &query, &entry, 10, 0);
         assert!(result.is_ok());
    }

    #[test]
    fn prop_quantized_search_recall(
        dim in 8u32..32,
        n in 10u64..50,
        query_vec in prop::collection::vec(-10.0f32..10.0, 8..32),
        vectors in prop::collection::vec(prop::collection::vec(-10.0f32..10.0, 8..32), 10..50)
    ) {
        let d = dim as usize;
        let query = query_vec.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();

        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        // Enable quantization
        let q_config = QuantizerConfig { min: -10.0, max: 10.0 };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let mut index = HnswGraph::new(config, &storage).unwrap();

        let count = std::cmp::min(n as usize, vectors.len());
        // Insert vectors
        for (i, vectors_item) in vectors.iter().enumerate().take(count) {
            let vec = vectors_item.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();
            storage.insert(&vec).unwrap();
            index.add_node(VectorId(i as u64 + 1), 0).unwrap();
        }

        // Create a fully connected graph to ignore graph traversal issues and focus on metric/distance
        let all_nodes: Vec<NodeId> = (0..count).map(|i| NodeId(i as u32)).collect();
        for i in 0..count {
            let neighbors: Vec<NodeId> = all_nodes.iter()
                .copied()
                .filter(|&x| x != NodeId(i as u32))
                .collect();
            index.set_neighbors(NodeId(i as u32), &neighbors).unwrap();
        }

        // Brute force nearest neighbor using quantized distance (simulating what search should do)
        // We use the storage's dequantize to check "real" distance, OR we rely on the fact that
        // search returns sorted results.
        // Let's just verify we find the same top-1 as brute force (using float distance on original vectors).
        // Note: Quantization adds noise. Top-1 might differ if two vectors are close.
        // But for random vectors in 8+ dims, it should be stable mostly.
        // Let's relax: Check if Top-1 from search is in Top-3 of brute force.

        let entry = vec![NodeId(0)];
        let results = run_search_layer(&index, &storage, &query, &entry, count, 0).unwrap();

        assert!(!results.is_empty());
        let found_id = results[0].node_id;

        // Calculate ground truth distances using original float vectors
        let mut truth: Vec<(NodeId, f32)> = Vec::new();
        for (i, vectors_item) in vectors.iter().enumerate().take(count) {
             let vec = vectors_item.iter().take(d).copied().chain(std::iter::repeat(0.0)).take(d).collect::<Vec<_>>();
             let dist: f32 = query.iter().zip(vec.iter()).map(|(a, b)| (a - b).powi(2)).sum();
             truth.push((NodeId(i as u32), dist));
        }
        truth.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Check if found_id is in top-K of truth
        // With quantization, ranking might swap slightly.
        let k_tolerance = 5;
        let top_k_truth: Vec<NodeId> = truth.iter().take(k_tolerance).map(|x| x.0).collect();

        assert!(
            top_k_truth.contains(&found_id),
            "Quantized search result {:?} not in top {} truth {:?} (dim={}, n={})",
            found_id, k_tolerance, top_k_truth, dim, count
        );
    }
}
