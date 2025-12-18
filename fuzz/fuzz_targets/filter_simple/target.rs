#![no_main]
//! FUZZ-011: Filter parser fuzzing for simple expressions.
//!
//! Tests that the filter parser handles arbitrary string input without panicking.
//! The parser should return Result for all inputs, never crash.

use edgevec::filter::parse;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // FUZZ-011: parse(arbitrary_string) must return Result, never panic.
    // Convert bytes to string (lossy to handle invalid UTF-8)
    if let Ok(input) = std::str::from_utf8(data) {
        // Limit input size to prevent OOM
        if input.len() <= 10_000 {
            let _ = parse(input);
        }
    }
});
