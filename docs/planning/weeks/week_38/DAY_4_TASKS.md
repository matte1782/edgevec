# Week 38 Day 4: SparseStorage Serialization

**Date:** 2026-01-22
**Focus:** Implement save/load for SparseStorage with binary format validation
**Estimated Duration:** 3 hours
**Phase:** RFC-007 Implementation Phase 2 (Storage)
**Dependencies:** Day 3 (SparseStorage Core) — MUST BE COMPLETE

---

## Tasks

### W38.4.1: Derive Serialize/Deserialize on SparseStorage

**Objective:** Add serde derives for SparseStorage with custom binary format.

**Rust Implementation:**

```rust
// src/sparse/storage.rs

use serde::{Deserialize, Serialize};

/// Magic number for EdgeVec Sparse Vector format: "ESPV" in ASCII.
pub const SPARSE_MAGIC: [u8; 4] = [b'E', b'S', b'P', b'V'];

/// Current format version. Increment on breaking changes.
pub const SPARSE_FORMAT_VERSION: u32 = 1;

/// Sparse vector storage using packed CSR-like format.
///
/// # Binary Format
///
/// ```text
/// [MAGIC: 4 bytes "ESPV"]
/// [VERSION: 4 bytes u32 LE]
/// [COUNT: 8 bytes u64 LE]
/// [OFFSETS: count * 4 bytes u32 LE]
/// [DIMS: count * 4 bytes u32 LE]
/// [DELETED: (count + 7) / 8 bytes]
/// [NEXT_ID: 8 bytes u64 LE]
/// [TOTAL_NNZ: 8 bytes u64 LE]
/// [INDICES: total_nnz * 4 bytes u32 LE]
/// [VALUES: total_nnz * 4 bytes f32 LE]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseStorage {
    /// Packed indices: all vectors' indices concatenated.
    indices: Vec<u32>,
    /// Packed values: all vectors' values concatenated.
    values: Vec<f32>,
    /// Offsets into packed arrays: offset[i] = start of vector i.
    offsets: Vec<u32>,
    /// Maximum dimension for each stored vector.
    dims: Vec<u32>,
    /// Deletion bitmap: bit i set means vector i is deleted.
    #[serde(with = "bitvec_serde")]
    deleted: BitVec,
    /// Next ID to assign.
    next_id: u64,
}

/// Serde helper for BitVec serialization.
mod bitvec_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(bits: &BitVec, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes: Vec<u8> = bits.iter().enumerate()
            .fold(vec![0u8; (bits.len() + 7) / 8], |mut acc, (i, b)| {
                if *b {
                    acc[i / 8] |= 1 << (i % 8);
                }
                acc
            });
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BitVec, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut bits = BitVec::with_capacity(bytes.len() * 8);
        for byte in bytes {
            for i in 0..8 {
                bits.push((byte >> i) & 1 == 1);
            }
        }
        Ok(bits)
    }
}
```

**Acceptance Criteria:**
- [ ] `SparseStorage` derives `Serialize` and `Deserialize`
- [ ] Magic number constant defined: `SPARSE_MAGIC = [b'E', b'S', b'P', b'V']`
- [ ] Format version constant defined: `SPARSE_FORMAT_VERSION = 1`
- [ ] BitVec has custom serde module
- [ ] Binary format documented in struct docstring

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

---

### W38.4.2: Implement `save(&self, path: &Path) -> Result<(), SparseError>`

**Objective:** Write SparseStorage to disk in binary format with validation header.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

impl SparseStorage {
    /// Save storage to a binary file.
    ///
    /// # Binary Format
    ///
    /// The file starts with a header for validation:
    /// - Magic number (4 bytes): "ESPV"
    /// - Version (4 bytes): u32 little-endian
    ///
    /// Followed by the data:
    /// - Count (8 bytes): u64 little-endian, number of vectors
    /// - Offsets (count * 4 bytes): u32 little-endian per vector
    /// - Dims (count * 4 bytes): u32 little-endian per vector
    /// - Deleted ((count + 7) / 8 bytes): packed bits
    /// - Next ID (8 bytes): u64 little-endian
    /// - Total NNZ (8 bytes): u64 little-endian
    /// - Indices (total_nnz * 4 bytes): u32 little-endian
    /// - Values (total_nnz * 4 bytes): f32 little-endian
    ///
    /// # Errors
    ///
    /// Returns `SparseError::Io` if the file cannot be written.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use edgevec::sparse::SparseStorage;
    /// use std::path::Path;
    ///
    /// let storage = SparseStorage::new();
    /// storage.save(Path::new("sparse.espv"))?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn save(&self, path: &Path) -> Result<(), SparseError> {
        let file = File::create(path).map_err(SparseError::Io)?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(&SPARSE_MAGIC).map_err(SparseError::Io)?;
        writer.write_all(&SPARSE_FORMAT_VERSION.to_le_bytes()).map_err(SparseError::Io)?;

        // Write count
        let count = self.offsets.len() as u64;
        writer.write_all(&count.to_le_bytes()).map_err(SparseError::Io)?;

        // Write offsets
        for offset in &self.offsets {
            writer.write_all(&offset.to_le_bytes()).map_err(SparseError::Io)?;
        }

        // Write dims
        for dim in &self.dims {
            writer.write_all(&dim.to_le_bytes()).map_err(SparseError::Io)?;
        }

        // Write deleted bitmap
        let deleted_bytes = self.pack_deleted_bits();
        writer.write_all(&deleted_bytes).map_err(SparseError::Io)?;

        // Write next_id
        writer.write_all(&self.next_id.to_le_bytes()).map_err(SparseError::Io)?;

        // Write total_nnz
        let total_nnz = self.indices.len() as u64;
        writer.write_all(&total_nnz.to_le_bytes()).map_err(SparseError::Io)?;

        // Write indices
        for idx in &self.indices {
            writer.write_all(&idx.to_le_bytes()).map_err(SparseError::Io)?;
        }

        // Write values
        for val in &self.values {
            writer.write_all(&val.to_le_bytes()).map_err(SparseError::Io)?;
        }

        writer.flush().map_err(SparseError::Io)?;
        Ok(())
    }

    /// Pack deleted bits into bytes.
    fn pack_deleted_bits(&self) -> Vec<u8> {
        let count = self.deleted.len();
        let byte_count = (count + 7) / 8;
        let mut bytes = vec![0u8; byte_count];

        for (i, b) in self.deleted.iter().enumerate() {
            if *b {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }

        bytes
    }
}
```

