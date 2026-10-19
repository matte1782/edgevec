use super::*;
use crate::hnsw::NodeId;
use crate::metadata::MetadataValue;
use crate::quantization::QuantizerConfig;
use std::collections::HashMap;
use std::fmt::Write as _;

// =============================================================================
// Fixtures
// =============================================================================

fn vector(seed: usize, dims: usize) -> Vec<f32> {
    #[allow(clippy::cast_precision_loss)]
    (0..dims)
        .map(|d| ((seed * 31 + d * 17) % 97) as f32 / 97.0 - 0.5)
        .collect()
}

fn hnsw(metric: u32, dims: u32, n: usize) -> (HnswIndex, VectorStorage) {
    let mut config = HnswConfig::new(dims);
    config.metric = metric;
    let mut storage = VectorStorage::new(&config, None);
    if metric == HnswConfig::METRIC_HAMMING {
        storage.set_storage_type(StorageType::Binary(dims));
    }
    let mut index = HnswIndex::new(config, &storage).unwrap();
    for i in 0..n {
        if metric == HnswConfig::METRIC_HAMMING {
            #[allow(clippy::cast_possible_truncation)]
            let bits: Vec<u8> = (0..dims as usize / 8)
                .map(|b| (i * 13 + b * 7) as u8)
                .collect();
            index.insert_binary(&bits, &mut storage).unwrap();
        } else {
            index
                .insert(&vector(i, dims as usize), &mut storage)
                .unwrap();
        }
    }
    (index, storage)
}

fn flat(n: usize) -> BinaryFlatIndex {
    let mut index = BinaryFlatIndex::new(64).unwrap();
    for i in 0..n {
        #[allow(clippy::cast_possible_truncation)]
        let v: Vec<u8> = (0..8).map(|b| (i * 29 + b) as u8).collect();
        index.insert(&v).unwrap();
    }
    index
}

fn metadata() -> MetadataStore {
    let mut m = MetadataStore::new();
    m.insert(1, "title", MetadataValue::String("doc".into()))
        .unwrap();
    m.insert(1, "tags", MetadataValue::StringArray(vec!["a".into()]))
        .unwrap();
    m.insert(2, "n", MetadataValue::Integer(7)).unwrap();
    m.insert(3, "score", MetadataValue::Float(0.5)).unwrap();
    m.insert(3, "ok", MetadataValue::Boolean(true)).unwrap();
    m
}

fn encode_hnsw(index: &HnswIndex, storage: &VectorStorage, bq: bool) -> Vec<u8> {
    encode(PersistedIndexRef::Hnsw { index, storage }, &metadata(), bq).unwrap()
}

/// Encodes a sample HNSW index after applying `tamper` to its internals.
fn tampered(tamper: impl FnOnce(&mut HnswIndex, &mut VectorStorage)) -> Vec<u8> {
    let (mut index, mut storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 40);
    tamper(&mut index, &mut storage);
    encode_hnsw(&index, &storage, false)
}

/// Recomputes length and CRC after the payload was modified.
fn reseal(buf: &mut [u8]) {
    let len = u32::try_from(buf.len() - BUFFER_HEADER_SIZE).unwrap();
    buf[8..12].copy_from_slice(&len.to_le_bytes());
    let crc = checksum(buf);
    buf[12..16].copy_from_slice(&crc.to_le_bytes());
}

fn restored_hnsw(bytes: &[u8]) -> (Box<HnswIndex>, VectorStorage, MetadataStore) {
    let r = decode(bytes).unwrap();
    match r.index {
        PersistedIndex::Hnsw { index, storage } => (index, storage, r.metadata),
        PersistedIndex::Flat { .. } => panic!("expected HNSW"),
    }
}

// =============================================================================
// Header
// =============================================================================

