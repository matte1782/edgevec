#![no_main]
use edgevec::persistence::header::FileHeader;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // FUZZ-004: FileHeader::from_bytes(random_bytes) must return Result, never panic.
    // We don't care about the result, just that it doesn't crash.
    let _ = FileHeader::from_bytes(data);
});
