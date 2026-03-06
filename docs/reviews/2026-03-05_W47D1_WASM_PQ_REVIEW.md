# HOSTILE_REVIEWER: Rejection -- W47 D1T4/D1T5 (WASM PQ Exports + Tests)

**Date:** 2026-03-05
**Artifact:** D1T4 (WASM PQ Exports) + D1T5 (PQ WASM Tests)
**Author:** RUST_ENGINEER / WASM_SPECIALIST
**Status:** REJECTED

---

## Summary

Review of three WASM PQ binding functions (`trainPq`, `encodePq`, `pqSearch`) in `src/wasm/mod.rs` lines 4335-4618, the new `PqCode::from_codes()` constructor in `src/quantization/product.rs` lines 121-135, and three integration tests (`test_wasm_pq_train_returns_handle`, `test_wasm_pq_encode_returns_codes`, `test_wasm_pq_search_returns_results`) in `src/quantization/product.rs` lines 1626-1788.

---

## Findings

### Critical Issues: 1

- [C1] **`pq_search` accepts arbitrary code bytes without validating against ksub, enabling OOB panic in `compute_distance`**
  - Description: `pq_search` (line 4574) calls `PqCode::from_codes(codes[i * m..(i + 1) * m].to_vec())` on raw bytes received from JavaScript. `from_codes` performs zero validation (line 133). If any byte value >= ksub, `compute_distance` (line 216) computes `m * self.ksub + centroid_idx as usize` which will index out of bounds on `self.table` (size `M * ksub`). Example: ksub=16, centroid_idx=255 yields index `m*16 + 255`, which exceeds table size 128. This is a **panic in library code** reachable from the public WASM API, exploitable by any JavaScript caller passing a malformed `Uint8Array`.
  - Evidence: `src/quantization/product.rs` line 216 -- `self.table[m * self.ksub + centroid_idx as usize]` with no bounds check. `from_codes` at line 133 accepts any `Vec<u8>`. `pq_search` at line 4574 passes raw JS bytes directly.
  - Impact: Violates "No panics in library code" (CLAUDE.md Section 3.1). A JavaScript caller can crash the entire WASM instance with a single malformed byte. This is a denial-of-service vector for any application using the WASM PQ API.
  - Required Action: Either (a) validate all code bytes < ksub inside `pq_search` before constructing `PqCode`, returning `Err(JsValue)` on violation, or (b) add validation inside `from_codes` itself (returning `Result`), or (c) add bounds checking inside `compute_distance` (replacing the `assert_eq!` at line 206 and the unchecked indexing at line 216 with proper error returns). Option (a) is the minimum fix for this review scope.

### Major Issues: 2

- [M1] **`compute_distance` uses `assert_eq!` (panic) instead of returning `Result`**
  - Description: `compute_distance` at line 206 uses `assert_eq!` to check that `code.codes.len() == self.num_subquantizers`. This is a panic in library code. While not directly introduced by this PR, it is on the call path of the new WASM `pq_search` function and is therefore part of the attack surface being reviewed.
  - Evidence: `src/quantization/product.rs` line 206-212.
  - Required Action: Convert `assert_eq!` to a proper error return, or document this as a tracked pre-existing issue with an issue reference. At minimum, `pq_search` must validate code length before calling `compute_distance` to prevent this panic from being reachable via WASM.

- [M2] **No tests for error/adversarial cases in the WASM PQ path**
  - Description: All three tests (`test_wasm_pq_train_returns_handle`, `test_wasm_pq_encode_returns_codes`, `test_wasm_pq_search_returns_results`) test only the happy path. D1T5 acceptance criteria state "Tests cover: train, encode, search pipeline" but do not exercise: (1) NaN input rejection, (2) wrong-dimension vector rejection, (3) codes with values >= ksub, (4) `n_codes` mismatch with actual codes length, (5) k > n_codes behavior, (6) empty codes array. For a public WASM API, error path coverage is mandatory per Section 3.1.
  - Evidence: `src/quantization/product.rs` lines 1626-1788 contain zero `#[should_panic]` or error-case assertions.
  - Required Action: Add at minimum tests for NaN rejection on train/encode/search, dimension mismatch on encode/search, and invalid code bytes on search.

