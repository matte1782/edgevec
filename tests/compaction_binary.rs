//! Storage representation and BQ state must survive an HNSW rebuild.
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::quantization::QuantizerConfig;
use edgevec::storage::{StorageType, VectorStorage};

#[test]
fn compaction_preserves_sq8_codes_when_float_decoding_loses_precision() {
    for deleted in [false, true] {
        let config = HnswConfig::new(64);
        let mut storage = VectorStorage::new(&config, None);
        storage.set_storage_type(StorageType::QuantizedU8(QuantizerConfig {
            min: 100_000_000.0,
            max: 100_000_008.0,
        }));
        let mut index = HnswIndex::new(config, &storage).unwrap();
        for code in [0, 128] {
            let id = storage.insert_quantized(&[code; 64]).unwrap();
            let node = index.add_node(id, 0).unwrap();
            if id == VectorId(1) {
                index.set_entry_point(node);
            }
        }
        if deleted {
            index.soft_delete(VectorId(1)).unwrap();
        }
        let (rebuilt_index, rebuilt, _) = index.compact(&storage).unwrap();
        let last_id = if deleted { 1 } else { 2 };
        assert_eq!(rebuilt.get_quantized_vector(VectorId(last_id)), &[128; 64]);
        assert_eq!(rebuilt_index.live_count(), usize::from(!deleted) + 1);
    }
}

#[test]
fn quantized_compaction_preserves_representation_and_calibration() {
    for deleted in [0_u64, 1, 3] {
        let config = HnswConfig::new(64);
        let mut storage = VectorStorage::new(&config, None);
        storage.set_storage_type(StorageType::QuantizedU8(QuantizerConfig {
            min: -5.0,
            max: 10.0,
        }));
        let mut index = HnswIndex::new(config, &storage).unwrap();
        for value in [-5.0, 2.5, 10.0] {
            index.insert(&[value; 64], &mut storage).unwrap();
        }
        for id in 1..=deleted {
            index.soft_delete(VectorId(id)).unwrap();
        }
        let (mut compacted, mut rebuilt, _) = index.compact(&storage).unwrap();
        for id in 1..=3 - deleted {
            assert_eq!(
                rebuilt.get_quantized_vector(VectorId(id)),
                storage.get_quantized_vector(VectorId(id + deleted))
            );
        }
        let next = compacted.insert(&[5.0; 64], &mut rebuilt).unwrap();
        assert_eq!(next, VectorId(4 - deleted));
        assert_eq!(rebuilt.get_quantized_vector(next).len(), 64);
        assert_eq!(
            compacted.search(&[5.0; 64], 1, &rebuilt).unwrap()[0].vector_id,
            next
        );
    }
}

#[test]
fn binary_compaction_handles_live_deleted_and_empty_indexes() {
    for deleted in [0, 1, 3] {
        let mut config = HnswConfig::new(64);
        config.metric = HnswConfig::METRIC_HAMMING;
        let mut storage = VectorStorage::new(&config, None);
        storage.set_storage_type(StorageType::Binary(64));
        let mut index = HnswIndex::new(config, &storage).unwrap();
        let vectors = [[0u8; 8], [0x55u8; 8], [0xFFu8; 8]];
        for vector in &vectors {
            index.insert_binary(vector, &mut storage).unwrap();
        }
        for id in 1..=deleted {
            index.soft_delete(VectorId(id)).unwrap();
        }
        let (mut compacted, mut rebuilt, result) = index.compact(&storage).unwrap();
        assert_eq!(result.tombstones_removed, deleted as usize);
        assert_eq!(compacted.live_count(), 3 - deleted as usize);
        for (offset, vector) in vectors.iter().skip(deleted as usize).enumerate() {
            let hits = compacted.search_binary(vector, 1, &rebuilt).unwrap();
            assert_eq!(hits[0].vector_id, VectorId(offset as u64 + 1));
            assert_eq!(hits[0].distance, 0.0);
        }
        let inserted = compacted.insert_binary(&[0xAA; 8], &mut rebuilt).unwrap();
        assert_eq!(inserted.0, result.new_size as u64 + 1);
        assert_eq!(index.live_count(), 3 - deleted as usize);
    }
}

#[test]
fn compact_rebuilds_enabled_bq_with_and_without_tombstones() {
    for deleted in [0, 1, 3] {
        let config = HnswConfig::new(64);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();
        for positive_chunks in 0..3 {
            let mut vector = [-1.0; 64];
            vector[..positive_chunks * 24].fill(1.0);
            index.insert(&vector, &mut storage).unwrap();
        }
        index.enable_bq(&storage).unwrap();
        for id in 1..=deleted {
            index.soft_delete(VectorId(id)).unwrap();
        }
        let (compacted, rebuilt, _) = index.compact(&storage).unwrap();
        assert!(compacted.has_bq());
        assert_eq!(compacted.bq_storage().unwrap().len(), 3 - deleted as usize);
        let hits = compacted.search_bq(&[1.0; 64], 1, &rebuilt).unwrap();
        if deleted == 3 {
            assert!(hits.is_empty());
        } else {
            assert_eq!(hits[0].0, VectorId(3 - deleted));
        }
    }
}
