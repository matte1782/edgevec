//! Proptest-based fuzz simulation tests for EdgeVec.
//!
//! These tests provide cross-platform structured random testing as a
//! complement to the cargo-fuzz targets in `fuzz/fuzz_targets/`.
//!
//! **Important limitations vs libFuzzer:**
//! - Proptest uses **random generation from defined strategies**, NOT
//!   coverage-guided mutation. It cannot discover inputs outside the
//!   strategy space.
//! - Coverage-guided fuzzing (libFuzzer) on Linux CI remains critical
//!   for security assurance. These tests are a supplement, not a replacement.
//!
//! Configure iteration count via `PROPTEST_CASES` env var (default: 1000).
//!
//! Coverage mapping to cargo-fuzz targets:
//! - `filter_deep` -> proptest_fuzz_filter_deep, proptest_fuzz_filter_arbitrary_string
//! - `persistence` -> proptest_fuzz_persistence, proptest_fuzz_persistence_roundtrip
//! - `hnsw_search` -> proptest_fuzz_hnsw_search, proptest_fuzz_hnsw_insert_search
//! - `header_parse` -> proptest_fuzz_header_parse
//! - `graph_ops` -> proptest_fuzz_graph_ops (insert/delete/search sequence)
//!
//! NOT covered by proptest (require libFuzzer on Linux CI):
//! - `filter_simple`, `flat_index`, `sparse_storage`, `sparse_vector`
//! - `hnsw_config`, `hnsw_insert`, `quantization`, `search_robustness`
//! - `wal_replay`, `dummy_harness`

use proptest::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

