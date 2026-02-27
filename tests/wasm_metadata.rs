#![cfg(target_arch = "wasm32")]
//! WASM Metadata API Tests (W21.3)
//!
//! Tests the JavaScript-facing metadata API for EdgeVec.
//! Run with: wasm-pack test --headless --chrome

use edgevec::wasm::init_logging;
use edgevec::wasm::{EdgeVec, EdgeVecConfig, JsMetadataValue};
use js_sys::{Array, Float32Array, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// =============================================================================
// JsMetadataValue Factory Tests
// =============================================================================

#[wasm_bindgen_test]
fn test_metadata_value_from_string() {
    let value = JsMetadataValue::from_string("hello".to_string());
    assert!(value.is_string());
    assert_eq!(value.get_type(), "string");
    assert_eq!(value.as_string(), Some("hello".to_string()));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer() {
    let value = JsMetadataValue::from_integer(42.0).expect("Should accept valid integer");
    assert!(value.is_integer());
    assert_eq!(value.get_type(), "integer");
    assert_eq!(value.as_integer(), Some(42.0));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer_negative() {
    let value = JsMetadataValue::from_integer(-1000.0).expect("Should accept negative integer");
    assert!(value.is_integer());
    assert_eq!(value.as_integer(), Some(-1000.0));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer_max_safe() {
    // MAX_SAFE_INTEGER in JavaScript: 2^53 - 1
    let max_safe = 9_007_199_254_740_991.0;
    let value = JsMetadataValue::from_integer(max_safe).expect("Should accept MAX_SAFE_INTEGER");
    assert!(value.is_integer());
    assert_eq!(value.as_integer(), Some(max_safe));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer_rejects_fractional() {
    let result = JsMetadataValue::from_integer(3.5);
    assert!(result.is_err(), "Should reject fractional value");
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer_rejects_nan() {
    let result = JsMetadataValue::from_integer(f64::NAN);
    assert!(result.is_err(), "Should reject NaN");
}

#[wasm_bindgen_test]
fn test_metadata_value_from_integer_rejects_infinity() {
    let result = JsMetadataValue::from_integer(f64::INFINITY);
    assert!(result.is_err(), "Should reject Infinity");
}

#[wasm_bindgen_test]
fn test_metadata_value_from_float() {
    let value = JsMetadataValue::from_float(3.125);
    assert!(value.is_float());
    assert_eq!(value.get_type(), "float");
    assert_eq!(value.as_float(), Some(3.125));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_boolean_true() {
    let value = JsMetadataValue::from_boolean(true);
    assert!(value.is_boolean());
    assert_eq!(value.get_type(), "boolean");
    assert_eq!(value.as_boolean(), Some(true));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_boolean_false() {
    let value = JsMetadataValue::from_boolean(false);
    assert!(value.is_boolean());
    assert_eq!(value.as_boolean(), Some(false));
}

#[wasm_bindgen_test]
fn test_metadata_value_from_string_array() {
    let arr = Array::new();
    arr.push(&JsValue::from_str("foo"));
    arr.push(&JsValue::from_str("bar"));
    arr.push(&JsValue::from_str("baz"));

    let value = JsMetadataValue::from_string_array(arr).expect("Should accept string array");
    assert!(value.is_string_array());
    assert_eq!(value.get_type(), "string_array");
}

#[wasm_bindgen_test]
fn test_metadata_value_from_string_array_rejects_non_strings() {
    let arr = Array::new();
    arr.push(&JsValue::from_str("foo"));
    arr.push(&JsValue::from(42)); // Not a string!

    let result = JsMetadataValue::from_string_array(arr);
    assert!(
        result.is_err(),
        "Should reject array with non-string elements"
    );
}

// =============================================================================
// JsMetadataValue toJS Conversion Tests
// =============================================================================

#[wasm_bindgen_test]
fn test_metadata_value_to_js_string() {
    let value = JsMetadataValue::from_string("test".to_string());
    let js_val = value.to_js();
    assert_eq!(js_val.as_string(), Some("test".to_string()));
}

#[wasm_bindgen_test]
fn test_metadata_value_to_js_integer() {
    let value = JsMetadataValue::from_integer(123.0).unwrap();
    let js_val = value.to_js();
    assert_eq!(js_val.as_f64(), Some(123.0));
}

#[wasm_bindgen_test]
fn test_metadata_value_to_js_float() {
    let value = JsMetadataValue::from_float(2.5);
    let js_val = value.to_js();
    assert_eq!(js_val.as_f64(), Some(2.5));
}

#[wasm_bindgen_test]
fn test_metadata_value_to_js_boolean() {
    let value = JsMetadataValue::from_boolean(true);
    let js_val = value.to_js();
    assert_eq!(js_val.as_bool(), Some(true));
}

#[wasm_bindgen_test]
fn test_metadata_value_to_js_string_array() {
    let arr = Array::new();
    arr.push(&JsValue::from_str("a"));
    arr.push(&JsValue::from_str("b"));

    let value = JsMetadataValue::from_string_array(arr).unwrap();
    let js_val = value.to_js();

    // Should be a JS Array
    assert!(js_sys::Array::is_array(&js_val));
    let result_arr: Array = js_val.into();
    assert_eq!(result_arr.length(), 2);
    assert_eq!(result_arr.get(0).as_string(), Some("a".to_string()));
    assert_eq!(result_arr.get(1).as_string(), Some("b".to_string()));
}

// =============================================================================
// EdgeVec Metadata Integration Tests
// =============================================================================

fn create_test_index() -> EdgeVec {
    init_logging();
    let config = EdgeVecConfig::new(3);
    EdgeVec::new(&config).unwrap()
}

fn insert_test_vector(index: &mut EdgeVec) -> u32 {
    let vec_data = vec![1.0f32, 0.0, 0.0];
    let vec_js = Float32Array::from(vec_data.as_slice());
    index.insert(vec_js).expect("Insert should succeed")
}

#[wasm_bindgen_test]
fn test_set_and_get_string_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("My Document".to_string());
    index
        .set_metadata(id, "title", &value)
        .expect("setMetadata should succeed");

    let retrieved = index.get_metadata(id, "title");
    assert!(retrieved.is_some(), "Should retrieve metadata");

    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_string());
    assert_eq!(retrieved.as_string(), Some("My Document".to_string()));
}

#[wasm_bindgen_test]
fn test_set_and_get_integer_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_integer(42.0).unwrap();
    index
        .set_metadata(id, "page_count", &value)
        .expect("setMetadata should succeed");

    let retrieved = index.get_metadata(id, "page_count");
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_integer());
    assert_eq!(retrieved.as_integer(), Some(42.0));
}

#[wasm_bindgen_test]
fn test_set_and_get_float_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_float(0.95);
    index
        .set_metadata(id, "score", &value)
        .expect("setMetadata should succeed");

    let retrieved = index.get_metadata(id, "score");
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_float());
    assert_eq!(retrieved.as_float(), Some(0.95));
}

#[wasm_bindgen_test]
fn test_set_and_get_boolean_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_boolean(true);
    index
        .set_metadata(id, "verified", &value)
        .expect("setMetadata should succeed");

    let retrieved = index.get_metadata(id, "verified");
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_boolean());
    assert_eq!(retrieved.as_boolean(), Some(true));
}

