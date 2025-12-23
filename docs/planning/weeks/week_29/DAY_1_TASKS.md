# Week 29 Day 1: Bundle Optimization + Documentation Polish

**Date:** 2025-12-23
**Focus:** Reduce WASM bundle size and polish documentation
**Estimated Duration:** 10 hours
**Phase:** v0.6.0 Release Preparation
**Dependencies:** Week 28 Complete (APPROVED 2025-12-22)

---

## Executive Summary

Day 1 focuses on two parallel objectives:
1. **Bundle Optimization (W29.1)** — Reduce WASM from 524KB toward <500KB target
2. **Documentation Polish (W29.2)** — Verify links, fix warnings, update badges

---

## Current State

### Bundle Size
```
pkg/edgevec_bg.wasm: 536,826 bytes (524 KB)
Target: < 512,000 bytes (500 KB)
Gap: 24,826 bytes to remove (~5% reduction needed)
```

### Documentation Status
- CHANGELOG.md: Updated for v0.6.0
- README.md: Updated with new features
- API docs: Complete (WASM_INDEX.md, MEMORY.md, FILTER_SYNTAX.md)
- Known issues: 2 minor doc link warnings in cargo doc

---

## Tasks

### W29.1: Bundle Size Optimization (6 hours)

#### W29.1.1: Research wasm-opt and Install Binaryen (1 hour)

**Objective:** Install binaryen tools and research optimal flags for EdgeVec.

**Commands:**

```bash
# Check if wasm-opt is available
wasm-opt --version

# If not installed (Windows):
# Option 1: Download from GitHub releases
# https://github.com/WebAssembly/binaryen/releases

# Option 2: Using npm
npm install -g binaryen

# Option 3: Using cargo (slower)
cargo install wasm-opt
```

**Research Tasks:**
1. Compare optimization flags:
   - `-Oz` — Aggressive size optimization
   - `-Os` — Balanced size optimization
   - `-O3` — Performance optimization
   - `--strip-debug` — Remove debug info
   - `--strip-producers` — Remove metadata

2. Document expected size reductions for each flag

**Deliverables:**
- [ ] `wasm-opt --version` outputs version number
- [ ] Comparison table documented in `docs/planning/weeks/week_29/OPTIMIZATION_RESEARCH.md`

**Acceptance Criteria (Binary):**
- `wasm-opt --version` returns valid version (PASS/FAIL)
- Research document exists with flag comparison table (PASS/FAIL)

---

#### W29.1.2: Baseline Measurement and Cargo.toml Audit (1 hour)

**Objective:** Record baseline metrics and verify release profile optimization.

**Baseline Measurement:**

```bash
# Record current size (pre-optimization)
ls -l pkg/edgevec_bg.wasm
# Expected: 536826 bytes

# Record gzipped size (for web delivery comparison)
gzip -c pkg/edgevec_bg.wasm | wc -c
# Expected: ~200KB gzipped
```

**Cargo.toml Audit:**

Verify these settings exist in `Cargo.toml`:

```toml
[profile.release]
opt-level = 'z'       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization
panic = 'abort'       # Smaller panic handling
strip = true          # Strip symbols
```

**Dependency Audit:**

```bash
# List all dependencies with sizes
cargo tree --edges features

# Check for unused dependencies
cargo +nightly udeps  # (if available)
```

**Deliverables:**
- [ ] Baseline size recorded: ___ bytes
- [ ] Gzipped size recorded: ___ bytes
- [ ] Cargo.toml release profile verified/updated
- [ ] Dependencies audited for unused features

**Acceptance Criteria (Binary):**
- Baseline size recorded in bytes (PASS/FAIL)
- Cargo.toml has all optimization settings (PASS/FAIL)

---

#### W29.1.3: Apply Optimization and Measure Results (2 hours)

**Objective:** Apply wasm-opt optimization and measure size reduction.

**Optimization Sequence:**

```bash
# Step 1: Fresh release build
wasm-pack build --release --target web

# Step 2: Record pre-optimization size
pre_size=$(ls -l pkg/edgevec_bg.wasm | awk '{print $5}')
echo "Pre-optimization: $pre_size bytes"

# Step 3: Strip debug info (always safe)
wasm-opt --strip-debug pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
post_strip=$(ls -l pkg/edgevec_bg.wasm | awk '{print $5}')
echo "After strip-debug: $post_strip bytes"

# Step 4: Apply size optimization
wasm-opt -Oz pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm
post_oz=$(ls -l pkg/edgevec_bg.wasm | awk '{print $5}')
echo "After -Oz: $post_oz bytes"

# Step 5: Calculate reduction
reduction=$((pre_size - post_oz))
percent=$(echo "scale=2; $reduction * 100 / $pre_size" | bc)
echo "Total reduction: $reduction bytes ($percent%)"
```

