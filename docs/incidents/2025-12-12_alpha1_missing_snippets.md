# Incident Report: v0.2.0-alpha.1 Missing Snippets Directory

**Date:** 2025-12-12
**Severity:** CRITICAL
**Status:** RESOLVED (v0.2.0-alpha.2)
**Incident ID:** INC-2025-12-12-001

---

## Executive Summary

Published npm package v0.2.0-alpha.1 was missing the `snippets/` directory containing required IndexedDB storage JavaScript code. This caused import failures in Node.js environments. The issue was detected during post-publish verification. Hotfix v0.2.0-alpha.2 was released within 15 minutes. Original version deprecated on npm registry.

---

## Timeline

| Time (UTC) | Event |
|:-----------|:------|
| ~21:40 | v0.2.0-alpha.1 published to npm |
| ~21:41 | Post-publish verification started |
| ~21:42 | Fresh install test failed with `ERR_MODULE_NOT_FOUND` |
| ~21:43 | Root cause identified: missing `snippets/` in package.json files array |
| ~21:45 | Hotfix initiated: package.json updated, version bumped to alpha.2 |
| ~21:48 | v0.2.0-alpha.2 published to npm |
| ~21:49 | v0.2.0-alpha.1 deprecated on npm |
| ~21:50 | Fresh install test of alpha.2 PASSED |
| ~21:55 | Git tag v0.2.0-alpha.2 created and pushed |

**Time to Detection:** ~2 minutes
**Time to Resolution:** ~10 minutes
**Total Incident Duration:** ~15 minutes

---

## Root Cause Analysis

### What Happened

The `wasm/pkg/package.json` `files` array did not include `"snippets"`, causing npm to exclude this directory from the published tarball. The `snippets/` directory contains `storage.js` which is imported by the main `edgevec.js` module for IndexedDB persistence.

**Defective package.json (v0.2.0-alpha.1):**
```json
"files": [
  "edgevec_bg.wasm",
  "edgevec_bg.wasm.d.ts",
  "edgevec.js",
  "edgevec.d.ts"
]
```

**Fixed package.json (v0.2.0-alpha.2):**
```json
"files": [
  "edgevec_bg.wasm",
  "edgevec_bg.wasm.d.ts",
  "edgevec.js",
  "edgevec.d.ts",
  "snippets"
]
```

### Why It Happened

1. **wasm-pack generates snippets dynamically** — The `snippets/` directory is created by wasm-pack during build but not automatically added to package.json
2. **Manual package.json maintenance** — The files array was manually maintained and incomplete
3. **No automated validation** — No script verified package contents before publish

### Why It Wasn't Caught Earlier

1. **Pre-release checklist gap** — Checklist focused on code quality, not package contents
2. **No `npm pack` dry-run** — Would have shown missing files
3. **No fresh install test before publish** — Local testing used source files, not packed tarball
4. **sideEffects field misleading** — `"sideEffects": ["./snippets/*"]` existed but didn't ensure inclusion

---

## Impact Assessment

| Metric | Value |
|:-------|:------|
| **Users Affected** | Anyone who ran `npm install edgevec@0.2.0-alpha.1` |
| **Functionality Lost** | Complete module import failure |
| **Error Message** | `ERR_MODULE_NOT_FOUND: Cannot find module '.../snippets/.../storage.js'` |
| **Duration** | ~10 minutes (until hotfix published) |
| **Downloads of Broken Version** | Unknown (likely 0-2 given immediate deprecation) |
| **Reputation Impact** | Minor — alpha release, quickly fixed, properly deprecated |

---

## Resolution Steps Taken

1. **Identified root cause** — Missing `snippets` in files array
2. **Fixed package.json** — Added `"snippets"` to files array
3. **Bumped version** — Changed to `0.2.0-alpha.2`
4. **Verified fix** — Ran `npm pack --dry-run` to confirm snippets included
5. **Published hotfix** — `npm publish --access public --tag alpha`
6. **Deprecated broken version** — `npm deprecate edgevec@0.2.0-alpha.1 "Missing snippets directory - use 0.2.0-alpha.2"`
7. **Verified fresh install** — Tested in clean directory, import succeeded
8. **Updated git** — Committed changes, created v0.2.0-alpha.2 tag
9. **Created GitHub Release** — For v0.2.0-alpha.2

---

## Prevention Measures

### Immediate (Applied to v0.2.0-alpha.2)

- [x] Verify package.json files array includes all required directories
- [x] Run `npm pack --dry-run` before publish
- [x] Test fresh install in clean directory before publish

### Short-Term (Before Next Release)

- [ ] Add `npm pack --dry-run` to pre-release checklist
- [ ] Add tarball content verification step
- [ ] Add fresh install smoke test to checklist
- [ ] Create `.claude/RELEASE_CHECKLIST.md` with explicit validation steps

### Long-Term (v0.3.0+)

- [ ] Create automated script: `scripts/verify-package-contents.sh`
- [ ] Add CI check: Compare packed files vs expected manifest
- [ ] Consider using `files` whitelist vs `.npmignore` blacklist
- [ ] Add pre-publish hook to package.json

---

## Action Items

| Item | Owner | Status | Due |
|:-----|:------|:-------|:----|
| Create `.claude/RELEASE_CHECKLIST.md` | RUST_ENGINEER | [ ] | W9D42 fix |
| Update CHANGELOG.md with severity warnings | RUST_ENGINEER | [ ] | W9D42 fix |
| Create `scripts/verify-package-contents.sh` | RUST_ENGINEER | [ ] | v0.3.0 |
| Add CI package validation | RUST_ENGINEER | [ ] | v0.3.0 |
| Review wasm-pack output handling | WASM_SPECIALIST | [ ] | v0.3.0 |

---

## Lessons Learned

### Technical

1. **wasm-pack snippets are easy to miss** — They're generated in a hash-named subdirectory
2. **npm pack is essential validation** — It shows exactly what will be published
3. **sideEffects doesn't mean inclusion** — It's for tree-shaking, not file inclusion

### Process

1. **Never assume package.json is complete** — Validate explicitly every release
2. **Pack before publish** — `npm pack --dry-run` is cheap insurance
3. **Test fresh installs** — Local dev environment != user experience
4. **Checklists must be specific** — "Verify build" is too vague; "Run npm pack and verify snippets/" is actionable

### Response

1. **Quick detection saved us** — Post-publish verification caught it in 2 minutes
2. **Hotfix process worked** — 10-minute resolution is acceptable for alpha
3. **Deprecation is critical** — Prevents users from installing broken version
4. **Documentation matters** — This report ensures we learn from the incident

---

## Appendix: Error Message

Users who installed v0.2.0-alpha.1 would see:

```
Error [ERR_MODULE_NOT_FOUND]: Cannot find module
'C:\...\node_modules\edgevec\snippets\edgevec-99205e7bace4fae1\src\js\storage.js'
imported from C:\...\node_modules\edgevec\edgevec.js
```

---

## Sign-Off

| Role | Name | Date |
|:-----|:-----|:-----|
| Incident Reporter | Claude Code | 2025-12-12 |
| Root Cause Analyst | Claude Code | 2025-12-12 |
| Fix Implementer | Human + Claude Code | 2025-12-12 |
| Reviewed By | HOSTILE_REVIEWER | 2025-12-12 |

---

**Document Status:** [COMPLETE]
**Last Updated:** 2025-12-12
