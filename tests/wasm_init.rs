#![cfg(target_arch = "wasm32")]
//! WASM Initialization Tests (W4.1)

use edgevec::wasm::init_logging;
use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_wasm_init_logging() {
    // Should not panic or fail
    init_logging();
}

#[wasm_bindgen_test]
fn test_wasm_init_success() {
    init_logging();

    // Create valid config
    let mut config = EdgeVecConfig::new(128);
    config.set_m(16);
    config.set_ef_construction(100);
    config.set_ef_search(50);
    config.set_metric("l2".to_string());

    // Attempt creation
    let index = EdgeVec::new(&config);

    // Assert success
    assert!(index.is_ok(), "EdgeVec should initialize with valid config");
}

#[wasm_bindgen_test]
fn test_wasm_init_error_propagation() {
    init_logging();

    // Create INVALID config (m=0 is not allowed, must be > 1)
    let mut config = EdgeVecConfig::new(128);
    config.set_m(0); // Invalid

    // Attempt creation
    let result = EdgeVec::new(&config);

    // Assert failure
    assert!(
        result.is_err(),
        "EdgeVec should fail with invalid config (m=0)"
    );

    let err = result.err().unwrap();

    if let Some(error_msg) = err.as_string() {
        assert!(
            error_msg.contains("m must be > 1"),
            "Error message should contain 'm must be > 1', got: {}",
            error_msg
        );
    }
}
