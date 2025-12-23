# Root Cause Analysis: iOS Filter Inconsistency

**Date:** 2025-12-19
**Investigator:** RUST_ENGINEER
**Task:** R1.1 (W25_DAY3_REMEDIATION_PLAN.md)
**Priority:** P0 (CRITICAL - Correctness violation)

---

## Executive Summary

iOS Safari testing revealed **platform-specific filter behavior** where the same filter expression returns different results on iOS vs desktop. This is a **CRITICAL correctness violation** as EdgeVec's WASM execution should be deterministic across platforms.

**Primary Issue:** `TypeError: wasmModule.parse_filter_js is not a function` (iOS only)
**Secondary Issue:** Filter results differ between iOS and desktop (when it does work)

---

## Investigation Findings

### Finding 1: WASM Module Export Issue (C1)

**Evidence:**
```
TypeError: wasmModule.parse_filter_js is not a function.
(In 'wasmModule.parse_filter_js(filterStr)', 'wasmModule.parse_filter_js' is undefined)
```
Source: W25_DAY3_IOS_TEST_CHECKLIST.md:43-44

**Analysis:**
- The function `parse_filter_js` is properly defined with `#[wasm_bindgen]` at `src/wasm/filter.rs:67`
- Function compiles successfully on desktop (tests pass)
- Function is undefined on iOS Safari specifically

**Hypothesis:**
1. **wasm-bindgen export naming:** Function may be exported with snake_case `parse_filter_js` on desktop but wasm-bindgen might mangle names differently on iOS
2. **WASM module initialization incomplete:** `await wasmModule.default()` may complete on desktop but fail silently on iOS
3. **Build artifact differences:** wasm-pack may generate different exports for different environments

**Investigation Needed:**
- [ ] Check actual wasm-pack build output for exported function names
- [ ] Verify wasm-bindgen version consistency
- [ ] Test if other WASM exports (EdgeVec, EdgeVecConfig) work on iOS
- [ ] Add console.log to check actual module exports on iOS

---

### Finding 2: Platform-Specific Filter Result Difference (C4)

**Evidence:**
```
The ilter demo doesnt work with filters only on mobile on desktop it work
but with the same it says no found with those while the desktop one does
```
Source: W25_DAY3_IOS_TEST_CHECKLIST.md:32

**Analysis:**
This is the **highest priority** issue as it violates WASM determinism guarantees.

**Code Review:**
1. **String comparison** (`src/filter/evaluator.rs:329`):
   ```rust
   (ResolvedValue::String(a), ResolvedValue::String(b)) => a == b,
   ```
   - Uses direct `==` on Rust `String` types
   - This performs **byte-for-byte UTF-8 comparison**
   - Should be platform-independent

2. **No locale-dependent operations:**
   - Filter parsing uses Pest grammar (deterministic)
   - No calls to `to_lowercase()` in evaluation (only in parser suggestions)
   - No floating point special values in comparison (uses EPSILON)

**Potential Root Causes:**

#### Hypothesis A: Hash Map Iteration Order
```rust
pub fn evaluate(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError>
```

- `HashMap` iteration order is non-deterministic
- However, we're doing **lookups**, not iteration
- `metadata.get(name)` is deterministic

**Verdict:** Unlikely to be the root cause

#### Hypothesis B: Float Precision Differences
```rust
(ResolvedValue::Float(a), ResolvedValue::Float(b)) => (a - b).abs() < f64::EPSILON,
```

- Different platforms might have slightly different `f64::EPSILON` behavior
- iOS ARM64 vs Desktop x86-64 floating point units

**Test Needed:**
- [ ] Check if filter tests involve float comparisons
- [ ] Add property test for float comparison determinism

#### Hypothesis C: WASM Module Not Fully Loaded
If `parse_filter_js` is undefined, then **maybe the WASM module loaded a partial/corrupted version** where:
- Core exports (EdgeVec, EdgeVecConfig) work
- Filter exports are missing
- Demo appears to "work" but filter logic is actually running different code paths

**This is the MOST LIKELY cause:**
- Error says `parse_filter_js is not a function`
- If parsing fails, JavaScript might fall back to a buggy pure-JS implementation
- Desktop uses correct Rust parser, iOS uses broken fallback → different results

**Verification Needed:**
- [ ] Check if filter-playground.html has a JavaScript fallback
- [ ] Verify what happens when `parse_filter_js` is undefined

---

### Finding 3: Compaction Non-Functional (C6)

**Evidence:**
```
dOSNT SEEM TO BE WORKING DOESNT RESET THE TOMBSTONE IT LAGS
```
Source: W25_DAY3_IOS_TEST_CHECKLIST.md:78

**Analysis:**
- Compaction is a Rust-side operation
- Should be platform-independent
- "IT LAGS" suggests the operation **is running** but may be slow/incomplete

**Hypothesis:**
- Compaction may be triggering iOS memory pressure
- iOS Safari may be pausing/throttling WASM execution
- Tombstone count UI may not be updating (rendering issue, not logic issue)

**Investigation Needed:**
- [ ] Add console logging to compact() function
- [ ] Verify tombstone_count() returns correct value after compaction
- [ ] Check if this is a UI update issue vs actual compaction failure

---

## Recommended Immediate Actions

### 1. Verify WASM Module Exports on iOS (P0)

