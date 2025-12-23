# W25 Day 3 Remediation Plan — iOS Testing Fixes

**Created:** 2025-12-19
**Status:** [PROPOSED]
**Priority:** P0 (Blocks v0.6.0 mobile support)
**Estimated Duration:** 12-16 hours (2 work days)

---

## Executive Summary

HOSTILE_REVIEWER rejected W25.3.3 deliverables with **6 CRITICAL** and **4 MAJOR** issues found during iPhone 15 Pro (iOS 18.2) testing. This plan outlines the remediation work required to fix all blocking issues and complete W25.3 properly.

**Rejection Document:** `docs/reviews/2025-12-19_W25_DAY3_IOS_TESTING_REJECTED.md`

---

## Remediation Phases

### Phase 1: CRITICAL Correctness Issues (Priority Order)

#### Task R1.1: Investigate Filter Result Inconsistency [C4]

**Issue:** Same filter returns different results on iOS vs desktop.

**Priority:** P0 (HIGHEST — Correctness violation)

**Agent:** RUST_ENGINEER

**Investigation Steps:**
1. Create minimal reproduction case:
   ```javascript
   // Test on both desktop and iOS Safari
   const filter = Filter.parse('category = "test"');
   const testData = [/* identical dataset */];
   const desktopResults = filter.evaluate(testData);
   const iosResults = filter.evaluate(testData);
   console.assert(desktopResults === iosResults);
   ```

2. Check filter parsing logic for platform-specific behavior:
   - String comparison (case sensitivity, locale)
   - Number parsing (float precision)
   - Boolean logic (short-circuit evaluation)

3. Verify WASM module determinism:
   - Compare WASM binary checksums (desktop vs iOS build)
   - Check wasm-bindgen version consistency
   - Verify no UB in Rust code (run Miri)

4. Add property test:
   ```rust
   #[proptest]
   fn filter_deterministic_across_platforms(filter: FilterExpr, data: Vec<Item>) {
       // Parse and evaluate should be platform-independent
   }
   ```

**Acceptance Criteria:**
- Root cause identified and documented
- Fix implemented and verified on both platforms
- Property test added to prevent regression
- Re-test on iPhone 15 Pro confirms consistency

**Estimated Duration:** 4-6 hours

**Deliverables:**
- Root cause analysis document
- Code fix (if Rust-side issue)
- Property test in `tests/filter_determinism.rs`

---

#### Task R1.2: Fix Filter Playground WASM Module [C1]

**Issue:** `parse_filter_js` function undefined on iOS Safari.

**Priority:** P0

**Agent:** WASM_SPECIALIST

**Investigation Steps:**
1. Check wasm-pack build output:
   ```bash
   wasm-pack build --target web --out-dir pkg
   # Inspect pkg/edgevec_wasm.js for parse_filter_js export
   ```

2. Verify wasm-bindgen export:
   ```rust
   // In src/lib.rs or filter module
   #[wasm_bindgen(js_name = parse_filter_js)]
   pub fn parse_filter(...) -> Result<JsValue, JsValue> {
       // ...
   }
   ```

3. Test on iOS Safari:
   - Load WASM module
   - Inspect `wasmModule` object in console
   - Verify all expected exports present