**Size Thresholds (per Section 4.5 of WEEKLY_TASK_PLAN):**

| Size Range | Action | Status |
|:-----------|:-------|:-------|
| ≤500KB (512,000 bytes) | PASS | Proceed to release |
| 500-520KB | ACCEPT | Document in CHANGELOG |
| 520-550KB | INVESTIGATE | Audit dependencies |
| >550KB | BLOCK | Do not release |

**Deliverables:**
- [ ] Pre-optimization size: ___ bytes
- [ ] Post-strip-debug size: ___ bytes
- [ ] Post-Oz size: ___ bytes
- [ ] Total reduction: ___ bytes (___%)

**Acceptance Criteria (Binary):**
- Post-optimization size ≤540,000 bytes (PASS/FAIL)
- Size reduction achieved (any positive amount) (PASS/FAIL)

---

#### W29.1.4: Browser Functional Test and Documentation (2 hours)

**Objective:** Verify optimized WASM works in browsers and document results.

**Browser Testing:**

```bash
# Start local server
cd wasm/examples
python -m http.server 8080
# OR
npx serve .
```

**Test Checklist:**

| Browser | Version | Test | Expected |
|:--------|:--------|:-----|:---------|
| Chrome | 120+ | Load demo | No console errors |
| Chrome | 120+ | Insert 100 vectors | Success toast |
| Chrome | 120+ | Search | Results displayed |
| Firefox | 121+ | Load demo | No console errors |
| Firefox | 121+ | Search with filter | Results filtered |

**Documentation Updates:**

1. Update CHANGELOG.md with bundle size info:
```markdown
### Changed
- WASM bundle optimized: XXX KB → XXX KB (XX% reduction)
```

2. Document optimization flags used:
```markdown
### Technical Notes
- Bundle optimized with wasm-opt -Oz and --strip-debug
```

**Deliverables:**
- [ ] Chrome test passed (no console errors)
- [ ] Firefox test passed (no console errors)
- [ ] CHANGELOG.md updated with size info
- [ ] Bundle size documented

**Acceptance Criteria (Binary):**
- v060_cyberpunk_demo.html loads in Chrome without JS errors (PASS/FAIL)
- v060_cyberpunk_demo.html loads in Firefox without JS errors (PASS/FAIL)
- CHANGELOG.md contains bundle size information (PASS/FAIL)

---

### W29.2: Documentation Polish (4 hours)

#### W29.2.1: Verify All Documentation Links (1 hour)

**Objective:** Ensure no broken links in documentation.

**Link Verification Script:**

```bash
# Find all markdown links
grep -rn '\]\(.*\.md' docs/ | while read line; do
  file=$(echo "$line" | cut -d: -f1)
  link=$(echo "$line" | grep -oP '\]\(\K[^)]+')

  # Resolve relative path
  dir=$(dirname "$file")
  target="$dir/$link"

  if [ ! -f "$target" ]; then
    echo "BROKEN: $file -> $link"
  fi
done
```

**Known Links to Verify:**

| Source | Link | Expected |
|:-------|:-----|:---------|
| docs/api/MEMORY.md | ../guides/BINARY_QUANTIZATION.md | EXISTS |
| docs/api/MEMORY.md | ../PERFORMANCE_TUNING.md | EXISTS |
| docs/api/WASM_INDEX.md | ./FILTER_SYNTAX.md | EXISTS |
| README.md | docs/TUTORIAL.md | EXISTS |

**Deliverables:**
- [ ] All documentation links verified
- [ ] Broken links fixed (if any)

**Acceptance Criteria (Binary):**
- Link verification script finds 0 broken links (PASS/FAIL)

---

#### W29.2.2: Run `cargo doc` and Fix Warnings (1 hour)

**Objective:** Generate documentation with zero warnings.

**Commands:**

