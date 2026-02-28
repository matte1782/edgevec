//! Proptest-based fuzz simulation tests for EdgeVec.
//!
//! These tests exercise the same critical paths as the cargo-fuzz targets
//! (`fuzz/fuzz_targets/{filter_deep, persistence, hnsw_search}`) but use
//! proptest for structured random input generation so they can run on any
//! platform, including Windows where libFuzzer is unavailable.
//!
//! Configure iteration count via the `PROPTEST_CASES` environment variable
//! (default: 1000).

use proptest::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

use edgevec::filter::parse;
use edgevec::hnsw::{
    HnswConfig, HnswIndex, NodeId, SearchContext, Searcher, VectorId, VectorProvider,
};
use edgevec::metric::L2Squared;
use edgevec::persistence::{MemoryBackend, StorageBackend};
use edgevec::storage::VectorStorage;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read the desired number of proptest cases from the environment, falling
/// back to `default` when the variable is absent or unparseable.
fn cases(default: u32) -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Build a `ProptestConfig` respecting `PROPTEST_CASES` (default 1000).
fn config() -> ProptestConfig {
    ProptestConfig::with_cases(cases(1000))
}

// ---------------------------------------------------------------------------
// 1. Filter deep — mirrors fuzz/fuzz_targets/filter_deep/target.rs
// ---------------------------------------------------------------------------

/// Generate a deeply nested filter expression from byte-level input.
///
/// This is a direct port of the `generate_nested_filter` function from
/// the cargo-fuzz target, ensuring identical coverage.
fn generate_nested_filter(data: &[u8], max_depth: usize) -> String {
    if data.is_empty() || max_depth == 0 {
        return "x = 1".to_string();
    }

    let byte = data[0];
    let op = byte & 0b11;
    let selector = (byte >> 2) & 0b111111;

    match op {
        0 => {
            // AND
            let mid = data.len() / 2;
            let (left_data, right_data) = if data.len() > 1 {
                (&data[1..mid.max(1)], &data[mid..])
            } else {
                (&[][..], &[][..])
            };
            let left = generate_nested_filter(left_data, max_depth - 1);
            let right = generate_nested_filter(right_data, max_depth - 1);
            format!("({left} AND {right})")
        }
        1 => {
            // OR
            let mid = data.len() / 2;
            let (left_data, right_data) = if data.len() > 1 {
                (&data[1..mid.max(1)], &data[mid..])
            } else {
                (&[][..], &[][..])
            };
            let left = generate_nested_filter(left_data, max_depth - 1);
            let right = generate_nested_filter(right_data, max_depth - 1);
            format!("({left} OR {right})")
        }
        2 => {
            // NOT
            let inner = if data.len() > 1 {
                generate_nested_filter(&data[1..], max_depth - 1)
            } else {
                "x = 1".to_string()
            };
            format!("NOT ({inner})")
        }
        _ => {
            // Leaf
            let field = match selector % 8 {
                0 => "a",
                1 => "b",
                2 => "c",
                3 => "x",
                4 => "y",
                5 => "count",
                6 => "status",
                _ => "val",
            };
            let operator = match (selector >> 3) % 8 {
                0 => "=",
                1 => "!=",
                2 => ">",
                3 => "<",
                4 => ">=",
                5 => "<=",
                6 => "IS NULL",
                _ => "IS NOT NULL",
            };
            if operator.starts_with("IS") {
                format!("{field} {operator}")
            } else {
                let value = i32::from(selector);
                format!("{field} {operator} {value}")
            }
        }
    }
}

proptest! {
    #![proptest_config(config())]

    /// FUZZ-012 equivalent: parse(deeply_nested_filter) must return Result,
    /// never panic, regardless of how deeply nested or malformed the
    /// expression is.
    #[test]
    fn proptest_fuzz_filter_deep(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let filter = generate_nested_filter(&data, 50);
        // Must not panic. Ok or Err are both acceptable.
        let _ = parse(&filter);
    }

    /// Additional coverage: feed completely arbitrary strings to the parser.
    /// The parser must never panic on any input.
    #[test]
    fn proptest_fuzz_filter_arbitrary_string(input in ".*") {
        let _ = parse(&input);
    }
}

