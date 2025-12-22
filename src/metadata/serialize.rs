//! Serialization support for MetadataStore (W26.4.2).
//!
//! Provides Postcard (binary, compact) and JSON (text, debugging) serialization
//! for persistence per RFC-002 Persistence Format.
//!
//! # Why Custom Serialization?
//!
//! Postcard doesn't support HashMap with String keys directly (only numeric keys).
//! We convert to Vec representation for binary serialization while maintaining
//! JSON compatibility for debugging.
//!
//! # Example
//!
//! ```rust
//! use edgevec::metadata::{MetadataStore, MetadataValue};
//!
//! let mut store = MetadataStore::new();
//! store.insert(0, "key", MetadataValue::String("value".to_string())).unwrap();
//!
//! // Serialize to Postcard (compact binary)
//! let bytes = store.to_postcard().unwrap();
//! let crc = MetadataStore::calculate_crc(&bytes);
//!
//! // Deserialize and verify
//! MetadataStore::verify_crc(&bytes, crc).unwrap();
//! let restored = MetadataStore::from_postcard(&bytes).unwrap();
//! assert_eq!(store.get(0, "key"), restored.get(0, "key"));
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::store::MetadataStore;
use super::types::MetadataValue;

/// Postcard-compatible representation of MetadataValue.
///
/// The original MetadataValue uses adjacently-tagged representation
/// (`#[serde(tag = "type", content = "value")]`) which postcard doesn't support.
/// This enum uses simple tuple variants compatible with postcard.
#[derive(Serialize, Deserialize)]
enum PostcardValue {
    /// String value (tag = 0)
    S(String),
    /// Integer value (tag = 1)
    I(i64),
    /// Float value (tag = 2)
    F(f64),
    /// Boolean value (tag = 3)
    B(bool),
    /// StringArray value (tag = 4)
    A(Vec<String>),
}

impl From<&MetadataValue> for PostcardValue {
    fn from(value: &MetadataValue) -> Self {
        match value {
            MetadataValue::String(s) => PostcardValue::S(s.clone()),
            MetadataValue::Integer(i) => PostcardValue::I(*i),
            MetadataValue::Float(f) => PostcardValue::F(*f),
            MetadataValue::Boolean(b) => PostcardValue::B(*b),
            MetadataValue::StringArray(a) => PostcardValue::A(a.clone()),
        }
    }
}

impl From<PostcardValue> for MetadataValue {
    fn from(pv: PostcardValue) -> Self {
        match pv {
            PostcardValue::S(s) => MetadataValue::String(s),
            PostcardValue::I(i) => MetadataValue::Integer(i),
            PostcardValue::F(f) => MetadataValue::Float(f),
            PostcardValue::B(b) => MetadataValue::Boolean(b),
            PostcardValue::A(a) => MetadataValue::StringArray(a),
        }
    }
}

/// Postcard-compatible representation of MetadataStore.
///
/// Converts HashMap<u32, HashMap<String, MetadataValue>> to Vec form
/// for Postcard serialization (which doesn't support String-keyed maps).
#[derive(Serialize, Deserialize)]
struct PostcardMetadata {
    /// List of (vector_id, key, value) tuples
    entries: Vec<(u32, String, PostcardValue)>,
}

impl From<&MetadataStore> for PostcardMetadata {
    fn from(store: &MetadataStore) -> Self {
        let mut entries = Vec::new();
        for vector_id in store.vector_ids() {
            if let Some(metadata) = store.get_all(*vector_id) {
                for (key, value) in metadata {
                    entries.push((*vector_id, key.clone(), PostcardValue::from(value)));
                }
            }
        }
        Self { entries }
    }
}

impl From<PostcardMetadata> for MetadataStore {
    fn from(pm: PostcardMetadata) -> Self {
        let mut store = MetadataStore::new();
        for (vector_id, key, pv) in pm.entries {
            // Convert PostcardValue back to MetadataValue
            let value = MetadataValue::from(pv);
            // We use insert directly, bypassing validation since this is from
            // trusted serialized data. Validation was done on original insert.
            let _ = store.insert(vector_id, &key, value);
        }
        store
    }
}

