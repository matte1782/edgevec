# Week 21, Day 4: Mobile Browser Testing

**Date:** 2026-01-02
**Sprint:** Week 21 (v0.5.0 Phase)
**Day Theme:** iOS Safari & Android Chrome Verification
**Status:** PLANNED

---

## Task W21.4: Mobile Browser Testing & Documentation

**Priority:** HIGH (P1)
**Estimated Effort:** 8 hours (3x rule: 2h optimistic × 3 = 6h + 2h buffer)
**Status:** PLANNED
**Depends On:** W21.3 complete (WASM bindings working)
**Blocks:** W21.5

---

### Context

Day 4 verifies that EdgeVec works correctly on mobile browsers — iOS Safari and Android Chrome. These platforms have different WASM implementations and memory constraints that must be validated.

**Strategic Importance:**
- Mobile is a key deployment target for edge computing
- iOS Safari has unique WASM quirks (memory limits, JIT differences)
- Proving mobile compatibility increases adoption confidence

**Reference Documents:**
- `docs/benchmarks/BUNDLE_SIZE_BASELINE.md` (current size)
- WebAssembly compatibility tables

---

### Objective

Validate EdgeVec functionality on mobile platforms:
1. iOS Safari (iPhone, iPad)
2. Android Chrome
3. Document any platform-specific limitations
4. Create mobile usage guide

---

### Technical Approach

#### 1. Test Environment Setup

**Options for Testing:**

| Method | iOS Safari | Android Chrome | Cost | Setup Time |
|:-------|:-----------|:---------------|:-----|:-----------|
| Physical devices | ✅ | ✅ | $0 (if owned) | 30m |
| iOS Simulator (macOS) | ✅ | ❌ | $0 | 1h |
| Android Emulator | ❌ | ✅ | $0 | 1h |
| BrowserStack | ✅ | ✅ | $$ | 2h |
| Sauce Labs | ✅ | ✅ | $$ | 2h |

**Recommended Approach:**
1. Use physical devices if available
2. Fall back to simulators/emulators
3. BrowserStack for automated CI (Day 5)

#### 2. Test Page Setup