**Update SparseError for IO:**

```rust
// src/sparse/error.rs (add this variant)

/// IO error during save/load operations.
#[error("IO error: {0}")]
Io(#[from] std::io::Error),
```

**Acceptance Criteria:**
- [ ] `save()` writes valid binary format
- [ ] Magic number "ESPV" at file start
- [ ] Version number written as u32 LE
- [ ] Count written as u64 LE
- [ ] Offsets array written correctly
- [ ] Dims array written correctly
- [ ] Deleted bitmap packed to bytes
- [ ] Next ID written as u64 LE
- [ ] Total NNZ written as u64 LE
- [ ] Indices written as u32 LE array
- [ ] Values written as f32 LE array
- [ ] Uses `BufWriter` for performance
- [ ] `SparseError::Io` variant added

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.4.3: Implement `load(path: &Path) -> Result<Self, SparseError>`

**Objective:** Load SparseStorage from binary file with format validation.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (continued)

use std::io::{BufReader, Read};

impl SparseStorage {
    /// Load storage from a binary file.
    ///
    /// # Format Validation
    ///
    /// - Validates magic number is "ESPV"
    /// - Validates version is compatible (currently only v1)
    ///
    /// # Errors
    ///
    /// - `SparseError::InvalidMagic` if magic number doesn't match
    /// - `SparseError::UnsupportedVersion` if version is not supported
    /// - `SparseError::Io` if file cannot be read
    /// - `SparseError::CorruptedData` if data is inconsistent
    ///
    /// # Example
    ///
    /// ```no_run
    /// use edgevec::sparse::SparseStorage;
    /// use std::path::Path;
    ///
    /// let storage = SparseStorage::load(Path::new("sparse.espv"))?;
    /// # Ok::<(), edgevec::sparse::SparseError>(())
    /// ```
    pub fn load(path: &Path) -> Result<Self, SparseError> {
        let file = File::open(path).map_err(SparseError::Io)?;
        let mut reader = BufReader::new(file);

        // Read and validate magic number
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).map_err(SparseError::Io)?;
        if magic != SPARSE_MAGIC {
            return Err(SparseError::InvalidMagic {
                expected: SPARSE_MAGIC,
                found: magic,
            });
        }

