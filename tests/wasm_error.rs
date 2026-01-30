#![cfg(target_arch = "wasm32")]

use edgevec::error::EdgeVecError;
use edgevec::hnsw::GraphError;
use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use js_sys::{Float32Array, Reflect, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Helper to setup corrupted backend to trigger deserialization error
fn setup_corrupted_backend() {
    let setup_code = r#"
        if (typeof globalThis.IndexedDbBackend === 'undefined' || globalThis.IndexedDbBackend.isMock) {
            class IndexedDbBackend {
                static isMock = true;
                static async write(name, data) {}
                static async read(name) {
                    // Return garbage bytes (not a valid postcard archive)
                    return new Uint8Array([0, 1, 2, 3, 255, 255]);
                }
            }
            globalThis.IndexedDbBackend = IndexedDbBackend;
        }
    "#;
    js_sys::eval(setup_code).expect("failed to evaluate mock JS");
}

#[wasm_bindgen_test]
fn verify_error_mapping_structure() {
    // 1. Verify IO Error mapping
    let io_err = EdgeVecError::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Simulated failure",
    ));
    let js_val: JsValue = io_err.into();

    let code = Reflect::get(&js_val, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_IO");

    let msg = Reflect::get(&js_val, &"message".into()).unwrap();
    assert_eq!(msg.as_string().unwrap(), "IO error: Simulated failure");

    // 2. Verify Graph Error mapping (Dimension)
    let graph_err = EdgeVecError::Graph(GraphError::DimensionMismatch {
        expected: 128,
        actual: 64,
    });
    let js_val: JsValue = graph_err.into();

    let code = Reflect::get(&js_val, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_DIMENSION");
}

#[wasm_bindgen_test]
async fn test_insert_dimension_mismatch() {
    let config = EdgeVecConfig::new(128);
    let mut db = EdgeVec::new(&config).unwrap();

    // Create vector with wrong dimensions (64 instead of 128)
    let wrong_dim_vec = Float32Array::new_with_length(64);

    let result = db.insert(wrong_dim_vec);

    assert!(result.is_err(), "Should fail with dimension mismatch");
    let err = result.unwrap_err();

    // Verify it's an object with code=ERR_DIMENSION
    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_DIMENSION");

    let msg = Reflect::get(&err, &"message".into()).unwrap();
    assert!(msg.as_string().unwrap().contains("Dimension mismatch"));
}

// Note: test_batch_dimension_mismatch removed - API changed to insert_batch_v2 with BatchInsertConfig

#[wasm_bindgen_test]
async fn test_persistence_error() {
    setup_corrupted_backend();

    // Attempt to load from corrupted backend
    let result = EdgeVec::load("corrupted_db".to_string()).await;

    assert!(result.is_err(), "Should fail to load corrupted data");
    let err = result.err().expect("already checked is_err");

    // The load function wraps postcard errors in EdgeVecError::Persistence(PersistenceError::Corrupted)
    // which maps to ERR_CORRUPTION

    let code = Reflect::get(&err, &"code".into()).expect("Error should have code property");
    let code_str = code.as_string().expect("code should be a string");

    assert_eq!(code_str, "ERR_CORRUPTION");

    let msg = Reflect::get(&err, &"message".into()).expect("Error should have message property");
    assert!(msg.as_string().unwrap().contains("Deserialization failed"));
}

// =============================================================================
// Binary + Metric Validation Tests
// =============================================================================

/// Test that VectorType::Binary with non-Hamming metric is rejected at construction.
#[wasm_bindgen_test]
fn test_binary_with_l2_metric_rejected() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(768);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::L2); // Incompatible!

    let result = EdgeVec::new(&config);

    assert!(result.is_err(), "Binary + L2 should be rejected");
    let err = result.err().expect("already checked is_err");

    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_VALIDATION");

    let msg = Reflect::get(&err, &"message".into()).unwrap();
    assert!(
        msg.as_string()
            .unwrap()
            .contains("requires metric='hamming'"),
        "Error should mention hamming requirement"
    );
}

