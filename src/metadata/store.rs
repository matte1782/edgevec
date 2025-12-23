//! Metadata storage implementation.
//!
//! This module provides the `MetadataStore` struct for storing key-value
//! metadata attached to vectors.
//!
//! # Architecture
//!
//! ```text
//! MetadataStore
//! +-- data: HashMap<VectorId, HashMap<Key, MetadataValue>>
//! |
//! +-- Operations:
//!     +-- insert(vector_id, key, value) -> Result<()>
//!     +-- get(vector_id, key) -> Option<&MetadataValue>
//!     +-- get_all(vector_id) -> Option<&HashMap<String, MetadataValue>>
//!     +-- update(vector_id, key, value) -> Result<()>
//!     +-- delete(vector_id, key) -> Result<bool>
//!     +-- delete_all(vector_id) -> bool
//!     +-- has_key(vector_id, key) -> bool
//!     +-- keys(vector_id) -> Option<impl Iterator>
//!     +-- key_count(vector_id) -> usize
//!     +-- clear()
//!     +-- merge(other)
//! ```
//!
//! # Example
//!
//! ```rust
//! use edgevec::metadata::{MetadataStore, MetadataValue};
//!
//! let mut store = MetadataStore::new();
//!
//! // Insert metadata for vector 0
//! store.insert(0, "title", MetadataValue::String("Document".to_string())).unwrap();
//! store.insert(0, "page_count", MetadataValue::Integer(42)).unwrap();
//!
//! // Retrieve metadata
//! let title = store.get(0, "title").unwrap();
//! assert_eq!(title.as_string(), Some("Document"));
//!
//! // Update metadata
//! store.update(0, "page_count", MetadataValue::Integer(50)).unwrap();
//!
//! // Delete metadata
//! store.delete(0, "page_count").unwrap();
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::error::MetadataError;
use super::types::MetadataValue;
use super::validation::{validate_key, validate_value, MAX_KEYS_PER_VECTOR};

/// Storage for vector metadata.
///
/// `MetadataStore` provides a key-value store for attaching metadata to vectors.
/// Each vector can have up to 64 metadata keys, and values can be one of 5 types.
///
/// # Thread Safety
///
/// `MetadataStore` is `Send` and `Sync` when wrapped in appropriate synchronization
/// primitives. The store itself does not provide internal synchronization.
///
/// # Memory Usage
///
/// Each vector's metadata is stored in a separate `HashMap`. Empty vectors are
/// automatically cleaned up when their last key is deleted.
///
/// # Serialization
///
/// The store implements `Serialize` and `Deserialize` for persistence support.
/// It can be serialized to JSON, MessagePack, or any other serde-compatible format.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::{MetadataStore, MetadataValue};
///
/// let mut store = MetadataStore::new();
///
/// // Insert metadata
/// store.insert(0, "title", MetadataValue::String("Hello".to_string())).unwrap();
/// store.insert(0, "count", MetadataValue::Integer(42)).unwrap();
///
/// // Check existence
/// assert!(store.has_key(0, "title"));
/// assert!(!store.has_key(0, "missing"));
///
/// // Get all metadata for a vector
/// let all = store.get_all(0).unwrap();
/// assert_eq!(all.len(), 2);
///
/// // Delete all metadata for a vector
/// store.delete_all(0);
/// assert!(store.is_empty());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetadataStore {
    /// Internal storage: VectorId -> (Key -> Value)
    data: HashMap<u32, HashMap<String, MetadataValue>>,
}

