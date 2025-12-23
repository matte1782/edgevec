# Week 29: Buffer, Polish & v0.6.0 Release [REVISED v3.1]

**Date:** 2025-12-23 to 2025-12-29
**Focus:** Final polish, bundle optimization, documentation fixes, v0.6.0 release
**Estimated Duration:** 24 hours (3 days focused work)
**Phase:** RFC-002 Completion + Release
**Previous:** Week 28 — WASM Bindings + Cyberpunk Demo (APPROVED)
**Gate File:** `.claude/GATE_W28_COMPLETE.md` (created 2025-12-22)
**Revision:** v3.1 — Minor fix: Added missing Dev.to subtask

---

## Changes Made (v3.1)

**v3.1 Fix (from v3 approval review):**

| ID | Finding | Resolution |
|:---|:--------|:-----------|
| **m1-v3** | Dev.to article in deliverables but no subtask | Added W29.6.4 "Draft Dev.to article" (0.75h), renumbered W29.6.5-6 |

**v3.0 Resolutions (all retained):**

| ID | Finding | Resolution |
|:---|:--------|:-----------|
| **C1** | Missing `.claude/GATE_W28_COMPLETE.md` | Created gate file based on `2025-12-22_W28_GATE_REVIEW.md` |
| **C2** | Day 2 header (10h) vs tasks (6h) mismatch | Fixed header to "6 hours" |
| **C3** | W29.5/W29.6 hours inconsistent | Reconciled: 4h each in both Section 2 and Day 3 |
| **C4** | Total hours mismatch (24h vs 20h) | Fixed: Day 1 (10h) + Day 2 (6h) + Day 3 (8h) = 24h |
| **M1** | Implementation Plan reference undefined | Added explicit path: `docs/rfcs/RFC-002_IMPLEMENTATION_PLAN.md` Section 7.2 |
| **m1** | Proofreading criteria subjective | Added scope: files in `docs/api/*.md` and `README.md` |
| **m2** | GitHub Pages fallback missing | Added fallback to Netlify/static screenshots |

---

## 1. Executive Summary

Week 29 is the **final release sprint** for EdgeVec v0.6.0. The implementation is complete (RFC-002 Phases 1-3 done). This week focuses on:

1. **Bundle Optimization** — Reduce WASM size from 524KB toward <500KB target
2. **Documentation Cleanup** — Fix any remaining issues, verify all links
3. **Internal Files Cleanup** — Remove .claude/, .cursor/ from git before release
4. **Release Preparation** — crates.io + npm publish, demo deployment
5. **Launch Preparation** — Draft launch posts for HN, Reddit, social media

**RFC-002 Status:** FULLY IMPLEMENTED (all phases complete)
**Week 28 Gate:** PASSED (2025-12-22, documented in `.claude/GATE_W28_COMPLETE.md`)
**ROADMAP Reference:** `docs/planning/ROADMAP.md` Section "Phase 7: v0.6.0" → Week 29

---

## 2. Week 29 Objectives

| ID | Objective | Hours | Risk | Agent | ROADMAP Ref |
|:---|:----------|:------|:-----|:------|:------------|
| W29.1 | Bundle Size Optimization | 6 | Medium | WASM_SPECIALIST | Week 29, line 159-162 |
| W29.2 | Documentation Polish | 4 | Low | DOCWRITER | Week 29, line 195-201 |
| W29.3 | Internal Files Cleanup | 2 | Low | RUST_ENGINEER | Week 29, line 166-193 |
| W29.4 | Final Testing & QA | 4 | Low | TEST_ENGINEER | Week 29, line 195-201 |
| W29.5 | Release Execution | 4 | Medium | RUST_ENGINEER | Week 29, line 159-162 |
| W29.6 | Launch Content Prep | 4 | Low | DOCWRITER | Week 29, line 298-324 |

**Total:** 24 hours (3 days)