### Minor Issues: 4

- [m1] **`pq_search` does not document behavior when `n_codes=0`**
  - Description: If `n_codes=0`, the function returns an empty array (via `scan_topk` early return). This is reasonable behavior but is not documented in the doc comment's "Errors" section. The doc explicitly lists errors for k=0 but is silent on n_codes=0.
  - Evidence: `src/wasm/mod.rs` lines 4512-4517 list error conditions but omit n_codes=0.

- [m2] **`from_codes` safety contract is doc-only with no runtime enforcement**
  - Description: `from_codes` doc comment (lines 127-131) states "Callers must ensure that every value in `codes` is < Ksub" but the function is `pub` and performs no validation. The safety contract is unenforceable. While not `unsafe` in the Rust sense, the naming `# Safety Contract` implies a stronger guarantee than exists.
  - Evidence: `src/quantization/product.rs` lines 127-135.

- [m3] **Doc comment example at line 4352 shows `codebook.encodePq(vector)` but the JS name for the method is also `encodePq` -- consistent, but the top-level example mixes free function and method call styles without explicit annotation**
  - Description: The example at lines 4347-4356 shows `trainPq` (free function), then `codebook.encodePq(vector)` (method), then `codebook.pqSearch(...)` (method). While technically correct, new users may not realize `trainPq` is a module-level export while `encodePq`/`pqSearch` are methods on the returned handle. A brief comment in the example would clarify.

- [m4] **`results.len() as u32` cast at line 4582 could truncate if results exceed u32::MAX**
  - Description: `Array::new_with_length(results.len() as u32)` truncates silently if results.len() > u32::MAX. In practice this is unreachable (would require >4B vectors), but the `#[allow(clippy::cast_possible_truncation)]` at line 4531 suppresses the warning for the entire function rather than scoping it to the specific casts that need it.

---

## Verdict

```
+---------------------------------------------------------------------+
|   HOSTILE_REVIEWER: REJECT                                          |
|                                                                     |
|   Artifact: W47 D1T4 (WASM PQ Exports) + D1T5 (PQ WASM Tests)     |
|   Author: RUST_ENGINEER / WASM_SPECIALIST                           |
|                                                                     |
|   Critical Issues: 1                                                |
|   Major Issues: 2                                                   |
|   Minor Issues: 4                                                   |
|                                                                     |
|   Disposition:                                                      |
|   - REJECT: C1 is a panic reachable from JS. Cannot ship.          |
|   - Required actions before resubmission listed below.              |
|                                                                     |
+---------------------------------------------------------------------+
```

**REJECTED**

This artifact fails 1 critical and 2 major quality gates and cannot proceed.

---

## Required Actions Before Resubmission

1. [ ] **[C1]** Add validation in `pq_search` that ALL code bytes are < `self.codebook.ksub()` before constructing `PqCode` objects. Return `Err(JsValue::from_str("PQ error: code byte N at index I >= ksub K"))` on violation.
2. [ ] **[M1]** Either (a) convert `compute_distance` `assert_eq!` to a proper error return, or (b) add a code-length validation in `pq_search` before calling `scan_topk` AND file an issue to track the pre-existing `assert_eq!` in `compute_distance`.
3. [ ] **[M2]** Add error-path tests: at minimum NaN rejection (train, encode, search), dimension mismatch (encode, search), and invalid code bytes >= ksub (search). These tests must assert the correct error message is returned.

---

## Resubmission Process

1. Address ALL critical issues
2. Address ALL major issues
3. Update artifact with `[REVISED]` tag
4. Resubmit for hostile review via `/review D1T4_D1T5`

---

*Reviewed by: HOSTILE_REVIEWER*
*Date: 2026-03-05*
*Verdict: REJECTED*
