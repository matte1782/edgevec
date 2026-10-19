//! In-memory buffer format used by the WASM `toBytes()` / `fromBytes()` API
//! and by IndexedDB `save()` / `load()` (issues #11, #12).
//!
//! # Layout (v1)
//!
//! ```text
//! offset  size  field
//! 0       4     magic "EVWB"
//! 4       1     format version (1)
//! 5       1     flags (bit 0: binary quantization enabled)
//! 6       2     reserved, must be zero
//! 8       4     payload length (u32 LE)
//! 12      4     CRC32 over header bytes 0..12 followed by the payload (u32 LE)
//! 16      N     postcard payload: `PersistedIndex` then `MetadataStore`
//! ```
//!
//! Readers reject versions they do not know, and — once the CRC has verified
//! the header — flags or reserved bits they do not know (a newer writer).
//!
//! # Trust model
//!
//! The CRC detects accidental corruption only. Every decoded index is then
//! checked against the structural invariants the index code relies on
//! ([`validate_hnsw`], [`validate_flat`], [`validate_metadata`]) so that a
//! malformed buffer is rejected with an error rather than causing a panic
//! later. This is exercised by a mutation test and by the `wasm_buffer` fuzz
//! target via [`exercise`].
//!
//! # Legacy buffers
//!
//! `save()` in EdgeVec 0.6.0 – 0.9.0 wrote a bare postcard dump with no header:
//!
//! ```text
//! 0.9.0:        str("hnsw") HnswIndex VectorStorage MetadataStore
//!               str("flat") BinaryFlatIndex         MetadataStore
//! 0.6.0–0.8.x:  HnswIndex VectorStorageV08 MetadataStore
//! ```
//!
//! 0.9.0 could not read either back; both are decoded here. 0.8.x storage
//! predates the `binary_data` field, hence [`VectorStorageV08`]. Releases
//! before 0.6.0 used a different `HnswIndex` layout and are not supported.

// `encode` is only called from the wasm32-only `wasm` module, but the codec is
// compiled on every target so it is covered by native tests and fuzzing.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use crate::flat::BinaryFlatIndex;
use crate::hnsw::{HnswConfig, HnswIndex, VectorId};
use crate::metadata::validation::{validate_key_value, MAX_KEYS_PER_VECTOR};
use crate::metadata::MetadataStore;
use crate::persistence::PersistenceError;
use crate::storage::{StorageType, VectorStorage};
use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

/// Magic bytes identifying an EdgeVec WASM buffer.
pub(crate) const BUFFER_MAGIC: [u8; 4] = *b"EVWB";

/// Current buffer format version.
pub(crate) const BUFFER_VERSION: u8 = 1;

/// Header size in bytes.
pub(crate) const BUFFER_HEADER_SIZE: usize = 16;

/// Flag: binary quantization was enabled and must be rebuilt on load.
pub(crate) const FLAG_BQ_ENABLED: u8 = 1 << 0;

/// All flags understood by this version.
const KNOWN_FLAGS: u8 = FLAG_BQ_ENABLED;

/// Legacy 0.9.0 buffers start with postcard(str): varint length 4 + tag.
const LEGACY_HNSW_PREFIX: &[u8] = b"\x04hnsw";
const LEGACY_FLAT_PREFIX: &[u8] = b"\x04flat";

/// Number of free-list buckets in a well-formed `NeighborPool`.
const NEIGHBOR_BUCKETS: usize = 32;
/// Free-list granularity of `NeighborPool` (bytes).
const NEIGHBOR_GRANULARITY: u64 = 16;

// =============================================================================
// Wire types
// =============================================================================

/// Index payload. The wire layout (externally tagged, variant order) is part
/// of the format: never reorder variants.
#[derive(Deserialize)]
pub(crate) enum PersistedIndex {
    /// HNSW graph with its vector storage.
    #[serde(rename = "hnsw")]
    Hnsw {
        /// The graph.
        index: Box<HnswIndex>,
        /// The vectors.
        storage: VectorStorage,
    },
    /// Binary flat index (owns its vectors).
    #[serde(rename = "flat")]
    Flat {
        /// The index.
        index: BinaryFlatIndex,
    },
}