**Hours Reconciliation (C4 Resolution):**
- Day 1: W29.1 (6h) + W29.2 (4h) = 10h
- Day 2: W29.3 (2h) + W29.4 (4h) = 6h
- Day 3: W29.5 (4h) + W29.6 (4h) = 8h
- **Grand Total: 10 + 6 + 8 = 24 hours** ✅

---

## 2.1 Contingency Clarification

**ROADMAP.md Week 29 specifies:**
> "Week 29: Buffer & Release (22 hours contingency)"

**Current Status:**
- RFC-002 Phases 1-3 completed **without using contingency hours**
- No unforeseen integration issues occurred
- Performance targets achieved (6.9x SIMD speedup, 0.936 recall)

**Resolution:**
The 22-hour contingency buffer was **NOT consumed** by Phase 1-3 overflow. This contingency is now available for:
1. Bundle optimization (if complex, absorbs extra hours)
2. Launch content polish
3. Post-launch bug fixes

**Hour Allocation:**
- Base Week 29 tasks: 24 hours
- Available contingency: 22 hours
- Total available: 46 hours
- Expected usage: 24-30 hours (leaving buffer for post-launch)

---

## 3. Daily Task Breakdown

### Day 1: Bundle Optimization + Documentation (10 hours)

#### W29.1: Bundle Size Optimization (6 hours)

**Current State:**
```
pkg/edgevec_bg.wasm: 536,826 bytes (524 KB)
Target: < 512,000 bytes (500 KB)
Gap: 24,826 bytes to remove
```

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.1.1 | Research wasm-opt flags and install binaryen | 1 | `wasm-opt --version` outputs version number AND comparison table of `-Oz`, `-Os`, `-O3` documented |
| W29.1.2 | Baseline measurement and Cargo.toml audit | 1 | Current size recorded via `ls -l pkg/edgevec_bg.wasm` AND Cargo.toml release profile verified |
| W29.1.3 | Apply optimization and measure results | 2 | Optimization applied AND new size recorded AND delta calculated |
| W29.1.4 | Browser functional test and documentation | 2 | Demo loads in Chrome AND Firefox AND size documented in CHANGELOG |

**Acceptance Criteria (Binary):**
- [ ] `wasm-opt --version` returns valid version (PASS/FAIL)
- [ ] `ls -l pkg/edgevec_bg.wasm` shows size in bytes (recorded value)
- [ ] Post-optimization size ≤ 512,000 bytes OR fallback documented per Section 4.5
- [ ] `v060_cyberpunk_demo.html` loads without JavaScript errors in Chrome DevTools Console

#### W29.2: Documentation Polish (4 hours)

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.2.1 | Verify all documentation links | 1 | `grep -rn '\]\(.*\.md' docs/` output has 0 broken links |
| W29.2.2 | Run `cargo doc` and fix warnings | 1 | `cargo doc 2>&1 \| grep -c "warning"` returns 0 |
| W29.2.3 | Update README badges for v0.6.0 | 0.5 | Badge shows "v0.6.0" not "v0.5.x" |
| W29.2.4 | Review CHANGELOG completeness | 0.5 | All RFC-002 features from `docs/rfcs/RFC-002_IMPLEMENTATION_PLAN.md` Section 7.2 are listed (M1 Resolution) |
| W29.2.5 | Proofread API docs for errors | 1 | Zero spelling errors found in `docs/api/*.md` and `README.md` via manual review (m1 Resolution) |

**ROADMAP Reference:** Section "Documentation Checklist for v0.6.0" lines 195-201

---

### Day 2: Cleanup + Final Testing (6 hours) [C2 Resolution — Fixed Header]

#### W29.3: Internal Files Cleanup (2 hours)

**Exact Files to Remove (from ROADMAP.md lines 166-193):**

