# Functional Filter API

**Version:** EdgeVec v0.8.0
**Last Updated:** 2026-01-08

---

## Overview

EdgeVec v0.8.0 introduces **standalone filter functions** as an alternative to string-based filter expressions. This functional API provides:

- **Type safety** — Compile-time validation with TypeScript
- **Composability** — Build complex filters from simple functions
- **IDE support** — Autocomplete and inline documentation
- **Readability** — Clear, declarative filter construction

---

## Quick Start

```typescript
import { filter, and, eq, gt, lt } from 'edgevec';

// Simple filter
const byCategory = eq('category', 'electronics');

// Composed filter
const query = filter(
  and(
    eq('category', 'electronics'),
    gt('price', 100),
    lt('price', 1000)
  )
);

// Use in search
const results = await db.search(embedding, 10, { filter: query });
```

---

## Functional vs String Syntax

| Aspect | String Syntax | Functional API |
|:-------|:--------------|:---------------|
| **Type safety** | None (runtime errors) | Full TypeScript support |
| **Composition** | String concatenation | Function composition |
| **IDE support** | Minimal | Autocomplete, hover docs |
| **Readability** | Good for simple filters | Better for complex logic |
| **Performance** | Identical | Identical (compiled to same AST) |

**When to use each:**

- **String syntax**: Quick, simple filters in scripts or REPL
- **Functional API**: Production code, complex filters, TypeScript projects

---

## Filter Functions Reference

### Comparison Functions

#### `eq(field, value)` — Equals

Tests for exact equality.

```typescript
import { eq } from 'edgevec';

// String equality
eq('category', 'electronics')  // category = "electronics"

// Number equality
eq('count', 42)  // count = 42

// Boolean equality
eq('active', true)  // active = true
```

#### `ne(field, value)` — Not Equals

Tests that a field does not equal a value.

```typescript
import { ne } from 'edgevec';

ne('status', 'deleted')  // status != "deleted"
ne('error_count', 0)     // error_count != 0
```

#### `gt(field, value)` — Greater Than

Tests that a field is strictly greater than a value.

```typescript
import { gt } from 'edgevec';

gt('price', 100)      // price > 100
gt('rating', 4.0)     // rating > 4.0
gt('stock', 0)        // stock > 0
```

#### `gte(field, value)` — Greater Than or Equal

Tests that a field is greater than or equal to a value.

```typescript
import { gte } from 'edgevec';

gte('rating', 4.0)    // rating >= 4.0
gte('version', 2)     // version >= 2
```

#### `lt(field, value)` — Less Than

Tests that a field is strictly less than a value.

```typescript
import { lt } from 'edgevec';

lt('price', 500)      // price < 500
lt('distance', 10.5)  // distance < 10.5
```

#### `lte(field, value)` — Less Than or Equal

Tests that a field is less than or equal to a value.

```typescript
import { lte } from 'edgevec';

lte('priority', 3)    // priority <= 3
lte('score', 0.95)    // score <= 0.95
```

#### `between(field, low, high)` — Range

Tests that a field value falls within an inclusive range.

```typescript
import { between } from 'edgevec';

between('price', 10, 100)      // price BETWEEN 10 AND 100
between('rating', 3.0, 5.0)    // rating BETWEEN 3.0 AND 5.0
between('year', 2020, 2025)    // year BETWEEN 2020 AND 2025
```

---

### String Functions

#### `contains(field, substring)` — Substring Match

Tests that a string field contains a substring.

```typescript
import { contains } from 'edgevec';

contains('title', 'vector')           // title CONTAINS "vector"
contains('description', 'machine')    // description CONTAINS "machine"
```

#### `startsWith(field, prefix)` — Prefix Match

Tests that a string field starts with a prefix.

```typescript
import { startsWith } from 'edgevec';

startsWith('path', '/home/')    // path STARTS_WITH "/home/"
startsWith('sku', 'PROD-')      // sku STARTS_WITH "PROD-"
```

#### `endsWith(field, suffix)` — Suffix Match

Tests that a string field ends with a suffix.

```typescript
import { endsWith } from 'edgevec';

endsWith('filename', '.pdf')         // filename ENDS_WITH ".pdf"
endsWith('email', '@company.com')    // email ENDS_WITH "@company.com"
```

#### `like(field, pattern)` — Pattern Match

Pattern matching with wildcards (`%` for any characters, `_` for single character).

