# HOSTILE_REVIEWER: Demo Hub 404 Errors — CRITICAL

**Date:** 2025-12-24
**Reviewer:** HOSTILE_REVIEWER v2.0.0
**Artifact:** Demo Hub + GitHub Pages Deployment
**Status:** REJECTED — CRITICAL ISSUES FOUND

---

## Executive Summary

**The Demo Hub is fundamentally broken.** Only 2 of 10 demo links work on GitHub Pages. This is a critical user-facing bug reported by a real Reddit user.

---

## Issue Source

### Reddit Feedback (Lucas)

```
https://matte1782.github.io/edgevec/demo/v060_cyberpunk_demo.html

seems to be down!

PS — I'm a bit confused how to best use the demo! + out of curiosity -
is BM25 support on roadmap here? I would love to build pretty simple
hybrid search, I have been working mostly with pgvector but it's a bit
large for my use! Curious if you've thought about this.

In any case: love your project / this is so cool! Want to dig in more /
thanks for building.

Lucas
```

---

## Critical Finding: Architecture Flaw

### GitHub Pages Deployment Structure

GitHub Pages serves ONLY the `/docs` folder:

```
GitHub Pages URL: https://matte1782.github.io/edgevec/
Maps to: /docs folder in repository

/docs/demo/index.html    → https://matte1782.github.io/edgevec/demo/index.html    ✅ WORKS
/docs/demo/hub.html      → https://matte1782.github.io/edgevec/demo/hub.html      ✅ WORKS
/docs/demo/cyberpunk.html→ https://matte1782.github.io/edgevec/demo/cyberpunk.html✅ WORKS

/wasm/examples/*.html    → NOT DEPLOYED — 404 ❌
```

### Demo Hub Links Analysis

| Link in Hub | Target Path | GitHub Pages URL | Status |
|:------------|:------------|:-----------------|:-------|
| `index.html` | docs/demo/index.html | .../demo/index.html | ✅ WORKS |
| `cyberpunk.html` | docs/demo/cyberpunk.html | .../demo/cyberpunk.html | ✅ WORKS |
| `../../wasm/examples/index.html` | wasm/examples/index.html | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/simd_benchmark.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/benchmark-dashboard.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/soft_delete.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/batch_insert.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/batch_delete.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/stress-test.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |
| `../../wasm/examples/simd_test.html` | wasm/examples/ | NOT IN /docs | ❌ 404 |

**Result: 8 of 10 links are 404 on GitHub Pages.**

### User-Reported Broken Link

```
https://matte1782.github.io/edgevec/demo/v060_cyberpunk_demo.html
```

This URL doesn't exist because:
- File `v060_cyberpunk_demo.html` exists only in `wasm/examples/`
- Not in `docs/demo/`
- Therefore: **404 Error**

---

## Root Cause Analysis

### The Problem

1. **Demos are in wrong location**: Key demos are in `wasm/examples/` which is NOT deployed to GitHub Pages
2. **Hub links use relative paths**: `../../wasm/examples/` assumes local file serving, not GitHub Pages structure
3. **Missing file**: `v060_cyberpunk_demo.html` doesn't exist in `docs/demo/`

### The Reality

The beautiful Demo Hub is **pure cosmetics** — clicking most links gives 404 errors. This is embarrassing for a production library.

---

## Files Inventory

### In `/docs/demo/` (DEPLOYED to GitHub Pages)

```
docs/demo/
├── index.html       # Filter Playground (v0.7.0) ✅
├── hub.html         # Demo Hub ✅ (but links broken)
├── cyberpunk.html   # v0.6.0 Demo ✅
├── css/             # Stylesheets
├── js/              # JavaScript
└── favicon.svg
```

### In `/wasm/examples/` (LOCAL ONLY — NOT DEPLOYED)

```
wasm/examples/
├── index.html                    # Main Demo (v0.5.x)
├── simd_benchmark.html           # SIMD Benchmark (v0.7.0)
├── benchmark-dashboard.html      # Benchmark Dashboard (v0.4.0)
├── soft_delete.html              # Soft Delete Demo (v0.3.0)
├── batch_insert.html             # Batch Insert (v0.2.0)
├── batch_delete.html             # Batch Delete (v0.2.0)
├── stress-test.html              # Stress Test (v0.4.0)
├── simd_test.html                # SIMD Test (v0.7.0)
├── filter-playground.html        # Old filter playground
├── v060_cyberpunk_demo.html      # v0.6.0 Demo copy
└── v070_filter_playground.html   # v0.7.0 copy
```

