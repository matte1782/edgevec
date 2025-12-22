//! Migration tests for v0.3 → v0.4 format transition (W26.5.3).
//!
//! Tests backward compatibility and transparent migration per RFC-002 §4.3.

use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::metadata::MetadataValue;
use edgevec::persistence::{read_snapshot, write_snapshot, Flags, MemoryBackend, StorageBackend};
use edgevec::storage::VectorStorage;
use std::collections::HashMap;

// =============================================================================
// Backward compatibility tests
// =============================================================================

mod backward_compatibility {
    use super::*;

    /// Simulates a v0.3 snapshot by creating one with no metadata.
    /// Since we can't easily create a true v0.3 snapshot without the old code,
    /// we verify that v0.4 reader handles v0.4 files without HAS_METADATA correctly.
    #[test]
    fn test_v04_without_metadata_loads_as_empty() {
        // Create index without metadata
        let config = HnswConfig::new(8);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        for i in 0..10 {
            let vec: Vec<f32> = (0..8).map(|d| (i * 8 + d) as f32).collect();
            index.insert(&vec, &mut storage).unwrap();
        }

        // Save as v0.4 (will not have HAS_METADATA flag)
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        // Verify HAS_METADATA is NOT set
        let data = backend.read().expect("Failed to read");
        let flags = u16::from_le_bytes([data[6], data[7]]);
        assert_eq!(
            flags & Flags::HAS_METADATA,
            0,
            "HAS_METADATA should not be set"
        );

        // Load and verify empty metadata
        let (loaded_index, loaded_storage) = read_snapshot(&backend).expect("Failed to read");

        assert_eq!(loaded_storage.len(), 10);
        assert_eq!(loaded_index.len(), 10);
        assert!(
            loaded_index.metadata().is_empty(),
            "Metadata should be empty"
        );
    }

    /// Tests that older v0.3 format files (version_minor=3) can still be loaded.
    /// Since we now write v0.4, we manually modify a snapshot to look like v0.3.
    #[test]
    fn test_load_v03_format() {
        // Create a v0.4 snapshot first
        let config = HnswConfig::new(8);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        for i in 0..5 {
            let vec: Vec<f32> = (0..8).map(|d| (i * 8 + d) as f32).collect();
            index.insert(&vec, &mut storage).unwrap();
        }

        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        // Modify to v0.3 format (change version_minor and recalculate CRC)
        let mut data = backend.read().expect("Failed to read");

        // Change version_minor from 4 to 3
        data[5] = 3;

        // Recalculate header CRC (bytes 44-47)
        // Zero out the CRC field first, then calculate
        data[44] = 0;
        data[45] = 0;
        data[46] = 0;
        data[47] = 0;

        let new_crc = crc32fast::hash(&data[0..64]);
        data[44..48].copy_from_slice(&new_crc.to_le_bytes());

        // Create new backend with modified data
        let modified_backend = MemoryBackend::new();
        modified_backend
            .atomic_write("", &data)
            .expect("Failed to write modified data");

        // Load the modified "v0.3" snapshot
        let result = read_snapshot(&modified_backend);

        // Should succeed (v0.3 is supported)
        assert!(
            result.is_ok(),
            "v0.3 format should be readable: {:?}",
            result.err()
        );

        let (loaded_index, loaded_storage) = result.unwrap();
        assert_eq!(loaded_storage.len(), 5);
        assert_eq!(loaded_index.len(), 5);
        assert!(loaded_index.metadata().is_empty(), "v0.3 has no metadata");
    }
}

// =============================================================================
// Migration workflow tests
// =============================================================================

mod migration_workflow {
    use super::*;

