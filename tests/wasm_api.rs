#![cfg(target_arch = "wasm32")]
//! WASM API Tests (W4.2)
//!
//! Tests the high-level JS API for insert and search.
//! Run with: wasm-pack test --headless --chrome

use edgevec::wasm::init_logging;
use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use js_sys::{Float32Array, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_insert_search_flow() {
    init_logging();

    // 1. Init (Dimensions = 3)
    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();

    // 2. Insert Vector A [1.0, 0.0, 0.0]
    let vec_a_data = vec![1.0f32, 0.0, 0.0];
    // Note: Float32Array::from(slice) copies memory into JS heap
    let vec_a_js = Float32Array::from(vec_a_data.as_slice());
    let id_a = index.insert(vec_a_js).expect("Insert A failed");

    // 3. Insert Vector B [0.0, 1.0, 0.0]
    let vec_b_data = vec![0.0f32, 1.0, 0.0];
    let vec_b_js = Float32Array::from(vec_b_data.as_slice());
    let id_b = index.insert(vec_b_js).expect("Insert B failed");

    assert_ne!(id_a, id_b, "IDs should be unique");

    // 4. Search for A [1.0, 0.0, 0.0]
    let query_data = vec![1.0f32, 0.0, 0.0];
    let query_js = Float32Array::from(query_data.as_slice());
    let results_js = index.search(query_js, 1).expect("Search failed");

    // 5. Verify Results
    let results_array: js_sys::Array = results_js.into();
    assert_eq!(results_array.length(), 1, "Should find 1 neighbor");

    let result_obj = results_array.get(0);

    // Check ID
    let id_prop = JsValue::from_str("id");
    let id_val = Reflect::get(&result_obj, &id_prop).unwrap();
    // JS numbers are f64, but we returned u32. wasm-bindgen handles conversions.
    let id_found = id_val.as_f64().expect("id should be a number") as u32;
    assert_eq!(id_found, id_a, "Should find Vector A");

    // Check Score
    let score_prop = JsValue::from_str("score");
    let score_val = Reflect::get(&result_obj, &score_prop).unwrap();
    let score_found = score_val.as_f64().expect("score should be a number") as f32;

    // L2 squared distance of vector to itself is 0.0
    assert!(
        score_found.abs() < f32::EPSILON,
        "Score should be 0.0 for exact match"
    );
}

#[wasm_bindgen_test]
fn test_insert_wrong_dimension() {
    init_logging();

    let config = EdgeVecConfig::new(3);
    let mut index = EdgeVec::new(&config).unwrap();

    // 2 dimensions instead of 3
    let vec_wrong = vec![1.0f32, 0.0];
    let vec_js = Float32Array::from(vec_wrong.as_slice());

    let result = index.insert(vec_js);
    assert!(result.is_err(), "Should fail with wrong dimensions");

    let err = result.unwrap_err();
    let err_str = err.as_string().unwrap_or_default();
    assert!(
        err_str.contains("Dimension mismatch"),
        "Error should mention dimensions"
    );
}
