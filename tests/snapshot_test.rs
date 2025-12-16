#[cfg(test)]
mod tests {
    use edgevec::hnsw::{HnswConfig, HnswIndex};
    use edgevec::persistence::storage::MemoryBackend;
    use edgevec::persistence::{read_snapshot, write_snapshot};
    use edgevec::storage::VectorStorage;

    #[test]
    fn test_snapshot_roundtrip() {
        // 1. Setup
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

        // 2. Add Data
        let v1 = vec![1.0, 1.0];
        let v2 = vec![2.0, 2.0];
        let id1 = storage.insert(&v1).unwrap();
        let id2 = storage.insert(&v2).unwrap();

        let node1 = index.add_node(id1, 0).unwrap();
        let node2 = index.add_node(id2, 1).unwrap(); // Higher layer

        index.set_neighbors(node1, &[node2]).unwrap();
        index.set_entry_point(node2);

        // 3. Delete one vector using RFC-001 soft_delete API
        index.soft_delete(id1).unwrap();

        // 3. Snapshot
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("write failed");

        // 4. Recover
        let (index2, storage2) = read_snapshot(&backend).expect("read failed");

        // 5. Verify
        assert_eq!(storage2.len(), 2);

        // Verify Data
        let v1_rec = storage2.get_vector(id1);
        assert_eq!(&v1_rec[..], &[1.0, 1.0]);

        // Verify Deletion (soft_delete marks in HnswIndex, not storage)
        assert!(index2.is_deleted(id1).unwrap());
        assert!(!index2.is_deleted(id2).unwrap());

        // Verify Graph
        let n1_rec = index2.get_node(node1).unwrap();
        assert_eq!(n1_rec.max_layer, 0);

        let neighbors = index2.get_neighbors(n1_rec).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], node2);

        assert_eq!(index2.entry_point(), Some(node2));
    }
}
