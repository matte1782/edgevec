# HOSTILE_REVIEWER: Rejection — Issues #11 / #12 Persistence Fix (uncommitted working tree)

**Date:** 2026-10-19
**Artifact:** Uncommitted changes fixing #11 (buffer save/load) and #12 (IndexedDB corrupted data / hang)
**Scope:** `src/persistence/buffer.rs` (new), `src/wasm/mod.rs`, `src/metadata/types.rs`, `src/flat/mod.rs`, `src/error.rs`, `src/persistence/mod.rs`, `src/js/storage.js`, `tests/wasm_persistence.rs`, `tests/js/storage.test.mjs` (new), `.github/workflows/ci.yml`, `package.json`, CHANGELOG, README, `docs/api/*`, `docs/guides/BINARY_QUANTIZATION.md`
**Author:** RUST_ENGINEER / WASM_SPECIALIST
**Type:** Code + Format design + Documentation
**Status:** REJECTED (NO_GO)

---

## Summary

The core diagnosis is correct and the happy path works: postcard cannot decode internally or adjacently tagged enums, and both were fixed. The `MetadataValue` byte-compatibility claim holds. The IndexedDB shim is much more robust than before.

The change fails the gate for three reasons:

1. **Validation does not deliver the guarantee it states.** `buffer.rs:17-20` and the CHANGELOG say malformed input is rejected "instead of causing out-of-bounds panics later". Hostile probes show that buffers which pass the CRC and `validate_hnsw` still panic on the first `search()` or `insert()`. One malformed field also panics inside the decoder itself, in `bitvec`.
2. **The new CI job cannot pass**, because it reads a `Cargo.lock` that is not in the repository.
3. **The backward-compatibility claim is false** for 0.6.0 to 0.8.0, and the compatibility tests cannot detect that class of regression.

### Probe evidence (re-runnable, not committed to the repo)

The probes ran against a scratch copy of the crate with 9 native tests appended to `persistence::buffer::tests`:
`C:\Users\matte\AppData\Local\Temp\claude\C--Users-matte-Desktop-Desktop-OLD-AI-Universit--AI-courses-personal-project-fortress-problem-driven-research-fortress-edgevec\f0f3565a-ef4d-479a-ba9f-e98c25ce4557\scratchpad\probe\src\persistence\buffer.rs`
(build output: `target/probe/` in the repo, which is gitignored). Result: 9/9 confirmed. Panic messages:

| Probe | Validation result | Panic |
|:--|:--|:--|
| `data_f32` shorter than `len*dims` | `validate_hnsw` Ok | `storage/mod.rs:621` "get_vector: VectorId 6 out of bounds (idx=5, end=48, data_len=40)" on `search` |
| `storage.config = Binary(8)` on an L2 index | Ok | `storage/mod.rs:391` "range end index 1 out of range for slice of length 0" on `search` |
| `storage.next_id = 500` (len 10) | Ok | `storage/mod.rs:621` "VectorId 500 out of bounds" on `insert` |
| `storage.next_id = 1` | Ok | no panic; `insert` hands out a **duplicate** `VectorId(1)` |
| `NeighborPool.buckets = []` | Ok | `hnsw/neighbor.rs:145` "index out of bounds: the len is 0 but the index is 0" on `insert` |
| free-list offset `u32::MAX-1024` | Ok | `hnsw/insert.rs:999` "range start index 4294966271 out of range for slice of length 176" on `insert` |
| `deleted` BitVec `bits=127`, 1 word | never reached | `bitvec-1.0.1/src/vec/api.rs:447` "bit-vector capacity exceeded: 127 > 64" **inside `decode_legacy`** |
| NaN in `data_f32` | Ok | accepted silently |
| NaN `MetadataValue::Float` | decoded | accepted silently |

An IndexedDB probe (`scratchpad\blocked_probe.mjs`) ran against the repo's `src/js/storage.js` and `fake-indexeddb`:

```
first write : {"err":"EdgeVecDB upgrade is blocked by another open connection (...)"}
second write: {"hang":true}
read        : {"hang":true}
after close, read: {"ok":{"0":2}}
```

---

## Findings

### Critical (BLOCKING)

