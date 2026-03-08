# NPM Pre-Publish Checklist — edgevec-langchain@0.2.0

**Date:** 2026-03-08
**Status:** [APPROVED] — ALREADY PUBLISHED (see Finding F6)

---

## 1. Version Check

| Check | Expected | Actual | Verdict |
|:------|:---------|:-------|:--------|
| `package.json` version | `0.2.0` | `0.2.0` | PASS |

**Source:** `pkg/langchain/package.json` line 3.

---

## 2. CHANGELOG Verification

| Check | Expected | Actual | Verdict |
|:------|:---------|:-------|:--------|
| v0.2.0 section exists | Yes | Yes | PASS |
| FilterExpression support documented | Yes | Yes (`FilterExpression` object support, `Filter` re-exports, 15 new tests, guide, quick reference) | PASS |
| Date present | Yes | `2026-03-29` | PASS (note: date is in the future relative to today 2026-03-08 — see Finding F1) |
| v0.1.0 section exists | Yes | Yes | PASS |

**Source:** `pkg/langchain/CHANGELOG.md` lines 8-19.

### Finding F1 — CHANGELOG date anomaly
The v0.2.0 date is `2026-03-29`, which is **21 days in the future** relative to today (2026-03-08). If the package is already published on npm, the CHANGELOG date should reflect the actual publish date. **Severity: Minor.**

---

## 3. README Verification

| Check | Expected | Actual | Verdict |
|:------|:---------|:-------|:--------|
| FilterExpression documented | Yes | Yes (Quick Start, Filter API table, DSL examples) | PASS |
| `Filter` import shown | Yes | Yes (`import { Filter } from "edgevec-langchain"`) | PASS |
| API reference current | Yes | Yes (constructor, factory methods, instance methods, Filter API) | PASS |
| Peer deps match package.json | `edgevec ^0.9.0`, `@langchain/core >=0.3.0 <0.5.0` | Match | PASS |
| Score normalization documented | Yes | Yes (cosine, l2, dotproduct formulas) | PASS |
| WASM init documented | Yes | Yes (auto + manual) | PASS |
| Persistence documented | Yes | Yes (IndexedDB save/load) | PASS |

---

## 4. package.json Entry Points

| Field | Value | Verdict |
|:------|:------|:--------|
| `main` | `./dist/index.cjs` | PASS |
| `module` | `./dist/index.js` | PASS |
| `types` | `./dist/index.d.ts` | PASS |
| `exports.".".import.types` | `./dist/index.d.ts` | PASS |
| `exports.".".import.default` | `./dist/index.js` | PASS |
| `exports.".".require.types` | `./dist/index.d.cts` | PASS |
| `exports.".".require.default` | `./dist/index.cjs` | PASS |
| `type` | `module` | PASS |
| `sideEffects` | `false` | PASS |
| `files` | `["dist"]` | PASS |
| `engines.node` | `>=20.0.0` | PASS |
| `license` | `MIT` | PASS |

---

## 5. Tarball Contents (`npm pack --dry-run`)

```
npm notice package: edgevec-langchain@0.2.0
npm notice Tarball Contents
npm notice 18.0kB README.md
npm notice  8.5kB dist/index.cjs
npm notice 45.1kB dist/index.cjs.map
npm notice 11.9kB dist/index.d.cts
npm notice 11.9kB dist/index.d.ts
npm notice  7.7kB dist/index.js
npm notice 44.8kB dist/index.js.map
npm notice  1.4kB package.json
npm notice Tarball Details
npm notice name:          edgevec-langchain
npm notice version:       0.2.0
npm notice filename:      edgevec-langchain-0.2.0.tgz
npm notice package size:  39.3 kB
npm notice unpacked size: 149.4 kB
npm notice total files:   8
```

| Check | Expected | Actual | Verdict |
|:------|:---------|:-------|:--------|
| Only `dist/` + `README.md` + `package.json` | Yes | Yes (8 files) | PASS |
| No test files included | None | None | PASS |
| No `src/` included | None | None | PASS |
| No `node_modules/` included | None | None | PASS |
| No `tsconfig.json` / config files | None | None | PASS |
| ESM bundle present (`dist/index.js`) | Yes | Yes (7.7kB) | PASS |
| CJS bundle present (`dist/index.cjs`) | Yes | Yes (8.5kB) | PASS |
| Type declarations present (`.d.ts`, `.d.cts`) | Yes | Yes (11.9kB each) | PASS |
| Source maps present (`.js.map`, `.cjs.map`) | Yes | Yes | PASS |
| Total bundle < 10KB (adapter code only) | ESM < 10KB, CJS < 10KB | 7.7kB + 8.5kB | PASS |

---

## 6. Publish Dry-Run (`npm publish --dry-run`)

```
npm error You cannot publish over the previously published versions: 0.2.0.
```

| Check | Result | Verdict |
|:------|:-------|:--------|
| Dry-run executed | Yes | PASS |
| Auth/OTP required | N/A (already published) | N/A |

### Finding F6 — Version 0.2.0 already published
`npm publish --dry-run` confirms that **edgevec-langchain@0.2.0 is already on the npm registry**. No further publish action is needed. The error `"You cannot publish over the previously published versions: 0.2.0"` is expected behavior.

### Warning W1 — repository.url normalization
npm auto-corrected `repository.url` from `https://github.com/matte1782/edgevec` to `git+https://github.com/matte1782/edgevec.git`. Consider running `npm pkg fix` to silence this warning in future publishes.

---

## 7. dist/ Directory Verification

```
dist/
├── index.cjs       (8,534 bytes)
├── index.cjs.map   (45,128 bytes)
├── index.d.cts     (11,897 bytes)
├── index.d.ts      (11,897 bytes)
├── index.js        (7,730 bytes)
└── index.js.map    (44,836 bytes)
```

| Check | Expected | Actual | Verdict |
|:------|:---------|:-------|:--------|
| ESM entry exists | `dist/index.js` | Yes | PASS |
| CJS entry exists | `dist/index.cjs` | Yes | PASS |
| ESM types exist | `dist/index.d.ts` | Yes | PASS |
| CJS types exist | `dist/index.d.cts` | Yes | PASS |
| Source maps exist | `.map` files | Yes (2 files) | PASS |
| No stale/extra files | 6 files only | 6 files | PASS |

---

## Summary

| Category | Pass | Fail | Findings |
|:---------|:-----|:-----|:---------|
| Version | 1 | 0 | — |
| CHANGELOG | 4 | 0 | F1 (minor: future date) |
| README | 7 | 0 | — |
| package.json entries | 12 | 0 | — |
| Tarball contents | 10 | 0 | — |
| Publish dry-run | 1 | 0 | F6 (already published), W1 (url normalization) |
| dist/ verification | 6 | 0 | — |
| **TOTAL** | **41** | **0** | **2 findings, 1 warning** |

### Overall Verdict: PASS

**edgevec-langchain@0.2.0 is already published to npm.** All checks pass. Two minor items noted:

1. **F1 (Minor):** CHANGELOG v0.2.0 date is `2026-03-29` but the package appears to already be published. Consider correcting the date to match the actual publish date.
2. **W1 (Warning):** Run `npm pkg fix` in `pkg/langchain/` to normalize `repository.url` and suppress the npm warning on future publishes.

No blocking issues found. No action required for publishing (already done).
