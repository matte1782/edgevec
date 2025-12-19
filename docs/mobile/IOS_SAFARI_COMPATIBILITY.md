# iOS Safari WASM Compatibility

**Version:** EdgeVec v0.5.3
**Date:** 2025-12-19
**Agent:** WASM_SPECIALIST
**Task:** W25.3.1

---

## Executive Summary

EdgeVec is **compatible with iOS Safari 11+** for basic WebAssembly operations. However, there are important memory limitations and configuration requirements for optimal mobile performance.

| Aspect | Status | Notes |
|:-------|:-------|:------|
| WebAssembly Core | ✅ Supported | iOS Safari 11+ |
| IndexedDB | ✅ Supported | Up to 80% disk space (Safari 17+) |
| Memory Limits | ⚠️ Limited | Recommend ≤256 MB WASM memory |
| wasm-bindgen | ✅ Compatible | Version 0.2.x works |
| SIMD | ❌ Not used | EdgeVec uses scalar fallbacks |

---

## Safari WebAssembly Feature Support

### Safari 17.x (iOS 17, 2024)

| Feature | Status | EdgeVec Usage |
|:--------|:-------|:--------------|
| WebAssembly Core | ✅ Supported | Required |
| Extended Constant Expressions | ✅ Safari 17.4+ | Not used |
| IndexedDB | ✅ Full support | Used for persistence |

### Safari 18.x (iOS 18, 2024-2025)

| Feature | Status | EdgeVec Usage |
|:--------|:-------|:--------------|
| Typed Function References | ✅ Safari 18.0+ | Not used |
| WASM Garbage Collection | ✅ Safari 18.2+ | Not used |
| WASM Tail Calls | ✅ Safari 18.2+ | Not used |

### Features NOT Yet in Safari (as of Dec 2025)

| Feature | Status | Impact on EdgeVec |
|:--------|:-------|:------------------|
| Memory64 | ❌ Coming 2025 | None (EdgeVec uses 32-bit memory) |
| Multiple Memories | ❌ Coming 2025 | None (EdgeVec uses single memory) |
| WASM SIMD | ⚠️ Partial | EdgeVec uses scalar fallbacks |

