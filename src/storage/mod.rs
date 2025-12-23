//! Vector Storage Module.
//!
//! Handles raw vector data storage, retrieval, and persistence.
//!
//! # Memory Layout
//!
//! - `data`: Flat `Vec<f32>` storing all vector data contiguously.
//! - `offsets`: `Vec<u32>` mapping `VectorId` to `data` index.
//! - `tombstones`: `BitVec` (simulated via `Vec<u8>`) marking deleted vectors.

use crate::hnsw::graph::VectorProvider;
use crate::hnsw::{HnswConfig, VectorId};
use crate::persistence::storage::StorageBackend;
use crate::persistence::wal::{WalAppender, WalError, WalIterator};
use crate::quantization::{QuantizerConfig, ScalarQuantizer};
use bitvec::prelude::*;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Cursor;
use thiserror::Error;

/// Binary vector storage for quantized vectors.
pub mod binary;

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Dimension mismatch between vector and storage config.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimensions.
        expected: u32,
        /// Actual dimensions provided.
        actual: u32,
    },

    /// WAL error: {0}
    #[error("WAL error: {0}")]
    Wal(#[from] WalError),

    /// I/O error during recovery.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid data encountered during recovery.
    #[error("corrupted data: {0}")]
    Corrupted(String),
}

/// Configuration for vector storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum StorageType {
    /// Store full precision f32 vectors.
    #[default]
    Float32,
    /// Store 8-bit quantized vectors.
    QuantizedU8(QuantizerConfig),
}

/// Contiguous vector storage with WAL persistence.
#[derive(Serialize, Deserialize)]
pub struct VectorStorage {
    /// Full precision vector data (layout: [`v0_d0`, ..., `v1_d0`, ...]).
    /// Populated if `storage_type` is `Float32` (or in dual mode).
    #[serde(default)]
    pub(crate) data_f32: Vec<f32>,

    /// Quantized vector data (layout: [`v0_d0`, ..., `v1_d0`, ...]).
    /// Populated if `storage_type` is `QuantizedU8`.
    #[serde(default)]
    pub(crate) quantized_data: Vec<u8>,

    /// Storage configuration.
    #[serde(default)]
    pub(crate) config: StorageType,

    /// Quantizer instance (derived from config).
    #[serde(skip)]
    pub(crate) quantizer: Option<ScalarQuantizer>,

    /// Tombstones for deleted vectors (1 bit per vector).
    pub(crate) deleted: BitVec,
    /// Number of dimensions per vector.
    pub(crate) dimensions: u32,
    /// Write-Ahead Log appender (optional).
    /// Skipped during serialization - must be re-attached after deserialization if needed.
    #[serde(skip)]
    pub(crate) wal: Option<WalAppender>,
    /// Next available ID.
    pub(crate) next_id: u64,
}

impl VectorStorage {
    /// Creates a new `VectorStorage`.
    ///
    /// # Arguments
    ///
    /// * `config` - HNSW configuration defining dimensions.
    /// * `wal` - Optional WAL appender for durability.
    #[must_use]
    pub fn new(config: &HnswConfig, wal: Option<WalAppender>) -> Self {
        Self {
            data_f32: Vec::new(),
            quantized_data: Vec::new(),
            config: StorageType::Float32,
            quantizer: None,
            deleted: BitVec::new(),
            dimensions: config.dimensions,
            wal,
            next_id: 1, // Start at 1 because 0 is reserved sentinel
        }
    }

    /// Set the storage type (e.g. to enable quantization).
    ///
    /// Note: This does not convert existing data. It only affects future inserts.
    pub fn set_storage_type(&mut self, config: StorageType) {
        if let StorageType::QuantizedU8(q_config) = &config {
            self.quantizer = Some(ScalarQuantizer::new(*q_config));
        }
        self.config = config;
    }