```bash
# Generate docs and capture warnings
cargo doc --no-deps 2>&1 | tee doc_output.txt

# Count warnings
grep -c "warning" doc_output.txt
# Target: 0

# If warnings exist, fix them:
# Common fixes:
# - Escape brackets in doc comments: `[update]` → `\[update\]`
# - Use full paths: `insert` → [`Self::insert`]
# - Remove broken intra-doc links
```

**Known Warnings (from Week 28 review):**
- `src/metadata/store.rs:157` — Unresolved link to `update`
- `src/metadata/store.rs:221` — Unresolved link to `insert`

**Deliverables:**
- [ ] `cargo doc` runs successfully
- [ ] Warning count: 0

**Acceptance Criteria (Binary):**
- `cargo doc 2>&1 | grep -c "warning"` returns 0 (PASS/FAIL)

---

#### W29.2.3: Update README Badges (0.5 hours)

**Objective:** Ensure all badges show correct v0.6.0 version.

**Badges to Update:**

```markdown
[![Crates.io](https://img.shields.io/crates/v/edgevec.svg)](https://crates.io/crates/edgevec)
[![npm](https://img.shields.io/npm/v/edgevec.svg)](https://www.npmjs.com/package/edgevec)
[![docs.rs](https://docs.rs/edgevec/badge.svg)](https://docs.rs/edgevec)
```

**Verification:**
- Badges auto-update after publish
- Ensure badge URLs are correct format
- Verify crate/package names match

**Deliverables:**
- [ ] Badge URLs verified correct
- [ ] No hardcoded version numbers in badges

**Acceptance Criteria (Binary):**
- README.md badges use dynamic version URLs (PASS/FAIL)

---

#### W29.2.4: Review CHANGELOG Completeness (0.5 hours)

**Objective:** Verify CHANGELOG documents all v0.6.0 features.

**RFC-002 Feature Checklist:**

| Feature | CHANGELOG Entry | Status |
|:--------|:----------------|:-------|
| insertWithMetadata() | Added | [ ] |
| searchFiltered() | Added | [ ] |
| getMetadata() | Added | [ ] |
| searchBQ() | Added | [ ] |
| searchBQRescored() | Added | [ ] |
| searchHybrid() | Added | [ ] |
| getMemoryPressure() | Added | [ ] |
| setMemoryConfig() | Added | [ ] |
| canInsert() | Added | [ ] |
| Performance metrics table | Added | [ ] |
| Migration guide section | Added | [ ] |

**Deliverables:**
- [ ] All 11 items checked in CHANGELOG

**Acceptance Criteria (Binary):**
- All RFC-002 features listed in CHANGELOG (PASS/FAIL)

---

#### W29.2.5: Proofread API Docs (1 hour)

**Objective:** Manual review for typos and clarity.

**Files to Review:**

| File | Review Focus |
|:-----|:-------------|
| README.md | Quick start examples work |
| docs/api/WASM_INDEX.md | Method signatures accurate |
| docs/api/MEMORY.md | Threshold values correct |
| docs/api/FILTER_SYNTAX.md | Operator examples accurate |

**Review Checklist:**
- [ ] No obvious typos
- [ ] Code examples have correct syntax
- [ ] Version numbers are 0.6.0 (not 0.5.x)
- [ ] Links work when clicked

**Deliverables:**
- [ ] API docs reviewed
- [ ] No critical errors found

**Acceptance Criteria (Binary):**
- Manual review completed with checklist (PASS/FAIL)

---

## Exit Criteria for Day 1

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| wasm-opt installed | `wasm-opt --version` | [ ] |
| Bundle optimized | Size recorded post-optimization | [ ] |
| Bundle ≤540KB | `ls -l pkg/edgevec_bg.wasm` | [ ] |
| Browser tests pass | Chrome + Firefox | [ ] |
| Doc links verified | Link check script | [ ] |
| cargo doc clean | 0 warnings | [ ] |
| CHANGELOG complete | Feature checklist | [ ] |

---

## Handoff

After completing Day 1:

**Artifacts:**
- Optimized WASM bundle
- Updated CHANGELOG.md with size info
- Clean cargo doc output
- Browser test verification

**Status:** PENDING Day 2

**Next:** Day 2 — Internal Files Cleanup + Final Testing

---

*Agent: WASM_SPECIALIST + DOCWRITER*
*Status: [PROPOSED]*
*Date: 2025-12-22*
*Dependencies: Week 28 (APPROVED)*
