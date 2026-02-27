#![no_main]
//! Fuzz target for FlatIndex::from_snapshot().
//!
//! Feeds random bytes to the snapshot deserialization path.
//! Invariant: MUST return Ok or Err, NEVER panic.
//!
//! The snapshot format includes a header (magic, version, checksum),
//! deleted bitmap, vector data, and optional quantized data.
//! Random bytes should exercise all validation and bounds-checking paths.

use edgevec::index::FlatIndex;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // from_snapshot must never panic, only return Ok or Err
    let result = FlatIndex::from_snapshot(data);

    // If parsing succeeded, exercise the loaded index
    if let Ok(index) = result {
        let _ = index.len();
        let _ = index.is_empty();
        let _ = index.dimensions();
        let _ = index.metric();
        let _ = index.capacity();
        let _ = index.deleted_count();
        let _ = index.deletion_ratio();

        // Try to get vectors at a few IDs
        for id in 0..index.capacity().min(5) as u64 {
            let _ = index.get(id);
            let _ = index.contains(id);
        }

        // If non-empty, try a search
        if !index.is_empty() && index.dimensions() > 0 {
            let query = vec![0.0f32; index.dimensions() as usize];
            let _ = index.search(&query, 3);
        }

        // Roundtrip: serialize back and verify
        if let Ok(bytes) = index.to_snapshot() {
            let roundtrip = FlatIndex::from_snapshot(&bytes);
            assert!(
                roundtrip.is_ok(),
                "roundtrip must succeed: to_snapshot -> from_snapshot"
            );
        }
    }
});
