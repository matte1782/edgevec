//! Neighbor pool for HNSW graph storage.
//!
//! # Lint Suppressions
//!
//! - **cast_possible_truncation**: Neighbor IDs are `u32` (max 4B vectors supported).
//!   Offsets into the byte buffer are validated at allocation time to fit in `u32`.
//!   Variable-byte encoding uses `u8` segments that cannot overflow.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::doc_markdown)]

use super::graph::GraphError;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

/// Byte-pool of neighbor lists using Variable-Byte Encoding with a Free List.
///
/// This structure manages a contiguous byte buffer for storing compressed neighbor lists.
/// It implements a "best-fit" memory recycling strategy using a free list to reuse
/// deallocated segments.
#[derive(Debug, Clone, Serialize, Deserialize)] // Clone might be expensive for buffer, but needed for HnswIndex derive
pub struct NeighborPool {
    /// Contiguous compressed data buffer.
    #[serde(with = "serde_bytes")]
    pub(crate) buffer: Vec<u8>,

    /// Free list buckets.
    ///
    /// Index `i` stores offsets for capacity `(i + 1) * GRANULARITY`.
    /// `GRANULARITY` is 16 bytes.
    /// Max bucketed size is 512 bytes (Index 31).
    /// Allocations larger than 512 bytes are not recycled (rare case).
    buckets: Vec<Vec<u32>>,
}

/// Zero-copy iterator over neighbors in a compressed list.
pub struct NeighborIter<'a> {
    data: &'a [u8],
    cursor: usize,
    count: u32,
    prev: u32,
    current_idx: u32,
}

impl NeighborIter<'_> {
    fn empty() -> Self {
        Self {
            data: &[],
            cursor: 0,
            count: 0,
            prev: 0,
            current_idx: 0,
        }
    }
}

impl Iterator for NeighborIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.count || self.cursor >= self.data.len() {
            return None;
        }

        let (delta, bytes) = NeighborPool::vbyte_decode(self.data, self.cursor);
        self.cursor += bytes;
        let val = self.prev.wrapping_add(delta);
        self.prev = val;
        self.current_idx += 1;
        Some(val)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.count - self.current_idx) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for NeighborIter<'_> {}

impl NeighborPool {
    /// Allocation granularity (bytes).
    const GRANULARITY: usize = 16;
    /// Maximum bucket index (corresponds to size 512).
    const MAX_BUCKET_IDX: usize = 31;

    /// Creates a new empty neighbor pool.
    #[must_use]
    pub fn new() -> Self {
        // Initialize buckets
        let mut buckets = Vec::with_capacity(Self::MAX_BUCKET_IDX + 1);
        for _ in 0..=Self::MAX_BUCKET_IDX {
            buckets.push(Vec::new());
        }

        Self {
            buffer: Vec::new(),
            buckets,
        }
    }

    /// Allocates space in the pool for neighbor data.
    ///
    /// Uses a segregated free list (buckets) to find a suitable slot in O(1).
    /// Rounds up `size` to the nearest multiple of `GRANULARITY`.
    ///
    /// # Arguments
    /// * `size` - The minimum number of bytes required.
    ///
    /// # Returns
    /// `(offset, capacity)` where:
    /// - `offset` is the byte offset in the buffer.
    /// - `capacity` is the actual size of the allocated slot (multiple of 16).
    ///
    /// # Errors
    /// Returns `GraphError::CapacityExceeded` if the pool exceeds `u32::MAX` bytes.
    /// Returns `GraphError::NeighborError` if `size` > `u16::MAX`.
    pub fn alloc(&mut self, size: usize) -> Result<(u32, u16), GraphError> {
        if size > u16::MAX as usize {
            return Err(GraphError::NeighborError);
        }

        // 1. Calculate target capacity (round up to next multiple of 16)
        let remainder = size % Self::GRANULARITY;
        let pad = if remainder == 0 {
            0
        } else {
            Self::GRANULARITY - remainder
        };
        let target_cap = size + pad;

        // Ensure at least one block size
        let target_cap = if target_cap == 0 {
            Self::GRANULARITY
        } else {
            target_cap
        };

        // 2. Check buckets
        let bucket_idx = (target_cap / Self::GRANULARITY).saturating_sub(1);

        if bucket_idx <= Self::MAX_BUCKET_IDX {
            if let Some(offset) = self.buckets[bucket_idx].pop() {
                return Ok((offset, target_cap as u16));
            }
        }

        // 3. Not found or too large, append to buffer
        let current_len = self.buffer.len();

        if current_len
            .checked_add(target_cap)
            .ok_or(GraphError::CapacityExceeded)?
            > u32::MAX as usize
        {
            return Err(GraphError::CapacityExceeded);
        }

        let offset = u32::try_from(current_len).map_err(|_| GraphError::CapacityExceeded)?;
        let capacity = u16::try_from(target_cap).map_err(|_| GraphError::NeighborError)?;

        // Grow buffer (fill with zeros to be safe)
        self.buffer.resize(current_len + target_cap, 0);

        Ok((offset, capacity))
    }

