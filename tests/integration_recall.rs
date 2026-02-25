use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metric::{L2Squared, Metric};
use edgevec::storage::VectorStorage;
use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};

#[test]
fn test_integration_recall_001() {
    const DIM: usize = 64;
    const NUM_VECTORS: usize = 1000;
    const NUM_QUERIES: usize = 100;
    const SEED: u64 = 42;
    const EXPECTED_RECALL: f32 = 0.95;

    // 1. Deterministic RNG
    let mut rng = StdRng::seed_from_u64(SEED);

    // 2. Setup Index
    let mut config = HnswConfig::new(DIM as u32);
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = 100; // Good quality
    config.ef_search = 64; // High enough for recall

    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Index creation failed");

    // 3. Generate Data
    let mut vectors = Vec::with_capacity(NUM_VECTORS);
    for _ in 0..NUM_VECTORS {
        let vec: Vec<f32> = (0..DIM).map(|_| rng.random::<f32>()).collect();
        vectors.push(vec);
    }

    // 4. Insert Data
    for vec in &vectors {
        index.insert(vec, &mut storage).expect("Insertion failed");
    }

    // 5. Generate Queries
    let mut queries = Vec::with_capacity(NUM_QUERIES);
    for _ in 0..NUM_QUERIES {
        let query: Vec<f32> = (0..DIM).map(|_| rng.random::<f32>()).collect();
        queries.push(query);
    }

    // 6. Benchmark
    let mut matches = 0;

    for query in &queries {
        // A. Ground Truth (Brute Force)
        let mut best_dist = f32::MAX;
        let mut best_idx = 0; // 0-based index in `vectors`

        for (i, vec) in vectors.iter().enumerate() {
            let dist = L2Squared::distance(query, vec);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        // VectorId is 1-based (i + 1)
        let best_vid = (best_idx + 1) as u64;

        // B. HNSW Search
        // We ask for k=1 to be strict about recall@1
        let results = index.search(query, 1, &storage).expect("Search failed");

        // C. Compare
        if let Some(res) = results.first() {
            if res.vector_id.0 == best_vid {
                matches += 1;
            } else {
                // Optional: Check if distances are extremely close (duplicate vectors or float noise)
                // For random float vectors in 64D, exact collisions are unlikely.
                // We'll trust the ID match.
            }
        }
    }

    let recall = matches as f32 / NUM_QUERIES as f32;
    println!(
        "Recall@1: {:.4} (Matches: {}/{})",
        recall, matches, NUM_QUERIES
    );

    assert!(
        recall > EXPECTED_RECALL,
        "Recall {:.4} is below threshold {}",
        recall,
        EXPECTED_RECALL
    );
}