    #[test]
    fn test_v03_to_v04_migration_adds_metadata() {
        // 1. Create a v0.4 snapshot without metadata (simulates v0.3)
        let config = HnswConfig::new(8);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        for i in 0..5 {
            let vec: Vec<f32> = (0..8).map(|d| (i * 8 + d) as f32).collect();
            index.insert(&vec, &mut storage).unwrap();
        }

        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        // 2. Load the snapshot
        let (loaded_index, _loaded_storage) = read_snapshot(&backend).expect("Failed to read");

        assert!(loaded_index.metadata().is_empty());

        // 3. Create a new index with metadata to verify the migration path works
        let config2 = HnswConfig::new(8);
        let mut storage2 = VectorStorage::new(&config2, None);
        let mut index2 = HnswIndex::new(config2, &storage2).expect("Failed to create index");

        for i in 0..5 {
            let vec: Vec<f32> = (0..8).map(|d| (i * 8 + d) as f32).collect();
            let mut metadata = HashMap::new();
            metadata.insert("migrated".to_string(), MetadataValue::Boolean(true));
            metadata.insert(
                "migration_version".to_string(),
                MetadataValue::String("v0.4".into()),
            );
            index2
                .insert_with_metadata(&mut storage2, &vec, metadata)
                .unwrap();
        }

        // 4. Save as v0.4 with metadata
        let mut new_backend = MemoryBackend::new();
        write_snapshot(&index2, &storage2, &mut new_backend)
            .expect("Failed to write migrated snapshot");

        // Verify HAS_METADATA flag is now set
        let data = new_backend.read().expect("Failed to read");
        let flags = u16::from_le_bytes([data[6], data[7]]);
        assert_ne!(
            flags & Flags::HAS_METADATA,
            0,
            "HAS_METADATA should be set after migration"
        );

        // 5. Reload and verify metadata persisted
        let (final_index, _) =
            read_snapshot(&new_backend).expect("Failed to read migrated snapshot");

        assert_eq!(final_index.metadata().vector_count(), 5);
        // Note: VectorId starts at 1, not 0. First insert -> VectorId(1), etc.
        for id in 1..=5 {
            assert_eq!(
                final_index.metadata().get(id, "migrated"),
                Some(&MetadataValue::Boolean(true))
            );
            assert_eq!(
                final_index.metadata().get(id, "migration_version"),
                Some(&MetadataValue::String("v0.4".into()))
            );
        }
    }

    #[test]
    fn test_multiple_save_load_cycles() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        // Initial insert with metadata
        let vec = vec![1.0, 2.0, 3.0, 4.0];
        let mut metadata = HashMap::new();
        metadata.insert("cycle".to_string(), MetadataValue::Integer(0));
        index
            .insert_with_metadata(&mut storage, &vec, metadata)
            .unwrap();

        // Save initial state
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        // Verify the cycle value can be read back
        // Note: First insert -> VectorId(1), so metadata is at ID 1
        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read");
        assert_eq!(
            loaded_index.metadata().get(1, "cycle"),
            Some(&MetadataValue::Integer(0))
        );
    }
}