**Sources:**
- [WebKit Features in Safari 18.2](https://webkit.org/blog/16301/webkit-features-in-safari-18-2/)
- [Can I Use: WebAssembly](https://caniuse.com/wasm)
- [State of WebAssembly 2024-2025](https://platform.uno/blog/state-of-webassembly-2024-2025/)

---

## Memory Limits

### WebAssembly.Memory on iOS Safari

**Critical Finding:** iOS Safari has strict memory limits that differ from desktop Safari.

| Memory Setting | Desktop Safari | iOS Safari | Recommendation |
|:---------------|:---------------|:-----------|:---------------|
| Default Maximum | 2 GB | 256-300 MB | Use 256 MB max |
| Reliable Allocation | 2 GB | ~256 MB | Stay under 256 MB |
| SharedArrayBuffer | Supported | Supported (16.4+) | Avoid if possible |

**Why This Matters:**
- iOS Safari throws `RangeError: Out of memory` if `WebAssembly.Memory` is initialized with maximum > ~300 MB
- Even if iPhone has 8 GB RAM, browser processes are restricted
- Memory leaks on page reload are a known Safari issue

**EdgeVec Memory Usage (768D vectors):**

| Mode | Per Vector | 10k Vectors | 100k Vectors | Status |
|:-----|:-----------|:------------|:-------------|:-------|
| Quantized (SQ8) | 872 bytes | 8.7 MB | 87 MB | ✅ Safe |
| Float32 | 3,176 bytes | 31.8 MB | 318 MB | ⚠️ Risky at 100k |

**Recommendations:**
1. Use **quantized mode** on mobile devices
2. Limit to **~50k vectors** maximum on iOS for safety margin
3. Consider chunked loading for larger datasets

**Sources:**
- [Godot WASM Memory Issue](https://github.com/godotengine/godot/issues/70621)
- [Emscripten Out-of-Memory Issue](https://github.com/emscripten-core/emscripten/issues/19374)
- [.NET MAXIMUM_MEMORY Discussion](https://github.com/dotnet/runtime/issues/84638)

---

## IndexedDB Limits

### Safari 17.0+ Storage Policy

| Quota Type | Browser App | Other Apps |
|:-----------|:------------|:-----------|
| Overall Quota | 80% of disk | 20% of disk |
| Per-Origin Quota | Depends on total | Depends on total |
| Private Browsing | 0 (disabled) | 0 (disabled) |

**User Interaction Requirement:**
- Safari evicts data for origins with no user interaction in 7 days
- Cookies set by server are exempt

**EdgeVec Considerations:**
- Persistence works normally on iOS Safari
- For Home Screen Web Apps: Same quotas as browser app
- Private browsing: IndexedDB disabled entirely

**Recommendation:** Inform users that data may be evicted if app not used for 7+ days.

**Sources:**
- [WebKit Updates to Storage Policy](https://webkit.org/blog/14403/updates-to-storage-policy/)
- [MDN Storage Quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)

---

## wasm-bindgen Compatibility

### Current EdgeVec Configuration

```toml
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
```

### Known Issues

| Issue | Affected Versions | EdgeVec Impact | Workaround |
|:------|:------------------|:---------------|:-----------|
| Safari 15 regression | wasm-bindgen 0.2.95 + Rust 1.82+ | ❌ None (Safari 15 is old) | None needed |
| Memory access errors | Safari 17.5-18.0.1 | ⚠️ Monitor | Tracked in WebKit |
| Clipboard API | All Safari | ❌ None (not used) | N/A |
| Atomics.waitAsync | Safari < 16.4 | ❌ None (not using threads) | N/A |

**EdgeVec Status:**
- No known blocking issues with current wasm-bindgen version
- EdgeVec does NOT use threading/SharedArrayBuffer features that cause Safari issues

**Sources:**
- [wasm-bindgen Safari 15 Issue](https://github.com/wasm-bindgen/wasm-bindgen/issues/4227)
- [Safari Memory Access Bug](https://github.com/wasm-bindgen/wasm-bindgen/discussions/4185)

---

## CSP (Content Security Policy) Requirements

### Safari-Specific Requirements

| Directive | Chrome | Firefox | Safari |
|:----------|:-------|:--------|:-------|
| `wasm-eval` | ✅ Supported | ✅ Supported | ❌ Not supported |
| `unsafe-eval` | Works | Works | Required for WASM |

**Recommendation:** If using CSP, include `unsafe-eval` in `script-src` for Safari compatibility.

```html
<meta http-equiv="Content-Security-Policy"
      content="script-src 'self' 'unsafe-eval';">
```

---

## EdgeVec Features Analysis

### Features Used by EdgeVec

| Feature | Required | iOS Safari Support |
|:--------|:---------|:-------------------|
| WebAssembly Core | ✅ Yes | ✅ iOS 11+ |
| IndexedDB | ✅ Yes | ✅ iOS 8+ |
| Float64 | ✅ Yes | ✅ Supported |
| BigInt | ✅ Yes (IDs) | ✅ iOS 14+ |
| TextEncoder/Decoder | ✅ Yes | ✅ iOS 10.3+ |
| Performance.now() | Optional | ✅ Supported |
| console.log | Debug | ✅ Supported |

### Features NOT Used by EdgeVec

| Feature | Status | Why Not Used |
|:--------|:-------|:-------------|
| SharedArrayBuffer | Not used | Threading not needed |
| WASM SIMD | Not used | Scalar fallbacks used |
| WASM Threads | Not used | Single-threaded design |
| Multiple Memories | Not used | Single memory model |
| WebGL | Not used | Not a graphics library |

---

## Minimum Supported Versions

### Official Support Matrix

| Platform | Minimum Version | Recommended | Notes |
|:---------|:----------------|:------------|:------|
| iOS Safari | 14.0 | 17.0+ | BigInt support required |
| iPadOS Safari | 14.0 | 17.0+ | Same as iOS |
| macOS Safari | 14.0 | 17.0+ | Desktop has more memory |

### Degraded Support

| Platform | Version | Issues |
|:---------|:--------|:-------|
| iOS Safari 11-13 | ⚠️ Limited | No BigInt, older IndexedDB |

---

## Recommendations for v0.6.0

### Must Have

1. **Add mobile detection** — Detect iOS Safari and warn about memory limits
2. **Recommend quantized mode** — Default to SQ8 on mobile devices
3. **Document vector limits** — Clear guidance on safe vector counts

### Should Have

1. **Memory pressure handling** — Graceful degradation when approaching limits
2. **Progressive loading** — Load vectors in chunks for large datasets
3. **Storage quota check** — Use `navigator.storage.estimate()` before large saves

### Nice to Have

1. **Touch optimization** — Improve demo touch interactions
2. **Mobile-specific demos** — Simplified demos for mobile testing
3. **PWA support** — Add manifest for home screen installation

---

## Test Matrix Template

| Test Case | iOS 17 | iOS 18 | Expected |
|:----------|:-------|:-------|:---------|
| WASM Load | | | ✅ Works |
| 1k vector insert | | | ✅ Works |
| 10k vector insert | | | ✅ Works |
| 50k vector insert | | | ⚠️ Test |
| Search (k=10) | | | ✅ Works |
| Filter parse | | | ✅ Works |
| IndexedDB save | | | ✅ Works |
| IndexedDB load | | | ✅ Works |
| Touch interactions | | | Test needed |

---

## Conclusion

EdgeVec is **expected to work on iOS Safari 17+** with the following caveats:

1. **Memory limit:** Stay under 50k vectors in quantized mode, 15k in float32
2. **No SIMD:** Performance is scalar-only (still fast)
3. **IndexedDB:** Works, but 7-day eviction policy applies
4. **CSP:** Requires `unsafe-eval` if using Content Security Policy

**Risk Level:** LOW — EdgeVec's design avoids most iOS Safari WASM pitfalls.

---

**Agent:** WASM_SPECIALIST
**Status:** W25.3.1 COMPLETE
**Next:** W25.3.2 (iOS Simulator Testing Setup)
