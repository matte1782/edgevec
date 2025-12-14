#![no_main]
use edgevec::hnsw::{HnswConfig, HnswIndex, SearchContext, Searcher};
use edgevec::metric::L2Squared;
use edgevec::storage::VectorStorage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // 4D vectors = 16 bytes
    let dim = 4;
    let mut config = HnswConfig::new(dim);
    config.m = 8;
    config.m0 = 16;
    config.ef_construction = 20;

    // In-memory storage (None for WAL)
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let mut cursor = 0;
    let mut ctx = SearchContext::new();

    while cursor < data.len() {
        // Read command byte
        let cmd = data[cursor];
        cursor += 1;

        // Ensure enough bytes for a vector
        if cursor + (dim as usize * 4) > data.len() {
            break;
        }

        // Read vector bytes
        let vec_bytes = &data[cursor..cursor + (dim as usize * 4)];
        cursor += dim as usize * 4;

        // Convert to f32
        let mut vector = vec![0.0f32; dim as usize];
        for (i, chunk) in vec_bytes.chunks_exact(4).enumerate() {
            // Fuzzing with arbitrary floats can cause issues if NaNs propagate to Metric
            // We enforce finiteness to focus on Graph Logic stability, not Float logic.
            let val = f32::from_le_bytes(chunk.try_into().unwrap());
            if val.is_finite() {
                vector[i] = val;
            } else {
                vector[i] = 0.0;
            }
        }

        if cmd < 220 {
            // INSERT (86% chance)
            // Ignore result, we just want to ensure no panic
            let _ = index.insert(&vector, &mut storage);
        } else {
            // SEARCH (14% chance) - Verify structural integrity
            if let Some(entry_point) = index.entry_point() {
                let searcher = Searcher::<L2Squared, VectorStorage>::new(&index, &storage);

                // Perform a search on layer 0 (base layer) starting from entry point.
                // This stresses the neighbor traversal logic.
                let _ = searcher.search_layer(
                    &mut ctx,
                    [entry_point],
                    &vector,
                    10, // ef
                    0,  // target level
                );
            }
        }
    }
});