#[wasm_bindgen_test]
fn test_get_nonexistent_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let retrieved = index.get_metadata(id, "nonexistent");
    assert!(
        retrieved.is_none(),
        "Should return None for nonexistent key"
    );
}

#[wasm_bindgen_test]
fn test_metadata_upsert() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    // Set initial value
    let value1 = JsMetadataValue::from_string("Initial".to_string());
    index.set_metadata(id, "title", &value1).unwrap();

    // Overwrite with new value
    let value2 = JsMetadataValue::from_string("Updated".to_string());
    index.set_metadata(id, "title", &value2).unwrap();

    let retrieved = index.get_metadata(id, "title").unwrap();
    assert_eq!(retrieved.as_string(), Some("Updated".to_string()));
}

#[wasm_bindgen_test]
fn test_delete_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("test".to_string());
    index.set_metadata(id, "key", &value).unwrap();

    // Verify exists
    assert!(index.has_metadata(id, "key"));

    // Delete
    let deleted = index.delete_metadata(id, "key").unwrap();
    assert!(deleted, "Should return true when key existed");

    // Verify deleted
    assert!(!index.has_metadata(id, "key"));
    assert!(index.get_metadata(id, "key").is_none());
}

#[wasm_bindgen_test]
fn test_delete_metadata_idempotent() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    // Delete nonexistent key should not error
    let deleted = index.delete_metadata(id, "nonexistent").unwrap();
    assert!(!deleted, "Should return false when key didn't exist");
}

