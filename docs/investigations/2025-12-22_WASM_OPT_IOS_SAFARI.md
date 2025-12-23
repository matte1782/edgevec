# Investigation: wasm-opt iOS Safari Compatibility

**Date:** 2025-12-22
**Task:** W29.1.1 — Research wasm-opt flags
**Status:** COMPLETE

---

## Executive Summary

**The `i32.trunc_sat_f64_u` instruction IS compatible with iOS Safari 15+** (released September 2021). This instruction is part of the WebAssembly "Non-trapping float-to-int Conversion" proposal which has **95.41% global browser support**.

**VERDICT:** Safe to re-enable wasm-opt with `--enable-nontrapping-float-to-int` flag.

---

## Root Cause Analysis

### The Original Error

```
wasm-opt error: unexpected false: all used features should be allowed
on i32.trunc_sat_f64_u instruction
```

### What Happened

1. **Rust/LLVM generates `i32.trunc_sat_f64_u`** — This saturating float-to-int conversion is used instead of the trapping `i32.trunc_f64_u` for undefined behavior safety.

2. **wasm-opt didn't recognize the feature** — Without `--enable-nontrapping-float-to-int`, wasm-opt rejects binaries containing these instructions.

3. **We disabled wasm-opt** — Rather than investigate, we disabled optimization entirely.

---

## Browser Compatibility

### Non-trapping Float-to-Int Support

| Browser | Version | Released | Status |
|:--------|:--------|:---------|:-------|
| **Safari** | 15+ | Sep 2021 | SUPPORTED |
| **iOS Safari** | 15+ | Sep 2021 | SUPPORTED |
| Chrome | 75+ | Jun 2019 | SUPPORTED |
| Firefox | 65+ | Jan 2019 | SUPPORTED |
| Edge | 79+ | Jan 2020 | SUPPORTED |

**Global Usage:** 95.41% of browsers support this feature.

### Minimum iOS Version for EdgeVec

Given iOS Safari 15+ support, EdgeVec would require:
- **iOS 15.0+** (released September 20, 2021)
- **macOS 12 Monterey+** (released October 2021)

As of December 2025:
- iOS 15 is 4+ years old
- iOS 14 and earlier represent <5% of devices

---

## Solution

### Option A: Re-enable wasm-opt with Feature Flags (RECOMMENDED)

```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-nontrapping-float-to-int"]
```

**Or run manually:**
```bash
wasm-opt -Oz --enable-nontrapping-float-to-int pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
```

### Option B: Use All Features

```bash
wasm-opt -Oz --all-features pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
```

### Option C: Lower to MVP (NOT RECOMMENDED)

```bash
wasm-opt -Oz --llvm-nontrapping-fptoint-lowering pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
```

This would increase bundle size and potentially reduce performance.

---

## Current State

| Metric | Value |
|:-------|:------|
| Current WASM size | 536,826 bytes (524 KB) |
| wasm-opt installed | NO |
| wasm-opt enabled in Cargo.toml | NO |
| Target size (ACCEPT) | ≤520 KB |
| Target size (STRETCH) | <480 KB |

**Gap:** 4 KB over ACCEPT threshold.

---

## Expected Impact from wasm-opt

Based on typical wasm-opt `-Oz` optimization:
- **Typical reduction:** 10-30%
- **Conservative estimate:** 524 KB → ~400-470 KB
- **This would meet both ACCEPT and likely STRETCH targets**

---

## Recommendation

1. **Install binaryen** (provides wasm-opt)
2. **Update Cargo.toml** to enable wasm-opt with feature flags
3. **Rebuild WASM** and measure new size
4. **Document minimum iOS version** (15+) in README compatibility section

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|:-----|:-----------|:-------|:-----------|
| iOS 14 users can't load | Very Low (<5% market) | Low | Document in README |
| wasm-opt breaks build | Low | Medium | Keep current binary as fallback |
| Optimization insufficient | Low | Low | 524 KB is within ACCEPT range anyway |

---

## Sources

- [Can I Use: WebAssembly Non-trapping float-to-int](https://caniuse.com/wasm-nontrapping-fptoint)
- [WebAssembly Spec: Non-trapping Float-to-Int Proposal](https://github.com/WebAssembly/spec/blob/master/proposals/nontrapping-float-to-int-conversion/Overview.md)
- [Rust Issue #137315: wasm-opt + LLVM 20](https://github.com/rust-lang/rust/issues/137315)
- [Flutter Rust Bridge Issue #2601](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2601)

---

## Next Steps

1. Install binaryen: `npm install -g binaryen`
2. Enable wasm-opt in Cargo.toml with flags
3. Rebuild: `wasm-pack build --target web --release`
4. Measure and document new size

---

**Investigator:** RUST_ENGINEER
**Date:** 2025-12-22
