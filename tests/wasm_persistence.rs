//! Buffer persistence (`toBytes` / `fromBytes`) regression tests — issues #11, #12.
//!
//! Runs under Node (no IndexedDB needed):
//! `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test
//! --target wasm32-unknown-unknown --test wasm_persistence`
//!
//! The IndexedDB shim (`src/js/storage.js`) is covered by
//! `tests/js/storage.test.mjs`.
#![cfg(target_arch = "wasm32")]

use edgevec::metadata::{MetadataStore, MetadataValue};
use edgevec::wasm::{EdgeVec, EdgeVecConfig, JsIndexType};
use edgevec::{BinaryFlatIndex, HnswConfig, HnswIndex, VectorStorage};
use js_sys::{Array, Float32Array, Object, Reflect, Uint8Array};
use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn vector(seed: u32, dim: u32) -> Vec<f32> {
    (0..dim)
        .map(|d| ((seed * 31 + d * 17) % 97) as f32 / 97.0 - 0.5)
        .collect()
}

fn search_ids(db: &EdgeVec, query: &[f32], k: usize) -> Vec<u32> {
    let results = Array::from(&db.search(Float32Array::from(query), k).unwrap());
    results
        .iter()
        .map(|r| Reflect::get(&r, &"id".into()).unwrap().as_f64().unwrap() as u32)
        .collect()
}

fn error_code(err: &JsValue) -> String {
    Reflect::get(err, &"code".into())
        .unwrap()
        .as_string()
        .unwrap_or_default()
}

fn hnsw_db(dim: u32, metric: &str, n: u32) -> EdgeVec {
    let mut config = EdgeVecConfig::new(dim);
    config.set_metric(metric.to_string());
    let mut db = EdgeVec::new(&config).unwrap();
    for i in 0..n {
        db.insert(Float32Array::from(&vector(i, dim)[..])).unwrap();
    }
    db
}

/// Exact scenario from issue #12: empty 768-d cosine index.
#[wasm_bindgen_test]
fn empty_index_roundtrip() {
    let db = hnsw_db(768, "cosine", 0);
    let restored = EdgeVec::from_bytes(&db.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.dimensions(), 768);
    assert_eq!(restored.live_count().unwrap(), 0);
}

#[wasm_bindgen_test]
fn zero_dimensions_are_rejected_at_construction() {
    for kind in [JsIndexType::Hnsw, JsIndexType::Flat] {
        let mut config = EdgeVecConfig::new(0);
        config.set_index_type(kind);
        let error = EdgeVec::new(&config).err().expect("zero dimensions");
        assert_eq!(error_code(&error), "ERR_VALIDATION");
    }
}

#[wasm_bindgen_test]
fn hnsw_roundtrip_preserves_search_deletes_and_ids() {
    let mut db = hnsw_db(32, "cosine", 200);
    db.soft_delete(3).unwrap();
    db.soft_delete(150).unwrap();

    let bytes = db.to_bytes().unwrap();
    assert_eq!(&bytes[..4], b"EVWB");

    let mut restored = EdgeVec::from_bytes(&bytes).unwrap();
    for seed in [1000, 1001, 1002] {
        let q = vector(seed, 32);
        assert_eq!(search_ids(&db, &q, 10), search_ids(&restored, &q, 10));
    }
    assert!(restored.is_deleted(3).unwrap());
    assert!(restored.is_deleted(150).unwrap());
    assert!(!restored.is_deleted(4).unwrap());
    assert_eq!(restored.live_count().unwrap(), 198);

    let next = restored
        .insert(Float32Array::from(&vector(9999, 32)[..]))
        .unwrap();
    assert_eq!(next, 201, "ids must continue after reload");
}

#[wasm_bindgen_test]
fn metadata_survives_roundtrip() {
    let mut db = hnsw_db(16, "l2", 0);
    for i in 0..20u32 {
        let meta = Object::new();
        Reflect::set(&meta, &"title".into(), &format!("doc-{i}").into()).unwrap();
        Reflect::set(&meta, &"n".into(), &JsValue::from(f64::from(i))).unwrap();
        Reflect::set(&meta, &"score".into(), &JsValue::from(0.25)).unwrap();
        Reflect::set(&meta, &"ok".into(), &JsValue::from(i % 2 == 0)).unwrap();
        let tags = Array::of2(&"a".into(), &"b".into());
        Reflect::set(&meta, &"tags".into(), &tags).unwrap();
        db.insert_with_metadata(Float32Array::from(&vector(i, 16)[..]), meta.into())
            .unwrap();
    }

    let restored = EdgeVec::from_bytes(&db.to_bytes().unwrap()).unwrap();
    for id in [1u32, 7, 20] {
        let before = db.get_all_metadata(id);
        let after = restored.get_all_metadata(id);
        for key in ["title", "n", "score", "ok", "tags"] {
            let a = js_sys::JSON::stringify(&Reflect::get(&before, &key.into()).unwrap()).unwrap();
            let b = js_sys::JSON::stringify(&Reflect::get(&after, &key.into()).unwrap()).unwrap();
            assert_eq!(a, b, "metadata {id}.{key}");
            assert_ne!(a, "undefined", "metadata {id}.{key} missing");
        }
    }
}

