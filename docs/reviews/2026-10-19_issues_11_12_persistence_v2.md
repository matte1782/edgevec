# HOSTILE_REVIEWER: Issues #11 / #12 Persistence — v2

**Date:** 2026-10-19
**Artifact:** Revised Rust/WASM persistence implementation, regression tests, CI, runtime artifacts, and documentation
**Type:** Code + tests + format compatibility + documentation
**Status:** APPROVED — GO

## Scope and method

This is an independent ordinary software-quality review of the final working-tree changes. The review used `.claude/CLAUDE.md`, `.claude/commands/review.md`, `.claude/HOSTILE_GATE_CHECKLIST.md`, the architecture's persistence/performance requirements, and the [first review](2026-10-19_issues_11_12_persistence.md). It covers `save()` / `load()`, `toBytes()` / `fromBytes()`, legacy formats, IndexedDB lifecycle, CI, examples, and the related load-performance changes.

The reviewer inspected implementation and regression tests, independently ran the IndexedDB tests and concurrent repair scenario, and checked final validation evidence supplied by the implementing agent against shared logs and the committed benchmark data. This review does not claim an exhaustive parser proof or a hosted CI result.

## Findings

**Remaining critical: 0. Major: 0. Minor: 0 within the reviewed scope.**

Two additional functional problems found during this pass were corrected and verified:

1. **Concurrent callers could still hang behind a blocked repair.** Previously, simultaneous write/write/read calls could issue their upgrade requests before the first `blocked` callback set the cached error. The first rejected while the others remained pending. `src/js/storage.js:22` now queues connection setup per IndexedDB factory before issuing opens; transactions retain separate connections. `tests/js/storage.test.mjs:126` covers simultaneous rejection and subsequent recovery. The reviewer's original three-call reproduction now returns three prompt rejections and successfully writes/reads after the blocking connection closes. The complete shim suite passes **11/11**.
2. **Zero-dimensional indexes could be created and serialized but rejected when restored.** `src/wasm/mod.rs:707` now rejects zero dimensions at construction with `ERR_VALIDATION`; `tests/wasm_persistence.rs:60` covers both HNSW and Flat. Flat decoding also rejects zero dimensions consistently (`src/persistence/buffer.rs:731`). The final WASM suite passes **10/10**.

The new golden-fixture test initially called `Object.keys` on an absent metadata value. Its final branch accepts null/undefined only when the recorded expected metadata object is empty; nonempty metadata remains fully asserted (`tests/wasm_persistence.rs:333`). The final test passes.

## Resolution of the first review

| Previous finding | Final assessment and evidence |
| --- | --- |
| C1 / C2: restoration invariants and tombstone decoding | Addressed. `validate_hnsw` checks storage shape/type, finite vectors, IDs, neighbor-list bounds/content, free slots, allocation ownership and entry points, and derives counters (`src/persistence/buffer.rs:506`). Endpoint arithmetic uses `u64`. The validating tombstone decoder checks word width, head, available bits and the platform bit-vector limit before construction (`src/storage/bitvec_serde.rs:44`). Regression tests include the WASM offset boundary. |
| M1: unusable lockfile/version resolution in CI | Addressed. The job generates `Cargo.lock`, requires a nonempty version, installs the matching wasm-bindgen tools, and runs locked commands (`.github/workflows/ci.yml:129`). Its WASM-specific `RUSTFLAGS` override the global native CPU flag. |
| M2 / M3: legacy compatibility and schema regression protection | Addressed. Separate decoders cover the 0.6–0.8 storage layout and the 0.9 tagged layout, including relocation of older HNSW metadata (`src/persistence/buffer.rs:436`, `:465`). Frozen real 0.8/0.9 and v1 fixtures cover HNSW, deletes, metadata, BQ and Flat. WASM checks complete recorded result ordering and metadata. Fixture writer hashes and reproduction instructions are in `tests/fixtures/wasm_buffers/README.md`; serialized types document version/migration requirements. |
| M4: blocked-repair retries | Addressed, including the concurrent gap discovered in this pass. Repeated and queued operations reject promptly; the pending upgrade's eventual result is closed and recovery is tested. |
| M5: duplicate WASM payload allocation while encoding | Addressed. Encoding writes postcard directly after a header placeholder and patches the header/CRC in place (`src/persistence/buffer.rs:200`). `save()` passes an owned JS byte copy across its asynchronous boundary. |
| M6: vector and metadata insertion constraints | Addressed. Restored vectors are checked for finiteness; both metadata stores use the existing key/value and per-vector limits (`src/persistence/buffer.rs:746`). |
| M7: parser regression/fuzz coverage | Addressed. The `wasm_buffer` fuzz target builds, and bounded post-load exercise covers search, insertion, BQ and deletion. The exact final release mutation run passed **200,000 variants** (50,000 across each of four seed kinds) in **11.36 s**. This is mutation-test evidence, not a claim that a coverage-guided fuzz campaign ran. |
| M8: Node persistence documentation/example | Addressed. The example uses `toBytes()` / `fromBytes()` with Node file APIs and successfully restores its three vectors against a fresh build. The documentation distinguishes IndexedDB from application-managed files. |

