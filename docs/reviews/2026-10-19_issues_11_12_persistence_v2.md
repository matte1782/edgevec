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

[Raw benchmark evidence](../benchmarks/2026-10-19_persistence_load.json) records the latest rebuilt SIMD runtime, commands, hardware, all six new idle trials, and the archived previously reviewed build/trials. The table below refers to WASM SHA-256 `2ac49da9589f315bc9df6c2a6d064883a38c48c9b67bc491c1f3481c6cdddfb3`. The protocol uses three fresh Node processes per mode after compiler/test jobs finish: 100,000 × 128-dimensional F32 vectors, L2, default HNSW parameters, deterministic seed 42, and 57,265,029-byte buffers. First-call timing includes JS-to-WASM copying, checksum, decoding, structural validation and BQ rebuilding; it excludes module initialization and file I/O. Hardware: Intel Core Ultra 9 285H, Windows x64, Node 22.21.0.

| Mode | Fresh first calls (ms) | First-call mean ± sample SD (ms) | Largest warm sample (ms) |
| --- | --- | --- | --- |
| Plain HNSW | 296.92, 272.48, 297.37 | 288.92 ± 14.24 | 182.29 |
| HNSW with BQ rebuild | 358.10, 241.00, 288.52 | 295.87 ± 58.90 | 186.22 |

All six controlled first calls satisfy the architecture's **<500 ms cold index-load** target for this dataset. Earlier pre-optimization and contended trials exceeded it and remain in the raw data; warm medians were not substituted for first-call evidence. These small Node samples establish this measured case, not browser/Safari performance, other dimensions, or P99 latency.

The latest rebuilt raw WASM is **683,701 bytes** (262,884 gzip); JS is **161,716 bytes** (29,205 gzip). Combined raw size is **845,417 bytes**, combined gzip **292,089 bytes**. The WASM artifact is below 700,000 bytes and the combined compressed runtime is below the architecture's 500 KB compressed target. The combined uncompressed runtime exceeds 700,000 bytes; these size measures are distinct. Both copied package and demo runtimes pass their five glue tests again.

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

## Appendix — Hosted CI follow-ups reviewed 2026-10-20

**Disposition: GO for the next CI iteration; merge remains gated on green hosted checks.** This appendix reviews the follow-up patches, not an assertion that all hosted jobs have passed. The original persistence approval remains scoped to the implementation and evidence above.

| Follow-up | Review assessment |
| --- | --- |
| Current Clippy byte-array diagnostics | `*b"EVFI"`, `*b"XXXX"` and `*b"ESPV"` have the same `[u8; 4]` values as the replaced character arrays (`src/index/flat.rs:1180`, `:2864`; `src/sparse/storage.rs:53`). No snapshot bytes, layout or behavior change. |
| BrowserStack dependency setup | `tests/browserstack/package-lock.json` now supplies the lock required by `npm ci` and the explicit cache path; Node 22 supports the locked Selenium dependency. The lock must be included in the published changes. |
| BrowserStack runtime assets | The generated JS/WASM and their imported snippets are copied together. This resolves the missing-snippet asset path rather than omitting that import. The workflow's scalar WASM build is limited to its legacy Safari/mobile matrix; regular WASM CI retains its SIMD build. |
| Mobile API assertions | Cases use the ID actually returned by insertion rather than the invalid zero-ID assumption (`tests/mobile/index.html:1077`). The reviewed smoke log records **12/12 actual mobile-page API cases passing under Node**. This verifies their API logic; mobile-browser execution still needs the hosted matrix. |
| ARM infrastructure | Native `ubuntu-24.04-arm` replaces cross/Docker setup while preserving the release build, complete library test command, SIMD-filtered test command, ARM lint and x86 regression job. An explicit `uname -m` check verifies execution architecture. No test assertions are removed. |

GitHub lists `ubuntu-24.04-arm` as an ARM64 hosted-runner label. The workflow therefore uses a documented execution environment rather than substituting an x86 compile-only check. Its runtime and lint results must still be verified. [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#standard-github-hosted-runners-for-public-repositories).

### bitvec constraint and Miri

`Cargo.toml:84` changes the dependency requirement from `"1.0"` to `"~1.0"`, retaining its serde feature. This is a reasonable constraint for the project's fixed MSRV: upstream explicitly recommends a limiting minor-version requirement for pinned Rust toolchains and permits MSRV increases in minor releases. [bitvec usage policy](https://github.com/ferrilab/bitvec#usage).

The distinction matters: `~1.0` permits `>=1.0.0, <1.1.0`; it is **not an exact 1.0.1 pin**. The local lockfile inspected during this review resolves **1.0.1**, matching the version used for the earlier local validation. Future 1.0 patch releases remain permitted. [Cargo version-requirement semantics](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#tilde-requirements).

This dependency policy is approved for testing; it does **not yet establish a Miri fix**. The linked upstream retag issue was opened in 2022 and reports a similar failure, so it cannot establish that the problem first appeared in 1.1.1 or that 1.0.1 is unaffected. Native test success also does not establish Miri success. This is an inference from the issue's age and the difference between the checks, not an upstream claim about the current CI run. [Upstream issue 193](https://github.com/ferrilab/bitvec/issues/193).

Both hosted Miri jobs must pass with the newly resolved dependency version and their existing model flags and test coverage. Disabling checks, adding failure tolerances or removing further coverage is outside this approval. If either still fails, that remains an unresolved merge blocker requiring further investigation.

### Evidence and publication gate

The patches are ready for CI. Formatting passes, the mobile-page Node smoke passes 12/12, and the code/value equivalence and workflow command coverage were inspected. The implementing agent reported the earlier hosted Linux suite, WASM compilation/persistence, fuzz-build and x86 checks passing; the follow-up revision still needs its own hosted results.

Before merge, verify the new revision's native lint, ARM build/tests/lint, both Miri jobs and configured BrowserStack matrix, together with the already-required main checks. A missing-secrets skip must be reported as a skip, not as executed mobile-browser coverage. The reviewer changed only this review document and performed no implementation or Git-state mutation.

### Additional ARM and BrowserStack corrections

**GO for another CI iteration; the same hosted merge gate remains.** Revision `9815857` was reported green for lint, WASM and fuzz checks. At this review point, both Miri jobs with bitvec 1.0.1 are still running; elapsed time is not a passing result.

The ARM workflow now overrides the obsolete internal LLVM switches from local Cargo configuration with `RUSTFLAGS="-C target-cpu=generic -C opt-level=3"`. This uses stable compiler options and an ARM baseline while leaving its build, test, SIMD and lint commands intact. The x86 regression step retains its own x86-specific override. This addresses the observed compiler preflight failure without suppressing a lint or test.

The BrowserStack runner now places `browserName` at the W3C capability root, retains `deviceName`, `osVersion` and `realMobile` under `bstack:options`, and removes the nested browser name and desktop-style OS selection (`tests/browserstack/run-tests.js:50`). This organization matches the vendor's mobile examples. [BrowserStack device-selection documentation](https://www.browserstack.com/docs/automate/selenium/select-browsers-and-devices#browserstack-specific-capabilities-mobile).

The reviewer independently ran `node --check tests/browserstack/run-tests.js` successfully. The hosted setup and scalar build had passed before the earlier client-side capability failure; that does not establish a real-device session or a passing mobile suite. Both Miri outcomes, actual ARM execution and the configured mobile-browser matrix remain unproven until their hosted jobs finish successfully. The latest runtime measurements and rebuilt-artifact glue results are recorded in the updated performance section above; the previous measurements remain archived in the raw data.