    /// Frees a previously allocated slot, making it available for reuse.
    ///
    /// # Arguments
    /// * `offset` - The byte offset of the slot.
    /// * `capacity` - The *allocated capacity* of the slot.
    pub fn free(&mut self, offset: u32, capacity: u16) {
        if capacity == 0 {
            return;
        }
        let cap = capacity as usize;

        // Only recycle if it fits in our buckets and is aligned
        if cap % Self::GRANULARITY == 0 {
            let bucket_idx = (cap / Self::GRANULARITY).saturating_sub(1);
            if bucket_idx <= Self::MAX_BUCKET_IDX {
                self.buckets[bucket_idx].push(offset);
            }
        }
    }

    /// Encodes a list of neighbors using Delta + VByte encoding.
    ///
    /// # Format
    /// `[Count (VByte), Delta_0 (VByte), Delta_1 (VByte), ...]`
    ///
    /// # Arguments
    /// * `neighbors` - List of neighbor IDs.
    ///
    /// # Returns
    /// Encoded bytes.
    pub fn encode_neighbors(neighbors: &[u32]) -> Vec<u8> {
        let mut buf = Vec::new();
        Self::encode_neighbors_to_buf(neighbors, &mut buf);
        buf
    }

    /// Encodes a list of neighbors into a provided buffer using Delta + VByte encoding.
    ///
    /// # Arguments
    /// * `neighbors` - List of neighbor IDs.
    /// * `buf` - Buffer to append encoded bytes to.
    pub fn encode_neighbors_to_buf(neighbors: &[u32], buf: &mut Vec<u8>) {
        if neighbors.is_empty() {
            Self::vbyte_encode(0, buf);
            return;
        }

        // 1. Sort neighbors (required for Delta encoding)
        // We can't sort in-place here as input is slice.
        // If caller passes sorted, it's faster, but we can't trust it.
        // For performance, we assume caller handles sorting if they want,
        // OR we allocate a small vector for sorting.
        // However, standard `encode_neighbors` sorts.
        // Let's alloc locally for sort. It's small (M=16-32 usually).
        let mut sorted = neighbors.to_vec();
        sorted.sort_unstable();

        // 2. Encode count
        Self::vbyte_encode(u32::try_from(sorted.len()).unwrap_or(u32::MAX), buf);

        // 3. Delta Encode
        let mut prev = 0u32;
        for &curr in &sorted {
            let delta = curr.wrapping_sub(prev);
            Self::vbyte_encode(delta, buf);
            prev = curr;
        }
    }

    /// Decodes a list of neighbors from the compressed buffer.
    ///
    /// # Arguments
    /// * `data` - The compressed data slice.
    ///
    /// # Returns
    /// Vector of neighbor IDs.
    pub fn decode_neighbors(data: &[u8]) -> Vec<u32> {
        let mut buf = Vec::new();
        Self::decode_neighbors_to_buf(data, &mut buf);
        buf
    }

