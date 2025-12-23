# Week 29 Day 2: Internal Files Cleanup + Final Testing

**Date:** 2025-12-24
**Focus:** Remove internal development files from git, run comprehensive tests
**Estimated Duration:** 6 hours
**Phase:** v0.6.0 Release Preparation
**Dependencies:** Day 1 Complete

---

## Executive Summary

Day 2 focuses on:
1. **Internal Files Cleanup (W29.3)** — Remove .claude/, .cursor/, etc. from git tracking
2. **Final Testing & QA (W29.4)** — Comprehensive browser and cargo testing

**CRITICAL:** Internal files contain agent prompts that should NOT be public.

---

## Tasks

### W29.3: Internal Files Cleanup (2 hours)

#### W29.3.1: DRY-RUN — List Files to Remove (30 minutes)

**Objective:** Verify which files will be removed before executing.

**IMPORTANT:** This is a safety step. DO NOT SKIP.

**Commands:**

```bash
# Step 1: View current git status
git status

# Step 2: List files that WILL be removed from git
# (These are the internal development files per ROADMAP.md)
echo "=== Files to remove from git tracking ==="
echo ".claude/"
echo ".cursor/"
echo ".cursorrules"
echo "CLAUDE.md"

# Step 3: Verify these files exist locally
echo ""
echo "=== Verifying local file existence ==="
ls -la .claude/ 2>/dev/null && echo ".claude/ EXISTS" || echo ".claude/ NOT FOUND"
ls -la .cursor/ 2>/dev/null && echo ".cursor/ EXISTS" || echo ".cursor/ NOT FOUND"
ls -la .cursorrules 2>/dev/null && echo ".cursorrules EXISTS" || echo ".cursorrules NOT FOUND"
ls -la CLAUDE.md 2>/dev/null && echo "CLAUDE.md EXISTS" || echo "CLAUDE.md NOT FOUND"

# Step 4: Preview what git rm --cached will do
echo ""
echo "=== DRY RUN: What will be removed from git ==="
git rm -r --cached --dry-run .claude/ 2>/dev/null || echo ".claude/ not tracked"
git rm -r --cached --dry-run .cursor/ 2>/dev/null || echo ".cursor/ not tracked"
git rm --cached --dry-run .cursorrules 2>/dev/null || echo ".cursorrules not tracked"
git rm --cached --dry-run CLAUDE.md 2>/dev/null || echo "CLAUDE.md not tracked"
```

**Verification Checklist:**
- [ ] .claude/ directory exists locally
- [ ] .cursor/ directory exists locally (if applicable)
- [ ] .cursorrules file exists locally (if applicable)
- [ ] CLAUDE.md file exists locally
- [ ] No unexpected files in dry-run output

**Deliverables:**
- [ ] Dry-run output reviewed
- [ ] All expected files identified

**Acceptance Criteria (Binary):**
- Dry-run executed and reviewed (PASS/FAIL)
- No unexpected files flagged (PASS/FAIL)

---

#### W29.3.2: Execute git rm --cached (30 minutes)

**Objective:** Remove internal files from git tracking while preserving local copies.

**Commands:**

```bash
# Step 1: Remove from git tracking (keeps local files)
git rm -r --cached .claude/
git rm -r --cached .cursor/ 2>/dev/null || echo ".cursor/ not tracked - skipping"
git rm --cached .cursorrules 2>/dev/null || echo ".cursorrules not tracked - skipping"
git rm --cached CLAUDE.md

# Step 2: Verify removal staged
git status
# Should show "deleted: .claude/..." in staging area
```

**Expected Output:**
```
Changes to be committed:
  (use "git restore --staged <file>..." to unstage)
        deleted:    .claude/CLAUDE.md
        deleted:    .claude/GATE_*.md
        deleted:    .claude/agents/*.md
        deleted:    .claude/commands/*.md
        deleted:    .claude/settings.json
        deleted:    CLAUDE.md
```

**Deliverables:**
- [ ] git rm --cached executed
- [ ] Files staged for deletion

**Acceptance Criteria (Binary):**
- `git status` shows internal files staged for deletion (PASS/FAIL)

---

#### W29.3.3: Update .gitignore (15 minutes)

**Objective:** Prevent internal files from being re-added to git.

**Commands:**

```bash
# Add patterns to .gitignore
cat >> .gitignore << 'EOF'

# Internal development files (removed from git before v0.6.0 release)
# These files contain agent prompts and should NOT be public
.claude/
.cursor/
.cursorrules
CLAUDE.md
EOF

# Verify .gitignore updated
tail -10 .gitignore
```

**Verification:**
- [ ] `.gitignore` contains `.claude/`
- [ ] `.gitignore` contains `.cursor/`
- [ ] `.gitignore` contains `.cursorrules`
- [ ] `.gitignore` contains `CLAUDE.md`

**Acceptance Criteria (Binary):**
- All 4 patterns exist in .gitignore (PASS/FAIL)

---

#### W29.3.4: Verify Local Copies Preserved (15 minutes)

**Objective:** Confirm files still exist locally after git removal.

