# Day 1: Research & Design

**Date:** 2026-01-13
**Focus:** W33.1.1 — Design Typed Filter API
**Hours:** 2h

---

## Objectives

1. Analyze existing filter system
2. Design functional composition API
3. Document type signatures

---

## Tasks

### Task 1.1: Analyze Current Filter System (30 min)

**Files to Read:**
- `pkg/filter.ts` — Understand Filter class
- `pkg/filter-builder.ts` — Understand FilterBuilder

**Questions to Answer:**
- How does Filter.eq() currently work?
- What is FilterExpression type?
- How do filters compose currently?

---

### Task 1.2: Design Functional API (1h)

**Create:** `docs/planning/weeks/week_33/FILTER_FUNCTIONS_DESIGN.md`

**Content:**

```typescript
// Target API Design

// 1. Comparison Functions
eq(field: string, value: MetadataValue): FilterExpression
ne(field: string, value: MetadataValue): FilterExpression
gt(field: string, value: number): FilterExpression
lt(field: string, value: number): FilterExpression
ge(field: string, value: number): FilterExpression
le(field: string, value: number): FilterExpression
between(field: string, low: number, high: number): FilterExpression

// 2. String Functions
contains(field: string, substring: string): FilterExpression
startsWith(field: string, prefix: string): FilterExpression
endsWith(field: string, suffix: string): FilterExpression
like(field: string, pattern: string): FilterExpression

// 3. Array Functions
inArray(field: string, values: MetadataValue[]): FilterExpression
notInArray(field: string, values: MetadataValue[]): FilterExpression
any(field: string, value: MetadataValue): FilterExpression
all(field: string, values: MetadataValue[]): FilterExpression
none(field: string, values: MetadataValue[]): FilterExpression

// 4. Null Functions
isNull(field: string): FilterExpression
isNotNull(field: string): FilterExpression

// 5. Logical Combinators
and(...filters: FilterExpression[]): FilterExpression
or(...filters: FilterExpression[]): FilterExpression
not(filter: FilterExpression): FilterExpression

// 6. Top-level Wrapper
filter(expression: FilterExpression): FilterExpression
```

**Design Decisions:**
- Use existing Filter class methods internally
- Functions return FilterExpression (same as Filter methods)
- Support variadic and/or for multiple conditions
- Keep names consistent with SQL conventions

---

### Task 1.3: Document Implementation Plan (30 min)

**Update:** This file with implementation notes

**Decisions:**
- [ ] Function naming: `inArray` vs `isIn` vs `in_`
- [ ] Export strategy: named exports vs namespace
- [ ] Backward compatibility: ensure existing API unchanged

---

## Verification

- [ ] Design document created
- [ ] All function signatures defined
- [ ] No breaking changes to existing API
- [ ] Ready for Day 2 implementation

---

## Notes

_Fill during work:_

---

**Status:** PENDING