/// Borrowed counterpart of [`PersistedIndex`] used for encoding.
#[derive(Serialize)]
pub(crate) enum PersistedIndexRef<'a> {
    /// HNSW graph with its vector storage.
    #[serde(rename = "hnsw")]
    Hnsw {
        /// The graph.
        index: &'a HnswIndex,
        /// The vectors.
        storage: &'a VectorStorage,
    },
    /// Binary flat index.
    #[serde(rename = "flat")]
    Flat {
        /// The index.
        index: &'a BinaryFlatIndex,
    },
}

#[derive(Serialize)]
struct SnapshotRef<'a> {
    index: PersistedIndexRef<'a>,
    metadata: &'a MetadataStore,
}

#[derive(Deserialize)]
struct Snapshot {
    index: PersistedIndex,
    metadata: MetadataStore,
}

#[derive(Deserialize)]
struct DeletedBits(#[serde(deserialize_with = "crate::storage::bitvec_serde::deserialize")] BitVec);

/// `VectorStorage` as serialized by EdgeVec 0.6.0 – 0.8.x (no `binary_data`).
#[derive(Deserialize)]
struct VectorStorageV08 {
    data_f32: Vec<f32>,
    quantized_data: Vec<u8>,
    config: StorageType,
    #[serde(deserialize_with = "crate::storage::bitvec_serde::deserialize")]
    deleted: BitVec,
    dimensions: u32,
    next_id: u64,
}

impl From<VectorStorageV08> for VectorStorage {
    fn from(v: VectorStorageV08) -> Self {
        VectorStorage {
            data_f32: v.data_f32,
            quantized_data: v.quantized_data,
            binary_data: Vec::new(),
            config: v.config,
            quantizer: None,
            deleted: v.deleted,
            dimensions: v.dimensions,
            wal: None,
            next_id: v.next_id,
        }
    }
}

/// Which layout a buffer was decoded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceFormat {
    /// `EVWB` v1.
    V1,
    /// EdgeVec 0.9.0 `save()`.
    Legacy090,
    /// EdgeVec 0.6.0 – 0.8.x `save()`.
    Legacy060,
}

/// A decoded, validated database ready to be wrapped by the WASM handle.
pub(crate) struct Restored {
    /// The index (BQ already rebuilt if the buffer requested it).
    pub index: PersistedIndex,
    /// Database-level metadata.
    pub metadata: MetadataStore,
    /// The layout the buffer used.
    pub source: SourceFormat,
}

// =============================================================================
// Encoding
// =============================================================================

/// Serializes an index and its metadata into a v1 buffer.
///
/// The payload is written directly after a header placeholder, so the only
/// full-size allocation is the returned buffer.
///
/// # Errors
///
/// Returns [`PersistenceError::SerializationError`] if serialization fails or
/// the payload exceeds `u32::MAX` bytes.
pub(crate) fn encode(
    index: PersistedIndexRef<'_>,
    metadata: &MetadataStore,
    bq_enabled: bool,
) -> Result<Vec<u8>, PersistenceError> {
    let snapshot = SnapshotRef { index, metadata };
    let mut out = postcard::to_extend(&snapshot, vec![0u8; BUFFER_HEADER_SIZE])
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

    let len = u32::try_from(out.len() - BUFFER_HEADER_SIZE).map_err(|_| {
        PersistenceError::SerializationError(format!(
            "payload too large: {} bytes (max {})",
            out.len() - BUFFER_HEADER_SIZE,
            u32::MAX
        ))
    })?;
    let flags = if bq_enabled { FLAG_BQ_ENABLED } else { 0 };

    out[0..4].copy_from_slice(&BUFFER_MAGIC);
    out[4] = BUFFER_VERSION;
    out[5] = flags;
    out[6..8].copy_from_slice(&[0, 0]);
    out[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = checksum(&out);
    out[12..16].copy_from_slice(&crc.to_le_bytes());
    Ok(out)
}

/// CRC32 over header bytes `0..12` and the payload.
fn checksum(buffer: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&buffer[..12]);
    hasher.update(&buffer[BUFFER_HEADER_SIZE..]);
    hasher.finalize()
}

// =============================================================================
// Decoding
// =============================================================================