**Commands:**

```bash
# Verify local files still exist
echo "=== Verifying local files preserved ==="

# Check .claude directory
if [ -d ".claude" ]; then
  echo "✓ .claude/ directory exists"
  ls .claude/ | head -5
else
  echo "✗ ERROR: .claude/ directory MISSING"
  exit 1
fi

# Check CLAUDE.md
if [ -f "CLAUDE.md" ]; then
  echo "✓ CLAUDE.md exists"
else
  echo "✗ ERROR: CLAUDE.md MISSING"
  exit 1
fi

echo ""
echo "=== Local files preserved successfully ==="
```

**Deliverables:**
- [ ] .claude/ directory exists locally
- [ ] CLAUDE.md exists locally
- [ ] All agent definitions accessible

**Acceptance Criteria (Binary):**
- `ls -la .claude/` shows files (PASS/FAIL)
- `ls -la CLAUDE.md` shows file (PASS/FAIL)

---

#### W29.3.5: Commit Cleanup Changes (30 minutes)

**Objective:** Create commit removing internal files from repository.

**Commands:**

```bash
# Stage .gitignore changes
git add .gitignore

# Create commit with descriptive message
git commit -m "chore: remove internal development files before v0.6.0 release

Per ROADMAP.md Week 29 pre-release requirements:

Removed from git tracking (files remain local):
- .claude/ — Agent prompts, gate files, settings
- .cursor/ — Cursor IDE commands (if tracked)
- .cursorrules — Development rules (if tracked)
- CLAUDE.md — Project instructions

These files contain internal development tooling that should not
be public in the GitHub repository.

Added to .gitignore to prevent accidental re-addition."

# Verify commit
git log -1 --stat
```

**Expected Commit Stats:**
```
 .gitignore              |  6 ++++++
 .claude/CLAUDE.md       |  (deleted)
 .claude/agents/*.md     |  (deleted)
 ...
 N files changed, X insertions(+), Y deletions(-)
```

**Deliverables:**
- [ ] Commit created successfully
- [ ] Commit message follows convention

**Acceptance Criteria (Binary):**
- `git log -1 --oneline` shows cleanup commit (PASS/FAIL)
- Commit deletes internal files from tracking (PASS/FAIL)

---

### W29.4: Final Testing & QA (4 hours)

#### W29.4.1: Run Full Test Suite (1 hour)

**Objective:** Verify all tests pass before release.

**Commands:**

```bash
# Run full test suite
cargo test 2>&1 | tee test_output.txt

# Check result
tail -5 test_output.txt
# Expected: "test result: ok. X passed; 0 failed"

# Count passed/failed
grep -E "^test result:" test_output.txt
```

**Expected Results:**
- All unit tests pass
- All integration tests pass (26 tests from Week 28)
- No test failures

**Test Categories:**

| Category | Count | Expected |
|:---------|:------|:---------|
| Unit tests | ~650+ | ALL PASS |
| Integration (hybrid_search) | 5 | ALL PASS |
| Integration (bq_persistence) | 7 | ALL PASS |
| Integration (bq_recall_roundtrip) | 7 | ALL PASS |
| Integration (metadata_roundtrip) | 7 | ALL PASS |

**Deliverables:**
- [ ] `cargo test` completes
- [ ] All tests pass
- [ ] Test count recorded: ___

**Acceptance Criteria (Binary):**
- `cargo test` exits with 0 failures (PASS/FAIL)

---

#### W29.4.2: Run Clippy Strict Mode (30 minutes)

**Objective:** Verify no lint warnings before release.

**Commands:**

```bash
# Run clippy with deny warnings
cargo clippy -- -D warnings 2>&1 | tee clippy_output.txt

# Check for errors
if grep -q "error" clippy_output.txt; then
  echo "FAIL: Clippy errors found"
  cat clippy_output.txt | grep "error"
  exit 1
else
  echo "PASS: No clippy errors"
fi
```

**Common Issues to Watch:**
- Unused variables
- Missing documentation
- Unsafe code without justification
- Dead code

**Deliverables:**
- [ ] Clippy runs without errors
- [ ] 0 warnings

**Acceptance Criteria (Binary):**
- `cargo clippy -- -D warnings` exits 0 (PASS/FAIL)

---

#### W29.4.3: Test Chrome Browser (30 minutes)

**Objective:** Verify demo works in Chrome.

**Setup:**

```bash
# Start local server
cd wasm/examples
python -m http.server 8080
```

**Test Checklist (Chrome 120+):**

| Test | Steps | Expected | Status |
|:-----|:------|:---------|:-------|
| Page Load | Open http://localhost:8080/v060_cyberpunk_demo.html | No console errors | [ ] |
| WASM Init | Check DevTools Console | "EdgeVec initialized" or similar | [ ] |
| Insert Vectors | Click Insert button | Success toast shown | [ ] |
| Search | Enter query, click Search | Results displayed | [ ] |
| BQ Search | Toggle BQ mode, search | Speedup metrics shown | [ ] |
| Filter | Enter filter expression | Filtered results | [ ] |
| Memory | Check memory indicator | Shows usage stats | [ ] |
| Animations | Observe particle/matrix | Smooth 60fps | [ ] |

