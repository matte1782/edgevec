# BQ validation and runnable onboarding

This follow-up builds on main `59146c7` and corrects a reproduced WASM trap and
the examples that introduced users to unsupported configuration paths.

## Corrected behavior

`HnswIndex::with_bq()` and `enable_bq()` reject packed binary storage before
allocation or mutation. Previously, `enableBQ()` on a populated WASM binary
index reached `get_vector()` and trapped with `RuntimeError: unreachable`.
Rejection now preserves the existing structured error contract (`ERR_GRAPH`),
and the index remains usable for binary insertion, search, and buffer loading.
Float and native SQ8 storage retain their supported BQ behavior.

README and active API examples construct `EdgeVecConfig` and explicitly enable
BQ. SQ8 configuration is identified as a native Rust storage API. BQ guidance
describes the implemented zero threshold, least-significant-bit-first packing,
and the distinction between compressed codes and retained F32 storage. Current
website text uses the same distinction; historical release archives remain
historical records.

Node examples use real EVWB files. The quickstart asserts unambiguous L2 nearest
neighbors before and after loading. The benchmark uses normalized cosine
vectors, validates the restored count/search, and completes persistence and
cleanup even if the P99 latency target misses, then returns failure.

Framework guide corrections distinguish the `EdgeVecIndex` wrapper from raw
`EdgeVec`. The raw React provider uses a keyed subtree so cleanup cannot free
an index while its old context is still published. Framework implementation
changes are outside this follow-up; hook source edits only clarify that the
application initializes WASM before mounting.

## Validation

| Check | Result |
|:------|:-------|
| Native targeted tests | 9 pass: 2 BQ validation, 4 compaction, 3 hybrid |
| Strict native lint | Rust 1.96, all targets, `-D warnings`, pass |
| Strict WASM lint | WASM library, `-D warnings`, pass |
| Formatting and TypeScript | Pass |
| WASM Rust persistence | 10 pass |
| Public SDK with release WASM | 17 pass, including binary BQ rejection and recovery |
| Jest | 33 pass |
| IndexedDB shim | 11 pass |
| Package/demo persistence glue | 5 pass each |
| Installed tarball consumers | ESM/CommonJS and strict Node16 type resolution pass |
| Node versions | Installed consumers pass on 16.20.2 and 22.21.0 |
| Runnable README quickstarts | Root, installed package, and API README all pass |
| React definitions | Independent strict check of edited Suspense/provider definitions: zero diagnostics |
| Node quickstart | File roundtrip and nearest-neighbor assertions pass |
| Full Node benchmark | 100k vectors, 128 dimensions, 1,000 searches; P99 1.143 ms |
| Node benchmark persistence | Save 245.80 ms; file reload/count/search verification 125.92 ms |
| Fresh-process `fromBytes()` | Plain: 139.29/121.00/124.35 ms; BQ: 133.95/134.01/133.32 ms |

The release WASM is 698,412 bytes, below the 700,000-byte budget. Combined gzip
WASM/JavaScript is 295,502 bytes. The rebuilt package and demo raw runtime files
and referenced storage snippet are byte-identical. Exact hashes, environment,
load samples, and measurement method are recorded in
[`2026-10-20_onboarding.json`](../benchmarks/2026-10-20_onboarding.json).
Snippet hashes use Git's canonical LF bytes.

The checked-in package and Pages demo intentionally use a scalar compatibility
build for the Safari 15/16 mobile baseline. The default source `npm run build`
enables SIMD; reproducing this compatibility artifact requires
`RUSTFLAGS="-C target-feature=-simd128" npm run build` in Bash. These are separate
compile-time profiles, with no automatic runtime fallback. All measurements
above use the frozen scalar artifact and compiler/flags recorded in the JSON.

Load timing excludes reading the EVWB file and WASM initialization. Benchmark
inputs are generated independently for this run; the results are validation
against budgets, not an algorithmic improvement comparison with earlier builds.

## Review and publication

Final independent review: **GO for source/demo publication**, with no remaining
functional blockers. The reviewer independently verified all 17 runtime cases,
the React definitions, matching package/demo hashes, and the final diff, and
reviewed the native guards, regression coverage, examples, tarball checks, and
performance evidence.

Hosted CI and actual mobile-device checks must pass on the final PR head before
merge. Main-branch checks and downloaded Pages artifacts are verified separately
after publication. No npm or crates.io version publication is part of this
source/demo follow-up.