- **[C1] `fromBytes()` / `load()` accept buffers that pass CRC and `validate_hnsw` and then panic on the first search or insert**
  - Location: `src/persistence/buffer.rs:256-302` (`validate_hnsw`). The false claim appears at `buffer.rs:17-20` and in CHANGELOG.md:24 ("with structural validation on load").
  - Evidence: the probe table above. `validate_hnsw` checks only dimensions equality, `1 <= node.vector_id <= storage.len()` (where `len()` is `deleted.len()`, `storage/mod.rs:580`), neighbor range vs pool length, and entry point. It does **not** check:
    1. `data_f32.len() == len * dimensions` for `Float32`, the `quantized_data` / `binary_data` lengths for the other storage types, or that `storage.config` is compatible with `config.metric`. A `QuantizedU8` storage always deserializes with `quantizer: None` (`storage/mod.rs:87`, `#[serde(skip)]`), so `get_vector` hits `expect("quantizer must be initialized")` at `storage/mod.rs:652`.
    2. `storage.next_id == len + 1`.
    3. `NeighborPool.buckets.len() == 32`, and that every free-list offset plus the bucket capacity lies within `buffer`.
    4. **wasm32 overflow:** `node.neighbor_offset as usize + node.neighbor_len as usize` at `buffer.rs:277`. On wasm32, `usize` is 32-bit and `[profile.release]` (Cargo.toml:281-285) leaves overflow checks off. With `neighbor_offset = 0xFFFF_FFF0` and `neighbor_len = 0x20`, `end` wraps to `0x10`, so validation passes. The identical bounds check in `hnsw/search.rs:219-226` (also `:736-743`, `graph.rs:753-761`, `:775-782`) passes too, and `&buffer[start..end]` panics with start > end. The native test `validate_hnsw_rejects_neighbor_overflow` cannot catch this because `usize` is 64-bit there. In debug wasm builds the addition panics inside `validate_hnsw` itself. This was found by inspection and was not run on wasm32.
  - Impact: on `wasm32-unknown-unknown` a panic is a trap. It aborts without unwinding, so the wasm-bindgen borrow flag of the `&mut self` receiver (insert) is never released, and the loaded `EdgeVec` handle is permanently unusable. The fix targets issue #11, where users load **files from disk**. The CRC is explicitly not authentication (`buffer.rs:17`). Legacy buffers have **no CRC at all** (`buffer.rs:119-121`), so accidental corruption of pre-0.10 data reaches these paths directly. This violates "no panics in library code" (HOSTILE_GATE_CHECKLIST.md:163) and the artifact's own contract.
  - Required action: extend validation to cover items 1-4. Use `checked_add`, or compute in `u64`, for every offset+len. Validate `buckets`. Add regression tests for each probe case (wasm32 tests for the overflow). Until full coverage exists, either remove the "rejected instead of panicking" claim or make it true.

- **[C2] Panic inside the decoder, plus a path to reading uninitialized memory, through `bitvec` 1.0.1 `BitVec` deserialization (`VectorStorage.deleted`)**
  - Location: `VectorStorage.deleted: BitVec` (`src/storage/mod.rs:91`), reached from `buffer::decode_exact` (`buffer.rs:183-192`) and `decode_legacy` (`buffer.rs:222-224`). The dependency code is `bitvec-1.0.1/src/serdes/slice.rs:151-158` (`try_from_vec`, then unchecked `set_head` and `set_len(bits)`) and `vec/api.rs:445-454`.
  - Evidence: probe `probe_deleted_bitvec_len_beyond_capacity_panics_inside_decode` panics with "bit-vector capacity exceeded: 127 > 64". `try_from_vec` (`vec.rs:243-251`) keeps the source `Vec`'s **capacity**, and `set_len` only asserts `bits <= capacity * W`. When the `data` vector exceeds serde's 1 MiB preallocation cap (`serde/core/private/size_hint.rs:12-23`; on wasm32 that is more than 262,144 words), the `Vec` grows by doubling, capacity exceeds len, and a crafted `bits` between `len*W` and `cap*W` passes the assert. The `BitVec` then covers uninitialized spare capacity, which is UB reachable from a safe public API. The UB part was derived from source and not executed. `slice.rs:217` also contains `.map_err(|_| todo!())`.
  - Impact: the error-returning API (`fromBytes`, documented to throw `ERR_CORRUPTION`) traps instead. This happens before any validation runs, so C1's fix alone cannot cover it.
  - Required action: deserialize `deleted` through a `#[serde(deserialize_with = ...)]` shim. It must read the same wire layout (order, head, bits, data), reject `head != 0 || bits > data.len() * W`, and build the value with `BitVec::from_vec(data)` plus `truncate(bits)`. Add a regression test.

### Major (MUST FIX)

