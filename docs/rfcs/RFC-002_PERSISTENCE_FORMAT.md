# RFC-002: Persistence Format Design

**Document:** W25.5.3 — Persistence Format Design
**Author:** META_ARCHITECT
**Date:** 2025-12-20
**Status:** [APPROVED]

---

## 1. Current Format (v0.3)

### 1.1 Header Layout

```
FileHeader (64 bytes, aligned to 8)
+-------+-------+------------------+------------------------+
| Offset| Size  | Field            | Description            |
+-------+-------+------------------+------------------------+
| 0x00  | 4     | magic            | "EVEC" [0x45,0x56,0x45,0x43] |
| 0x04  | 1     | version_major    | 0                      |
| 0x05  | 1     | version_minor    | 3                      |
| 0x06  | 2     | flags            | Bit 0: compressed, etc.|
| 0x08  | 8     | vector_count     | Number of vectors      |
| 0x10  | 8     | index_offset     | Byte offset to HNSW    |
| 0x18  | 8     | metadata_offset  | Offset to deleted bits |
| 0x20  | 8     | rng_seed         | RNG state for replay   |
| 0x28  | 4     | dimensions       | Vector dimensionality  |
| 0x2C  | 4     | header_crc       | CRC32 of header        |
| 0x30  | 4     | hnsw_m           | M parameter            |
| 0x34  | 4     | hnsw_m0          | M0 parameter           |
| 0x38  | 4     | data_crc         | CRC32 of all data      |
| 0x3C  | 4     | deleted_count    | Tombstone count        |
+-------+-------+------------------+------------------------+
```

### 1.2 Data Layout

```
+0x00: FileHeader (64 bytes)
+0x40: Vector data
       ├── N vectors × D dimensions × 4 bytes (F32)
       └── Total: N × D × 4 bytes
+index_offset: HNSW Index
       ├── HnswNodes: N × 16 bytes
       └── Neighbors: variable (compressed)
+metadata_offset: Deleted bitvec
       └── N/8 bytes (rounded up)
```

### 1.3 Checksum Strategy

- `header_crc`: CRC32 of header bytes (0-63) with `header_crc` field zeroed
- `data_crc`: CRC32 of all data after header (0x40 onwards)

---

## 2. Proposed Format (v0.4)

### 2.1 Design Goals

1. **Backward Compatibility:** v0.4 reader can load v0.3 files
2. **Forward Compatibility:** v0.3 reader fails gracefully on v0.4 files
3. **Atomic Metadata:** MetadataStore serialized as single blob
4. **Independent CRCs:** Each section has its own checksum
5. **Streaming Support:** Sections can be loaded independently

### 2.2 Header Changes

```
FileHeader v0.4 (64 bytes, unchanged size)
+-------+-------+------------------+------------------------+
| Offset| Size  | Field            | v0.3 → v0.4 Change     |
+-------+-------+------------------+------------------------+
| 0x00  | 4     | magic            | Unchanged              |
| 0x04  | 1     | version_major    | 0 (unchanged)          |
| 0x05  | 1     | version_minor    | 3 → 4                  |
| 0x06  | 2     | flags            | Bit 2: has_metadata    |
| 0x08  | 8     | vector_count     | Unchanged              |
| 0x10  | 8     | index_offset     | Unchanged              |
| 0x18  | 8     | tombstone_offset | Renamed (was metadata) |
| 0x20  | 8     | rng_seed         | Unchanged              |
| 0x28  | 4     | dimensions       | Unchanged              |
| 0x2C  | 4     | header_crc       | Unchanged              |
| 0x30  | 4     | hnsw_m           | Unchanged              |
| 0x34  | 4     | hnsw_m0          | Unchanged              |
| 0x38  | 4     | data_crc         | Unchanged              |
| 0x3C  | 4     | deleted_count    | Unchanged              |
+-------+-------+------------------+------------------------+
```

### 2.3 New Flags

```rust
pub mod Flags {
    pub const COMPRESSED: u16 = 1 << 0;     // Data is compressed
    pub const QUANTIZED: u16 = 1 << 1;      // Vectors are quantized
    pub const HAS_METADATA: u16 = 1 << 2;   // NEW: MetadataStore present
}
```

### 2.4 Data Layout (v0.4)

```
+0x00: FileHeader (64 bytes)
+0x40: Vector data
       └── N × D × 4 bytes (F32)
+index_offset: HNSW Index
       ├── HnswNodes: N × 16 bytes
       └── Neighbors: variable
+tombstone_offset: Deleted bitvec
       └── ceil(N/8) bytes
+after bitvec: MetadataStore Section (if HAS_METADATA flag set)
       ├── metadata_header (16 bytes)
       │   ├── magic: "META" (4 bytes)
       │   ├── version: u16 (2 bytes)
       │   ├── format: u8 (1 byte) — 1=Postcard, 2=JSON
       │   ├── reserved: u8 (1 byte)
       │   ├── size: u32 (4 bytes) — size of serialized data
       │   └── crc: u32 (4 bytes) — CRC of serialized data
       └── serialized_metadata (variable)
```

---

## 3. Metadata Section Format