| File/Folder | Action | Verification |
|:------------|:-------|:-------------|
| `.claude/` | `git rm -r --cached` | `git status` shows "deleted: .claude/*" |
| `.cursor/` | `git rm -r --cached` | `git status` shows "deleted: .cursor/*" |
| `.cursorrules` | `git rm --cached` | `git status` shows "deleted: .cursorrules" |
| `CLAUDE.md` | `git rm --cached` | `git status` shows "deleted: CLAUDE.md" |

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.3.1 | DRY-RUN: List files to remove | 0.5 | `git status --porcelain` output reviewed, no unexpected files |
| W29.3.2 | Execute git rm --cached for internal files | 0.5 | `git status` shows 4 items staged for deletion |
| W29.3.3 | Update .gitignore with internal patterns | 0.25 | `.gitignore` contains all 4 patterns |
| W29.3.4 | Verify local copies preserved | 0.25 | `ls -la .claude/ CLAUDE.md` shows files still exist locally |
| W29.3.5 | Commit cleanup changes | 0.5 | `git log -1 --oneline` shows cleanup commit |

**Git Commands (with dry-run verification):**
```bash
# Step 1: DRY-RUN verification (CRITICAL — do NOT skip)
git status --porcelain | grep -E "^\?\?" | head -20  # Review untracked files
ls -la .claude/ .cursor/ .cursorrules CLAUDE.md     # Verify files exist locally

# Step 2: Remove from git tracking (keeps local copies)
git rm -r --cached .claude/
git rm -r --cached .cursor/
git rm --cached .cursorrules
git rm --cached CLAUDE.md

# Step 3: Add to .gitignore
cat >> .gitignore << 'EOF'

# Internal development files (removed from git before v0.6.0 release)
.claude/
.cursor/
.cursorrules
CLAUDE.md
EOF

# Step 4: Verify local copies still exist
ls -la .claude/ CLAUDE.md  # Must show files still on disk

# Step 5: Commit cleanup
git commit -m "chore: remove internal development files before v0.6.0 release

Per ROADMAP.md Week 29 requirements:
- .claude/ (agent prompts, gate files)
- .cursor/ (Cursor IDE commands)
- .cursorrules (development rules)
- CLAUDE.md (project instructions)

Files remain locally but are excluded from public repository."
```

#### W29.4: Final Testing & QA (4 hours)

**Browser Compatibility Matrix:**

| Browser | Version | Platform | Test Scope |
|:--------|:--------|:---------|:-----------|
| Chrome | 120+ | Windows/Mac | Full demo, DevTools memory |
| Firefox | 121+ | Windows/Mac | Full demo, filter syntax |
| Safari | 17+ | Mac | Full demo, iOS compat check |
| Edge | 120+ | Windows | Basic functionality only |

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.4.1 | Run full test suite | 1 | `cargo test 2>&1 \| tail -1` shows "test result: ok" |
| W29.4.2 | Run clippy strict mode | 0.5 | `cargo clippy -- -D warnings 2>&1 \| grep -c "error"` returns 0 |
| W29.4.3 | Test Chrome | 0.5 | Demo loads, search works, no console errors |
| W29.4.4 | Test Firefox | 0.5 | Demo loads, search works, no console errors |
| W29.4.5 | Test Safari | 0.5 | Demo loads, search works, touch works |
| W29.4.6 | Verify npm package.json | 0.5 | `grep '"version"' pkg/package.json` shows "0.6.0" |
| W29.4.7 | Dry-run cargo publish | 0.5 | `cargo publish --dry-run 2>&1 \| tail -1` shows success |

---

### Day 3: Release + Launch Prep (8 hours) [C3 Resolution — Hours Matched]

**Note:** Section 2 allocates 4h each for W29.5 and W29.6. Day 3 breakdown now matches.

#### W29.5: Release Execution (4 hours) [C3 Resolution]

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.5.1 | Final version verification | 0.5 | `grep "^version" Cargo.toml` shows "0.6.0" |
| W29.5.2 | Publish to crates.io | 1 | `cargo publish` exits 0 AND crate page shows v0.6.0 |
| W29.5.3 | Publish to npm | 1 | `npm publish` exits 0 AND npm page shows v0.6.0 |
| W29.5.4 | Create and push git tag | 0.5 | `git tag -l v0.6.0` shows tag exists |
| W29.5.5 | Create GitHub release | 1 | Release page shows v0.6.0 with CHANGELOG notes |

