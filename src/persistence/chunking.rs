//! Storage chunking logic for incremental persistence.
//!
//! This module provides the [`ChunkedWriter`] trait and implementation
//! to serialize the database into small, manageable chunks suitable for
//! environment-constrained I/O (e.g., IndexedDB in browsers).

use crate::hnsw::HnswIndex;
use crate::persistence::header::FileHeader;
use crate::storage::VectorStorage;
use std::cmp::min;

/// A trait for exporting data in chunks.
pub trait ChunkedWriter {
    /// Returns an iterator that yields chunks of serialized data.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum size of each chunk in bytes.
    fn export_chunked(&self, chunk_size: usize) -> ChunkIter<'_>;
}

/// Iterator that yields serialized database chunks.
///
/// This iterator manages the state machine for serializing:
/// 1. File Header
/// 2. Vector Data
/// 3. HNSW Index Nodes
/// 4. HNSW Neighbor Pool
pub struct ChunkIter<'a> {
    storage: &'a VectorStorage,
    index: &'a HnswIndex,
    chunk_size: usize,
    state: SerializationState,
    buffer: Vec<u8>,

    // Header
    header_bytes: [u8; 64],

    // State tracking
    vector_data_offset: usize, // Offset in f32 slice
    node_index: usize,
    neighbor_offset: usize,
    metadata_offset: usize, // Offset in deleted bits bytes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerializationState {
    Header,
    VectorData,
    IndexNodes,
    IndexNeighbors,
    Metadata,
    Done,
}

impl<'a> ChunkedWriter for (&'a VectorStorage, &'a HnswIndex) {
    fn export_chunked(&self, chunk_size: usize) -> ChunkIter<'a> {
        let (storage, index) = self;
        // Clamp chunk size to ensure header fits (prevent panic in next())
        let chunk_size = std::cmp::max(chunk_size, 64);

        // Calculate offsets for header
        let dimensions = storage.dimensions();
        let vector_count = storage.len() as u64;

        // Size calculations
        let vector_data_size = (storage.raw_data().len() * 4) as u64;
        let nodes_size = (index.node_count() * 16) as u64; // 16 bytes per HnswNode
        let neighbors_size = index.neighbors.buffer.len() as u64;

        // Offsets
        // Header: 0..64
        // Vectors: 64..(64 + vector_data_size)
        let index_offset = 64 + vector_data_size;
        // Metadata: After index nodes + neighbors
        let metadata_offset_start = index_offset + nodes_size + neighbors_size;

        let mut header = FileHeader::new(dimensions);
        header.vector_count = vector_count;
        header.index_offset = index_offset;
        header.metadata_offset = metadata_offset_start;
        header.hnsw_m = index.config.m;
        header.hnsw_m0 = index.config.m0;

        // v0.3: Persist deleted_count from index (W16.5)
        // SAFETY: deleted_count is usize, header field is u32.
        // For indices with > 4B deleted nodes, we'd truncate, but that's
        // practically impossible (memory limits would be hit first).
        #[allow(clippy::cast_possible_truncation)]
        {
            header.deleted_count = index.deleted_count as u32;
        }

        // TODO: RNG seed persistence if needed (index.rng is private or needs exposure?
        // For now we skip RNG state persistence as it is transient/reseeded).

        header.update_checksum();

        ChunkIter {
            storage,
            index,
            chunk_size,
            state: SerializationState::Header,
            buffer: Vec::with_capacity(chunk_size),
            header_bytes: *header.as_bytes(),
            vector_data_offset: 0,
            node_index: 0,
            neighbor_offset: 0,
            metadata_offset: 0,
        }
    }
}

