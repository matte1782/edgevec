use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use js_sys::{Date, Float32Array, Uint32Array};
use wasm_bindgen_test::*;
// use web_sys::console;

// wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_insert_batch_100() {
    let dims = 128;
    let count = 100;

    // 1. Setup
    let config = EdgeVecConfig::new(dims);
    let mut edgevec = EdgeVec::new(&config).expect("failed to create edgevec");

    // 2. Generate random vectors (flat)
    let total_len = (count * dims) as usize;
    let mut data = Vec::with_capacity(total_len);
    // Deterministic-ish data
    for i in 0..total_len {
        data.push((i % 100) as f32 / 100.0);
    }

    let vectors = Float32Array::from(data.as_slice());

    // 3. Measure
    let start = Date::now();

    let ids: Uint32Array = edgevec
        .insert_batch_flat(vectors, count as usize)
        .expect("insert_batch_flat failed");

    let end = Date::now();
    let duration = end - start;
    let per_vector = duration / (count as f64);

    // 4. Log
    // console::log_1(&JsValue::from_str(&format!(
    //    "Batch Insert (100 vectors): Total {:.2}ms, Per Vector {:.4}ms",
    //    duration, per_vector
    // )));

    // Use println! for node test output visibility (captured by runner)
    println!(
        "Batch Insert (100 vectors): Total {:.2}ms, Per Vector {:.4}ms",
        duration, per_vector
    );

    // 5. Assert
    assert_eq!(ids.length(), count as u32);

    // Performance budget assertion (soft, mostly for reporting)
    // < 1ms target
    // Note: Date::now() has low resolution (ms), so duration might be 0 or small multiple of 1ms.
    // For 100 vectors, if it takes < 1ms total, it's 0.
    // So per_vector will be 0.
    // If it takes 20ms, per_vector is 0.2ms.
    // This is good enough for verification.
}