Other earlier functional/documentation concerns are resolved: CRC covers the header and payload, unsupported flags/reserved fields map to `ERR_PERSISTENCE`, the repair's mixed-version limitation is documented, buffer allocation behavior is described, and the actual Rust-to-JS persistence glue is tested. Refreshed `pkg` and demo runtimes each pass their five glue tests; the copied storage snippet matches the source.

The existing Miri jobs exclude the new graph-driving fixture/mutation loops consistently with their HNSW exclusion; tombstone-decoder and metadata-serde tests remain included. The BrowserStack workflow's sole change replaces the invalid `dtolnay/rust-action@stable` reference with the repository's established `dtolnay/rust-toolchain@stable` (`.github/workflows/browserstack.yml:40`). A secrets-based skip must not be represented as mobile-browser execution.

## Performance and wire compatibility

The optimized v1 storage decoder preserves postcard field/variant order and checks the complete F32 span before allocation (`src/persistence/buffer.rs:372`, `:402`). A differential test compares it with the derived decoder across all seed kinds, in addition to frozen fixtures (`src/persistence/buffer/tests.rs:671`). Neighbor allocation ownership validation includes overlapping live/free ranges and duplicate free slots.

The shared binary quantizer constructs one output byte per eight components (`src/quantization/variable.rs:158`). Its existing dimension guard remains, and a regression checks all 256 bit patterns with signed zeros and non-finite comparison values (`:361`). Bit order and comparison semantics are unchanged.

[Raw benchmark evidence](../benchmarks/2026-10-19_persistence_load.json) records the final build hashes, commands, hardware, all six idle trials, and the earlier diagnostic trials. The final protocol uses three fresh Node processes per mode after compiler/test jobs finish: 100,000 × 128-dimensional F32 vectors, L2, default HNSW parameters, deterministic seed 42, and 57,265,029-byte buffers. First-call timing includes JS-to-WASM copying, checksum, decoding, structural validation and BQ rebuilding; it excludes module initialization and file I/O. Hardware: Intel Core Ultra 9 285H, Windows x64, Node 22.21.0.

| Mode | Fresh first calls (ms) | First-call mean ± sample SD (ms) | Largest warm sample (ms) |
| --- | --- | --- | --- |
| Plain HNSW | 350.57, 337.32, 290.66 | 326.19 ± 31.47 | 232.83 |
| HNSW with BQ rebuild | 374.68, 322.36, 280.82 | 325.95 ± 47.03 | 246.95 |

All six controlled first calls satisfy the architecture's **<500 ms cold index-load** target for this dataset. Earlier pre-optimization and contended trials exceeded it and remain in the raw data; warm medians were not substituted for first-call evidence. These small Node samples establish this measured case, not browser/Safari performance, other dimensions, or P99 latency.

The final raw WASM is **683,701 bytes** (262,881 gzip); JS is **161,716 bytes** (29,205 gzip). Combined raw size is **845,417 bytes**, combined gzip **292,086 bytes**. The WASM artifact is below 700,000 bytes and the combined compressed runtime is below the architecture's 500 KB compressed target. The combined uncompressed runtime exceeds 700,000 bytes; these size measures are distinct.

## Final validation

| Check | Result |
| --- | --- |
| Exact final native suite, including doctests | **3,483 passed**, 0 failed, 32 ignored across 92 suites: 1,091 library + 2,208 integration + 184 documentation |
| Focused buffer codec | **32/32 passed** |
| WASM persistence under Node | **10/10 passed** |
| IndexedDB shim, independently rerun | **11/11 passed** |
| Actual WASM/IndexedDB glue | **5/5 passed**; copied package and demo runtimes also **5/5 each** |
| Final release mutation exercise | **200,000 variants passed** |
| Native and WASM strict Clippy; formatting | **Passed** |
| New fuzz-target compilation | **Passed** |
| Node file persistence example | **Passed**, three vectors restored |
| Final release old-versus-new reproduction | **27 assertions passed** |

The [validation record](2026-10-19_issues_11_12_validation.md) lists reproduction commands and environment details. The native run uses `PROPTEST_CASES=32` and a writable temporary directory. Initial restricted Windows temporary-directory failures were reproduced as environment failures; the final complete run passes without changes to the existing file-corruption/atomic-save tests. Hosted workflows remain a separate publication check.

## Accepted exclusions and verdict

The pre-existing TypeScript-wrapper type errors and the pre-existing binary-index compaction defect remain outside this change. The two historical review edits predate this work and are preserved; their removal is not a requirement of this review. No implementation or Git-state changes were made by the reviewer.

**GO for the reviewed persistence change.** The original blocking functional findings and the additional concurrent-repair finding are resolved, regression coverage exercises the public behavior and real historical bytes, and the final measured load case meets its budget. This approval does not certify hosted CI or unmeasured browser performance. No gate file was created; the requested review document is the deliverable.

**Handoff:** HOSTILE_REVIEWER complete. Status: READY_FOR_NEXT_PHASE. Next: verify the publication's hosted CI results.