/// Decodes and validates a buffer in any supported layout.
///
/// # Errors
///
/// - [`PersistenceError::UnsupportedVersion`] / [`PersistenceError::Unsupported`]
///   for buffers written by a newer format version.
/// - [`PersistenceError::InvalidMagic`], [`PersistenceError::BufferTooSmall`],
///   [`PersistenceError::ChecksumMismatch`] or [`PersistenceError::Corrupted`]
///   for anything that is not a valid EdgeVec buffer.
pub(crate) fn decode(bytes: &[u8]) -> Result<Restored, PersistenceError> {
    let (mut index, metadata, bq_enabled, source) = if bytes.starts_with(&BUFFER_MAGIC) {
        let (flags, payload) = parse_header(bytes)?;
        let snapshot = decode_snapshot(payload)?;
        (
            snapshot.index,
            snapshot.metadata,
            flags & FLAG_BQ_ENABLED != 0,
            SourceFormat::V1,
        )
    } else if bytes.starts_with(LEGACY_HNSW_PREFIX) || bytes.starts_with(LEGACY_FLAT_PREFIX) {
        match decode_legacy_090(bytes) {
            Ok((index, metadata)) => (index, metadata, false, SourceFormat::Legacy090),
            // A 0.6–0.8 buffer could, in principle, start with these bytes.
            Err(e) => match decode_legacy_060(bytes) {
                Ok((index, metadata)) => (index, metadata, false, SourceFormat::Legacy060),
                Err(_) => return Err(e),
            },
        }
    } else {
        match decode_legacy_060(bytes) {
            Ok((index, metadata)) => (index, metadata, false, SourceFormat::Legacy060),
            Err(_) if bytes.len() < 4 => {
                return Err(PersistenceError::BufferTooSmall {
                    expected: BUFFER_HEADER_SIZE,
                    actual: bytes.len(),
                })
            }
            Err(_) => {
                return Err(PersistenceError::InvalidMagic {
                    expected: BUFFER_MAGIC,
                    actual: [bytes[0], bytes[1], bytes[2], bytes[3]],
                })
            }
        }
    };

    match &mut index {
        PersistedIndex::Hnsw { index, storage } => validate_hnsw(index, storage)?,
        PersistedIndex::Flat { index } => validate_flat(index)?,
    }
    validate_metadata(&metadata)?;
    if let PersistedIndex::Hnsw { index, .. } = &index {
        validate_metadata(&index.metadata)?;
    }

    if bq_enabled {
        // BQ codes are not serialized; rebuild them from the f32 vectors.
        match &mut index {
            PersistedIndex::Hnsw { index, storage } if storage.config == StorageType::Float32 => {
                index
                    .enable_bq(storage)
                    .map_err(|e| PersistenceError::Corrupted(format!("BQ rebuild failed: {e}")))?;
            }
            _ => {
                return Err(PersistenceError::Corrupted(
                    "BQ flag set on an index without f32 vectors".into(),
                ))
            }
        }
    }

    Ok(Restored {
        index,
        metadata,
        source,
    })
}

/// Verifies a v1 header and returns `(flags, payload)`.
fn parse_header(bytes: &[u8]) -> Result<(u8, &[u8]), PersistenceError> {
    if bytes.len() < BUFFER_HEADER_SIZE {
        return Err(PersistenceError::BufferTooSmall {
            expected: BUFFER_HEADER_SIZE,
            actual: bytes.len(),
        });
    }
    let version = bytes[4];
    if version != BUFFER_VERSION {
        return Err(PersistenceError::UnsupportedVersion(version, 0));
    }

    let len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let payload = &bytes[BUFFER_HEADER_SIZE..];
    if payload.len() != len {
        return Err(PersistenceError::Corrupted(format!(
            "payload length mismatch: header says {len} bytes, buffer has {}",
            payload.len()
        )));
    }

    let expected = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let actual = checksum(bytes);
    if actual != expected {
        return Err(PersistenceError::ChecksumMismatch { expected, actual });
    }

    // The header passed its CRC check: unknown bits mean a newer writer.
    let flags = bytes[5];
    if flags & !KNOWN_FLAGS != 0 || bytes[6] != 0 || bytes[7] != 0 {
        return Err(PersistenceError::Unsupported(format!(
            "buffer uses features unknown to this version (flags {flags:#04x}, reserved {:#04x}{:02x}); upgrade EdgeVec",
            bytes[6], bytes[7]
        )));
    }
    Ok((flags, payload))
}

