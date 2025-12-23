# Android Chrome WASM Compatibility

**Document:** W25.4.1 — Android Chrome WASM Research
**Author:** WASM_SPECIALIST
**Date:** 2025-12-20
**Status:** [PROPOSED]

---

## Executive Summary

Android Chrome provides **full WebAssembly 1.0 support** with excellent feature coverage matching desktop Chrome. EdgeVec is fully compatible with Chrome Android 120+. Key considerations are **memory constraints** (~300MB practical limit) and **IndexedDB quotas** (6-20% of available storage per origin).

---

## 1. WebAssembly Support Matrix

### Chrome Android Version Support

| Feature | Chrome 57+ | Chrome 120+ | Chrome 133+ | Notes |
|:--------|:-----------|:------------|:------------|:------|
| WebAssembly 1.0 | ✅ Full | ✅ Full | ✅ Full | Core spec since 2017 |
| SIMD | ❌ | ✅ Full | ✅ Full | 128-bit SIMD operations |
| Bulk Memory | ❌ | ✅ Full | ✅ Full | Memory.copy, Memory.fill |
| Reference Types | ❌ | ✅ Full | ✅ Full | externref, funcref |
| Multi-Value Returns | ❌ | ✅ Full | ✅ Full | Functions return multiple values |
| Tail Calls | ❌ | ✅ Full | ✅ Full | Tail call optimization |
| Exception Handling | ❌ | ⚠️ Partial | ✅ Full | try/catch/throw |
| Garbage Collection | ❌ | ❌ | ✅ Full | Required for GC languages |
| Memory64 | ❌ | ❌ | ✅ Full | 64-bit address space |
| Relaxed SIMD | ❌ | ❌ | ✅ Full | Relaxed determinism SIMD |
| Multiple Memories | ❌ | ⚠️ Partial | ✅ Full | Module isolation |