    /// Decodes a list of neighbors into a provided buffer.
    ///
    /// # Arguments
    /// * `data` - The compressed data slice.
    /// * `buf` - Buffer to append decoded IDs to.
    pub fn decode_neighbors_to_buf(data: &[u8], buf: &mut Vec<u32>) {
        if data.is_empty() {
            return;
        }
        let _ = Self::decode_one_list_to_buf(data, 0, buf);
    }

    /// Decodes neighbors for a specific level, assuming lists are concatenated [L0, L1, ...].
    pub fn decode_layer(data: &[u8], target_level: u8) -> Vec<u32> {
        Self::iter_layer(data, target_level).collect()
    }

    /// Decodes neighbors for a specific level into a buffer.
    pub fn decode_layer_to_buf(data: &[u8], target_level: u8, buf: &mut Vec<u32>) {
        buf.extend(Self::iter_layer(data, target_level));
    }

    /// Returns a zero-copy iterator over neighbors for a specific level.
    pub fn iter_layer(data: &[u8], target_level: u8) -> NeighborIter<'_> {
        if data.is_empty() {
            return NeighborIter::empty();
        }

        let mut cursor = 0;
        let mut current_level = 0;

        while cursor < data.len() {
            if current_level == target_level {
                let (count, bytes) = Self::vbyte_decode(data, cursor);
                return NeighborIter {
                    data,
                    cursor: cursor + bytes,
                    count,
                    prev: 0,
                    current_idx: 0,
                };
            }

            // Skip this list
            let (count, bytes) = Self::vbyte_decode(data, cursor);
            cursor += bytes;

            // Skip deltas
            for _ in 0..count {
                if cursor >= data.len() {
                    break;
                }
                let (_, b) = Self::vbyte_decode(data, cursor);
                cursor += b;
            }

            current_level += 1;
        }

        NeighborIter::empty()
    }

    /// Helper: Decodes one list into buffer. Returns bytes_read.
    fn decode_one_list_to_buf(data: &[u8], mut cursor: usize, buf: &mut Vec<u32>) -> usize {
        let start_cursor = cursor;
        if cursor >= data.len() {
            return 0;
        }

        // 1. Decode count
        let (count, bytes_read) = Self::vbyte_decode(data, cursor);
        cursor += bytes_read;

        if count == 0 {
            return cursor - start_cursor;
        }

        // Reserve space if needed (optimization)
        buf.reserve(count as usize);

        let mut prev = 0u32;

        for _ in 0..count {
            if cursor >= data.len() {
                break; // Truncated data?
            }
            let (delta, bytes_read) = Self::vbyte_decode(data, cursor);
            cursor += bytes_read;

            let curr = prev.wrapping_add(delta);
            buf.push(curr);
            prev = curr;
        }

        cursor - start_cursor
    }

    /// Helper: Encodes a single u32 using VByte.
    fn vbyte_encode(mut val: u32, buf: &mut Vec<u8>) {
        loop {
            if val < 128 {
                buf.push(val as u8);
                break;
            }
            buf.push((val as u8 & 0x7F) | 0x80);
            val >>= 7;
        }
    }

    /// Returns the approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let buffer_size = self.buffer.capacity();
        let buckets_size = self
            .buckets
            .iter()
            .map(|b| b.capacity() * std::mem::size_of::<u32>())
            .sum::<usize>();
        let buckets_overhead = self.buckets.capacity() * std::mem::size_of::<Vec<u32>>();

        std::mem::size_of::<Self>() + buffer_size + buckets_size + buckets_overhead
    }

    /// Helper: Decodes a single u32 from VByte.
    /// Returns (value, bytes_read).
    fn vbyte_decode(data: &[u8], start: usize) -> (u32, usize) {
        let mut val = 0u32;
        let mut shift = 0;
        let mut bytes_read = 0;

        for byte in data.iter().skip(start) {
            bytes_read += 1;

            val |= u32::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return (val, bytes_read);
            }
            shift += 7;

            // Protection against overflow (u32 max is 32 bits, 5 bytes max)
            if shift >= 35 {
                // Corrupted data or too large? Return what we have.
                return (val, bytes_read);
            }
        }

        (val, bytes_read)
    }
}

