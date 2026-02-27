//! Storage chunking logic for incremental persistence.
//!
//! This module provides the [`ChunkedWriter`] trait and implementation
//! to serialize the database into small, manageable chunks suitable for
//! environment-constrained I/O (e.g., IndexedDB in browsers).
//!
//! # Chunk Size Requirements
//!
//! The minimum chunk size is [`MIN_CHUNK_SIZE`] (64 bytes), which is the size
//! of the file header. Chunk sizes smaller than this are automatically clamped
//! to the minimum to ensure the header can be written in a single chunk.

use crate::hnsw::HnswIndex;
use crate::persistence::header::{FileHeader, Flags, MetadataSectionHeader};
use crate::storage::VectorStorage;
use std::cmp::min;

/// Minimum allowed chunk size in bytes.
///
/// This equals the file header size (64 bytes). Smaller chunk sizes would require
/// splitting the header across multiple chunks, which adds complexity without
/// practical benefit (no real-world I/O system has a 64-byte limit).
///
/// If a smaller `chunk_size` is passed to [`ChunkedWriter::export_chunked`],
/// it will be silently clamped to this minimum.
pub const MIN_CHUNK_SIZE: usize = 64;

/// A trait for exporting data in chunks.
pub trait ChunkedWriter {
    /// Returns an iterator that yields chunks of serialized data.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum size of each chunk in bytes. Values below
    ///   [`MIN_CHUNK_SIZE`] (64 bytes) are clamped to the minimum.
    ///
    /// # Chunk Size Constraints
    ///
    /// The minimum effective chunk size is 64 bytes (file header size).
    /// Passing a smaller value will not cause an error; instead, the
    /// implementation clamps it to [`MIN_CHUNK_SIZE`].
    fn export_chunked(&self, chunk_size: usize) -> ChunkIter<'_>;
}

/// Iterator that yields serialized database chunks.
///
/// This iterator manages the state machine for serializing:
/// 1. File Header
/// 2. Vector Data
/// 3. HNSW Index Nodes
/// 4. HNSW Neighbor Pool
/// 5. Tombstone bitvec
/// 6. Metadata section (v0.4+, if non-empty)
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
    tombstone_offset: usize, // Offset in deleted bits bytes (renamed for clarity)
    metadata_section: Vec<u8>, // Pre-serialized metadata section (header + data)
    metadata_section_offset: usize, // Current offset in metadata_section
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SerializationState {
    Header,
    VectorData,
    IndexNodes,
    IndexNeighbors,
    Tombstones,      // Deleted bitvec (was "Metadata" - renamed for clarity)
    MetadataSection, // v0.4+: MetadataSectionHeader + serialized MetadataStore
    Done,
}

