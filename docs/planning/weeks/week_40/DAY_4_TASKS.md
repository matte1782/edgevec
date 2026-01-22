# Week 40 Day 4: Persistence & Metadata

**Date:** 2026-02-06
**Focus:** Snapshot format, save/load, metadata integration
**Estimated Duration:** 5 hours
**Phase:** RFC-008 Phase 4 (Persistence)
**Dependencies:** Day 2 COMPLETE (Search implementation)

---

## Context

Day 4 implements persistence for FlatIndex, enabling:
- Save to IndexedDB (WASM) or file system (native)
- Load from snapshot
- Metadata storage integration

**Architecture Reference:**
- Reuse existing persistence format from HNSW
- Add FlatIndex variant to snapshot format
- Maintain backward compatibility

---

## Tasks

### W40.4.1: Extend Snapshot Format

**Objective:** Add FlatIndex variant to the snapshot format.

**File:** `src/persistence/format.rs` (or equivalent)

```rust
/// Snapshot format version for FlatIndex.
pub const FLAT_INDEX_VERSION: u32 = 1;

/// Magic number for FlatIndex snapshots.
pub const FLAT_INDEX_MAGIC: [u8; 4] = [b'E', b'V', b'F', b'I']; // EdgeVec Flat Index

/// FlatIndex snapshot header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatIndexHeader {
    /// Magic number
    pub magic: [u8; 4],

    /// Format version
    pub version: u32,

    /// Vector dimension
    pub dimensions: u32,

    /// Distance metric
    pub metric: Metric,

    /// Number of vectors (including deleted)
    pub count: u64,

    /// Number of deleted vectors
    pub delete_count: usize,

    /// Next ID to assign
    pub next_id: u64,

    /// Whether quantization is enabled
    pub is_quantized: bool,

    /// Cleanup threshold
    pub cleanup_threshold: f32,

    /// Checksum of data section
    pub checksum: u32,
}

impl FlatIndexHeader {
    /// Create header from FlatIndex.
    pub fn from_index(index: &FlatIndex, checksum: u32) -> Self {
        Self {
            magic: FLAT_INDEX_MAGIC,
            version: FLAT_INDEX_VERSION,
            dimensions: index.config.dimensions,
            metric: index.config.metric,
            count: index.count,
            delete_count: index.delete_count,
            next_id: index.next_id,
            is_quantized: index.quantized.is_some(),
            cleanup_threshold: index.config.cleanup_threshold,
            checksum,
        }
    }

    /// Validate header.
    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.magic != FLAT_INDEX_MAGIC {
            return Err(PersistenceError::InvalidMagic);
        }
        if self.version > FLAT_INDEX_VERSION {
            return Err(PersistenceError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}
```

**Acceptance Criteria:**
- [ ] `FlatIndexHeader` struct defined
- [ ] Magic number unique to FlatIndex
- [ ] Version field for future compatibility
- [ ] Checksum for integrity validation
- [ ] Header serialization/deserialization works

**Deliverables:**
- `FlatIndexHeader` struct
- Format constants

**Dependencies:** None

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.4.2: Implement to_snapshot()

**Objective:** Serialize FlatIndex to bytes for persistence.

**File:** `src/index/flat.rs`

```rust
use crate::persistence::{FlatIndexHeader, PersistenceError, FLAT_INDEX_MAGIC, FLAT_INDEX_VERSION};

impl FlatIndex {
    /// Serialize the index to a snapshot.
    ///
    /// The snapshot format is:
    /// 1. Header (fixed size, includes checksum)
    /// 2. Deleted bitmap (variable size)
    /// 3. Vectors (n * dim * 4 bytes for F32)
    /// 4. Quantized vectors (optional, n * ceil(dim/8) bytes)
    ///
    /// # Returns
    ///
    /// Bytes that can be written to IndexedDB or file system.
    pub fn to_snapshot(&self) -> Result<Vec<u8>, PersistenceError> {
        let mut buffer = Vec::new();

        // Serialize data sections first to compute checksum
        let deleted_bytes = self.serialize_deleted_bitmap();
        let vectors_bytes = self.serialize_vectors();
        let quantized_bytes = self.serialize_quantized();

        // Compute checksum of data
        let checksum = self.compute_checksum(&deleted_bytes, &vectors_bytes, &quantized_bytes);

        // Create and serialize header
        let header = FlatIndexHeader::from_index(self, checksum);
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        // Write header length (u32) + header
        buffer.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&header_bytes);

        // Write deleted bitmap length + data
        buffer.extend_from_slice(&(deleted_bytes.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&deleted_bytes);

        // Write vectors length + data
        buffer.extend_from_slice(&(vectors_bytes.len() as u64).to_le_bytes());
        buffer.extend_from_slice(&vectors_bytes);

        // Write quantized length + data (0 if not enabled)
        buffer.extend_from_slice(&(quantized_bytes.len() as u64).to_le_bytes());
        if !quantized_bytes.is_empty() {
            buffer.extend_from_slice(&quantized_bytes);
        }

        Ok(buffer)
    }

    fn serialize_deleted_bitmap(&self) -> Vec<u8> {
        // Convert BitVec to bytes
        self.deleted.as_raw_slice().to_vec()
    }

    fn serialize_vectors(&self) -> Vec<u8> {
        // Convert f32 slice to bytes
        let bytes: Vec<u8> = self.vectors
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        bytes
    }

    fn serialize_quantized(&self) -> Vec<u8> {
        self.quantized.clone().unwrap_or_default()
    }

    fn compute_checksum(&self, deleted: &[u8], vectors: &[u8], quantized: &[u8]) -> u32 {
        // Simple CRC32 checksum
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(deleted);
        hasher.update(vectors);
        hasher.update(quantized);
        hasher.finalize()
    }
}
```

