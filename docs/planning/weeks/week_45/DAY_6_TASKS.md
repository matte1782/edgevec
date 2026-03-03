# Week 45 — Day 6 Tasks (Saturday, Mar 29)

**Date:** 2026-03-29
**Focus:** Fix Hostile Review Findings + Publish v0.2.0 + Gate
**Agent:** RUST_ENGINEER + DOCWRITER
**Status:** PENDING

---

## Day Objective

Fix all critical and major findings from the Day 5 hostile review. Publish edgevec-langchain@0.2.0 to npm (user handles OTP). Create the W45 gate file. Commit and push all work.

**Success Criteria:**
- All critical findings fixed
- All major findings fixed
- edgevec-langchain@0.2.0 published to npm
- All work committed and pushed
- `.claude/GATE_W45_COMPLETE.md` created

---

## Tasks

### W45.6a: Fix Critical Hostile Review Findings (up to 2h)

**Description:** Address all findings marked CRITICAL from `docs/reviews/2026-03-28_W45_HOSTILE_REVIEW.md`.

**Process:**
1. Read each critical finding carefully
2. Implement the fix
3. Verify fix doesn't introduce regressions
4. Mark as fixed in review document

**Acceptance:**
- [ ] All critical findings fixed
- [ ] Each fix verified with relevant test run
- [ ] No new regressions introduced

### W45.6b: Fix Major Hostile Review Findings (up to 2h)

**Description:** Address all findings marked MAJOR from the hostile review.

**Process:** Same as W45.6a.

**Acceptance:**
- [ ] All major findings fixed
- [ ] Each fix verified

### W45.6c: Publish edgevec-langchain@0.2.0 (0.5h)

**Description:** Publish the updated package to npm.

**Pre-publish Checklist:**
- [ ] Version is 0.2.0 in package.json
- [ ] All tests pass (`npx vitest run`)
- [ ] Build output clean (`npm run build`)
- [ ] `npm pack --dry-run` shows expected files
- [ ] No secrets or unnecessary files in package

**Publish:**
```bash
cd pkg/langchain
npm publish
# User provides OTP when prompted
```

**Post-publish Smoke Test:**
- [ ] Verify package visible on npmjs.com
- [ ] Verify install works: `npm install edgevec-langchain@0.2.0`
- [ ] Create scratch project: import `{ EdgeVecStore, Filter }`, verify TypeScript compiles
- [ ] Verify existing `string` filter usage compiles without changes

**Rollback Procedure (if regression found post-publish):**
1. `npm deprecate edgevec-langchain@0.2.0 "Regression found, use 0.1.0"` — marks version as deprecated
2. Fix the issue and publish `edgevec-langchain@0.2.1` hotfix
3. `npm deprecate edgevec-langchain@0.2.0 ""` — remove deprecation once 0.2.1 ships
4. Document incident in CHANGELOG

**Acceptance:**
- [ ] Package published and installable
- [ ] Version 0.2.0 visible on npm
- [ ] Post-publish smoke test passes (types compile, import works)

### W45.6d: Commit and Push All Work (0.5h)

**Description:** Create a clean commit with all W45 changes.

**Commit message format:**
```
feat(w45): edgevec-langchain v0.2.0 + PQ research + API audit

- FilterExpression edge case tests ([ACTUAL_COUNT] langchain tests)
- FilterExpression usage guide
- Product Quantization literature review
- API stability audit for v1.0 prep
- Hostile review: all critical + major fixed
```

**Acceptance:**
- [ ] All files staged (no forgotten changes)
- [ ] Commit message follows conventional commit style
- [ ] Push to main successful

### W45.6e: Create Gate File (0.5h)

**Description:** Create `.claude/GATE_W45_COMPLETE.md` documenting the sprint outcome.

**Content:**
- Sprint summary
- Hostile review results (initial + post-fix)
- Deliverables table with status
- Quality metrics (test counts, clippy)
- PQ research preliminary finding
- v0.2.0 publish status

**Acceptance:**
- [ ] All deliverables listed with final status
- [ ] Test counts accurate
- [ ] Hostile review summary included

---

## If Surplus Time Available

Priority order:
1. **Additional PQ research depth** — Deeper dive into specific PQ variant (OPQ, LOPQ) if relevant
2. **Begin W46 PQ implementation planning** — Draft task breakdown for codebook training
3. **Main README improvements** — Expand EdgeVec main README with more examples
4. **Blog post draft** — Draft blog post about edgevec-langchain for community engagement

---

## Day 6 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~5.5h (+ surplus if available) |
| Critical fixes | ALL |
| Major fixes | ALL |
| npm publish | edgevec-langchain@0.2.0 |
| Gate file | `.claude/GATE_W45_COMPLETE.md` |

---

## Sprint Closure

After Day 6 completion:
- **W45 Status:** COMPLETE
- **Next Sprint:** W46 — Product Quantization Implementation (if GO from PQ research)
- **Key Decision:** PQ GO/NO-GO finalized in W46 after running benchmarks designed in W45

---

**END OF DAY 6 TASKS**
