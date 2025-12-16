use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::persistence::chunking::ChunkedWriter;
use edgevec::persistence::storage::{MemoryBackend, StorageBackend};
use edgevec::persistence::{read_snapshot, write_snapshot};
use edgevec::storage::VectorStorage;
use proptest::prelude::*;

// Define strategies for generating test data
fn vector_strategy(dim: usize) -> impl Strategy<Value = Vec<f32>> {
    // Exclude NaN/Inf for float generation as comparisons fail
    proptest::collection::vec(proptest::num::f32::NORMAL, dim)
}

#[allow(dead_code)]
fn hnsw_config_strategy() -> impl Strategy<Value = HnswConfig> {
    Just(HnswConfig::new(2)) // Use 2D for simplicity in these tests
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))] // Run 50 iterations

    #[test]
    fn prop_snapshot_roundtrip(
        vectors in proptest::collection::vec(vector_strategy(2), 1..50)
    ) {
        // 1. Setup
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

        // 2. Insert Data
        let mut ids = Vec::new();
        for vec in &vectors {
            let id = storage.insert(vec).unwrap();
            ids.push(id);
            // Add to graph (simple level 0 for now to verify connectivity persistence)
            let node_id = index.add_node(id, 0).unwrap();
            // Just link to previous for connectivity
            if node_id.0 > 0 {
                 let prev_node_id = edgevec::hnsw::graph::NodeId(node_id.0 - 1);
                 index.set_neighbors(prev_node_id, &[node_id]).unwrap();
            }
        }

        // Ensure entry point is set
        if !ids.is_empty() {
             index.set_entry_point(edgevec::hnsw::graph::NodeId(0));
        }

        // 3. Snapshot
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("write failed");

        // 4. Recover
        let (index2, storage2) = read_snapshot(&backend).expect("read failed");

        // 5. Verify Invariants

        // [I1] Vector Count
        prop_assert_eq!(storage2.len(), vectors.len());
        prop_assert_eq!(index2.node_count(), vectors.len());

        // [I2] Vector Data Integrity
        for (i, id) in ids.iter().enumerate() {
            let vec_rec = storage2.get_vector(*id);
            prop_assert_eq!(&vec_rec[..], &vectors[i][..]);
        }

        // [I3] Graph Structure Integrity (Node count & Neighbors)
        // Check random node neighbor count matches
        if ids.len() > 1 {
            let node0 = index.get_node(edgevec::hnsw::graph::NodeId(0)).unwrap();
            let node0_rec = index2.get_node(edgevec::hnsw::graph::NodeId(0)).unwrap();

            let neighbors = index.get_neighbors(node0).unwrap();
            let neighbors_rec = index2.get_neighbors(node0_rec).unwrap();

            prop_assert_eq!(neighbors, neighbors_rec);
        }
    }
}

#[test]
fn test_snapshot_streaming() {
    // 1. Create Data spanning multiple chunks
    // Use small vectors but enough to force splitting with small chunk size
    let dim = 16;
    let config = HnswConfig::new(dim);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    // Insert 500 vectors. 500 * 16 * 4 = 32KB.
    let vec = vec![1.0; dim as usize];
    for _ in 0..500 {
        let id = storage.insert(&vec).unwrap();
        index.add_node(id, 0).unwrap();
    }

    // Use tiny chunk size: 1KB
    let chunk_size = 1024;
    let writer = (&storage, &index);
    let iter = writer.export_chunked(chunk_size);

    let mut chunks = Vec::new();
    let mut raw_data = Vec::new();

    for chunk in iter {
        assert!(
            chunk.len() <= chunk_size,
            "Chunk size exceeded: {} > {}",
            chunk.len(),
            chunk_size
        );
        assert!(!chunk.is_empty(), "Empty chunk yielded");
        chunks.push(chunk.clone());
        raw_data.extend_from_slice(&chunk);
    }

    // Expect multiple chunks (32KB / 1KB ~ 32 chunks)
    assert!(
        chunks.len() > 20,
        "Expected >20 chunks, got {}",
        chunks.len()
    );

    // Verify Integrity via Manual Reassembly & CRC Patching
    // `export_chunked` produces a header with 0 CRC.
    // We must patch it to pass `read_snapshot` verification.

    // 1. Calculate CRC of payload (data after 64-byte header)
    assert!(raw_data.len() > 64);
    let payload = &raw_data[64..];
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(payload);
    let crc = hasher.finalize();

    // 2. Patch Header
    let mut header_bytes = [0u8; 64];
    header_bytes.copy_from_slice(&raw_data[0..64]);
    let mut header = edgevec::persistence::header::FileHeader::from_bytes(&header_bytes).unwrap();
    header.data_crc = crc;
    header.update_checksum();

    // Write back patched header
    raw_data[0..64].copy_from_slice(header.as_bytes());

    // 3. Read back
    struct ReassembledBackend(Vec<u8>);
    impl StorageBackend for ReassembledBackend {
        fn append(&mut self, _data: &[u8]) -> Result<(), edgevec::persistence::PersistenceError> {
            Ok(())
        }
        fn read(&self) -> Result<Vec<u8>, edgevec::persistence::PersistenceError> {
            Ok(self.0.clone())
        }
        fn atomic_write(
            &self,
            _key: &str,
            _data: &[u8],
        ) -> Result<(), edgevec::persistence::PersistenceError> {
            Ok(())
        }
    }

    let backend = ReassembledBackend(raw_data);
    let (index2, storage2) = read_snapshot(&backend).expect("read failed on reassembled chunks");

    assert_eq!(storage2.len(), 500);
    assert_eq!(index2.node_count(), 500);
}

#[test]
fn test_snapshot_corruption() {
    // Manually test corruption scenarios
    let config = HnswConfig::new(2);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

    storage.insert(&[1.0, 2.0]).unwrap();
    index.add_node(edgevec::hnsw::VectorId(1), 0).unwrap();

    let mut backend = MemoryBackend::new();
    write_snapshot(&index, &storage, &mut backend).expect("write failed");

    // Corrupt the data
    let mut data = backend.read().unwrap();
    // Corrupt a byte in the middle (likely payload)
    let len = data.len();
    if len > 100 {
        data[len / 2] ^= 0xFF; // Flip bits

        // Write back corrupted data
        // We need a backend that lets us write raw corrupted data.
        // MemoryBackend append just appends.
        // The snapshot writer recalculates checksums before persisting.
        // We need to bypass the normal snapshot writer to simulate corruption on disk.
        // We can make a specialized test backend or just use a mock.
        // Or we use FileBackend and modify file manually.
        // Since MemoryBackend is simple, let's just make a "CorruptedBackend" that returns bad data.

        struct CorruptedBackend(Vec<u8>);
        impl StorageBackend for CorruptedBackend {
            fn append(
                &mut self,
                _data: &[u8],
            ) -> Result<(), edgevec::persistence::PersistenceError> {
                Ok(())
            }
            fn read(&self) -> Result<Vec<u8>, edgevec::persistence::PersistenceError> {
                Ok(self.0.clone())
            }
            fn atomic_write(
                &self,
                _key: &str,
                _data: &[u8],
            ) -> Result<(), edgevec::persistence::PersistenceError> {
                Ok(())
            }
        }

        let corrupted_backend = CorruptedBackend(data);
        let result = read_snapshot(&corrupted_backend);

        // Expect failure (ChecksumMismatch or Header error)
        assert!(result.is_err());
    }
}