#[wasm_bindgen_test]
fn test_delete_all_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value1 = JsMetadataValue::from_string("a".to_string());
    let value2 = JsMetadataValue::from_integer(1.0).unwrap();
    index.set_metadata(id, "key1", &value1).unwrap();
    index.set_metadata(id, "key2", &value2).unwrap();

    assert_eq!(index.metadata_key_count(id), 2);

    let had_metadata = index.delete_all_metadata(id);
    assert!(had_metadata, "Should return true when vector had metadata");
    assert_eq!(index.metadata_key_count(id), 0);
}

#[wasm_bindgen_test]
fn test_has_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    assert!(!index.has_metadata(id, "key"));

    let value = JsMetadataValue::from_string("test".to_string());
    index.set_metadata(id, "key", &value).unwrap();

    assert!(index.has_metadata(id, "key"));
}

#[wasm_bindgen_test]
fn test_metadata_key_count() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    assert_eq!(index.metadata_key_count(id), 0);

    let value = JsMetadataValue::from_string("test".to_string());
    index.set_metadata(id, "key1", &value).unwrap();
    assert_eq!(index.metadata_key_count(id), 1);

    index.set_metadata(id, "key2", &value).unwrap();
    assert_eq!(index.metadata_key_count(id), 2);
}

#[wasm_bindgen_test]
fn test_metadata_vector_count() {
    let mut index = create_test_index();

    assert_eq!(index.metadata_vector_count(), 0);

    let id1 = insert_test_vector(&mut index);
    let id2 = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("test".to_string());
    index.set_metadata(id1, "key", &value).unwrap();
    assert_eq!(index.metadata_vector_count(), 1);

    index.set_metadata(id2, "key", &value).unwrap();
    assert_eq!(index.metadata_vector_count(), 2);
}

#[wasm_bindgen_test]
fn test_total_metadata_count() {
    let mut index = create_test_index();

    assert_eq!(index.total_metadata_count(), 0);

    let id1 = insert_test_vector(&mut index);
    let id2 = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("test".to_string());
    index.set_metadata(id1, "key1", &value).unwrap();
    index.set_metadata(id1, "key2", &value).unwrap();
    index.set_metadata(id2, "key1", &value).unwrap();

    assert_eq!(index.total_metadata_count(), 3);
}

#[wasm_bindgen_test]
fn test_get_all_metadata() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let str_val = JsMetadataValue::from_string("Document".to_string());
    let int_val = JsMetadataValue::from_integer(42.0).unwrap();
    let bool_val = JsMetadataValue::from_boolean(true);

    index.set_metadata(id, "title", &str_val).unwrap();
    index.set_metadata(id, "pages", &int_val).unwrap();
    index.set_metadata(id, "verified", &bool_val).unwrap();

    let all_metadata = index.get_all_metadata(id);

    // Should be a JS object
    assert!(all_metadata.is_object());

    // Check individual properties
    let title = Reflect::get(&all_metadata, &JsValue::from_str("title")).unwrap();
    assert_eq!(title.as_string(), Some("Document".to_string()));

    let pages = Reflect::get(&all_metadata, &JsValue::from_str("pages")).unwrap();
    assert_eq!(pages.as_f64(), Some(42.0));

    let verified = Reflect::get(&all_metadata, &JsValue::from_str("verified")).unwrap();
    assert_eq!(verified.as_bool(), Some(true));
}