#### W29.6: Launch Content Preparation (4 hours) [C3 Resolution]

**Deliverables:**

| Platform | File | Word Count | Format |
|:---------|:-----|:-----------|:-------|
| Hacker News | `docs/marketing/LAUNCH_HN.md` | 200-300 | Show HN post |
| Reddit r/rust | `docs/marketing/LAUNCH_REDDIT.md` | 150-200 | Self-post |
| Twitter/X | `docs/marketing/LAUNCH_TWITTER.md` | 5 tweets | Thread |
| Dev.to | `docs/marketing/LAUNCH_DEVTO.md` | 500-800 | Technical article |

| ID | Task | Hours | Acceptance Criteria |
|:---|:-----|:------|:--------------------|
| W29.6.1 | Draft Show HN post | 0.5 | File exists at `docs/marketing/LAUNCH_HN.md` AND word count 200-300 |
| W29.6.2 | Draft Reddit r/rust post | 0.5 | File exists at `docs/marketing/LAUNCH_REDDIT.md` AND word count 150-200 |
| W29.6.3 | Draft Twitter thread | 0.25 | File exists at `docs/marketing/LAUNCH_TWITTER.md` AND contains 5 tweets |
| W29.6.4 | Draft Dev.to article | 0.75 | File exists at `docs/marketing/LAUNCH_DEVTO.md` AND word count 500-800 (m1 Resolution) |
| W29.6.5 | Record 60-second demo GIF | 1 | File exists at `wasm/examples/demo.gif` AND file size >500KB |
| W29.6.6 | Deploy demo to GitHub Pages | 1 | URL accessible: `https://[username].github.io/edgevec/` |

**Fallback for W29.6.5/W29.6.6 (m2 Resolution):**
- If GIF recording fails: Use 3 static screenshots instead
- If GitHub Pages deployment fails: Deploy to Netlify or Vercel

**Key Messaging (from ROADMAP.md lines 326-333):**
1. "Vector search in your browser, no server required"
2. "32x memory reduction with Binary Quantization"
3. "Sub-10ms search on 100k vectors in WASM"
4. "MIT licensed, works offline, Safari/iOS compatible"

---

## 4. Technical Details

### 4.1 Estimation Methodology

All estimates follow the **3x Rule** per CLAUDE.md Section 3.3:
```
Final Estimate = Base Estimate × 3
```

**Week 29 Adjustments:**
- Many tasks are polish/verification, not new development
- 3x rule applied to release execution (W29.5)
- Contingency buffer available (22 hours per Section 2.1) absorbs variance

### 4.2 ROADMAP Cross-References

| Task | ROADMAP.md Section | Line Numbers |
|:-----|:-------------------|:-------------|
| W29.1 Bundle Opt | Week 29 objectives | 159-162 |
| W29.2 Docs | Documentation Checklist | 195-201 |
| W29.3 Cleanup | Pre-Release Cleanup | 166-193 |
| W29.4 Testing | Success Metrics | 203-210 |
| W29.5 Release | Version History + future | 336-347 |
| W29.6 Launch | Publication Strategy | 262-324 |

### 4.3 wasm-opt Optimization Strategy

**Research-Based Approach:**

| Flag | Purpose | Expected Reduction | Use Case |
|:-----|:--------|:-------------------|:---------|
| `-Oz` | Aggressive size | 10-15% | Maximum size reduction |
| `-Os` | Balanced size | 8-12% | Size + some performance |
| `-O3` | Performance | 5-8% | Maximum performance |
| `--strip-debug` | Remove debug info | 5-10% | Always apply first |
| `--strip-producers` | Remove metadata | 1-2% | Optional cleanup |

