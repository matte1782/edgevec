#![no_main]
use edgevec::persistence::{MemoryBackend, StorageBackend};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to load a snapshot from arbitrary bytes.
    // This stresses the full persistence deserialization pipeline:
    // header parsing, vector data extraction, HNSW node reconstruction.
    //
    // Invariant: MUST return Result (Ok or Err), NEVER panic.
    let backend = MemoryBackend::new();
    let _ = backend.atomic_write("", data);
    let _ = edgevec::persistence::snapshot::read_snapshot(&backend);
});
