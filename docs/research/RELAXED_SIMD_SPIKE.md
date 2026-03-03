# WASM Relaxed SIMD Research Spike

**Date:** 2026-03-19
**Author:** META_ARCHITECT + WASM_SPECIALIST
**Status:** [APPROVED]
**Sprint:** W44 (Task IDs: W44.3a-W44.3e)
**Time Budget:** 4h (1-day spike)

---

## Executive Summary

This spike investigated the feasibility of adding WASM Relaxed SIMD support to EdgeVec's distance computation hot paths. Relaxed SIMD (part of WebAssembly 3.0, finalized September 2025) provides fused multiply-add (FMA) and integer dot product instructions that can accelerate vector similarity computations.

**VERDICT: NO-GO for v0.10.0**

Safari still gates Relaxed SIMD behind a flag with no announced timeline for default enablement. While Chrome (113+), Edge (113+), and Firefox (145+) all ship it by default, Safari's gap means we cannot rely on Relaxed SIMD without shipping dual WASM bundles and adding JavaScript-level feature detection. The estimated 1.5x speedup (primarily on ARM hardware) does not justify the build complexity and bundle size doubling for v0.10.0. However, the integration path via `simd_dispatch!` is clean and well-understood — revisit when Safari ships default enablement.

---

## 1. Browser Support Matrix

| Browser | Version | Status | Notes |
|:--------|:--------|:-------|:------|
| **Chrome** | 114+ | Enabled by default | Shipped June 2023 |
| **Edge** | 114+ | Enabled by default | Chromium-based, inherits V8 |
| **Firefox** | 145+ | Enabled by default | Flag removed November 2025 |
| **Safari** | All | **Behind a flag** | Available behind flag since ~2024; no announced timeline for default |

### Critical Finding: Safari

Safari remains the blocker. As of March 2026, Relaxed SIMD is behind a developer flag in all Safari versions (macOS, iOS, iPadOS, visionOS). There is no announced WebKit timeline for default enablement. This is the single deciding factor for the NO-GO.

