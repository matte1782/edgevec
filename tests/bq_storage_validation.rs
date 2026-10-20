//! Invalid BQ storage must fail before mutation, never panic on packed bits.
use edgevec::hnsw::{GraphError, HnswConfig, HnswIndex};
use edgevec::quantization::QuantizerConfig;
use edgevec::storage::{StorageType, VectorStorage};

#[test]
fn bq_rejects_empty_and_populated_binary_storage_without_mutation() {
    for count in [0, 1, 3] {
        let mut config = HnswConfig::new(64);
        config.metric = HnswConfig::METRIC_HAMMING;
        let mut storage = VectorStorage::new(&config, None);
        storage.set_storage_type(StorageType::Binary(64));
        let mut index = HnswIndex::new(config.clone(), &storage).unwrap();
        for vector in [[0u8; 8], [0x55; 8], [0xff; 8]].iter().take(count) {
            index.insert_binary(vector, &mut storage).unwrap();
        }
        assert!(matches!(
            HnswIndex::with_bq(config, &storage),
            Err(GraphError::InvalidConfig(message)) if message.contains("packed binary storage")
        ));
        assert!(matches!(
            index.enable_bq(&storage),
            Err(GraphError::InvalidConfig(message)) if message.contains("packed binary storage")
        ));
        assert!(!index.has_bq());
        assert_eq!(index.live_count(), count);
        let id = index.insert_binary(&[0xaa; 8], &mut storage).unwrap();
        assert_eq!(index.live_count(), count + 1);
        let hits = index.search_binary(&[0xaa; 8], 1, &storage).unwrap();
        assert_eq!(hits[0].vector_id, id);
        assert_eq!(hits[0].distance, 0.0);
    }
}

#[test]
fn bq_accepts_float_and_sq8_storage() {
    for kind in [
        StorageType::Float32,
        StorageType::QuantizedU8(QuantizerConfig {
            min: -1.0,
            max: 1.0,
        }),
    ] {
        let config = HnswConfig::new(64);
        let mut storage = VectorStorage::new(&config, None);
        storage.set_storage_type(kind);
        let mut index = HnswIndex::with_bq(config, &storage).unwrap();
        let id = index.insert_bq(&[1.0; 64], &mut storage).unwrap();
        index.enable_bq(&storage).unwrap();
        assert!(index.has_bq());
        assert_eq!(index.search_bq(&[1.0; 64], 1, &storage).unwrap()[0].0, id);
    }
}
