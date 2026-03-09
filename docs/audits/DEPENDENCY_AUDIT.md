# Dependency Audit

**Document:** `docs/audits/DEPENDENCY_AUDIT.md`
**Date:** 2026-03-08
**Status:** [PROPOSED]
**Scope:** All direct and transitive dependencies for `edgevec v0.9.0`
**Tools:** cargo-audit v0.22.0, cargo-deny (not installed -- skipped)

---

## 1. Vulnerability Scan (`cargo audit`)

```
$ cargo audit
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 946 security advisories (from C:\Users\matte\.cargo\advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (143 crate dependencies)

Crate:     atomic-polyfill
Version:   1.0.3
Warning:   unmaintained
Title:     atomic-polyfill is unmaintained
Date:      2023-07-11
ID:        RUSTSEC-2023-0089
URL:       https://rustsec.org/advisories/RUSTSEC-2023-0089
Dependency tree:
  atomic-polyfill 1.0.3
  └── heapless 0.7.17
      └── postcard 1.1.3
          └── edgevec 0.9.0

warning: 1 allowed warning found
```

**Result:** 0 vulnerabilities, 1 warning (unmaintained crate).

### RUSTSEC-2023-0089: `atomic-polyfill` is unmaintained

- **Severity:** Warning (unmaintained, not a security vulnerability)
- **Impact:** `atomic-polyfill` provides portable atomics for `no_std` targets. It is a transitive dependency via `postcard` -> `heapless` -> `atomic-polyfill`. The crate author recommends migrating to `portable-atomic`.
- **Risk to EdgeVec:** LOW. EdgeVec does not target `no_std` platforms where `atomic-polyfill` would be exercised. On all EdgeVec targets (native x86_64, wasm32), native atomics are available and `atomic-polyfill` simply re-exports `core::sync::atomic`.
- **Remediation:** Wait for `postcard` to update `heapless` to v0.8+ (which dropped `atomic-polyfill`). Alternatively, consider `postcard` v2.x when it stabilizes. No action required at this time.

---

## 2. Policy Check (`cargo deny check`)

**Status:** `cargo-deny` is NOT installed.

```
$ cargo deny --version
error: no such command: `deny`
```

**To install:** `cargo install cargo-deny`

**To run:** `cargo deny check` (requires a `deny.toml` configuration file)

**Recommendation:** Install `cargo-deny` and create a `deny.toml` to enforce license policies and ban unwanted crates. This is a non-blocking enhancement.

---

## 3. Dependency Tree

### 3.1 Direct Dependencies (from `Cargo.toml`)

#### Production Dependencies (14 crates)

| Crate | Version | Purpose |
|:------|:--------|:--------|
| `bitvec` | 1.0 | Bit-level vector operations (binary quantization) |
| `bytemuck` | 1.14 | Safe type casting for WASM, zero-copy deserialization |
| `cfg-if` | 1.0.4 | Conditional compilation helper |
| `crc32fast` | 1.3 | Checksum for persistence format |
| `log` | 0.4 | Logging facade |
| `pest` | 2.7 | PEG parser for filter expressions |
| `pest_derive` | 2.7 | Proc-macro for pest grammar compilation |
| `postcard` | 1.0 | Binary serialization (serde format) |
| `rand` | 0.8 | Random number generation (HNSW construction) |
| `rand_chacha` | 0.3 | Deterministic PRNG with serde support |
| `serde` | 1.0 | Serialization framework |
| `serde_bytes` | 0.11 | Efficient byte slice serialization |
| `serde_json` | 1.0 | JSON serialization for WASM API |
| `thiserror` | 1.0 | Ergonomic error type derivation |

#### Optional Dependencies (1 crate)

| Crate | Version | Feature | Purpose |
|:------|:--------|:--------|:--------|
| `rayon` | 1.10 | `parallel` | Parallel PQ training (not available on WASM) |

#### WASM-Only Dependencies (8 crates, `cfg(target_arch = "wasm32")`)