#[wasm_bindgen_test]
fn test_get_all_metadata_nonexistent_vector() {
    let index = create_test_index();

    let all_metadata = index.get_all_metadata(999);
    assert!(
        all_metadata.is_undefined(),
        "Should return undefined for vector without metadata"
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[wasm_bindgen_test]
fn test_set_metadata_empty_key_fails() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("test".to_string());
    let result = index.set_metadata(id, "", &value);
    assert!(result.is_err(), "Should reject empty key");
}

#[wasm_bindgen_test]
fn test_set_metadata_invalid_key_fails() {
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    let value = JsMetadataValue::from_string("test".to_string());
    let result = index.set_metadata(id, "invalid key!", &value);
    assert!(result.is_err(), "Should reject key with invalid characters");
}

// =============================================================================
// Serialization Tests (M2 - Hostile Review Requirement)
// =============================================================================

#[wasm_bindgen_test]
async fn test_metadata_survives_save_load_roundtrip() {
    // Setup mock IndexedDB backend
    let setup_code = r#"
        class IndexedDbBackend {
            static storage = new Map();

            static async write(name, data) {
                IndexedDbBackend.storage.set(name, new Uint8Array(data));
            }

            static async read(name) {
                if (!IndexedDbBackend.storage.has(name)) {
                    throw new Error(`File not found: ${name}`);
                }
                return IndexedDbBackend.storage.get(name);
            }
        }
        globalThis.IndexedDbBackend = IndexedDbBackend;
    "#;
    js_sys::eval(setup_code).expect("failed to evaluate mock JS");

    // 1. Create index with metadata
    let mut index = create_test_index();
    let id = insert_test_vector(&mut index);

    // Set various metadata types
    let str_val = JsMetadataValue::from_string("Test Document".to_string());
    let int_val = JsMetadataValue::from_integer(42.0).unwrap();
    let float_val = JsMetadataValue::from_float(3.125);
    let bool_val = JsMetadataValue::from_boolean(true);

    index.set_metadata(id, "title", &str_val).unwrap();
    index.set_metadata(id, "count", &int_val).unwrap();
    index.set_metadata(id, "score", &float_val).unwrap();
    index.set_metadata(id, "active", &bool_val).unwrap();

    // 2. Save to mock IndexedDB
    index
        .save("test_metadata_db".to_string())
        .await
        .expect("save should succeed");

    // 3. Load from mock IndexedDB
    let loaded = EdgeVec::load("test_metadata_db".to_string())
        .await
        .expect("load should succeed");

    // 4. Verify metadata survived
    assert_eq!(
        loaded.metadata_key_count(id),
        4,
        "Should have 4 metadata keys"
    );

    let title = loaded
        .get_metadata(id, "title")
        .expect("title should exist");
    assert!(title.is_string());
    assert_eq!(title.as_string(), Some("Test Document".to_string()));

    let count = loaded
        .get_metadata(id, "count")
        .expect("count should exist");
    assert!(count.is_integer());
    assert_eq!(count.as_integer(), Some(42.0));

    let score = loaded
        .get_metadata(id, "score")
        .expect("score should exist");
    assert!(score.is_float());
    // Use approximate comparison for floats
    let score_val = score.as_float().unwrap();
    assert!((score_val - 3.125).abs() < 0.00001);

    let active = loaded
        .get_metadata(id, "active")
        .expect("active should exist");
    assert!(active.is_boolean());
    assert_eq!(active.as_boolean(), Some(true));
}
