#![cfg(target_arch = "wasm32")]

use edgevec::wasm::{EdgeVec, EdgeVecConfig};
use js_sys::{Array, Date, Float32Array, Function, Uint32Array};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

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
    assert_eq!(ids.length(), count);

    // Performance budget assertion (soft, mostly for reporting)
    // < 1ms target
    // Note: Date::now() has low resolution (ms), so duration might be 0 or small multiple of 1ms.
    // For 100 vectors, if it takes < 1ms total, it's 0.
    // So per_vector will be 0.
    // If it takes 20ms, per_vector is 0.2ms.
    // This is good enough for verification.
}

// =============================================================================
// W14.1: Progress Callback Tests
// =============================================================================

/// Test: Progress callback is invoked with correct start/end values
#[wasm_bindgen_test]
fn test_progress_callback_invoked() {
    let dims = 32;
    let count = 10u32;

    // Setup index
    let config = EdgeVecConfig::new(dims);
    let mut edgevec = EdgeVec::new(&config).expect("failed to create edgevec");

    // Create vectors array
    let vectors = Array::new();
    for _ in 0..count {
        let vec = Float32Array::new_with_length(dims);
        for j in 0..dims {
            vec.set_index(j, (j as f32) * 0.01);
        }
        vectors.push(&vec);
    }

    // Track callback invocations using a closure that stores results
    let call_log: Rc<RefCell<Vec<(u32, u32)>>> = Rc::new(RefCell::new(Vec::new()));
    let call_log_clone = call_log.clone();

    // Create JS callback function
    let callback = Closure::wrap(Box::new(move |done: u32, total: u32| {
        call_log_clone.borrow_mut().push((done, total));
    }) as Box<dyn FnMut(u32, u32)>);

    let js_func: &Function = callback.as_ref().unchecked_ref();

    // Execute batch insert with progress
    let result = edgevec
        .insert_batch_with_progress(vectors, js_func.clone())
        .expect("insert_batch_with_progress failed");

    // Verify callback was invoked
    let log = call_log.borrow();
    assert!(
        log.len() >= 2,
        "Callback should be invoked at least twice (start and end)"
    );

    // Verify start progress (0, total)
    assert_eq!(log[0], (0, count), "First callback should report 0/{count}");

    // Verify end progress (total, total)
    let last = log.last().unwrap();
    assert_eq!(
        last,
        &(count, count),
        "Last callback should report {count}/{count}"
    );

    // Verify result
    assert_eq!(
        result.inserted(),
        count as usize,
        "Should insert {count} vectors"
    );

    // Prevent callback from being dropped
    callback.forget();
}

/// Test: Progress callback with empty vectors array
#[wasm_bindgen_test]
fn test_progress_callback_empty_vectors() {
    let dims = 32;

    // Setup index
    let config = EdgeVecConfig::new(dims);
    let mut edgevec = EdgeVec::new(&config).expect("failed to create edgevec");

    // Empty vectors array
    let vectors = Array::new();

    // Track callback invocations
    let call_count: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
    let call_count_clone = call_count.clone();

    let callback = Closure::wrap(Box::new(move |_done: u32, _total: u32| {
        *call_count_clone.borrow_mut() += 1;
    }) as Box<dyn FnMut(u32, u32)>);

    let js_func: &Function = callback.as_ref().unchecked_ref();

    // Execute - should handle empty gracefully
    let result = edgevec.insert_batch_with_progress(vectors, js_func.clone());

    // Empty batch should return an error (no vectors to insert)
    assert!(
        result.is_err(),
        "Empty batch should return error, not silently succeed"
    );

    callback.forget();
}

/// Test: Progress callback returns correct IDs
#[wasm_bindgen_test]
fn test_progress_callback_returns_ids() {
    let dims = 16;
    let count = 5;

    let config = EdgeVecConfig::new(dims);
    let mut edgevec = EdgeVec::new(&config).expect("failed to create edgevec");

    // Create vectors
    let vectors = Array::new();
    for i in 0..count {
        let vec = Float32Array::new_with_length(dims);
        for j in 0..dims {
            vec.set_index(j, (i * dims + j) as f32 * 0.1);
        }
        vectors.push(&vec);
    }

    // No-op callback
    let callback = Closure::wrap(Box::new(|_: u32, _: u32| {}) as Box<dyn FnMut(u32, u32)>);
    let js_func: &Function = callback.as_ref().unchecked_ref();

    let result = edgevec
        .insert_batch_with_progress(vectors, js_func.clone())
        .expect("insert failed");

    // Verify IDs are returned
    let ids = result.ids();
    assert_eq!(ids.len(), count as usize, "Should return {count} IDs");

    // IDs should be sequential starting from 0
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(*id, i as u64, "ID at index {i} should be {i}");
    }

    callback.forget();
}

/// Test: Progress callback with large batch
#[wasm_bindgen_test]
fn test_progress_callback_large_batch() {
    let dims = 64;
    let count = 500u32;

    let config = EdgeVecConfig::new(dims);
    let mut edgevec = EdgeVec::new(&config).expect("failed to create edgevec");

    // Create many vectors
    let vectors = Array::new();
    for i in 0..count {
        let vec = Float32Array::new_with_length(dims);
        for j in 0..dims {
            vec.set_index(j, ((i * dims + j) % 1000) as f32 * 0.001);
        }
        vectors.push(&vec);
    }

    // Track progress values
    let progress_vals: Rc<RefCell<Vec<(u32, u32)>>> = Rc::new(RefCell::new(Vec::new()));
    let progress_clone = progress_vals.clone();

    let callback = Closure::wrap(Box::new(move |done: u32, total: u32| {
        progress_clone.borrow_mut().push((done, total));
    }) as Box<dyn FnMut(u32, u32)>);

    let js_func: &Function = callback.as_ref().unchecked_ref();

    let start = Date::now();
    let result = edgevec
        .insert_batch_with_progress(vectors, js_func.clone())
        .expect("large batch failed");
    let elapsed = Date::now() - start;

    assert_eq!(
        result.inserted(),
        count as usize,
        "Should insert all {count} vectors"
    );

    let progress = progress_vals.borrow();
    assert!(
        progress.len() >= 2,
        "Should have at least start and end progress"
    );

    // All progress reports should have correct total
    for (_, total) in progress.iter() {
        assert_eq!(*total, count, "Total should always be {count}");
    }

    println!(
        "Large batch ({count} vectors): {elapsed:.2}ms, {:.0} vec/sec",
        count as f64 / elapsed * 1000.0
    );

    callback.forget();
}