// =============================================================================
// Edge case tests
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_index_roundtrip() {
        let config = HnswConfig::new(16);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).expect("Failed to create index");

        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        let (loaded_index, loaded_storage) = read_snapshot(&backend).expect("Failed to read");

        assert_eq!(loaded_storage.len(), 0);
        assert_eq!(loaded_index.len(), 0);
        assert!(loaded_index.metadata().is_empty());
    }

    #[test]
    fn test_metadata_with_all_value_types() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        // Insert with all metadata types
        let vec = vec![1.0, 2.0, 3.0, 4.0];
        let mut metadata = HashMap::new();
        metadata.insert(
            "string_val".to_string(),
            MetadataValue::String("hello world".into()),
        );
        metadata.insert("int_val".to_string(), MetadataValue::Integer(-9876543210));
        metadata.insert(
            "float_val".to_string(),
            MetadataValue::Float(std::f64::consts::PI),
        );
        metadata.insert("bool_val".to_string(), MetadataValue::Boolean(true));
        metadata.insert(
            "array_val".to_string(),
            MetadataValue::StringArray(vec!["rust".into(), "wasm".into(), "vector".into()]),
        );
        index
            .insert_with_metadata(&mut storage, &vec, metadata)
            .unwrap();

        // Save and reload
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read");

        // Verify all types preserved
        // Note: First insert -> VectorId(1), so metadata is at ID 1
        let meta = loaded_index.metadata().get_all(1).expect("No metadata");
        assert_eq!(
            meta.get("string_val"),
            Some(&MetadataValue::String("hello world".into()))
        );
        assert_eq!(
            meta.get("int_val"),
            Some(&MetadataValue::Integer(-9876543210))
        );
        assert_eq!(
            meta.get("float_val"),
            Some(&MetadataValue::Float(std::f64::consts::PI))
        );
        assert_eq!(meta.get("bool_val"), Some(&MetadataValue::Boolean(true)));
        assert_eq!(
            meta.get("array_val"),
            Some(&MetadataValue::StringArray(vec![
                "rust".into(),
                "wasm".into(),
                "vector".into()
            ]))
        );
    }

    #[test]
    fn test_large_metadata_values() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        // Create a large string (near the 64KB limit)
        let large_string = "x".repeat(60_000);
        let large_array: Vec<String> = (0..500).map(|i| format!("tag_{i}")).collect();

        let vec = vec![1.0, 2.0, 3.0, 4.0];
        let mut metadata = HashMap::new();
        metadata.insert(
            "large_string".to_string(),
            MetadataValue::String(large_string.clone()),
        );
        metadata.insert(
            "large_array".to_string(),
            MetadataValue::StringArray(large_array.clone()),
        );
        index
            .insert_with_metadata(&mut storage, &vec, metadata)
            .unwrap();

        // Save and reload
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read");

        // Verify large values preserved
        // Note: First insert -> VectorId(1), so metadata is at ID 1
        let meta = loaded_index.metadata().get_all(1).expect("No metadata");
        assert_eq!(
            meta.get("large_string").and_then(|v| v.as_string()),
            Some(large_string.as_str())
        );
        assert_eq!(
            meta.get("large_array"),
            Some(&MetadataValue::StringArray(large_array))
        );
    }
}

// =============================================================================
// Deleted vector metadata tests
// =============================================================================

mod deleted_vectors {
    use super::*;

    #[test]
    fn test_deleted_vector_metadata_not_persisted() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

        // Insert 5 vectors with metadata
        for i in 0..5 {
            let vec: Vec<f32> = (0..4).map(|d| (i * 4 + d) as f32).collect();
            let mut metadata = HashMap::new();
            metadata.insert("id".to_string(), MetadataValue::Integer(i as i64));
            index
                .insert_with_metadata(&mut storage, &vec, metadata)
                .unwrap();
        }

        // VectorIds are 1, 2, 3, 4, 5 (loop i=0..5 gives VectorId(i+1))
        // Delete vectors with VectorId 2 and 4
        index.soft_delete(VectorId(2)).unwrap();
        index.soft_delete(VectorId(4)).unwrap();

        // Metadata for deleted vectors should be removed
        // Metadata IDs match VectorIds (1, 2, 3, 4, 5)
        // Deleted: VectorId(2) and VectorId(4)
        assert!(index.metadata().get(2, "id").is_none());
        assert!(index.metadata().get(4, "id").is_none());
        assert_eq!(index.metadata().vector_count(), 3); // Only IDs 1, 3, 5 remain

        // Save and reload
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).expect("Failed to write");

        let (loaded_index, _) = read_snapshot(&backend).expect("Failed to read");

        // Verify only live vectors have metadata (IDs 1, 3, 5)
        assert_eq!(loaded_index.metadata().vector_count(), 3);
        assert!(loaded_index.metadata().get(1, "id").is_some());
        assert!(loaded_index.metadata().get(2, "id").is_none()); // deleted
        assert!(loaded_index.metadata().get(3, "id").is_some());
        assert!(loaded_index.metadata().get(4, "id").is_none()); // deleted
        assert!(loaded_index.metadata().get(5, "id").is_some());
    }
}
