# Issues #11/#12: final persistence validation

Date: 2026-10-19. Scope: WASM buffer persistence, IndexedDB lifecycle, compatibility, generated raw runtime and demo bundles.

## Final local checks

| Check | Result |
| --- | --- |
| `PROPTEST_CASES=32 cargo test` | 3,483 passed, 0 failed, 32 ignored across 92 suites: 1,091 library, 2,208 integration and 184 documentation tests |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` | Passed |
| `cargo check --manifest-path fuzz/Cargo.toml --bin wasm_buffer` | Passed |
| Buffer codec regression suite | 32 passed (included in full native suite) |
| Variable quantizer regression suite | 22 passed, including every packed sign pattern and signed zero/non-finite comparison semantics |
| `cargo test --target wasm32-unknown-unknown --test wasm_persistence` with wasm-bindgen runner | 10 passed |
| `npm run test:storage` | 11 passed |
| Actual release WASM bindings + IndexedDB (`persistence.e2e.mjs`) | 5 passed |
| Release malformed-buffer stress (`EDGEVEC_MUTATION_ITERS=50000`) | 200,000 mutations across four seeds; no panic; 11.36 seconds |
| Old-versus-new runtime reproduction | 27 assertions passed; old runtime reproduced both reported failures |

The end-to-end tests exercise the generated bindings, not only the standalone JavaScript storage shim. Golden fixtures come from real 0.8.0 and 0.9.0 builds and the EVWB v1 format. They compare complete result ordering, dimensions, metadata, soft deletes, BQ state and next-ID behavior. The wasm32 suite includes offset-overflow coverage. The mutation harness loads and exercises decoded indexes.

On this Windows sandbox, native tests used a writable workspace TMP/TEMP directory. The wasm-bindgen test runner used an ordinary temporary directory outside the repository's `type: module` package boundary. Default sandbox temporary directories caused filesystem permission failures; reruns with writable temporary directories passed. libFuzzer execution is unavailable on this Windows environment, so its target was compiled and deterministic native mutation stress was run instead.

## Release measurements

Measured on Windows 11 x64, Intel Core Ultra 9 285H, Node v22.21.0. The dataset contains 100,000 vectors with 128 float32 dimensions, L2, seed 42, default HNSW m=12, m0=24, efConstruction=100, efSearch=50. Each buffer is 57,265,029 bytes.

Three separate fresh Node processes per mode were run after compiler and test jobs completed. Each first-call timer includes the JavaScript-to-WASM copy, checksum, decode, structural/metadata validation and BQ reconstruction where enabled. Module initialization and file I/O are excluded. Each process then performs seven warm loads.

| Mode | Fresh-process load times (ms) | Warm medians per process (ms) |
| --- | --- | --- |
| Plain | 296.92 / 272.48 / 297.37 | 152.26 / 137.25 / 132.11 |
| BQ reconstruction | 358.10 / 241.00 / 288.52 | 167.76 / 138.23 / 139.61 |

All six final cold loads meet the architecture's <500 ms target on this machine. Before optimization, BQ first calls were 517.57–604.81 ms. Packing each quantized byte once removed that measured bottleneck without changing bit order or the wire format. Trials taken during concurrent compilation included a 678.51 ms outlier; these diagnostic trials and all final raw samples are preserved in [the measurement data](../benchmarks/2026-10-19_persistence_load.json). These are local measurements, not a guarantee for other hardware.

| Release asset | Raw bytes | gzip bytes |
| --- | ---: | ---: |
| `edgevec_bg.wasm` | 683,701 | 262,884 |
| `edgevec.js` | 161,716 | 29,205 |
| Combined WASM + generated JS | 845,417 | 292,089 |

The WASM artifact is below the current 700 KB WASM size budget. The combined compressed runtime is below the architecture's 500 KB compressed bundle target. Source snippets add a small separate JavaScript transfer; their lifecycle code is also included in both published runtime directories.

## Publication and remaining scope

The fix is prepared on an isolated branch based on GitHub `main`; four unrelated private local commits and pre-existing working-tree files are excluded. `pkg/` and `docs/demo/pkg/` contain the refreshed raw generated runtime, declarations, and required storage snippet. GitHub PR/main workflows and the Pages deployment are verified separately during publication.

The pre-existing high-level TypeScript wrapper build errors and its `load()` dimension bookkeeping are outside this raw WASM persistence fix. The pre-existing Hamming/binary `compact()` panic with tombstones is also unchanged; the fuzz exercise explicitly avoids that unrelated path. No npm package or crates.io version is published by this GitHub source/demo deployment.

These exclusions describe the original persistence PR. The subsequent [wrapper and compaction closeout](2026-10-20_closeout_validation.md) addresses the TypeScript, packaging, and binary-compaction gaps and restores binary compaction to the mutation exercise.

See [the second hostile review](2026-10-19_issues_11_12_persistence_v2.md) for the final review verdict and disposition of the original findings.

## Hosted CI follow-up

The first hosted Linux suite, WASM persistence job, all-target fuzz build, x86 regression check, and performance benchmark workflow passed. Other jobs exposed maintenance issues that were not visible with the local toolchain.

The CI follow-up normalizes three existing byte-array constants to equivalent byte strings for newer Clippy. It constrains bitvec to its 1.0 minor line, following the dependency's documented guidance for a fixed MSRV; this retains the locally validated dependency series. Both existing Miri jobs keep their borrow/provenance checks and test coverage, and a successful hosted result is required before merge.

ARM release builds, complete library tests, SIMD tests, and lint run on GitHub's native Ubuntu ARM64 runners. This replaces the failed cross/Docker setup while preserving command coverage. The x86 regression job remains separate.

The optional mobile workflow has a committed dependency lockfile, uses Node 22 as required by the resolved Selenium dependency, caches that lockfile, and copies the generated storage snippets alongside the WASM glue. Its legacy Safari 15/16 matrix builds the scalar runtime. The mobile page uses the ID actually returned by insertion for search/metadata assertions. All 12 page API cases passed against the release runtime in Node; real-device results are a separate BrowserStack check.