#[test]
fn v1_roundtrip_preserves_index_and_metadata() {
    let (index, storage) = hnsw(HnswConfig::METRIC_COSINE, 16, 60);
    let bytes = encode_hnsw(&index, &storage, false);
    assert_eq!(&bytes[..4], b"EVWB");
    assert_eq!(bytes[4], BUFFER_VERSION);

    let restored = decode(&bytes).unwrap();
    assert_eq!(restored.source, SourceFormat::V1);
    assert_eq!(restored.metadata, metadata());
    let PersistedIndex::Hnsw {
        index: decoded,
        storage: decoded_storage,
    } = restored.index
    else {
        panic!("expected HNSW");
    };
    assert_eq!(decoded_storage.data_f32, storage.data_f32);
    assert_eq!(decoded_storage.deleted, storage.deleted);
    assert_eq!(decoded_storage.next_id, storage.next_id);
    assert_eq!(decoded.entry_point, index.entry_point);
    let query = vector(999, 16);
    let ids = |idx: &HnswIndex, st: &VectorStorage| -> Vec<VectorId> {
        idx.search(&query, 10, st)
            .unwrap()
            .iter()
            .map(|hit| hit.vector_id)
            .collect()
    };
    assert_eq!(ids(&index, &storage), ids(&decoded, &decoded_storage));
}

#[test]
fn v1_roundtrip_flat_and_empty() {
    let f = flat(10);
    let bytes = encode(
        PersistedIndexRef::Flat { index: &f },
        &MetadataStore::new(),
        false,
    )
    .unwrap();
    let PersistedIndex::Flat { index } = decode(&bytes).unwrap().index else {
        panic!("expected flat");
    };
    assert_eq!(index.len(), 10);

    let (index, storage) = hnsw(HnswConfig::METRIC_COSINE, 768, 0);
    let (d, _, _) = restored_hnsw(&encode_hnsw(&index, &storage, false));
    assert!(d.is_empty());
}

#[test]
fn bq_flag_rebuilds_binary_quantization() {
    let (mut index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 64, 50);
    index.enable_bq(&storage).unwrap();
    let bytes = encode_hnsw(&index, &storage, true);
    let (d, s, _) = restored_hnsw(&bytes);
    assert!(d.bq_storage.is_some());
    let q = vector(5, 64);
    assert_eq!(
        index.search_bq(&q, 10, &storage).unwrap(),
        d.search_bq(&q, 10, &s).unwrap()
    );
}

#[test]
fn bq_flag_on_binary_storage_is_rejected() {
    let (index, storage) = hnsw(HnswConfig::METRIC_HAMMING, 64, 5);
    assert!(decode(&encode_hnsw(&index, &storage, true)).is_err());
}

#[test]
fn rejects_short_garbage_and_future_versions() {
    assert!(matches!(
        decode(&[]),
        Err(PersistenceError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        decode(b"EVWB\x01"),
        Err(PersistenceError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        decode(&[0x55; 64]),
        Err(PersistenceError::InvalidMagic { .. })
    ));

    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 3);
    let mut bytes = encode_hnsw(&index, &storage, false);
    bytes[4] = 2;
    assert!(matches!(
        decode(&bytes),
        Err(PersistenceError::UnsupportedVersion(2, 0))
    ));
}

#[test]
fn unknown_flags_from_newer_writer_are_unsupported_not_corrupt() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 3);
    for (byte, value) in [(5, 0x80u8), (6, 1), (7, 1)] {
        let mut bytes = encode_hnsw(&index, &storage, false);
        bytes[byte] = value;
        reseal(&mut bytes);
        assert!(
            matches!(decode(&bytes), Err(PersistenceError::Unsupported(_))),
            "byte {byte}"
        );
    }
}

#[test]
fn crc_covers_header_and_payload() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 10);
    let bytes = encode_hnsw(&index, &storage, false);
    for i in 0..bytes.len() {
        for bit in 0..8 {
            let mut corrupt = bytes.clone();
            corrupt[i] ^= 1 << bit;
            assert!(
                decode(&corrupt).is_err(),
                "flip at byte {i} bit {bit} accepted"
            );
        }
    }
    // A flipped BQ flag in particular must not silently toggle BQ.
    let mut flipped = bytes;
    flipped[5] ^= FLAG_BQ_ENABLED;
    assert!(matches!(
        decode(&flipped),
        Err(PersistenceError::ChecksumMismatch { .. })
    ));
}