**Acceptance Criteria:**
- [ ] `to_snapshot()` returns valid bytes
- [ ] Header includes all index state
- [ ] Deleted bitmap serialized correctly
- [ ] Vectors serialized in consistent format
- [ ] Quantized data included if enabled
- [ ] Checksum computed and stored

**Deliverables:**
- `to_snapshot()` method
- Serialization helpers

**Dependencies:** W40.4.1

**Estimated Duration:** 1 hour

**Agent:** RUST_ENGINEER

---

### W40.4.3: Implement from_snapshot()

**Objective:** Deserialize FlatIndex from snapshot bytes.

**File:** `src/index/flat.rs`

```rust
impl FlatIndex {
    /// Restore index from a snapshot.
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes from `to_snapshot()` or loaded from storage
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Magic number doesn't match
    /// - Version is unsupported
    /// - Checksum doesn't match
    /// - Data is corrupted
    pub fn from_snapshot(data: &[u8]) -> Result<Self, PersistenceError> {
        let mut cursor = 0;

        // Read header length
        if data.len() < 4 {
            return Err(PersistenceError::TruncatedData);
        }
        let header_len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        cursor += 4;

        // Read and parse header
        if data.len() < cursor + header_len {
            return Err(PersistenceError::TruncatedData);
        }
        let header: FlatIndexHeader = bincode::deserialize(&data[cursor..cursor + header_len])
            .map_err(|e| PersistenceError::DeserializationError(e.to_string()))?;
        cursor += header_len;

        // Validate header
        header.validate()?;

        // Read deleted bitmap
        if data.len() < cursor + 4 {
            return Err(PersistenceError::TruncatedData);
        }
        let deleted_len = u32::from_le_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;

        if data.len() < cursor + deleted_len {
            return Err(PersistenceError::TruncatedData);
        }
        let deleted_bytes = &data[cursor..cursor + deleted_len];
        cursor += deleted_len;

        // Read vectors
        if data.len() < cursor + 8 {
            return Err(PersistenceError::TruncatedData);
        }
        let vectors_len = u64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8;

        if data.len() < cursor + vectors_len {
            return Err(PersistenceError::TruncatedData);
        }
        let vectors_bytes = &data[cursor..cursor + vectors_len];
        cursor += vectors_len;

        // Read quantized data
        if data.len() < cursor + 8 {
            return Err(PersistenceError::TruncatedData);
        }
        let quantized_len = u64::from_le_bytes(data[cursor..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8;

        let quantized_bytes = if quantized_len > 0 {
            if data.len() < cursor + quantized_len {
                return Err(PersistenceError::TruncatedData);
            }
            Some(data[cursor..cursor + quantized_len].to_vec())
        } else {
            None
        };

        // Verify checksum
        let computed_checksum = {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(deleted_bytes);
            hasher.update(vectors_bytes);
            if let Some(ref q) = quantized_bytes {
                hasher.update(q);
            }
            hasher.finalize()
        };

        if computed_checksum != header.checksum {
            return Err(PersistenceError::ChecksumMismatch);
        }

        // Reconstruct index
        let config = FlatIndexConfig {
            dimensions: header.dimensions,
            metric: header.metric,
            initial_capacity: header.count as usize,
            cleanup_threshold: header.cleanup_threshold,
        };

        // Deserialize vectors
        let vectors: Vec<f32> = vectors_bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        // Deserialize deleted bitmap
        let deleted = BitVec::from_slice(deleted_bytes);

        Ok(Self {
            config,
            vectors,
            count: header.count,
            deleted,
            delete_count: header.delete_count,
            next_id: header.next_id,
            quantized: quantized_bytes,
        })
    }
}
```