/// Errors that can occur during metadata serialization/deserialization.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SerializationError {
    /// Postcard encoding failed.
    #[error("postcard encode failed: {0}")]
    PostcardEncode(String),

    /// Postcard decoding failed.
    #[error("postcard decode failed: {0}")]
    PostcardDecode(String),

    /// JSON encoding failed.
    #[error("json encode failed: {0}")]
    JsonEncode(String),

    /// JSON decoding failed.
    #[error("json decode failed: {0}")]
    JsonDecode(String),

    /// CRC32 mismatch during verification.
    #[error("CRC mismatch: expected {expected:#x}, got {actual:#x}")]
    CrcMismatch {
        /// Expected CRC32 value.
        expected: u32,
        /// Actual calculated CRC32 value.
        actual: u32,
    },
}

impl MetadataStore {
    /// Serializes the metadata store to Postcard format (binary, compact).
    ///
    /// Postcard is a no-std compatible, compact binary format ideal for
    /// embedded storage and WASM environments.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the serialized bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen with valid data).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "key", MetadataValue::Integer(42)).unwrap();
    ///
    /// let bytes = store.to_postcard().unwrap();
    /// assert!(!bytes.is_empty());
    /// ```
    pub fn to_postcard(&self) -> Result<Vec<u8>, SerializationError> {
        let pm = PostcardMetadata::from(self);
        postcard::to_allocvec(&pm).map_err(|e| SerializationError::PostcardEncode(e.to_string()))
    }

    /// Deserializes a metadata store from Postcard format.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The serialized Postcard bytes.
    ///
    /// # Returns
    ///
    /// A reconstructed `MetadataStore`.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid Postcard data or the data
    /// is corrupted.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut original = MetadataStore::new();
    /// original.insert(0, "key", MetadataValue::Integer(42)).unwrap();
    ///
    /// let bytes = original.to_postcard().unwrap();
    /// let restored = MetadataStore::from_postcard(&bytes).unwrap();
    ///
    /// assert_eq!(original.get(0, "key"), restored.get(0, "key"));
    /// ```
    pub fn from_postcard(bytes: &[u8]) -> Result<Self, SerializationError> {
        let pm: PostcardMetadata = postcard::from_bytes(bytes)
            .map_err(|e| SerializationError::PostcardDecode(e.to_string()))?;
        Ok(MetadataStore::from(pm))
    }

    /// Serializes the metadata store to JSON format (for debugging/interop).
    ///
    /// JSON is human-readable and useful for debugging, but less compact than
    /// Postcard. Use [`to_postcard`] for production persistence.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the UTF-8 encoded JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen with valid data).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "key", MetadataValue::String("hello".to_string())).unwrap();
    ///
    /// let json_bytes = store.to_json().unwrap();
    /// let json_str = String::from_utf8(json_bytes).unwrap();
    /// assert!(json_str.contains("key"));
    /// ```
    pub fn to_json(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(|e| SerializationError::JsonEncode(e.to_string()))
    }

    /// Deserializes a metadata store from JSON format.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The UTF-8 encoded JSON bytes.
    ///
    /// # Returns
    ///
    /// A reconstructed `MetadataStore`.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid JSON or the data structure
    /// doesn't match the expected format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut original = MetadataStore::new();
    /// original.insert(0, "key", MetadataValue::String("hello".to_string())).unwrap();
    ///
    /// let json_bytes = original.to_json().unwrap();
    /// let restored = MetadataStore::from_json(&json_bytes).unwrap();
    ///
    /// assert_eq!(original.get(0, "key"), restored.get(0, "key"));
    /// ```
    pub fn from_json(bytes: &[u8]) -> Result<Self, SerializationError> {
        serde_json::from_slice(bytes).map_err(|e| SerializationError::JsonDecode(e.to_string()))
    }

    /// Calculates CRC32 checksum for serialized bytes.
    ///
    /// Use this to create a checksum for persistence headers.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The serialized bytes to checksum.
    ///
    /// # Returns
    ///
    /// The CRC32 checksum as a `u32`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// let store = MetadataStore::new();
    /// let bytes = store.to_postcard().unwrap();
    /// let crc = MetadataStore::calculate_crc(&bytes);
    /// assert_ne!(crc, 0); // CRC is calculated
    /// ```
    #[must_use]
    pub fn calculate_crc(bytes: &[u8]) -> u32 {
        crc32fast::hash(bytes)
    }

