# WASM persistence fixtures

These are frozen bytes captured from real `wasm-pack --target web` builds,
not buffers synthesized from the decoder's current Rust types. Keep them when
the Rust structs change: a wire-layout change requires a `BUFFER_VERSION`
bump and a migration, rather than regenerating fixtures to make tests pass.

| Files | Writer |
| --- | --- |
| `v0_8_0_hnsw_l2.bin` | v0.8.0, `a3fa5bc7b91d395774e011726380232830050b67` |
| `v0_9_0_hnsw_l2.bin`, `v0_9_0_flat.bin` | v0.9.0, `ab196240eb1e8f1186b3408d2832a66b1b9be343` |
| `v1_hnsw_l2.bin`, `v1_hnsw_cosine_bq.bin`, `v1_flat.bin` | EVWB v1 writer introduced by the issues #11/#12 changes in this patch |

`expected.json` records the writer's dimensions, query vectors, full search
results, and (for HNSW) live count, deleted IDs, metadata and BQ state. HNSW
fixtures contain 40 vectors, metadata of all five supported types, and two
deletions. Flat fixtures contain 20 binary vectors. The legacy writer's missing
BQ state cannot be recovered; only the v1 flag preserves it.

`generate.mjs` uses a deterministic seed and captures legacy bytes from
`fake-indexeddb` after `save()`. The `--current` flag captures `toBytes()` and
also generates a cosine index with BQ enabled. To reproduce, build each old
tag in a separate checkout and run from this repository (install the root
development dependencies first):

```sh
# In each writer's checkout:
wasm-pack build --target web --dev --out-dir /path/to/writer-pkg --no-pack

# In this checkout, with the corresponding three build directories:
node tests/fixtures/wasm_buffers/generate.mjs v0_8_0 /path/to/v0.8-pkg
node tests/fixtures/wasm_buffers/generate.mjs v0_9_0 /path/to/v0.9-pkg
node tests/fixtures/wasm_buffers/generate.mjs v1 /path/to/v1-pkg --current
```

The native `persistence::buffer::tests::fixtures` tests guard decoding and
metadata recovery. `tests/wasm_persistence.rs` verifies every recorded result
ID in WASM, plus deletes, BQ state and complete metadata values. Native search
checks use the exact-match first result because SIMD arithmetic differs across
architectures; the full ordering is verified on the WASM target that wrote it.
