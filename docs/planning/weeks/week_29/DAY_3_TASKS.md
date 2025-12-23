# Week 29 Day 3: Release Execution + Launch Content

**Date:** 2025-12-25
**Focus:** Publish to crates.io/npm and prepare launch content
**Estimated Duration:** 8 hours
**Phase:** v0.6.0 Release Execution
**Dependencies:** Day 2 Complete (Tests Pass, Cleanup Done)

---

## Executive Summary

Day 3 focuses on:
1. **Release Execution (W29.5)** — Publish to crates.io and npm
2. **Launch Content Preparation (W29.6)** — GitHub release, announcements

**CRITICAL:** Only execute after Day 2 passes all tests.

---

## Pre-Day 3 Checklist

Before starting Day 3, verify:

| Criterion | Command | Expected |
|:----------|:--------|:---------|
| All tests pass | `cargo test` | 0 failures |
| Clippy clean | `cargo clippy -- -D warnings` | 0 errors |
| Internal files removed | `git status` | .claude/ not tracked |
| npm package ready | `grep version pkg/package.json` | "0.6.0" |
| Cargo.toml version | `grep version Cargo.toml` | "0.6.0" |

---

## Tasks

### W29.5: Release Execution (4 hours)

#### W29.5.1: Final Pre-Release Verification (30 minutes)

**Objective:** Last verification before publishing.

**Commands:**

```bash
# Step 1: Clean rebuild
cargo clean
cargo build --release

# Step 2: Run tests one final time
cargo test 2>&1 | tail -5
# Expected: "test result: ok"

# Step 3: Verify WASM build
wasm-pack build --release --target web

# Step 4: Check bundle size (must be ≤570KB per W29.1)
ls -l pkg/edgevec_bg.wasm
```

**Deliverables:**
- [ ] Clean build succeeds
- [ ] All tests pass
- [ ] WASM bundle ≤570KB

**Acceptance Criteria (Binary):**
- `cargo build --release` exits 0 (PASS/FAIL)
- `cargo test` shows 0 failures (PASS/FAIL)

---

#### W29.5.2: Publish to crates.io (1 hour)

**Objective:** Publish edgevec v0.6.0 to crates.io.

**Pre-requisites:**
- crates.io account authenticated (`cargo login`)
- Version 0.6.0 not already published

**Commands:**

```bash
# Step 1: Final dry-run
cargo publish --dry-run 2>&1 | tail -10

# Step 2: PUBLISH (requires confirmation)
cargo publish

# Step 3: Verify on crates.io
echo "Verify at: https://crates.io/crates/edgevec"
```

**Expected Output:**
```
Uploading edgevec v0.6.0
Uploaded edgevec v0.6.0 to registry 'crates.io'
```

**Deliverables:**
- [ ] `cargo publish` succeeds
- [ ] Version visible on crates.io

**Acceptance Criteria (Binary):**
- `cargo publish` exits 0 (PASS/FAIL)
- https://crates.io/crates/edgevec shows v0.6.0 (PASS/FAIL)

---

#### W29.5.3: Publish to npm (1 hour)

**Objective:** Publish @anthropic/edgevec v0.6.0 to npm.

**Pre-requisites:**
- npm account authenticated (`npm login`)
- Version 0.6.0 not already published

**Commands:**

```bash
# Step 1: Navigate to pkg directory
cd pkg

# Step 2: Verify package.json
cat package.json | head -10

# Step 3: Dry-run publish
npm publish --dry-run

# Step 4: PUBLISH (requires confirmation)
npm publish --access public

# Step 5: Verify on npm
echo "Verify at: https://www.npmjs.com/package/edgevec"
```

**Expected Output:**
```
npm notice
npm notice 📦  edgevec@0.6.0
npm notice === Tarball Contents ===
npm notice ...
+ edgevec@0.6.0
```

**Deliverables:**
- [ ] `npm publish` succeeds
- [ ] Version visible on npm

**Acceptance Criteria (Binary):**
- `npm publish` exits 0 (PASS/FAIL)
- https://www.npmjs.com/package/edgevec shows v0.6.0 (PASS/FAIL)

---

#### W29.5.4: Create Git Tag + GitHub Release (1 hour)

**Objective:** Create v0.6.0 tag and GitHub release.

**Commands:**

```bash
# Step 1: Create annotated tag
git tag -a v0.6.0 -m "Release v0.6.0: RFC-002 Complete

Features:
- Binary Quantization (32x memory reduction)
- Metadata filtering with expression syntax
- Memory pressure monitoring
- Hybrid search (BQ + rescore)

See CHANGELOG.md for full details."

# Step 2: Push tag
git push origin v0.6.0

# Step 3: Create GitHub release via gh CLI (if available)
gh release create v0.6.0 \
  --title "EdgeVec v0.6.0 — RFC-002 Complete" \
  --notes-file CHANGELOG_RELEASE_NOTES.md \
  pkg/edgevec_bg.wasm
```

**Release Notes Template:**

