#![no_main]
use edgevec::wasm::EdgeVec;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize the full EdgeVec structure
    // This stresses the deserialization logic of HnswIndex and VectorStorage
    // combined via the EdgeVec wrapper.
    //
    // Invariant: MUST return Result (Ok or Err), NEVER panic.
    if let Ok(db) = postcard::from_bytes::<EdgeVec>(data) {
        // If loaded successfully, we just drop it.
        // We can't easily call methods on it because they require JS types (Float32Array)
        // or JS environment (IndexedDB).
        let _ = db;
    }
});