- **[M1] The new CI job `wasm-persistence` cannot work: `Cargo.lock` is not in the repository**
  - Location: `.github/workflows/ci.yml:126-135`; `.gitignore:3` (`Cargo.lock`); `git ls-files Cargo.lock` returns nothing.
  - Failure: on checkout `grep ... Cargo.lock` fails. Because the command substitution is in an assignment, `v` is empty and the step still exits 0. `taiki-e/install-action` then receives `tool: wasm-bindgen@` and fails. If it resolved to "latest" instead, `wasm-bindgen-test-runner` would not match the `wasm-bindgen` version cargo resolves, giving a schema mismatch.
  - Required action: run `cargo generate-lockfile` (or `cargo update -w`) before the resolve step, or derive the version with `cargo metadata --format-version 1 | jq`. Add `set -euo pipefail` and fail on an empty version. Show a green run.

- **[M2] The backward-compatibility claim "0.9.0 and earlier" is false for 0.6.0 to 0.8.0**
  - Location: CHANGELOG.md:16, `src/wasm/mod.rs:1798` (`fromBytes` doc), `src/persistence/buffer.rs:24-33`.
  - Evidence: `git show v0.8.0:src/wasm/mod.rs` (lines 262-269; identical in v0.6.0 and v0.7.0) shows `EdgeVec { inner: HnswIndex, storage: VectorStorage, metadata }`, with no enum and no `"\x04hnsw"` prefix. `git log -S'tag = "variant_type"'` shows the tagged enum was introduced in ff0f940, first tagged v0.9.0. A 0.6-0.8 buffer starts with `postcard(HnswConfig)`, so `parse()` returns `InvalidMagic` and `ERR_CORRUPTION`. Those releases could round-trip indexes without metadata, so the data is real and was previously loadable.
  - Required action: either decode the 0.6-0.8 layout as an additional fallback (with a fixture from a real v0.8.0 build), or correct every claim to "0.9.0" and document that 0.6-0.8 IndexedDB data is not recoverable.

- **[M3] The compatibility tests are tautological, and the payload schema is unversioned and unguarded**
  - Location: `buffer.rs:471-488` and `tests/wasm_persistence.rs:177-193` build the "legacy" bytes from the **current** `HnswIndex` / `VectorStorage` / `MetadataStore` types. The envelope `BUFFER_VERSION` (`buffer.rs:51`) versions the header only.
  - Failure scenario: someone adds a `#[serde(default)]` field to `HnswIndex`. That attribute pattern already appears at `hnsw/graph.rs:269,275,285`, `storage/mod.rs:68-84` and `wasm/mod.rs:651`, and postcard ignores it. Every v1 buffer and every 0.9.0 buffer in user storage then becomes unreadable, while all tests stay green. This is the same failure class as #12.
  - Required action: commit golden fixtures, one from the real 0.9.0 build and one from this v1 writer (HNSW with metadata and deletes, HNSW with BQ, Flat), plus a test that decodes them and checks search results. Document on `EdgeVec`, `IndexVariant`, `HnswIndex`, `VectorStorage`, `NeighborPool` and `MetadataStore` that any serde change requires a `BUFFER_VERSION` bump and a migration.

- **[M4] A blocked repair makes later save()/load() calls hang, which is the #12 symptom in a new scenario**
  - Location: `src/js/storage.js:86-94` (rejects on `blocked`) and `:22-39`.
  - Evidence: the `blocked_probe.mjs` output above. Rejecting on `blocked` does not cancel the version-change request. It stays first in IndexedDB's connection queue, and every later `indexedDB.open("EdgeVecDB")` waits behind it with no event fired. Tabs running ≤0.9.0 never close their connections (old `read()` never called `close()`) and ignore `versionchange`, so the wait can last indefinitely. `tests/js/storage.test.mjs:101-106` only checks the first call.
  - Required action: keep one module-level pending-repair promise. Concurrent and later callers must await or fail fast on that promise rather than queue new opens behind it. Alternatively, do not reject on `blocked` but report it and add a timeout. Add a test that issues a second operation while the blocker stays open.

- **[M5] Peak WASM linear memory during `save()` / `toBytes()` doubles, with no measurement or justification**
  - Location: `src/wasm/mod.rs:1785-1795` and `buffer.rs:100-107`.
  - Evidence: `postcard::to_stdvec(self)` produces the payload, then `encode()` allocates a second buffer of `16 + payload.len()` and copies it. Both live in linear memory at once. The old `save()` held one copy. WASM memory never shrinks, so the high-water mark stays raised for the life of the page. At 100k x 768-d (~307 MB of vectors), this adds roughly 300 MB at peak. That is relevant to the ~1 GB Safari ceiling (HOSTILE_GATE_CHECKLIST.md:61) and to "No regressions vs baseline (or justified)" (:195).
  - Required action: serialize once into a buffer with a 16-byte placeholder, e.g. `postcard::to_extend(self, vec![0u8; 16])`, then patch length and CRC in place. Alternatively, benchmark the cost and justify it.

