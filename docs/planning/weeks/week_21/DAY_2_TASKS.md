# Week 21, Day 2: Metadata Storage Implementation

**Date:** 2025-12-31
**Sprint:** Week 21 (v0.5.0 Phase)
**Day Theme:** CRUD Operations & Persistence Integration
**Status:** PLANNED

---

## Task W21.2: Metadata Storage CRUD Implementation

**Priority:** CRITICAL (P0)
**Estimated Effort:** 8 hours (3x rule: 2h optimistic × 3 = 6h + 2h buffer)
**Status:** PLANNED
**Depends On:** W21.1 complete (Core types defined)
**Blocks:** W21.3, W21.4, W21.5

---

### Context

Day 2 implements the `MetadataStore` struct with full CRUD operations and integrates with the existing persistence layer. This is the core logic that makes metadata storage functional.

**Strategic Importance:**
- CRUD operations are the user-facing API
- Persistence integration ensures data survives restarts
- Error handling must be production-ready

**Reference Documents:**
- `docs/planning/weeks/week_21/DAY_1_TASKS.md` (prerequisite)
- `src/persistence/snapshot.rs` (integration target)

---

### Objective

Implement the complete `MetadataStore` with:
1. All CRUD operations (insert, get, update, delete)
2. Validation enforcement at API boundary
3. Persistence integration via extended snapshot format
4. Comprehensive error handling

---

### Technical Approach

#### 1. MetadataStore Implementation

**File: `src/metadata/store.rs`**
```rust
use std::collections::HashMap;
use super::types::MetadataValue;
use super::error::MetadataError;
use super::validation::{validate_key, validate_value, MAX_KEYS_PER_VECTOR};
use serde::{Deserialize, Serialize};

/// Storage for vector metadata.
///
/// `MetadataStore` provides a key-value store for attaching metadata
/// to vectors. Each vector can have up to 64 metadata keys.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::{MetadataStore, MetadataValue};
///
/// let mut store = MetadataStore::new();
///
/// // Insert metadata for vector 0
/// store.insert(0, "title", MetadataValue::String("Document".to_string())).unwrap();
/// store.insert(0, "page_count", MetadataValue::Integer(42)).unwrap();
///
/// // Retrieve metadata
/// let title = store.get(0, "title").unwrap();
/// assert_eq!(title.as_string(), Some("Document"));
///
/// // Update metadata
/// store.update(0, "page_count", MetadataValue::Integer(50)).unwrap();
///
/// // Delete metadata
/// store.delete(0, "page_count").unwrap();
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MetadataStore {
    /// Internal storage: VectorId → (Key → Value)
    data: HashMap<u32, HashMap<String, MetadataValue>>,
}

impl MetadataStore {
    /// Creates a new empty metadata store.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Creates a metadata store with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
        }
    }

    /// Inserts a metadata key-value pair for a vector.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Key is empty or invalid format
    /// - Value fails validation (e.g., string too long)
    /// - Vector already has maximum number of keys (64)
    pub fn insert(
        &mut self,
        vector_id: u32,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), MetadataError> {
        validate_key(key)?;
        validate_value(&value)?;

        let entry = self.data.entry(vector_id).or_insert_with(HashMap::new);

        // Check key limit before inserting new key
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

    /// Gets a metadata value for a vector.
    ///
    /// Returns `None` if the vector or key doesn't exist.
    pub fn get(&self, vector_id: u32, key: &str) -> Option<&MetadataValue> {
        self.data.get(&vector_id)?.get(key)
    }

    /// Gets all metadata for a vector.
    ///
    /// Returns `None` if the vector has no metadata.
    pub fn get_all(&self, vector_id: u32) -> Option<&HashMap<String, MetadataValue>> {
        self.data.get(&vector_id)
    }

    /// Updates an existing metadata key-value pair.
    ///
    /// # Errors
    ///
    /// Returns an error if the key doesn't exist for the vector.
    pub fn update(
        &mut self,
        vector_id: u32,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), MetadataError> {
        validate_value(&value)?;

        let entry = self.data.get_mut(&vector_id).ok_or(MetadataError::VectorNotFound { vector_id })?;

        if !entry.contains_key(key) {
            return Err(MetadataError::KeyNotFound {
                vector_id,
                key: key.to_string(),
            });
        }

        entry.insert(key.to_string(), value);
        Ok(())
    }

    /// Deletes a metadata key for a vector.
    ///
    /// Returns `true` if the key existed and was deleted, `false` otherwise.
    pub fn delete(&mut self, vector_id: u32, key: &str) -> Result<bool, MetadataError> {
        let entry = match self.data.get_mut(&vector_id) {
            Some(e) => e,
            None => return Ok(false),
        };

        let removed = entry.remove(key).is_some();

        // Clean up empty entries
        if entry.is_empty() {
            self.data.remove(&vector_id);
        }

        Ok(removed)
    }

    /// Deletes all metadata for a vector.
    ///
    /// Returns `true` if the vector had metadata that was deleted.
    pub fn delete_all(&mut self, vector_id: u32) -> Result<bool, MetadataError> {
        Ok(self.data.remove(&vector_id).is_some())
    }

    /// Checks if a key exists for a vector.
    pub fn has_key(&self, vector_id: u32, key: &str) -> bool {
        self.data
            .get(&vector_id)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
    }

    /// Returns an iterator over all keys for a vector.
    pub fn keys(&self, vector_id: u32) -> Option<impl Iterator<Item = &String>> {
        self.data.get(&vector_id).map(|m| m.keys())
    }

    /// Returns the number of keys for a vector.
    pub fn key_count(&self, vector_id: u32) -> usize {
        self.data.get(&vector_id).map(|m| m.len()).unwrap_or(0)
    }

    /// Returns the total number of vectors with metadata.
    pub fn vector_count(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears all metadata from the store.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Merges another metadata store into this one.
    ///
    /// Existing keys are overwritten by the other store's values.
    pub fn merge(&mut self, other: MetadataStore) {
        for (vector_id, other_metadata) in other.data {
            let entry = self.data.entry(vector_id).or_insert_with(HashMap::new);
            entry.extend(other_metadata);
        }
    }
}
```

