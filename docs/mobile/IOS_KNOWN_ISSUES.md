# iOS Safari Known Issues

**Version:** EdgeVec v0.5.3
**Date:** 2025-12-19
**Agent:** WASM_SPECIALIST
**Task:** W25.3.4

---

## Issue Summary

| Category | Count | Severity |
|:---------|:------|:---------|
| Memory Limits | 2 | High |
| IndexedDB | 2 | Medium |
| wasm-bindgen | 1 | Low |
| Touch/UI | 1 | Low |
| **Total** | **6** | - |

---

## High Severity Issues

### I1: WebAssembly Memory Limit (256 MB)

**Severity:** HIGH
**Affected:** iOS Safari (all versions)
**Status:** Known limitation, no workaround

**Description:**
iOS Safari limits WebAssembly.Memory to approximately 256-300 MB maximum. Exceeding this causes `RangeError: Out of memory` at WASM module initialization.

**Impact on EdgeVec:**
- Float32 mode: Max ~15k vectors (768D) safely
- Quantized mode: Max ~50k vectors (768D) safely
- Exceeding limits crashes the page

**Evidence:**
- [Godot Issue #70621](https://github.com/godotengine/godot/issues/70621)
- [Emscripten Issue #19374](https://github.com/emscripten-core/emscripten/issues/19374)

**Workaround:**
```javascript
// Detect iOS and warn users
const isIOS = /iPad|iPhone|iPod/.test(navigator.userAgent);
if (isIOS && vectorCount > 50000) {
    console.warn('EdgeVec: Large datasets may exceed iOS memory limits');
}
```

**v0.6.0 Action:**
- Add mobile detection in EdgeVec constructor
- Recommend quantized mode on mobile
- Document vector count limits prominently

---

### I2: Memory Leak on Page Reload

**Severity:** HIGH
**Affected:** iOS/macOS Safari (SharedArrayBuffer cases)
**Status:** Known WebKit bug

**Description:**
When using SharedArrayBuffer (for threading), Safari may not deallocate WASM memory on page reload, leading to eventual crash.

**Impact on EdgeVec:**
- EdgeVec does NOT use SharedArrayBuffer
- EdgeVec does NOT use threading
- **This issue should NOT affect EdgeVec**

**Evidence:**
- [Emscripten Issue #19374](https://github.com/emscripten-core/emscripten/issues/19374)

**Status:** No action needed (EdgeVec not affected)

---

## Medium Severity Issues

### I3: IndexedDB 7-Day Eviction

**Severity:** MEDIUM
**Affected:** Safari 17+ with ITP enabled
**Status:** By design (Safari privacy feature)

**Description:**
Safari's Intelligent Tracking Prevention (ITP) may delete IndexedDB data for origins with no user interaction in 7 days.

**Impact on EdgeVec:**
- Saved databases may be deleted if user doesn't visit site
- No warning before deletion
- Server-set cookies are exempt

**Evidence:**
- [WebKit Storage Policy](https://webkit.org/blog/14403/updates-to-storage-policy/)

**Workaround:**
```javascript
// Inform users about data retention
if (/Safari/.test(navigator.userAgent) && !/Chrome/.test(navigator.userAgent)) {
    console.info('EdgeVec: Data is stored locally. Visit periodically to prevent deletion.');
}
```

**v0.6.0 Action:**
- Add documentation about Safari data eviction
- Consider export/import functionality for backup

---

### I4: Private Browsing Mode

**Severity:** MEDIUM
**Affected:** Safari Private Browsing
**Status:** By design

**Description:**
In Private Browsing mode, Safari disables IndexedDB entirely (quota = 0).

**Impact on EdgeVec:**
- `db.save()` will fail
- `EdgeVec.load()` will fail
- Vector operations still work (in-memory only)

**Evidence:**
- [RxDB IndexedDB Limits](https://rxdb.info/articles/indexeddb-max-storage-limit.html)

**Workaround:**
```javascript
// Detect private browsing via quota check
async function isPrivateBrowsing() {
    try {
        const estimate = await navigator.storage.estimate();
        return estimate.quota < 1000000; // < 1MB suggests private mode
    } catch {
        return true; // Assume private if API unavailable
    }
}
```

**v0.6.0 Action:**
- Add private browsing detection
- Show warning when persistence unavailable

---

## Low Severity Issues

### I5: wasm-bindgen Safari 15 Compatibility

**Severity:** LOW
**Affected:** Safari 15 and below
**Status:** Edge case, older browsers

**Description:**
wasm-bindgen 0.2.95+ with Rust 1.82+ may break Safari 15 due to automatic enabling of reference-types and multivalue features.

**Impact on EdgeVec:**
- EdgeVec targets Safari 14+ minimum
- Safari 15 is from 2021, low usage
- iOS 15 users mostly upgraded

**Evidence:**
- [wasm-bindgen Issue #4227](https://github.com/wasm-bindgen/wasm-bindgen/issues/4227)

**Workaround:**
- Pin wasm-bindgen version if needed
- Test on Safari 15 specifically if users report issues

**v0.6.0 Action:**
- Monitor user reports
- No immediate action needed

---

### I6: Touch Interaction Optimization

**Severity:** LOW
**Affected:** All iOS Safari versions
**Status:** Enhancement opportunity

**Description:**
EdgeVec demos are designed for desktop first. Touch interactions may not be optimized:
- Text input focus
- Button tap areas
- Scroll behavior
- Viewport scaling

**Impact on EdgeVec:**
- Demos functional but not optimized for touch
- No blocking issues expected

**Evidence:**
- General mobile web best practices

**v0.6.0 Action:**
- Add `touch-action: manipulation` to interactive elements
- Increase tap target sizes
- Test with actual iOS devices

---

## Safari-Specific WebAssembly Notes

### Features EdgeVec Uses

| Feature | Safari Support | Status |
|:--------|:---------------|:-------|
| WebAssembly Core | Safari 11+ | ✅ OK |
| BigInt | Safari 14+ | ✅ OK |
| IndexedDB | Safari 8+ | ✅ OK |
| TextEncoder | Safari 10.3+ | ✅ OK |
| Float32Array | All versions | ✅ OK |
| Performance.now() | All versions | ✅ OK |

### Features EdgeVec Does NOT Use

| Feature | Safari Support | Why Not Used |
|:--------|:---------------|:-------------|
| SharedArrayBuffer | Safari 15.2+ | Threading not needed |
| Atomics.waitAsync | Safari 16.4+ | No threading |
| WASM SIMD | Partial | Using scalar fallbacks |
| WASM Threads | Partial | Single-threaded design |
| Multiple Memories | Not yet | Single memory model |
| Memory64 | Not yet | 32-bit sufficient |

---

## Version Compatibility Matrix

| iOS Version | Safari Version | EdgeVec Status |
|:------------|:---------------|:---------------|
| iOS 18.x | Safari 18.x | ✅ Fully compatible |
| iOS 17.x | Safari 17.x | ✅ Fully compatible |
| iOS 16.x | Safari 16.x | ✅ Fully compatible |
| iOS 15.x | Safari 15.x | ⚠️ May work, not tested |
| iOS 14.x | Safari 14.x | ⚠️ Minimum supported |
| iOS 13.x | Safari 13.x | ❌ Not supported (no BigInt) |

---

## Issue Priority for v0.6.0

### Must Address

1. **I1 (Memory Limit)** — Add mobile detection and vector count warnings
2. **I3 (7-Day Eviction)** — Document in user guide
3. **I4 (Private Browsing)** — Add detection and warning

### Should Address

4. **I6 (Touch)** — Improve demo touch interactions

### Can Defer

5. **I5 (Safari 15)** — Monitor only, low user base
6. **I2 (Memory Leak)** — Not applicable to EdgeVec

---

## Testing Recommendations

### Before v0.6.0 Release

1. Test on real iOS 17+ device
2. Verify 10k vector insertion works
3. Verify IndexedDB save/load works
4. Check demo touch interactions
5. Monitor console for errors

### Ongoing

1. Track user-reported iOS issues
2. Update this document with new findings
3. Test each release on iOS Safari

---

## References

- [WebKit Blog - Safari 18.2 Features](https://webkit.org/blog/16301/webkit-features-in-safari-18-2/)
- [WebKit Blog - Storage Policy Updates](https://webkit.org/blog/14403/updates-to-storage-policy/)
- [Can I Use - WebAssembly](https://caniuse.com/wasm)
- [MDN - Storage Quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)
- [State of WebAssembly 2024-2025](https://platform.uno/blog/state-of-webassembly-2024-2025/)

---

**Agent:** WASM_SPECIALIST
**Status:** W25.3.4 COMPLETE
**Next:** Day 3 Review