```markdown
# EdgeVec v0.6.0 — RFC-002 Complete

This release implements RFC-002: Metadata & Binary Quantization.

## Highlights

- **32x Memory Reduction** — Binary Quantization compresses vectors from 3KB to 96 bytes
- **Metadata Filtering** — Filter search by category, tags, and numeric ranges
- **Memory Monitoring** — Track pressure levels and prevent OOM
- **Hybrid Search** — Combine BQ speed with F32 accuracy via rescoring

## New WASM Exports

| Function | Description |
|:---------|:------------|
| `insertWithMetadata()` | Insert vectors with JSON metadata |
| `searchFiltered()` | Search with filter expressions |
| `searchBQ()` | Fast binary quantized search |
| `searchBQRescored()` | BQ search with F32 rescoring |
| `searchHybrid()` | Adaptive hybrid search |
| `getMemoryPressure()` | Get current memory status |

## Performance

| Metric | v0.5.x | v0.6.0 | Improvement |
|:-------|:-------|:-------|:------------|
| Memory/vector (BQ) | 3KB | 96B | 32x reduction |
| Search latency (BQ) | - | 2-5ms | NEW |
| Recall@10 (BQ+rescore) | - | 0.936 | NEW |

## Installation

```bash
# Rust
cargo add edgevec

# npm
npm install edgevec
```

## Links

- [Documentation](https://docs.rs/edgevec)
- [CHANGELOG](./CHANGELOG.md)
- [Cyberpunk Demo](./wasm/examples/v060_cyberpunk_demo.html)
```

**Deliverables:**
- [ ] Git tag v0.6.0 created
- [ ] Tag pushed to origin
- [ ] GitHub release created (optional but recommended)

**Acceptance Criteria (Binary):**
- `git tag -l v0.6.0` shows tag exists (PASS/FAIL)
- `git ls-remote --tags origin | grep v0.6.0` shows remote tag (PASS/FAIL)

---

#### W29.5.5: Post-Publish Verification (30 minutes)

**Objective:** Verify packages work from public registries.

**Commands:**

```bash
# Step 1: Test crates.io installation (new temp project)
mkdir /tmp/test-edgevec && cd /tmp/test-edgevec
cargo init
echo 'edgevec = "0.6.0"' >> Cargo.toml
cargo build

# Step 2: Test npm installation
mkdir /tmp/test-edgevec-npm && cd /tmp/test-edgevec-npm
npm init -y
npm install edgevec
node -e "const edgevec = require('edgevec'); console.log('npm install OK');"
```

**Deliverables:**
- [ ] crates.io installation works
- [ ] npm installation works

**Acceptance Criteria (Binary):**
- `cargo build` in test project succeeds (PASS/FAIL)
- `npm install edgevec` succeeds (PASS/FAIL)

---

### W29.6: Launch Content Preparation (4 hours)

#### W29.6.1: Write Release Announcement (1.5 hours)

**Objective:** Create announcement for social media and dev communities.

**Target Platforms:**
- Twitter/X
- Reddit r/rust
- Reddit r/MachineLearning
- Dev.to (optional)
- Hacker News (optional)

**Twitter/X Thread Template (280 chars per tweet):**

```
Tweet 1:
EdgeVec v0.6.0 is out! A browser-native vector database in Rust/WASM.

New in v0.6.0:
- 32x memory reduction with Binary Quantization
- Metadata filtering
- Memory pressure monitoring

npm install edgevec
cargo add edgevec

Thread /1

---

Tweet 2:
Binary Quantization compresses 768-dim vectors from 3KB to 96 bytes while maintaining 93%+ recall with rescoring.

Perfect for mobile/edge deployment where memory matters.

Demo: [link to cyberpunk demo]

/2

---

Tweet 3:
Filter your searches without post-processing:

searchFiltered(query, 10, "category = 'docs' AND year >= 2024")

Expression syntax supports =, !=, >, <, AND, OR, IN, CONTAINS.

/3

---

Tweet 4:
Try the interactive demo with cyberpunk aesthetics:
[screenshot of demo]

Built with the WASM bindings — everything runs in your browser.

Link: [demo URL]

/4
```

**Reddit Post Template:**

```markdown
# EdgeVec v0.6.0: Browser-Native Vector Database with 32x Memory Reduction

I just released EdgeVec v0.6.0, implementing RFC-002 (Metadata & Binary Quantization).

## What is EdgeVec?

A vector database that runs entirely in the browser via WebAssembly. No server required — your vectors stay on-device.

## What's New in v0.6.0?

1. **Binary Quantization** — Compress vectors 32x (768-dim: 3KB → 96 bytes)
2. **Metadata Filtering** — Query with expressions: `category = 'docs' AND year > 2023`
3. **Memory Monitoring** — Track pressure, prevent OOM
4. **Hybrid Search** — BQ speed + F32 accuracy via rescoring

## Performance

| Metric | Result |
|:-------|:-------|
| Memory per vector (BQ) | 96 bytes |
| Search latency (BQ, 100k) | 2-5ms |
| Recall@10 (BQ+rescore) | 0.936 |
| Bundle size | ~500KB gzipped |

## Try It

- **npm:** `npm install edgevec`
- **Rust:** `cargo add edgevec`
- **Demo:** [cyberpunk demo link]
- **GitHub:** [repo link]

Feedback welcome!
```