| Crate | Version | Purpose |
|:------|:--------|:--------|
| `console_error_panic_hook` | 0.1 | Panic handler for browser console |
| `console_log` | 1.0 | Log output to browser console |
| `getrandom` | 0.2.14 | JS-based RNG on WASM targets |
| `js-sys` | 0.3 | JavaScript interop bindings |
| `serde-wasm-bindgen` | 0.6 | Serde <-> JsValue conversion |
| `wasm-bindgen` | 0.2 | Rust/WASM FFI bindings |
| `wasm-bindgen-futures` | 0.4 | Async/await support for WASM |
| `web-sys` | 0.3 | Web API bindings (IndexedDB, Performance, etc.) |

#### Dev Dependencies (5 crates)

| Crate | Version | Purpose |
|:------|:--------|:--------|
| `criterion` | 0.5 | Benchmarking framework |
| `proptest` | =1.4.0 | Property-based testing |
| `serde_json` | 1.0 | JSON in tests (also a production dep) |
| `tempfile` | =3.10.0 | Temporary files for persistence tests |
| `wasm-bindgen-test` | 0.3 | WASM test runner |

### 3.2 Full Dependency Tree

```
edgevec v0.9.0
├── bitvec v1.0.1
│   ├── funty v2.0.0
│   ├── radium v0.7.0
│   ├── serde v1.0.228
│   │   ├── serde_core v1.0.228
│   │   └── serde_derive v1.0.228 (proc-macro)
│   │       ├── proc-macro2 v1.0.103
│   │       │   └── unicode-ident v1.0.22
│   │       ├── quote v1.0.42
│   │       └── syn v2.0.111
│   ├── tap v1.0.1
│   └── wyz v0.5.1
│       └── tap v1.0.1
├── bytemuck v1.24.0
│   └── bytemuck_derive v1.10.2 (proc-macro)
├── cfg-if v1.0.4
├── crc32fast v1.5.0
│   └── cfg-if v1.0.4
├── log v0.4.29
├── pest v2.8.4
│   ├── memchr v2.7.6
│   └── ucd-trie v0.1.7
├── pest_derive v2.8.4 (proc-macro)
│   ├── pest v2.8.4
│   └── pest_generator v2.8.4
│       ├── pest v2.8.4
│       ├── pest_meta v2.8.4
│       │   └── pest v2.8.4
│       │   [build-dependencies]
│       │   └── sha2 v0.10.9
│       │       ├── cfg-if v1.0.4
│       │       ├── cpufeatures v0.2.17
│       │       └── digest v0.10.7
│       │           ├── block-buffer v0.10.4
│       │           │   └── generic-array v0.14.7
│       │           │       └── typenum v1.19.0
│       │           └── crypto-common v0.1.7
│       ├── proc-macro2 v1.0.103
│       ├── quote v1.0.42
│       └── syn v2.0.111
├── postcard v1.1.3
│   ├── cobs v0.3.0
│   │   └── thiserror v2.0.17
│   ├── heapless v0.7.17
│   │   ├── hash32 v0.2.1
│   │   │   └── byteorder v1.5.0
│   │   ├── serde v1.0.228
│   │   ├── spin v0.9.8
│   │   │   └── lock_api v0.4.14
│   │   │       └── scopeguard v1.2.0
│   │   └── stable_deref_trait v1.2.1
│   │   [build-dependencies]
│   │   └── rustc_version v0.4.1
│   │       └── semver v1.0.27
│   └── serde v1.0.228
├── rand v0.8.5
│   ├── rand_chacha v0.3.1
│   │   ├── ppv-lite86 v0.2.21
│   │   │   └── zerocopy v0.8.31
│   │   │       └── zerocopy-derive v0.8.31 (proc-macro)
│   │   ├── rand_core v0.6.4
│   │   │   └── getrandom v0.2.16
│   │   │       └── cfg-if v1.0.4
│   │   └── serde v1.0.228
│   └── rand_core v0.6.4
├── rand_chacha v0.3.1
├── serde v1.0.228
├── serde_bytes v0.11.19
│   └── serde_core v1.0.228
├── serde_json v1.0.145
│   ├── itoa v1.0.15
│   ├── memchr v2.7.6
│   ├── ryu v1.0.20
│   └── serde_core v1.0.228
└── thiserror v1.0.69
    └── thiserror-impl v1.0.69 (proc-macro)
[dev-dependencies]
├── criterion v0.5.1
│   ├── anes v0.1.6
│   ├── cast v0.3.0
│   ├── ciborium v0.2.2
│   │   ├── ciborium-io v0.2.2
│   │   ├── ciborium-ll v0.2.2
│   │   │   └── half v2.7.1
│   │   └── serde v1.0.228
│   ├── clap v4.5.53
│   │   └── clap_builder v4.5.53
│   │       ├── anstyle v1.0.13
│   │       └── clap_lex v0.7.6
│   ├── criterion-plot v0.5.0
│   │   ├── cast v0.3.0
│   │   └── itertools v0.10.5
│   │       └── either v1.15.0
│   ├── is-terminal v0.4.17
│   │   └── windows-sys v0.61.2
│   │       └── windows-link v0.2.1
│   ├── itertools v0.10.5
│   ├── num-traits v0.2.19
│   │   └── libm v0.2.15
│   ├── once_cell v1.21.3
│   ├── oorandom v11.1.5
│   ├── regex v1.12.2
│   │   ├── regex-automata v0.4.13
│   │   │   └── regex-syntax v0.8.8
│   │   └── regex-syntax v0.8.8
│   ├── serde v1.0.228
│   ├── serde_json v1.0.145
│   ├── tinytemplate v1.2.1
│   └── walkdir v2.5.0
│       ├── same-file v1.0.6
│       │   └── winapi-util v0.1.11
│       │       └── windows-sys v0.61.2
│       └── winapi-util v0.1.11
├── proptest v1.4.0
│   ├── bitflags v2.10.0
│   ├── lazy_static v1.5.0
│   ├── num-traits v0.2.19
│   ├── rand v0.8.5
│   ├── rand_chacha v0.3.1
│   ├── rand_xorshift v0.3.0
│   │   └── rand_core v0.6.4
│   ├── regex-syntax v0.8.8
│   └── unarray v0.1.4
├── tempfile v3.10.0
│   ├── cfg-if v1.0.4
│   ├── fastrand v2.3.0
│   └── windows-sys v0.52.0
│       └── windows-targets v0.52.6
│           └── windows_x86_64_msvc v0.52.6
└── wasm-bindgen-test v0.3.56
    ├── async-trait v0.1.89 (proc-macro)
    ├── cast v0.3.0
    ├── js-sys v0.3.83
    │   ├── once_cell v1.21.3
    │   └── wasm-bindgen v0.2.106
    │       ├── cfg-if v1.0.4
    │       ├── once_cell v1.21.3
    │       ├── wasm-bindgen-macro v0.2.106 (proc-macro)
    │       │   ├── quote v1.0.42
    │       │   └── wasm-bindgen-macro-support v0.2.106
    │       │       ├── bumpalo v3.19.0
    │       │       ├── proc-macro2 v1.0.103
    │       │       ├── quote v1.0.42
    │       │       ├── syn v2.0.111
    │       │       └── wasm-bindgen-shared v0.2.106
    │       └── wasm-bindgen-shared v0.2.106
    ├── libm v0.2.15
    ├── nu-ansi-term v0.50.3
    │   └── windows-sys v0.61.2
    ├── num-traits v0.2.19
    ├── oorandom v11.1.5
    ├── serde v1.0.228
    ├── serde_json v1.0.145
    ├── wasm-bindgen v0.2.106
    ├── wasm-bindgen-futures v0.4.56
    └── wasm-bindgen-test-macro v0.3.56 (proc-macro)
```