#### 2. Persistence Integration

**Update: `src/persistence/snapshot.rs`**

Add metadata to the snapshot format:

```rust
/// Snapshot format version 2 (with metadata support)
pub const SNAPSHOT_VERSION_V2: u32 = 2;

/// Extended snapshot data with metadata
#[derive(Serialize, Deserialize)]
pub struct SnapshotDataV2 {
    pub config: HnswConfig,
    pub storage: VectorStorageData,
    pub graph: HnswGraphData,
    pub metadata: MetadataStore, // NEW
}

impl SnapshotDataV2 {
    /// Creates a new V2 snapshot from components.
    pub fn new(
        config: HnswConfig,
        storage: VectorStorageData,
        graph: HnswGraphData,
        metadata: MetadataStore,
    ) -> Self {
        Self { config, storage, graph, metadata }
    }
}

/// Write a V2 snapshot with metadata.
pub fn write_snapshot_v2<W: Write>(
    index: &HnswIndex,
    storage: &VectorStorage,
    metadata: &MetadataStore,
    writer: &mut W,
) -> Result<(), PersistenceError> {
    // ... implementation
}

/// Read a snapshot, auto-detecting version.
pub fn read_snapshot_auto<R: Read>(
    reader: &mut R,
) -> Result<(HnswIndex, VectorStorage, MetadataStore), PersistenceError> {
    // Read version and dispatch to V1 or V2 reader
    // V1 snapshots return empty MetadataStore for compatibility
}
```

#### 3. HnswIndex Integration

**Update: `src/hnsw/graph.rs`**

Add metadata store field to index:

```rust
pub struct HnswIndex {
    // ... existing fields

    /// Metadata storage for vectors
    pub metadata: MetadataStore,
}

impl HnswIndex {
    /// Inserts metadata for a vector.
    pub fn set_metadata(
        &mut self,
        vector_id: u32,
        key: &str,
        value: MetadataValue,
    ) -> Result<(), EdgeVecError> {
        self.metadata.insert(vector_id, key, value)
            .map_err(EdgeVecError::Metadata)
    }

    /// Gets metadata for a vector.
    pub fn get_metadata(&self, vector_id: u32, key: &str) -> Option<&MetadataValue> {
        self.metadata.get(vector_id, key)
    }
}
```