#[test]
fn rejects_truncation_and_trailing_bytes() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 10);
    let bytes = encode_hnsw(&index, &storage, false);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "prefix {end} accepted");
    }
    let mut longer = bytes;
    longer.push(0);
    assert!(decode(&longer).is_err());
    longer.truncate(longer.len() - 1);
    longer.push(0);
    reseal(&mut longer);
    assert!(matches!(
        decode(&longer),
        Err(PersistenceError::Corrupted(_))
    ));
}

// =============================================================================
// Legacy layouts (synthesized; real-binary fixtures are in `fixtures` below)
// =============================================================================

#[derive(Serialize)]
struct Legacy090<'a> {
    inner: Legacy090Variant<'a>,
    metadata: &'a MetadataStore,
}

#[derive(Serialize)]
#[serde(tag = "variant_type")]
enum Legacy090Variant<'a> {
    #[serde(rename = "hnsw")]
    Hnsw {
        index: &'a HnswIndex,
        storage: &'a VectorStorage,
    },
    #[serde(rename = "flat")]
    Flat { index: &'a BinaryFlatIndex },
}

#[derive(Serialize)]
struct StorageV08Ref<'a> {
    data_f32: &'a [f32],
    quantized_data: &'a [u8],
    config: &'a StorageType,
    deleted: &'a BitVec,
    dimensions: u32,
    next_id: u64,
}

#[test]
fn legacy_090_hnsw_and_flat() {
    let (index, storage) = hnsw(HnswConfig::METRIC_COSINE, 16, 30);
    let m = metadata();
    let bytes = postcard::to_stdvec(&Legacy090 {
        inner: Legacy090Variant::Hnsw {
            index: &index,
            storage: &storage,
        },
        metadata: &m,
    })
    .unwrap();
    let r = decode(&bytes).unwrap();
    assert_eq!(r.source, SourceFormat::Legacy090);
    assert_eq!(r.metadata, m);

    let f = flat(4);
    let bytes = postcard::to_stdvec(&Legacy090 {
        inner: Legacy090Variant::Flat { index: &f },
        metadata: &m,
    })
    .unwrap();
    assert!(matches!(
        decode(&bytes).unwrap().index,
        PersistedIndex::Flat { .. }
    ));
}

#[test]
fn legacy_060_hnsw() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 16, 30);
    let m = metadata();
    let bytes = postcard::to_stdvec(&(
        &index,
        StorageV08Ref {
            data_f32: &storage.data_f32,
            quantized_data: &storage.quantized_data,
            config: &storage.config,
            deleted: &storage.deleted,
            dimensions: storage.dimensions,
            next_id: storage.next_id,
        },
        &m,
    ))
    .unwrap();
    let r = decode(&bytes).unwrap();
    assert_eq!(r.source, SourceFormat::Legacy060);
    assert_eq!(r.metadata, m);
    let PersistedIndex::Hnsw { storage: s, .. } = r.index else {
        panic!("expected HNSW");
    };
    assert_eq!(s.data_f32, storage.data_f32);
}

#[test]
fn legacy_truncations_are_rejected() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 5);
    let m = metadata();
    let bytes = postcard::to_stdvec(&Legacy090 {
        inner: Legacy090Variant::Hnsw {
            index: &index,
            storage: &storage,
        },
        metadata: &m,
    })
    .unwrap();
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "prefix {end} accepted");
    }
}

// =============================================================================
// Structural validation — each case panicked in search/insert before (review C1)
// =============================================================================

fn assert_rejected(bytes: &[u8], what: &str) {
    assert!(decode(bytes).is_err(), "{what}: accepted");
    exercise(bytes); // and never panics
}