### 3.1 Section Header

```rust
/// Metadata section header (16 bytes)
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MetadataSectionHeader {
    /// Magic: "META" = [0x4D, 0x45, 0x54, 0x41]
    pub magic: [u8; 4],

    /// Section format version
    pub version: u16,

    /// Serialization format: 1=Postcard, 2=JSON
    pub format: u8,

    /// Reserved for future use
    pub reserved: u8,

    /// Size of serialized metadata in bytes
    pub size: u32,

    /// CRC32 of serialized metadata bytes
    pub crc: u32,
}

const METADATA_MAGIC: [u8; 4] = *b"META";
const METADATA_VERSION: u16 = 1;
const FORMAT_POSTCARD: u8 = 1;
const FORMAT_JSON: u8 = 2;

// Compile-time size and alignment verification
const _: () = assert!(std::mem::size_of::<MetadataSectionHeader>() == 16);
const _: () = assert!(std::mem::align_of::<MetadataSectionHeader>() == 4);
```

### 3.2 Serialization Format

**Primary: Postcard (binary)**
- Compact binary format
- Native Rust serde support
- ~2-3x smaller than JSON
- Faster to serialize/deserialize

**Fallback: JSON (text)**
- Human-readable for debugging
- Interoperability with other tools
- Larger but more portable

```rust
impl MetadataStore {
    /// Serialize to postcard bytes
    pub fn to_postcard(&self) -> Result<Vec<u8>, SerializationError> {
        postcard::to_allocvec(self).map_err(Into::into)
    }

    /// Deserialize from postcard bytes
    pub fn from_postcard(bytes: &[u8]) -> Result<Self, SerializationError> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }

    /// Serialize to JSON bytes
    pub fn to_json(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(Into::into)
    }

    /// Deserialize from JSON bytes
    pub fn from_json(bytes: &[u8]) -> Result<Self, SerializationError> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }
}
```

### 3.3 Size Estimation

For 100K vectors with average 5 keys and 50 bytes per key-value:

| Format | Size | Notes |
|:-------|:-----|:------|
| Postcard | ~12 MB | Compact binary |
| JSON | ~30 MB | Text overhead |
| MessagePack | ~15 MB | Alternative binary |

**Recommendation:** Use Postcard as primary format.

---

## 4. Migration Strategy

### 4.1 Reading v0.3 Files

```rust
pub fn read_snapshot(backend: &dyn StorageBackend)
    -> Result<(HnswIndex, VectorStorage), PersistenceError>
{
    let (header, data) = load_snapshot(backend)?;

    // Check version
    if header.version_minor < 4 {
        // v0.3 format: no metadata section
        let (index, storage) = load_v3_format(&header, &data)?;
        // Return with empty MetadataStore
        return Ok((index.with_empty_metadata(), storage));
    }

    // v0.4+ format
    if header.flags & Flags::HAS_METADATA != 0 {
        // Load metadata section
        let metadata = load_metadata_section(&header, &data)?;
        let (mut index, storage) = load_v4_format(&header, &data)?;
        index.metadata = metadata;
        return Ok((index, storage));
    }

    // v0.4 without metadata
    load_v4_format(&header, &data)
}
```

### 4.2 Writing v0.4 Files

```rust
pub fn write_snapshot(
    index: &HnswIndex,
    storage: &VectorStorage,
    backend: &mut dyn StorageBackend,
) -> Result<(), PersistenceError> {
    // Build header
    let mut header = FileHeader::new(storage.dimensions());
    header.version_minor = 4;

    // Check if metadata exists
    if !index.metadata.is_empty() {
        header.flags |= Flags::HAS_METADATA;
    }

    // Serialize metadata if present
    let metadata_bytes = if !index.metadata.is_empty() {
        Some(serialize_metadata_section(&index.metadata)?)
    } else {
        None
    };

    // Build sections...
    // Write atomically...
}

fn serialize_metadata_section(store: &MetadataStore) -> Result<Vec<u8>, PersistenceError> {
    let serialized = store.to_postcard()?;
    let crc = crc32fast::hash(&serialized);

    let header = MetadataSectionHeader {
        magic: METADATA_MAGIC,
        version: METADATA_VERSION,
        format: FORMAT_POSTCARD,
        reserved: 0,
        size: serialized.len() as u32,
        crc,
    };

    let mut result = Vec::with_capacity(16 + serialized.len());
    result.extend_from_slice(bytemuck::bytes_of(&header));
    result.extend_from_slice(&serialized);

    Ok(result)
}
```

### 4.3 Version Compatibility Matrix

| Writer Version | Reader v0.3 | Reader v0.4 |
|:---------------|:------------|:------------|
| v0.3 | OK | OK (empty metadata) |
| v0.4 (no meta) | FAIL (version) | OK |
| v0.4 (with meta) | FAIL (version) | OK |

**Note:** v0.3 reader will fail on v0.4 files due to version check. This is intentional for safety.

---

## 5. Checksum Strategy

### 5.1 Independent Checksums