```typescript
import { like } from 'edgevec';

like('name', 'John%')       // name LIKE "John%"
like('email', '%@gmail.com') // email LIKE "%@gmail.com"
like('code', 'A_B')         // code LIKE "A_B" (matches A1B, AXB, etc.)
```

---

### Array Functions

#### `inArray(field, values)` — Value in Set

Tests that a field value is contained in a set of values.

```typescript
import { inArray } from 'edgevec';

inArray('category', ['electronics', 'computers'])  // category IN [...]
inArray('author_id', [1, 2, 3, 4, 5])              // author_id IN [...]
inArray('status', ['active', 'pending'])           // status IN [...]
```

#### `notInArray(field, values)` — Value Not in Set

Tests that a field value is not contained in a set.

```typescript
import { notInArray } from 'edgevec';

notInArray('category', ['spam', 'test'])  // category NOT IN [...]
notInArray('user_id', [0, -1])            // user_id NOT IN [...]
```

#### `any(field, values)` — Array Contains Any

Tests that an array field contains any of the specified values.

```typescript
import { any } from 'edgevec';

// Metadata: { tags: ['featured', 'sale', 'new'] }
any('tags', ['featured', 'popular'])  // tags contains 'featured' OR 'popular'
```

#### `all(field, values)` — Array Contains All

Tests that an array field contains all of the specified values.

```typescript
import { all } from 'edgevec';

// Must have both tags
all('tags', ['featured', 'sale'])  // tags contains 'featured' AND 'sale'
```

#### `none(field, values)` — Array Contains None

Tests that an array field contains none of the specified values.

```typescript
import { none } from 'edgevec';

// Must not have any of these tags
none('tags', ['spam', 'test'])  // tags contains neither 'spam' nor 'test'
```

---

### Null Functions

#### `isNull(field)` — Field is Null

Tests that a field is null or does not exist.

```typescript
import { isNull } from 'edgevec';

isNull('description')  // description IS NULL
isNull('category')     // category IS NULL
```

#### `isNotNull(field)` — Field Exists

Tests that a field exists and is not null.

```typescript
import { isNotNull } from 'edgevec';

isNotNull('description')  // description IS NOT NULL
isNotNull('category')     // category IS NOT NULL
```

---

### Logical Functions

#### `and(...conditions)` — All Conditions True

Combines conditions that must all be true.

```typescript
import { and, eq, gt, lt } from 'edgevec';

// Two conditions
and(
  eq('category', 'electronics'),
  gt('price', 100)
)

// Three or more conditions
and(
  eq('status', 'active'),
  gte('rating', 4.0),
  gt('stock', 0)
)
```

#### `or(...conditions)` — Any Condition True

Combines conditions where at least one must be true.

```typescript
import { or, eq } from 'edgevec';

// Either category
or(
  eq('category', 'gpu'),
  eq('category', 'cpu')
)

// Multiple options
or(
  eq('status', 'active'),
  eq('status', 'pending'),
  eq('status', 'review')
)
```

#### `not(condition)` — Negate Condition

Negates a condition.

```typescript
import { not, eq } from 'edgevec';

not(eq('deleted', true))    // NOT (deleted = true)
not(eq('category', 'spam')) // NOT (category = "spam")
```

---

### Special Functions

#### `filter(expression)` — Wrap Expression

Wraps a filter expression for use in search options.

```typescript
import { filter, and, eq, gt } from 'edgevec';

const query = filter(
  and(
    eq('category', 'electronics'),
    gt('price', 100)
  )
);

// Use in search
const results = await db.search(embedding, 10, { filter: query });
```

#### `matchAll()` — Match Everything

Returns a filter that matches all vectors.

```typescript
import { matchAll } from 'edgevec';

const allResults = await db.search(embedding, 10, { filter: matchAll() });
```

#### `matchNone()` — Match Nothing

Returns a filter that matches no vectors.

```typescript
import { matchNone } from 'edgevec';

// Useful for conditional filtering
const query = shouldFilter ? myFilter : matchNone();
```

---

## Composition Patterns

### Basic Composition

```typescript
import { filter, and, or, eq, gt, lt, between } from 'edgevec';

// E-commerce product search
const productFilter = filter(
  and(
    eq('category', 'electronics'),
    between('price', 100, 500),
    gte('rating', 4.0),
    eq('in_stock', true)
  )
);
```

### Nested Logic