#[test]
fn rejects_short_vector_data() {
    assert_rejected(&tampered(|_, s| s.data_f32.truncate(8)), "short data_f32");
}

#[test]
fn rejects_nan_vectors() {
    assert_rejected(&tampered(|_, s| s.data_f32[3] = f32::NAN), "NaN");
    assert_rejected(&tampered(|_, s| s.data_f32[3] = f32::INFINITY), "Inf");
}

#[test]
fn rejects_storage_type_metric_mismatch() {
    assert_rejected(
        &tampered(|_, s| s.config = StorageType::Binary(8)),
        "binary storage on L2",
    );
    assert_rejected(
        &tampered(|i, _| i.config.metric = HnswConfig::METRIC_HAMMING),
        "hamming on f32 storage",
    );
    assert_rejected(&tampered(|i, _| i.config.metric = 77), "unknown metric");
}

#[test]
fn rejects_scalar_quantized_storage() {
    let bytes = tampered(|_, s| {
        s.config = StorageType::QuantizedU8(QuantizerConfig { min: 0.0, max: 1.0 });
    });
    assert!(matches!(
        decode(&bytes),
        Err(PersistenceError::Unsupported(_))
    ));
}

#[test]
fn normalizes_next_id() {
    for next in [0, 5, 1000] {
        let (mut d, mut s, _) = restored_hnsw(&tampered(|_, s| s.next_id = next));
        assert_eq!(s.next_id, 41);
        let id = d.insert(&vector(7, 8), &mut s).unwrap();
        assert_eq!(id, VectorId(41));
    }
}

#[test]
fn rejects_bad_nodes() {
    assert_rejected(
        &tampered(|i, _| i.nodes[0].vector_id = VectorId(999)),
        "dangling vector id",
    );
    assert_rejected(
        &tampered(|i, _| i.nodes[0].vector_id = VectorId(0)),
        "vector id 0",
    );
    assert_rejected(
        &tampered(|i, _| i.nodes[1].vector_id = i.nodes[0].vector_id),
        "duplicate vector id",
    );
    assert_rejected(&tampered(|i, _| i.nodes[2].deleted = 9), "deleted flag");
    assert_rejected(
        &tampered(|i, _| i.nodes[1].neighbor_offset = u32::MAX - 1),
        "neighbor offset overflow",
    );
    assert_rejected(
        &tampered(|i, _| {
            let last = i.nodes.len() - 1;
            i.nodes[last].neighbor_len = u16::MAX;
        }),
        "neighbor range past pool",
    );
}

#[test]
fn rejects_bad_neighbor_lists() {
    // Point a node's level-0 list at a non-existent node id.
    assert_rejected(
        &tampered(|i, _| {
            let off = i.nodes[0].neighbor_offset as usize;
            i.neighbors.buffer[off] = 1; // count = 1
            i.neighbors.buffer[off + 1] = 0x7F; // neighbor 127 (only 40 nodes)
        }),
        "dangling neighbor",
    );
    // Neighbor referenced at a layer above its own max_layer.
    assert_rejected(
        &tampered(|i, _| {
            let top = i
                .nodes
                .iter()
                .skip(1)
                .position(|n| n.max_layer == 0)
                .unwrap()
                + 1;
            let node = &mut i.nodes[0];
            node.max_layer = 1;
            let off = node.neighbor_offset as usize;
            // L0: empty, L1: [top]
            #[allow(clippy::cast_possible_truncation)]
            let enc = [0u8, 1, top as u8];
            i.neighbors.buffer[off..off + 3].copy_from_slice(&enc);
        }),
        "neighbor layer above its max_layer",
    );
    // Unterminated varint.
    assert_rejected(
        &tampered(|i, _| {
            let n = &i.nodes[0];
            let (off, len) = (n.neighbor_offset as usize, n.neighbor_len as usize);
            i.neighbors.buffer[off..off + len].fill(0xFF);
        }),
        "unterminated varint",
    );
}