/// Decodes a postcard value that must consume the entire input.
fn decode_exact<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, PersistenceError> {
    let (value, rest) = postcard::take_from_bytes::<T>(bytes).map_err(corrupted)?;
    if !rest.is_empty() {
        return Err(PersistenceError::Corrupted(format!(
            "{} trailing bytes after payload",
            rest.len()
        )));
    }
    Ok(value)
}

/// Reads the v1 sequence without routing each f32 through serde separately.
/// The wire format stays identical to Snapshot's derived serde layout; golden
/// fixtures and the reference-decoder test guard the field and variant order.
fn decode_snapshot(bytes: &[u8]) -> Result<Snapshot, PersistenceError> {
    let (variant, rest) = postcard::take_from_bytes::<u32>(bytes).map_err(corrupted)?;
    let (index, rest) = match variant {
        0 => {
            let (index, rest) = postcard::take_from_bytes::<HnswIndex>(rest).map_err(corrupted)?;
            let (storage, rest) = decode_storage(rest)?;
            (
                PersistedIndex::Hnsw {
                    index: Box::new(index),
                    storage,
                },
                rest,
            )
        }
        1 => {
            let (index, rest) =
                postcard::take_from_bytes::<BinaryFlatIndex>(rest).map_err(corrupted)?;
            (PersistedIndex::Flat { index }, rest)
        }
        _ => return Err(invalid("unknown index variant")),
    };
    Ok(Snapshot {
        index,
        metadata: decode_exact(rest)?,
    })
}

/// Postcard Vec<f32> is a varint element count followed by little-endian f32
/// values. Bounds-check the whole span before allocating, then decode the
/// contiguous data in one loop. Vec<u8> and serde bytes have identical layouts.
fn decode_storage(bytes: &[u8]) -> Result<(VectorStorage, &[u8]), PersistenceError> {
    let (count, rest) = postcard::take_from_bytes::<usize>(bytes).map_err(corrupted)?;
    let byte_len = count
        .checked_mul(4)
        .ok_or_else(|| invalid("f32 data length overflow"))?;
    let data = rest
        .get(..byte_len)
        .ok_or_else(|| invalid("truncated f32 vector data"))?;
    let rest = &rest[byte_len..];
    let data_f32 = data
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]]))
        .collect();
    let (quantized_data, rest) = postcard::take_from_bytes::<&[u8]>(rest).map_err(corrupted)?;
    let (binary_data, rest) = postcard::take_from_bytes::<&[u8]>(rest).map_err(corrupted)?;
    let ((config, deleted, dimensions, next_id), rest) =
        postcard::take_from_bytes::<(StorageType, DeletedBits, u32, u64)>(rest)
            .map_err(corrupted)?;
    Ok((
        VectorStorage {
            data_f32,
            quantized_data: quantized_data.to_vec(),
            binary_data: binary_data.to_vec(),
            config,
            quantizer: None,
            deleted: deleted.0,
            dimensions,
            wal: None,
            next_id,
        },
        rest,
    ))
}

fn decode_legacy_090(bytes: &[u8]) -> Result<(PersistedIndex, MetadataStore), PersistenceError> {
    let (tag, rest) = postcard::take_from_bytes::<String>(bytes).map_err(corrupted)?;
    let (index, rest) = match tag.as_str() {
        "hnsw" => {
            let (index, rest) = postcard::take_from_bytes::<HnswIndex>(rest).map_err(corrupted)?;
            let (storage, rest) =
                postcard::take_from_bytes::<VectorStorage>(rest).map_err(corrupted)?;
            (
                PersistedIndex::Hnsw {
                    index: Box::new(index),
                    storage,
                },
                rest,
            )
        }
        "flat" => {
            let (index, rest) =
                postcard::take_from_bytes::<BinaryFlatIndex>(rest).map_err(corrupted)?;
            (PersistedIndex::Flat { index }, rest)
        }
        other => {
            return Err(PersistenceError::Corrupted(format!(
                "unknown legacy index variant: {other:?}"
            )))
        }
    };
    Ok((index, decode_exact::<MetadataStore>(rest)?))
}