| Section | CRC Field | Validated By |
|:--------|:----------|:-------------|
| Header | `header_crc` | `FileHeader::from_bytes()` |
| All Data | `data_crc` | `load_snapshot()` |
| Metadata | `metadata_header.crc` | `load_metadata_section()` |

### 5.2 Partial Validation

For streaming/lazy load scenarios:

```rust
/// Load only header and verify
pub fn validate_header(backend: &dyn StorageBackend) -> Result<FileHeader, PersistenceError> {
    let header_bytes = backend.read(0, 64)?;
    FileHeader::from_bytes(&header_bytes)
}

/// Load only metadata without loading vectors
pub fn load_metadata_only(backend: &dyn StorageBackend) -> Result<MetadataStore, PersistenceError> {
    let header = validate_header(backend)?;

    if header.flags & Flags::HAS_METADATA == 0 {
        return Ok(MetadataStore::new());
    }

    // Calculate metadata offset
    let tombstone_end = header.tombstone_offset as usize + (header.vector_count as usize + 7) / 8;

    // Read metadata section
    let section_bytes = backend.read(tombstone_end, 16)?;
    let section_header = MetadataSectionHeader::from_bytes(&section_bytes)?;

    let data = backend.read(tombstone_end + 16, section_header.size as usize)?;

    // Verify CRC
    let actual_crc = crc32fast::hash(&data);
    if actual_crc != section_header.crc {
        return Err(PersistenceError::Corrupted("Metadata CRC mismatch".into()));
    }

    // Deserialize
    match section_header.format {
        FORMAT_POSTCARD => MetadataStore::from_postcard(&data),
        FORMAT_JSON => MetadataStore::from_json(&data),
        _ => Err(PersistenceError::Corrupted("Unknown metadata format".into())),
    }
}
```

---

## 6. WASM Considerations

### 6.1 IndexedDB Storage

WASM stores snapshots in IndexedDB as blobs. The new format is compatible:

```javascript
// IndexedDB transaction
const tx = db.transaction('snapshots', 'readwrite');
const store = tx.objectStore('snapshots');

// Write snapshot (includes metadata section)
await store.put(snapshotBytes, 'my-index');

// Read snapshot
const bytes = await store.get('my-index');
// Parse returns (HnswIndex with metadata, VectorStorage)
```

### 6.2 Memory Impact

| Vector Count | v0.3 Size | v0.4 Size (no meta) | v0.4 Size (5 keys/vec) |
|:-------------|:----------|:--------------------|:-----------------------|
| 10K | 5.2 MB | 5.2 MB | 6.4 MB (+23%) |
| 50K | 26 MB | 26 MB | 32 MB (+23%) |
| 100K | 52 MB | 52 MB | 64 MB (+23%) |

### 6.3 Load Time Impact

Postcard deserialization is fast. [HYPOTHESIS — ~50 MB/s on WASM, needs benchmarking. Native Rust shows ~1.6 GB/s per [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark), WASM may differ by 2-10x.] For 100K vectors with metadata:

```
Metadata size: ~12 MB
Deserialize time: ~240ms
Total load time impact: +5-10%
```

---

## 7. Error Handling

### 7.1 Error Types

```rust
#[derive(Debug, Error)]
pub enum MetadataSerializationError {
    #[error("Postcard serialization failed: {0}")]
    PostcardError(#[from] postcard::Error),

    #[error("JSON serialization failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid metadata section magic")]
    InvalidMagic,

    #[error("Unsupported metadata version: {0}")]
    UnsupportedVersion(u16),

    #[error("Unknown metadata format: {0}")]
    UnknownFormat(u8),

    #[error("Metadata CRC mismatch: expected {expected:#x}, got {actual:#x}")]
    CrcMismatch { expected: u32, actual: u32 },

    #[error("Metadata section too large: {size} bytes (max: {max})")]
    TooLarge { size: usize, max: usize },
}
```

### 7.2 Recovery Strategies

| Error | Recovery |
|:------|:---------|
| CRC mismatch | Load without metadata, warn user |
| Unknown format | Load without metadata, warn user |
| Too large | Fail with clear error |
| Postcard error | Try JSON fallback (if available) |

---

## 8. Implementation Checklist

- [ ] Add `MetadataSectionHeader` struct
- [ ] Add `Flags::HAS_METADATA` constant
- [ ] Implement `MetadataStore::to_postcard()` / `from_postcard()`
- [ ] Implement `serialize_metadata_section()`
- [ ] Implement `load_metadata_section()`
- [ ] Update `write_snapshot()` for v0.4
- [ ] Update `read_snapshot()` for v0.4
- [ ] Add v0.3 → v0.4 migration tests
- [ ] Add metadata CRC validation tests
- [ ] Update WASM bindings for save/load

---

## 9. Open Questions

1. **Compression:** Should metadata section support zstd compression?
   - Recommendation: Defer to v0.5.0, evaluate need first

2. **Streaming Write:** Should metadata be written incrementally?
   - Recommendation: No, keep atomic write for simplicity

3. **Metadata Limit:** What's the maximum metadata section size?
   - Recommendation: 100 MB (error if exceeded)

---

**Document Status:** [APPROVED]
**Next:** W25.5.4 Draft RFC-002 Complete Document

