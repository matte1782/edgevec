#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Decode arbitrary bytes as a WASM `toBytes()` / `save()` buffer (current
    // EVWB format and the 0.6–0.9 legacy layouts). Accepted buffers are then
    // driven through search, insert, BQ and soft delete.
    //
    // Invariant: MUST return (Ok or Err), NEVER panic. Note that the CRC only
    // gates the v1 format; seed the corpus with real buffers (see
    // tests/fixtures/wasm_buffers/) to reach the structural validator.
    edgevec::persistence::fuzz_wasm_buffer(data);
});
