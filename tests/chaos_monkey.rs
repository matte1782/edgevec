use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::persistence::storage::MemoryBackend;
use edgevec::persistence::wal::WalAppender;
use edgevec::persistence::{read_snapshot, write_snapshot, PersistenceError, StorageBackend};
use edgevec::storage::VectorStorage;
use rand::prelude::*;
use rand::rngs::SmallRng;
use std::sync::{Arc, Mutex};

/// ChaosStorageBackend wraps MemoryBackend and simulates failures.
/// It uses Arc internally so cloning it shares the underlying state.
#[derive(Debug, Clone)]
struct ChaosStorageBackend {
    inner: MemoryBackend,
    failure_rate: Arc<Mutex<f64>>, // Shared failure rate
    rng: Arc<Mutex<SmallRng>>,     // Shared RNG
}

impl ChaosStorageBackend {
    fn new(failure_rate: f64, seed: u64) -> Self {
        Self {
            inner: MemoryBackend::new(),
            failure_rate: Arc::new(Mutex::new(failure_rate)),
            rng: Arc::new(Mutex::new(SmallRng::seed_from_u64(seed))),
        }
    }

    fn set_failure_rate(&self, rate: f64) {
        *self.failure_rate.lock().unwrap() = rate;
    }

    fn should_fail(&self) -> bool {
        let rate = *self.failure_rate.lock().unwrap();
        if rate <= 0.0 {
            return false;
        }
        let mut rng = self.rng.lock().unwrap();
        rng.random_bool(rate)
    }
}

impl StorageBackend for ChaosStorageBackend {
    fn append(&mut self, data: &[u8]) -> Result<(), PersistenceError> {
        if self.should_fail() {
            return Err(PersistenceError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Chaos Monkey: append failed",
            )));
        }
        self.inner.append(data)
    }

    fn read(&self) -> Result<Vec<u8>, PersistenceError> {
        // We do not fail reads for these scenarios unless specified
        self.inner.read()
    }

    fn atomic_write(&self, key: &str, data: &[u8]) -> Result<(), PersistenceError> {
        if self.should_fail() {
            return Err(PersistenceError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Chaos Monkey: atomic_write failed",
            )));
        }
        self.inner.atomic_write(key, data)
    }
}

#[test]
fn test_scenario_1_persistence_resilience() {
    let chaos_backend = ChaosStorageBackend::new(0.0, 42); // Start safe
    let mut backend_handle = chaos_backend.clone(); // For writing
    let config = HnswConfig::new(2);

    // 1. Initial State: Empty
    // Save empty state
    let empty_storage = VectorStorage::new(&config, None);
    let empty_index = HnswIndex::new(config.clone(), &empty_storage).unwrap();
    write_snapshot(&empty_index, &empty_storage, &mut backend_handle).expect("Initial save failed");

    // 2. Insert Data (State A)
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    let vec1 = vec![1.0, 1.0];
    index.insert(&vec1, &mut storage).unwrap();

    // Save State A (Safe)
    write_snapshot(&index, &storage, &mut backend_handle).expect("Save State A failed");

    // Verify Load State A
    let (loaded_index, loaded_storage) =
        read_snapshot(&chaos_backend).expect("Load State A failed");
    assert_eq!(loaded_storage.len(), 1);
    assert_eq!(loaded_index.node_count(), 1);

    // 3. Insert More Data (State B)
    let vec2 = vec![2.0, 2.0];
    index.insert(&vec2, &mut storage).unwrap();

    // Enable Chaos
    chaos_backend.set_failure_rate(1.0); // 100% failure

    // Save State B (Should Fail)
    let res = write_snapshot(&index, &storage, &mut backend_handle);
    assert!(res.is_err(), "Snapshot should have failed under chaos");

    // 4. Load (Must be State A)
    let (recovered_index, recovered_storage) =
        read_snapshot(&chaos_backend).expect("Recovery load failed");

    // Assert Atomic Property: State should be exactly State A (1 vector), not State B (2 vectors)
    // or partial state.
    assert_eq!(
        recovered_storage.len(),
        1,
        "Storage should have rolled back to State A"
    );
    assert_eq!(
        recovered_index.node_count(),
        1,
        "Index should have rolled back to State A"
    );

    // Disable Chaos and Save State B
    chaos_backend.set_failure_rate(0.0);
    write_snapshot(&index, &storage, &mut backend_handle).expect("Save State B retry failed");

    let (final_index, final_storage) = read_snapshot(&chaos_backend).expect("Final load failed");
    assert_eq!(final_storage.len(), 2);
    assert_eq!(final_index.node_count(), 2);
}