---

### Acceptance Criteria

**CRITICAL (Must Pass):**
- [ ] `MetadataStore::insert()` works for all 5 value types
- [ ] `MetadataStore::get()` returns correct values
- [ ] `MetadataStore::update()` modifies existing keys
- [ ] `MetadataStore::delete()` removes keys
- [ ] `MetadataStore::delete_all()` removes all metadata for a vector
- [ ] Validation enforced on all insert/update operations
- [ ] `TooManyKeys` error returned when limit exceeded
- [ ] All operations compile with `cargo check`

**MAJOR (Should Pass):**
- [ ] Persistence integration: snapshots include metadata
- [ ] V1 snapshot compatibility: loading V1 returns empty metadata
- [ ] `HnswIndex` integration: convenience methods added
- [ ] Unit test coverage >90% for store.rs
- [ ] Property tests for CRUD operations

**MINOR (Nice to Have):**
- [ ] `merge()` operation for combining stores
- [ ] Benchmarks for CRUD operations
- [ ] Memory usage estimation method

---

### Implementation Checklist

- [ ] Implement `MetadataStore::new()` and `with_capacity()`
- [ ] Implement `insert()` with validation
- [ ] Implement `get()` and `get_all()`
- [ ] Implement `update()` with key existence check
- [ ] Implement `delete()` and `delete_all()`
- [ ] Implement `has_key()`, `keys()`, `key_count()`
- [ ] Implement `vector_count()`, `is_empty()`, `clear()`
- [ ] Add `MetadataStore` to snapshot format
- [ ] Implement `write_snapshot_v2()`
- [ ] Implement `read_snapshot_auto()` with version detection
- [ ] Add `metadata` field to `HnswIndex`
- [ ] Add `set_metadata()` and `get_metadata()` to `HnswIndex`
- [ ] Update `EdgeVecError` with `Metadata` variant
- [ ] Write unit tests for all CRUD operations
- [ ] Write property tests for store invariants
- [ ] Verify `cargo test` passes

---

### Test Requirements

**Unit Tests (Required):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::String("value".to_string())).unwrap();
        assert_eq!(
            store.get(0, "key").unwrap().as_string(),
            Some("value")
        );
    }

    #[test]
    fn test_insert_all_types() {
        let mut store = MetadataStore::new();
        store.insert(0, "string", MetadataValue::String("s".to_string())).unwrap();
        store.insert(0, "integer", MetadataValue::Integer(42)).unwrap();
        store.insert(0, "float", MetadataValue::Float(3.14)).unwrap();
        store.insert(0, "boolean", MetadataValue::Boolean(true)).unwrap();
        store.insert(0, "array", MetadataValue::StringArray(vec!["a".to_string()])).unwrap();
        assert_eq!(store.key_count(0), 5);
    }

    #[test]
    fn test_update_existing() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::Integer(1)).unwrap();
        store.update(0, "key", MetadataValue::Integer(2)).unwrap();
        assert_eq!(store.get(0, "key").unwrap().as_integer(), Some(2));
    }

    #[test]
    fn test_update_nonexistent_fails() {
        let mut store = MetadataStore::new();
        let result = store.update(0, "key", MetadataValue::Integer(1));
        assert!(matches!(result, Err(MetadataError::VectorNotFound { .. })));
    }

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
    fn test_too_many_keys_error() {
        let mut store = MetadataStore::new();
        for i in 0..MAX_KEYS_PER_VECTOR {
            store.insert(0, &format!("key_{}", i), MetadataValue::Integer(i as i64)).unwrap();
        }
        let result = store.insert(0, "one_more", MetadataValue::Integer(0));
        assert!(matches!(result, Err(MetadataError::TooManyKeys { .. })));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut store = MetadataStore::new();
        store.insert(0, "key", MetadataValue::String("value".to_string())).unwrap();
        store.insert(1, "num", MetadataValue::Integer(42)).unwrap();

        let json = serde_json::to_string(&store).unwrap();
        let loaded: MetadataStore = serde_json::from_str(&json).unwrap();

        assert_eq!(store.get(0, "key"), loaded.get(0, "key"));
        assert_eq!(store.get(1, "num"), loaded.get(1, "num"));
    }
}
```

**Property Tests (Required):**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_insert_then_get(vector_id: u32, key in "[a-z_]{1,10}", value: i64) {
        let mut store = MetadataStore::new();
        store.insert(vector_id, &key, MetadataValue::Integer(value)).unwrap();
        prop_assert_eq!(store.get(vector_id, &key).unwrap().as_integer(), Some(value));
    }

    #[test]
    fn prop_delete_removes_key(vector_id: u32, key in "[a-z_]{1,10}") {
        let mut store = MetadataStore::new();
        store.insert(vector_id, &key, MetadataValue::Boolean(true)).unwrap();
        store.delete(vector_id, &key).unwrap();
        prop_assert!(!store.has_key(vector_id, &key));
    }
}
```