// ---------------------------------------------------------------------------
// 2. Persistence — mirrors fuzz/fuzz_targets/persistence/target.rs
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// Persistence deserialization must return Result, never panic, when
    /// given arbitrary (potentially corrupted) byte sequences.
    #[test]
    fn proptest_fuzz_persistence(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let backend = MemoryBackend::new();
        // Write arbitrary bytes into the backend.
        let _ = backend.atomic_write("", &data);
        // Attempt to read a snapshot — must not panic.
        let _ = edgevec::persistence::snapshot::read_snapshot(&backend);
    }

    /// Roundtrip: write a valid snapshot then read it back. The loaded
    /// index must have the same vector count as the original.
    #[test]
    fn proptest_fuzz_persistence_roundtrip(
        dim in 2u32..=32,
        num_vectors in 0usize..=20,
        seed in any::<u64>(),
    ) {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = match HnswIndex::new(config, &storage) {
            Ok(i) => i,
            Err(_) => return Ok(()),
        };

        let mut rng = SmallRng::seed_from_u64(seed);
        for _ in 0..num_vectors {
            let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0)).collect();
            let _ = index.insert(&vec, &mut storage);
        }

        // Save
        let mut backend = MemoryBackend::new();
        if edgevec::persistence::write_snapshot(&index, &storage, &mut backend).is_ok() {
            // Load back — must not panic
            let result = edgevec::persistence::read_snapshot(&backend);
            if let Ok((_loaded_index, loaded_storage)) = result {
                // Vector count must match
                prop_assert_eq!(
                    storage.len(),
                    loaded_storage.len(),
                    "Roundtrip vector count mismatch"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. HNSW search — mirrors fuzz/fuzz_targets/hnsw_search/target.rs
// ---------------------------------------------------------------------------

/// Mock vector provider for HNSW search fuzzing, identical to the one in the
/// cargo-fuzz target.
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
        Cow::Borrowed(self.vectors.get(&id).expect("Mock provider missing vector"))
    }
}

proptest! {
    #![proptest_config(config())]

    /// HNSW search must return Result (Ok or Err), never panic, when given
    /// random graph topologies, random vectors, and random query parameters.
    #[test]
    fn proptest_fuzz_hnsw_search(
        dimensions in 2u32..=16,
        node_count in 5usize..=50,
        ef in 1usize..=100,
        ep_count in 0usize..=5,
        seed in any::<u64>(),
    ) {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let mut rng = SmallRng::seed_from_u64(seed);

        let config = HnswConfig::new(dimensions);
        let empty_storage = VectorStorage::new(&config, None);
        let mut index = match HnswIndex::new(config, &empty_storage) {
            Ok(i) => i,
            Err(_) => return Ok(()),
        };

        // Build mock provider and populate graph nodes
        let mut provider = MockVectorProvider::new();
        for i in 0..node_count {
            let vec: Vec<f32> = (0..dimensions)
                .map(|_| {
                    let v: f32 = rng.gen();
                    if v.is_nan() { 0.0 } else { v }
                })
                .collect();
            provider.add(i as u64 + 1, vec);
            let _ = index.add_node(VectorId(i as u64 + 1), 0);
        }

        // Random neighbor links
        for i in 0..node_count {
            let link_count: usize = rng.gen_range(0..=16);
            let mut neighbors = Vec::new();
            for _ in 0..link_count {
                let neighbor_idx = rng.gen_range(0..node_count);
                if neighbor_idx != i {
                    neighbors.push(NodeId(neighbor_idx as u32));
                }
            }
            let _ = index.set_neighbors(NodeId(i as u32), &neighbors);
        }

        // Random query vector
        let query: Vec<f32> = (0..dimensions)
            .map(|_| {
                let v: f32 = rng.gen();
                if v.is_nan() { 0.0 } else { v }
            })
            .collect();

        // Random entry points
        let mut entry_points = Vec::new();
        for _ in 0..ep_count {
            let idx = rng.gen_range(0..node_count);
            entry_points.push(NodeId(idx as u32));
        }

        // Execute search — must not panic
        let searcher = Searcher::<L2Squared, _>::new(&index, &provider);
        let mut ctx = SearchContext::new();
        let _ = searcher.search_layer(&mut ctx, entry_points.into_iter(), &query, ef, 0);
    }

    /// HNSW insert + search roundtrip: after inserting vectors, search must
    /// return results without panicking and result count must be <= k.
    #[test]
    fn proptest_fuzz_hnsw_insert_search(
        dim in 2u32..=16,
        num_vectors in 1usize..=30,
        k in 1usize..=10,
        seed in any::<u64>(),
    ) {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let mut rng = SmallRng::seed_from_u64(seed);

        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = match HnswIndex::new(config, &storage) {
            Ok(i) => i,
            Err(_) => return Ok(()),
        };

        let mut inserted = 0usize;
        for _ in 0..num_vectors {
            let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0)).collect();
            if index.insert(&vec, &mut storage).is_ok() {
                inserted += 1;
            }
        }

        if inserted == 0 {
            return Ok(());
        }

        let query: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0)).collect();
        let result = index.search(&query, k, &storage);

        match result {
            Ok(results) => {
                prop_assert!(results.len() <= k, "Got more results than k");
                prop_assert!(results.len() <= inserted, "Got more results than inserted");
            }
            Err(_) => {
                // Errors are acceptable, panics are not
            }
        }
    }
}