#[wasm_bindgen_test]
fn bq_state_is_restored() {
    let mut db = hnsw_db(64, "l2", 100);
    db.enable_bq().unwrap();
    let restored = EdgeVec::from_bytes(&db.to_bytes().unwrap()).unwrap();
    assert!(restored.has_bq());

    let q = Float32Array::from(&vector(5000, 64)[..]);
    let a = js_sys::JSON::stringify(&db.search_bq(q.clone(), 10).unwrap()).unwrap();
    let b = js_sys::JSON::stringify(&restored.search_bq(q, 10).unwrap()).unwrap();
    assert_eq!(a, b);

    let plain = hnsw_db(64, "l2", 10);
    assert!(!EdgeVec::from_bytes(&plain.to_bytes().unwrap())
        .unwrap()
        .has_bq());
}

#[wasm_bindgen_test]
fn flat_index_roundtrip() {
    let mut config = EdgeVecConfig::new(128);
    config.set_index_type(JsIndexType::Flat);
    let mut db = EdgeVec::new(&config).unwrap();
    for i in 0..30u8 {
        let v: Vec<u8> = (0..16)
            .map(|j| i.wrapping_mul(13).wrapping_add(j))
            .collect();
        db.insert_binary(Uint8Array::from(&v[..])).unwrap();
    }
    let restored = EdgeVec::from_bytes(&db.to_bytes().unwrap()).unwrap();
    assert_eq!(restored.dimensions(), 128);

    let q: Vec<u8> = (0..16)
        .map(|j| 7u8.wrapping_mul(13).wrapping_add(j))
        .collect();
    let a =
        js_sys::JSON::stringify(&db.search_binary(Uint8Array::from(&q[..]), 5).unwrap()).unwrap();
    let b = js_sys::JSON::stringify(&restored.search_binary(Uint8Array::from(&q[..]), 5).unwrap())
        .unwrap();
    assert_eq!(a, b);
}

#[wasm_bindgen_test]
fn corrupted_buffers_are_rejected_with_error_code() {
    let bytes = hnsw_db(8, "l2", 20).to_bytes().unwrap();

    let mut flipped = bytes.clone();
    let i = flipped.len() - 5;
    flipped[i] ^= 0x01;

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("header only", bytes[..10].to_vec()),
        ("truncated", bytes[..bytes.len() - 1].to_vec()),
        ("bit flip", flipped),
        ("garbage", (0..64u8).map(|b| b.wrapping_mul(37)).collect()),
    ];
    for (name, case) in cases {
        let err = EdgeVec::from_bytes(&case).err().expect(name);
        assert_eq!(error_code(&err), "ERR_CORRUPTION", "{name}");
    }

    let mut future = bytes;
    future[4] = 99;
    let err = EdgeVec::from_bytes(&future).err().unwrap();
    assert_eq!(error_code(&err), "ERR_PERSISTENCE");
}

/// Replica of the EdgeVec 0.9.0 save() layout for targeted corruption tests.
#[derive(Serialize)]
struct LegacyEdgeVec<'a> {
    inner: LegacyVariant<'a>,
    metadata: &'a MetadataStore,
}

#[derive(Serialize)]
#[serde(tag = "variant_type")]
enum LegacyVariant<'a> {
    #[serde(rename = "hnsw")]
    Hnsw {
        index: &'a HnswIndex,
        storage: &'a VectorStorage,
    },
    #[serde(rename = "flat")]
    #[allow(dead_code)]
    Flat { index: &'a BinaryFlatIndex },
}

#[wasm_bindgen_test]
fn loads_buffers_written_by_0_9_0() {
    let config = HnswConfig::new(8);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();
    for i in 0..40 {
        index.insert(&vector(i, 8), &mut storage).unwrap();
    }
    let mut metadata = MetadataStore::new();
    metadata
        .insert(2, "title", MetadataValue::String("old".into()))
        .unwrap();

    let legacy = postcard::to_stdvec(&LegacyEdgeVec {
        inner: LegacyVariant::Hnsw {
            index: &index,
            storage: &storage,
        },
        metadata: &metadata,
    })
    .unwrap();

    let restored = EdgeVec::from_bytes(&legacy).unwrap();
    assert_eq!(restored.live_count().unwrap(), 40);
    assert_eq!(
        restored.get_metadata(2, "title").unwrap().as_string(),
        Some("old".to_string())
    );

    // Re-saving upgrades to the current format.
    let upgraded = restored.to_bytes().unwrap();
    assert_eq!(&upgraded[..4], b"EVWB");
    EdgeVec::from_bytes(&upgraded).unwrap();
}