**Sources:**
- [WebAssembly Feature Status — webassembly.org](https://webassembly.org/features/)
- [Relaxed-width SIMD — webstatus.dev](https://webstatus.dev/features/wasm-simd-relaxed)
- [State of WebAssembly 2025/2026 — platform.uno](https://platform.uno/blog/the-state-of-webassembly-2025-2026/)

---

## 2. Runtime Feature Detection

### The Core Problem

WebAssembly has no introspection API. A WASM module cannot query its own supported features at runtime. A binary compiled with `+relaxed-simd` will **fail to instantiate** on a runtime that does not support it.

### Solution: `wasm-feature-detect`

The [`wasm-feature-detect`](https://github.com/GoogleChromeLabs/wasm-feature-detect) package (Google Chrome Labs) provides async boolean detection:

```javascript
import { relaxedSimd } from "wasm-feature-detect";

if (await relaxedSimd()) {
  const { default: init } = await import("./edgevec_relaxed_simd.js");
  await init();
} else {
  const { default: init } = await import("./edgevec_simd128.js");
  await init();
}
```

**Consequence:** This requires shipping **two separate WASM builds** — one with `+relaxed-simd` and one without. This approximately doubles the total WASM bundle size (~477KB x 2 = ~954KB) unless lazy-loaded.

### EdgeVec Integration Point

EdgeVec's `pkg/langchain/src/init.ts` already handles WASM initialization. A Relaxed SIMD path would add detection before `init()`, routing to the appropriate bundle.

---

## 3. Hot Path Analysis

### EdgeVec Functions That Benefit from Relaxed FMA

The primary beneficiary is `f32x4.relaxed_madd` (fused multiply-add), which replaces the two-instruction sequence `f32x4.mul` + `f32x4.add` with a single instruction.

| Hot Path | Current Instructions | With Relaxed SIMD | Instruction Reduction |
|:---------|:--------------------|:-------------------|:---------------------|
| **Dot product** (cosine similarity) | `f32x4.mul` + `f32x4.add` per 4 elements | `f32x4.relaxed_madd` per 4 elements | 2 → 1 (50%) |
| **L2 distance** | `f32x4.sub` + `f32x4.mul` + `f32x4.add` per 4 elements | `f32x4.sub` + `f32x4.relaxed_madd` per 4 elements | 3 → 2 (33%) |
| **Hamming distance** | Popcount-based (integer ops) | No relaxed equivalent | 0% |
| **Binary quantization** | Integer ops (XOR, popcount) | No benefit | 0% |

### Additional Relaxed Instructions

| Instruction | EdgeVec Use Case | Benefit |
|:-----------|:-----------------|:--------|
| `i16x8.relaxed_dot_i8x16_i7x16_s` | Future int8 quantization | High (direct hardware dot product) |
| `f32x4.relaxed_min/max` | Top-K selection | Minor (avoids NaN masking overhead) |
| `i32x4.relaxed_laneselect` | Conditional blending | Minor |

### Key Insight

The largest gains appear on **ARM hardware** (Apple Silicon, Android, ARM Chromebooks) where standard SIMD128 lacks native FMA and must emulate it. On x86 with AVX2+FMA, standard SIMD128 already maps well to hardware FMA instructions, so the marginal gain is smaller.

---

## 4. Estimated Speedup

### Published Benchmarks

| Source | Workload | Speedup vs SIMD128 |
|:-------|:---------|:-------------------|
| Chrome DevRel (Google I/O 2024) | General Relaxed SIMD | 1.5x-3x |
| Chrome DevRel | Integer dot product | ~8x vs non-SIMD scalar |
| Tesseract.js (OCR) | OCR inference pipeline | ~1.6x |

### EdgeVec-Specific Estimate

For 768-D dot product (the primary hot path):

| Platform | Standard SIMD128 | With Relaxed FMA | Estimated Speedup |
|:---------|:-----------------|:-----------------|:-----------------|
| x86_64 (Intel/AMD) | Already maps to FMA | Marginal improvement | **1.1-1.2x** |
| ARM (Apple Silicon) | No native FMA mapping | Direct `FMLA` instruction | **1.5-2.0x** |
| ARM (Android) | No native FMA mapping | Direct `VFMA` instruction | **1.5-2.0x** |

**For EdgeVec's overall search (HNSW traversal + distance):** The SIMD distance computation is ~30-50% of total search time (rest is graph traversal, memory access). So a 1.5x distance speedup translates to ~1.15-1.25x total search speedup on ARM, and negligible on x86.

---

## 5. Non-Determinism Analysis

### What "Relaxed" Means

Each hardware+runtime pair globally chooses a fixed projection from allowed results. Within a single browser on a single machine, results are 100% deterministic. Non-determinism only appears when comparing results across different hardware.

For `f32x4.relaxed_madd`, the allowed results are:
- **Option A:** Single-rounded FMA: `round(a*b + c)`
- **Option B:** Double-rounded: `round(round(a*b) + c)`

Difference: at most 1 ULP of f32 (~1.2e-7 relative error).

### Impact on EdgeVec

| Concern | Severity | Mitigation |
|:--------|:---------|:-----------|
| Score differences across platforms | Very low (<1e-5 absolute for 768-D) | Below ranking threshold; accept |
| Test determinism | Medium | Use epsilon-relative comparisons (already in place) |
| Persistence correctness | None | Vectors stored verbatim; scores computed at query time |
| Ranking order changes | Very low | Only affects ties within 1e-5 score distance |

**Conclusion:** Non-determinism is not a practical concern for EdgeVec.

---

## 6. Integration Path via `simd_dispatch!`

### Current Dispatch Architecture

EdgeVec's `simd_dispatch!` macro uses compile-time `cfg_if!` to select between branches:

```
1. wasm_simd    → cfg(all(target_arch = "wasm32", target_feature = "simd128"))
2. avx2         → cfg(all(target_arch = "x86_64", target_feature = "avx2"))
3. neon         → cfg(target_arch = "aarch64")
4. fallback     → all other platforms
```

### Proposed Extension

Add `wasm_relaxed_simd` as the highest-priority WASM branch:

```
1. wasm_relaxed_simd → cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))
2. wasm_simd         → cfg(all(target_arch = "wasm32", target_feature = "simd128"))
3. avx2              → cfg(all(target_arch = "x86_64", target_feature = "avx2"))
4. neon              → cfg(target_arch = "aarch64")
5. fallback          → all other platforms
```

### Implementation Sketch

```rust
// src/metric/simd/wasm_relaxed.rs
#[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
pub mod wasm_relaxed {
    use std::arch::wasm32::*;

    #[target_feature(enable = "relaxed-simd")]
    pub unsafe fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        let mut acc = f32x4_splat(0.0);
        let mut i = 0;
        while i + 4 <= a.len() {
            let va = v128_load(a.as_ptr().add(i) as *const v128);
            let vb = v128_load(b.as_ptr().add(i) as *const v128);
            acc = f32x4_relaxed_madd(va, vb, acc);  // FMA: acc += va * vb
            i += 4;
        }
        // Horizontal sum + scalar tail
        let arr: [f32; 4] = std::mem::transmute(acc);
        let mut sum = arr[0] + arr[1] + arr[2] + arr[3];
        while i < a.len() { sum += a[i] * b[i]; i += 1; }
        sum
    }
}
```

### Rust Toolchain Support

- **Stabilized in Rust 1.82.0** (October 2024) via [PR #117468](https://github.com/rust-lang/rust/pull/117468)
- All intrinsics available in `std::arch::wasm32` with `#[target_feature(enable = "relaxed-simd")]`
- LLVM 15+ backend support (Rust 1.82 ships LLVM 19)
- No nightly required

### Build Configuration

```bash
# Standard build (existing)
RUSTFLAGS="-C target-feature=+simd128" cargo build --target wasm32-unknown-unknown --release

# Relaxed SIMD build (new, separate binary)
RUSTFLAGS="-C target-feature=+simd128,+relaxed-simd" cargo build --target wasm32-unknown-unknown --release
```

---

## 7. GO/NO-GO Decision

### Criteria Evaluation

| Criterion | Required | Actual | Pass/Fail |
|:----------|:---------|:-------|:----------|
| Safari enables Relaxed SIMD by default, OR reliable runtime detection exists | Default-on OR detection | Detection exists but requires dual bundles; Safari behind flag | **FAIL** |
| Estimated speedup >1.3x for at least one hot path | >1.3x | 1.5-2.0x on ARM (dot product) | **PASS** |
| Can use existing `simd_dispatch!` macro pattern | Clean integration | Yes, new branch priority slot | **PASS** |
| No regression risk for browsers without support | No regression | Dual bundles + feature detection handles this | **PASS** |

### VERDICT: NO-GO for v0.10.0

**One criterion fails (Safari).** The speedup is real on ARM hardware, but the implementation cost is significant:

1. **Dual WASM bundles** (~954KB total vs. current 477KB)
2. **JavaScript-level feature detection** (adds `wasm-feature-detect` dependency)
3. **Modified init path** (dynamic import of correct bundle)
4. **CI/CD changes** (build and test two WASM targets)
5. **Safari users get no benefit** (fall back to existing SIMD128)

This is not worth the complexity for v0.10.0 when EdgeVec is already meeting performance targets.

### When to Revisit

Relaxed SIMD becomes a **GO** when:
- **Safari ships Relaxed SIMD by default** (the primary trigger)
- **OR** EdgeVec adds a build system that already produces multiple WASM variants (e.g., for future WASM threads or memory64)
- **OR** int8 quantization is added (the `i16x8.relaxed_dot_i8x16_i7x16_s` instruction provides very high value for int8 dot products)

---

## 8. Closure Statement

**Relaxed SIMD research is closed for v0.10.0.** The NO-GO decision is based on Safari's lack of default enablement, which forces dual bundles and detection complexity that isn't justified by the 1.5x ARM speedup alone.

The integration path is clean and well-documented. When Safari ships default Relaxed SIMD support, implementation should take ~4-6h using the `simd_dispatch!` extension pattern documented above. If int8 quantization is added to EdgeVec, Relaxed SIMD should be re-evaluated immediately for the integer dot product instructions.

---

## Sources

- [WebAssembly Feature Status — webassembly.org](https://webassembly.org/features/)
- [Wasm 3.0 Completed — webassembly.org](https://webassembly.org/news/2025-09-17-wasm-3.0/)
- [Relaxed SIMD Proposal — GitHub](https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md)
- [Chrome for Developers — WebAssembly and WebGPU (I/O 2024)](https://developer.chrome.com/blog/io24-webassembly-webgpu-1)
- [Rust Tracking Issue #111196: Relaxed SIMD](https://github.com/rust-lang/rust/issues/111196)
- [Rust PR #117468: Stabilize Relaxed SIMD](https://github.com/rust-lang/rust/pull/117468)
- [wasm-feature-detect — npm](https://www.npmjs.com/package/wasm-feature-detect)
- [WebAssembly Feature Detection — web.dev](https://web.dev/articles/webassembly-feature-detection)
- [State of WebAssembly 2025/2026 — platform.uno](https://platform.uno/blog/the-state-of-webassembly-2025-2026/)
- [f32x4_relaxed_madd — Rust docs](https://doc.rust-lang.org/nightly/core/arch/wasm32/fn.f32x4_relaxed_madd.html)

---

**END OF RELAXED SIMD SPIKE**
