# Week 45 — Day 1 Tasks (Monday, Mar 24)

**Date:** 2026-03-24
**Focus:** LangChain.js FilterExpression Edge Cases + Testing
**Agent:** TEST_ENGINEER + RUST_ENGINEER
**Status:** PENDING

---

## Day Objective

Harden the FilterExpression integration with comprehensive edge case tests. The W44 implementation added 6 happy-path tests; Day 1 adds adversarial and boundary tests to ensure robustness before v0.2.0 publish.

**Success Criteria:**
- 15 new `it()` blocks added to `pkg/langchain/tests/store.test.ts` (149 total)
- All edge cases documented in test descriptions
- Full regression passes (Rust 980+ / LangChain 149)
- No compilation or runtime regressions

---

## Tasks

### W45.1a: FilterExpression Edge Case Tests (2h)

**Description:** Add tests for boundary conditions in FilterExpression handling.

**Test Cases:**
1. **Empty AND/OR:** `Filter.and([])` — should handle gracefully (tautology/contradiction)
2. **Deeply nested filters:** AND(OR(eq, ne), AND(gt, lt)) — verify depth 3+ works
3. **Unicode field names:** `Filter.eq("città", "Milano")` — non-ASCII metadata keys
4. **Special characters in values:** Values containing quotes, backslashes, null bytes
5. **Numeric edge cases:** `Filter.eq("score", 0)`, `Filter.eq("score", -1)`, `Filter.eq("score", Infinity)`

**Files:**
- `pkg/langchain/tests/store.test.ts` — new `describe("FilterExpression edge cases")`

**Acceptance:**
- [ ] All 5 test cases pass
- [ ] Each test has descriptive name explaining the edge case

### W45.1b: Null/Undefined Filter Handling Tests (1h)

**Description:** Verify that null, undefined, and empty string filters are handled correctly at the adapter boundary.

**Test Cases:**
1. **Explicit `null` filter:** `similaritySearchVectorWithScore(query, k, null)` — should search without filter
2. **Empty string filter:** `similaritySearchVectorWithScore(query, k, "")` — should search without filter or throw clear error
3. **Whitespace-only filter:** `similaritySearchVectorWithScore(query, k, "   ")` — same behavior as empty

**Files:**
- `pkg/langchain/tests/store.test.ts` — extend existing filter tests

**Acceptance:**
- [ ] All 3 test cases pass
- [ ] Behavior is consistent and documented

### W45.1c: Complex Nested Filter Tests (1.5h)

**Description:** Test real-world complex filter patterns that users will actually write.

**Test Cases:**
1. **E-commerce pattern:** `Filter.and([Filter.eq("category", "electronics"), Filter.ge("price", 100), Filter.le("price", 500)])`
2. **Multi-tenant pattern:** `Filter.and([Filter.eq("tenant_id", "org_123"), Filter.or([Filter.eq("status", "active"), Filter.eq("status", "pending")])])`
3. **Date range pattern:** `Filter.and([Filter.ge("created_at", "2026-01-01"), Filter.lt("created_at", "2026-04-01")])`
4. **Negation pattern:** `Filter.and([Filter.eq("type", "document"), Filter.not(Filter.eq("deleted", true))])` — `Filter.not` is confirmed in `pkg/filter.js`

**Files:**
- `pkg/langchain/tests/store.test.ts` — new `describe("FilterExpression real-world patterns")`

**Acceptance:**
- [ ] All patterns compile and produce valid filter expressions
- [ ] Mock search receives the correct serialized filter

### W45.1d: FilterExpression Error Handling (1h)

**Description:** Verify behavior when FilterExpression objects are malformed or have unexpected shapes.

**Test Cases:**
1. **Missing `_json` field:** Object without `_json` property
2. **Invalid `toJSON` return:** `toJSON` returns non-object
3. **Type coercion:** Plain object passed as FilterExpression (not from Filter factory)

**Note:** Since the adapter passes filters through to WASM without transformation, these tests verify that the TypeScript type system prevents misuse at compile time, and document runtime behavior for JavaScript users.

**Files:**
- `pkg/langchain/tests/store.test.ts` — extend edge case describe block

**Acceptance:**
- [ ] Runtime behavior documented for each case
- [ ] TypeScript compiler rejects invalid types (verified via `// @ts-expect-error` annotations)

### W45.1e: Full Regression (0.5h)

**Description:** Run complete test suites to verify no regressions.

**Commands:**
```bash
cargo test --lib                     # 980+ Rust tests
cargo clippy -- -D warnings          # Clean linting
cd pkg/langchain && npx vitest run   # 142+ LangChain tests
```

**Acceptance:**
- [ ] All Rust lib tests pass
- [ ] Clippy clean
- [ ] All LangChain tests pass (149 after 15 new additions)

---

## Day 1 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6h |
| New `it()` blocks | 15 (5 + 3 + 4 + 3) |
| Files modified | 1 (`store.test.ts`) |
| Regressions allowed | 0 |

---

## Handoff

After Day 1 completion:
- **Status:** PENDING_HOSTILE_REVIEW (bundled with Day 5 review)
- **Next:** Day 2 — Documentation + v0.2.0 prep
- **Artifacts:** Updated `pkg/langchain/tests/store.test.ts`

---

**END OF DAY 1 TASKS**