- **[M6] `fromBytes()` bypasses the project's NaN guard and metadata limits**
  - Location: `src/wasm/mod.rs:689-731` (`decode`); compare the insert paths at `wasm/mod.rs:863-865, 1048-1049, 1491-1493, 1676`, which enforce finiteness, and `metadata::validation` (limits and NaN floats).
  - Evidence: probes `probe_nan_vectors_accepted` and `probe_metadata_nan_and_limits_not_validated`.
  - Rule violated: CLAUDE.md §3.1 "NaN Guard: All public entry points validate finiteness".
  - Required action: during decode, reject non-finite `data_f32` and validate metadata keys and values with the existing validators (`validate_key`, `validate_value`, `MAX_KEYS_PER_VECTOR`). This covers both `EdgeVec.metadata` and `HnswIndex.metadata`.

- **[M7] No fuzz target for a new untrusted-input binary parser**
  - Location: `fuzz/fuzz_targets/` has no target for `persistence::buffer`.
  - Rule violated: CLAUDE.md §3.1 "Fuzzing for all parsers/inputs"; HOSTILE_GATE_CHECKLIST.md:175.
  - Impact: C1 and C2 are exactly what a fuzzer finds within minutes.
  - Required action: add a fuzz target covering `parse`, then decode (V1 and legacy), then validate, then a bounded sequence of `search` and `insert` on the decoded index. It must report no panics.

- **[M8] Other docs still claim `save()` writes to the filesystem in Node.js, and the Node example is now broken**
  - Location: `docs/API_REFERENCE.md:247`; `docs/TUTORIAL.md:420` and `:442-444`; `examples/nodejs/quickstart.js:41-50`.
  - Failure: `quickstart.js` calls `client.save()` and then `EdgeVecClient.load()` under Node. With the new shim, step 4 rejects with "IndexedDB is not available". Before, it hung.
  - Rule violated: CLAUDE.md §3.1, which says to grep ALL docs when correcting a claim.
  - Required action: rewrite these sections to use `toBytes()` / `EdgeVec.fromBytes()` with `node:fs`, and run the example.

### Minor (SHOULD FIX)