impl<'a> ChunkedWriter for (&'a VectorStorage, &'a HnswIndex) {
    fn export_chunked(&self, chunk_size: usize) -> ChunkIter<'a> {
        let (storage, index) = self;
        // Clamp chunk size to minimum (header must fit in one chunk).
        // See MIN_CHUNK_SIZE documentation for rationale.
        let chunk_size = chunk_size.max(MIN_CHUNK_SIZE);

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
        // Tombstone offset: After index nodes + neighbors
        let tombstone_offset_start = index_offset + nodes_size + neighbors_size;

        // v0.4: Serialize metadata section if non-empty (RFC-002)
        let metadata_section = if index.metadata.is_empty() {
            Vec::new()
        } else {
            // Serialize MetadataStore to Postcard
            match index.metadata.to_postcard() {
                Ok(serialized) => {
                    let crc = crc32fast::hash(&serialized);
                    #[allow(clippy::cast_possible_truncation)]
                    let meta_header =
                        MetadataSectionHeader::new_postcard(serialized.len() as u32, crc);

                    // Combine header (16 bytes) + serialized data
                    let mut section = Vec::with_capacity(16 + serialized.len());
                    section.extend_from_slice(meta_header.as_bytes());
                    section.extend_from_slice(&serialized);
                    section
                }
                Err(_) => {
                    // If serialization fails, proceed without metadata
                    // This should not happen with valid data
                    Vec::new()
                }
            }
        };

        let has_metadata = !metadata_section.is_empty();

        let mut header = FileHeader::new(dimensions);
        header.vector_count = vector_count;
        header.index_offset = index_offset;
        header.metadata_offset = tombstone_offset_start; // Points to tombstone bitvec start
        header.hnsw_m = index.config.m;
        header.hnsw_m0 = index.config.m0;

        // v0.4: Set HAS_METADATA flag if metadata section is present
        if has_metadata {
            header.flags |= Flags::HAS_METADATA;
        }

        // v0.3: Persist deleted_count from index (W16.5)
        // SAFETY: deleted_count is usize, header field is u32.
        // For indices with > 4B deleted nodes, we'd truncate, but that's
        // practically impossible (memory limits would be hit first).
        #[allow(clippy::cast_possible_truncation)]
        {
            header.deleted_count = index.deleted_count as u32;
        }

        // Note: RNG state is intentionally not persisted. It's transient and reseeded
        // on load from system entropy, which is fine for HNSW level selection.

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
            tombstone_offset: 0,
            metadata_section,
            metadata_section_offset: 0,
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
                    // Header (MIN_CHUNK_SIZE bytes) always fits because constructor
                    // clamps chunk_size to minimum. This branch is defensive only.
                    if space_left >= bytes.len() {
                        self.buffer.extend_from_slice(bytes);
                        self.state = SerializationState::VectorData;
                    } else {
                        // Defensive: should never execute due to MIN_CHUNK_SIZE clamp.
                        // If reached, return partial chunk without panicking.
                        debug_assert!(
                            false,
                            "chunk_size {} < MIN_CHUNK_SIZE {}",
                            self.chunk_size, MIN_CHUNK_SIZE
                        );
                        self.buffer.extend_from_slice(&bytes[..space_left]);
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
                        self.state = SerializationState::Tombstones;
                        continue;
                    }

                    let bytes_to_copy = min(remaining_bytes, space_left);
                    let end = self.neighbor_offset + bytes_to_copy;
                    self.buffer
                        .extend_from_slice(&neighbors[self.neighbor_offset..end]);

                    self.neighbor_offset += bytes_to_copy;

                    if self.neighbor_offset == neighbors.len() {
                        self.state = SerializationState::Tombstones;
                    } else if bytes_to_copy == 0 {
                        break;
                    }
                }
                SerializationState::Tombstones => {
                    // Serialize storage.deleted (BitVec) as bytes
                    // We pack bits into bytes (Lsb0)
                    let total_bits = self.storage.deleted.len();
                    let total_bytes = (total_bits + 7) / 8;

                    let remaining_bytes = total_bytes - self.tombstone_offset;

                    if remaining_bytes == 0 {
                        // After tombstones, write metadata section if present
                        self.state = SerializationState::MetadataSection;
                        continue;
                    }

                    // How many bytes can we fit?
                    let bytes_to_produce = min(remaining_bytes, space_left);

                    // Produce bytes
                    for _ in 0..bytes_to_produce {
                        let byte_idx = self.tombstone_offset;
                        let start_bit = byte_idx * 8;
                        let mut byte: u8 = 0;
                        for bit_offset in 0..8 {
                            let bit_idx = start_bit + bit_offset;
                            if bit_idx < total_bits {
                                // We access bitvec by index.
                                // Note: This is random access, not super efficient but acceptable for tombstones
                                if self.storage.deleted[bit_idx] {
                                    byte |= 1 << bit_offset;
                                }
                            }
                        }
                        self.buffer.push(byte);
                        self.tombstone_offset += 1;
                    }

                    if self.tombstone_offset == total_bytes {
                        self.state = SerializationState::MetadataSection;
                    } else if bytes_to_produce == 0 {
                        break;
                    }
                }
                SerializationState::MetadataSection => {
                    // v0.4: Write metadata section (header + serialized data)
                    // This section is pre-serialized in export_chunked()
                    let remaining_bytes =
                        self.metadata_section.len() - self.metadata_section_offset;

                    if remaining_bytes == 0 {
                        self.state = SerializationState::Done;
                        continue;
                    }

                    let bytes_to_copy = min(remaining_bytes, space_left);
                    let start = self.metadata_section_offset;
                    let end = start + bytes_to_copy;
                    self.buffer
                        .extend_from_slice(&self.metadata_section[start..end]);

                    self.metadata_section_offset += bytes_to_copy;

                    if self.metadata_section_offset == self.metadata_section.len() {
                        self.state = SerializationState::Done;
                    } else if bytes_to_copy == 0 {
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

        // Header (64) + 10 vectors * 4 dims * 4 bytes (160) + tombstones (2 bytes for 10 bits) = 226 bytes
        // No metadata section since index has no metadata
        let expected = 64 + 160 + 2; // Header + vector data + tombstones
        assert_eq!(total_bytes, expected);
    }

    /// Tests that chunk_size = 0 is clamped to MIN_CHUNK_SIZE and works correctly.
    #[test]
    fn test_chunk_size_zero_edge_case() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        storage.insert(&[1.0, 2.0, 3.0, 4.0]).unwrap();

        let index = HnswIndex::new(config, &storage).unwrap();
        let writer = (&storage, &index);

        // chunk_size = 0 should be clamped to MIN_CHUNK_SIZE (64)
        let iter = writer.export_chunked(0);

        let chunks: Vec<_> = iter.collect();
        assert!(!chunks.is_empty(), "Should produce at least one chunk");

        // All chunks should be <= MIN_CHUNK_SIZE (since 0 was clamped to 64)
        for chunk in &chunks {
            assert!(
                chunk.len() <= MIN_CHUNK_SIZE,
                "Chunk size {} exceeds clamped minimum {}",
                chunk.len(),
                MIN_CHUNK_SIZE
            );
        }

        // Verify header is valid
        let header = FileHeader::from_bytes(&chunks[0][0..64]).unwrap();
        assert_eq!(header.vector_count, 1);
    }

    /// Tests that chunk_size = 1 is clamped to MIN_CHUNK_SIZE.
    #[test]
    fn test_chunk_size_one_edge_case() {
        let config = HnswConfig::new(4);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();
        let writer = (&storage, &index);

        // chunk_size = 1 should be clamped to MIN_CHUNK_SIZE (64)
        let iter = writer.export_chunked(1);

        let chunks: Vec<_> = iter.collect();
        assert!(!chunks.is_empty());

        // First chunk should be exactly header (64 bytes)
        assert_eq!(chunks[0].len(), MIN_CHUNK_SIZE);
    }

    /// Tests that chunk_size = 63 (one less than minimum) is clamped.
    #[test]
    fn test_chunk_size_just_below_minimum() {
        let config = HnswConfig::new(4);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();
        let writer = (&storage, &index);

        // chunk_size = 63 should be clamped to MIN_CHUNK_SIZE (64)
        let iter = writer.export_chunked(63);

        let chunks: Vec<_> = iter.collect();
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].len(), MIN_CHUNK_SIZE);
    }

    /// Tests that chunk_size = 64 (exactly minimum) works correctly.
    #[test]
    fn test_chunk_size_exactly_minimum() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        // Insert multiple vectors to force multiple chunks
        #[allow(clippy::cast_precision_loss)]
        for i in 0..5 {
            storage.insert(&[i as f32; 4]).unwrap();
        }

        let index = HnswIndex::new(config, &storage).unwrap();
        let writer = (&storage, &index);

        // chunk_size = 64 (exactly MIN_CHUNK_SIZE)
        let iter = writer.export_chunked(MIN_CHUNK_SIZE);

        let mut total_bytes = 0;
        let mut chunk_count = 0;
        for chunk in iter {
            assert!(
                chunk.len() <= MIN_CHUNK_SIZE,
                "Chunk {} has size {} > {}",
                chunk_count,
                chunk.len(),
                MIN_CHUNK_SIZE
            );
            total_bytes += chunk.len();
            chunk_count += 1;
        }

        // Should have multiple chunks due to small chunk size
        assert!(
            chunk_count > 1,
            "Expected multiple chunks, got {chunk_count}"
        );

        // Header (64) + 5 vectors * 4 dims * 4 bytes (80) + tombstones (1 byte for 5 bits) = 145 bytes
        let expected = 64 + 80 + 1;
        assert_eq!(total_bytes, expected);
    }

    /// Tests data integrity with edge case chunk sizes (round-trip verification).
    #[test]
    fn test_chunk_size_edge_case_data_integrity() {
        let config = HnswConfig::new(4);
        let mut storage = VectorStorage::new(&config, None);
        let test_vectors = vec![
            [1.0_f32, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
        ];
        for v in &test_vectors {
            storage.insert(v).unwrap();
        }

        let index = HnswIndex::new(config, &storage).unwrap();
        let writer = (&storage, &index);

        // Test with chunk_size = 0 (clamped to 64)
        let iter = writer.export_chunked(0);
        let mut combined = Vec::new();
        for chunk in iter {
            combined.extend_from_slice(&chunk);
        }

        // Verify header
        let header = FileHeader::from_bytes(&combined[0..64]).unwrap();
        assert_eq!(header.vector_count, 3);
        assert_eq!(header.dimensions, 4);

        // Verify vector data (starts at offset 64)
        let vector_bytes = &combined[64..64 + 48]; // 3 vectors * 4 dims * 4 bytes = 48
        let vectors: Vec<f32> = vector_bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();

        assert_eq!(vectors[0..4], test_vectors[0]);
        assert_eq!(vectors[4..8], test_vectors[1]);
        assert_eq!(vectors[8..12], test_vectors[2]);
    }
}