#[test]
fn rejects_bad_free_list() {
    assert_rejected(&tampered(|i, _| i.neighbors.buckets.clear()), "no buckets");
    assert_rejected(
        &tampered(|i, _| i.neighbors.buckets[0].push(u32::MAX)),
        "free offset out of pool",
    );
}

#[test]
fn entry_point_and_counters() {
    assert_rejected(
        &tampered(|i, _| i.entry_point = Some(NodeId(40))),
        "entry out of range",
    );
    assert_rejected(&tampered(|i, _| i.entry_point = None), "missing entry");

    let (d, _, _) = restored_hnsw(&tampered(|i, _| {
        i.nodes[3].deleted = 1;
        i.deleted_count = 999;
        i.max_layer = 200;
    }));
    assert_eq!(d.deleted_count, 1);
    assert_eq!(
        d.max_layer,
        d.nodes[d.entry_point.unwrap().0 as usize].max_layer
    );
}

#[test]
fn rejects_bad_config() {
    assert_rejected(&tampered(|i, _| i.config.m = 1), "m <= 1");
    assert_rejected(&tampered(|i, _| i.config.m0 = 2), "m0 < m");
    assert_rejected(&tampered(|i, _| i.level_mult = f32::NAN), "level_mult");
    assert_rejected(
        &tampered(|i, s| {
            i.config.dimensions = 4;
            s.dimensions = 4;
        }),
        "dimension change without data change",
    );
}

#[test]
fn rejects_invalid_metadata() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 3);
    // The insert API refuses these; JSON lets us build them anyway.
    let cases = [
        r#"{"data":{"1":{"bad key!":{"type":"integer","value":1}}}}"#,
        r#"{"data":{"1":{"":{"type":"integer","value":1}}}}"#,
        r#"{"data":{"1":{"k":{"type":"float","value":1e309}}}}"#,
    ];
    for json in cases {
        let Ok(bad) = serde_json::from_str::<MetadataStore>(json) else {
            continue; // e.g. 1e309 is not representable in JSON input
        };
        let bytes = encode(
            PersistedIndexRef::Hnsw {
                index: &index,
                storage: &storage,
            },
            &bad,
            false,
        )
        .unwrap();
        assert_rejected(&bytes, json);
    }
    let mut too_many = String::from(r#"{"data":{"1":{"#);
    for k in 0..=MAX_KEYS_PER_VECTOR {
        if k > 0 {
            too_many.push(',');
        }
        write!(too_many, r#""k{k}":{{"type":"integer","value":1}}"#).unwrap();
    }
    too_many.push_str("}}}");
    let bad: MetadataStore = serde_json::from_str(&too_many).unwrap();
    let bytes = encode(
        PersistedIndexRef::Hnsw {
            index: &index,
            storage: &storage,
        },
        &bad,
        false,
    )
    .unwrap();
    assert_rejected(&bytes, "too many keys");
}

#[test]
fn flat_validation() {
    let mut bytes = postcard::to_stdvec(&flat(1)).unwrap();
    let last = bytes.len() - 1;
    bytes[last] = 7; // count 1 -> 7 with one vector of data
    let tampered: BinaryFlatIndex = postcard::from_bytes(&bytes).unwrap();
    assert!(validate_flat(&tampered).is_err());
    validate_flat(&flat(3)).unwrap();
    validate_flat(&BinaryFlatIndex::new(64).unwrap()).unwrap();
    assert!(validate_flat(&BinaryFlatIndex::new(0).unwrap()).is_err());
}

// =============================================================================
// Mutation harness: corrupted payloads that pass the CRC must never panic
// =============================================================================

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0 >> 33
    }
    #[allow(clippy::cast_possible_truncation)]
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn seeds() -> Vec<Vec<u8>> {
    let (l2, l2s) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 40);
    let (mut cos, cos_s) = hnsw(HnswConfig::METRIC_COSINE, 16, 30);
    cos.soft_delete(VectorId(3)).unwrap();
    cos.enable_bq(&cos_s).unwrap();
    let (ham, ham_s) = hnsw(HnswConfig::METRIC_HAMMING, 32, 30);
    let f = flat(12);
    vec![
        encode_hnsw(&l2, &l2s, false),
        encode_hnsw(&cos, &cos_s, true),
        encode_hnsw(&ham, &ham_s, false),
        encode(PersistedIndexRef::Flat { index: &f }, &metadata(), false).unwrap(),
    ]
}