    /// Inserts a vector into storage.
    ///
    /// # Durability
    ///
    /// If a WAL is configured, this operation writes to the WAL *before* updating
    /// in-memory state. If WAL write fails, memory is not modified.
    ///
    /// # Arguments
    ///
    /// * `vector` - The vector data slice.
    ///
    /// # Returns
    ///
    /// The new `VectorId` or `StorageError`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if dimensions mismatch or WAL write fails.
    ///
    /// # Panics
    ///
    /// Panics if quantizer is `None` in `QuantizedU8` storage mode (logic error).
    pub fn insert(&mut self, vector: &[f32]) -> Result<VectorId, StorageError> {
        // Step 1: Validate dimensions
        // Check for overflow or mismatch
        if let Ok(len) = u32::try_from(vector.len()) {
            if len != self.dimensions {
                return Err(StorageError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: len,
                });
            }
        } else {
            // If vector length exceeds u32, it definitely doesn't match dimensions (which is u32)
            // We can safely cast to u32 for the error since we know it's too big,
            // but strictly we should handle it.
            return Err(StorageError::DimensionMismatch {
                expected: self.dimensions,
                actual: u32::MAX, // Saturate or similar
            });
        }

        // Step 2: Create Payload
        let id = self.next_id;
        // In a real implementation, we'd serialize the ID + Vector.
        // For this simple WAL payload, let's assume: [u64 ID] + [f32...] (as bytes)
        // But WalEntry stores payload.
        // We need a defined format for the payload.
        // Let's use:
        // [0..8]: ID (u64 LE)
        // [8..]: Vector data (f32 LE)

        // We only write to WAL if it exists
        if let Some(wal) = &mut self.wal {
            let mut payload = Vec::with_capacity(8 + vector.len() * 4);
            payload.extend_from_slice(&id.to_le_bytes());
            for val in vector {
                payload.extend_from_slice(&val.to_le_bytes());
            }

            // Step 3: Append and Sync
            // Entry Type 0 = Insert (F32)
            wal.append(0, &payload)?;
            // wal.sync() is implied by append
        }

        // Step 4: Update Memory
        match &self.config {
            StorageType::Float32 => {
                self.data_f32.extend_from_slice(vector);
            }
            StorageType::QuantizedU8(config) => {
                // Ensure quantizer is initialized
                if self.quantizer.is_none() {
                    self.quantizer = Some(ScalarQuantizer::new(*config));
                }
                let q = self
                    .quantizer
                    .as_ref()
                    .expect("quantizer initialized above");
                let quantized = q.quantize(vector);
                self.quantized_data.extend_from_slice(&quantized);
            }
        }

        self.deleted.push(false);
        self.next_id += 1;