    /// Verifies CRC32 checksum matches expected value.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The serialized bytes to verify.
    /// * `expected` - The expected CRC32 value.
    ///
    /// # Errors
    ///
    /// Returns `SerializationError::CrcMismatch` if the calculated CRC doesn't
    /// match the expected value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// let store = MetadataStore::new();
    /// let bytes = store.to_postcard().unwrap();
    /// let crc = MetadataStore::calculate_crc(&bytes);
    ///
    /// // Valid CRC passes
    /// assert!(MetadataStore::verify_crc(&bytes, crc).is_ok());
    ///
    /// // Invalid CRC fails
    /// assert!(MetadataStore::verify_crc(&bytes, crc + 1).is_err());
    /// ```
    pub fn verify_crc(bytes: &[u8], expected: u32) -> Result<(), SerializationError> {
        let actual = Self::calculate_crc(bytes);
        if actual != expected {
            return Err(SerializationError::CrcMismatch { expected, actual });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataValue;

    // =========================================================================
    // Postcard serialization tests
    // =========================================================================

    #[test]
    fn test_postcard_roundtrip_empty() {
        let store = MetadataStore::new();
        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_postcard_roundtrip_with_data() {
        let mut store = MetadataStore::new();
        store
            .insert(1, "key", MetadataValue::String("value".into()))
            .unwrap();
        store
            .insert(1, "count", MetadataValue::Integer(42))
            .unwrap();
        store
            .insert(2, "price", MetadataValue::Float(29.99))
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(store.get(1, "key"), restored.get(1, "key"));
        assert_eq!(store.get(1, "count"), restored.get(1, "count"));
        assert_eq!(store.get(2, "price"), restored.get(2, "price"));
        assert_eq!(store, restored);
    }

    #[test]
    fn test_postcard_roundtrip_all_types() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "string", MetadataValue::String("hello".into()))
            .unwrap();
        store
            .insert(0, "integer", MetadataValue::Integer(-42))
            .unwrap();
        store
            .insert(0, "float", MetadataValue::Float(core::f64::consts::PI))
            .unwrap();
        store
            .insert(0, "boolean", MetadataValue::Boolean(true))
            .unwrap();
        store
            .insert(
                0,
                "array",
                MetadataValue::StringArray(vec!["a".into(), "b".into(), "c".into()]),
            )
            .unwrap();

        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(store, restored);
    }

    #[test]
    fn test_postcard_decode_invalid_bytes() {
        let invalid_bytes = [0xFF, 0xFF, 0xFF, 0xFF];
        let result = MetadataStore::from_postcard(&invalid_bytes);
        assert!(result.is_err());
        match result {
            Err(SerializationError::PostcardDecode(_)) => {}
            _ => panic!("Expected PostcardDecode error"),
        }
    }

    #[test]
    fn test_postcard_is_compact() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        let postcard_bytes = store.to_postcard().unwrap();
        let json_bytes = store.to_json().unwrap();

        // Postcard should be more compact than JSON
        assert!(
            postcard_bytes.len() < json_bytes.len(),
            "Postcard ({} bytes) should be smaller than JSON ({} bytes)",
            postcard_bytes.len(),
            json_bytes.len()
        );
    }

    // =========================================================================
    // JSON serialization tests
    // =========================================================================

    #[test]
    fn test_json_roundtrip_empty() {
        let store = MetadataStore::new();
        let bytes = store.to_json().unwrap();
        let restored = MetadataStore::from_json(&bytes).unwrap();
        assert!(restored.is_empty());
    }

    #[test]
    fn test_json_roundtrip_with_data() {
        let mut store = MetadataStore::new();
        store
            .insert(1, "key", MetadataValue::String("value".into()))
            .unwrap();

        let bytes = store.to_json().unwrap();
        let restored = MetadataStore::from_json(&bytes).unwrap();

        assert_eq!(store.get(1, "key"), restored.get(1, "key"));
    }