**Recommended Sequence:**
```bash
# Step 1: Strip debug info (always safe)
wasm-opt --strip-debug pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm

# Step 2: Apply size optimization
wasm-opt -Oz pkg/edgevec_bg.wasm -o pkg/edgevec_bg.wasm

# Step 3: Measure result
ls -l pkg/edgevec_bg.wasm
```

**Rationale:**
- `-Oz` is appropriate for EdgeVec because:
  - Target is <500KB (size-constrained)
  - Operations are already vectorized (SIMD)
  - Browser caching mitigates load-time impact
- `--strip-debug` should ALWAYS be applied first
- `-Os` fallback if `-Oz` causes runtime issues

### 4.4 Bundle Size Measurement Methodology

**Exact Measurement Protocol:**

```bash
# Baseline (before optimization)
ls -l pkg/edgevec_bg.wasm | awk '{print $5}'  # Output: bytes
# Current: 536826 bytes (524 KB)

# After wasm-opt
ls -l pkg/edgevec_bg.wasm | awk '{print $5}'  # Target: <512000 bytes

# Verification formula
size_bytes=$(ls -l pkg/edgevec_bg.wasm | awk '{print $5}')
if [ $size_bytes -le 512000 ]; then
  echo "PASS: $size_bytes bytes (<500KB)"
else
  echo "FALLBACK: $size_bytes bytes — see Section 4.5"
fi
```

**Target:**
- PASS: ≤512,000 bytes (500 KB)
- ACCEPTABLE: ≤540,000 bytes (527 KB) with documented rationale
- FAIL: >540,000 bytes — requires investigation

### 4.5 Bundle Optimization Fallback Strategy

**If <500KB NOT achieved:**

| Size Range | Action | Documentation |
|:-----------|:-------|:--------------|
| 500-520KB | ACCEPT with note | Add CHANGELOG note: "Bundle slightly larger due to full v0.6.0 feature set" |
| 520-550KB | INVESTIGATE | Audit dependencies, consider feature flags |
| >550KB | BLOCK | Do not release until resolved |

**Fallback Options (if needed):**
1. **Feature flag metadata module** — Make metadata storage opt-in (~10KB reduction)
2. **Lazy load BQ module** — Split BQ into separate chunk (~15KB reduction)
3. **Remove dev dependencies** — Audit for accidental inclusion (~5KB reduction)

**Decision Tree:**
```
size <= 500KB?  → PASS: Proceed to release
    ↓ NO
size <= 520KB?  → ACCEPT: Document in CHANGELOG, proceed
    ↓ NO
size <= 550KB?  → INVESTIGATE: Apply fallback options
    ↓ NO
BLOCK: Do not release, escalate to architect review
```

### 4.6 Cargo.toml Release Profile

**Verify/Update:**
```toml
[profile.release]
opt-level = 'z'       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization (slower compile)
panic = 'abort'       # Smaller panic handling
strip = true          # Strip symbols
```

### 4.7 Browser Compatibility Matrix

| Browser | Version | Platform | Test Type | Verification |
|:--------|:--------|:---------|:----------|:-------------|
| Chrome | 120+ | Windows 11 | Full | DevTools Console shows 0 errors |
| Chrome | 120+ | macOS | Full | All demo features work |
| Firefox | 121+ | Windows 11 | Full | Filter syntax works |
| Firefox | 121+ | macOS | Full | All demo features work |
| Safari | 17+ | macOS | Full | Touch targets 44px+ |
| Safari | 17+ | iOS 17+ | Smoke | Demo loads, basic search works |
| Edge | 120+ | Windows 11 | Smoke | Demo loads |

**Test Checklist:**
- [ ] Demo page loads without JS errors
- [ ] Insert 100 vectors → success toast shown
- [ ] Search returns results with correct ordering
- [ ] BQ search shows speedup metrics
- [ ] Filter syntax `category = "test"` works
- [ ] Memory pressure indicator updates
- [ ] Particle/Matrix effects render (not Safari iOS)
- [ ] Touch targets ≥44px on mobile

---

## 5. Rollback Procedures

### 5.1 Pre-Release Abort