**File: `tests/mobile/index.html`**
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>EdgeVec Mobile Test</title>
  <style>
    body { font-family: -apple-system, system-ui, sans-serif; padding: 20px; }
    .test { margin: 10px 0; padding: 10px; border-radius: 8px; }
    .pass { background: #d4edda; }
    .fail { background: #f8d7da; }
    .running { background: #fff3cd; }
    pre { overflow-x: auto; font-size: 12px; }
  </style>
</head>
<body>
  <h1>EdgeVec Mobile Tests</h1>
  <div id="results"></div>
  <pre id="log"></pre>

  <script type="module">
    import init, { HnswIndex, JsMetadataValue } from './edgevec.js';

    const results = document.getElementById('results');
    const log = document.getElementById('log');

    function logMessage(msg) {
      log.textContent += msg + '\n';
      console.log(msg);
    }

    function addResult(name, passed, error = null) {
      const div = document.createElement('div');
      div.className = `test ${passed ? 'pass' : 'fail'}`;
      div.innerHTML = `<strong>${passed ? '✅' : '❌'} ${name}</strong>`;
      if (error) {
        div.innerHTML += `<br><small>${error}</small>`;
      }
      results.appendChild(div);
    }

    async function runTests() {
      logMessage('Initializing EdgeVec WASM...');

      try {
        await init();
        addResult('WASM Initialization', true);
      } catch (e) {
        addResult('WASM Initialization', false, e.message);
        return;
      }

      // Test 1: Create Index
      try {
        logMessage('Creating index with 128 dimensions...');
        const index = new HnswIndex(128);
        addResult('Create HnswIndex', true);

        // Test 2: Insert Vector
        logMessage('Inserting vector...');
        const vector = new Float32Array(128).fill(0.5);
        const id = index.insert(vector);
        addResult('Insert Vector', id === 0);

        // Test 3: Search
        logMessage('Searching...');
        const query = new Float32Array(128).fill(0.5);
        const results = index.search(query, 1);
        addResult('Search', results.length === 1 && results[0].vectorId === 0);

        // Test 4: Metadata String
        logMessage('Testing metadata (string)...');
        index.setMetadata(0, 'title', JsMetadataValue.fromString('Test'));
        const title = index.getMetadata(0, 'title');
        addResult('Metadata String', title?.asString() === 'Test');

        // Test 5: Metadata Integer
        logMessage('Testing metadata (integer)...');
        index.setMetadata(0, 'count', JsMetadataValue.fromInteger(42));
        const count = index.getMetadata(0, 'count');
        addResult('Metadata Integer', count?.asInteger() === 42);

        // Test 6: Metadata Float
        logMessage('Testing metadata (float)...');
        index.setMetadata(0, 'score', JsMetadataValue.fromFloat(3.14));
        const score = index.getMetadata(0, 'score');
        addResult('Metadata Float', Math.abs(score?.asFloat() - 3.14) < 0.001);

        // Test 7: Metadata Boolean
        logMessage('Testing metadata (boolean)...');
        index.setMetadata(0, 'verified', JsMetadataValue.fromBoolean(true));
        const verified = index.getMetadata(0, 'verified');
        addResult('Metadata Boolean', verified?.asBoolean() === true);

        // Test 8: Metadata Array
        logMessage('Testing metadata (array)...');
        index.setMetadata(0, 'tags', JsMetadataValue.fromStringArray(['a', 'b']));
        const tags = index.getMetadata(0, 'tags');
        const tagsCorrect = tags?.asStringArray()?.join(',') === 'a,b';
        addResult('Metadata Array', tagsCorrect);

        // Test 9: Get All Metadata
        logMessage('Testing getAllMetadata...');
        const all = index.getAllMetadata(0);
        addResult('Get All Metadata', all && all.title === 'Test');

        // Test 10: Delete Metadata
        logMessage('Testing deleteMetadata...');
        const deleted = index.deleteMetadata(0, 'title');
        const gone = !index.hasMetadata(0, 'title');
        addResult('Delete Metadata', deleted && gone);

        // Test 11: Memory Stress (1000 vectors)
        logMessage('Memory stress test (1000 vectors)...');
        const stressIndex = new HnswIndex(128);
        for (let i = 0; i < 1000; i++) {
          const v = new Float32Array(128);
          for (let j = 0; j < 128; j++) v[j] = Math.random();
          stressIndex.insert(v);
        }
        const stressSearch = stressIndex.search(new Float32Array(128).fill(0.5), 10);
        addResult('Memory Stress (1000 vectors)', stressSearch.length === 10);

        // Test 12: Performance Timing
        logMessage('Performance timing...');
        const start = performance.now();
        for (let i = 0; i < 100; i++) {
          stressIndex.search(new Float32Array(128).fill(Math.random()), 10);
        }
        const elapsed = performance.now() - start;
        const avgMs = elapsed / 100;
        addResult(`Performance (100 searches)`, true);
        logMessage(`Average search time: ${avgMs.toFixed(2)}ms`);

        logMessage('All tests complete!');

      } catch (e) {
        addResult('Unexpected Error', false, e.message);
        logMessage(`Error: ${e.stack}`);
      }
    }

    runTests();
  </script>
</body>
</html>
```

#### 3. Test Matrix

**Minimum Test Coverage:**

| Platform | Browser | Version | Tests |
|:---------|:--------|:--------|:------|
| iOS | Safari | 16+ | All 12 tests |
| iOS | Safari | 15 | All 12 tests |
| Android | Chrome | 100+ | All 12 tests |
| Android | Chrome | 90+ | All 12 tests |

**Test Cases:**

| ID | Test | Pass Criteria |
|:---|:-----|:--------------|
| T1 | WASM Initialization | `init()` resolves without error |
| T2 | Create Index | `HnswIndex(128)` succeeds |
| T3 | Insert Vector | Returns vector ID 0 |
| T4 | Search | Returns 1 result with correct ID |
| T5 | Metadata String | Value roundtrip matches |
| T6 | Metadata Integer | Value roundtrip matches |
| T7 | Metadata Float | Value within epsilon |
| T8 | Metadata Boolean | Value matches |
| T9 | Metadata Array | Array elements match |
| T10 | Get All Metadata | Returns object with all keys |
| T11 | Delete Metadata | Key removed |
| T12 | Memory Stress | 1000 vectors insertable |

#### 4. Known Platform Limitations

**iOS Safari:**
- WASM memory limited to ~1GB (vs 4GB on desktop)
- No SharedArrayBuffer without cross-origin isolation
- JIT compilation may be slower initially
- Background tabs may have WASM suspended

**Android Chrome:**
- Memory varies by device (typically 512MB-2GB for WASM)
- Older devices may have slower WASM execution
- Battery saver mode may throttle WASM

#### 5. Documentation Output

**File: `docs/guides/MOBILE_USAGE.md`**
```markdown
# Mobile Browser Usage Guide

EdgeVec is fully compatible with mobile browsers. This guide covers
platform-specific considerations for iOS Safari and Android Chrome.

## Browser Support

| Platform | Browser | Minimum Version | Status |
|:---------|:--------|:----------------|:-------|
| iOS | Safari | 15+ | ✅ Supported |
| Android | Chrome | 90+ | ✅ Supported |
| Android | Firefox | 100+ | ✅ Supported |

## Memory Considerations

### iOS Safari
- Maximum WASM memory: ~1GB
- Recommended max vectors: 500,000 (128-dim)
- Use `HnswConfig.with_memory_limit()` for explicit control

### Android Chrome
- Maximum WASM memory: varies by device
- Low-end devices: 100,000 vectors max
- High-end devices: 500,000+ vectors

## Performance Tips

1. **Lazy Loading**: Load EdgeVec only when needed
2. **Web Workers**: Use for background indexing
3. **Batch Operations**: Insert vectors in batches
4. **Quantization**: Use SQ8 for 4x memory reduction

## Example: Safe Mobile Initialization

```javascript
async function initEdgeVec() {
  try {
    // Dynamic import for code splitting
    const { default: init, HnswIndex } = await import('edgevec');
    await init();

    // Conservative config for mobile
    const config = {
      dimensions: 128,
      maxElements: 100000,  // Safe for most mobile devices
      efConstruction: 100,
      m: 16
    };

    return new HnswIndex(config.dimensions, config);
  } catch (e) {
    console.error('EdgeVec initialization failed:', e);
    // Fallback to server-side search
    return null;
  }
}
```

## Troubleshooting

### "Out of memory" errors
- Reduce `maxElements` in config
- Use quantization (SQ8) for 4x memory reduction
- Implement pagination for large datasets

### Slow initial load
- WASM JIT compilation takes time on first load
- Use `wasm-opt` for smaller bundle size
- Consider preloading WASM in service worker

### Search hangs on iOS
- Background tabs may suspend WASM
- Use `requestIdleCallback` for non-urgent operations
- Handle visibility changes with Page Visibility API
```

---

### Acceptance Criteria

**CRITICAL (Must Pass):**
- [ ] All 12 tests pass on iOS Safari 16+
- [ ] All 12 tests pass on Android Chrome 100+
- [ ] WASM initialization succeeds on both platforms
- [ ] Metadata operations work on both platforms
- [ ] No crashes or memory errors during stress test

**MAJOR (Should Pass):**
- [ ] Tests pass on iOS Safari 15
- [ ] Tests pass on Android Chrome 90+
- [ ] Performance < 50ms/search (1000 vectors, 128 dims)
- [ ] Mobile usage guide documented
- [ ] Known limitations documented

**MINOR (Nice to Have):**
- [ ] Tests pass on Firefox for Android
- [ ] Performance benchmarks recorded
- [ ] Video recording of test execution

---

### Implementation Checklist

- [ ] Create `tests/mobile/index.html` test page
- [ ] Deploy test page to accessible URL (localhost tunnel or staging)
- [ ] Test on iOS Safari (device or Simulator)
- [ ] Test on Android Chrome (device or Emulator)
- [ ] Record test results in markdown
- [ ] Document any failures or limitations
- [ ] Create `docs/guides/MOBILE_USAGE.md`
- [ ] Update README with mobile support status

---

### Test Results Template

**File: `docs/testing/MOBILE_TEST_RESULTS.md`**
```markdown
# Mobile Browser Test Results

**Date:** 2026-01-02
**Tester:** [Name]
**EdgeVec Version:** 0.5.0-alpha

## iOS Safari

**Device:** [iPhone model or Simulator]
**iOS Version:** [version]
**Safari Version:** [version]

| Test | Result | Notes |
|:-----|:-------|:------|
| WASM Init | PASS/FAIL | |
| Create Index | PASS/FAIL | |
| Insert Vector | PASS/FAIL | |
| Search | PASS/FAIL | |
| Metadata String | PASS/FAIL | |
| Metadata Integer | PASS/FAIL | |
| Metadata Float | PASS/FAIL | |
| Metadata Boolean | PASS/FAIL | |
| Metadata Array | PASS/FAIL | |
| Get All Metadata | PASS/FAIL | |
| Delete Metadata | PASS/FAIL | |
| Memory Stress | PASS/FAIL | |

**Performance:** [avg search time]ms

## Android Chrome

**Device:** [device model or Emulator]
**Android Version:** [version]
**Chrome Version:** [version]

| Test | Result | Notes |
|:-----|:-------|:------|
| WASM Init | PASS/FAIL | |
| ... | | |

**Performance:** [avg search time]ms

## Limitations Discovered

1. [Description of any issues]
2. [Workarounds if applicable]

## Screenshots

[Attach screenshots of test results]
```

---

### Dependencies

**Blocks:**
- W21.5 (BrowserStack needs test page ready)

**Blocked By:**
- W21.3 complete (WASM bindings must work)

**External Dependencies:**
- Physical iOS device OR macOS with Xcode (for Simulator)
- Physical Android device OR Android Studio (for Emulator)
- OR BrowserStack/Sauce Labs account

---

### Verification Method

**Day 4 is COMPLETE when:**

1. Test page created and accessible
2. iOS Safari tests executed (12/12 pass)
3. Android Chrome tests executed (12/12 pass)
4. Test results documented in markdown
5. Mobile usage guide created
6. Known limitations documented

---

### Rollback Plan

If Day 4 encounters blocking issues:

1. **No iOS device/Simulator:** Use BrowserStack (defer to Day 5)
2. **WASM fails on mobile:** Document as known limitation, investigate root cause
3. **Memory stress fails:** Lower test threshold, document memory limits
4. **Specific browser version fails:** Document minimum supported version

---

### Estimated Timeline

| Phase | Time | Cumulative |
|:------|:-----|:-----------|
| Test page creation | 1.5h | 1.5h |
| iOS Safari testing | 2h | 3.5h |
| Android Chrome testing | 2h | 5.5h |
| Documentation | 1.5h | 7h |
| Troubleshooting buffer | 1h | 8h |

---

### Hostile Review Checkpoint

**End of Day 4:** Submit for `/review` with:
- `tests/mobile/index.html`
- `docs/testing/MOBILE_TEST_RESULTS.md`
- `docs/guides/MOBILE_USAGE.md`
- Screenshots/evidence of test execution

**Expected Review Focus:**
- Test coverage completeness
- Documentation quality
- Platform support claims accuracy
- Known limitations honesty

---

**Task Owner:** WASM_SPECIALIST / TEST_ENGINEER
**Review Required:** HOSTILE_REVIEWER
**Next Task:** W21.5 (BrowserStack CI & Schema Freeze)

---

*"Test on real devices. Document real limitations. No surprises in production."*