**Acceptance Criteria:**
- [ ] `from_snapshot()` parses header correctly
- [ ] Header validation rejects invalid snapshots
- [ ] Deleted bitmap restored
- [ ] Vectors restored
- [ ] Quantized data restored if present
- [ ] Checksum validated
- [ ] Corrupted data rejected with error

**Deliverables:**
- `from_snapshot()` method

**Dependencies:** W40.4.2

**Estimated Duration:** 1.5 hours

**Agent:** RUST_ENGINEER

---

### W40.4.4: Integration Tests

**Objective:** Validate persistence round-trip.

**File:** `tests/flat_persistence_test.rs`

```rust
//! Integration tests for FlatIndex persistence.

use edgevec::index::{FlatIndex, FlatIndexConfig};
use edgevec::metric::Metric;

#[test]
fn test_snapshot_round_trip() {
    let mut index = FlatIndex::new(FlatIndexConfig::new(64));

    // Insert vectors
    for i in 0..100 {
        let v: Vec<f32> = (0..64).map(|j| (i * 64 + j) as f32 / 1000.0).collect();
        index.insert(&v).unwrap();
    }

    // Save snapshot
    let snapshot = index.to_snapshot().unwrap();

    // Restore from snapshot
    let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

    // Verify state
    assert_eq!(restored.dimensions(), index.dimensions());
    assert_eq!(restored.len(), index.len());
    assert_eq!(restored.metric(), index.metric());

    // Verify vectors
    for i in 0..100 {
        let original = index.get(i as u64).unwrap();
        let restored_vec = restored.get(i as u64).unwrap();
        assert_eq!(original, restored_vec);
    }
}

#[test]
fn test_snapshot_with_deletions() {
    let mut index = FlatIndex::new(FlatIndexConfig::new(16));

    // Insert vectors
    for i in 0..50 {
        index.insert(&vec![i as f32; 16]).unwrap();
    }

    // Delete some
    index.delete(10);
    index.delete(20);
    index.delete(30);

    // Save and restore
    let snapshot = index.to_snapshot().unwrap();
    let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

    // Verify deletions preserved
    assert!(restored.get(10).is_none());
    assert!(restored.get(20).is_none());
    assert!(restored.get(30).is_none());
    assert!(restored.get(0).is_some());
    assert!(restored.get(49).is_some());
}

#[test]
fn test_snapshot_with_quantization() {
    let mut index = FlatIndex::new(FlatIndexConfig::new(128));

    // Insert vectors
    for i in 0..100 {
        let v: Vec<f32> = (0..128).map(|j| if (i + j) % 2 == 0 { 1.0 } else { -1.0 }).collect();
        index.insert(&v).unwrap();
    }

    // Enable quantization
    index.enable_quantization().unwrap();

    // Save and restore
    let snapshot = index.to_snapshot().unwrap();
    let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

    // Verify quantization state
    assert!(restored.is_quantized());

    // Search should work
    let query: Vec<f32> = (0..128).map(|j| if j % 2 == 0 { 1.0 } else { -1.0 }).collect();
    let results = restored.search_quantized(&query, 5).unwrap();
    assert_eq!(results.len(), 5);
}

#[test]
fn test_snapshot_different_metrics() {
    for metric in [Metric::Cosine, Metric::DotProduct, Metric::L2] {
        let config = FlatIndexConfig::new(32).with_metric(metric);
        let mut index = FlatIndex::new(config);

        index.insert(&vec![0.5; 32]).unwrap();

        let snapshot = index.to_snapshot().unwrap();
        let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

        assert_eq!(restored.metric(), metric);
    }
}

#[test]
fn test_snapshot_invalid_magic() {
    let mut data = vec![0u8; 100];
    data[4..8].copy_from_slice(b"XXXX"); // Wrong magic

    let result = FlatIndex::from_snapshot(&data);
    assert!(result.is_err());
}

#[test]
fn test_snapshot_truncated() {
    let mut index = FlatIndex::new(FlatIndexConfig::new(16));
    index.insert(&vec![1.0; 16]).unwrap();

    let snapshot = index.to_snapshot().unwrap();

    // Truncate snapshot
    let truncated = &snapshot[..snapshot.len() / 2];

    let result = FlatIndex::from_snapshot(truncated);
    assert!(result.is_err());
}

#[test]
fn test_snapshot_corrupted_checksum() {
    let mut index = FlatIndex::new(FlatIndexConfig::new(16));
    index.insert(&vec![1.0; 16]).unwrap();

    let mut snapshot = index.to_snapshot().unwrap();

    // Corrupt some data
    if snapshot.len() > 50 {
        snapshot[50] ^= 0xFF;
    }

    let result = FlatIndex::from_snapshot(&snapshot);
    assert!(result.is_err());
}

#[test]
fn test_search_after_restore() {
    let config = FlatIndexConfig::new(64).with_metric(Metric::Cosine);
    let mut index = FlatIndex::new(config);

    // Insert known vectors
    index.insert(&vec![1.0; 64]).unwrap(); // ID 0
    index.insert(&vec![0.5; 64]).unwrap(); // ID 1
    index.insert(&vec![0.0; 64]).unwrap(); // ID 2

    // Save and restore
    let snapshot = index.to_snapshot().unwrap();
    let restored = FlatIndex::from_snapshot(&snapshot).unwrap();

    // Search should return same results
    let query = vec![1.0; 64];
    let original_results = index.search(&query, 3).unwrap();
    let restored_results = restored.search(&query, 3).unwrap();

    assert_eq!(original_results.len(), restored_results.len());
    for (orig, rest) in original_results.iter().zip(restored_results.iter()) {
        assert_eq!(orig.id, rest.id);
        assert!((orig.score - rest.score).abs() < 1e-6);
    }
}
```