        Ok(VectorId(id))
    }

    /// Inserts a pre-quantized vector into storage.
    ///
    /// # Arguments
    ///
    /// * `data` - The quantized vector data slice.
    ///
    /// # Returns
    ///
    /// The new `VectorId` or `StorageError`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::DimensionMismatch` if `data.len()` doesn't match
    /// the configured dimensions. Returns `StorageError::Corrupted` if storage
    /// is not in quantized mode.
    pub fn insert_quantized(&mut self, data: &[u8]) -> Result<VectorId, StorageError> {
        // Step 1: Validate dimensions
        if let Ok(len) = u32::try_from(data.len()) {
            if len != self.dimensions {
                return Err(StorageError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: len,
                });
            }
        } else {
            return Err(StorageError::DimensionMismatch {
                expected: self.dimensions,
                actual: u32::MAX,
            });
        }

        // Ensure we are in Quantized mode
        if !matches!(self.config, StorageType::QuantizedU8(_)) {
            // Alternatively, we could auto-switch, but strict is better.
            // For now, if we are in Float32, we cannot store quantized data reliably without dequantizing (which we can't do without config).
            // We'll return an error or panic. Let's return error.
            return Err(StorageError::Corrupted(
                "Cannot insert quantized data into Float32 storage".into(),
            ));
        }

        let id = self.next_id;

        if let Some(wal) = &mut self.wal {
            let mut payload = Vec::with_capacity(8 + data.len());
            payload.extend_from_slice(&id.to_le_bytes());
            payload.extend_from_slice(data);

            // Entry Type 1 = Insert Quantized
            wal.append(1, &payload)?;
            // wal.sync() implied
        }

        self.quantized_data.extend_from_slice(data);
        self.deleted.push(false);
        self.next_id += 1;

        Ok(VectorId(id))
    }

    /// Recovers storage state from a WAL backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - The storage backend to read from.
    /// * `config` - HNSW configuration (must match WAL data).
    ///
    /// # Returns
    ///
    /// Restored `VectorStorage`.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if file I/O fails or data is corrupted.
    ///
    /// # Panics
    ///
    /// Panics if internal byte conversions fail (guaranteed safe by length checks).
    #[allow(clippy::needless_pass_by_value)]
    pub fn recover(
        backend: Box<dyn StorageBackend>,
        config: &HnswConfig,
    ) -> Result<Self, StorageError> {
        let mut storage = Self::new(config, None);

        // Read all data from backend
        let data = backend.read().map_err(|e| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
        if data.is_empty() {
            return Ok(storage);
        }

        let reader = Cursor::new(data);
        let iterator = WalIterator::new(reader);

        let mut max_id = 0;

        for result in iterator {
            let (entry, payload) = match result {
                Ok(val) => val,
                Err(WalError::Truncated { expected, actual }) => {
                    // [C1] Handle truncation gracefully (partial write at end of log)
                    eprintln!(
                        "WAL truncated (expected {expected} bytes, got {actual}). Stopping recovery at valid prefix."
                    );
                    break;
                }
                Err(WalError::ChecksumMismatch { expected, actual }) => {
                    // [C1] Handle checksum mismatch.
                    // Assumption: Mismatch implies torn write at the end of the log.
                    // We treat this as a truncation and preserve the valid prefix.
                    // NOTE: If this happens in the middle of a file, we still stop here,
                    // effectively truncating the potentially corrupted tail.
                    eprintln!(
                        "WAL checksum mismatch (expected {expected:#x}, got {actual:#x}). Stopping recovery at valid prefix."
                    );
                    break;
                }
                Err(e) => return Err(StorageError::Wal(e)),
            };

            if entry.entry_type == 0 {
                // Insert (Float32)
                if payload.len() < 8 {
                    return Err(StorageError::Corrupted("Insert payload too short".into()));
                }
                // SAFETY: payload length is checked to be at least 8 in the block ending at line 210.
                let id_bytes: [u8; 8] = payload[0..8].try_into().expect("payload length checked");
                let id = u64::from_le_bytes(id_bytes);

                // Validate ID continuity (simple check for now)
                if id != storage.next_id {
                    // In a strict replay, IDs should match sequence if we only append.
                    // But for now, we trust the WAL ID.
                }

                let vec_bytes = &payload[8..];
                if vec_bytes.len() % 4 != 0 {
                    return Err(StorageError::Corrupted(
                        "Vector bytes alignment error".into(),
                    ));
                }

                let vec_len = vec_bytes.len() / 4;
                if let Ok(len) = u32::try_from(vec_len) {
                    if len != config.dimensions {
                        return Err(StorageError::DimensionMismatch {
                            expected: config.dimensions,
                            actual: len,
                        });
                    }
                } else {
                    return Err(StorageError::DimensionMismatch {
                        expected: config.dimensions,
                        actual: u32::MAX,
                    });
                }

                // Convert bytes back to f32
                let vector: Vec<f32> = vec_bytes
                    .chunks_exact(4)
                    .map(|chunk| {
                        // SAFETY: chunks_exact(4) guarantees each chunk is exactly 4 bytes.
                        let b: [u8; 4] = chunk
                            .try_into()
                            .expect("chunks_exact returns exact size slices");
                        f32::from_le_bytes(b)
                    })
                    .collect();

                // Apply to memory - defaulting to data_f32 because that's what we have from entry_type 0
                storage.data_f32.extend_from_slice(&vector);
                storage.deleted.push(false);
                storage.next_id = id + 1;
                max_id = max_id.max(id);
            } else if entry.entry_type == 1 {
                // Insert Quantized
                if payload.len() < 8 {
                    return Err(StorageError::Corrupted("Insert payload too short".into()));
                }
                let id_bytes: [u8; 8] = payload[0..8].try_into().expect("checked");
                let id = u64::from_le_bytes(id_bytes);

                let vec_bytes = &payload[8..];
                // Check dimensions
                if let Ok(len) = u32::try_from(vec_bytes.len()) {
                    if len != config.dimensions {
                        return Err(StorageError::DimensionMismatch {
                            expected: config.dimensions,
                            actual: len,
                        });
                    }
                }

                storage.quantized_data.extend_from_slice(vec_bytes);
                storage.deleted.push(false);
                storage.next_id = id + 1;
                max_id = max_id.max(id);
            }
        }

        // Truncation repair removed as it's not supported by StorageBackend trait directly.
        // We rely on append-only semantics.

        Ok(storage)
    }

    /// Returns the number of vectors stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.deleted.len()
    }

    /// Returns true if the storage is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deleted.is_empty()
    }

    /// Returns the vector dimensionality.
    #[must_use]
    pub fn dimensions(&self) -> u32 {
        self.dimensions
    }

    /// Returns the vector slice for a given ID.
    ///
    /// # Panics
    ///
    /// Panics if the vector ID is invalid (0).
    /// Panics if data is missing for the configured storage type.
    #[must_use]
    pub fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]> {
        assert!(
            id != VectorId::INVALID,
            "attempted to access invalid vector id 0"
        );
        #[allow(clippy::cast_possible_truncation)]
        // IDs start at 1, so index is id - 1
        let idx = (id.0 as usize) - 1;
        let dim = self.dimensions as usize;
        let start = idx * dim;
        let end = start + dim;

        match &self.config {
            StorageType::Float32 => {
                assert!(
                    !self.data_f32.is_empty(),
                    "get_vector called on storage without f32 data"
                );
                assert!(
                    end <= self.data_f32.len(),
                    "get_vector: VectorId {} out of bounds (idx={}, end={}, data_len={})",
                    id.0,
                    idx,
                    end,
                    self.data_f32.len()
                );
                Cow::Borrowed(&self.data_f32[start..end])
            }
            StorageType::QuantizedU8(_) => {
                assert!(
                    !self.quantized_data.is_empty(),
                    "get_vector called on storage without quantized data"
                );
                assert!(
                    end <= self.quantized_data.len(),
                    "get_vector: VectorId {} out of bounds (idx={}, end={}, data_len={})",
                    id.0,
                    idx,
                    end,
                    self.quantized_data.len()
                );
                let q_data = &self.quantized_data[start..end];
                let q = self
                    .quantizer
                    .as_ref()
                    .expect("quantizer not initialized in QuantizedU8 mode");
                Cow::Owned(q.dequantize(q_data))
            }
        }
    }

    /// Returns the quantized vector slice for a given ID.
    ///
    /// # Panics
    ///
    /// Panics if `quantized_data` is empty.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn get_quantized_vector(&self, id: VectorId) -> &[u8] {
        assert!(
            id != VectorId::INVALID,
            "attempted to access invalid vector id 0"
        );
        let idx = (id.0 as usize) - 1;
        let dim = self.dimensions as usize;
        let start = idx * dim;

        assert!(
            !self.quantized_data.is_empty(),
            "get_quantized_vector called on storage without quantized data"
        );
        &self.quantized_data[start..start + dim]
    }

    /// Marks a vector as deleted.
    ///
    /// # Arguments
    ///
    /// * `id` - The vector ID to delete.
    ///
    /// # Returns
    ///
    /// `true` if the vector was active and is now deleted.
    /// `false` if it was already deleted.
    ///
    /// # Panics
    ///
    /// Panics if the ID is invalid (0). If ID is out of bounds, returns false (robustness).
    #[allow(clippy::cast_possible_truncation)]
    pub fn mark_deleted(&mut self, id: VectorId) -> bool {
        assert!(id != VectorId::INVALID, "invalid vector id 0");
        let idx = (id.0 as usize) - 1;

        // Robustness: If ID is out of bounds, treat as "already deleted" (not found)
        // This allows graph operations to be robust against speculative deletes.
        if idx >= self.deleted.len() {
            return false;
        }

        let was_active = !self.deleted[idx];
        self.deleted.set(idx, true);
        was_active
    }

    /// Checks if a vector is deleted.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn is_deleted(&self, id: VectorId) -> bool {
        if id == VectorId::INVALID {
            return false;
        }
        let idx = (id.0 as usize) - 1;
        if idx >= self.deleted.len() {
            return false;
        }
        self.deleted[idx]
    }

    /// Compacts internal buffers to minimize memory usage.
    pub fn compact(&mut self) {
        self.data_f32.shrink_to_fit();
        self.quantized_data.shrink_to_fit();
        self.deleted.shrink_to_fit();
    }

    /// Returns the raw vector data slice (internal use).
    pub(crate) fn raw_data(&self) -> &[f32] {
        &self.data_f32
    }
}