4. Check for iOS-specific wasm-bindgen issues:
   - Review [wasm-bindgen Safari compatibility](https://github.com/rustwasm/wasm-bindgen/issues)
   - Test with latest wasm-bindgen (if upgrade needed)

**Acceptance Criteria:**
- `parse_filter_js` function available on iOS Safari
- Filter Playground loads without errors on iOS
- All filter parsing tests pass on iOS

**Estimated Duration:** 2-3 hours

**Deliverables:**
- Fixed WASM build (if config issue)
- Updated wasm-bindgen (if version issue)
- iOS Safari verification screenshots

---

#### Task R1.3: Fix Filter Playground Mobile UI [C2]

**Issue:** UI broken on mobile (horizontal overflow, touch issues).

**Priority:** P0

**Agent:** WASM_SPECIALIST (with DOCWRITER for UX polish)

**Implementation Steps:**
1. Add responsive CSS:
   ```css
   /* filter-playground.html */
   @media (max-width: 768px) {
     .container {
       width: 100%;
       padding: 1rem;
     }
     .filter-input {
       font-size: 16px; /* Prevents iOS zoom on focus */
       width: 100%;
     }
     .results {
       overflow-x: auto;
       -webkit-overflow-scrolling: touch;
     }
   }
   ```

2. Fix touch targets (iOS minimum 44x44px):
   ```css
   button {
     min-height: 44px;
     min-width: 44px;
     padding: 12px 24px;
   }
   ```

3. Add viewport meta tag:
   ```html
   <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=5.0">
   ```

4. Test on iOS Safari:
   - Verify no horizontal scrolling
   - Test all touch interactions
   - Check keyboard behavior

**Acceptance Criteria:**
- No horizontal overflow on iPhone 15 Pro
- All buttons tappable (44x44px minimum)
- Keyboard appears correctly on input focus
- Layout responsive from 320px to 1920px

**Estimated Duration:** 3-4 hours

**Deliverables:**
- Updated `filter-playground.html` with responsive CSS
- Mobile-specific styles in `examples.css`
- iOS Safari screenshots showing fixed UI

---

#### Task R1.4: Fix Demo Catalog Horizontal Layout [C3]

**Issue:** Index page UI broken horizontally on iOS only.

**Priority:** P0

**Agent:** WASM_SPECIALIST

**Investigation Steps:**
1. Use iOS Safari Web Inspector (Mac required):
   - Connect iPhone via USB
   - Safari > Develop > [iPhone] > index.html
   - Inspect layout issues in Elements tab

2. Check for common iOS layout bugs:
   - Flexbox min-width issues
   - Viewport units (vh/vw) rendering
   - CSS Grid auto-placement on iOS

3. Fix identified issues:
   ```css
   /* Example: Flexbox fix */
   .demo-card {
     flex: 1 1 300px; /* Add min-width */
     min-width: 0; /* Prevent flex overflow */
   }
   ```

4. Test on iOS Safari:
   - Verify cards display correctly
   - Check horizontal alignment
   - Test portrait and landscape orientations

**Acceptance Criteria:**
- Index page renders correctly on iPhone 15 Pro
- No horizontal scrolling or cut-off elements
- Layout works in both portrait and landscape

**Estimated Duration:** 2-3 hours

**Deliverables:**
- Fixed `index.html` layout
- iOS Safari screenshots
- Documented iOS-specific CSS fixes

---

#### Task R1.5: Fix Benchmark Dashboard Invalid Data [C5]

**Issue:** Benchmarks return 0 results and +NaN%.

**Priority:** P0

**Agent:** BENCHMARK_SCIENTIST

**Investigation Steps:**
1. Check benchmark execution on iOS:
   ```javascript
   // In benchmark-dashboard.html
   console.log('Benchmark start:', performance.now());
   const result = runBenchmark();
   console.log('Benchmark result:', result);
   console.log('Benchmark end:', performance.now());
   ```

2. Verify timing API precision on iOS:
   - iOS Safari may have reduced `performance.now()` precision
   - Check for zero-duration measurements

3. Fix NaN calculation:
   ```javascript
   // Bad
   const improvement = (baseline / current - 1) * 100; // NaN if current=0

   // Good
   const improvement = current > 0
     ? ((baseline / current - 1) * 100)
     : 0; // Or display "N/A"
   ```

4. Add error handling:
   ```javascript
   if (benchmarkResult.ops === 0) {
     console.error('Benchmark failed to execute');
     displayError('Benchmark execution failed on this device');
   }
   ```

**Acceptance Criteria:**
- Benchmarks execute and return valid numbers on iOS
- No NaN values displayed
- Clear error messages if benchmark fails
- Performance characteristics documented for iOS

**Estimated Duration:** 2-3 hours

**Deliverables:**
- Fixed benchmark execution logic
- iOS Safari benchmark results
- Updated `IOS_TEST_RESULTS.md` with performance data

---

#### Task R1.6: Fix Soft Delete Compaction [C6]

**Issue:** Compaction doesn't reset tombstones on iOS.

**Priority:** P0

**Agent:** RUST_ENGINEER

**Investigation Steps:**
1. Add logging to compaction:
   ```rust
   pub fn compact(&mut self) {
       #[cfg(target_arch = "wasm32")]
       web_sys::console::log_1(&"Compaction start".into());

       let before = self.tombstone_count;
       // ... compaction logic ...
       let after = self.tombstone_count;

       #[cfg(target_arch = "wasm32")]
       web_sys::console::log_1(&format!("Compaction: {} -> {}", before, after).into());
   }
   ```

2. Test on iOS Safari with console open:
   - Verify compaction logic executes
   - Check if tombstone count resets

3. Check for memory issues:
   - Compaction may trigger iOS memory pressure
   - Monitor WebAssembly.Memory growth during compaction

4. Add error handling:
   ```rust
   pub fn compact(&mut self) -> Result<(), EdgeVecError> {
       // Existing logic with error propagation
   }
   ```

**Acceptance Criteria:**
- Compaction resets tombstone count on iOS
- No lag during compaction (or acceptable performance)
- Memory usage documented
- Unit test for compaction on WASM target

**Estimated Duration:** 3-4 hours

**Deliverables:**
- Fixed compaction logic (if broken)
- Performance profile of compaction on iOS
- Unit test: `#[wasm_bindgen_test] fn test_compact()`

---

### Phase 2: MAJOR Issues

#### Task R2.1: Update Performance Limits in Documentation [M1]

**Issue:** Docs claim 50k vectors safe, but lag starts at 15k on iOS.

**Priority:** P1

**Agent:** WASM_SPECIALIST + BENCHMARK_SCIENTIST

**Steps:**
1. Profile EdgeVec on iOS with varying vector counts:
   - 5k, 10k, 15k, 20k, 25k vectors
   - Measure insert latency, search latency, memory usage

2. Identify performance cliff:
   - Determine maximum "smooth" vector count
   - Document P50/P95/P99 latencies

3. Update `IOS_SAFARI_COMPATIBILITY.md`:
   ```markdown
   ## Tested Performance Limits (iOS Safari 18.2)

   | Mode | Smooth Operation | Max Safe | Notes |
   |:-----|:-----------------|:---------|:------|
   | Quantized (SQ8) | ≤15k vectors | ≤25k | Lag increases above 15k |
   | Float32 | ≤5k vectors | ≤10k | Higher memory pressure |

   **Test Device:** iPhone 15 Pro (iOS 18.2)
   **Test Date:** 2025-12-19
   [FACT - Tested]
   ```

4. Update README.md with realistic limits

**Acceptance Criteria:**
- Actual performance limits documented with evidence
- `IOS_SAFARI_COMPATIBILITY.md` updated
- README.md updated
- Replace all `[HYPOTHESIS]` tags with `[FACT - Tested YYYY-MM-DD]`

**Estimated Duration:** 2-3 hours

**Deliverables:**
- Performance profile report
- Updated compatibility docs
- Updated README.md

---

#### Task R2.2: Update IOS_TEST_RESULTS.md with Actual Findings [M2]

**Issue:** Document still says "PENDING VERIFICATION".

**Priority:** P1

**Agent:** WASM_SPECIALIST

**Steps:**
1. Migrate findings from `W25_DAY3_IOS_TEST_CHECKLIST.md` to `IOS_TEST_RESULTS.md`

2. Replace "Expected" columns with "Actual" results:
   ```markdown
   | Test Case | iOS 18 Actual | Notes |
   |:----------|:--------------|:------|
   | WASM Module Load | ✅ PASS | Loaded successfully |
   | Filter Playground | ❌ FAIL | TypeError: parse_filter_js undefined (FIXED: see R1.2) |
   | ...
   ```

3. Remove "PENDING VERIFICATION" warning:
   ```markdown
   ## Test Environment

   **Status:** ✅ VERIFIED ON DEVICE

   | Parameter | Value |
   |:----------|:------|
   | Testing Method | Physical device testing |
   | Device | iPhone 15 Pro |
   | iOS Version | 18.2 |
   | Test Date | 2025-12-19 |
   ```

4. Add metadata:
   ```markdown
   **Last Updated:** 2025-12-19
   **Tested By:** Human tester + WASM_SPECIALIST
   **Review Status:** Approved by HOSTILE_REVIEWER (after fixes)
   ```

**Acceptance Criteria:**
- All "Expected" replaced with "Actual"
- "PENDING VERIFICATION" removed
- Test results match checklist findings
- Document approved by HOSTILE_REVIEWER

**Estimated Duration:** 1-2 hours

**Deliverables:**
- Updated `docs/mobile/IOS_TEST_RESULTS.md`

---

#### Task R2.3: Complete Test Matrix [M3]

**Issue:** Test cells left blank in checklist.

**Priority:** P1

**Agent:** Human tester (with WASM_SPECIALIST guidance)

**Steps:**
1. Re-test Soft Delete demo sections that were incomplete:
   - Lines 74-75: Fill in "Page loads" and "WASM initializes" results

2. Verify all demos after fixes:
   - Re-run full test suite from W25_DAY3_IOS_TEST_CHECKLIST.md
   - Confirm all previous failures are now fixed

3. Document any remaining issues

**Acceptance Criteria:**
- No blank cells in test matrix
- All tests pass (or documented failures with GitHub issues)

**Estimated Duration:** 1-2 hours (after fixes deployed)

**Deliverables:**
- Completed test matrix
- Updated checklist with all results

---

#### Task R2.4: Verify Error Messages on iOS [M4]

**Issue:** W25.2 error improvements not working on iOS.

**Priority:** P1

**Agent:** WASM_SPECIALIST

**Steps:**
1. Test W25.2 filter error messages on iOS Safari:
   ```javascript
   // In Filter Playground
   const invalidFilter = "price >> 100"; // Invalid operator
   const error = Filter.parse(invalidFilter);
   // Should show: "Invalid operator '>>'. Did you mean '>' or '>='?"
   ```

2. Verify error rendering on mobile:
   - Check font size (readable on mobile)
   - Check color contrast (visible in bright light)
   - Check error position (not cut off)

3. Test contextual suggestions:
   - Verify suggestions render correctly
   - Test touch interaction with suggestions (if clickable)

**Acceptance Criteria:**
- Error messages display correctly on iOS
- Suggestions are readable and helpful
- No rendering issues on mobile

**Estimated Duration:** 1 hour

**Deliverables:**
- iOS Safari screenshots of error messages
- Verification that W25.2 improvements work on mobile

---

### Phase 3: W25.3.4 Completion

#### Task R3.1: Create IOS_KNOWN_ISSUES.md

**Priority:** P1

**Agent:** WASM_SPECIALIST

**Content:**
```markdown
# iOS Safari Known Issues

**Version:** EdgeVec v0.5.3
**Last Updated:** 2025-12-19

## Critical Issues (Fixed in v0.5.4)

### [FIXED] Filter Playground WASM Module Error
- **Issue:** `parse_filter_js` function undefined
- **Affected:** iOS Safari 18.0-18.2
- **Fix:** [PR link]
- **Workaround:** Use desktop Safari until v0.5.4

### [FIXED] Demo Catalog Layout Broken
- **Issue:** Horizontal overflow on iOS
- **Affected:** iPhone/iPad portrait mode
- **Fix:** [PR link]

## Performance Characteristics

### Vector Count Limits
- **Quantized (SQ8):** Smooth up to 15k vectors (tested on iPhone 15 Pro)
- **Float32:** Smooth up to 5k vectors
- **Recommendation:** Use quantized mode on mobile

### Memory Pressure
- iOS Safari terminates tabs using >1 GB memory
- EdgeVec memory usage: ~872 bytes per vector (quantized)

## Workarounds

### Compaction Lag (15k+ vectors)
- **Symptom:** UI freezes during compaction
- **Workaround:** Batch compaction (call compact() every 1000 deletes)
- **Status:** Investigating optimization

## Compatibility Matrix

| iOS Version | Safari | Status |
|:------------|:-------|:-------|
| 18.2 | 18.2 | ✅ Tested |
| 17.4+ | 17.4+ | ⚠️ Expected to work |
| <17.0 | <17.0 | ❌ Unsupported |
```

**Estimated Duration:** 1 hour

**Deliverables:**
- `docs/mobile/IOS_KNOWN_ISSUES.md`

---

#### Task R3.2: Create GitHub Issues for All Bugs

**Priority:** P1

**Agent:** PLANNER

**Issues to Create:**

| Issue | Title | Priority | Assignee |
|:------|:------|:---------|:---------|
| #1 | [iOS] Filter results inconsistent between iOS and desktop | P0 | RUST_ENGINEER |
| #2 | [iOS] Filter Playground WASM module missing parse_filter_js | P0 | WASM_SPECIALIST |
| #3 | [iOS] Filter Playground UI broken on mobile | P0 | WASM_SPECIALIST |
| #4 | [iOS] Demo Catalog horizontal layout broken | P0 | WASM_SPECIALIST |
| #5 | [iOS] Benchmark Dashboard returns NaN | P0 | BENCHMARK_SCIENTIST |
| #6 | [iOS] Soft Delete compaction non-functional | P0 | RUST_ENGINEER |
| #7 | [iOS] Performance degradation at 15k vectors | P1 | BENCHMARK_SCIENTIST |
| #8 | Update IOS_SAFARI_COMPATIBILITY.md with tested limits | P1 | WASM_SPECIALIST |

**Estimated Duration:** 1 hour

**Deliverables:**
- 8 GitHub issues created
- Issues linked to this remediation plan

---

### Phase 4: Process Improvements

#### Task R4.1: Add iOS Safari to CI/CD

**Priority:** P2 (Nice to have for v0.6.0)

**Agent:** WASM_SPECIALIST + RUST_ENGINEER

**Options:**
1. **BrowserStack** (recommended):
   - Automated iOS Safari testing
   - $39/month for open source projects

2. **Sauce Labs**:
   - Similar to BrowserStack
   - Free tier available

3. **GitHub Actions + Playwright**:
   - May support iOS WebKit simulation

**Steps:**
1. Sign up for BrowserStack (or alternative)
2. Add iOS Safari to test matrix:
   ```yaml
   # .github/workflows/wasm-test.yml
   - name: Test on iOS Safari
     uses: browserstack/github-actions@master
     with:
       browsers: 'safari_iphone_15'
       test-command: 'npm run test:wasm'
   ```
3. Configure to fail builds on iOS test failures

**Estimated Duration:** 4-6 hours

**Deliverables:**
- CI/CD config with iOS testing
- Automated test runs on every PR

---

#### Task R4.2: Create HYPOTHESIS Tagging Convention

**Priority:** P2

**Agent:** META_ARCHITECT

**Create:** `docs/architecture/HYPOTHESIS_TAGGING.md`

**Content:**
```markdown
# Hypothesis Tagging Convention

## Tags

### [HYPOTHESIS]
Use for untested predictions:
- Performance estimates before benchmarking
- Browser compatibility before device testing
- Memory limits before profiling

### [FACT - Tested YYYY-MM-DD]
Use for verified claims:
- Actual benchmark results
- Device-tested compatibility
- Profiled memory usage

### [FACT - Source: URL]
Use for externally verified claims:
- MDN documentation references
- WebKit blog posts
- Published standards

## Example

**Before Testing:**
```markdown
EdgeVec supports 50k vectors on iOS Safari. [HYPOTHESIS]
```

**After Testing:**
```markdown
EdgeVec supports 15k vectors smoothly on iOS Safari 18.2. [FACT - Tested 2025-12-19, iPhone 15 Pro]
```

## Enforcement

- HOSTILE_REVIEWER will reject documents with untagged predictions
- All compatibility claims require tags
```

**Estimated Duration:** 1 hour

**Deliverables:**
- Hypothesis tagging convention document
- Updated CLAUDE.md to reference convention

---

## Remediation Timeline

### Day R1 (Immediate)

**Focus:** Critical correctness issues

- [ ] R1.1: Investigate filter inconsistency [C4] (4-6h)
- [ ] R1.2: Fix WASM module [C1] (2-3h)
- [ ] R1.3: Fix Filter Playground UI [C2] (3-4h)

**Total:** ~10-12 hours

---

### Day R2 (Next)

**Focus:** Remaining critical issues + major fixes

- [ ] R1.4: Fix Demo Catalog layout [C3] (2-3h)
- [ ] R1.5: Fix Benchmark Dashboard [C5] (2-3h)
- [ ] R1.6: Fix Soft Delete compaction [C6] (3-4h)
- [ ] R2.1: Update performance limits [M1] (2-3h)

**Total:** ~10-12 hours

---

### Day R3 (Verification)

**Focus:** Documentation and verification

- [ ] R2.2: Update IOS_TEST_RESULTS.md [M2] (1-2h)
- [ ] R2.3: Complete test matrix [M3] (1-2h)
- [ ] R2.4: Verify error messages [M4] (1h)
- [ ] R3.1: Create IOS_KNOWN_ISSUES.md (1h)
- [ ] R3.2: Create GitHub issues (1h)
- [ ] **RETEST ALL** on iPhone 15 Pro (2-3h)

**Total:** ~7-10 hours

---

### Day R4 (Optional - Process Improvements)

**Focus:** CI/CD and process

- [ ] R4.1: Add iOS Safari to CI/CD (4-6h)
- [ ] R4.2: Create HYPOTHESIS tagging convention (1h)

**Total:** ~5-7 hours

---

## Total Effort Estimate

| Phase | Hours |
|:------|:------|
| Phase 1: Critical | 20-24h |
| Phase 2: Major | 5-6h |
| Phase 3: W25.3.4 | 7-10h |
| Phase 4: Process | 5-7h |
| **TOTAL** | **37-47h** |

**Realistic Timeline:** 5-6 work days (accounting for testing cycles)

---

## Success Criteria

W25.3 remediation is complete when:

1. ✅ All 6 CRITICAL issues resolved and verified on iOS
2. ✅ All 4 MAJOR issues resolved
3. ✅ `IOS_TEST_RESULTS.md` updated with actual findings
4. ✅ W25.3.4 completed (`IOS_KNOWN_ISSUES.md` + GitHub issues)
5. ✅ Re-test on iPhone 15 Pro passes all demos
6. ✅ HOSTILE_REVIEWER approves W25.3.3 resubmission

---

## Resubmission Process

1. **Complete all tasks in Phases 1-3**
2. **Re-run full test suite** on iPhone 15 Pro (iOS 18.2)
3. **Update W25_DAY3_IOS_TEST_CHECKLIST.md** with new results
4. **Submit for review:** `/review W25.3`
5. **HOSTILE_REVIEWER re-evaluates** against checklist

---

**Created:** 2025-12-19
**Author:** HOSTILE_REVIEWER
**Status:** [PROPOSED] — Ready for agent assignment
**Next:** Assign tasks to RUST_ENGINEER, WASM_SPECIALIST, BENCHMARK_SCIENTIST