impl Default for NeighborPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vbyte_roundtrip() {
        let original = vec![10, 100, 1000, 10000, 100_000, 1_000_000];
        // Note: encode sorts them, but they are already sorted.
        let encoded = NeighborPool::encode_neighbors(&original);
        let decoded = NeighborPool::decode_neighbors(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_vbyte_unsorted() {
        let original = vec![50, 10, 30, 20];
        let expected = vec![10, 20, 30, 50];

        let encoded = NeighborPool::encode_neighbors(&original);
        let decoded = NeighborPool::decode_neighbors(&encoded);
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_alloc_free_recycle() {
        let mut pool = NeighborPool::new();

        // 1. Alloc 32 bytes (GRANULARITY=16, 2 chunks)
        let (offset1, cap1) = pool.alloc(30).unwrap();
        assert_eq!(offset1, 0);
        assert_eq!(cap1, 32); // Rounded up to 32
        assert_eq!(pool.buffer.len(), 32);

        // 2. Free it
        pool.free(offset1, cap1);
        // Buckets should have {32: [0]}

        // 3. Alloc 10 bytes (should round up to 16, so different bucket?)
        // Wait, 10 -> round up to 16. Bucket idx 0.
        // 32 was Bucket idx 1.
        // So it WON'T reuse the 32 byte slot for a 16 byte request.
        let (offset2, cap2) = pool.alloc(10).unwrap();
        assert_eq!(offset2, 32); // Appended
        assert_eq!(cap2, 16);

        // 4. Alloc 32 bytes (should reuse the first slot)
        let (offset3, cap3) = pool.alloc(32).unwrap();
        assert_eq!(offset3, 0); // Reused
        assert_eq!(cap3, 32);

        // 5. Alloc large (no recycle)
        let (offset4, cap4) = pool.alloc(600).unwrap();
        // 32 + 16 + 608 (rounded up) = 656
        assert!(offset4 > 0);
        assert_eq!(cap4, 608); // 600 -> 608 (multiple of 16)
    }

    #[test]
    fn test_alloc_too_large() {
        let mut pool = NeighborPool::new();
        let res = pool.alloc(100_000); // > u16::MAX
        assert!(matches!(res, Err(GraphError::NeighborError)));
    }

    #[test]
    fn test_decode_layer() {
        // Construct a multi-layer blob
        // Layer 0: [1, 2]
        // Layer 1: [3]
        // Layer 2: [4, 5, 6]

        let l0 = NeighborPool::encode_neighbors(&[1, 2]);
        let l1 = NeighborPool::encode_neighbors(&[3]);
        let l2 = NeighborPool::encode_neighbors(&[4, 5, 6]);

        let mut blob = Vec::new();
        blob.extend_from_slice(&l0);
        blob.extend_from_slice(&l1);
        blob.extend_from_slice(&l2);

        // Test decoding
        assert_eq!(NeighborPool::decode_layer(&blob, 0), vec![1, 2]);
        assert_eq!(NeighborPool::decode_layer(&blob, 1), vec![3]);
        assert_eq!(NeighborPool::decode_layer(&blob, 2), vec![4, 5, 6]);
        assert_eq!(NeighborPool::decode_layer(&blob, 3), Vec::<u32>::new()); // Out of bounds
    }

    #[test]
    fn test_iter_layer() {
        let l0 = NeighborPool::encode_neighbors(&[1, 2]);
        let l1 = NeighborPool::encode_neighbors(&[3]);
        let l2 = NeighborPool::encode_neighbors(&[4, 5, 6]);

        let mut blob = Vec::new();
        blob.extend_from_slice(&l0);
        blob.extend_from_slice(&l1);
        blob.extend_from_slice(&l2);

        let iter0: Vec<u32> = NeighborPool::iter_layer(&blob, 0).collect();
        assert_eq!(iter0, vec![1, 2]);

        let iter1: Vec<u32> = NeighborPool::iter_layer(&blob, 1).collect();
        assert_eq!(iter1, vec![3]);

        let iter2: Vec<u32> = NeighborPool::iter_layer(&blob, 2).collect();
        assert_eq!(iter2, vec![4, 5, 6]);
    }
}