#[test]
fn mutated_payloads_never_panic() {
    let iterations: usize = std::env::var("EDGEVEC_MUTATION_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4000);
    let mut rng = Lcg(0x5EED);
    let mut accepted = 0usize;
    for seed in seeds() {
        for _ in 0..iterations {
            let mut buf = seed.clone();
            match rng.below(4) {
                // Overwrite 1-4 random payload bytes.
                0 | 1 => {
                    for _ in 0..=rng.below(4) {
                        let i = BUFFER_HEADER_SIZE + rng.below(buf.len() - BUFFER_HEADER_SIZE);
                        #[allow(clippy::cast_possible_truncation)]
                        let b = rng.next() as u8;
                        buf[i] = b;
                    }
                }
                // Insert a byte.
                2 => {
                    let i = BUFFER_HEADER_SIZE + rng.below(buf.len() - BUFFER_HEADER_SIZE + 1);
                    #[allow(clippy::cast_possible_truncation)]
                    buf.insert(i, rng.next() as u8);
                }
                // Delete a byte.
                _ => {
                    if buf.len() > BUFFER_HEADER_SIZE {
                        let i = BUFFER_HEADER_SIZE + rng.below(buf.len() - BUFFER_HEADER_SIZE);
                        buf.remove(i);
                    }
                }
            }
            reseal(&mut buf);
            if decode(&buf).is_ok() {
                accepted += 1;
            }
            let result = std::panic::catch_unwind(|| exercise(&buf));
            assert!(result.is_ok(), "panic on mutated buffer: {buf:?}");
        }
    }
    // Sanity: the harness must reach post-decode code, not only reject.
    assert!(accepted > 0, "no mutated buffer was ever accepted");
}

#[test]
fn exercise_bounds_queries_for_large_empty_indexes() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, u32::MAX, 0);
    exercise(&encode_hnsw(&index, &storage, false));
    let index = BinaryFlatIndex::new(usize::MAX - 7).unwrap();
    exercise(
        &encode(
            PersistedIndexRef::Flat { index: &index },
            &MetadataStore::new(),
            false,
        )
        .unwrap(),
    );
}

#[test]
fn optimized_decoder_matches_derived_wire_schema() {
    for bytes in seeds() {
        let (_, payload) = parse_header(&bytes).unwrap();
        let reference: Snapshot = decode_exact(payload).unwrap();
        let fast = decode_snapshot(payload).unwrap();
        assert_eq!(
            serde_json::to_value(&reference.metadata).unwrap(),
            serde_json::to_value(&fast.metadata).unwrap()
        );
        match (reference.index, fast.index) {
            (
                PersistedIndex::Hnsw {
                    index: a,
                    storage: sa,
                },
                PersistedIndex::Hnsw {
                    index: b,
                    storage: sb,
                },
            ) => {
                assert_eq!(
                    serde_json::to_value(a).unwrap(),
                    serde_json::to_value(b).unwrap()
                );
                assert_eq!(
                    postcard::to_stdvec(&sa).unwrap(),
                    postcard::to_stdvec(&sb).unwrap()
                );
            }
            (PersistedIndex::Flat { index: a }, PersistedIndex::Flat { index: b }) => {
                assert_eq!(
                    postcard::to_stdvec(&a).unwrap(),
                    postcard::to_stdvec(&b).unwrap()
                );
            }
            _ => panic!("variant changed"),
        }
    }
}