    #[test]
    fn test_json_is_human_readable() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "title", MetadataValue::String("Hello World".into()))
            .unwrap();

        let bytes = store.to_json().unwrap();
        let json_str = String::from_utf8(bytes).unwrap();

        // JSON should be human-readable with field names visible
        assert!(json_str.contains("title"));
        assert!(json_str.contains("Hello World"));
    }

    #[test]
    fn test_json_decode_invalid_bytes() {
        let invalid_bytes = b"not valid json";
        let result = MetadataStore::from_json(invalid_bytes);
        assert!(result.is_err());
        match result {
            Err(SerializationError::JsonDecode(_)) => {}
            _ => panic!("Expected JsonDecode error"),
        }
    }

    // =========================================================================
    // CRC32 tests
    // =========================================================================

    #[test]
    fn test_crc_calculates_correctly() {
        let store = MetadataStore::new();
        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // CRC should be non-zero for any data
        // (Empty store still serializes to some bytes)
        assert!(!bytes.is_empty());

        // Same input should produce same CRC (deterministic)
        let crc2 = MetadataStore::calculate_crc(&bytes);
        assert_eq!(crc, crc2);
    }

    #[test]
    fn test_crc_validates_correctly() {
        let store = MetadataStore::new();
        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // Valid CRC should pass
        assert!(MetadataStore::verify_crc(&bytes, crc).is_ok());
    }

    #[test]
    fn test_crc_rejects_mismatch() {
        let store = MetadataStore::new();
        let bytes = store.to_postcard().unwrap();
        let crc = MetadataStore::calculate_crc(&bytes);

        // Invalid CRC should fail
        let result = MetadataStore::verify_crc(&bytes, crc + 1);
        assert!(result.is_err());
        match result {
            Err(SerializationError::CrcMismatch { expected, actual }) => {
                assert_eq!(expected, crc + 1);
                assert_eq!(actual, crc);
            }
            _ => panic!("Expected CrcMismatch error"),
        }
    }

    #[test]
    fn test_crc_detects_corruption() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();

        let mut bytes = store.to_postcard().unwrap();
        let original_crc = MetadataStore::calculate_crc(&bytes);

        // Corrupt a byte
        if !bytes.is_empty() {
            bytes[0] ^= 0xFF;
        }

        // CRC should now be different
        let corrupted_crc = MetadataStore::calculate_crc(&bytes);
        assert_ne!(original_crc, corrupted_crc);

        // Verification should fail
        assert!(MetadataStore::verify_crc(&bytes, original_crc).is_err());
    }

    // =========================================================================
    // Round-trip between formats
    // =========================================================================

    #[test]
    fn test_formats_produce_same_data() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".into()))
            .unwrap();
        store
            .insert(0, "count", MetadataValue::Integer(42))
            .unwrap();

        // Serialize to both formats
        let postcard_bytes = store.to_postcard().unwrap();
        let json_bytes = store.to_json().unwrap();

        // Deserialize both
        let from_postcard = MetadataStore::from_postcard(&postcard_bytes).unwrap();
        let from_json = MetadataStore::from_json(&json_bytes).unwrap();

        // Both should produce identical stores
        assert_eq!(from_postcard, from_json);
        assert_eq!(from_postcard, store);
    }

    // =========================================================================
    // Large data tests
    // =========================================================================

    #[test]
    fn test_postcard_large_store() {
        let mut store = MetadataStore::new();

        // Insert 100 vectors with 10 keys each
        for v in 0..100u32 {
            for k in 0..10 {
                store
                    .insert(
                        v,
                        &format!("key_{k}"),
                        MetadataValue::Integer(i64::from(v) * 10 + i64::from(k)),
                    )
                    .unwrap();
            }
        }

        assert_eq!(store.vector_count(), 100);
        assert_eq!(store.total_key_count(), 1000);

        // Serialize and deserialize
        let bytes = store.to_postcard().unwrap();
        let restored = MetadataStore::from_postcard(&bytes).unwrap();

        assert_eq!(store, restored);
        assert_eq!(restored.vector_count(), 100);
        assert_eq!(restored.total_key_count(), 1000);
    }
}