        // Read and validate version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes).map_err(SparseError::Io)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != SPARSE_FORMAT_VERSION {
            return Err(SparseError::UnsupportedVersion {
                expected: SPARSE_FORMAT_VERSION,
                found: version,
            });
        }

        // Read count
        let mut count_bytes = [0u8; 8];
        reader.read_exact(&mut count_bytes).map_err(SparseError::Io)?;
        let count = u64::from_le_bytes(count_bytes) as usize;

        // Read offsets
        let mut offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf).map_err(SparseError::Io)?;
            offsets.push(u32::from_le_bytes(buf));
        }

        // Read dims
        let mut dims = Vec::with_capacity(count);
        for _ in 0..count {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf).map_err(SparseError::Io)?;
            dims.push(u32::from_le_bytes(buf));
        }

        // Read deleted bitmap
        let deleted_byte_count = (count + 7) / 8;
        let mut deleted_bytes = vec![0u8; deleted_byte_count];
        reader.read_exact(&mut deleted_bytes).map_err(SparseError::Io)?;
        let deleted = Self::unpack_deleted_bits(&deleted_bytes, count);

        // Read next_id
        let mut next_id_bytes = [0u8; 8];
        reader.read_exact(&mut next_id_bytes).map_err(SparseError::Io)?;
        let next_id = u64::from_le_bytes(next_id_bytes);

        // Read total_nnz
        let mut total_nnz_bytes = [0u8; 8];
        reader.read_exact(&mut total_nnz_bytes).map_err(SparseError::Io)?;
        let total_nnz = u64::from_le_bytes(total_nnz_bytes) as usize;

        // Read indices
        let mut indices = Vec::with_capacity(total_nnz);
        for _ in 0..total_nnz {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf).map_err(SparseError::Io)?;
            indices.push(u32::from_le_bytes(buf));
        }

        // Read values
        let mut values = Vec::with_capacity(total_nnz);
        for _ in 0..total_nnz {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf).map_err(SparseError::Io)?;
            values.push(f32::from_le_bytes(buf));
        }

        // Validate consistency
        if indices.len() != values.len() {
            return Err(SparseError::CorruptedData {
                message: format!(
                    "indices length {} != values length {}",
                    indices.len(),
                    values.len()
                ),
            });
        }

        Ok(Self {
            indices,
            values,
            offsets,
            dims,
            deleted,
            next_id,
        })
    }

    /// Unpack deleted bits from bytes.
    fn unpack_deleted_bits(bytes: &[u8], count: usize) -> BitVec {
        let mut bits = BitVec::with_capacity(count);
        for i in 0..count {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            let is_deleted = if byte_idx < bytes.len() {
                (bytes[byte_idx] >> bit_idx) & 1 == 1
            } else {
                false
            };
            bits.push(is_deleted);
        }
        bits
    }
}
```

**Update SparseError with new variants:**

```rust
// src/sparse/error.rs (add these variants)

/// Invalid magic number in file header.
#[error("invalid magic number: expected {:?}, found {:?}", expected, found)]
InvalidMagic {
    /// Expected magic bytes.
    expected: [u8; 4],
    /// Found magic bytes.
    found: [u8; 4],
},

/// Unsupported format version.
#[error("unsupported format version: expected {expected}, found {found}")]
UnsupportedVersion {
    /// Expected version number.
    expected: u32,
    /// Found version number.
    found: u32,
},