#[test]
fn rejects_overlapping_neighbor_allocations() {
    let live_alias = tampered(|index, _| {
        index.neighbors.buckets[0].push(index.nodes[0].neighbor_offset);
    });
    assert!(decode(&live_alias)
        .err()
        .unwrap()
        .to_string()
        .contains("overlap"));
    let (mut index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 0);
    index.neighbors.buffer.resize(32, 0);
    index.neighbors.buckets[0] = vec![0, 0];
    assert!(decode(&encode_hnsw(&index, &storage, false))
        .err()
        .unwrap()
        .to_string()
        .contains("overlap"));
}

#[test]
fn legacy_mutations_never_panic() {
    let (index, storage) = hnsw(HnswConfig::METRIC_L2_SQUARED, 8, 20);
    let m = metadata();
    let seed = postcard::to_stdvec(&Legacy090 {
        inner: Legacy090Variant::Hnsw {
            index: &index,
            storage: &storage,
        },
        metadata: &m,
    })
    .unwrap();
    let mut rng = Lcg(0x00C0_FFEE);
    for _ in 0..3000 {
        let mut buf = seed.clone();
        for _ in 0..=rng.below(3) {
            let i = rng.below(buf.len());
            #[allow(clippy::cast_possible_truncation)]
            let b = rng.next() as u8;
            buf[i] = b;
        }
        let result = std::panic::catch_unwind(|| exercise(&buf));
        assert!(result.is_ok(), "panic on mutated legacy buffer: {buf:?}");
    }
}

// =============================================================================
// Golden fixtures captured from real builds (tests/fixtures/wasm_buffers)
// =============================================================================

mod fixtures {
    use super::*;
    use serde_json::Value;

    const EXPECTED: &str = include_str!("../../../tests/fixtures/wasm_buffers/expected.json");

