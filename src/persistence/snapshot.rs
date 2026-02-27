use crate::hnsw::graph::{HnswIndex, HnswNode, NodeId};
use crate::hnsw::HnswConfig;
use crate::metadata::MetadataStore;
use crate::persistence::chunking::ChunkedWriter;
use crate::persistence::header::{FileHeader, Flags, HeaderError, MetadataSectionHeader};
use crate::persistence::storage::load_snapshot;
use crate::persistence::{PersistenceError, StorageBackend};
use crate::storage::VectorStorage;
use bitvec::prelude::*;
use bytemuck::try_cast_slice;
use log::{debug, info, warn};
use std::mem::size_of;

/// Standard chunk size for snapshot streaming (1MB).
/// Kept small to ensure WASM compatibility and low peak memory.
const SNAPSHOT_CHUNK_SIZE: usize = 1024 * 1024;

/// Writes a full snapshot of the index and storage to the backend.
///
/// This operation is atomic: it uses the backend's blob-level `atomic_write`.
///
/// # Arguments
///
/// * `index` - The HNSW index to snapshot.
/// * `storage` - The vector storage to snapshot.
/// * `backend` - The storage backend to write to.
///
/// # Errors
///
/// Returns `PersistenceError` if I/O fails or data cannot be serialized.
pub fn write_snapshot(
    index: &HnswIndex,
    storage: &VectorStorage,
    backend: &mut dyn StorageBackend,
) -> Result<(), PersistenceError> {
    let writer = (storage, index);
    let mut buffer = Vec::new();
    for chunk in writer.export_chunked(SNAPSHOT_CHUNK_SIZE) {
        buffer.extend_from_slice(&chunk);
    }

    if buffer.len() < 64 {
        return Err(PersistenceError::BufferTooSmall {
            expected: 64,
            actual: buffer.len(),
        });
    }

    let mut header_bytes = [0u8; 64];
    header_bytes.copy_from_slice(&buffer[..64]);
    let mut header = FileHeader::from_bytes(&header_bytes)?;

    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&buffer[64..]);
    header.data_crc = hasher.finalize();
    header.update_checksum();

    buffer[..64].copy_from_slice(header.as_bytes());
    backend.atomic_write("", &buffer)
}