/// Data corruption detected during load.
#[error("corrupted data: {message}")]
CorruptedData {
    /// Description of the corruption.
    message: String,
},
```

**Acceptance Criteria:**
- [ ] `load()` reads binary format correctly
- [ ] Validates magic number, returns `InvalidMagic` on mismatch
- [ ] Validates version, returns `UnsupportedVersion` on mismatch
- [ ] Reads count as u64 LE
- [ ] Reads offsets array correctly
- [ ] Reads dims array correctly
- [ ] Unpacks deleted bitmap from bytes
- [ ] Reads next_id as u64 LE
- [ ] Reads total_nnz as u64 LE
- [ ] Reads indices as u32 LE array
- [ ] Reads values as f32 LE array
- [ ] Validates indices.len() == values.len()
- [ ] Uses `BufReader` for performance
- [ ] New error variants added: `InvalidMagic`, `UnsupportedVersion`, `CorruptedData`

**Estimated Duration:** 45 minutes

**Agent:** RUST_ENGINEER

---

### W38.4.4: Add Magic Number and Version Header Validation Tests

**Objective:** Test format validation catches invalid files.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (tests module)

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_magic_number_constant() {
        assert_eq!(SPARSE_MAGIC, [b'E', b'S', b'P', b'V']);
        assert_eq!(&SPARSE_MAGIC, b"ESPV");
    }

    #[test]
    fn test_format_version_constant() {
        assert_eq!(SPARSE_FORMAT_VERSION, 1);
    }

    #[test]
    fn test_save_creates_file_with_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let storage = SparseStorage::new();
        storage.save(&path).unwrap();

        // Read first 4 bytes and verify magic
        let mut file = File::open(&path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(magic, SPARSE_MAGIC);
    }

    #[test]
    fn test_save_writes_version() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let storage = SparseStorage::new();
        storage.save(&path).unwrap();

        // Read bytes 4-7 and verify version
        let mut file = File::open(&path).unwrap();
        let mut buf = [0u8; 8];
        file.read_exact(&mut buf).unwrap();
        let version = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(version, SPARSE_FORMAT_VERSION);
    }

    #[test]
    fn test_load_invalid_magic_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with wrong magic
        let mut file = File::create(&path).unwrap();
        file.write_all(b"XXXX").unwrap();
        file.write_all(&1u32.to_le_bytes()).unwrap();
        drop(file);

        let result = SparseStorage::load(&path);
        assert!(matches!(
            result,
            Err(SparseError::InvalidMagic { expected: _, found: _ })
        ));
    }

    #[test]
    fn test_load_unsupported_version_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with unsupported version
        let mut file = File::create(&path).unwrap();
        file.write_all(&SPARSE_MAGIC).unwrap();
        file.write_all(&999u32.to_le_bytes()).unwrap(); // Future version
        drop(file);

        let result = SparseStorage::load(&path);
        assert!(matches!(
            result,
            Err(SparseError::UnsupportedVersion { expected: 1, found: 999 })
        ));
    }

    #[test]
    fn test_load_truncated_file_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        // Write file with valid header but truncated data
        let mut file = File::create(&path).unwrap();
        file.write_all(&SPARSE_MAGIC).unwrap();
        file.write_all(&SPARSE_FORMAT_VERSION.to_le_bytes()).unwrap();
        // Missing count and other data
        drop(file);

        let result = SparseStorage::load(&path);
        assert!(matches!(result, Err(SparseError::Io(_))));
    }

    #[test]
    fn test_pack_unpack_deleted_bits_empty() {
        let storage = SparseStorage::new();
        let packed = storage.pack_deleted_bits();
        assert!(packed.is_empty());

        let unpacked = SparseStorage::unpack_deleted_bits(&packed, 0);
        assert!(unpacked.is_empty());
    }

    #[test]
    fn test_pack_unpack_deleted_bits_roundtrip() {
        let mut storage = SparseStorage::new();

        // Insert some vectors
        let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
        let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
        let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

        storage.insert(&v1).unwrap();
        let id2 = storage.insert(&v2).unwrap();
        storage.insert(&v3).unwrap();

        // Delete one
        storage.delete(id2).unwrap();

        // Pack and unpack
        let packed = storage.pack_deleted_bits();
        let unpacked = SparseStorage::unpack_deleted_bits(&packed, 3);

        assert_eq!(unpacked.len(), 3);
        assert!(!unpacked[0]); // Not deleted
        assert!(unpacked[1]);  // Deleted
        assert!(!unpacked[2]); // Not deleted
    }
}
```

**Acceptance Criteria:**
- [ ] `test_magic_number_constant` verifies "ESPV"
- [ ] `test_format_version_constant` verifies version 1
- [ ] `test_save_creates_file_with_magic` checks file header
- [ ] `test_save_writes_version` checks version in file
- [ ] `test_load_invalid_magic_fails` returns `InvalidMagic`
- [ ] `test_load_unsupported_version_fails` returns `UnsupportedVersion`
- [ ] `test_load_truncated_file_fails` returns `Io` error
- [ ] `test_pack_unpack_deleted_bits_empty` handles empty case
- [ ] `test_pack_unpack_deleted_bits_roundtrip` preserves deletion state

**Estimated Duration:** 30 minutes

**Agent:** TEST_ENGINEER

---

### W38.4.5: Add Roundtrip Tests

**Objective:** Verify save/load preserves all data exactly.

**Rust Implementation:**

