# WASM Optimization Research (W29.1.1)

**Date:** 2025-12-22
**Status:** COMPLETE
**Agent:** RUST_ENGINEER

---

## wasm-opt Installation

**Installed Version:** wasm-opt version 125

**Installation Method:** Previously installed (binaryen available in PATH)

---

## Optimization Flags Comparison

| Flag | Description | Size Impact | Speed Impact | Recommendation |
|:-----|:------------|:------------|:-------------|:---------------|
| `-O0` | No optimization | Baseline | Baseline | Debug only |
| `-O1` | Basic optimization | ~5-10% smaller | Slightly faster | Development |
| `-O2` | Standard optimization | ~10-20% smaller | Faster | Production |
| `-O3` | Aggressive optimization | ~15-25% smaller | Maximum speed | Speed-critical |
| `-Os` | Optimize for size (balanced) | ~20-30% smaller | Good speed | Size-conscious |
| **`-Oz`** | **Optimize for size (aggressive)** | **~25-40% smaller** | Acceptable speed | **RECOMMENDED for WASM bundles** |
| `-O4` | Like -O3 + loop opts | Similar to -O3 | Maximum | Rarely needed |

---

## Strip Flags

| Flag | Description | Size Impact | Notes |
|:-----|:------------|:------------|:------|
| `--strip-debug` | Remove debug info | ~5-15% smaller | **Always safe for production** |
| `--strip-producers` | Remove producers metadata | ~1-2 KB | Safe, removes build info |
| `--strip-dwarf` | Remove DWARF debug info | Varies | May already be stripped |

---

## Feature Flags (EdgeVec-relevant)

| Flag | Description | Why EdgeVec Needs It |
|:-----|:------------|:---------------------|
| `--enable-simd` | Enable SIMD support | Required for fast distance calculations |
| `--enable-nontrapping-float-to-int` | Enable non-trapping float-to-int | **Required for iOS Safari** (previous issue) |
| `--enable-bulk-memory` | Enable bulk memory ops | Performance optimization |
| `--enable-mutable-globals` | Enable mutable globals | Required by wasm-bindgen |

---

## Recommended Optimization Pipeline for EdgeVec

### Option A: Conservative (Maximum Compatibility)
```bash
wasm-opt -Oz \
  --strip-debug \
  --strip-producers \
  --enable-nontrapping-float-to-int \
  pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
```

### Option B: Aggressive (Maximum Size Reduction)
```bash
wasm-opt -Oz \
  --strip-debug \
  --strip-producers \
  --strip-dwarf \
  --enable-nontrapping-float-to-int \
  --enable-simd \
  --enable-bulk-memory \
  pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
```

**Recommended for EdgeVec:** Option A (conservative) to maintain iOS Safari compatibility.

---

## Expected Size Reductions

Based on typical Rust+wasm-bindgen bundles:

| Baseline | After -Oz | After strip-debug | Combined |
|:---------|:----------|:------------------|:---------|
| 536 KB | ~430-480 KB (~10-20%) | ~5-10% additional | **~400-450 KB** |

**Target:** <512 KB (500 KB)
**Expected Post-Optimization:** ~420-480 KB

---

## Notes

1. **wasm-opt was previously disabled** in Cargo.toml due to iOS Safari compatibility issues with `i32.trunc_sat_f64_u` instruction
2. The `--enable-nontrapping-float-to-int` flag should resolve this issue
3. After optimization, browser testing is MANDATORY before release

---

## Deliverables

- [x] `wasm-opt --version` outputs version number: **v125**
- [x] Research document with flag comparison table: **THIS FILE**

---

*Agent: RUST_ENGINEER*
*Status: [APPROVED]*
*Date: 2025-12-22*
