#![no_main]
//! Fuzz target for SparseStorage::from_bytes().
//!
//! Feeds random bytes to the deserialization path.
//! Invariant: MUST return Ok or Err, NEVER panic.
//!
//! The v2 format includes magic bytes, version, CRC32 checksum, and vector data.
//! Random bytes should exercise all validation paths.

use edgevec::sparse::{SparseId, SparseStorage};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // from_bytes must never panic, only return Ok or Err
    let result = SparseStorage::from_bytes(data);

    // If parsing succeeded, exercise the loaded storage
    if let Ok(storage) = result {
        let _ = storage.len();

        // Try to get vectors at various IDs
        for id in 0..storage.len().min(10) as u64 {
            let _ = storage.get(SparseId::from(id));
        }

        // Roundtrip: serialize back and verify
        let bytes = storage.to_bytes();
        let roundtrip = SparseStorage::from_bytes(&bytes);
        assert!(
            roundtrip.is_ok(),
            "roundtrip must succeed: to_bytes -> from_bytes"
        );
    }
});