/// Test that VectorType::Binary with Cosine metric is rejected.
#[wasm_bindgen_test]
fn test_binary_with_cosine_metric_rejected() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(768);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::Cosine);

    let result = EdgeVec::new(&config);

    assert!(result.is_err(), "Binary + Cosine should be rejected");
    let err = result.err().expect("already checked is_err");

    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_VALIDATION");
}

/// Test that VectorType::Binary with Dot metric is rejected.
#[wasm_bindgen_test]
fn test_binary_with_dot_metric_rejected() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(768);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::Dot);

    let result = EdgeVec::new(&config);

    assert!(result.is_err(), "Binary + Dot should be rejected");
    let err = result.err().expect("already checked is_err");

    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_VALIDATION");
}

/// Test that VectorType::Binary with Hamming metric succeeds.
#[wasm_bindgen_test]
fn test_binary_with_hamming_metric_succeeds() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(768);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::Hamming);

    let result = EdgeVec::new(&config);

    assert!(result.is_ok(), "Binary + Hamming should succeed");
}

// =============================================================================
// insertBinary / searchBinary Metric Guard Tests
// =============================================================================

/// Test that insertBinary on HNSW with non-Hamming metric is rejected.
#[wasm_bindgen_test]
fn test_insert_binary_non_hamming_rejected() {
    // Create HNSW index with L2 metric (default)
    let config = EdgeVecConfig::new(64);
    let mut db = EdgeVec::new(&config).unwrap();

    // Try to insert binary vector - should fail
    let binary_vec = Uint8Array::new_with_length(8); // 64 bits = 8 bytes

    let result = db.insert_binary(binary_vec);

    assert!(
        result.is_err(),
        "insertBinary should fail on non-Hamming index"
    );
    let err = result.unwrap_err();

    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_VALIDATION");

    let msg = Reflect::get(&err, &"message".into()).unwrap();
    assert!(
        msg.as_string()
            .unwrap()
            .contains("requires metric='hamming'"),
        "Error should mention hamming requirement"
    );
}

/// Test that searchBinary on HNSW with non-Hamming metric is rejected.
#[wasm_bindgen_test]
fn test_search_binary_non_hamming_rejected() {
    // Create HNSW index with Cosine metric
    let mut config = EdgeVecConfig::new(64);
    config.set_metric_type(edgevec::wasm::MetricType::Cosine);
    let mut db = EdgeVec::new(&config).unwrap();

    // Insert a regular f32 vector first
    let f32_vec = Float32Array::new_with_length(64);
    db.insert(f32_vec).unwrap();

    // Try to search with binary query - should fail
    let binary_query = Uint8Array::new_with_length(8);

    let result = db.search_binary(binary_query, 5);

    assert!(
        result.is_err(),
        "searchBinary should fail on non-Hamming index"
    );
    let err = result.err().expect("already checked is_err");

    let code = Reflect::get(&err, &"code".into()).unwrap();
    assert_eq!(code.as_string().unwrap(), "ERR_VALIDATION");
}

/// Test that insertBinary with Hamming metric succeeds.
#[wasm_bindgen_test]
fn test_insert_binary_hamming_succeeds() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(64);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::Hamming);

    let mut db = EdgeVec::new(&config).unwrap();

    let binary_vec = Uint8Array::new_with_length(8);
    let result = db.insert_binary(binary_vec);

    assert!(
        result.is_ok(),
        "insertBinary should succeed on Hamming index"
    );
}

/// Test that searchBinary with Hamming metric succeeds.
#[wasm_bindgen_test]
fn test_search_binary_hamming_succeeds() {
    use edgevec::wasm::{MetricType, VectorType};

    let mut config = EdgeVecConfig::new(64);
    config.set_vector_type(VectorType::Binary);
    config.set_metric_type(MetricType::Hamming);

    let mut db = EdgeVec::new(&config).unwrap();

    // Insert some binary vectors
    for i in 0..5 {
        let binary_vec = Uint8Array::new_with_length(8);
        binary_vec.fill(i, 0, 8);
        db.insert_binary(binary_vec).unwrap();
    }

    // Search should succeed
    let query = Uint8Array::new_with_length(8);
    let result = db.search_binary(query, 3);

    assert!(
        result.is_ok(),
        "searchBinary should succeed on Hamming index"
    );
}