**Trigger:** Critical bug found during Day 2 testing

**Procedure:**
1. Document bug in `docs/reviews/W29_ABORT_[timestamp].md`
2. DO NOT publish to crates.io or npm
3. Create bug fix branch
4. Re-run test suite after fix
5. Resume release process

### 5.2 Post-crates.io Abort

**Trigger:** Critical bug found after crates.io publish but before npm

**Procedure:**
1. Yank crate: `cargo yank --version 0.6.0`
2. Document incident
3. Create v0.6.1 hotfix
4. Resume release with v0.6.1

### 5.3 Full Rollback

**Trigger:** Critical bug found after both crates.io and npm publish

**Procedure:**
1. Yank crate: `cargo yank --version 0.6.0`
2. Deprecate npm: `npm deprecate edgevec@0.6.0 "Critical bug - use 0.6.1"`
3. Create GitHub issue documenting bug
4. Create v0.6.1 hotfix immediately
5. Update all launch posts with correction

---

## 6. Risk Analysis

### 6.1 Medium Risk: Bundle Size

**Risk:** wasm-opt may not achieve <500KB
**Probability:** 30%
**Impact:** Low (cosmetic target, Section 4.5 fallback exists)
**Mitigation:** Fallback strategy defined, accept up to 520KB with documentation

### 6.2 Low Risk: Publishing Issues

**Risk:** crates.io or npm publish fails
**Probability:** 5%
**Impact:** Medium
**Mitigation:** Dry-run before actual publish, credentials verified

### 6.3 Low Risk: Browser Compatibility

**Risk:** Demo fails in specific browser
**Probability:** 10%
**Impact:** Low
**Mitigation:** Full browser matrix defined, Safari iOS already tested Week 28

### 6.4 Low Risk: Deployment Failures (m2 Resolution)

**Risk:** GitHub Pages deployment or GIF recording fails
**Probability:** 5%
**Impact:** Low
**Mitigation:**
- GitHub Pages fallback: Deploy to Netlify or Vercel
- GIF fallback: Use 3 static screenshots with captions

---

## 7. Exit Criteria

### 7.1 Release Criteria (All MUST Pass)

| Criterion | Verification Command | Expected Output |
|:----------|:---------------------|:----------------|
| All tests pass | `cargo test 2>&1 \| tail -1` | "test result: ok" |
| Clippy clean | `cargo clippy -- -D warnings 2>&1 \| grep -c error` | "0" |
| WASM builds | `wasm-pack build --release` | Exit code 0 |
| Bundle size | `ls -l pkg/edgevec_bg.wasm \| awk '{print $5}'` | ≤540000 (with fallback) |
| Internal files removed | `git status \| grep -c "deleted.*\.claude"` | "1" (or more) |
| Chrome test | Manual | Demo works, 0 console errors |
| Firefox test | Manual | Demo works |
| Safari test | Manual | Demo works |
| crates.io | `cargo publish` | Exit code 0 |
| npm | `npm publish` | Exit code 0 |
| GitHub release | Manual | Release page exists |

### 7.2 Launch Prep Criteria

| Criterion | Verification | Expected |
|:----------|:-------------|:---------|
| HN post | `wc -w docs/marketing/LAUNCH_HN.md` | 200-300 words |
| Reddit post | `wc -w docs/marketing/LAUNCH_REDDIT.md` | 150-200 words |
| Twitter thread | `grep -c "Tweet" docs/marketing/LAUNCH_TWITTER.md` | 5 |
| Demo GIF | `ls -l wasm/examples/demo.gif` | File exists, >500KB |
| Demo deployed | `curl -I https://[user].github.io/edgevec/` | HTTP 200 |

---

## 8. Week 29 Checklist

