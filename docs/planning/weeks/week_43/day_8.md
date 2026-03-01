# Week 43 — Day 8: Hostile Review Fixes / Overflow Buffer

**Date:** 2026-03-14
**Status:** PENDING
**Focus:** Fix all critical and major hostile review findings; use as overflow buffer if no fixes needed
**Prerequisite:** Day 7 complete (hostile review verdict)
**Reference:** `docs/reviews/2026-03-13_langchain_package.md`

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Fix all critical hostile reviewer findings | W43.8a | 3h | PENDING | Day 7 (W43.7a) |
| Fix all major hostile reviewer findings | W43.8b | 2h | PENDING | Day 7 (W43.7a) |
| Resubmit for re-review if needed | W43.8c | 1h | PENDING | W43.8a, W43.8b |

**Total Estimated Hours:** 6h (worst case)

---

## Conditional Execution

This day is **conditional** on the Day 7 hostile review outcome:

### Path A: NO_GO / GO-with-conditions (Critical/Major findings)

```
W43.8a (fix critical) → W43.8b (fix major) → W43.8c (re-review)
```

1. Read review report carefully
2. Address ALL critical issues first
3. Address ALL major issues
4. Run full test suite after fixes: `npx vitest run`
5. Run lint + type check: `npx tsc --noEmit`
6. Resubmit for re-review via `/review`

### Path B: GO (No critical/major findings)

If Day 7 verdict is GO, use Day 8 for:
- Fix minor findings (polish)
- Additional test coverage (edge cases from review feedback)
- Documentation improvements
- Early start on W44 prep (npm publish planning)

---

## Fix Protocol

For each finding:

1. **Read** the finding and understand the root cause
2. **Fix** the code/test/doc
3. **Verify** the fix addresses the finding (not just the symptom)
4. **Test** that the fix doesn't break existing tests
5. **Document** what changed and why

### Commit Convention

```
fix(w43): address hostile reviewer finding [severity] — [brief description]
```

Examples:
- `fix(w43): address hostile reviewer finding [CRITICAL] — add missing null check in delete`
- `fix(w43): address hostile reviewer finding [MAJOR] — fix score normalization for dot product`

---

## Re-review Protocol (W43.8c)

If re-review is needed:

1. Only changed artifacts are re-reviewed (not full package)
2. Previous findings are checked against new code
3. New issues from the fix are checked
4. Verdict: GO / NO_GO

**If second NO_GO:** Escalate to Day 9 for deep rework. This has never happened in project history but plan exists.

---

## Acceptance Criteria

- [ ] All critical findings fixed (zero remaining)
- [ ] All major findings fixed (zero remaining)
- [ ] `npx vitest run` — all tests passing after fixes
- [ ] `npx tsc --noEmit` — zero TypeScript errors after fixes
- [ ] Re-review verdict: GO (if re-review was needed)
- [ ] Fixes committed with proper commit messages

### If Path B (overflow):
- [ ] Minor findings fixed
- [ ] Additional test coverage added (if identified)
- [ ] Documentation polished

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| Critical finding requires architectural change | Day 9 provides additional buffer; simplify if needed |
| Fix introduces regression | Run full test suite after every fix |
| Re-review finds new issues from fixes | Fixes should be minimal and targeted |

---

## Exit Criteria

**Day 8 is complete when:**
1. All critical + major findings are addressed
2. All tests pass
3. Re-review (if needed) returns GO
4. All fixes committed

**Handoff to Day 9:** Package is review-approved. Day 9 runs final validation and closes the week.

---

**END OF DAY 8 PLAN**
