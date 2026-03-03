# Week 45 — Day 2 Tasks (Tuesday, Mar 25)

**Date:** 2026-03-25
**Focus:** LangChain.js Documentation + v0.2.0 Release Prep
**Agent:** DOCWRITER + WASM_SPECIALIST
**Status:** PENDING

---

## Day Objective

Create comprehensive FilterExpression usage documentation with real-world examples. Prepare the edgevec-langchain package for v0.2.0 publish. Update CHANGELOG and verify build output.

**Success Criteria:**
- Usage guide with 3+ real-world examples complete
- README updated with expanded code examples
- package.json bumped to 0.2.0
- Backward compatibility verified (existing `string` filter code compiles against v0.2.0 types)
- Build output verified (ESM + CJS, size under 10KB each)
- CHANGELOG v0.2.0 entry drafted
- ROADMAP.md updated to reflect PQ research pulled forward to W45

---

## Tasks

### W45.2a: FilterExpression Usage Guide (2h)

**Description:** Write a comprehensive guide showing how to use FilterExpression with edgevec-langchain for common retrieval patterns.

**Content Structure:**
1. **Introduction** — Why FilterExpression vs DSL strings
2. **Basic Usage** — Simple equality, comparison filters
3. **Combining Filters** — AND, OR, NOT composition
4. **Real-World Examples:**
   - Semantic search with metadata constraints (e-commerce product search)
   - RAG pipeline with document type filtering
   - Multi-tenant vector store with tenant isolation
   - Time-bounded search (recent documents only)
5. **Migration from DSL Strings** — Side-by-side comparison
6. **TypeScript Types Reference** — Key interfaces and methods

**Files:**
- `pkg/langchain/docs/FILTER_GUIDE.md` (new file)

**Acceptance:**
- [ ] 3+ real-world examples with complete, runnable code
- [ ] Each example explains the use case and expected behavior
- [ ] Migration section shows DSL string equivalent for each FilterExpression example

### W45.2b: README Code Examples Update (1h)

**Description:** Enhance the main edgevec-langchain README with more practical code examples demonstrating the full workflow.

**Updates:**
1. **Quick Start** — Expand with FilterExpression example alongside DSL string
2. **Advanced Usage** — Add section showing LangChain retriever chain with filters
3. **API Quick Reference** — Add concise table of all Filter factory methods with one-liner examples

**Files:**
- `pkg/langchain/README.md`

**Acceptance:**
- [ ] Quick Start includes both filter forms
- [ ] Advanced usage shows a realistic retriever chain
- [ ] All Filter factory methods listed with examples

### W45.2c: CHANGELOG v0.2.0 Entry (0.5h)

**Description:** Draft the CHANGELOG entry for edgevec-langchain@0.2.0.

**Content:**
- FilterExpression object support (from W44)
- New edge case tests
- Usage guide and documentation
- Filter re-exports from package root

**Files:**
- `pkg/langchain/CHANGELOG.md` (new file, or update existing)

**Acceptance:**
- [ ] Follows Keep a Changelog format
- [ ] Lists all user-facing changes

### W45.2d: Version Bump to 0.2.0 (0.5h)

**Description:** Update package.json version, verify peer dependencies, update any version references.

**Changes:**
1. `pkg/langchain/package.json` — `"version": "0.1.0"` → `"version": "0.2.0"`
2. `pkg/langchain/src/index.ts` — `@version 0.1.0` → `@version 0.2.0`
3. Verify `@langchain/core` peer dependency range still valid

**Files:**
- `pkg/langchain/package.json`
- `pkg/langchain/src/index.ts`

**Backward Compatibility Verification:**
The `FilterType` widening from `string` to `string | FilterExpression` is additive at the value level. Verify that existing user code compiles without changes:
```typescript
// This must still compile without errors:
const store = new EdgeVecStore(...);
const results = await store.similaritySearchVectorWithScore(query, k, "category = 'docs'");
```
Document the type widening as a known non-breaking change in the CHANGELOG.

**Acceptance:**
- [ ] Version is 0.2.0 in package.json
- [ ] Version comment updated in index.ts
- [ ] Peer dependency range unchanged (no breaking changes in @langchain/core)
- [ ] Existing `string` filter usage compiles against v0.2.0 types (verified via typecheck)

### W45.2e: Build Output Verification (0.5h)

**Description:** Build the package and verify output sizes and correctness.

**Commands:**
```bash
cd pkg/langchain
npm run build          # tsup build
ls -la dist/           # Check output files
```

**Checks:**
1. `dist/index.js` (ESM) exists and < 10KB
2. `dist/index.cjs` (CJS) exists and < 10KB
3. `dist/index.d.ts` type declarations include FilterExpression
4. No unexpected files in dist/

**Acceptance:**
- [ ] ESM output < 10KB
- [ ] CJS output < 10KB
- [ ] Type declarations include Filter and FilterExpression exports
- [ ] `npm pack --dry-run` shows expected file list

### W45.2f: Update ROADMAP.md — PQ Schedule Pull-Forward (0.5h)

**Description:** Update ROADMAP.md Milestone 10.4 to reflect that PQ Phase 1 (literature review) was pulled forward from W46 to W45. This prevents the document drift pattern caught in W44 Round 2.

**Changes:**
1. `docs/planning/ROADMAP.md` line 458: Change "Phase 1: Research (8h, Week 46)" to "Phase 1: Research (8h, Week 45 — pulled forward)"
2. Update Milestone 10.4 status from "RESEARCH" to "IN PROGRESS (Phase 1: W45 literature review)"
3. Add note explaining the pull-forward rationale (W44 NO-GO freed capacity)

**Acceptance:**
- [ ] ROADMAP Milestone 10.4 reflects W45 schedule
- [ ] Rationale for pull-forward documented
- [ ] No other ROADMAP sections affected

---

## Day 2 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~5h |
| New files | 1 (FILTER_GUIDE.md) |
| Modified files | 4 (README.md, package.json, index.ts, ROADMAP.md) |
| v0.2.0 ready for review | YES |

---

## Handoff

After Day 2 completion:
- **Status:** PENDING_HOSTILE_REVIEW (bundled with Day 5 review)
- **Next:** Day 3 — Product Quantization Literature Review
- **Artifacts:** `FILTER_GUIDE.md`, updated README, v0.2.0 package ready

---

**END OF DAY 2 TASKS**
