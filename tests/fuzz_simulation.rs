#![cfg(target_arch = "wasm32")]
use edgevec::hnsw::VectorId;
use edgevec::wasm::EdgeVec;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

// Mock Op for Graph Ops
#[derive(Debug, Clone)]
enum Op {
    Insert { vector: Vec<f32> },
    Delete { id: u64 },
    Search { vector: Vec<f32>, k: u8 },
    SaveLoad,
}

#[test]
fn fuzz_simulation_persistence_load() {
    // Reduced to 1s for CI sanity.
    // Run with higher duration for campaign.
    let duration = Duration::from_secs(1);
    let start = Instant::now();
    let mut rng = rand::thread_rng();
    let mut buffer = vec![0u8; 1024];
    let mut iterations = 0;

    println!("Starting persistence_load fuzz simulation (smoke test)...");
    while start.elapsed() < duration {
        // Generate random length and data
        let len = rng.gen_range(0..buffer.len());
        rng.fill(&mut buffer[0..len]);
        let data = &buffer[0..len];

        // Action: Load
        // Invariant: MUST return Result, NEVER panic.
        if let Ok(db) = postcard::from_bytes::<EdgeVec>(data) {
            let _ = db;
        }
        iterations += 1;
    }
    println!(
        "persistence_load: {} iterations in 1s. Status: PASSED",
        iterations
    );
}

#[test]
fn fuzz_simulation_graph_ops() {
    // Reduced to 1s for CI sanity.
    let duration = Duration::from_secs(1);
    let start = Instant::now();
    let dim = 4;
    let mut iterations = 0;

    // Deterministic RNG for regression stability
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEADBEEF);

    println!("Starting graph_ops fuzz simulation (smoke test)...");

    let config = HnswConfig::new(dim as u32);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();
    let mut inserted_ids: Vec<VectorId> = Vec::new();

    while start.elapsed() < duration {
        let op_type = rng.gen_range(0..4);
        let op = match op_type {
            0 => {
                // Insert
                let vec: Vec<f32> = (0..dim).map(|_| rng.gen()).collect();
                Op::Insert { vector: vec }
            }
            1 => {
                // Delete
                let id = rng.gen();
                Op::Delete { id }
            }
            2 => {
                // Search
                let vec: Vec<f32> = (0..dim).map(|_| rng.gen()).collect();
                let k = rng.gen_range(1..20);
                Op::Search { vector: vec, k }
            }
            3 => Op::SaveLoad,
            _ => unreachable!(),
        };

        match op {
            Op::Insert { mut vector } => {
                for v in &mut vector {
                    if !v.is_finite() {
                        *v = 0.0;
                    }
                }
                if let Ok(id) = index.insert(&vector, &mut storage) {
                    inserted_ids.push(id);
                }
            }
            Op::Delete { id } => {
                let vid = VectorId(id);
                // RFC-001: soft_delete returns Result<bool>, never panics on invalid ID
                let _ = index.soft_delete(vid);
            }
            Op::Search { mut vector, k } => {
                for v in &mut vector {
                    if !v.is_finite() {
                        *v = 0.0;
                    }
                }
                let k_usize = k as usize;
                if k_usize > 0 {
                    let _ = index.search(&vector, k_usize, &storage);
                }
            }
            Op::SaveLoad => {
                if let Ok(index_bytes) = postcard::to_stdvec(&index) {
                    if let Ok(storage_bytes) = postcard::to_stdvec(&storage) {
                        if let Ok(restored_index) = postcard::from_bytes::<HnswIndex>(&index_bytes)
                        {
                            if let Ok(restored_storage) =
                                postcard::from_bytes::<VectorStorage>(&storage_bytes)
                            {
                                index = restored_index;
                                storage = restored_storage;
                            }
                        }
                    }
                }
            }
        }
        iterations += 1;
    }
    println!("graph_ops: {} iterations in 1s. Status: PASSED", iterations);
}