```rust
// src/sparse/storage.rs (tests module continued)

#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn test_roundtrip_empty_storage() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let original = SparseStorage::new();
        original.save(&path).unwrap();

        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 0);
        assert_eq!(loaded.next_id(), original.next_id());
    }

    #[test]
    fn test_roundtrip_single_vector() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v = SparseVector::new(vec![0, 5, 10], vec![0.1, 0.2, 0.3], 100).unwrap();
        let id = original.insert(&v).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        let recovered = loaded.get(id).unwrap();
        assert_eq!(recovered.indices(), v.indices());
        assert_eq!(recovered.values(), v.values());
        assert_eq!(recovered.dim(), v.dim());
    }

    #[test]
    fn test_roundtrip_multiple_vectors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let vectors = vec![
            SparseVector::new(vec![0, 5], vec![0.1, 0.2], 100).unwrap(),
            SparseVector::new(vec![1, 2, 3], vec![1.0, 2.0, 3.0], 50).unwrap(),
            SparseVector::singleton(99, 9.9, 100).unwrap(),
        ];

        let ids: Vec<_> = vectors.iter().map(|v| original.insert(v).unwrap()).collect();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.len(), 3);
        for (id, expected) in ids.iter().zip(vectors.iter()) {
            let recovered = loaded.get(*id).unwrap();
            assert_eq!(recovered.indices(), expected.indices());
            assert_eq!(recovered.values(), expected.values());
            assert_eq!(recovered.dim(), expected.dim());
        }
    }

    #[test]
    fn test_roundtrip_with_deletions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v1 = SparseVector::singleton(0, 1.0, 100).unwrap();
        let v2 = SparseVector::singleton(1, 2.0, 100).unwrap();
        let v3 = SparseVector::singleton(2, 3.0, 100).unwrap();

        let id1 = original.insert(&v1).unwrap();
        let id2 = original.insert(&v2).unwrap();
        let id3 = original.insert(&v3).unwrap();

        // Delete middle vector
        original.delete(id2).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        // Active count should be 2
        assert_eq!(loaded.active_count(), 2);

        // id1 and id3 should exist
        assert!(loaded.get(id1).is_some());
        assert!(loaded.get(id2).is_none()); // Deleted
        assert!(loaded.get(id3).is_some());
    }

    #[test]
    fn test_roundtrip_preserves_next_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();
        let v = SparseVector::singleton(0, 1.0, 100).unwrap();
        original.insert(&v).unwrap();
        original.insert(&v).unwrap();
        original.insert(&v).unwrap();

        let expected_next_id = original.next_id();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.next_id(), expected_next_id);
    }

    #[test]
    fn test_roundtrip_large_vectors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();

        // Create vector with 500 non-zeros
        let indices: Vec<u32> = (0..500).map(|i| i * 20).collect();
        let values: Vec<f32> = (0..500).map(|i| i as f32 * 0.01).collect();
        let v = SparseVector::new(indices.clone(), values.clone(), 10000).unwrap();

        let id = original.insert(&v).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        let recovered = loaded.get(id).unwrap();
        assert_eq!(recovered.indices(), &indices);
        assert_eq!(recovered.values(), &values);
        assert_eq!(recovered.nnz(), 500);
    }

    #[test]
    fn test_roundtrip_edge_case_dims() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.espv");

        let mut original = SparseStorage::new();

        // Min dim
        let v1 = SparseVector::singleton(0, 1.0, 1).unwrap();
        // Large dim
        let v2 = SparseVector::singleton(999_999, 1.0, 1_000_000).unwrap();

        let id1 = original.insert(&v1).unwrap();
        let id2 = original.insert(&v2).unwrap();

        original.save(&path).unwrap();
        let loaded = SparseStorage::load(&path).unwrap();

        assert_eq!(loaded.get(id1).unwrap().dim(), 1);
        assert_eq!(loaded.get(id2).unwrap().dim(), 1_000_000);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Property: save/load roundtrip preserves all vector data.
        #[test]
        fn prop_roundtrip_preserves_data(
            nnz in 1usize..100,
            dim in 100u32..1000,
        ) {
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.espv");

            // Generate sorted unique indices
            let indices: Vec<u32> = (0..nnz as u32).map(|i| i * (dim / nnz as u32)).collect();
            let values: Vec<f32> = (0..nnz).map(|i| i as f32 * 0.1).collect();

            let v = SparseVector::new(indices.clone(), values.clone(), dim).unwrap();

            let mut original = SparseStorage::new();
            let id = original.insert(&v).unwrap();

            original.save(&path).unwrap();
            let loaded = SparseStorage::load(&path).unwrap();

            let recovered = loaded.get(id).unwrap();
            prop_assert_eq!(recovered.indices(), v.indices());
            prop_assert_eq!(recovered.dim(), v.dim());

            // Float comparison
            for (a, b) in recovered.values().iter().zip(v.values().iter()) {
                prop_assert!((a - b).abs() < 1e-7);
            }
        }
    }
}
```

