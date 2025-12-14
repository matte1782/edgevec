#![no_main]
use edgevec::persistence::wal::WalIterator;
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let reader = Cursor::new(data);
    let iter = WalIterator::new(reader);

    for result in iter {
        // We just want to ensure this iteration doesn't panic.
        // Errors are expected for random data (ChecksumMismatch, Truncated, etc.)
        match result {
            Ok(_) => {}
            Err(_) => {}
        }
    }
});