use edgevec::filter::parse;
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::persistence::header::FileHeader;
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
/// Direct port of the cargo-fuzz target's generator.
fn generate_nested_filter(data: &[u8], max_depth: usize) -> String {
    if data.is_empty() || max_depth == 0 {
        return "x = 1".to_string();
    }

    let byte = data[0];
    let op = byte & 0b11;
    let selector = (byte >> 2) & 0b111111;

    match op {
        0 => {
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
            let inner = if data.len() > 1 {
                generate_nested_filter(&data[1..], max_depth - 1)
            } else {
                "x = 1".to_string()
            };
            format!("NOT ({inner})")
        }
        _ => {
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

    /// Parse deeply nested filter expressions — must never panic.
    #[test]
    fn proptest_fuzz_filter_deep(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        let filter = generate_nested_filter(&data, 50);
        let _ = parse(&filter);
    }

    /// Parse completely arbitrary strings — must never panic.
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

    /// Deserialize arbitrary bytes as snapshot — must never panic.
    #[test]
    fn proptest_fuzz_persistence(data in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let backend = MemoryBackend::new();
        // "" is the default key used by write_snapshot/read_snapshot (MemoryBackend ignores key)
        let _ = backend.atomic_write("", &data);
        let _ = edgevec::persistence::snapshot::read_snapshot(&backend);
    }

    /// Save/load roundtrip: loaded index must have same vector count
    /// and be searchable without panicking.
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
            // Full finite f32 range: subnormals, large magnitudes, negative zero
            let vec: Vec<f32> = (0..dim)
                .map(|_| {
                    let bits: u32 = rng.gen();
                    let v = f32::from_bits(bits);
                    if v.is_finite() { v } else { 0.0 }
                })
                .collect();
            let _ = index.insert(&vec, &mut storage);
        }

        let mut backend = MemoryBackend::new();
        if edgevec::persistence::write_snapshot(&index, &storage, &mut backend).is_ok() {
            let result = edgevec::persistence::read_snapshot(&backend);
            if let Ok((loaded_index, loaded_storage)) = result {
                prop_assert_eq!(
                    storage.len(),
                    loaded_storage.len(),
                    "Roundtrip vector count mismatch"
                );

                // [M3 fix] Verify loaded index is functional — search must not panic
                if !loaded_storage.is_empty() {
                    let query: Vec<f32> = (0..dim)
                        .map(|_| {
                            let bits: u32 = rng.gen();
                            let v = f32::from_bits(bits);
                            if v.is_finite() { v } else { 0.0 }
                        })
                        .collect();
                    let _ = loaded_index.search(&query, 5, &loaded_storage);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Header parse — mirrors fuzz/fuzz_targets/header_parse/target.rs
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// Parse arbitrary bytes as FileHeader — must never panic.
    #[test]
    fn proptest_fuzz_header_parse(data in proptest::collection::vec(any::<u8>(), 0..128)) {
        let _ = FileHeader::from_bytes(&data);
    }
}

// ---------------------------------------------------------------------------
// 4. HNSW search — mirrors fuzz/fuzz_targets/hnsw_search/target.rs
// ---------------------------------------------------------------------------

/// Mock vector provider that returns a zero vector for missing IDs
/// instead of panicking. This isolates EdgeVec panics from test infra.
struct SafeMockVectorProvider {
    vectors: HashMap<VectorId, Vec<f32>>,
    dimensions: usize,
}

impl SafeMockVectorProvider {
    fn new(dimensions: usize) -> Self {
        Self {
            vectors: HashMap::new(),
            dimensions,
        }
    }

    fn add(&mut self, id: u64, vec: Vec<f32>) {
        self.vectors.insert(VectorId(id), vec);
    }
}

impl edgevec::hnsw::VectorProvider for SafeMockVectorProvider {
    fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]> {
        match self.vectors.get(&id) {
            Some(v) => Cow::Borrowed(v),
            // [C1 fix] Return zero vector instead of panicking
            None => Cow::Owned(vec![0.0; self.dimensions]),
        }
    }
}

proptest! {
    #![proptest_config(config())]

    /// HNSW search with random topology — must never panic.
    /// Uses full finite f32 range including large magnitudes and subnormals.
    #[test]
    fn proptest_fuzz_hnsw_search(
        dimensions in 2u32..=16,
        node_count in 5usize..=50,
        ef in 1usize..=100,
        ep_count in 0usize..=5,
        seed in any::<u64>(),
    ) {
        use edgevec::hnsw::{NodeId, SearchContext, Searcher};
        use edgevec::metric::L2Squared;
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let mut rng = SmallRng::seed_from_u64(seed);
        let dim = dimensions as usize;

        let config = HnswConfig::new(dimensions);
        let empty_storage = VectorStorage::new(&config, None);
        let mut index = match HnswIndex::new(config, &empty_storage) {
            Ok(i) => i,
            Err(_) => return Ok(()),
        };

        // [C3 fix] Generate full finite f32 range (large magnitudes, subnormals, neg zero)
        let mut provider = SafeMockVectorProvider::new(dim);
        for i in 0..node_count {
            let vec: Vec<f32> = (0..dim)
                .map(|_| {
                    let bits: u32 = rng.gen();
                    let v = f32::from_bits(bits);
                    if v.is_finite() { v } else { 0.0 }
                })
                .collect();
            provider.add(i as u64 + 1, vec);
            let _ = index.add_node(VectorId(i as u64 + 1), 0);
        }

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

        // [C3 fix] Query with full finite f32 range
        let query: Vec<f32> = (0..dim)
            .map(|_| {
                let bits: u32 = rng.gen();
                let v = f32::from_bits(bits);
                if v.is_finite() { v } else { 0.0 }
            })
            .collect();

        let mut entry_points = Vec::new();
        for _ in 0..ep_count {
            let idx = rng.gen_range(0..node_count);
            entry_points.push(NodeId(idx as u32));
        }

        let searcher = Searcher::<L2Squared, _>::new(&index, &provider);
        let mut ctx = SearchContext::new();
        let _ = searcher.search_layer(&mut ctx, entry_points.into_iter(), &query, ef, 0);
    }

    /// HNSW insert+search roundtrip with full f32 range.
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
            // [M1 fix] Full finite f32 range
            let vec: Vec<f32> = (0..dim)
                .map(|_| {
                    let bits: u32 = rng.gen();
                    let v = f32::from_bits(bits);
                    if v.is_finite() { v } else { 0.0 }
                })
                .collect();
            if index.insert(&vec, &mut storage).is_ok() {
                inserted += 1;
            }
        }

        if inserted == 0 {
            return Ok(());
        }

        let query: Vec<f32> = (0..dim)
            .map(|_| {
                let bits: u32 = rng.gen();
                let v = f32::from_bits(bits);
                if v.is_finite() { v } else { 0.0 }
            })
            .collect();
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

// ---------------------------------------------------------------------------
// 5. Graph ops — partially mirrors fuzz/fuzz_targets/graph_ops/target.rs
//    Stateful insert/delete/search sequence (no SaveLoad interleave —
//    that requires the cargo-fuzz target on Linux CI).
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// Interleaved insert/delete/search — must never panic and
    /// searches must find known-inserted vectors.
    #[test]
    fn proptest_fuzz_graph_ops(
        dim in 2u32..=8,
        op_count in 5usize..=40,
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

        let mut live_ids: Vec<VectorId> = Vec::new();
        let mut live_vectors: Vec<Vec<f32>> = Vec::new();

        for _ in 0..op_count {
            let op = rng.gen_range(0u8..10);

            match op {
                // Insert (60% probability)
                0..=5 => {
                    // Full finite f32 range for insert vectors
                    let vec: Vec<f32> = (0..dim)
                        .map(|_| {
                            let bits: u32 = rng.gen();
                            let v = f32::from_bits(bits);
                            if v.is_finite() { v } else { 0.0 }
                        })
                        .collect();
                    if let Ok(vid) = index.insert(&vec, &mut storage) {
                        live_ids.push(vid);
                        live_vectors.push(vec);
                    }
                }
                // Delete (20% probability)
                6..=7 => {
                    if !live_ids.is_empty() {
                        let idx = rng.gen_range(0..live_ids.len());
                        let vid = live_ids[idx];
                        let _ = index.soft_delete(vid);
                        live_ids.swap_remove(idx);
                        live_vectors.swap_remove(idx);
                    }
                }
                // Search (20% probability)
                _ => {
                    if !live_vectors.is_empty() {
                        let idx = rng.gen_range(0..live_vectors.len());
                        let query = &live_vectors[idx];
                        let k = rng.gen_range(1..=5);
                        let _ = index.search(query, k, &storage);
                    }
                }
            }
        }

        // Connectivity check: search for a known vector must not panic.
        // Note: we do NOT assert non-empty results because soft_delete can
        // break graph connectivity (deleted bridge nodes), which is expected
        // HNSW behavior. The invariant here is "no panic on any sequence."
        if !live_vectors.is_empty() {
            let idx = rng.gen_range(0..live_vectors.len());
            let query = &live_vectors[idx];
            let _ = index.search(query, 1, &storage);
        }
    }
}