**Deliverables:**
- [ ] Twitter thread draft created
- [ ] Reddit post draft created
- [ ] Screenshots captured

**Acceptance Criteria (Binary):**
- Draft files exist in `docs/release/v0.6.0/announcements/` (PASS/FAIL)

---

#### W29.6.2: Capture Demo Screenshots (1 hour)

**Objective:** Create visual assets for announcements.

**Screenshots Needed:**

| Screenshot | Description | Dimensions |
|:-----------|:------------|:-----------|
| demo_hero.png | Full demo with matrix rain | 1200x630 (Twitter card) |
| demo_search.png | Search results with metrics | 800x600 |
| demo_bq_toggle.png | BQ mode toggle with speedup | 800x400 |
| demo_filter.png | Filter expression input | 800x400 |
| demo_memory.png | Memory gauge in action | 400x300 |

**Commands:**

```bash
# Start local server
cd wasm/examples
python -m http.server 8080

# Open Chrome DevTools
# Set device mode to 1200x630 for Twitter card
# Take screenshot with Ctrl+Shift+P → "Capture screenshot"
```

**Deliverables:**
- [ ] demo_hero.png (1200x630)
- [ ] demo_search.png
- [ ] At least 3 screenshots total

**Acceptance Criteria (Binary):**
- `ls docs/release/v0.6.0/screenshots/*.png` shows ≥3 files (PASS/FAIL)

---

#### W29.6.3: Update docs.rs Documentation (1 hour)

**Objective:** Verify docs.rs renders correctly after publish.

**Checks:**

| Check | URL | Expected |
|:------|:----|:---------|
| Crate page | https://docs.rs/edgevec/0.6.0 | Shows v0.6.0 |
| Module index | https://docs.rs/edgevec/0.6.0/edgevec/ | All modules visible |
| Search function | Click search() | Has documentation |
| Examples | Check code examples | Syntax highlighted |

**Common Issues:**
- Missing doc comments → Add `///` before public items
- Broken intra-doc links → Use full paths
- Hidden modules → Add `#[doc(hidden)]` or make public

**Deliverables:**
- [ ] docs.rs renders v0.6.0
- [ ] All public APIs documented
- [ ] No broken links

**Acceptance Criteria (Binary):**
- https://docs.rs/edgevec/0.6.0 accessible (PASS/FAIL)

---

#### W29.6.4: Create Demo Video (Optional, 0.5 hours)

**Objective:** Record short demo GIF/video for announcements.

**Tools:**
- LICEcap (GIF)
- OBS Studio (MP4)
- ScreenToGif (GIF)

**Storyboard:**

1. Open demo page (0-2s)
2. Insert 100 vectors (2-5s)
3. Perform search (5-8s)
4. Toggle BQ mode (8-10s)
5. Show filter input (10-13s)
6. Show results with metrics (13-15s)

**Output:**
- `demo_walkthrough.gif` (<5MB for Twitter)
- Duration: 10-15 seconds
- Resolution: 800x450 minimum

**Deliverables:**
- [ ] demo_walkthrough.gif created (optional)

**Acceptance Criteria (Binary):**
- GIF exists and is <5MB (PASS/FAIL) — or SKIP if not done

---

## Exit Criteria for Day 3

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| crates.io published | URL shows v0.6.0 | [ ] |
| npm published | URL shows v0.6.0 | [ ] |
| Git tag created | `git tag -l v0.6.0` | [ ] |
| Tag pushed | Remote tag exists | [ ] |
| Installation verified | Test projects build | [ ] |
| Twitter thread drafted | File exists | [ ] |
| Reddit post drafted | File exists | [ ] |
| Screenshots captured | ≥3 images | [ ] |
| docs.rs live | URL accessible | [ ] |

---

## Rollback Procedures

### If crates.io Publish Fails

```bash
# Check error message
cargo publish 2>&1 | grep error

# Common fixes:
# - Version already exists: Bump to 0.6.1
# - Auth failed: cargo login
# - Missing fields: Update Cargo.toml
```

### If npm Publish Fails

```bash
# Check error message
npm publish 2>&1

# Common fixes:
# - Auth failed: npm login
# - Version exists: Bump version in package.json
# - Missing access: npm publish --access public
```

### If Critical Bug Found Post-Publish

1. **DO NOT PANIC** — Published versions are immutable
2. Immediately publish 0.6.1 with fix
3. Yank 0.6.0 only if security issue: `cargo yank --vers 0.6.0`
4. Update announcements to point to 0.6.1

---

## Handoff

After completing Day 3:

**Artifacts:**
- v0.6.0 live on crates.io
- v0.6.0 live on npm
- Git tag v0.6.0 pushed
- GitHub release created
- Announcement drafts ready
- Screenshots ready

**Status:** WEEK 29 COMPLETE

**Next:** Week 30 — Post-launch monitoring, community response, start RFC-003 planning

---

*Agent: DOCWRITER + RUST_ENGINEER*
*Status: [PROPOSED]*
*Date: 2025-12-22*
*Dependencies: Day 2 Complete*