**Coverage Target:** >90% line coverage for store.rs

---

### Performance Targets

| Operation | Target | Notes |
|:----------|:-------|:------|
| `insert()` | <500ns | Single key |
| `get()` | <100ns | HashMap lookup |
| `update()` | <500ns | Single key |
| `delete()` | <200ns | Single key |
| `get_all()` | <50ns | Reference return |
| Store serialization (1000 vectors, 10 keys each) | <10ms | Full store |

---

### Documentation Requirements

- [ ] Doc comments for `MetadataStore` struct with usage example
- [ ] Doc comments for all public methods
- [ ] Error documentation for each method
- [ ] Persistence format documentation update

---

### Dependencies

**Blocks:**
- W21.3 (WASM bindings need working CRUD)
- W21.4 (Mobile testing needs working API)
- W21.5 (CI needs everything working)

**Blocked By:**
- W21.1 complete (Core types must exist)

**External Dependencies:**
- None new (uses existing serde, thiserror)

---

### Verification Method

**Day 2 is COMPLETE when:**

1. Run verification commands:
   ```bash
   cargo check
   cargo test --lib metadata
   cargo test --test metadata_integration  # if created
   cargo clippy -- -D warnings
   ```

2. All commands exit with code 0

3. Demonstrate CRUD operations:
   ```rust
   let mut store = MetadataStore::new();
   store.insert(0, "title", MetadataValue::String("Test".to_string()))?;
   assert_eq!(store.get(0, "title").unwrap().as_string(), Some("Test"));
   store.update(0, "title", MetadataValue::String("Updated".to_string()))?;
   store.delete(0, "title")?;
   assert!(!store.has_key(0, "title"));
   ```

---

### Rollback Plan

If Day 2 encounters blocking issues:

1. **Persistence integration fails:** Defer to Day 3, ship metadata as in-memory only
2. **HnswIndex integration fails:** Provide standalone API only
3. **Performance issues:** Optimize after functional correctness
4. **Property test failures:** Fix bugs, don't reduce coverage

---

### Estimated Timeline

| Phase | Time | Cumulative |
|:------|:-----|:-----------|
| MetadataStore CRUD | 3h | 3h |
| Validation integration | 1h | 4h |
| Persistence integration | 2h | 6h |
| Unit tests | 1h | 7h |
| Property tests | 0.5h | 7.5h |
| Buffer | 0.5h | 8h |

---

### Hostile Review Checkpoint

**End of Day 2:** Submit for `/review` with:
- `src/metadata/store.rs`
- Updated `src/persistence/snapshot.rs`
- Updated `src/hnsw/graph.rs` (if integrated)
- Unit tests
- Property tests

**Expected Review Focus:**
- CRUD correctness
- Error handling completeness
- Persistence format compatibility
- Performance characteristics

---

**Task Owner:** RUST_ENGINEER
**Review Required:** HOSTILE_REVIEWER
**Next Task:** W21.3 (WASM Bindings)

---

*"CRUD done right. Persistence done right. Quality always."*