impl Iterator for ChunkIter<'_> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == SerializationState::Done {
            return None;
        }

        self.buffer.clear();

        while self.buffer.len() < self.chunk_size && self.state != SerializationState::Done {
            let space_left = self.chunk_size - self.buffer.len();

            match self.state {
                SerializationState::Header => {
                    let bytes = &self.header_bytes;
                    // Header is small (64 bytes), always fits in chunk_size (unless chunk_size < 64, which is pathological)
                    if space_left >= bytes.len() {
                        self.buffer.extend_from_slice(bytes);
                        self.state = SerializationState::VectorData;
                    } else {
                        // Very small chunk size edge case
                        self.buffer.extend_from_slice(&bytes[..space_left]);
                        // This case is complex to handle with simple state, assume chunk_size >= 64
                        // But for correctness, we should implement offset tracking for header too.
                        // Given constraints (10MB chunks), this is fine.
                        // If strictness required, we'd need header_offset.
                        // SAFETY: Validated in constructor or effectively no-op if caller ignores logic,
                        // but strictly we should not panic. We just stop here and return what we have,
                        // then next call will fail to make progress if chunk_size is permanently < 64.
                        // Actually, let's just force header state to finish if we wrote something,
                        // assuming the caller provided a sane chunk_size.
                        // Better fix: Clamp chunk_size in constructor or return error.
                        // Since we can't change signature of next() to return Result, we accept this edge case
                        // might result in corrupted stream if chunk_size < 64.
                        // But we MUST remove the panic.
                        // Let's just assume we wrote it all for now to avoid panic, or better:
                        // Since we are in a tight loop, we can just error out by finishing early?
                        // No, silence is bad.
                        // Best effort: write partial, but we don't track offset in header_bytes.
                        // So we will just write partial header and move to VectorData? No, that corrupts stream.

                        // Valid Fix: We assume chunk_size >= 64 was checked at creation.
                        // But to satisfy "No Panic", we just return.
                        break;
                    }
                }
                SerializationState::VectorData => {
                    let data = self.storage.raw_data();
                    let remaining_floats = data.len() - self.vector_data_offset;

                    if remaining_floats == 0 {
                        self.state = SerializationState::IndexNodes;
                        continue;
                    }

                    // How many floats can we fit?
                    // space_left bytes / 4 bytes per float
                    let floats_to_copy = min(remaining_floats, space_left / 4);

                    if floats_to_copy > 0 {
                        let end = self.vector_data_offset + floats_to_copy;
                        let slice = &data[self.vector_data_offset..end];

                        // Bulk copy as bytes
                        let byte_slice = bytemuck::cast_slice(slice);
                        self.buffer.extend_from_slice(byte_slice);

                        self.vector_data_offset += floats_to_copy;
                    }

                    // If we couldn't fit even one float but have space (e.g. 1-3 bytes),
                    // we break to yield current chunk and resume next time with fresh buffer.
                    // Or if we finished vectors, move state.
                    if self.vector_data_offset == data.len() {
                        self.state = SerializationState::IndexNodes;
                    } else if floats_to_copy == 0 {
                        // Buffer is full (modulo alignment)
                        break;
                    }
                }
                SerializationState::IndexNodes => {
                    // We need to serialize HnswNode structs.
                    // HnswNode is repr(C) but contains VectorId(u64) etc.
                    // Layout: 16 bytes.
                    let nodes = &self.index.nodes;
                    let remaining_nodes = nodes.len() - self.node_index;

                    if remaining_nodes == 0 {
                        self.state = SerializationState::IndexNeighbors;
                        continue;
                    }

                    // How many nodes fit?
                    // HnswNode size = 16 bytes
                    let nodes_to_copy = min(remaining_nodes, space_left / 16);

                    if nodes_to_copy > 0 {
                        let end = self.node_index + nodes_to_copy;
                        let slice = &nodes[self.node_index..end];

                        // Safe cast using bytemuck: HnswNode → [u8]
                        //
                        // This is safe because:
                        // 1. HnswNode derives Pod (all fields are primitives, no padding gaps)
                        // 2. Casting to u8 always succeeds (u8 has alignment 1)
                        // 3. bytemuck verifies at compile time that HnswNode is Pod
                        //
                        // Fixed in: W13.2 (bytemuck integration)
                        // See: docs/audits/unsafe_audit_persistence.md
                        let byte_slice: &[u8] = bytemuck::cast_slice(slice);
                        self.buffer.extend_from_slice(byte_slice);

                        self.node_index += nodes_to_copy;
                    }

                    if self.node_index == nodes.len() {
                        self.state = SerializationState::IndexNeighbors;
                    } else if nodes_to_copy == 0 {
                        break;
                    }
                }
                SerializationState::IndexNeighbors => {
                    let neighbors = &self.index.neighbors.buffer;
                    let remaining_bytes = neighbors.len() - self.neighbor_offset;

                    if remaining_bytes == 0 {
                        self.state = SerializationState::Done;
                        continue;
                    }

                    let bytes_to_copy = min(remaining_bytes, space_left);
                    let end = self.neighbor_offset + bytes_to_copy;
                    self.buffer
                        .extend_from_slice(&neighbors[self.neighbor_offset..end]);

                    self.neighbor_offset += bytes_to_copy;

                    if self.neighbor_offset == neighbors.len() {
                        self.state = SerializationState::Metadata;
                    } else if bytes_to_copy == 0 {
                        break;
                    }
                }
                SerializationState::Metadata => {
                    // Serialize storage.deleted (BitVec) as bytes
                    // We pack bits into bytes (Lsb0)
                    let total_bits = self.storage.deleted.len();
                    let total_bytes = (total_bits + 7) / 8;

                    let remaining_bytes = total_bytes - self.metadata_offset;

                    // #[cfg(test)]
                    // println!("DEBUG: chunking: Metadata: bits={}, bytes={}, remaining={}, offset={}", total_bits, total_bytes, remaining_bytes, self.metadata_offset);

                    if remaining_bytes == 0 {
                        self.state = SerializationState::Done;
                        continue;
                    }

                    // How many bytes can we fit?
                    let bytes_to_produce = min(remaining_bytes, space_left);

                    // Produce bytes
                    for _ in 0..bytes_to_produce {
                        let byte_idx = self.metadata_offset;
                        let start_bit = byte_idx * 8;
                        let mut byte: u8 = 0;
                        for bit_offset in 0..8 {
                            let bit_idx = start_bit + bit_offset;
                            if bit_idx < total_bits {
                                // We access bitvec by index.
                                // Note: This is random access, not super efficient but acceptable for metadata
                                if self.storage.deleted[bit_idx] {
                                    byte |= 1 << bit_offset;
                                }
                            }
                        }
                        self.buffer.push(byte);
                        self.metadata_offset += 1;
                    }

                    if self.metadata_offset == total_bytes {
                        self.state = SerializationState::Done;
                    } else if bytes_to_produce == 0 {
                        break;
                    }
                }
                SerializationState::Done => break,
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw::{HnswConfig, HnswIndex};
    use crate::storage::VectorStorage;

    #[test]
    fn test_chunked_export_empty() {
        let config = HnswConfig::new(128);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();

        let writer = (&storage, &index);
        let mut iter = writer.export_chunked(1024);

        // Should produce at least one chunk (header)
        let chunk1 = iter.next();
        assert!(chunk1.is_some());
        let data = chunk1.unwrap();
        assert!(data.len() >= 64); // Header

        // Verify Header
        let header = FileHeader::from_bytes(&data[0..64]).unwrap();
        assert_eq!(header.vector_count, 0);
    }

    #[test]
    fn test_chunked_export_data() {
        let config = HnswConfig::new(4); // Small dimension
        let mut storage = VectorStorage::new(&config, None);
        // Insert some vectors
        #[allow(clippy::cast_precision_loss)]
        for i in 0..10 {
            storage.insert(&[i as f32; 4]).unwrap();
        }

        let index = HnswIndex::new(config, &storage).unwrap();
        // (Note: Index is empty of nodes unless we add them, but storage has data)

        let writer = (&storage, &index);

        // Chunk size small enough to split data
        // Header (64) + 1 Vector (16) = 80.
        // Let's use chunk size 70 to force split after header or inside vector data.
        let chunk_size = 70;
        let iter = writer.export_chunked(chunk_size);

        let mut total_bytes = 0;
        for chunk in iter {
            assert!(chunk.len() <= chunk_size);
            total_bytes += chunk.len();
        }

        // Header (64) + 10 * 4 * 4 (160) = 224 bytes
        assert_eq!(total_bytes, 64 + 160);
    }
}