**Action:** Add debugging to filter-playground.html
```javascript
async function initWasm() {
    try {
        wasmModule = await import(wasmPath);
        await wasmModule.default();

        // ADD THIS:
        console.log('[FilterPlayground] Module exports:', Object.keys(wasmModule));
        console.log('[FilterPlayground] parse_filter_js:', typeof wasmModule.parse_filter_js);
        console.log('[FilterPlayground] EdgeVec:', typeof wasmModule.EdgeVec);

        if (typeof wasmModule.parse_filter_js !== 'function') {
            throw new Error('parse_filter_js not exported');
        }

        loadingPlaceholder.innerHTML = '<span style="color: var(--green);">WASM module loaded. Enter a filter expression above.</span>';
        filterInput.focus();
        return true;
    } catch (e) {
        // ...
    }
}
```

**Expected Outcome:**
- Desktop: `parse_filter_js: function`
- iOS (current): `parse_filter_js: undefined`

---

### 2. Add Determinism Property Test (P0)

**Action:** Create `tests/filter_platform_determinism.rs`
```rust
use edgevec::filter::{evaluate, parse};
use edgevec::metadata::MetadataValue;
use std::collections::HashMap;

#[test]
fn test_string_comparison_deterministic() {
    let mut metadata = HashMap::new();
    metadata.insert("category".to_string(), MetadataValue::String("gpu".to_string()));

    // Parse and evaluate 1000 times - should always return same result
    for _ in 0..1000 {
        let expr = parse("category = \"gpu\"").unwrap();
        let result = evaluate(&expr, &metadata).unwrap();
        assert!(result, "String equality should be deterministic");
    }
}

#[test]
fn test_filter_parsing_deterministic() {
    // Same filter string should parse to identical AST
    let filter_str = "price > 100 AND category = \"electronics\"";

    let ast1 = parse(filter_str).unwrap();
    let ast2 = parse(filter_str).unwrap();

    // ASTs should be byte-for-byte identical
    let json1 = serde_json::to_string(&ast1).unwrap();
    let json2 = serde_json::to_string(&ast2).unwrap();

    assert_eq!(json1, json2, "Parsing should be deterministic");
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
fn test_wasm_filter_determinism() {
    use wasm_bindgen_test::*;
    use crate::wasm::filter::parse_filter_js;

    let filter_str = "category = \"test\"";

    // Parse 100 times - should return identical JSON
    let first_parse = parse_filter_js(filter_str).unwrap();

    for _ in 0..100 {
        let parse_result = parse_filter_js(filter_str).unwrap();
        assert_eq!(parse_result, first_parse, "WASM parsing should be deterministic");
    }
}
```

---

### 3. Check wasm-pack Build Output (P0)

**Action:**
```bash
cd wasm
wasm-pack build --target web --out-dir pkg
# Inspect pkg/edgevec.js
grep -A 5 "parse_filter_js" pkg/edgevec.js
```

**Expected:**
```javascript
export function parse_filter_js(filter_str) {
    // ...
}
```

**If missing:**
- wasm-bindgen export is broken
- Need to check Cargo.toml dependencies
- May need to update wasm-bindgen version

---

### 4. Add WASM Export Verification Test (P0)

**Action:** Create `wasm/__tests__/exports.test.js`
```javascript
import * as wasm from '../pkg/edgevec.js';

describe('WASM Module Exports', () => {
    beforeAll(async () => {
        await wasm.default();
    });

    test('parse_filter_js should be exported', () => {
        expect(typeof wasm.parse_filter_js).toBe('function');
    });

    test('parse_filter_js should parse simple filter', () => {
        const result = wasm.parse_filter_js('category = "test"');
        expect(typeof result).toBe('string');
        const ast = JSON.parse(result);
        expect(ast).toBeDefined();
    });

    test('EdgeVec should be exported', () => {
        expect(typeof wasm.EdgeVec).toBe('function');
    });

    test('EdgeVecConfig should be exported', () => {
        expect(typeof wasm.EdgeVecConfig).toBe('function');
    });
});
```

---

## Platform-Specific Differences to Investigate

### Rust WASM Targets

| Aspect | Desktop (x86-64) | iOS (arm64) |
|:-------|:----------------|:------------|
| **Target** | `wasm32-unknown-unknown` | `wasm32-unknown-unknown` |
| **Float ABI** | IEEE 754 | IEEE 754 |
| **Endianness** | Little | Little |
| **String encoding** | UTF-8 | UTF-8 |

**Conclusion:** Should be identical - WASM is platform-independent

### Possible Safari-Specific Issues

1. **wasm-bindgen version compatibility:**
   - Safari 18.2 is very new
   - May have regressions with wasm-bindgen 0.2.x

2. **WASM module caching:**
   - Desktop may cache compiled module
   - iOS may compile differently

3. **Memory limits during compilation:**
   - iOS Safari may OOM during WASM instantiation
   - Partial module load → missing exports

---

## Next Steps (Ordered by Priority)

1. **P0 - IMMEDIATE:** Add WASM export debugging to filter-playground.html
2. **P0 - IMMEDIATE:** Test on iPhone 15 Pro with debugging enabled
3. **P0 - URGENT:** Create property tests for determinism
4. **P1:** Inspect wasm-pack build artifacts
5. **P1:** Add WASM export verification tests
6. **P2:** Profile compaction performance on iOS

---

## Success Criteria

**R1.1 is complete when:**
1. **Root cause identified:** Concrete explanation for why `parse_filter_js` is undefined on iOS
2. **Fix implemented:** WASM exports work correctly on iOS Safari 18.2
3. **Determinism verified:** Property test confirms identical behavior across platforms
4. **Re-tested on iPhone 15 Pro:** Filter Playground works without errors

---

**Status:** Investigation in progress
**Next Action:** Add WASM export debugging (immediate)
**Blocker:** None (can proceed)