### 3.3 Duplicate Dependencies

```
$ cargo tree --duplicates

thiserror v1.0.69
└── edgevec v0.9.0

thiserror v2.0.17
└── cobs v0.3.0
    └── postcard v1.1.3
        └── edgevec v0.9.0

thiserror-impl v1.0.69 (proc-macro)
└── thiserror v1.0.69

thiserror-impl v2.0.17 (proc-macro)
└── thiserror v2.0.17

windows-sys v0.52.0
└── tempfile v3.10.0
    [dev-dependencies]
    └── edgevec v0.9.0

windows-sys v0.61.2
├── is-terminal v0.4.17
│   └── criterion v0.5.1
│       [dev-dependencies]
│       └── edgevec v0.9.0
├── nu-ansi-term v0.50.3
│   └── wasm-bindgen-test v0.3.56
│       [dev-dependencies]
│       └── edgevec v0.9.0
└── winapi-util v0.1.11
    ├── same-file v1.0.6
    │   └── walkdir v2.5.0
    │       └── criterion v0.5.1 (*)
    └── walkdir v2.5.0 (*)
```

**Duplicate crates: 3** (with 2 versions each)

| Crate | Versions | Cause | Impact |
|:------|:---------|:------|:-------|
| `thiserror` | v1.0.69, v2.0.17 | EdgeVec uses v1; `postcard`'s `cobs` dep uses v2 | Compile-time only (proc-macro). Minor binary size increase. |
| `thiserror-impl` | v1.0.69, v2.0.17 | Mirrors `thiserror` duplication | Compile-time only (proc-macro). |
| `windows-sys` | v0.52.0, v0.61.2 | `tempfile` uses v0.52; `criterion`/`wasm-bindgen-test` use v0.61 | Dev-dependency only. Does not affect production binary. |