impl VectorProvider for VectorStorage {
    fn get_vector(&self, id: VectorId) -> Cow<'_, [f32]> {
        self.get_vector(id)
    }

    fn is_deleted(&self, id: VectorId) -> bool {
        self.is_deleted(id)
    }

    fn get_quantized_vector(&self, id: VectorId) -> Option<&[u8]> {
        match self.config {
            StorageType::QuantizedU8(_) => Some(self.get_quantized_vector(id)),
            StorageType::Float32 => None,
        }
    }

    fn quantize_query<'a>(&self, query: &[f32], output: &'a mut Vec<u8>) -> Option<&'a [u8]> {
        match &self.config {
            StorageType::QuantizedU8(_) => {
                if let Some(q) = &self.quantizer {
                    *output = q.quantize(query);
                    Some(output)
                } else {
                    None
                }
            }
            StorageType::Float32 => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_insert_and_retrieve() {
        let config = HnswConfig::new(2); // 2D vectors
        let mut storage = VectorStorage::new(&config, None);

        let vec1 = vec![1.0, 2.0];
        let id1 = storage.insert(&vec1).unwrap();

        assert_eq!(id1.0, 1);
        let retrieved = storage.get_vector(id1);
        assert_eq!(&retrieved[..], &[1.0, 2.0]);
    }

    #[test]
    fn test_dimension_mismatch() {
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);
        let vec = vec![1.0, 2.0, 3.0]; // 3D
        let res = storage.insert(&vec);
        assert!(matches!(res, Err(StorageError::DimensionMismatch { .. })));
    }

    #[test]
    fn test_quantized_storage() {
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);

        let q_config = QuantizerConfig {
            min: 0.0,
            max: 10.0,
        };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let vec = vec![0.0, 10.0];
        let id = storage.insert(&vec).unwrap();

        // Check quantized data
        let q_vec = storage.get_quantized_vector(id);
        assert_eq!(q_vec, &[0, 255]);

        // Check f32 data access works via dequantization (Cow::Owned)
        let vec_out = storage.get_vector(id);
        assert!(matches!(vec_out, Cow::Owned(_)));
        let slice: &[f32] = &vec_out;

        // Quantization introduces error, check proximity
        // 0.0 -> 0 -> 0.0
        // 10.0 -> 255 -> 10.0
        // Should be exact for endpoints in this case
        assert!((slice[0] - 0.0).abs() < 1e-5);
        assert!((slice[1] - 10.0).abs() < 1e-5);
    }

    #[test]
    fn test_insert_quantized() {
        let config = HnswConfig::new(2);
        let mut storage = VectorStorage::new(&config, None);

        let q_config = QuantizerConfig { min: 0.0, max: 1.0 };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let q_vec = vec![0, 255];
        let id = storage.insert_quantized(&q_vec).unwrap();

        assert_eq!(storage.get_quantized_vector(id), &[0, 255]);
    }
}