**Deliverables:**
- [ ] All 8 tests pass in Chrome
- [ ] No console errors
- [ ] Screenshot taken (optional)

**Acceptance Criteria (Binary):**
- Demo loads without JS errors in Chrome (PASS/FAIL)
- All 8 functional tests pass (PASS/FAIL)

---

#### W29.4.4: Test Firefox Browser (30 minutes)

**Objective:** Verify demo works in Firefox.

**Test Checklist (Firefox 121+):**

| Test | Steps | Expected | Status |
|:-----|:------|:---------|:-------|
| Page Load | Open demo URL | No console errors | [ ] |
| Insert | Insert 100 vectors | Success | [ ] |
| Search | Perform search | Results displayed | [ ] |
| Filter | Use filter syntax | category = "test" works | [ ] |

**Deliverables:**
- [ ] All 4 tests pass in Firefox
- [ ] No console errors

**Acceptance Criteria (Binary):**
- Demo loads without errors in Firefox (PASS/FAIL)

---

#### W29.4.5: Test Safari Browser (30 minutes)

**Objective:** Verify demo works in Safari (iOS compatibility check).

**Test Checklist (Safari 17+):**

| Test | Steps | Expected | Status |
|:-----|:------|:---------|:-------|
| Page Load | Open demo URL | No errors | [ ] |
| Basic Search | Perform search | Results displayed | [ ] |
| Touch Targets | Inspect buttons | ≥44px tap targets | [ ] |
| Reduced Motion | Enable in preferences | Animations disabled | [ ] |

**Note:** Canvas effects (particles, matrix) may not render on iOS Safari. This is expected behavior per Week 28 Day 7 implementation.

**Deliverables:**
- [ ] All 4 tests pass in Safari
- [ ] Touch targets verified

**Acceptance Criteria (Binary):**
- Demo loads without errors in Safari (PASS/FAIL)

---

#### W29.4.6: Verify npm package.json (30 minutes)

**Objective:** Ensure package.json is ready for npm publish.

**Checks:**

```bash
# Navigate to pkg directory
cd pkg

# Check version
grep '"version"' package.json
# Expected: "version": "0.6.0"

# Check package name
grep '"name"' package.json
# Expected: "name": "edgevec"

# Check main entry
grep '"main"' package.json

# Check types entry
grep '"types"' package.json

# Verify files exist
ls -la edgevec.js edgevec.d.ts edgevec_bg.wasm
```

**package.json Requirements:**

| Field | Expected Value | Status |
|:------|:---------------|:-------|
| name | "edgevec" | [ ] |
| version | "0.6.0" | [ ] |
| main | "edgevec.js" | [ ] |
| types | "edgevec.d.ts" | [ ] |
| files | Includes .wasm | [ ] |

**Deliverables:**
- [ ] Version is 0.6.0
- [ ] All required fields present
- [ ] All referenced files exist

**Acceptance Criteria (Binary):**
- `grep '"version"' pkg/package.json` shows "0.6.0" (PASS/FAIL)

---

#### W29.4.7: Dry-Run cargo publish (30 minutes)

**Objective:** Verify crate is ready for crates.io.

**Commands:**

```bash
# Verify Cargo.toml version
grep "^version" Cargo.toml
# Expected: version = "0.6.0"

# Run publish dry-run
cargo publish --dry-run 2>&1 | tee publish_dryrun.txt

# Check for errors
if grep -q "error" publish_dryrun.txt; then
  echo "FAIL: Publish dry-run failed"
  exit 1
else
  echo "PASS: Ready to publish"
fi
```

**Common Issues:**
- Missing license file
- Invalid package metadata
- Yanked dependencies
- Version already exists

**Deliverables:**
- [ ] Cargo.toml version is 0.6.0
- [ ] Dry-run succeeds

**Acceptance Criteria (Binary):**
- `cargo publish --dry-run` exits 0 (PASS/FAIL)

---

## Exit Criteria for Day 2

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Internal files removed | `git status` shows deleted | [ ] |
| .gitignore updated | Contains 4 patterns | [ ] |
| Local copies preserved | Files exist on disk | [ ] |
| Cleanup committed | `git log -1` shows commit | [ ] |
| All tests pass | `cargo test` | [ ] |
| Clippy clean | 0 errors | [ ] |
| Chrome works | Manual test | [ ] |
| Firefox works | Manual test | [ ] |
| Safari works | Manual test | [ ] |
| npm package ready | Version 0.6.0 | [ ] |
| crates.io ready | Dry-run passes | [ ] |

---

## Handoff

After completing Day 2:

**Artifacts:**
- Git history cleaned (no internal files)
- All tests verified passing
- Browser compatibility confirmed
- Publish dry-runs successful

**Status:** PENDING Day 3

**Next:** Day 3 — Release Execution + Launch Content

---

*Agent: RUST_ENGINEER + TEST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2025-12-22*
*Dependencies: Day 1 Complete*