---

## 4. Findings

### 4.1 Vulnerabilities

**0 known vulnerabilities as of 2026-03-08.**

The RustSec advisory database (946 advisories loaded) found no security vulnerabilities in any of EdgeVec's 143 crate dependencies.

### 4.2 Warnings

| ID | Crate | Version | Type | Severity | Status |
|:---|:------|:--------|:-----|:---------|:-------|
| RUSTSEC-2023-0089 | `atomic-polyfill` | 1.0.3 | Unmaintained | Low | Transitive dep via `postcard` -> `heapless`. No action required -- see Section 1 for details. |

### 4.3 Pinned Dependencies

Two dev-dependencies are pinned to exact versions:

| Crate | Pinned Version | Reason |
|:------|:---------------|:-------|
| `proptest` | =1.4.0 | Ensures reproducible property tests; `default-features = false` for `std`-only mode |
| `tempfile` | =3.10.0 | Ensures deterministic test behavior across CI environments |

Pinning is intentional and documented in `Cargo.toml`.

### 4.4 Supply Chain Notes

- All 14 production dependencies are from crates.io (the official Rust registry). No git dependencies, no path dependencies (except the workspace member `xtask`).
- All production crates are widely used in the Rust ecosystem (`serde`, `rand`, `thiserror`, etc.) with high download counts and active maintenance.
- The `pest` parser has a build-time dependency on `sha2` (for grammar checksum). This is compile-time only and does not affect the runtime binary.

---

## 5. Summary

| Metric | Count |
|:-------|------:|
| Direct production dependencies | 14 |
| Optional dependencies | 1 (`rayon`) |
| WASM-only dependencies | 8 |
| Dev dependencies | 5 |
| Total unique crates in dependency tree | 106 |
| Total crate entries in `Cargo.lock` | 143 |
| Known vulnerabilities (CVEs/RUSTSECs) | 0 |
| Warnings (unmaintained crates) | 1 |
| Duplicate crate versions | 3 |

### Recommendations

1. **No immediate action required.** The dependency tree is clean with 0 vulnerabilities.

2. **Monitor `postcard` for `heapless` v0.8 update.** This will eliminate the `atomic-polyfill` warning (RUSTSEC-2023-0089). The `postcard` v2.x line has already dropped this dependency -- consider upgrading when it stabilizes.

3. **Install `cargo-deny`.** Running `cargo install cargo-deny` and creating a `deny.toml` would add license compliance checks and the ability to ban specific crates. This is a low-priority enhancement.

4. **Consider unifying `thiserror` versions.** EdgeVec uses `thiserror` v1.0, while `cobs` (via `postcard`) uses v2.0. When EdgeVec migrates to `thiserror` v2.x, this duplication will resolve. Not urgent -- both versions are compile-time proc-macros.

5. **`tempfile` pin review.** The pinned version `=3.10.0` uses `windows-sys` v0.52.0 while other dev-deps use v0.61.2, causing duplication. Unpinning would allow `tempfile` to update and potentially unify the `windows-sys` version. Evaluate during next dependency review cycle.

---

**END OF DOCUMENT**