- **[m1]** The CRC covers only the payload (`buffer.rs:106`, `:167`). Header bytes 4-11 are unprotected, so a single bit flip of flag bit 0 silently enables or disables BQ on load. Include `bytes[4..12]` in the CRC.
- **[m2]** Unknown flags and non-zero reserved bytes are reported as `Corrupted` and therefore `ERR_CORRUPTION` (`buffer.rs:143-154`). The docs promise `ERR_PERSISTENCE` for newer-format buffers. Rejecting unknown flags is the right default, but document the policy (are all flags "must understand"?) and map the error to the unsupported-format code.
- **[m3]** `validate_flat` / `is_consistent` (`flat/mod.rs:297-301`) accept `dimensions == 0` with any `count`, since `count * 0 == 0 == vectors.len()`. A search then iterates `count` times (for example 2^40), which hangs. Require `dimensions > 0`.
- **[m4]** `deleted_count`, node `deleted` flags vs `storage.deleted`, and `nodes.len()` vs `storage.len()` are not cross-checked. `liveCount()` / `tombstone_ratio()` / `needsCompaction()` then return wrong values after loading a crafted or buggy buffer (`hnsw/graph.rs:1520-1547`).
- **[m5]** The TypeScript wrappers are not updated. `pkg/edgevec-wrapper.ts:405-412` (`EdgeVecIndex.load` still sets `dims = 0` even though `dimensions()` now exists) and `pkg/edgevec-types.d.ts:357-360` have no `toBytes` / `fromBytes`. Neither does `wasm/EdgeVecClient.ts:97-118`.
- **[m6]** The tracked build artifacts are stale. `pkg/edgevec.{js,d.ts,_bg.wasm}` and `docs/demo/pkg/**` still contain the old broken `storage.js` (10 snippet copies under `docs/demo/pkg/snippets/*/src/js/storage.js`), so the live demo still has #12. Rebuild and `git add -f` the snippets, per CLAUDE.md §3.1.
- **[m7]** After a repair, `EdgeVecDB` is at version 2. Any tab or app still on ≤0.9.0 calls `indexedDB.open("EdgeVecDB", 1)` and gets `VersionError`, which breaks downgrade and mixed-version deployments. This is not documented in the CHANGELOG.
- **[m8]** The Rust↔JS glue (`IndexedDbBackend::write(&Uint8Array)`, `read` → `Uint8Array::new`) is not exercised in CI. The end-to-end script (the reporter's repro plus data from the old build) was not committed.
- **[m9]** There are unrelated whitespace-only edits to historical reviews: `docs/reviews/phase4_gate_review_v2.md` and `docs/reviews/week3_W3D11_review_v3.md`. Revert them.
- **[m10]** CHANGELOG: `### Fixed` now comes before `### Added`, against the Keep a Changelog order. A stray blank line is left where the old Fixed block was. The historical v0.6.0 entry at CHANGELOG.md:532 still shows the non-existent `index.loadSnapshot(...)`; annotate it.
- **[m11]** There is no measurement against the budgets for "Index load time <500 ms @100k" (`fromBytes`, including the BQ rebuild) or "Bundle < 700 KB" (new code paths).
- **[m12]** Allocation amplification is bounded but undocumented. Huge length prefixes do **not** cause huge allocations: serde caps preallocation at 1 MiB, and postcard checks byte and string lengths against the remaining input. Element amplification remains: for example one input byte becomes a 12-byte `Vec` header in `buckets`, and HashMap entries in `MetadataStore` amplify further, so memory can reach roughly 10x the input size. Document this in the `fromBytes` docs.

---

## Verified (no finding)

- **MetadataValue byte compatibility: correct.** Under postcard, the old adjacently-tagged derive emitted `serialize_unit_variant` (varint index) followed by the content, which is byte-identical to an externally-tagged newtype variant. `types.rs` test `test_postcard_reads_legacy_adjacently_tagged_bytes` asserts byte equality for every variant.
- **Other formats are unaffected.** `serde_json` is human-readable, so its output is unchanged (including `serde_json::to_value` at `wasm/mod.rs:1424, 3293, 3479`). `MetadataStore::to_postcard` uses its own `PostcardValue` mirror (`metadata/serialize.rs:42-53`). `serde_wasm_bindgen` is used only for `MemoryConfig` and the pressure / recommendation types. `bincode` is not a dependency.
- **`unsafe` and liveness are unaffected.** `save_stream`'s `transmute` (`wasm/mod.rs:1749-1750`) is untouched. `liveness` is re-armed by `#[serde(default = "default_liveness")]` (`:671`) and explicitly in the legacy path (`:713`).
- **Huge postcard length prefixes do not cause huge allocations** (see m12).
- **New non-test code has no `unwrap`, `expect`, `TODO` or `FIXME`.** `rustfmt --check` is clean on all changed Rust files.
- **storage.js lifecycle is correct apart from M4.** The `settled` flags are correct, the transaction lifecycle (resolve on `oncomplete`, reject on `onerror` / `onabort`, always `close()`) is correct, the in-page concurrent repair works (`onversionchange` closes the connection synchronously, so no `blocked` fires), and the `VersionError` fallback is correct.

---

## VERDICT

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: REJECT (NO_GO)                                  │
│                                                                     │
│   Artifact: Issues #11/#12 persistence fix (working tree)           │
│   Author: RUST_ENGINEER / WASM_SPECIALIST                           │
│                                                                     │
│   Critical Issues: 2                                                │
│   Major Issues: 8                                                   │
│   Minor Issues: 12                                                  │
│                                                                     │
│   Disposition: BLOCK merge until C1, C2, M1-M8 are resolved.        │
└─────────────────────────────────────────────────────────────────────┘
```

## Required Actions Before Resubmission

1. [ ] C1: complete structural validation (storage data lengths and type vs metric, `next_id`, `buckets` and free-list offsets, overflow-safe offset arithmetic) with one regression test per probe case, including a wasm32 test for the overflow.
2. [ ] C2: add a validating `deserialize_with` for `VectorStorage.deleted`, plus a test.
3. [ ] M1: fix lockfile resolution in the CI job and show a green run.
4. [ ] M2: support the 0.6-0.8 layout or correct every compatibility claim.
5. [ ] M3: add committed golden fixtures, a test that decodes them, and the schema-change rule on the serialized types.
6. [ ] M4: stop later calls from hanging after a blocked repair, with a test.
7. [ ] M5: remove the double payload copy, or benchmark and justify it.
8. [ ] M6: add finiteness and metadata validation on decode.
9. [ ] M7: add a fuzz target for the buffer parser plus post-load operations.
10. [ ] M8: fix API_REFERENCE.md, TUTORIAL.md and `examples/nodejs/quickstart.js`.

Resubmit with the `[REVISED]` tag via `/review`.

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-10-19*
*Verdict: REJECTED*