fn decode_legacy_060(bytes: &[u8]) -> Result<(PersistedIndex, MetadataStore), PersistenceError> {
    let (mut index, rest) = postcard::take_from_bytes::<HnswIndex>(bytes).map_err(corrupted)?;
    let (storage, rest) = postcard::take_from_bytes::<VectorStorageV08>(rest).map_err(corrupted)?;
    let metadata = decode_exact::<MetadataStore>(rest)?;
    // The 0.6–0.8 WASM API kept its metadata on HnswIndex; the outer store
    // was present on the wire but unused. Move it to the current API's store.
    let metadata = if metadata.is_empty() {
        std::mem::take(&mut index.metadata)
    } else {
        metadata
    };
    Ok((
        PersistedIndex::Hnsw {
            index: Box::new(index),
            storage: storage.into(),
        },
        metadata,
    ))
}

#[allow(clippy::needless_pass_by_value)]
fn corrupted(e: postcard::Error) -> PersistenceError {
    PersistenceError::Corrupted(format!("deserialization failed: {e}"))
}

// =============================================================================
// Validation
// =============================================================================

fn invalid(msg: impl Into<String>) -> PersistenceError {
    PersistenceError::Corrupted(msg.into())
}

/// Checks every invariant the HNSW code relies on, and normalizes derived
/// bookkeeping (`deleted_count`, `max_layer`, `next_id`) from the primary data.
///
/// # Errors
///
/// Returns [`PersistenceError::Corrupted`] describing the first violation, or
/// [`PersistenceError::Unsupported`] for scalar-quantized storage (never
/// produced by the WASM API; its quantizer is not persisted).
pub(crate) fn validate_hnsw(
    index: &mut HnswIndex,
    storage: &mut VectorStorage,
) -> Result<(), PersistenceError> {
    let config = &index.config;
    let dims = config.dimensions;
    if dims == 0 {
        return Err(invalid("index has zero dimensions"));
    }
    if dims != storage.dimensions {
        return Err(invalid(format!(
            "index dimensions {dims} do not match storage dimensions {}",
            storage.dimensions
        )));
    }
    if !matches!(
        config.metric,
        HnswConfig::METRIC_L2_SQUARED
            | HnswConfig::METRIC_COSINE
            | HnswConfig::METRIC_DOT_PRODUCT
            | HnswConfig::METRIC_HAMMING
    ) {
        return Err(invalid(format!("unknown metric code {}", config.metric)));
    }
    if config.m <= 1 || config.m0 < config.m {
        return Err(invalid(format!(
            "invalid HNSW parameters m={} m0={}",
            config.m, config.m0
        )));
    }
    if !index.level_mult.is_finite() || index.level_mult < 0.0 {
        return Err(invalid("invalid level multiplier"));
    }
    if !index.compaction_threshold().is_finite()
        || !(0.0..=1.0).contains(&index.compaction_threshold())
    {
        return Err(invalid("invalid compaction threshold"));
    }

    // --- Vector storage --------------------------------------------------
    let len = storage.deleted.len();
    let is_hamming = config.metric == HnswConfig::METRIC_HAMMING;
    match storage.config {
        StorageType::Float32 => {
            if is_hamming {
                return Err(invalid("hamming metric requires binary storage"));
            }
            if Some(storage.data_f32.len()) != len.checked_mul(dims as usize)
                || !storage.binary_data.is_empty()
                || !storage.quantized_data.is_empty()
            {
                return Err(invalid(format!(
                    "f32 vector data length {} does not match {len} vectors x {dims} dimensions",
                    storage.data_f32.len()
                )));
            }
            if storage.data_f32.iter().any(|v| !v.is_finite()) {
                return Err(invalid("vector data contains NaN or Infinity"));
            }
        }
        StorageType::Binary(bits) => {
            if !is_hamming || bits != dims {
                return Err(invalid(
                    "binary storage requires hamming metric and matching bits",
                ));
            }
            if Some(storage.binary_data.len()) != len.checked_mul(((dims as usize) + 7) / 8)
                || !storage.data_f32.is_empty()
                || !storage.quantized_data.is_empty()
            {
                return Err(invalid(format!(
                    "binary vector data length {} does not match {len} vectors",
                    storage.binary_data.len()
                )));
            }
        }
        StorageType::QuantizedU8(_) => {
            return Err(PersistenceError::Unsupported(
                "scalar-quantized storage cannot be restored from a buffer".into(),
            ));
        }
    }
    // Every insert assigns `next_id` then pushes exactly one vector.
    storage.next_id = len as u64 + 1;

    // --- Nodes -----------------------------------------------------------
    let node_count = index.nodes.len();
    if node_count > len || u32::try_from(node_count).is_err() {
        return Err(invalid(format!(
            "{node_count} graph nodes but only {len} stored vectors"
        )));
    }
    let pool = &index.neighbors.buffer;
    let pool_len = pool.len() as u64;
    let mut seen: BitVec = BitVec::repeat(false, len);
    let mut ranges = Vec::with_capacity(node_count);
    for (i, node) in index.nodes.iter().enumerate() {
        let VectorId(vid) = node.vector_id;
        if vid == 0 || vid > len as u64 {
            return Err(invalid(format!(
                "node {i} references vector id {vid} but storage holds {len} vectors"
            )));
        }
        // vid <= len, which is an in-memory length.
        #[allow(clippy::cast_possible_truncation)]
        let vid = vid as usize;
        if seen.replace(vid - 1, true) {
            return Err(invalid(format!("vector id {vid} is used by two nodes")));
        }
        if node.deleted > 1 {
            return Err(invalid(format!("node {i} has invalid deleted flag")));
        }
        let start = u64::from(node.neighbor_offset);
        let end = start + u64::from(node.neighbor_len);
        if end > pool_len {
            return Err(invalid(format!(
                "node {i} neighbor range ends at {end} but pool holds {pool_len} bytes"
            )));
        }
        if end > start {
            ranges.push((start, end));
        }
        // end <= pool_len, an in-memory length.
        #[allow(clippy::cast_possible_truncation)]
        let lists = &pool[start as usize..end as usize];
        check_neighbor_lists(lists, node.max_layer, |id, level| {
            index
                .nodes
                .get(id as usize)
                .is_some_and(|n| n.max_layer >= level)
        })
        .map_err(|e| invalid(format!("node {i}: {e}")))?;
    }

    // --- Free list -------------------------------------------------------
    let buckets = &index.neighbors.buckets;
    if buckets.len() != NEIGHBOR_BUCKETS {
        return Err(invalid(format!(
            "neighbor pool has {} free-list buckets, expected {NEIGHBOR_BUCKETS}",
            buckets.len()
        )));
    }
    for (b, offsets) in buckets.iter().enumerate() {
        let capacity = (b as u64 + 1) * NEIGHBOR_GRANULARITY;
        for &off in offsets {
            let start = u64::from(off);
            let end = start + capacity;
            if end > pool_len {
                return Err(invalid("neighbor free list points outside the pool"));
            }
            ranges.push((start, end));
        }
    }
    // Live allocations and free slots must have exclusive ownership. Reusing
    // an aliased slot would overwrite another node's validated neighbor list.
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(invalid("neighbor pool allocations overlap"));
    }

    // --- Entry point and derived counters ---------------------------------
    match index.entry_point {
        None if node_count == 0 => index.max_layer = 0,
        None => return Err(invalid("non-empty index has no entry point")),
        Some(ep) => {
            let ep_node = index.nodes.get(ep.0 as usize).ok_or_else(|| {
                invalid(format!(
                    "entry point {} out of range for {node_count} nodes",
                    ep.0
                ))
            })?;
            index.max_layer = ep_node.max_layer;
        }
    }
    index.deleted_count = index.nodes.iter().filter(|n| n.deleted != 0).count();

    Ok(())
}

