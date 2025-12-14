#![no_main]
use edgevec::hnsw::{
    HnswConfig, HnswIndex, NodeId, SearchContext, Searcher, VectorId, VectorProvider,
};
use edgevec::metric::L2Squared;
use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;

/// Mock storage for fuzzer that provides vectors for search operations.
/// This decouples the fuzz target from the VectorStorage implementation.
struct FuzzStorage {
    vectors: Vec<Vec<f32>>,
}

impl VectorProvider for FuzzStorage {
    // VectorProvider::get_vector now returns Cow<'_, [f32]>
    fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]> {
        let idx = (id.0 as usize).saturating_sub(1);
        if idx < self.vectors.len() {
            Cow::Borrowed(&self.vectors[idx])
        } else {
            // Fallback for invalid IDs if they somehow pass graph checks
            Cow::Borrowed(&self.vectors[0])
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // Min size: 4 bytes for NodeId + 1 float (4 bytes)
    if data.len() < 8 {
        return;
    }

    // 1. Parse Entry Point (first 4 bytes)
    let entry_point_raw = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let entry_point = NodeId(entry_point_raw);

    // 2. Parse Query Vector (remaining bytes as f32s)
    let query_bytes = &data[4..];
    // Need at least 4 bytes per f32
    if query_bytes.len() < 4 {
        return;
    }

    let num_floats = query_bytes.len() / 4;
    let mut query: Vec<f32> = Vec::with_capacity(num_floats);
    for i in 0..num_floats {
        let start = i * 4;
        let bytes: [u8; 4] = [
            query_bytes[start],
            query_bytes[start + 1],
            query_bytes[start + 2],
            query_bytes[start + 3],
        ];
        query.push(f32::from_le_bytes(bytes));
    }

    // Sanitize query (Metric panics on NaN)
    if query.iter().any(|x| x.is_nan()) {
        return;
    }

    // Ensure non-empty query
    if query.is_empty() {
        return;
    }

    // 3. Setup Static Graph (Small Diamond)
    // Vectors: fill with different values based on index
    // Dimension must match query length.
    let dim = query.len();

    let vectors = vec![
        vec![0.0; dim],
        vec![1.0; dim],
        vec![0.5; dim],
        vec![0.1; dim],
    ];

    let storage = FuzzStorage { vectors };

    // Create HnswConfig with correct dimensions
    let config = HnswConfig::new(dim as u32);

    // HnswIndex::new requires a VectorStorage for validation.
    // Create a minimal valid VectorStorage for initialization.
    let dummy_storage = edgevec::storage::VectorStorage::new(&config, None);
    let mut index = match HnswIndex::new(config, &dummy_storage) {
        Ok(i) => i,
        Err(_) => return,
    };

    // Create 4 nodes manually
    let n1 = match index.add_node(VectorId(1), 0) {
        Ok(n) => n,
        Err(_) => return,
    };
    let n2 = match index.add_node(VectorId(2), 0) {
        Ok(n) => n,
        Err(_) => return,
    };
    let n3 = match index.add_node(VectorId(3), 0) {
        Ok(n) => n,
        Err(_) => return,
    };
    let n4 = match index.add_node(VectorId(4), 0) {
        Ok(n) => n,
        Err(_) => return,
    };

    // Connect them in a diamond pattern
    let _ = index.set_neighbors(n1, &[n2, n3]);
    let _ = index.set_neighbors(n2, &[n4]);
    let _ = index.set_neighbors(n3, &[n4]);
    let _ = index.set_neighbors(n4, &[]); // Sink

    // 4. Run Search
    // Searcher::new takes &HnswIndex and a VectorProvider
    let searcher = Searcher::<L2Squared, _>::new(&index, &storage);

    // Create search context
    let mut ctx = SearchContext::new();

    // We specifically want to fuzz the `search_layer` robustness against random NodeIds
    // passed as entry_point.
    // search_layer signature: (ctx, entry_points, query, ef, level)
    let _result = searcher.search_layer(&mut ctx, [entry_point], &query, 10, 0);

    // We don't assert result correctness here (that's for proptest),
    // just that it DOES NOT PANIC (unless Metric panics on NaN, which we handled).
    // search_layer returns Result, so it should handle invalid entry points gracefully.
});