```typescript
import { filter, and, or, not, eq, gt } from 'edgevec';

// Complex boolean logic
const complexFilter = filter(
  and(
    or(
      eq('category', 'gpu'),
      eq('category', 'cpu')
    ),
    lt('price', 1000),
    not(eq('status', 'discontinued'))
  )
);
```

### Dynamic Filter Building

```typescript
import { filter, and, eq, gt, inArray, matchAll } from 'edgevec';

function buildFilter(options: {
  category?: string;
  minPrice?: number;
  brands?: string[];
}) {
  const conditions = [];

  if (options.category) {
    conditions.push(eq('category', options.category));
  }

  if (options.minPrice !== undefined) {
    conditions.push(gt('price', options.minPrice));
  }

  if (options.brands?.length) {
    conditions.push(inArray('brand', options.brands));
  }

  return conditions.length > 0
    ? filter(and(...conditions))
    : matchAll();
}

// Usage
const searchFilter = buildFilter({
  category: 'electronics',
  minPrice: 100,
  brands: ['Apple', 'Samsung']
});
```

### Reusable Filter Components

```typescript
import { and, or, eq, gt, gte, not } from 'edgevec';

// Define reusable filter components
const isActive = eq('status', 'active');
const isHighRated = gte('rating', 4.0);
const isInStock = gt('stock', 0);
const isNotDeleted = not(eq('deleted', true));

// Compose into complex filters
const featuredProducts = and(
  isActive,
  isHighRated,
  isInStock,
  isNotDeleted
);

const budgetProducts = and(
  isActive,
  lt('price', 50),
  isInStock
);
```

---

## TypeScript Types

```typescript
// Filter expression type
type FilterExpression = string;

// Function signatures
declare function eq(field: string, value: string | number | boolean): FilterExpression;
declare function ne(field: string, value: string | number | boolean): FilterExpression;
declare function gt(field: string, value: number): FilterExpression;
declare function gte(field: string, value: number): FilterExpression;
declare function lt(field: string, value: number): FilterExpression;
declare function lte(field: string, value: number): FilterExpression;
declare function between(field: string, low: number, high: number): FilterExpression;

declare function contains(field: string, substring: string): FilterExpression;
declare function startsWith(field: string, prefix: string): FilterExpression;
declare function endsWith(field: string, suffix: string): FilterExpression;
declare function like(field: string, pattern: string): FilterExpression;

declare function inArray(field: string, values: (string | number)[]): FilterExpression;
declare function notInArray(field: string, values: (string | number)[]): FilterExpression;
declare function any(field: string, values: (string | number)[]): FilterExpression;
declare function all(field: string, values: (string | number)[]): FilterExpression;
declare function none(field: string, values: (string | number)[]): FilterExpression;

declare function isNull(field: string): FilterExpression;
declare function isNotNull(field: string): FilterExpression;

declare function and(...conditions: FilterExpression[]): FilterExpression;
declare function or(...conditions: FilterExpression[]): FilterExpression;
declare function not(condition: FilterExpression): FilterExpression;

declare function filter(expression: FilterExpression): FilterExpression;
declare function matchAll(): FilterExpression;
declare function matchNone(): FilterExpression;
```

---

## Migration from String Syntax

| String Syntax | Functional API |
|:--------------|:---------------|
| `'category = "gpu"'` | `eq('category', 'gpu')` |
| `'price > 100'` | `gt('price', 100)` |
| `'rating >= 4.0'` | `gte('rating', 4.0)` |
| `'price BETWEEN 10 AND 100'` | `between('price', 10, 100)` |
| `'category IN ["a", "b"]'` | `inArray('category', ['a', 'b'])` |
| `'title CONTAINS "vector"'` | `contains('title', 'vector')` |
| `'a = 1 AND b = 2'` | `and(eq('a', 1), eq('b', 2))` |
| `'a = 1 OR b = 2'` | `or(eq('a', 1), eq('b', 2))` |
| `'NOT (deleted = true)'` | `not(eq('deleted', true))` |

---

## See Also

- [Filter Syntax Reference](./FILTER_SYNTAX.md) — String-based filter syntax
- [Filter Examples](../guides/FILTER_EXAMPLES.md) — 25 copy-paste ready examples
- [TypeScript API](./TYPESCRIPT_API.md) — Complete type definitions
- [Vue Integration](../guides/VUE_INTEGRATION.md) — Using filters with Vue
- [React Integration](../guides/REACT_INTEGRATION.md) — Using filters with React
