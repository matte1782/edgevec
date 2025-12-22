//! Integration tests for MetadataStore serialization (W26.4.2).
//!
//! Tests Postcard and JSON serialization with CRC32 validation per RFC-002.

use edgevec::metadata::{MetadataStore, MetadataValue, SerializationError};

// =============================================================================
// Postcard round-trip tests
// =============================================================================

mod postcard_roundtrip {
    use super::*;

    #[test]
    fn test_empty_store_roundtrip() {
        let store = MetadataStore::new();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert!(restored.is_empty());
        assert_eq!(store, restored);
    }

    #[test]
    fn test_single_vector_roundtrip() {
        let mut store = MetadataStore::new();
        store
            .insert(1, "key", MetadataValue::String("value".into()))
            .unwrap();
        store
            .insert(1, "count", MetadataValue::Integer(42))
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(store.get(1, "key"), restored.get(1, "key"));
        assert_eq!(store.get(1, "count"), restored.get(1, "count"));
        assert_eq!(store, restored);
    }

    #[test]
    fn test_multiple_vectors_roundtrip() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "name", MetadataValue::String("first".into()))
            .unwrap();
        store
            .insert(1, "name", MetadataValue::String("second".into()))
            .unwrap();
        store
            .insert(2, "name", MetadataValue::String("third".into()))
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(restored.vector_count(), 3);
        assert_eq!(store, restored);
    }

    #[test]
    fn test_all_value_types_roundtrip() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "string_val", MetadataValue::String("hello world".into()))
            .unwrap();
        store
            .insert(0, "int_val", MetadataValue::Integer(-9876543210))
            .unwrap();
        store
            .insert(0, "float_val", MetadataValue::Float(std::f64::consts::PI))
            .unwrap();
        store
            .insert(0, "bool_true", MetadataValue::Boolean(true))
            .unwrap();
        store
            .insert(0, "bool_false", MetadataValue::Boolean(false))
            .unwrap();
        store
            .insert(
                0,
                "array_val",
                MetadataValue::StringArray(vec![
                    "rust".into(),
                    "wasm".into(),
                    "vector".into(),
                    "search".into(),
                ]),
            )
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(store.key_count(0), 6);
        assert_eq!(store, restored);

        // Verify specific values
        assert_eq!(
            restored.get(0, "string_val").unwrap().as_string().unwrap(),
            "hello world"
        );
        assert_eq!(
            restored.get(0, "int_val").unwrap().as_integer().unwrap(),
            -9876543210
        );
    }

    #[test]
    fn test_large_store_roundtrip() {
        let mut store = MetadataStore::new();

        // Create 50 vectors with 5 keys each
        for v in 0..50u32 {
            store
                .insert(v, "id", MetadataValue::Integer(i64::from(v)))
                .unwrap();
            store
                .insert(v, "name", MetadataValue::String(format!("vector_{v}")))
                .unwrap();
            store
                .insert(v, "score", MetadataValue::Float(f64::from(v) * 0.1))
                .unwrap();
            store
                .insert(v, "active", MetadataValue::Boolean(v % 2 == 0))
                .unwrap();
            store
                .insert(
                    v,
                    "tags",
                    MetadataValue::StringArray(vec![format!("tag_{v}")]),
                )
                .unwrap();
        }

        assert_eq!(store.vector_count(), 50);
        assert_eq!(store.total_key_count(), 250);

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(restored.vector_count(), 50);
        assert_eq!(restored.total_key_count(), 250);
        assert_eq!(store, restored);
    }
}

// =============================================================================
// CRC32 validation tests
// =============================================================================

mod crc_validation {
    use super::*;

    #[test]
    fn test_crc_roundtrip() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // Verify CRC passes
        assert!(MetadataStore::verify_crc(&bytes, crc).is_ok());

        // Restore and verify data
        let restored = MetadataStore::from_postcard(&bytes).unwrap();
        assert_eq!(store, restored);
    }

    #[test]
    fn test_crc_mismatch_detected() {
        let store = MetadataStore::new();
        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // Wrong CRC should fail
        let result = MetadataStore::verify_crc(&bytes, crc.wrapping_add(1));
        assert!(matches!(
            result,
            Err(SerializationError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn test_crc_detects_byte_corruption() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        let mut bytes = store.to_postcard().unwrap();
        let original_crc = MetadataStore::calculate_crc(&bytes);

        // Corrupt middle byte
        if bytes.len() > 2 {
            let mid = bytes.len() / 2;
            bytes[mid] ^= 0xFF;
        }

        // CRC verification should fail
        assert!(MetadataStore::verify_crc(&bytes, original_crc).is_err());
    }

    #[test]
    fn test_crc_is_deterministic() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(42)).unwrap();

        let bytes = store.to_postcard().unwrap();
        let crc1 = MetadataStore::calculate_crc(&bytes);
        let crc2 = MetadataStore::calculate_crc(&bytes);
        let crc3 = MetadataStore::calculate_crc(&bytes);

        assert_eq!(crc1, crc2);
        assert_eq!(crc2, crc3);
    }

    #[test]
    fn test_crc_different_for_different_data() {
        let mut store1 = MetadataStore::new();
        store1.insert(0, "key", MetadataValue::Integer(1)).unwrap();

        let mut store2 = MetadataStore::new();
        store2.insert(0, "key", MetadataValue::Integer(2)).unwrap();

        let bytes1 = store1.to_postcard().unwrap();
        let bytes2 = store2.to_postcard().unwrap();

        let crc1 = MetadataStore::calculate_crc(&bytes1);
        let crc2 = MetadataStore::calculate_crc(&bytes2);

        // Different data should produce different CRCs
        assert_ne!(crc1, crc2);
    }
}