/// Walks the concatenated `[L0][L1]...` neighbor lists of one node the same
/// way `NeighborPool::iter_layer` does, requiring each list for levels
/// `0..=max_layer` that is present to be fully in bounds and every neighbor
/// to satisfy `valid(id, level)`.
fn check_neighbor_lists(
    data: &[u8],
    max_layer: u8,
    valid: impl Fn(u32, u8) -> bool,
) -> Result<(), String> {
    let mut cursor = 0usize;
    for level in 0..=max_layer {
        if cursor >= data.len() {
            return Ok(()); // remaining levels have no neighbors
        }
        let count = read_vbyte(data, &mut cursor)?;
        let mut id = 0u32;
        for _ in 0..count {
            id = id.wrapping_add(read_vbyte(data, &mut cursor)?);
            if !valid(id, level) {
                return Err(format!("invalid neighbor {id} at layer {level}"));
            }
        }
    }
    Ok(())
}

fn read_vbyte(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let mut value = 0u32;
    for shift in [0u32, 7, 14, 21, 28] {
        let byte = *data
            .get(*cursor)
            .ok_or_else(|| "neighbor list truncated".to_string())?;
        *cursor += 1;
        value |= u32::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("neighbor list varint too long".into())
}

/// Checks the invariants of a decoded flat index.
///
/// # Errors
///
/// Returns [`PersistenceError::Corrupted`] if the index is inconsistent.
pub(crate) fn validate_flat(index: &BinaryFlatIndex) -> Result<(), PersistenceError> {
    if index.is_consistent() && index.dimensions() > 0 {
        Ok(())
    } else {
        Err(invalid(
            "flat index vector buffer does not match its dimensions/count",
        ))
    }
}

/// Applies the same key/value limits as the insert API.
///
/// # Errors
///
/// Returns [`PersistenceError::Corrupted`] on the first invalid entry.
pub(crate) fn validate_metadata(metadata: &MetadataStore) -> Result<(), PersistenceError> {
    for &id in metadata.vector_ids() {
        let Some(entries) = metadata.get_all(id) else {
            continue;
        };
        if entries.len() > MAX_KEYS_PER_VECTOR {
            return Err(invalid(format!(
                "vector {id} has {} metadata keys (max {MAX_KEYS_PER_VECTOR})",
                entries.len()
            )));
        }
        for (key, value) in entries {
            validate_key_value(key, value)
                .map_err(|e| invalid(format!("invalid metadata on vector {id}: {e}")))?;
        }
    }
    Ok(())
}

// =============================================================================
// Robustness harness (shared by tests and the `wasm_buffer` fuzz target)
// =============================================================================

/// Decodes `bytes` and, if accepted, drives the restored index through the
/// operations a WASM user can call. Must never panic.
pub(crate) fn exercise(bytes: &[u8]) {
    // Empty indexes may legitimately declare very large dimensions. Bound the
    // synthesized queries so fuzzing the decoder cannot allocate huge vectors.
    const MAX_QUERY_DIMENSIONS: usize = 4096;
    let Ok(restored) = decode(bytes) else {
        return;
    };
    match restored.index {
        PersistedIndex::Hnsw {
            mut index,
            mut storage,
        } => {
            let dims = index.config.dimensions as usize;
            if dims > MAX_QUERY_DIMENSIONS {
                return;
            }
            if let StorageType::Binary(_) = storage.config {
                let query = vec![0xA5u8; (dims + 7) / 8];
                let _ = index.search_binary(&query, 5, &storage);
                let _ = index.insert_binary(&query, &mut storage);
                let _ = index.search_binary(&query, 5, &storage);
            } else {
                #[allow(clippy::cast_precision_loss)]
                let query: Vec<f32> = (0..dims).map(|d| (d % 7) as f32 * 0.1 - 0.3).collect();
                let _ = index.search(&query, 5, &storage);
                let _ = index.insert(&query, &mut storage);
                let _ = index.search(&query, 5, &storage);
                if index.bq_storage.is_some() || index.enable_bq(&storage).is_ok() {
                    let _ = index.search_bq(&query, 5, &storage);
                }
            }
            let _ = index.soft_delete(VectorId(1));
            // `compact` panics on ANY binary-storage index with tombstones
            // (pre-existing bug in `HnswIndex::compact`, unrelated to
            // persistence), so it is only exercised for f32 storage.
            if storage.config == StorageType::Float32 {
                let _ = index.compact(&storage);
            }
        }
        PersistedIndex::Flat { mut index } => {
            if index.dimensions() > MAX_QUERY_DIMENSIONS {
                return;
            }
            let query = vec![0x5Au8; index.bytes_per_vector()];
            let _ = index.search(&query, 5);
            let _ = index.insert(&query);
            let _ = index.search(&query, 5);
        }
    }
}

#[cfg(test)]
mod tests;