---

## Required Fixes

### Option A: Copy Essential Demos to /docs/demo (RECOMMENDED)

Copy these files to make them live on GitHub Pages:

1. `wasm/examples/simd_benchmark.html` → `docs/demo/simd_benchmark.html`
2. `wasm/examples/soft_delete.html` → `docs/demo/soft_delete.html`
3. `wasm/examples/benchmark-dashboard.html` → `docs/demo/benchmark_dashboard.html`

Update hub.html links to use relative paths within `/docs/demo/`.

### Option B: Update Hub to Show Local-Only Correctly

Mark local-only demos more clearly and remove false links. Provide instructions to run locally.

### Option C: Serve from Root (Requires GitHub Config)

Change GitHub Pages to serve from root `/` instead of `/docs`. This would make all files accessible but requires restructuring.

---

## Immediate Actions Required

### 1. Fix Reddit User's Broken Link

The URL `https://matte1782.github.io/edgevec/demo/v060_cyberpunk_demo.html` needs to work.

**Solution**: Create redirect or copy `cyberpunk.html` to `v060_cyberpunk_demo.html`:

```html
<!-- docs/demo/v060_cyberpunk_demo.html -->
<!DOCTYPE html>
<html>
<head>
  <meta http-equiv="refresh" content="0; url=cyberpunk.html">
  <title>Redirecting...</title>
</head>
<body>
  <p>Redirecting to <a href="cyberpunk.html">v0.6.0 Demo</a>...</p>
</body>
</html>
```

### 2. Fix Hub Links

Either:
- Copy demos to `/docs/demo/`
- OR update hub to use correct paths
- OR mark local-only demos as "Run Locally" without clickable links

### 3. Add Usage Instructions

The user said "I'm a bit confused how to best use the demo!"

Add clear onboarding to the demos.

### 4. Address BM25 Question

Add to roadmap or respond directly. BM25 is a text scoring algorithm useful for hybrid search.

---

## Answer: Is the Tech Actually Live?

**Partially.**

| Component | Status | Notes |
|:----------|:-------|:------|
| Filter Playground | ✅ LIVE | Fully functional WASM demo |
| Cyberpunk Demo | ✅ LIVE | BQ, filtering, memory pressure |
| Demo Hub | ⚠️ BROKEN | Links to 404 pages |
| SIMD Benchmark | ❌ LOCAL ONLY | Not on GitHub Pages |
| Other Demos | ❌ LOCAL ONLY | Not on GitHub Pages |

**The core tech is real and works.** The Filter Playground and Cyberpunk demos are live and functional. But the Demo Hub creates a false impression by linking to demos that don't exist on GitHub Pages.

---

## VERDICT

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: REJECT                                          │
│                                                                     │
│   Artifact: Demo Hub GitHub Pages Deployment                        │
│   Date: 2025-12-24                                                  │
│                                                                     │
│   Critical Issues: 2                                                │
│   - 8 of 10 demo links return 404                                   │
│   - User-reported broken link confirmed                             │
│                                                                     │
│   Major Issues: 1                                                   │
│   - No usage instructions for confused users                        │
│                                                                     │
│   Minor Issues: 1                                                   │
│   - BM25 question unanswered                                        │
│                                                                     │
│   Disposition: REJECTED                                             │
│   - MUST fix 404 errors before v0.7.0 release                       │
│   - MUST address Reddit user's report                               │
│   - MUST improve demo discoverability                               │
│                                                                     │
│   BLOCK: v0.7.0 release until Demo Hub is production-ready          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. **Immediate**: Create `v060_cyberpunk_demo.html` redirect in `/docs/demo/`
2. **Today**: Copy essential demos to `/docs/demo/` and update hub links
3. **Today**: Respond to Lucas on Reddit
4. **Before Release**: Re-verify all links work

---

**Reviewer:** HOSTILE_REVIEWER
**Date:** 2025-12-24
**Verdict:** REJECTED — Critical 404 errors must be fixed