// =============================================================================
// JSON serialization tests
// =============================================================================

mod json_roundtrip {
    use super::*;

    #[test]
    fn test_json_empty_store() {
        let store = MetadataStore::new();
        let bytes = store.to_json().unwrap();
        let restored = MetadataStore::from_json(&bytes).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_json_with_data() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "title", MetadataValue::String("Hello World".into()))
            .unwrap();

        let bytes = store.to_json().unwrap();
        let restored = MetadataStore::from_json(&bytes).unwrap();

        assert_eq!(store, restored);
    }

    #[test]
    fn test_json_is_readable() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "title", MetadataValue::String("Test Document".into()))
            .unwrap();

        let bytes = store.to_json().unwrap();
        let json_str = String::from_utf8(bytes).expect("JSON should be valid UTF-8");

        // JSON should contain the field name
        assert!(json_str.contains("title"));
    }
}

// =============================================================================
// Error handling tests
// =============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn test_postcard_decode_garbage() {
        let garbage = [0xFF, 0xFE, 0xFD, 0xFC, 0xFB];
        let result = MetadataStore::from_postcard(&garbage);
        assert!(matches!(result, Err(SerializationError::PostcardDecode(_))));
    }

    #[test]
    fn test_postcard_decode_truncated() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        let bytes = store.to_postcard().unwrap();

        // Truncate the bytes
        if bytes.len() > 2 {
            let truncated = &bytes[..bytes.len() / 2];
            let result = MetadataStore::from_postcard(truncated);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_json_decode_invalid() {
        let invalid = b"{ invalid json }}}";
        let result = MetadataStore::from_json(invalid);
        assert!(matches!(result, Err(SerializationError::JsonDecode(_))));
    }

    #[test]
    fn test_json_decode_wrong_structure() {
        // Valid JSON but wrong structure
        let wrong_structure = b"[]";
        let result = MetadataStore::from_json(wrong_structure);
        assert!(result.is_err());
    }
}

// =============================================================================
// Format comparison tests
// =============================================================================

mod format_comparison {
    use super::*;

    #[test]
    fn test_postcard_is_smaller_than_json() {
        let mut store = MetadataStore::new();
        store
            .insert(
                0,
                "title",
                MetadataValue::String("The Quick Brown Fox".into()),
            )
            .unwrap();
        store
            .insert(0, "count", MetadataValue::Integer(12345))
            .unwrap();
        store
            .insert(0, "ratio", MetadataValue::Float(0.87654))
            .unwrap();

        let postcard = store.to_postcard().unwrap();
        let json = store.to_json().unwrap();

        assert!(
            postcard.len() < json.len(),
            "Postcard ({} bytes) should be smaller than JSON ({} bytes)",
            postcard.len(),
            json.len()
        );
    }

    #[test]
    fn test_both_formats_produce_equivalent_data() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();
        store.insert(1, "num", MetadataValue::Integer(-42)).unwrap();

        let postcard_bytes = store.to_postcard().unwrap();
        let json_bytes = store.to_json().unwrap();

        let from_postcard = MetadataStore::from_postcard(&postcard_bytes).unwrap();
        let from_json = MetadataStore::from_json(&json_bytes).unwrap();

        // Both should produce identical stores
        assert_eq!(from_postcard, from_json);
        assert_eq!(from_postcard, store);
    }
}

// =============================================================================
// Header integration tests
// =============================================================================

mod header_integration {
    use super::*;
    use edgevec::persistence::{MetadataSectionHeader, FORMAT_POSTCARD};

    #[test]
    fn test_create_header_from_serialized_store() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        // Serialize to postcard
        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // Create header
        #[allow(clippy::cast_possible_truncation)]
        let header = MetadataSectionHeader::new_postcard(bytes.len() as u32, crc);

        // Verify header values
        assert_eq!(header.magic, *b"META");
        assert_eq!(header.version, 1);
        assert_eq!(header.format, FORMAT_POSTCARD);
        assert_eq!(header.size, bytes.len() as u32);
        assert_eq!(header.crc, crc);
    }

    #[test]
    fn test_header_crc_matches_calculated() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "test", MetadataValue::Integer(12345))
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let calculated_crc = MetadataStore::calculate_crc(&bytes);

        #[allow(clippy::cast_possible_truncation)]
        let header = MetadataSectionHeader::new_postcard(bytes.len() as u32, calculated_crc);

        // Verify CRC in header matches what we'd verify
        assert!(MetadataStore::verify_crc(&bytes, header.crc).is_ok());
    }
}
