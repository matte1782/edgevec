# Day 2: Filter Functions Implementation

**Date:** 2026-01-14
**Focus:** W33.1.2, W33.1.3 — Implement Filter Functions
**Hours:** 2h

---

## Objectives

1. Implement comparison functions
2. Implement logical combinators
3. Write unit tests

---

## Tasks

### Task 2.1: Create filter-functions.ts (1h)

**Create:** `pkg/filter-functions.ts`

```typescript
/**
 * EdgeVec Filter Functions - Functional Composition API
 *
 * Provides standalone functions for building filters with functional composition.
 *
 * @module filter-functions
 * @version 0.8.0
 *
 * @example
 * import { filter, and, eq, gt } from 'edgevec';
 *
 * const query = filter(
 *   and(
 *     eq('category', 'electronics'),
 *     gt('price', 100)
 *   )
 * );
 */

import { Filter, FilterExpression, MetadataValue } from './filter.js';

// =============================================================================
// Comparison Functions
// =============================================================================

/** Equal to */
export function eq(field: string, value: MetadataValue): FilterExpression {
  return Filter.eq(field, value);
}

/** Not equal to */
export function ne(field: string, value: MetadataValue): FilterExpression {
  return Filter.ne(field, value);
}

/** Greater than */
export function gt(field: string, value: number): FilterExpression {
  return Filter.gt(field, value);
}

/** Less than */
export function lt(field: string, value: number): FilterExpression {
  return Filter.lt(field, value);
}

/** Greater than or equal */
export function ge(field: string, value: number): FilterExpression {
  return Filter.ge(field, value);
}

/** Less than or equal */
export function le(field: string, value: number): FilterExpression {
  return Filter.le(field, value);
}

/** Between (inclusive) */
export function between(field: string, low: number, high: number): FilterExpression {
  return Filter.between(field, low, high);
}

// =============================================================================
// String Functions
// =============================================================================

/** Contains substring */
export function contains(field: string, substring: string): FilterExpression {
  return Filter.contains(field, substring);
}

/** Starts with prefix */
export function startsWith(field: string, prefix: string): FilterExpression {
  return Filter.startsWith(field, prefix);
}

/** Ends with suffix */
export function endsWith(field: string, suffix: string): FilterExpression {
  return Filter.endsWith(field, suffix);
}

/** LIKE pattern match */
export function like(field: string, pattern: string): FilterExpression {
  return Filter.like(field, pattern);
}

// =============================================================================
// Array Functions
// =============================================================================

/** In array of values */
export function inArray(field: string, values: MetadataValue[]): FilterExpression {
  return Filter.in(field, values);
}

/** Not in array of values */
export function notInArray(field: string, values: MetadataValue[]): FilterExpression {
  return Filter.notIn(field, values);
}

/** ANY - array field contains value */
export function any(field: string, value: MetadataValue): FilterExpression {
  return Filter.any(field, value);
}

/** ALL - array field contains all values */
export function all(field: string, values: MetadataValue[]): FilterExpression {
  return Filter.allOf(field, values);
}

/** NONE - array field contains none of values */
export function none(field: string, values: MetadataValue[]): FilterExpression {
  return Filter.none(field, values);
}

// =============================================================================
// Null Functions
// =============================================================================

/** Is null */
export function isNull(field: string): FilterExpression {
  return Filter.isNull(field);
}

/** Is not null */
export function isNotNull(field: string): FilterExpression {
  return Filter.isNotNull(field);
}

// =============================================================================
// Logical Combinators
// =============================================================================

/** AND - all conditions must match */
export function and(...filters: FilterExpression[]): FilterExpression {
  if (filters.length === 0) {
    throw new Error('and() requires at least one filter');
  }
  if (filters.length === 1) {
    return filters[0];
  }
  return filters.reduce((acc, f) => Filter.and(acc, f));
}

/** OR - any condition must match */
export function or(...filters: FilterExpression[]): FilterExpression {
  if (filters.length === 0) {
    throw new Error('or() requires at least one filter');
  }
  if (filters.length === 1) {
    return filters[0];
  }
  return filters.reduce((acc, f) => Filter.or(acc, f));
}

/** NOT - negate condition */
export function not(f: FilterExpression): FilterExpression {
  return Filter.not(f);
}

// =============================================================================
// Top-level Wrapper
// =============================================================================

/**
 * Identity wrapper for filter expressions.
 * Useful for readability at the top level.
 *
 * @example
 * const query = filter(and(eq('a', 1), gt('b', 2)));
 */
export function filter(expression: FilterExpression): FilterExpression {
  return expression;
}
```

---

### Task 2.2: Update index.ts Exports (15 min)

**Modify:** `pkg/index.ts`

Add exports:
```typescript
// Re-export filter functions
export {
  eq, ne, gt, lt, ge, le, between,
  contains, startsWith, endsWith, like,
  inArray, notInArray, any, all, none,
  isNull, isNotNull,
  and, or, not,
  filter
} from './filter-functions.js';
```

---

### Task 2.3: Write Unit Tests (45 min)

**Create:** `pkg/__tests__/filter-functions.test.ts` (or add to existing test file)

**Test Cases:**
- [ ] eq() creates correct expression
- [ ] gt/lt/ge/le work with numbers
- [ ] and() combines multiple filters
- [ ] or() combines multiple filters
- [ ] not() negates filter
- [ ] Nested and/or works
- [ ] Empty and/or throws error

---

## Verification

- [ ] `pkg/filter-functions.ts` created
- [ ] Exports added to `index.ts`
- [ ] TypeScript compiles: `npx tsc --noEmit`
- [ ] Unit tests pass
- [ ] No breaking changes to existing API

---

## Notes

_Fill during work:_

---

**Status:** PENDING