**Acceptance Criteria:**
- [ ] `test_roundtrip_empty_storage` handles empty storage
- [ ] `test_roundtrip_single_vector` preserves single vector
- [ ] `test_roundtrip_multiple_vectors` preserves all vectors
- [ ] `test_roundtrip_with_deletions` preserves deletion state
- [ ] `test_roundtrip_preserves_next_id` preserves ID counter
- [ ] `test_roundtrip_large_vectors` handles 500+ nnz
- [ ] `test_roundtrip_edge_case_dims` handles dim=1 and dim=1M
- [ ] `prop_roundtrip_preserves_data` property test (50 cases)
- [ ] All values compared with float tolerance
- [ ] Uses `tempfile` for temp directories

**Estimated Duration:** 30 minutes

**Agent:** TEST_ENGINEER

---

## Day 4 Checklist

- [ ] W38.4.1: Serde derives on SparseStorage
- [ ] W38.4.2: `save()` implementation with binary format
- [ ] W38.4.3: `load()` implementation with validation
- [ ] W38.4.4: Magic number and version tests
- [ ] W38.4.5: Roundtrip tests (unit + property)
- [ ] All tests pass
- [ ] No clippy warnings

---

## Day 4 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| `save()` writes valid format | File starts with "ESPV" + version |
| `load()` reads format correctly | Roundtrip tests pass |
| Invalid magic rejected | `test_load_invalid_magic_fails` passes |
| Invalid version rejected | `test_load_unsupported_version_fails` passes |
| Deletions preserved | `test_roundtrip_with_deletions` passes |
| Large vectors work | 500 nnz roundtrip passes |
| Property tests pass | 50 cases, no failures |
| No clippy warnings | `cargo clippy --features sparse -- -D warnings` |

---

## Day 4 Verification Commands

```bash
# Run all sparse storage tests
cargo test --features sparse sparse::storage -- --nocapture

# Run only roundtrip tests
cargo test --features sparse roundtrip -- --nocapture

# Run property tests with more cases
PROPTEST_CASES=200 cargo test --features sparse prop_roundtrip

# Check binary format manually
hexdump -C sparse.espv | head -5
# Should show: 45 53 50 56 (ESPV) followed by version bytes

# Clippy check
cargo clippy --features sparse -- -D warnings

# Check file sizes make sense
ls -la *.espv
```

---

## Day 4 Handoff

After completing Day 4:

**Artifacts Generated:**
- `src/sparse/storage.rs` (save/load methods)
- `src/sparse/error.rs` (new IO/validation error variants)
- Unit tests for format validation
- Property tests for roundtrip

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 5 — SparseStorage Benchmarks and Optimization

---

## Binary Format Reference

```
Offset  Size     Field         Description
------  -------  -----------   ----------------------------------
0x00    4        MAGIC         "ESPV" (0x45 0x53 0x50 0x56)
0x04    4        VERSION       u32 LE (currently 1)
0x08    8        COUNT         u64 LE (number of vectors)
0x10    count*4  OFFSETS       u32 LE per vector
...     count*4  DIMS          u32 LE per vector
...     (c+7)/8  DELETED       packed bits
...     8        NEXT_ID       u64 LE
...     8        TOTAL_NNZ     u64 LE
...     nnz*4    INDICES       u32 LE per index
...     nnz*4    VALUES        f32 LE per value
```

**Example for 2 vectors:**
- Vector 0: indices=[0, 5], values=[0.1, 0.2], dim=100
- Vector 1: indices=[1, 2, 3], values=[1.0, 2.0, 3.0], dim=50

```
ESPV           45 53 50 56
VERSION=1      01 00 00 00
COUNT=2        02 00 00 00 00 00 00 00
OFFSET[0]=0    00 00 00 00
OFFSET[1]=2    02 00 00 00
DIM[0]=100     64 00 00 00
DIM[1]=50      32 00 00 00
DELETED=0x00   00
NEXT_ID=2      02 00 00 00 00 00 00 00
TOTAL_NNZ=5    05 00 00 00 00 00 00 00
INDICES        00 00 00 00  05 00 00 00  01 00 00 00  02 00 00 00  03 00 00 00
VALUES         cd cc cc 3d  cd cc 4c 3e  00 00 80 3f  00 00 00 40  00 00 40 40
```

---

*Agent: PLANNER*
*Status: [PROPOSED]*
*Date: 2026-01-09*