/// Real releases and the v1 writer captured these buffers. These assertions
/// guard the wire schema independently of the types compiled into this test.
#[wasm_bindgen_test]
fn golden_fixtures_preserve_search_and_metadata() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/wasm_buffers/expected.json")).unwrap();
    let fixtures: [(&str, &[u8]); 6] = [
        (
            "v0_8_0_hnsw_l2",
            include_bytes!("fixtures/wasm_buffers/v0_8_0_hnsw_l2.bin"),
        ),
        (
            "v0_9_0_hnsw_l2",
            include_bytes!("fixtures/wasm_buffers/v0_9_0_hnsw_l2.bin"),
        ),
        (
            "v0_9_0_flat",
            include_bytes!("fixtures/wasm_buffers/v0_9_0_flat.bin"),
        ),
        (
            "v1_hnsw_l2",
            include_bytes!("fixtures/wasm_buffers/v1_hnsw_l2.bin"),
        ),
        (
            "v1_hnsw_cosine_bq",
            include_bytes!("fixtures/wasm_buffers/v1_hnsw_cosine_bq.bin"),
        ),
        (
            "v1_flat",
            include_bytes!("fixtures/wasm_buffers/v1_flat.bin"),
        ),
    ];
    for (name, bytes) in fixtures {
        let exp = &expected[name];
        let restored = EdgeVec::from_bytes(bytes).unwrap();
        assert_eq!(
            u64::from(restored.dimensions()),
            exp["dims"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            restored.has_bq(),
            exp["bq"].as_bool().unwrap_or(false),
            "{name}"
        );
        if exp["kind"] == "hnsw" {
            assert_eq!(
                u64::from(restored.live_count().unwrap()),
                exp["liveCount"].as_u64().unwrap(),
                "{name}"
            );
            for id in exp["deleted"].as_array().unwrap() {
                assert!(
                    restored
                        .is_deleted(u32::try_from(id.as_u64().unwrap()).unwrap())
                        .unwrap(),
                    "{name}"
                );
            }
        }
        for (query, want) in exp["queries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(exp["results"].as_array().unwrap())
        {
            let got: Vec<u32> = if exp["kind"] == "flat" {
                let q: Vec<u8> = query
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
                    .collect();
                Array::from(
                    &restored
                        .search_binary(Uint8Array::from(q.as_slice()), 5)
                        .unwrap(),
                )
                .iter()
                .map(|r| Reflect::get(&r, &"id".into()).unwrap().as_f64().unwrap() as u32)
                .collect()
            } else {
                let q: Vec<f32> = query
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect();
                search_ids(&restored, &q, 10)
            };
            let want: Vec<u32> = want
                .as_array()
                .unwrap()
                .iter()
                .map(|v| u32::try_from(v.as_u64().unwrap()).unwrap())
                .collect();
            assert_eq!(got, want, "{name}: full search ordering");
        }
        for (id, want) in exp["metadata"].as_object().into_iter().flatten() {
            let got = restored.get_all_metadata(id.parse().unwrap());
            // The API returns null for an ID without metadata; the fixture
            // generator normalizes that to an empty object for stable JSON.
            if got.is_null() || got.is_undefined() {
                assert!(
                    want.as_object().unwrap().is_empty(),
                    "{name}: missing metadata of {id}"
                );
                continue;
            }
            let want_js = js_sys::JSON::parse(&serde_json::to_string(want).unwrap()).unwrap();
            let keys = Object::keys(&Object::from(got.clone()));
            assert_eq!(
                keys.length() as usize,
                want.as_object().unwrap().len(),
                "{name}: metadata keys of {id}"
            );
            for key in want.as_object().unwrap().keys() {
                let actual = Reflect::get(&got, &key.as_str().into()).unwrap();
                let expected = Reflect::get(&want_js, &key.as_str().into()).unwrap();
                assert_eq!(
                    js_sys::JSON::stringify(&actual).unwrap(),
                    js_sys::JSON::stringify(&expected).unwrap(),
                    "{name}: metadata {id}.{key}"
                );
            }
        }
    }
}

/// Offset arithmetic must reject this on 32-bit WASM as well as on native.
#[wasm_bindgen_test]
fn neighbor_offset_overflow_returns_corruption() {
    let config = HnswConfig::new(8);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();
    index.insert(&vector(0, 8), &mut storage).unwrap();
    let mut json = serde_json::to_value(&index).unwrap();
    json["nodes"][0]["neighbor_offset"] = serde_json::json!(0xffff_fff0u32);
    json["nodes"][0]["neighbor_len"] = serde_json::json!(0x20u16);
    let index: HnswIndex = serde_json::from_value(json).unwrap();
    let legacy = postcard::to_stdvec(&LegacyEdgeVec {
        inner: LegacyVariant::Hnsw {
            index: &index,
            storage: &storage,
        },
        metadata: &MetadataStore::new(),
    })
    .unwrap();
    let error = EdgeVec::from_bytes(&legacy)
        .err()
        .expect("overflow must be rejected");
    assert_eq!(error_code(&error), "ERR_CORRUPTION");
}