/// Reads a snapshot from the backend and reconstructs the index and storage.
///
/// # Arguments
///
/// * `backend` - The storage backend to read from.
///
/// # Returns
///
/// Tuple of `(HnswIndex, VectorStorage)`.
///
/// # Errors
///
/// Returns `PersistenceError` if data is corrupted, version mismatch, or I/O error.
///
/// # Panics
///
/// This function does not panic. All internal `expect` calls are guarded by
/// prior validation that guarantees their conditions are met.
#[allow(clippy::missing_panics_doc)] // Panics are unreachable by design
pub fn read_snapshot(
    backend: &dyn StorageBackend,
) -> Result<(HnswIndex, VectorStorage), PersistenceError> {
    // 1. Load Data (Verifies Checksum and Header)
    let (header, data) = load_snapshot(backend)?;

    // Verify Flags (C3)
    // v0.4: We now support HAS_METADATA flag (bit 2)
    // Other flags (COMPRESSED, QUANTIZED) are still unsupported.
    let supported_flags = Flags::HAS_METADATA;
    let unsupported = header.flags & !supported_flags;
    if unsupported != 0 {
        return Err(PersistenceError::Corrupted(format!(
            "Unsupported flags: 0x{:x}. Supported: 0x{:x} (HAS_METADATA).",
            header.flags, supported_flags
        )));
    }

    // 2. Reconstruct VectorStorage
    // Calculate sizes
    let dim = header.dimensions;
    // SAFETY: On 32-bit targets, vector counts > 2^32 would exceed memory anyway.
    // This cast is intentional and documented.
    #[allow(clippy::cast_possible_truncation)]
    let vec_count = header.vector_count as usize;
    let vec_data_len = vec_count * (dim as usize) * 4;

    // Validate offsets
    // SAFETY: On 32-bit targets, offsets > 2^32 would exceed addressable memory.
    #[allow(clippy::cast_possible_truncation)]
    let index_offset_local = (header.index_offset as usize).saturating_sub(64);
    if index_offset_local > data.len() {
        return Err(PersistenceError::Header(HeaderError::BufferTooShort(
            index_offset_local,
        )));
    }

    // Slice vector data
    let vector_bytes = &data[0..index_offset_local];
    if vector_bytes.len() != vec_data_len {
        return Err(PersistenceError::Corrupted(format!(
            "Vector data length mismatch: expected {}, got {}",
            vec_data_len,
            vector_bytes.len()
        )));
    }

    // Create storage
    let config = HnswConfig::new(dim);
    // Initialize empty storage (no WAL attached yet)
    let mut storage = VectorStorage::new(&config, None);

    // Bulk load vector data
    // Assuming F32 (flags checked above)
    // Note: Use try_cast_slice to handle potential alignment issues gracefully.
    // If the slice is misaligned, fall back to copying byte-by-byte.
    if !vector_bytes.is_empty() {
        match bytemuck::try_cast_slice::<u8, f32>(vector_bytes) {
            Ok(floats) => {
                storage.data_f32.extend_from_slice(floats);
            }
            Err(_) => {
                // Alignment issue - copy manually via LittleEndian reads
                // This is slower but handles arbitrary alignment
                for chunk in vector_bytes.chunks_exact(4) {
                    // SAFETY: chunks_exact(4) guarantees each chunk is exactly 4 bytes,
                    // so try_into() to [u8; 4] is infallible.
                    let bytes: [u8; 4] = chunk.try_into().expect("chunks_exact guarantees 4 bytes");
                    storage.data_f32.push(f32::from_le_bytes(bytes));
                }
            }
        }
    }

    // Reconstruct 'deleted' bitvec
    if header.metadata_offset > 0 {
        // SAFETY: On 32-bit targets, offsets > 2^32 would exceed addressable memory.
        #[allow(clippy::cast_possible_truncation)]
        let meta_offset = (header.metadata_offset as usize).saturating_sub(64);

        // Robustness: Allow meta_offset to equal data.len() (implies 0 bytes of metadata found).
        // If vec_count > 0, we expect bytes, but if they are missing, we treat as "all active"
        // by defaulting to the loop below if slice is empty.
        if meta_offset <= data.len() {
            let deleted_bytes = &data[meta_offset..];
            let deleted_bits = BitVec::<u8, Lsb0>::from_slice(deleted_bytes);

            storage.deleted.clear();
            for i in 0..vec_count {
                if i < deleted_bits.len() {
                    storage.deleted.push(deleted_bits[i]);
                } else {
                    storage.deleted.push(false);
                }
            }
        } else {
            // Offset beyond data length -> Corruption or Truncation.
            // Fallback to all active.
            warn!(
                "Metadata offset {} exceeds data length {}. Treating all vectors as active.",
                meta_offset,
                data.len()
            );
            for _ in 0..vec_count {
                storage.deleted.push(false);
            }
        }
    } else {
        // Legacy/Fallback: assume all active
        for _ in 0..vec_count {
            storage.deleted.push(false);
        }
    }

    storage.next_id = (vec_count as u64) + 1;

    // 3. Reconstruct HnswIndex
    // Index data starts at index_offset_local
    let index_bytes = &data[index_offset_local..];

    // Calculate node size
    let nodes_len_bytes = vec_count * size_of::<HnswNode>();

    // Note: metadata_offset indicates where neighbors end.
    // If metadata_offset > index_offset, we can use it to bound the index data.
    let neighbors_end = if header.metadata_offset > 0 {
        // SAFETY: On 32-bit targets, offsets > 2^32 would exceed addressable memory.
        #[allow(clippy::cast_possible_truncation)]
        let meta_offset_usize = header.metadata_offset as usize;
        meta_offset_usize.saturating_sub(64) - index_offset_local
    } else {
        index_bytes.len()
    };

    if nodes_len_bytes > index_bytes.len() {
        return Err(PersistenceError::Corrupted(
            "Index data too short for nodes".into(),
        ));
    }

    let (nodes_bytes, neighbors_rest) = index_bytes.split_at(nodes_len_bytes);

    // neighbors are from split point to neighbors_end
    // neighbors_rest starts at 0 relative to split.
    // We want up to neighbors_end - nodes_len_bytes.
    let neighbors_len = neighbors_end.saturating_sub(nodes_len_bytes);
    let neighbors_bytes = &neighbors_rest[..std::cmp::min(neighbors_len, neighbors_rest.len())];

    // Parse Nodes using bytemuck for alignment-safe casting.
    //
    // This replaces the previous unsafe pointer cast that had undefined behavior
    // when alignment was not guaranteed. bytemuck::try_cast_slice verifies
    // alignment at runtime and returns an error if misaligned.
    //
    // See: docs/audits/unsafe_audit_persistence.md for the original issue.
    // Fixed in: W13.2 (bytemuck integration)
    // Updated in: W26.5 - handle alignment fallback by copying to aligned Vec
    let nodes: Vec<HnswNode> = if nodes_bytes.is_empty() {
        Vec::new()
    } else if let Ok(nodes) = try_cast_slice::<u8, HnswNode>(nodes_bytes) {
        nodes.to_vec()
    } else {
        // Alignment issue - copy to aligned Vec<u8> first, then cast
        // This is slower but handles arbitrary alignment
        let mut aligned: Vec<u8> = Vec::with_capacity(nodes_bytes.len());
        aligned.extend_from_slice(nodes_bytes);
        // Now aligned should be properly aligned for HnswNode
        try_cast_slice::<u8, HnswNode>(&aligned)
            .map_err(|e| {
                PersistenceError::Corrupted(format!(
                    "HnswNode alignment error after copy: {e:?}. Data may be corrupted."
                ))
            })?
            .to_vec()
    };

    // Verify we got the expected number of nodes
    if nodes.len() != vec_count {
        return Err(PersistenceError::Corrupted(format!(
            "Node count mismatch: expected {}, got {}",
            vec_count,
            nodes.len()
        )));
    }

    // Construct Index
    let mut index =
        HnswIndex::new(config, &storage).map_err(|e| PersistenceError::Corrupted(e.to_string()))?;

    // Restore nodes
    index.nodes = nodes;

    // Restore neighbors
    index.neighbors.buffer.extend_from_slice(neighbors_bytes);

    // Restore max_layer and entry_point
    let mut max_layer = 0;
    let mut entry_point = None;

    for (i, node) in index.nodes.iter().enumerate() {
        if node.max_layer > max_layer {
            max_layer = node.max_layer;
        }
        if node.max_layer == max_layer {
            // SAFETY: Node index fits in u32 (max ~4B nodes, well within limits).
            #[allow(clippy::cast_possible_truncation)]
            let node_id = i as u32;
            entry_point = Some(NodeId(node_id));
        }
    }

    index.max_layer = max_layer;
    index.entry_point = entry_point;

    // v0.3: Restore deleted_count from header or recalculate for older formats
    if header.supports_soft_delete() {
        // Trust header value for v0.3+
        index.deleted_count = header.deleted_count as usize;

        // Verify consistency: count actual deleted nodes
        let actual_deleted = index.nodes.iter().filter(|n| n.deleted != 0).count();
        if actual_deleted != index.deleted_count {
            // Mismatch detected — use actual count for safety
            // This can happen if the snapshot was corrupted or manually edited
            warn!(
                "Snapshot deleted_count mismatch (header={}, actual={}). Using actual count.",
                index.deleted_count, actual_deleted
            );
            index.deleted_count = actual_deleted;
        }
    } else if header.needs_migration() {
        // Migration from v0.1/v0.2: node.pad was always 0, now interpreted as deleted=0
        // All nodes are live in old format, deleted_count = 0
        index.deleted_count = 0;

        info!(
            "Migrated snapshot from v0.{} to v0.3 format (soft delete enabled)",
            header.version_minor
        );
    }

    // v0.4: Load metadata section if HAS_METADATA flag is set
    if header.has_metadata() {
        // Calculate metadata section offset
        // It comes after: vector data + hnsw nodes + neighbors + tombstone bitvec
        let tombstone_bytes = (vec_count + 7) / 8;
        // SAFETY: On 32-bit targets, offsets > 2^32 would exceed addressable memory.
        #[allow(clippy::cast_possible_truncation)]
        let tombstone_offset = (header.metadata_offset as usize).saturating_sub(64);
        let metadata_section_offset = tombstone_offset + tombstone_bytes;

        if metadata_section_offset + 16 > data.len() {
            return Err(PersistenceError::Corrupted(
                "Metadata section header extends beyond file".into(),
            ));
        }

        // Read MetadataSectionHeader (16 bytes)
        let meta_header = MetadataSectionHeader::from_bytes(&data[metadata_section_offset..])
            .map_err(|e| PersistenceError::Corrupted(format!("Invalid metadata header: {e}")))?;

        // Validate CRC before deserializing
        let meta_data_start = metadata_section_offset + 16;
        let meta_data_end = meta_data_start + meta_header.size as usize;

        if meta_data_end > data.len() {
            return Err(PersistenceError::Corrupted(format!(
                "Metadata section data extends beyond file: need {} bytes, have {}",
                meta_data_end,
                data.len()
            )));
        }

        let meta_data = &data[meta_data_start..meta_data_end];
        let actual_crc = crc32fast::hash(meta_data);
        if actual_crc != meta_header.crc {
            return Err(PersistenceError::Corrupted(format!(
                "Metadata CRC mismatch: expected {:#x}, got {:#x}",
                meta_header.crc, actual_crc
            )));
        }

        // Deserialize based on format
        let loaded_metadata = if meta_header.is_postcard() {
            MetadataStore::from_postcard(meta_data).map_err(|e| {
                PersistenceError::Corrupted(format!("Metadata postcard decode failed: {e}"))
            })?
        } else if meta_header.is_json() {
            MetadataStore::from_json(meta_data).map_err(|e| {
                PersistenceError::Corrupted(format!("Metadata JSON decode failed: {e}"))
            })?
        } else {
            return Err(PersistenceError::Corrupted(format!(
                "Unknown metadata format: {}",
                meta_header.format
            )));
        };

        debug!(
            "Loaded metadata section: {} vectors, {} total keys",
            loaded_metadata.vector_count(),
            loaded_metadata.total_key_count()
        );

        index.metadata = loaded_metadata;
    } else {
        // No metadata section (v0.3 or v0.4 without metadata)
        index.metadata = MetadataStore::new();
    }

    Ok((index, storage))
}
