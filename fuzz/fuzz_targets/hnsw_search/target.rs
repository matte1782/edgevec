#![no_main]

use arbitrary::Unstructured;
use edgevec::hnsw::{
    HnswConfig, HnswIndex, NodeId, SearchContext, Searcher, VectorId, VectorProvider,
};
use edgevec::metric::L2Squared;
use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;
use std::collections::HashMap;

// [M5] Fix Fuzz Dependency: Use MockProvider instead of VectorStorage
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
                .unwrap_or_else(|| panic!("Mock provider missing vector {:?}", id)),
        )
    }
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    // Config
    let dimensions = match u.int_in_range(2..=16) {
        Ok(d) => d,
        Err(_) => return,
    };

    let config = HnswConfig::new(dimensions);

    // Create random vectors for mock storage
    let node_count = match u.int_in_range(5..=50) {
        Ok(n) => n,
        Err(_) => return,
    };

    // Use MockProvider to decouple from storage logic
    let mut provider = MockVectorProvider::new();
    // We need a dummy storage just to initialize HnswIndex,
    // or we use internal APIs if possible. HnswIndex::new takes generic P? No, specific VectorStorage currently?
    // Checking HnswIndex::new signature: `pub fn new(config: HnswConfig, storage: &VectorStorage) -> Result<Self, GraphError>`
    // The Index currently depends on VectorStorage for validation during init.
    // However, after init, Searcher can use any VectorProvider.
    // So we still need a dummy VectorStorage for HnswIndex creation, OR we cheat by creating HnswIndex with empty storage
    // and manually populating nodes.

    // Let's create a minimal valid storage for HnswIndex init.
    // Note: HnswIndex doesn't strictly require the storage to contain the vectors if we add nodes manually?
    // Actually HnswIndex checks storage for count/capacity.
    // But `add_node` only checks if we can add to graph.

    // To satisfy the type system, we must provide VectorStorage to HnswIndex::new.
    // We can use an empty one.
    let empty_storage = edgevec::storage::VectorStorage::new(&config, None);
    let mut index = match HnswIndex::new(config, &empty_storage) {
        Ok(i) => i,
        Err(_) => return,
    };

    // Populate provider and index nodes
    for i in 0..node_count {
        let mut vec = Vec::with_capacity(dimensions as usize);
        for _ in 0..dimensions {
            let val = match u.arbitrary::<f32>() {
                Ok(v) => {
                    if v.is_nan() {
                        0.0
                    } else {
                        v
                    }
                }
                Err(_) => return,
            };
            vec.push(val);
        }

        provider.add(i as u64 + 1, vec);

        // Add node to index (VectorId, level)
        let _ = index.add_node(VectorId(i as u64 + 1), 0);
    }

    // Links
    for i in 0..node_count {
        // Generate neighbor count
        let link_count = u.int_in_range(0..=16).unwrap_or_default();

        let mut neighbors = Vec::new();
        for _ in 0..link_count {
            let neighbor_idx = match u.int_in_range(0..=node_count - 1) {
                Ok(idx) => idx,
                Err(_) => continue,
            };
            if neighbor_idx != i {
                neighbors.push(NodeId(neighbor_idx as u32));
            }
        }

        let _ = index.set_neighbors(NodeId(i as u32), &neighbors);
    }

    // Query
    let mut query = Vec::with_capacity(dimensions as usize);
    for _ in 0..dimensions {
        let val = match u.arbitrary::<f32>() {
            Ok(v) => {
                if v.is_nan() {
                    0.0
                } else {
                    v
                }
            }
            Err(_) => return,
        };
        query.push(val);
    }

    // Entry points
    let ep_count = u.int_in_range(0..=5).unwrap_or_default();
    let mut entry_points = Vec::new();
    for _ in 0..ep_count {
        let idx = match u.int_in_range(0..=node_count - 1) {
            Ok(i) => i,
            Err(_) => continue,
        };
        entry_points.push(NodeId(idx as u32));
    }

    let ef = u.int_in_range(1..=100).unwrap_or(10) as usize;

    // EXECUTE SEARCH
    // Use the MockProvider here, which is safer/faster for fuzzing than real storage
    let searcher = Searcher::<L2Squared, _>::new(&index, &provider);
    let mut ctx = SearchContext::new();
    let _result = searcher.search_layer(&mut ctx, entry_points.iter().copied(), &query, ef, 0);
});