- [ ] **Day 1: Optimization + Docs (10 hours)**
  - [ ] W29.1.1: Install wasm-opt, research flags (1h)
  - [ ] W29.1.2: Baseline measurement, Cargo.toml audit (1h)
  - [ ] W29.1.3: Apply optimization, measure results (2h)
  - [ ] W29.1.4: Browser test, document size (2h)
  - [ ] W29.2.1: Verify all doc links (1h)
  - [ ] W29.2.2: Fix cargo doc warnings (1h)
  - [ ] W29.2.3: Update README badges (0.5h)
  - [ ] W29.2.4: Review CHANGELOG per RFC-002 Implementation Plan (0.5h)
  - [ ] W29.2.5: Proofread docs/api/*.md and README.md (1h)

- [ ] **Day 2: Cleanup + Testing (6 hours)**
  - [ ] W29.3.1: DRY-RUN file list review (0.5h)
  - [ ] W29.3.2: Execute git rm --cached (0.5h)
  - [ ] W29.3.3: Update .gitignore (0.25h)
  - [ ] W29.3.4: Verify local copies (0.25h)
  - [ ] W29.3.5: Commit cleanup (0.5h)
  - [ ] W29.4.1: Run full test suite (1h)
  - [ ] W29.4.2: Run clippy strict (0.5h)
  - [ ] W29.4.3: Test Chrome (0.5h)
  - [ ] W29.4.4: Test Firefox (0.5h)
  - [ ] W29.4.5: Test Safari (0.5h)
  - [ ] W29.4.6: Verify npm package.json (0.5h)
  - [ ] W29.4.7: Dry-run cargo publish (0.5h)

- [ ] **Day 3: Release + Launch (8 hours)**
  - [ ] W29.5.1: Final version verification (0.5h)
  - [ ] W29.5.2: Publish to crates.io (1h)
  - [ ] W29.5.3: Publish to npm (1h)
  - [ ] W29.5.4: Create and push git tag (0.5h)
  - [ ] W29.5.5: Create GitHub release (1h)
  - [ ] W29.6.1: Draft Show HN post (0.5h)
  - [ ] W29.6.2: Draft Reddit post (0.5h)
  - [ ] W29.6.3: Draft Twitter thread (0.25h)
  - [ ] W29.6.4: Draft Dev.to article (0.75h)
  - [ ] W29.6.5: Record demo GIF (1h)
  - [ ] W29.6.6: Deploy demo (1h)

---

## 9. Success Metrics

| Metric | Target | Priority |
|:-------|:-------|:---------|
| Bundle size | ≤500KB (accept ≤520KB with note) | MEDIUM |
| Tests passing | 100% | HIGH |
| Browser compatibility | Chrome, Firefox, Safari | HIGH |
| Doc warnings | 0 | MEDIUM |
| Internal files removed | YES | HIGH |
| crates.io published | v0.6.0 | HIGH |
| npm published | v0.6.0 | HIGH |
| Launch posts ready | 4 platforms | MEDIUM |

---

## 10. Handoff

After completing Week 29:

**Artifacts Generated:**
- Optimized WASM bundle (target <500KB)
- Clean git history (no internal files)
- v0.6.0 on crates.io
- v0.6.0 on npm
- GitHub release with notes
- Live demo on GitHub Pages
- Launch content (HN, Reddit, Twitter, Dev.to)

**Status:** PENDING_HOSTILE_REVIEW

**Next:** v0.6.0 Launch Day + Marketing Push

---

## 11. RFC-002 Final Status

| Phase | Week | Status |
|:------|:-----|:-------|
| Phase 1 | Week 26 | COMPLETE |
| Phase 2 | Week 27 | COMPLETE |
| Phase 3 | Week 28 | COMPLETE |
| Release | Week 29 | IN_PROGRESS |

**v0.6.0 Feature Complete:** YES
**Release Blockers:** NONE

---

*Agent: PLANNER*
*Status: [REVISED v3.1]*
*Date: 2025-12-22*
*Previous Week: Week 28 (APPROVED)*
*Gate: `.claude/GATE_W28_COMPLETE.md`*
*HOSTILE_REVIEWER Findings Addressed: C1-C4, M1, m1-m2 (v3) + m1-v3 (v3.1)*
