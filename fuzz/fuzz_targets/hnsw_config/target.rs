#![no_main]
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Strategy: Interpret fuzz data as HnswConfig parameters
    // We need at least enough bytes for basic config fields
    if data.len() < 20 {
        return;
    }

    // Extract parameters safely
    // 0-3: m (u32)
    // 4-7: m0 (u32)
    // 8-11: ef_construction (u32)
    // 12-15: ef_search (u32)
    // 16-19: dimensions (u32)

    let m = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let m0 = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let ef_construction = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let ef_search = u32::from_le_bytes(data[12..16].try_into().unwrap());
    let dimensions = u32::from_le_bytes(data[16..20].try_into().unwrap());

    // Construct config (manually, since we want to fuzz the constructor validation)
    // Note: HnswConfig fields are public, so we can set them directly to test validation logic
    let mut config = HnswConfig::new(dimensions);
    config.m = m;
    config.m0 = m0;
    config.ef_construction = ef_construction;
    config.ef_search = ef_search;

    // Create storage with matching dimensions (so we pass the first check and hit the logic checks)
    // Note: VectorStorage requires dimensions > 0 usually, but let's see how it handles 0 or huge
    let storage = VectorStorage::new(&config, None);

    // Test the invariant: HnswIndex::new should NEVER panic
    let result = HnswIndex::new(config, &storage);

    // Verification
    match result {
        Ok(index) => {
            // If it succeeded, the state must be valid
            assert_eq!(index.node_count(), 0);
            assert!(index.entry_point().is_none());
        }
        Err(_) => {
            // Error is acceptable (and expected for garbage input), panic is not
        }
    }
});