**Source:** [V8 Blog](https://v8.dev/blog/4gb-wasm-memory), [Can I Use WASM](https://caniuse.com/wasm)

### EdgeVec Feature Requirements

| EdgeVec Feature | Required WASM Feature | Chrome Android Support |
|:----------------|:---------------------|:-----------------------|
| Vector Operations | WASM 1.0 + SIMD | ✅ Chrome 91+ |
| HNSW Search | WASM 1.0 | ✅ Chrome 57+ |
| Filter Parser | WASM 1.0 | ✅ Chrome 57+ |
| IndexedDB Storage | Web API | ✅ Chrome 57+ |
| Bulk Memory Ops | Bulk Memory | ✅ Chrome 75+ |

**Verdict:** EdgeVec works on Chrome Android 91+ (SIMD), recommended 120+.

---

## 2. WebAssembly.Memory Limits

### Desktop vs Mobile Memory

| Platform | Practical Limit | Maximum (wasm32) | Maximum (Memory64) |
|:---------|:----------------|:-----------------|:-------------------|
| Desktop Chrome | 2-4 GB | 4 GB | 16 GB |
| **Android Chrome** | **~300 MB** | 4 GB (theoretical) | Not recommended |
| iOS Safari | ~256-512 MB | 4 GB (theoretical) | Not available |

**Source:** [V8 4GB WASM Memory](https://v8.dev/blog/4gb-wasm-memory), [Chromium Bug #1175564](https://bugs.chromium.org/p/chromium/issues/detail?id=1175564)

### Android Memory Constraints

```
CRITICAL: Android Chrome runs as a 32-bit process on most devices.
This limits practical WASM memory allocation to ~300MB.
```

**Key Factors:**
1. **32-bit Process:** Most Android Chrome instances are 32-bit, limiting address space
2. **Memory Pressure:** Android aggressively kills background apps for memory
3. **Device RAM:** Low-end devices (2-3GB RAM) have tighter constraints
4. **Browser Memory:** Chrome itself consumes significant memory

### EdgeVec Memory Profile

| Vector Count | Dimension | Memory (F32) | Memory (Quantized) | Android Feasible |
|:-------------|:----------|:-------------|:-------------------|:-----------------|
| 10,000 | 128 | ~5 MB | ~1.3 MB | ✅ Excellent |
| 50,000 | 128 | ~25 MB | ~6.4 MB | ✅ Good |
| 100,000 | 128 | ~50 MB | ~12.8 MB | ✅ Good |
| 500,000 | 128 | ~250 MB | ~64 MB | ⚠️ Risky |
| 1,000,000 | 128 | ~500 MB | ~128 MB | ⚠️ May fail |

**Recommendation:** Stay under 100K vectors on Android for reliable operation.

### Memory Best Practices

```javascript
// Check available memory before large operations
const performance = window.performance;
if (performance.memory) {
  const { usedJSHeapSize, jsHeapSizeLimit } = performance.memory;
  const available = jsHeapSizeLimit - usedJSHeapSize;
  console.log(`Available JS heap: ${(available / 1024 / 1024).toFixed(1)} MB`);
}

// Handle memory errors gracefully
try {
  const index = new EdgeVec(config);
  // ... operations
} catch (e) {
  if (e.message.includes('memory')) {
    console.error('Memory allocation failed. Reduce vector count.');
  }
}
```

---

## 3. IndexedDB Storage Limits

### Quota Calculation

| Condition | Per-Origin Quota |
|:----------|:-----------------|
| Default | 6% of free disk space |
| Maximum | ~20% of browser storage pool |
| Browser Pool | 33% of free disk space |
| Chrome 67+ Minimum | min(2GB, 10% of total storage) reserved |

**Source:** [MDN Storage Quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria), [RxDB IndexedDB Limits](https://rxdb.info/articles/indexeddb-max-storage-limit.html)

### Practical Android Examples

| Device Storage | Free Space | Browser Pool | Origin Quota |
|:---------------|:-----------|:-------------|:-------------|
| 32 GB | 10 GB | 3.3 GB | ~200-660 MB |
| 64 GB | 30 GB | 10 GB | ~600 MB - 2 GB |
| 128 GB | 80 GB | 26 GB | ~1.5-5 GB |

### Checking Available Quota

```javascript
async function checkStorageQuota() {
  if ('storage' in navigator && 'estimate' in navigator.storage) {
    const { quota, usage } = await navigator.storage.estimate();
    return {
      quotaMB: (quota / 1024 / 1024).toFixed(2),
      usageMB: (usage / 1024 / 1024).toFixed(2),
      availableMB: ((quota - usage) / 1024 / 1024).toFixed(2)
    };
  }
  return null;
}

// Example output on typical Android device:
// { quotaMB: "487.50", usageMB: "12.30", availableMB: "475.20" }
```

### Eviction Policy

Chrome uses **LRU (Least Recently Used)** eviction:
1. When browser storage exceeds limit, oldest-accessed origins are cleared first
2. EdgeVec indices may be evicted if user hasn't accessed the app recently
3. No warning before eviction — data simply disappears

**Mitigation:**
```javascript
// Request persistent storage to prevent eviction
if (navigator.storage && navigator.storage.persist) {
  const isPersisted = await navigator.storage.persist();
  console.log(`Persistent storage: ${isPersisted ? 'granted' : 'denied'}`);
}
```

---

## 4. Android-Specific WASM Quirks

### 4.1 Touch Event Handling

Unlike desktop, Android requires proper touch event handling:

```javascript
// WASM-bound functions should work with both mouse and touch
element.addEventListener('pointerdown', handleInteraction);
element.addEventListener('pointerup', handleInteraction);

// Avoid mousedown/mouseup which may have 300ms delay on mobile
```

### 4.2 Performance Timing Resolution

**Android Chrome has FULL timer precision** (unlike iOS Safari which limits to 1ms).

However, for very fast operations, batch timing can still improve measurement stability:

```javascript
// Optional: Batch timing for sub-millisecond operations
const ITERATIONS = 50;
const start = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  index.search(query, 10);
}
const avgTime = (performance.now() - start) / ITERATIONS;
```

**Note:** iOS Safari limits `performance.now()` to 1ms resolution (Spectre mitigation). Android Chrome does NOT have this limitation.

### 4.3 Background Tab Throttling

Chrome on Android aggressively throttles background tabs:
- Timers reduced to 1 per minute after 5 minutes
- WASM execution may be paused entirely
- IndexedDB transactions may timeout

**Recommendation:** Complete IndexedDB operations quickly; don't rely on background processing.

### 4.4 WebView vs Chrome

Android WebView (used in hybrid apps) may have different WASM behavior:
- Older WebView versions may lack SIMD support
- Memory limits may be more restrictive
- Check `navigator.userAgent` to detect WebView:

```javascript
const isWebView = /wv/.test(navigator.userAgent);
const isChrome = /Chrome/.test(navigator.userAgent) && !isWebView;
```

### 4.5 JIT Compilation

Android Chrome's V8 JIT may behave differently under memory pressure:
- Hot functions may not be optimized
- Tier-up compilation may be delayed
- First-run performance may be slower than desktop

---

## 5. Feature Detection

### Comprehensive Feature Check

```javascript
async function detectAndroidWASMCapabilities() {
  const capabilities = {
    platform: 'android-chrome',
    wasm: typeof WebAssembly !== 'undefined',
    simd: false,
    bulkMemory: false,
    indexedDB: false,
    persistentStorage: false,
    storageQuota: null
  };

  // Check WASM SIMD
  try {
    const simdTest = new Uint8Array([0,97,115,109,1,0,0,0,1,5,1,96,0,1,123,3,2,1,0,10,10,1,8,0,65,0,253,15,253,98,11]);
    await WebAssembly.compile(simdTest);
    capabilities.simd = true;
  } catch (e) {}

  // Check Bulk Memory
  try {
    const bulkTest = new Uint8Array([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,5,3,1,0,1,10,14,1,12,0,65,0,65,0,65,0,252,10,0,0,11]);
    await WebAssembly.compile(bulkTest);
    capabilities.bulkMemory = true;
  } catch (e) {}

  // Check IndexedDB
  try {
    const db = indexedDB.open('test');
    capabilities.indexedDB = true;
    db.result?.close();
  } catch (e) {}

  // Check persistent storage
  if (navigator.storage?.persist) {
    capabilities.persistentStorage = await navigator.storage.persisted();
  }

  // Check storage quota
  if (navigator.storage?.estimate) {
    const { quota, usage } = await navigator.storage.estimate();
    capabilities.storageQuota = { quotaMB: quota / 1024 / 1024, usageMB: usage / 1024 / 1024 };
  }

  return capabilities;
}
```

---

## 6. EdgeVec Compatibility Summary

### Full Compatibility ✅

| Feature | Status | Notes |
|:--------|:-------|:------|
| WASM Core | ✅ Works | All Chrome Android versions |
| SIMD | ✅ Works | Chrome 91+ |
| Filter API | ✅ Works | Parser runs in WASM |
| IndexedDB Persistence | ✅ Works | Standard quota limits apply |
| Batch Insert | ✅ Works | Stay under memory limits |
| Search | ✅ Works | Full HNSW algorithm |

### Considerations ⚠️

| Concern | Impact | Mitigation |
|:--------|:-------|:-----------|
| Memory limit ~300MB | Large indices may fail | Cap at 100K vectors |
| Storage eviction | Data loss possible | Request persistent storage |
| Background throttling | Operations may timeout | Complete ops in foreground |
| Timer precision | Benchmarks inaccurate | Use batch timing |

---

## 7. Recommended Chrome Android Versions

| Version | Release Date | WASM Features | EdgeVec Support |
|:--------|:-------------|:--------------|:----------------|
| 91+ | May 2021 | SIMD | ✅ Minimum |
| 100+ | Mar 2022 | Exception Handling | ✅ Recommended |
| 120+ | Dec 2023 | Full Feature Set | ✅ **Best** |
| 133+ | Feb 2025 | Memory64, Relaxed SIMD | ✅ Latest |

**Recommendation:** Target Chrome Android 91+ for SIMD, recommend 120+ for best experience.

---

## References

- [V8 Blog: 4GB WASM Memory](https://v8.dev/blog/4gb-wasm-memory)
- [Can I Use: WebAssembly](https://caniuse.com/wasm)
- [MDN: Storage Quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)
- [RxDB: IndexedDB Limits](https://rxdb.info/articles/indexeddb-max-storage-limit.html)
- [Chrome Status: WASM Features](https://chromestatus.com/feature/5453022515691520)
- [State of WebAssembly 2024-2025](https://platform.uno/blog/state-of-webassembly-2024-2025/)

---

**Document Status:** [PROPOSED]
**Next:** W25.4.2 Android Testing Setup
