# Wrapper, compaction, and packaging closeout

This follow-up closes the wrapper/build and binary-compaction exclusions recorded
in the issues #11/#12 persistence validation. Publication uses an isolated branch
from GitHub main; unrelated private local commits and working-tree files remain
outside the public change.

## Corrected behavior

- Loaded `EdgeVecClient` and `EdgeVecIndex` use saved dimensions and live counts.
  The wrapper exposes persistence buffers and parsed sparse/hybrid results.
- Binary compaction rebuilds packed binary storage. SQ8 compaction retains exact
  encoded bytes and calibration, including ranges where decoding to f32 loses
  precision. Enabled BQ is rebuilt. WASM metadata follows reassigned vector IDs.
- Hybrid search translates sparse IDs into dense document IDs, excludes deleted
  documents before sparse top-k, and remaps pairings after compaction. Explicit
  `insertSparseForVector()` supports partial coverage and arbitrary insertion
  order. Pending future reservations reject compaction before mutation.
- Linear hybrid fusion converts distances to similarities and normalizes finite
  signed scores without range overflow. Non-finite dense/sparse search scores
  produce an error instead of invalid fused/JSON scores.
- WASM `save_stream()` owns a serialized snapshot, eliminating lifetime-erased
  references into a mutable/freed source. Its concatenated chunks now use EVWB,
  identical to `toBytes()`. Chunk sizes still clamp to at least 64 bytes. Memory
  includes the entire snapshot plus emitted JavaScript chunks.
- Both npm tarballs include runtime snippets and public exports. The core SDK
  has a generated CommonJS bundle and conditional CJS declarations. Node
  initialization reads the packaged WASM file, and browser bundling succeeds.
- The root dependency lock and build dependencies support clean-checkout
  typechecks/builds. CI executes unit, runtime, browser-bundling, and installed
  ESM/CommonJS tarball checks.

## Validation

| Check | Result |
|---|---|
| Full native suite | 3,490 pass; 32 existing ignored; zero failures |
| Strict lint | Native all-targets and WASM library, `-D warnings`, pass |
| TypeScript projects | Both typechecks pass |
| Jest unit tests | 33 pass |
| Actual public SDK/WASM runtime | 16 pass |
| Existing IndexedDB shim tests | 11 pass |
| Actual persistence glue, package runtime | 5 pass |
| Actual persistence glue, demo runtime | 5 pass |
| WASM Rust persistence tests under Node | 10 pass |
| Buffer mutation exercise | 200,000 iterations pass, now including binary compaction |
| Installed package declarations | ESM `.mts` and CommonJS `.cts`, Node16 resolution, strict, `skipLibCheck: false`; no diagnostics |
| Installed package runtime | ESM and CommonJS both pass on Node 22.21.0 and Node 16.20.2 |
| Browser bundle | Public emitted SDK bundles successfully with esbuild |

Hosted CI, mobile devices, and Pages are verified for the published commit
separately; merge requires successful hosted checks.

Artifact hashes and compressed sizes are recorded in
[`2026-10-20_closeout.json`](../benchmarks/2026-10-20_closeout.json). The raw WASM is
685,769 bytes; combined gzip WASM/JS is 293,018 bytes, within the existing budgets.
`pkg/` and `docs/demo/pkg/` contain identical raw runtime files and required snippets.

Three fresh-process 100k-vector/128-dimensional load trials measured 141.35,
139.17, and 144.40 ms for plain indexes, and 160.06, 153.78, and 155.05 ms with BQ.
All are below the 500 ms budget. These were measured after other builds/tests
finished. Initial contended trials, including a slower plain-index run, are
retained separately in the JSON; workload differences are not attributed to an
algorithmic optimization.

## Compatibility contracts

Native compaction retains its existing new-index contract: IDs are sequential and
its integrated metadata store starts empty. Native callers manage that metadata
and hybrid ID mapping; the WASM API remaps its separate metadata/pairing stores.
Sparse vectors and pairings remain runtime state, excluded from v0.9 snapshots;
regenerate them after loading. These contracts are explicit in the API docs.

No npm or crates.io version is published by this GitHub source/demo closeout.

## Hosted reporting follow-up

The first hosted closeout run passed all 12 PR checks and the four-device mobile
job. It exposed an existing reporting error hidden by `continue-on-error`: both
benchmark workflows attempted PR comments without integration write permission.
Reports now use the Actions job summary and artifacts without requesting broader
permissions. Regression failure status still propagates through `tee`, and an
incomplete/failed check is no longer summarized as a successful benchmark run.