impl MetadataStore {
    /// Creates a new, empty metadata store.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// let store = MetadataStore::new();
    /// assert!(store.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Creates a metadata store with pre-allocated capacity.
    ///
    /// This is useful when you know approximately how many vectors will have metadata.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The number of vectors to pre-allocate space for.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// // Pre-allocate for 1000 vectors
    /// let store = MetadataStore::with_capacity(1000);
    /// assert!(store.is_empty());
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
        }
    }

    // =========================================================================
    // INSERT / UPDATE Operations
    // =========================================================================

    /// Inserts or updates a metadata key-value pair for a vector (upsert).
    ///
    /// This method performs an "upsert" operation:
    /// - If the key doesn't exist, it's created (subject to key count limit).
    /// - If the key already exists, its value is overwritten.
    ///
    /// Use [`Self::update`] if you want to fail when the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector to attach metadata to.
    /// * `key` - The metadata key (must be alphanumeric + underscore, max 256 bytes).
    /// * `value` - The metadata value.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key is empty ([`MetadataError::EmptyKey`])
    /// - Key exceeds maximum length ([`MetadataError::KeyTooLong`])
    /// - Key contains invalid characters ([`MetadataError::InvalidKeyFormat`])
    /// - Value fails validation (e.g., string too long, NaN float)
    /// - Vector already has maximum number of keys (64) and this is a new key
    ///   ([`MetadataError::TooManyKeys`])
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "title", MetadataValue::String("Hello".to_string())).unwrap();
    /// store.insert(0, "count", MetadataValue::Integer(42)).unwrap();
    ///
    /// // Overwrite existing key (upsert behavior)
    /// store.insert(0, "count", MetadataValue::Integer(100)).unwrap();
    /// assert_eq!(store.get(0, "count").unwrap().as_integer(), Some(100));
    ///
    /// assert_eq!(store.key_count(0), 2);
    /// ```
    pub fn insert(
        &mut self,
        vector_id: u32,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), MetadataError> {
        // Validate key and value
        validate_key(key)?;
        validate_value(&value)?;

        let entry = self.data.entry(vector_id).or_default();

        // Check key limit before inserting a NEW key
        if !entry.contains_key(key) && entry.len() >= MAX_KEYS_PER_VECTOR {
            return Err(MetadataError::TooManyKeys {
                vector_id,
                count: entry.len(),
                max: MAX_KEYS_PER_VECTOR,
            });
        }

        entry.insert(key.to_string(), value);
        Ok(())
    }

    /// Updates an existing metadata key-value pair.
    ///
    /// Unlike [`Self::insert`], this method fails if the key doesn't exist.
    /// Use this when you want to ensure you're modifying an existing value.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    /// * `key` - The metadata key to update.
    /// * `value` - The new metadata value.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Vector has no metadata ([`MetadataError::VectorNotFound`])
    /// - Key doesn't exist for the vector ([`MetadataError::KeyNotFound`])
    /// - Value fails validation
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue, MetadataError};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "count", MetadataValue::Integer(1)).unwrap();
    ///
    /// // Update existing key
    /// store.update(0, "count", MetadataValue::Integer(2)).unwrap();
    /// assert_eq!(store.get(0, "count").unwrap().as_integer(), Some(2));
    ///
    /// // Update non-existent key fails
    /// let result = store.update(0, "missing", MetadataValue::Integer(0));
    /// assert!(matches!(result, Err(MetadataError::KeyNotFound { .. })));
    /// ```
    pub fn update(
        &mut self,
        vector_id: u32,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), MetadataError> {
        // Validate value (key is already stored, no need to validate again)
        validate_value(&value)?;

        let entry = self
            .data
            .get_mut(&vector_id)
            .ok_or(MetadataError::VectorNotFound { vector_id })?;

        if !entry.contains_key(key) {
            return Err(MetadataError::KeyNotFound {
                vector_id,
                key: key.to_string(),
            });
        }

        entry.insert(key.to_string(), value);
        Ok(())
    }

    // =========================================================================
    // GET Operations
    // =========================================================================

    /// Gets a metadata value for a vector.
    ///
    /// Returns `None` if the vector or key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    /// * `key` - The metadata key to retrieve.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "title", MetadataValue::String("Hello".to_string())).unwrap();
    ///
    /// assert_eq!(store.get(0, "title").unwrap().as_string(), Some("Hello"));
    /// assert!(store.get(0, "missing").is_none());
    /// assert!(store.get(999, "title").is_none());
    /// ```
    #[must_use]
    pub fn get(&self, vector_id: u32, key: &str) -> Option<&MetadataValue> {
        self.data.get(&vector_id)?.get(key)
    }

    /// Gets all metadata for a vector.
    ///
    /// Returns `None` if the vector has no metadata.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// let all = store.get_all(0).unwrap();
    /// assert_eq!(all.len(), 2);
    /// assert!(store.get_all(999).is_none());
    /// ```
    #[must_use]
    pub fn get_all(&self, vector_id: u32) -> Option<&HashMap<String, MetadataValue>> {
        self.data.get(&vector_id)
    }

    // =========================================================================
    // DELETE Operations
    // =========================================================================

    /// Deletes a metadata key for a vector.
    ///
    /// Returns `Ok(true)` if the key existed and was deleted, `Ok(false)` if
    /// the key or vector didn't exist. This method is idempotent—calling it
    /// multiple times with the same arguments has no additional effect.
    ///
    /// If this was the last key for the vector, the vector entry is also removed
    /// (automatic cleanup to prevent memory leaks).
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    /// * `key` - The metadata key to delete.
    ///
    /// # Errors
    ///
    /// This method is infallible and always returns `Ok`. The `Result` is provided
    /// for API consistency with other mutation methods.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "title", MetadataValue::String("Hello".to_string())).unwrap();
    ///
    /// assert!(store.delete(0, "title").unwrap());
    /// assert!(!store.delete(0, "title").unwrap()); // Already deleted
    /// assert!(!store.delete(999, "title").unwrap()); // Non-existent vector
    /// ```
    pub fn delete(&mut self, vector_id: u32, key: &str) -> Result<bool, MetadataError> {
        let Some(entry) = self.data.get_mut(&vector_id) else {
            return Ok(false);
        };

        let removed = entry.remove(key).is_some();

        // Clean up empty entries to prevent memory leaks
        if entry.is_empty() {
            self.data.remove(&vector_id);
        }

        Ok(removed)
    }

    /// Deletes all metadata for a vector.
    ///
    /// Returns `true` if the vector had metadata that was deleted, `false` otherwise.
    /// This method is idempotent—calling it multiple times has no additional effect.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// assert!(store.delete_all(0));
    /// assert!(!store.delete_all(0)); // Already deleted
    /// assert!(store.is_empty());
    /// ```
    pub fn delete_all(&mut self, vector_id: u32) -> bool {
        self.data.remove(&vector_id).is_some()
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Checks if a key exists for a vector.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    /// * `key` - The metadata key to check.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "title", MetadataValue::String("Hello".to_string())).unwrap();
    ///
    /// assert!(store.has_key(0, "title"));
    /// assert!(!store.has_key(0, "missing"));
    /// assert!(!store.has_key(999, "title"));
    /// ```
    #[must_use]
    pub fn has_key(&self, vector_id: u32, key: &str) -> bool {
        self.data
            .get(&vector_id)
            .is_some_and(|m| m.contains_key(key))
    }

    /// Returns an iterator over all keys for a vector.
    ///
    /// Returns `None` if the vector has no metadata.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// let keys: Vec<_> = store.keys(0).unwrap().collect();
    /// assert_eq!(keys.len(), 2);
    /// ```
    #[must_use]
    pub fn keys(&self, vector_id: u32) -> Option<impl Iterator<Item = &String>> {
        self.data.get(&vector_id).map(HashMap::keys)
    }

    /// Returns the number of keys for a vector.
    ///
    /// Returns 0 if the vector has no metadata.
    ///
    /// # Arguments
    ///
    /// * `vector_id` - The ID of the vector.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// assert_eq!(store.key_count(0), 0);
    ///
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// assert_eq!(store.key_count(0), 1);
    /// ```
    #[must_use]
    pub fn key_count(&self, vector_id: u32) -> usize {
        self.data.get(&vector_id).map_or(0, HashMap::len)
    }

    // =========================================================================
    // Store-level Operations
    // =========================================================================

    /// Returns the total number of vectors with metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// assert_eq!(store.vector_count(), 0);
    ///
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(1, "b", MetadataValue::Integer(2)).unwrap();
    /// assert_eq!(store.vector_count(), 2);
    /// ```
    #[must_use]
    pub fn vector_count(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the store contains no metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// let store = MetadataStore::new();
    /// assert!(store.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears all metadata from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(1, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Merges another metadata store into this one, validating key limits.
    ///
    /// For each vector in the other store:
    /// - If the vector doesn't exist in this store, it's added.
    /// - If the vector exists, the keys are merged (other store's values overwrite).
    ///
    /// This method validates that no vector exceeds the 64-key limit after merging.
    /// If any vector would exceed the limit, the entire merge is aborted and no
    /// changes are made.
    ///
    /// # Arguments
    ///
    /// * `other` - The metadata store to merge into this one (consumed).
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the merge succeeded
    /// * `Err(MetadataError::TooManyKeys)` if any vector would exceed the key limit
    ///
    /// # Errors
    ///
    /// Returns an error if merging would cause any vector to exceed
    /// `MAX_KEYS_PER_VECTOR` (64) keys. The error includes the first vector_id
    /// that would exceed the limit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store1 = MetadataStore::new();
    /// store1.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    ///
    /// let mut store2 = MetadataStore::new();
    /// store2.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    /// store2.insert(1, "c", MetadataValue::Integer(3)).unwrap();
    ///
    /// store1.merge(store2).unwrap();
    ///
    /// assert_eq!(store1.key_count(0), 2); // Both "a" and "b"
    /// assert_eq!(store1.key_count(1), 1); // "c" from store2
    /// ```
    pub fn merge(&mut self, other: MetadataStore) -> Result<(), MetadataError> {
        // Phase 1: Validate that no vector will exceed limits
        for (vector_id, other_metadata) in &other.data {
            let current_keys = self.data.get(vector_id);

            // Count unique keys after merge
            let merged_count = if let Some(current) = current_keys {
                // Count keys that are in other but not in current (new keys)
                let new_keys = other_metadata
                    .keys()
                    .filter(|k| !current.contains_key(*k))
                    .count();
                current.len() + new_keys
            } else {
                other_metadata.len()
            };

            if merged_count > MAX_KEYS_PER_VECTOR {
                return Err(MetadataError::TooManyKeys {
                    vector_id: *vector_id,
                    count: merged_count,
                    max: MAX_KEYS_PER_VECTOR,
                });
            }
        }

        // Phase 2: Perform merge (validation passed)
        for (vector_id, other_metadata) in other.data {
            let entry = self.data.entry(vector_id).or_default();
            entry.extend(other_metadata);
        }

        Ok(())
    }

    /// Merges another metadata store without validation (unchecked).
    ///
    /// This method bypasses the key limit check and should only be used when
    /// you have already validated that the merge won't exceed limits, or in
    /// controlled scenarios where exceeding limits is acceptable (e.g., disaster
    /// recovery, data migration).
    ///
    /// # Safety Note
    ///
    /// While memory-safe, this method can create stores that violate the
    /// documented 64-key-per-vector invariant. Subsequent `insert()` calls
    /// on over-limit vectors will fail until keys are deleted.
    ///
    /// # Arguments
    ///
    /// * `other` - The metadata store to merge into this one (consumed).
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store1 = MetadataStore::new();
    /// store1.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    ///
    /// let mut store2 = MetadataStore::new();
    /// store2.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// store1.merge_unchecked(store2);
    ///
    /// assert_eq!(store1.key_count(0), 2);
    /// ```
    pub fn merge_unchecked(&mut self, other: MetadataStore) {
        for (vector_id, other_metadata) in other.data {
            let entry = self.data.entry(vector_id).or_default();
            entry.extend(other_metadata);
        }
    }

    /// Returns an iterator over all vector IDs that have metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(5, "b", MetadataValue::Integer(2)).unwrap();
    ///
    /// let ids: Vec<_> = store.vector_ids().collect();
    /// assert_eq!(ids.len(), 2);
    /// ```
    pub fn vector_ids(&self) -> impl Iterator<Item = &u32> {
        self.data.keys()
    }

    /// Returns the total number of key-value pairs across all vectors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::{MetadataStore, MetadataValue};
    ///
    /// let mut store = MetadataStore::new();
    /// store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
    /// store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
    /// store.insert(1, "c", MetadataValue::Integer(3)).unwrap();
    ///
    /// assert_eq!(store.total_key_count(), 3);
    /// ```
    #[must_use]
    pub fn total_key_count(&self) -> usize {
        self.data.values().map(HashMap::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::validation::MAX_KEYS_PER_VECTOR;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_new_store_is_empty() {
        let store = MetadataStore::new();
        assert!(store.is_empty());
        assert_eq!(store.vector_count(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let store = MetadataStore::with_capacity(100);
        assert!(store.is_empty());
        assert_eq!(store.vector_count(), 0);
    }

    #[test]
    fn test_with_capacity_preallocates() {
        // Verify with_capacity works by inserting many vectors without rehashing
        // HashMap with capacity 100 should handle 100 vectors without realloc
        let mut store = MetadataStore::with_capacity(100);

        // Insert 100 different vectors - this would cause rehashing if capacity wasn't set
        for i in 0..100u32 {
            store
                .insert(i, "key", MetadataValue::Integer(i64::from(i)))
                .unwrap();
        }

        assert_eq!(store.vector_count(), 100);

        // Zero capacity should work
        let store_zero = MetadataStore::with_capacity(0);
        assert!(store_zero.is_empty());
    }

    #[test]
    fn test_default_trait() {
        let store: MetadataStore = MetadataStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn test_clone() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(42)).unwrap();

        let cloned = store.clone();
        assert_eq!(store.get(0, "key"), cloned.get(0, "key"));
    }

    #[test]
    fn test_debug() {
        let store = MetadataStore::new();
        let debug_str = format!("{store:?}");
        assert!(debug_str.contains("MetadataStore"));
    }

    #[test]
    fn test_partial_eq() {
        let mut store1 = MetadataStore::new();
        store1.insert(0, "key", MetadataValue::Integer(42)).unwrap();

        let mut store2 = MetadataStore::new();
        store2.insert(0, "key", MetadataValue::Integer(42)).unwrap();

        assert_eq!(store1, store2);
    }

    // =========================================================================
    // INSERT Tests
    // =========================================================================

    #[test]
    fn test_insert_and_get() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".to_string()))
            .unwrap();
        assert_eq!(store.get(0, "key").unwrap().as_string(), Some("value"));
    }

    #[test]
    fn test_insert_all_types() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "string", MetadataValue::String("s".to_string()))
            .unwrap();
        store
            .insert(0, "integer", MetadataValue::Integer(42))
            .unwrap();
        store.insert(0, "float", MetadataValue::Float(2.5)).unwrap();
        store
            .insert(0, "boolean", MetadataValue::Boolean(true))
            .unwrap();
        store
            .insert(
                0,
                "array",
                MetadataValue::StringArray(vec!["a".to_string()]),
            )
            .unwrap();
        assert_eq!(store.key_count(0), 5);
    }

    #[test]
    fn test_insert_overwrites_existing() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        store.insert(0, "key", MetadataValue::Integer(2)).unwrap();
        assert_eq!(store.get(0, "key").unwrap().as_integer(), Some(2));
        assert_eq!(store.key_count(0), 1); // Still just one key
    }

    #[test]
    fn test_insert_empty_key_fails() {
        let mut store = MetadataStore::new();
        let result = store.insert(0, "", MetadataValue::Integer(1));
        assert!(matches!(result, Err(MetadataError::EmptyKey)));
    }

    #[test]
    fn test_insert_invalid_key_fails() {
        let mut store = MetadataStore::new();
        let result = store.insert(0, "bad-key", MetadataValue::Integer(1));
        assert!(matches!(
            result,
            Err(MetadataError::InvalidKeyFormat { .. })
        ));
    }

    #[test]
    fn test_insert_nan_float_fails() {
        let mut store = MetadataStore::new();
        let result = store.insert(0, "key", MetadataValue::Float(f64::NAN));
        assert!(matches!(result, Err(MetadataError::InvalidFloat { .. })));
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_too_many_keys_error() {
        let mut store = MetadataStore::new();
        for i in 0..MAX_KEYS_PER_VECTOR {
            store
                .insert(0, &format!("key_{i}"), MetadataValue::Integer(i as i64))
                .unwrap();
        }
        let result = store.insert(0, "one_more", MetadataValue::Integer(0));
        assert!(matches!(result, Err(MetadataError::TooManyKeys { .. })));
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn test_insert_at_max_keys_overwrites_ok() {
        let mut store = MetadataStore::new();
        for i in 0..MAX_KEYS_PER_VECTOR {
            store
                .insert(0, &format!("key_{i}"), MetadataValue::Integer(i as i64))
                .unwrap();
        }
        // Overwriting an existing key should still work
        store
            .insert(0, "key_0", MetadataValue::Integer(999))
            .unwrap();
        assert_eq!(store.get(0, "key_0").unwrap().as_integer(), Some(999));
    }

    // =========================================================================
    // UPDATE Tests
    // =========================================================================

    #[test]
    fn test_update_existing() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        store.update(0, "key", MetadataValue::Integer(2)).unwrap();
        assert_eq!(store.get(0, "key").unwrap().as_integer(), Some(2));
    }

    #[test]
    fn test_update_nonexistent_vector_fails() {
        let mut store = MetadataStore::new();
        let result = store.update(0, "key", MetadataValue::Integer(1));
        assert!(matches!(result, Err(MetadataError::VectorNotFound { .. })));
    }

    #[test]
    fn test_update_nonexistent_key_fails() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "other_key", MetadataValue::Integer(1))
            .unwrap();
        let result = store.update(0, "missing_key", MetadataValue::Integer(1));
        assert!(matches!(result, Err(MetadataError::KeyNotFound { .. })));
    }

    #[test]
    fn test_update_with_invalid_value_fails() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        let result = store.update(0, "key", MetadataValue::Float(f64::NAN));
        assert!(matches!(result, Err(MetadataError::InvalidFloat { .. })));
    }

    // =========================================================================
    // GET Tests
    // =========================================================================

    #[test]
    fn test_get_nonexistent_vector() {
        let store = MetadataStore::new();
        assert!(store.get(999, "key").is_none());
    }

    #[test]
    fn test_get_nonexistent_key() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        assert!(store.get(0, "other").is_none());
    }

    #[test]
    fn test_get_all() {
        let mut store = MetadataStore::new();
        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(0, "b", MetadataValue::Integer(2)).unwrap();

        let all = store.get_all(0).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
    }

    #[test]
    fn test_get_all_nonexistent_vector() {
        let store = MetadataStore::new();
        assert!(store.get_all(999).is_none());
    }

    // =========================================================================
    // DELETE Tests
    // =========================================================================

    #[test]
    fn test_delete_returns_true_when_exists() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        assert!(store.delete(0, "key").unwrap());
        assert!(!store.has_key(0, "key"));
    }

    #[test]
    fn test_delete_returns_false_when_not_exists() {
        let mut store = MetadataStore::new();
        assert!(!store.delete(0, "key").unwrap());
    }

    #[test]
    fn test_delete_cleans_up_empty_vector() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        assert_eq!(store.vector_count(), 1);

        store.delete(0, "key").unwrap();
        assert_eq!(store.vector_count(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_delete_all() {
        let mut store = MetadataStore::new();
        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(0, "b", MetadataValue::Integer(2)).unwrap();

        assert!(store.delete_all(0));
        assert!(!store.delete_all(0)); // Already deleted
        assert!(store.is_empty());
    }

    #[test]
    fn test_delete_all_nonexistent_vector() {
        let mut store = MetadataStore::new();
        assert!(!store.delete_all(999));
    }

    // =========================================================================
    // Query Tests
    // =========================================================================

    #[test]
    fn test_has_key() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();

        assert!(store.has_key(0, "key"));
        assert!(!store.has_key(0, "other"));
        assert!(!store.has_key(999, "key"));
    }

    #[test]
    fn test_keys() {
        let mut store = MetadataStore::new();
        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(0, "b", MetadataValue::Integer(2)).unwrap();

        let keys: Vec<_> = store.keys(0).unwrap().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&&"a".to_string()));
        assert!(keys.contains(&&"b".to_string()));
    }

    #[test]
    fn test_keys_nonexistent_vector() {
        let store = MetadataStore::new();
        assert!(store.keys(999).is_none());
    }

    #[test]
    fn test_key_count() {
        let mut store = MetadataStore::new();
        assert_eq!(store.key_count(0), 0);

        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        assert_eq!(store.key_count(0), 1);

        store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
        assert_eq!(store.key_count(0), 2);
    }

    // =========================================================================
    // Store-level Tests
    // =========================================================================

    #[test]
    fn test_vector_count() {
        let mut store = MetadataStore::new();
        assert_eq!(store.vector_count(), 0);

        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        assert_eq!(store.vector_count(), 1);

        store.insert(1, "b", MetadataValue::Integer(2)).unwrap();
        assert_eq!(store.vector_count(), 2);
    }

    #[test]
    fn test_clear() {
        let mut store = MetadataStore::new();
        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(1, "b", MetadataValue::Integer(2)).unwrap();

        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.vector_count(), 0);
    }

    #[test]
    fn test_merge() {
        let mut store1 = MetadataStore::new();
        store1.insert(0, "a", MetadataValue::Integer(1)).unwrap();

        let mut store2 = MetadataStore::new();
        store2.insert(0, "b", MetadataValue::Integer(2)).unwrap();
        store2.insert(1, "c", MetadataValue::Integer(3)).unwrap();

        store1.merge(store2).unwrap();

        assert_eq!(store1.key_count(0), 2);
        assert_eq!(store1.key_count(1), 1);
        assert_eq!(store1.get(0, "a").unwrap().as_integer(), Some(1));
        assert_eq!(store1.get(0, "b").unwrap().as_integer(), Some(2));
        assert_eq!(store1.get(1, "c").unwrap().as_integer(), Some(3));
    }

    #[test]
    fn test_merge_overwrites() {
        let mut store1 = MetadataStore::new();
        store1.insert(0, "key", MetadataValue::Integer(1)).unwrap();

        let mut store2 = MetadataStore::new();
        store2.insert(0, "key", MetadataValue::Integer(2)).unwrap();

        store1.merge(store2).unwrap();
        assert_eq!(store1.get(0, "key").unwrap().as_integer(), Some(2));
    }

    #[test]
    fn test_merge_exceeds_key_limit_fails() {
        let mut store1 = MetadataStore::new();
        let mut store2 = MetadataStore::new();

        // Add 40 keys to vector 0 in store1
        for i in 0..40 {
            store1
                .insert(
                    0,
                    &format!("key_a_{i}"),
                    MetadataValue::Integer(i64::from(i)),
                )
                .unwrap();
        }

        // Add 40 different keys to vector 0 in store2
        for i in 0..40 {
            store2
                .insert(
                    0,
                    &format!("key_b_{i}"),
                    MetadataValue::Integer(i64::from(i)),
                )
                .unwrap();
        }

        // Merge should fail because 40 + 40 = 80 > 64
        let result = store1.merge(store2);
        assert!(matches!(result, Err(MetadataError::TooManyKeys { .. })));

        // Original store should be unchanged (verify &mut self preserves store on error)
        assert_eq!(store1.key_count(0), 40);
        // Verify we can still use the store after failed merge
        assert!(store1.has_key(0, "key_a_0"));
        store1
            .insert(0, "key_a_0", MetadataValue::Integer(999))
            .unwrap();
        assert_eq!(store1.get(0, "key_a_0").unwrap().as_integer(), Some(999));
    }

    #[test]
    fn test_merge_at_limit_with_overlap_succeeds() {
        let mut store1 = MetadataStore::new();
        let mut store2 = MetadataStore::new();

        // Add 64 keys to vector 0 in store1
        for i in 0..64 {
            store1
                .insert(0, &format!("key_{i}"), MetadataValue::Integer(1))
                .unwrap();
        }

        // Add overlapping keys to store2 (same keys, different values)
        for i in 0..10 {
            store2
                .insert(0, &format!("key_{i}"), MetadataValue::Integer(2))
                .unwrap();
        }

        // Merge should succeed because all keys overlap (64 unique keys after merge)
        store1.merge(store2).unwrap();
        assert_eq!(store1.key_count(0), 64);

        // Values should be overwritten
        assert_eq!(store1.get(0, "key_0").unwrap().as_integer(), Some(2));
    }

    #[test]
    fn test_merge_unchecked_allows_exceeding_limit() {
        let mut store1 = MetadataStore::new();
        let mut store2 = MetadataStore::new();

        // Add 40 keys to vector 0 in each store
        for i in 0..40 {
            store1
                .insert(
                    0,
                    &format!("key_a_{i}"),
                    MetadataValue::Integer(i64::from(i)),
                )
                .unwrap();
            store2
                .insert(
                    0,
                    &format!("key_b_{i}"),
                    MetadataValue::Integer(i64::from(i)),
                )
                .unwrap();
        }

        // merge_unchecked allows exceeding the limit
        store1.merge_unchecked(store2);
        assert_eq!(store1.key_count(0), 80); // Exceeds limit but allowed
    }

    #[test]
    fn test_vector_ids() {
        let mut store = MetadataStore::new();
        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(5, "b", MetadataValue::Integer(2)).unwrap();

        let ids: Vec<_> = store.vector_ids().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&&0));
        assert!(ids.contains(&&5));
    }

    #[test]
    fn test_total_key_count() {
        let mut store = MetadataStore::new();
        assert_eq!(store.total_key_count(), 0);

        store.insert(0, "a", MetadataValue::Integer(1)).unwrap();
        store.insert(0, "b", MetadataValue::Integer(2)).unwrap();
        store.insert(1, "c", MetadataValue::Integer(3)).unwrap();

        assert_eq!(store.total_key_count(), 3);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_serialization_roundtrip() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "key", MetadataValue::String("value".to_string()))
            .unwrap();
        store.insert(1, "num", MetadataValue::Integer(42)).unwrap();

        let json = serde_json::to_string(&store).unwrap();
        let loaded: MetadataStore = serde_json::from_str(&json).unwrap();

        assert_eq!(store.get(0, "key"), loaded.get(0, "key"));
        assert_eq!(store.get(1, "num"), loaded.get(1, "num"));
        assert_eq!(store, loaded);
    }

    #[test]
    fn test_serialization_all_types() {
        let mut store = MetadataStore::new();
        store
            .insert(0, "string", MetadataValue::String("hello".to_string()))
            .unwrap();
        store
            .insert(0, "integer", MetadataValue::Integer(-42))
            .unwrap();
        store
            .insert(0, "float", MetadataValue::Float(1.23456))
            .unwrap();
        store
            .insert(0, "boolean", MetadataValue::Boolean(true))
            .unwrap();
        store
            .insert(
                0,
                "array",
                MetadataValue::StringArray(vec!["a".to_string(), "b".to_string()]),
            )
            .unwrap();

        let json = serde_json::to_string(&store).unwrap();
        let loaded: MetadataStore = serde_json::from_str(&json).unwrap();

        assert_eq!(store, loaded);
    }

    #[test]
    fn test_serialization_empty_store() {
        let store = MetadataStore::new();
        let json = serde_json::to_string(&store).unwrap();
        let loaded: MetadataStore = serde_json::from_str(&json).unwrap();
        assert!(loaded.is_empty());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_insert_then_get(vector_id: u32, key in "[a-z_]{1,10}", value: i64) {
            let mut store = MetadataStore::new();
            store.insert(vector_id, &key, MetadataValue::Integer(value)).unwrap();
            prop_assert_eq!(store.get(vector_id, &key).unwrap().as_integer(), Some(value));
        }

        #[test]
        fn prop_insert_then_delete(vector_id: u32, key in "[a-z_]{1,10}") {
            let mut store = MetadataStore::new();
            store.insert(vector_id, &key, MetadataValue::Boolean(true)).unwrap();
            store.delete(vector_id, &key).unwrap();
            prop_assert!(!store.has_key(vector_id, &key));
        }

        #[test]
        fn prop_delete_cleans_empty_vectors(vector_id: u32, key in "[a-z_]{1,10}") {
            let mut store = MetadataStore::new();
            store.insert(vector_id, &key, MetadataValue::Integer(1)).unwrap();
            store.delete(vector_id, &key).unwrap();
            prop_assert!(store.is_empty());
        }

        #[test]
        fn prop_serialization_roundtrip(vector_id: u32, key in "[a-z_]{1,10}", value: i64) {
            let mut store = MetadataStore::new();
            store.insert(vector_id, &key, MetadataValue::Integer(value)).unwrap();

            let json = serde_json::to_string(&store).unwrap();
            let loaded: MetadataStore = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(store, loaded);
        }

        #[test]
        fn prop_key_count_accurate(vector_id: u32, keys in proptest::collection::vec("[a-z]{1,5}", 1..10)) {
            let mut store = MetadataStore::new();
            let mut unique_keys = std::collections::HashSet::new();

            for key in &keys {
                store.insert(vector_id, key, MetadataValue::Integer(1)).unwrap();
                unique_keys.insert(key.clone());
            }

            prop_assert_eq!(store.key_count(vector_id), unique_keys.len());
        }

        #[test]
        fn prop_update_preserves_other_keys(
            vector_id: u32,
            key1 in "[a-z]{1,5}",
            key2 in "[a-z]{6,10}",  // Ensure different from key1
            v1: i64,
            v2: i64,
            v3: i64
        ) {
            let mut store = MetadataStore::new();
            store.insert(vector_id, &key1, MetadataValue::Integer(v1)).unwrap();
            store.insert(vector_id, &key2, MetadataValue::Integer(v2)).unwrap();
            store.update(vector_id, &key1, MetadataValue::Integer(v3)).unwrap();

            // key2 should be unchanged
            prop_assert_eq!(store.get(vector_id, &key2).unwrap().as_integer(), Some(v2));
            // key1 should be updated
            prop_assert_eq!(store.get(vector_id, &key1).unwrap().as_integer(), Some(v3));
        }
    }
}
