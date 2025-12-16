//! Metadata storage implementation.
//!
//! This module provides the `MetadataStore` struct for storing key-value
//! metadata attached to vectors. Full implementation in Day 2 (W21.2).
//!
//! # Current Status
//!
//! **Day 1 (W21.1):** Stub with type definition only.
//! **Day 2 (W21.2):** Full CRUD implementation.
//!
//! # Architecture
//!
//! ```text
//! MetadataStore
//! ├── data: HashMap<VectorId, HashMap<Key, MetadataValue>>
//! └── Operations:
//!     ├── insert(vector_id, key, value)
//!     ├── get(vector_id, key)
//!     ├── get_all(vector_id)
//!     ├── delete(vector_id, key)
//!     └── clear(vector_id)
//! ```

use std::collections::HashMap;

use super::types::MetadataValue;

/// Storage for vector metadata.
///
/// `MetadataStore` provides a key-value store for attaching metadata to vectors.
/// Each vector can have up to 64 metadata keys, and values can be one of 5 types.
///
/// # Example (Day 2)
///
/// ```rust,ignore
/// use edgevec::metadata::{MetadataStore, MetadataValue};
///
/// let mut store = MetadataStore::new();
///
/// // Attach metadata to vector 0
/// store.insert(0, "title", MetadataValue::String("Hello".into()))?;
/// store.insert(0, "page", MetadataValue::Integer(42))?;
///
/// // Retrieve metadata
/// let title = store.get(0, "title")?;
/// assert_eq!(title.as_string(), Some("Hello"));
/// ```
#[derive(Debug, Clone, Default)]
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

    /// Returns the number of vectors with metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataStore;
    ///
    /// let store = MetadataStore::new();
    /// assert_eq!(store.vector_count(), 0);
    /// ```
    #[must_use]
    pub fn vector_count(&self) -> usize {
        self.data.len()
    }

    // =========================================================================
    // Day 2 (W21.2) will implement:
    // - insert(&mut self, vector_id: u32, key: &str, value: MetadataValue) -> Result<()>
    // - get(&self, vector_id: u32, key: &str) -> Option<&MetadataValue>
    // - get_all(&self, vector_id: u32) -> Option<&HashMap<String, MetadataValue>>
    // - delete(&mut self, vector_id: u32, key: &str) -> Result<bool>
    // - clear(&mut self, vector_id: u32)
    // - has_key(&self, vector_id: u32, key: &str) -> bool
    // =========================================================================
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store_is_empty() {
        let store = MetadataStore::new();
        assert!(store.is_empty());
        assert_eq!(store.vector_count(), 0);
    }

    #[test]
    fn test_default_trait() {
        let store: MetadataStore = MetadataStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn test_clone() {
        let store = MetadataStore::new();
        let cloned = store.clone();
        assert!(cloned.is_empty());
    }

    #[test]
    fn test_debug() {
        let store = MetadataStore::new();
        let debug_str = format!("{store:?}");
        assert!(debug_str.contains("MetadataStore"));
    }
}