    fn fixture(name: &str) -> (&'static [u8], Value, SourceFormat) {
        let bytes: &'static [u8] = match name {
            "v0_8_0_hnsw_l2" => {
                include_bytes!("../../../tests/fixtures/wasm_buffers/v0_8_0_hnsw_l2.bin")
            }
            "v0_9_0_hnsw_l2" => {
                include_bytes!("../../../tests/fixtures/wasm_buffers/v0_9_0_hnsw_l2.bin")
            }
            "v0_9_0_flat" => include_bytes!("../../../tests/fixtures/wasm_buffers/v0_9_0_flat.bin"),
            "v1_hnsw_l2" => include_bytes!("../../../tests/fixtures/wasm_buffers/v1_hnsw_l2.bin"),
            "v1_hnsw_cosine_bq" => {
                include_bytes!("../../../tests/fixtures/wasm_buffers/v1_hnsw_cosine_bq.bin")
            }
            "v1_flat" => include_bytes!("../../../tests/fixtures/wasm_buffers/v1_flat.bin"),
            other => panic!("unknown fixture {other}"),
        };
        let source = if name.starts_with("v0_8") {
            SourceFormat::Legacy060
        } else if name.starts_with("v0_9") {
            SourceFormat::Legacy090
        } else {
            SourceFormat::V1
        };
        let all: Value = serde_json::from_str(EXPECTED).unwrap();
        (bytes, all[name].clone(), source)
    }

    fn to_json(v: &MetadataValue) -> Value {
        match v {
            MetadataValue::String(s) => Value::from(s.as_str()),
            MetadataValue::Integer(i) => Value::from(*i as f64),
            MetadataValue::Float(f) => Value::from(*f),
            MetadataValue::Boolean(b) => Value::from(*b),
            MetadataValue::StringArray(a) => Value::from(a.clone()),
        }
    }

    fn as_f64_json(v: &Value) -> Value {
        v.as_f64().map_or_else(|| v.clone(), Value::from)
    }

    fn check_hnsw(name: &str) {
        let (bytes, exp, source) = fixture(name);
        let restored = decode(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(restored.source, source, "{name}");
        let PersistedIndex::Hnsw { index, storage } = restored.index else {
            panic!("{name}: expected HNSW");
        };
        assert_eq!(
            index.live_count() as u64,
            exp["liveCount"].as_u64().unwrap(),
            "{name}"
        );
        for id in exp["deleted"].as_array().unwrap() {
            let id = VectorId(id.as_u64().unwrap());
            assert!(index.is_deleted(id).unwrap(), "{name}: {id:?} not deleted");
        }
        assert_eq!(
            index.bq_storage.is_some(),
            exp["bq"].as_bool().unwrap(),
            "{name}"
        );

        // Each query is a stored vector: it must come back first (exact match).
        for (q, want) in exp["queries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(exp["results"].as_array().unwrap())
        {
            #[allow(clippy::cast_possible_truncation)]
            let q: Vec<f32> = q
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect();
            let got = index.search(&q, 10, &storage).unwrap();
            assert_eq!(
                got[0].vector_id.0,
                want[0].as_u64().unwrap(),
                "{name}: top-1"
            );
        }

        for (id, want) in exp["metadata"].as_object().unwrap() {
            let id: u32 = id.parse().unwrap();
            let want = want.as_object().unwrap();
            let got = restored.metadata.get_all(id);
            assert_eq!(
                got.map_or(0, HashMap::len),
                want.len(),
                "{name}: keys of {id}"
            );
            for (key, value) in want {
                let actual = got.and_then(|m| m.get(key)).map(to_json);
                assert_eq!(
                    actual.as_ref().map(as_f64_json),
                    Some(as_f64_json(value)),
                    "{name}: {id}.{key}"
                );
            }
        }

        // Re-encoding upgrades to v1 and stays loadable.
        let reencoded = encode(
            PersistedIndexRef::Hnsw {
                index: &index,
                storage: &storage,
            },
            &restored.metadata,
            index.bq_storage.is_some(),
        )
        .unwrap();
        assert_eq!(decode(&reencoded).unwrap().source, SourceFormat::V1);
    }

    fn check_flat(name: &str) {
        let (bytes, exp, source) = fixture(name);
        let restored = decode(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(restored.source, source);
        let PersistedIndex::Flat { index } = restored.index else {
            panic!("{name}: expected flat");
        };
        for (q, want) in exp["queries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(exp["results"].as_array().unwrap())
        {
            #[allow(clippy::cast_possible_truncation)]
            let q: Vec<u8> = q
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u8)
                .collect();
            let got: Vec<u64> = index
                .search(&q, 5)
                .unwrap()
                .iter()
                .map(|r| r.id.0)
                .collect();
            let want: Vec<u64> = want
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap())
                .collect();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn v0_8_0_save_loads() {
        check_hnsw("v0_8_0_hnsw_l2");
    }

    #[test]
    fn v0_9_0_saves_load() {
        check_hnsw("v0_9_0_hnsw_l2");
        check_flat("v0_9_0_flat");
    }

    /// Locks the v1 format: if a struct change alters the wire layout, these
    /// fail instead of silently breaking users' saved data (review M3).
    #[test]
    fn v1_format_is_stable() {
        check_hnsw("v1_hnsw_l2");
        check_hnsw("v1_hnsw_cosine_bq");
        check_flat("v1_flat");
    }

    #[test]
    fn fixture_mutations_never_panic() {
        let mut rng = Lcg(0xF1);
        for name in ["v0_8_0_hnsw_l2", "v0_9_0_hnsw_l2", "v0_9_0_flat"] {
            let (seed, _, _) = fixture(name);
            for _ in 0..1500 {
                let mut buf = seed.to_vec();
                for _ in 0..=rng.below(3) {
                    let i = rng.below(buf.len());
                    #[allow(clippy::cast_possible_truncation)]
                    let b = rng.next() as u8;
                    buf[i] = b;
                }
                let result = std::panic::catch_unwind(|| exercise(&buf));
                assert!(result.is_ok(), "{name}: panic on mutated fixture {buf:?}");
            }
        }
    }
}
