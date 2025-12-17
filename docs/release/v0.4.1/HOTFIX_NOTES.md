# EdgeVec v0.4.1 Hotfix Release Notes

**Release Date:** 2025-12-17
**Type:** HOTFIX (Critical Bug Fix)
**Severity:** CRITICAL

---

## Summary

Fixes a critical packaging issue that prevented v0.4.0 from working with modern bundlers (Vite, webpack, Rollup, etc.).

## Bug Fixed

### Issue: Missing `snippets` directory in npm package

**Symptom:**
```
Could not resolve "./snippets/edgevec-e9d90602e6bb6c2c/src/js/storage.js"
```

**Root Cause:**
The `pkg/package.json` `files` array did not include the `snippets` directory, even though:
1. The WASM bundle imports from `./snippets/edgevec-HASH/src/js/storage.js`
2. The `sideEffects` array references `"./snippets/*"`

**Fix:**
Added `"snippets"` to the `files` array in `package.json`.

## Changes

### pkg/package.json

**Before (v0.4.0):**
```json
"files": [
  "edgevec_bg.wasm",
  "edgevec.js",
  "edgevec.d.ts",
  "LICENSE-APACHE",
  "LICENSE-MIT"
]
```

**After (v0.4.1):**
```json
"files": [
  "edgevec_bg.wasm",
  "edgevec.js",
  "edgevec.d.ts",
  "snippets",
  "LICENSE-APACHE",
  "LICENSE-MIT"
]
```

## Verification

```bash
# npm pack now includes snippets
npm pack --dry-run 2>&1 | grep snippets
# Output:
# npm notice 1.7kB snippets/edgevec-e9d90602e6bb6c2c/src/js/storage.js
# ... (all snippet files included)
```

## Affected Versions

| Version | Status |
|:--------|:-------|
| 0.4.0 | ❌ BROKEN (do not use with bundlers) |
| 0.4.1 | ✅ FIXED |

## Upgrade Instructions

```bash
npm install edgevec@0.4.1
# or
npm update edgevec
```

## Workaround (for those still on v0.4.0)

If you cannot upgrade immediately, use this postinstall script:

```javascript
// scripts/postinstall.js
import { mkdir, writeFile } from 'fs/promises'

const snippetsDir = 'node_modules/edgevec/snippets/edgevec-e9d90602e6bb6c2c/src/js'
await mkdir(snippetsDir, { recursive: true })
await writeFile(`${snippetsDir}/storage.js`, `// content from src/js/storage.js`)
```

## Acknowledgments

Thanks to the community member who reported this issue with a clear reproduction case and suggested fix. Your detailed report enabled a rapid resolution.

---

**Full Changelog:** v0.4.0...v0.4.1
