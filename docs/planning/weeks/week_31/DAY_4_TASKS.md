# Week 31 Day 4: Pre-Release Testing

**Date:** 2025-12-30
**Focus:** Comprehensive testing before release
**Estimated Duration:** 3 hours
**Priority:** P0 — Critical quality gate

---

## Objectives

1. Run complete test suite
2. Verify clippy compliance
3. Test WASM build
4. Verify bundle size
5. Get HOSTILE_REVIEWER pre-release approval

---

## Tasks

### W31.4.1: Run Complete Test Suite

**Duration:** 0.5 hours
**Agent:** TEST_ENGINEER

**Commands:**

```bash
# Full library tests
cargo test --lib

# All tests including integration
cargo test --all

# With verbose output for CI
cargo test --all -- --nocapture
```

**Expected Results:**
- 677+ tests passing
- 0 failures
- 0 ignored (unless documented)

**If Tests Fail:**
1. Document failing test
2. Fix immediately (P0)
3. Re-run full suite
4. Do NOT proceed to release if tests fail

**Acceptance Criteria:**
- [ ] All tests pass
- [ ] Test count documented
- [ ] No new warnings

---

### W31.4.2: Run Clippy Strict Mode

**Duration:** 0.5 hours
**Agent:** TEST_ENGINEER

**Commands:**

```bash
# Strict clippy
cargo clippy -- -D warnings

# Extra pedantic (optional)
cargo clippy -- -D warnings -W clippy::pedantic

# Format check
cargo fmt -- --check
```

**Expected Results:**
- 0 warnings
- 0 errors
- Format clean

**Common Issues to Check:**
- `cast_precision_loss` — Should have `#[allow]` with comment
- `must_use_candidate` — Public functions should have `#[must_use]`
- `uninlined_format_args` — Use `format!("{x}")` not `format!("{}", x)`

**Acceptance Criteria:**
- [ ] Clippy clean
- [ ] Format clean
- [ ] All `#[allow]` have justification comments

---

### W31.4.3: Run WASM Tests

**Duration:** 0.5 hours
**Agent:** TEST_ENGINEER

**Commands:**

```bash
# Headless browser tests
wasm-pack test --headless --chrome

# Firefox (optional)
wasm-pack test --headless --firefox
```

**Manual Browser Test:**
1. Start local server: `npx serve wasm/examples`
2. Open `http://localhost:3000/simd_test.html`
3. Verify all tests pass
4. Check console for errors

**Acceptance Criteria:**
- [ ] WASM tests pass
- [ ] No console errors
- [ ] SIMD detection works

---

### W31.4.4: Build Release WASM Bundle

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Commands:**

```bash
# Clean previous builds
cargo clean

# Release build with SIMD
wasm-pack build --release --target web --out-dir pkg

# Optimize with wasm-opt (if available)
wasm-opt pkg/edgevec_bg.wasm -O3 -o pkg/edgevec_bg.wasm
```

**Verify Build:**
```bash
# Check file sizes
ls -la pkg/

# Verify SIMD instructions present
wasm2wat pkg/edgevec_bg.wasm | grep -c "v128"
# Expected: 200+ SIMD instructions
```

**Acceptance Criteria:**
- [ ] Release build completes
- [ ] SIMD instructions present
- [ ] No build warnings

---

### W31.4.5: Verify Bundle Size

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Target:** <500KB (uncompressed), <250KB (gzipped)

**Check Size:**

```bash
# Raw size
ls -la pkg/edgevec_bg.wasm

# Gzipped size (approximate)
gzip -c pkg/edgevec_bg.wasm | wc -c
```

**Size Breakdown (Expected):**

| File | Size | Notes |
|:-----|:-----|:------|
| `edgevec_bg.wasm` | ~477 KB | Main WASM binary |
| `edgevec.js` | ~10 KB | JavaScript glue |
| `edgevec.d.ts` | ~15 KB | TypeScript definitions |
| **Total** | ~500 KB | Under target |

**If Over Budget:**
1. Check for unnecessary features
2. Run `wasm-opt -Oz` for size optimization
3. Consider `--features` flags to exclude unused code

**Acceptance Criteria:**
- [ ] Bundle <500KB uncompressed
- [ ] Bundle <250KB gzipped
- [ ] Size documented for release notes

---

### W31.4.6: HOSTILE_REVIEWER Pre-Release Check

**Duration:** 0.5 hours
**Agent:** HOSTILE_REVIEWER

**Checklist:**

| Item | Verification | Status |
|:-----|:-------------|:-------|
| All tests pass | 677+ green | [ ] |
| Clippy clean | 0 warnings | [ ] |
| WASM builds | Release mode | [ ] |
| Bundle size | <500KB | [ ] |
| CHANGELOG complete | v0.7.0 section | [ ] |
| @jsonMartin credited | In CHANGELOG + README | [ ] |
| Demo pages v0.7.0 | All updated | [ ] |
| No TODO/FIXME | No release blockers | [ ] |

**Blockers Check:**

```bash
# Search for blocking TODOs
grep -rn "TODO\|FIXME\|XXX\|HACK" src/ | grep -v "test"
```

**Security Check:**
- No credentials in code
- No internal URLs exposed
- No debug code in release

**Verdict Options:**
- ✅ **APPROVE** — Proceed to release
- ⚠️ **CONDITIONAL** — Fix issues first
- ❌ **REJECT** — Major issues found

**Acceptance Criteria:**
- [ ] HOSTILE_REVIEWER approval received
- [ ] No blocking issues
- [ ] Ready for release

---

## Day 4 Exit Criteria

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Tests pass | 677+ green | [ ] |
| Clippy clean | 0 warnings | [ ] |
| WASM tests pass | Chrome/Firefox | [ ] |
| Release build | Completes successfully | [ ] |
| Bundle size | <500KB | [ ] |
| HOSTILE_REVIEWER | APPROVED | [ ] |

---

## Go/No-Go Decision

**At end of Day 4:**

| Condition | Go | No-Go |
|:----------|:---|:------|
| Tests passing | ✅ Proceed | ❌ Fix first |
| Clippy clean | ✅ Proceed | ❌ Fix first |
| Bundle under 500KB | ✅ Proceed | ⚠️ Document |
| HOSTILE approval | ✅ Proceed | ❌ Address issues |
| Docs complete | ✅ Proceed | ⚠️ Can release anyway |

**If Go:** Proceed to Day 5 (Release Execution)
**If No-Go:** Fix issues, repeat Day 4 checks

---

**Day 4 Total:** 3 hours
**Agent:** TEST_ENGINEER, WASM_SPECIALIST, HOSTILE_REVIEWER