**Acceptance Criteria:**
- [ ] `test_snapshot_round_trip` passes
- [ ] `test_snapshot_with_deletions` passes
- [ ] `test_snapshot_with_quantization` passes
- [ ] `test_snapshot_different_metrics` passes
- [ ] `test_snapshot_invalid_magic` passes
- [ ] `test_snapshot_truncated` passes
- [ ] `test_snapshot_corrupted_checksum` passes
- [ ] `test_search_after_restore` passes

**Deliverables:**
- `tests/flat_persistence_test.rs`

**Dependencies:** W40.4.2, W40.4.3

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Verification Strategy

### Integration Tests
- Round-trip save/load
- Preservation of state (deletions, quantization)
- Error handling (corruption, truncation)
- Search consistency after restore

### Manual Verification
- Inspect snapshot file size
- Verify checksum catches corruption

---

## Exit Criteria for Day 4

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Snapshot format defined | Header struct compiles | [ ] |
| to_snapshot() works | Unit tests | [ ] |
| from_snapshot() works | Unit tests | [ ] |
| Checksum validation | Corruption test | [ ] |
| Round-trip preserves data | Integration tests | [ ] |
| Deletions preserved | Integration tests | [ ] |
| Quantization preserved | Integration tests | [ ] |
| 8+ persistence tests | `cargo test` | [ ] |
| Clippy clean | 0 warnings | [ ] |

---

## Technical Notes

### Snapshot Format Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Header Length (u32, 4 bytes)                                    │
├─────────────────────────────────────────────────────────────────┤
│ Header (bincode serialized FlatIndexHeader)                     │
│   - magic: [u8; 4]                                              │
│   - version: u32                                                │
│   - dimensions: u32                                             │
│   - metric: Metric                                              │
│   - count: u64                                                  │
│   - delete_count: usize                                         │
│   - next_id: u64                                                │
│   - is_quantized: bool                                          │
│   - cleanup_threshold: f32                                      │
│   - checksum: u32                                               │
├─────────────────────────────────────────────────────────────────┤
│ Deleted Bitmap Length (u32, 4 bytes)                            │
├─────────────────────────────────────────────────────────────────┤
│ Deleted Bitmap (variable bytes)                                 │
├─────────────────────────────────────────────────────────────────┤
│ Vectors Length (u64, 8 bytes)                                   │
├─────────────────────────────────────────────────────────────────┤
│ Vectors (n * dim * 4 bytes, little-endian f32)                  │
├─────────────────────────────────────────────────────────────────┤
│ Quantized Length (u64, 8 bytes, 0 if not enabled)               │
├─────────────────────────────────────────────────────────────────┤
│ Quantized Vectors (n * ceil(dim/8) bytes, optional)             │
└─────────────────────────────────────────────────────────────────┘
```

### Backward Compatibility

- Version field allows future format changes
- Magic number distinguishes FlatIndex from HNSW snapshots
- Can add migration paths for older versions

---

**Day 4 Total:** 5 hours
**Agent:** RUST_ENGINEER
**Status:** [DRAFT]