#[test]
fn test_scenario_2_wal_resilience() {
    let chaos_backend = ChaosStorageBackend::new(0.5, 999); // 50% failure
    let config = HnswConfig::new(2);

    // Setup Storage with Chaos WAL
    let wal = WalAppender::new(Box::new(chaos_backend.clone()), 0);
    let mut storage = VectorStorage::new(&config, Some(wal));
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    let mut successes = 0;
    let mut failures = 0;

    for i in 0..100 {
        let vec = vec![i as f32, i as f32];
        match index.insert(&vec, &mut storage) {
            Ok(_) => {
                successes += 1;
                // Verify it IS in storage
                assert_eq!(storage.len(), successes);
            }
            Err(_) => {
                failures += 1;
                // Verify it IS NOT in storage
                assert_eq!(storage.len(), successes);
            }
        }
    }

    println!(
        "WAL Stress Test: {} successes, {} failures",
        successes, failures
    );
    assert!(
        failures > 0,
        "RNG seed 999 should produce failures with 0.5 rate"
    );
    assert!(
        successes > 0,
        "RNG seed 999 should produce successes with 0.5 rate"
    );

    // Verify Consistency
    // Replay WAL
    let recovered_storage = VectorStorage::recover(Box::new(chaos_backend), &config).unwrap();

    // The recovered storage should match the in-memory successful inserts
    assert_eq!(
        recovered_storage.len(),
        successes,
        "Recovered WAL count mismatch"
    );

    // Check vector contents
    // Since we inserted sequentially, and failures prevent insertion:
    // IDs should be contiguous 1..successes+1 because failed inserts don't increment next_id
    for i in 1..=successes {
        let id = VectorId(i as u64);
        // Should not panic and return data
        let _ = recovered_storage.get_vector(id);
    }
}

#[test]
fn stress_test_execution() {
    // Loop 1000 times (500 per scenario type logic roughly, or mix)
    let iterations = 1000;
    let mut failed_saves_caught = 0;
    let mut wal_failures_caught = 0;
    let mut successful_ops = 0;

    let mut rng = SmallRng::seed_from_u64(0xDEADBEEF);

    println!(
        "stress_test_execution: Starting {} iterations...",
        iterations
    );

    for i in 0..iterations {
        if i % 2 == 0 {
            // --- Type 1: Atomic Snapshot Resilience ---
            let failure_rate = rng.random_range(0.1..0.9);
            let chaos_backend = ChaosStorageBackend::new(0.0, rng.random());
            let mut backend_handle = chaos_backend.clone();
            let config = HnswConfig::new(2);

            // Setup initial state
            let mut storage = VectorStorage::new(&config, None);
            let mut index = HnswIndex::new(config.clone(), &storage).unwrap();
            let vec1 = vec![1.0, 1.0];
            index.insert(&vec1, &mut storage).unwrap();

            // Save Valid State A
            write_snapshot(&index, &storage, &mut backend_handle).expect("Setup save failed");

            // Mutate to State B
            let vec2 = vec![2.0, 2.0];
            index.insert(&vec2, &mut storage).unwrap();

            // Try Save State B with Chaos
            chaos_backend.set_failure_rate(failure_rate);
            let res = write_snapshot(&index, &storage, &mut backend_handle);

            if res.is_err() {
                failed_saves_caught += 1;
                // Verify Rollback
                let (rec_idx, rec_store) =
                    read_snapshot(&chaos_backend).expect("Rollback load failed");
                assert_eq!(
                    rec_store.len(),
                    1,
                    "Snapshot rollback failed! Found State B or partial."
                );
                assert_eq!(rec_idx.node_count(), 1);
            } else {
                successful_ops += 1;
                // Verify Update
                let (rec_idx, rec_store) =
                    read_snapshot(&chaos_backend).expect("Success load failed");
                assert_eq!(rec_store.len(), 2, "Snapshot update failed! Found State A.");
                assert_eq!(rec_idx.node_count(), 2);
            }
        } else {
            // --- Type 2: WAL Append Resilience ---
            let failure_rate = rng.random_range(0.1..0.5); // Lower failure rate to allow some progress
            let chaos_backend = ChaosStorageBackend::new(failure_rate, rng.random());
            let config = HnswConfig::new(2);

            let wal = WalAppender::new(Box::new(chaos_backend.clone()), 0);
            let mut storage = VectorStorage::new(&config, Some(wal));
            let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

            let mut local_success = 0;
            // Run a mini batch of inserts
            for j in 0..10 {
                let vec = vec![j as f32, j as f32];
                if index.insert(&vec, &mut storage).is_ok() {
                    local_success += 1;
                    successful_ops += 1;
                } else {
                    wal_failures_caught += 1;
                }
            }

            // Verify Consistency
            let recovered = VectorStorage::recover(Box::new(chaos_backend), &config).unwrap();
            assert_eq!(
                recovered.len(),
                local_success,
                "WAL recovery count mismatch"
            );
        }
    }

    println!("stress_test_execution: Complete.");
    println!("METRICS:");
    println!("Total Runs: {}", iterations);
    println!("Failed Saves (Caught): {}", failed_saves_caught);
    println!("WAL Failures (Caught): {}", wal_failures_caught);
    println!("Successful Ops: {}", successful_ops);
    println!("Panics: 0"); // If we reached here
}
