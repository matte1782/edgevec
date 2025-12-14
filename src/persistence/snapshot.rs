use crate::hnsw::graph::{HnswIndex, HnswNode, NodeId};
use crate::hnsw::HnswConfig;
use crate::persistence::chunking::ChunkedWriter;
use crate::persistence::header::{FileHeader, HeaderError};
use crate::persistence::storage::load_snapshot;
use crate::persistence::{PersistenceError, StorageBackend};
use crate::storage::VectorStorage;
use bitvec::prelude::*;
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
pub fn read_snapshot(
    backend: &dyn StorageBackend,
) -> Result<(HnswIndex, VectorStorage), PersistenceError> {
    // 1. Load Data (Verifies Checksum and Header)
    let (header, data) = load_snapshot(backend)?;

    // Verify Flags (C3)
    // We currently don't support any flags (like quantization).
    // If flags are set, it implies data format we don't understand.
    if header.flags != 0 {
        return Err(PersistenceError::Corrupted(format!(
            "Unsupported flags: 0x{:x}. This version only supports flags=0.",
            header.flags
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
    let floats: &[f32] = bytemuck::cast_slice(vector_bytes);
    storage.data_f32.extend_from_slice(floats);

    // Reconstruct 'deleted' bitvec
    #[cfg(test)]
    println!(
        "DEBUG: read_snapshot: metadata_offset in header = {}",
        header.metadata_offset
    );

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
            #[cfg(test)]
            println!(
                "DEBUG: read_snapshot: meta_offset {} > data.len() {}. Fallback.",
                meta_offset,
                data.len()
            );
            for _ in 0..vec_count {
                storage.deleted.push(false);
            }
        }
    } else {
        // Legacy/Fallback: assume all active
        #[cfg(test)]
        println!(
            "DEBUG: read_snapshot: Fallback path, vec_count={}",
            vec_count
        );
        for _ in 0..vec_count {
            storage.deleted.push(false);
        }
    }

    #[cfg(test)]
    println!(
        "DEBUG: read_snapshot: storage.deleted.len() after reconstruction = {}",
        storage.deleted.len()
    );

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

    // Parse Nodes
    //
    // SAFETY WARNING: UNSOUND — This block is known to have undefined behavior.
    // The pointer cast from `*const u8` to `*const HnswNode` does NOT verify
    // alignment. `HnswNode` requires 8-byte alignment (due to VectorId(u64)),
    // but `nodes_bytes` is an arbitrary byte slice with alignment 1.
    //
    // This WILL cause UB on ARM and WASM strict mode if the slice happens
    // to be misaligned. On x86_64 it may appear to work due to lenient
    // alignment handling, but it is still technically undefined behavior.
    //
    // RESOLUTION: This will be replaced with `bytemuck::try_cast_slice()` in
    // task W13.2, which verifies alignment at runtime. See RFC-001 and
    // `docs/audits/unsafe_audit_persistence.md` for details.
    //
    // Tracked: https://github.com/[repo]/issues/XX (replace with actual issue)
    #[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
    let nodes: &[HnswNode] = unsafe {
        let ptr = nodes_bytes.as_ptr() as *const HnswNode;
        std::slice::from_raw_parts(ptr, vec_count)
    };

    // Construct Index
    let mut index =
        HnswIndex::new(config, &storage).map_err(|e| PersistenceError::Corrupted(e.to_string()))?;

    // Restore nodes
    index.nodes = nodes.to_vec();

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

    Ok((index, storage))
}
