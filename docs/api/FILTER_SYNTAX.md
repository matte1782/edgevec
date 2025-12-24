# Filter Syntax Reference

**Version:** EdgeVec v0.7.0
**Last Updated:** 2025-12-24

> **Try it live:** [Filter Playground](https://matte1782.github.io/edgevec/demo/) — Build and test filters interactively in your browser

---

## Quick Reference

| Category | Operator | Example | Description |
|:---------|:---------|:--------|:------------|
| **Comparison** | `=` | `category = "gpu"` | Equals |
| | `!=` | `status != "deleted"` | Not equals |
| | `<` | `price < 100` | Less than |
| | `<=` | `rating <= 4.5` | Less than or equal |
| | `>` | `count > 0` | Greater than |
| | `>=` | `score >= 0.9` | Greater than or equal |
| **Range** | `BETWEEN` | `price BETWEEN 10 AND 100` | Inclusive range |
| **Set** | `IN` | `category IN ["a", "b"]` | Value in set |
| | `NOT IN` | `status NOT IN ["x", "y"]` | Value not in set |
| **String** | `CONTAINS` | `title CONTAINS "vector"` | Substring match |
| | `STARTS_WITH` | `name STARTS_WITH "test"` | Prefix match |
| | `ENDS_WITH` | `file ENDS_WITH ".rs"` | Suffix match |
| | `LIKE` | `name LIKE "user_%"` | Pattern match (% = wildcard) |
| **Null** | `IS NULL` | `description IS NULL` | Field is null/missing |
| | `IS NOT NULL` | `category IS NOT NULL` | Field exists |
| **Logical** | `AND` | `a = 1 AND b = 2` | Both conditions |
| | `OR` | `a = 1 OR b = 2` | Either condition |
| | `NOT` | `NOT (deleted = true)` | Negate condition |

---

## Comparison Operators

### `=` (Equals)

Tests for exact equality between a field and a value.

```javascript
// String equality
const filter = 'category = "electronics"';

// Integer equality
const filter = 'count = 42';

// Float equality
const filter = 'score = 0.95';

// Boolean equality
const filter = 'active = true';
```

**Supported Types:** String, Integer, Float, Boolean

---

### `!=` (Not Equals)

Tests that a field does not equal a value.

```javascript
// Exclude a category
const filter = 'status != "deleted"';

// Non-zero values
const filter = 'error_count != 0';
```

**Supported Types:** String, Integer, Float, Boolean

---

### `<` (Less Than)

Tests that a field is strictly less than a value.

```javascript
// Price under $100
const filter = 'price < 100';

// Score below threshold
const filter = 'similarity < 0.5';
```

**Supported Types:** Integer, Float

---

### `<=` (Less Than or Equal)

Tests that a field is less than or equal to a value.

```javascript
// Rating up to 4 stars
const filter = 'rating <= 4.0';

// Items within budget
const filter = 'price <= 99.99';
```

**Supported Types:** Integer, Float

---

### `>` (Greater Than)

Tests that a field is strictly greater than a value.

```javascript
// More than 10 items
const filter = 'quantity > 10';

// High confidence results
const filter = 'confidence > 0.9';
```

**Supported Types:** Integer, Float

---

### `>=` (Greater Than or Equal)

Tests that a field is greater than or equal to a value.

```javascript
// 4+ star ratings
const filter = 'rating >= 4.0';

// Minimum stock level
const filter = 'stock >= 5';
```

**Supported Types:** Integer, Float

---

## Range Operators

### `BETWEEN`

Tests that a field value falls within an inclusive range.

```javascript
// Price range $10-$100
const filter = 'price BETWEEN 10 AND 100';

// Date range (as integers)
const filter = 'timestamp BETWEEN 1700000000 AND 1800000000';

// Score range
const filter = 'rating BETWEEN 3.0 AND 5.0';
```

**Syntax:** `field BETWEEN low AND high` (inclusive on both ends)

**Supported Types:** Integer, Float

**Edge Cases:**
- `BETWEEN 5 AND 5` matches only the value 5
- `BETWEEN 10 AND 5` matches nothing (low > high)

---

## Set Operators

### `IN`

Tests that a field value is contained in a set of values.

```javascript
// Multiple categories
const filter = 'category IN ["electronics", "computers", "accessories"]';

// Specific IDs
const filter = 'author_id IN [1, 2, 3, 4, 5]';

// Status whitelist
const filter = 'status IN ["active", "pending"]';
```

**Syntax:** `field IN [value1, value2, ...]`

**Supported Types:** String, Integer, Float

**Performance Note:** IN with large arrays (>100 elements) may be slower than multiple OR conditions for small sets.

---

### `NOT IN`

Tests that a field value is not contained in a set of values.

```javascript
// Exclude categories
const filter = 'category NOT IN ["spam", "test", "deleted"]';

// Exclude user IDs
const filter = 'user_id NOT IN [0, -1]';
```

**Syntax:** `field NOT IN [value1, value2, ...]`

**Supported Types:** String, Integer, Float

---

## String Operators

### `CONTAINS`

Tests that a string field contains a substring.

```javascript
// Title contains word
const filter = 'title CONTAINS "vector"';

// Description contains phrase
const filter = 'description CONTAINS "machine learning"';
```

**Case Sensitivity:** Case-sensitive by default.

**Supported Types:** String only

---

### `STARTS_WITH`

Tests that a string field starts with a prefix.

```javascript
// Files in directory
const filter = 'path STARTS_WITH "/home/user/"';

// Product codes
const filter = 'sku STARTS_WITH "PROD-"';
```

**Supported Types:** String only

---

### `ENDS_WITH`

Tests that a string field ends with a suffix.

```javascript
// File extension
const filter = 'filename ENDS_WITH ".pdf"';

// Email domain
const filter = 'email ENDS_WITH "@company.com"';
```

**Supported Types:** String only

---

### `LIKE`

Pattern matching with wildcards. Uses `%` for any sequence of characters.

```javascript
// Names starting with "John"
const filter = 'name LIKE "John%"';

// Emails from domain
const filter = 'email LIKE "%@gmail.com"';

// Contains substring
const filter = 'description LIKE "%important%"';

// Single character wildcard (use _)
const filter = 'code LIKE "A_B"';  // Matches "A1B", "AXB", etc.
```

**Wildcards:**
- `%` - Matches any sequence of characters (including empty)
- `_` - Matches exactly one character

**Supported Types:** String only

---

## Null Operators

### `IS NULL`

Tests that a field is null or does not exist.

```javascript
// Missing description
const filter = 'description IS NULL';

// No assigned category
const filter = 'category IS NULL';
```

**Use Case:** Finding vectors with incomplete metadata.

---

### `IS NOT NULL`

Tests that a field exists and is not null.

```javascript
// Has description
const filter = 'description IS NOT NULL';

// Has been categorized
const filter = 'category IS NOT NULL';
```

**Use Case:** Finding vectors with complete metadata.

---

## Logical Operators

### `AND`

Combines conditions that must all be true.

```javascript
// Multiple conditions
const filter = 'category = "electronics" AND price < 500';

// Three conditions
const filter = 'status = "active" AND rating >= 4.0 AND stock > 0';

// Alternative syntax
const filter = 'category = "electronics" && price < 500';
```

**Precedence:** AND has higher precedence than OR.

---

### `OR`

Combines conditions where at least one must be true.

```javascript
// Either category
const filter = 'category = "gpu" OR category = "cpu"';

// Multiple options
const filter = 'status = "active" OR status = "pending" OR status = "review"';

// Alternative syntax
const filter = 'category = "gpu" || category = "cpu"';
```

**Precedence:** OR has lower precedence than AND.

---

### `NOT`

Negates a condition.

```javascript
// Not deleted
const filter = 'NOT (deleted = true)';

// Not in category
const filter = 'NOT (category = "spam")';

// Alternative syntax
const filter = '!(deleted = true)';
```

**Note:** Always use parentheses with NOT for clarity.

---

## Complex Examples

### Multi-Field Filters

```javascript
// E-commerce product search
const filter = `
    category = "electronics"
    AND price BETWEEN 100 AND 500
    AND rating >= 4.0
    AND in_stock = true
`;

// Document search
const filter = `
    author = "John Doe"
    AND created_at >= 1700000000
    AND status IN ["published", "draft"]
`;
```

### Nested Expressions

```javascript
// Complex boolean logic
const filter = `
    (category = "gpu" OR category = "cpu")
    AND price < 1000
    AND NOT (status = "discontinued")
`;

// Multiple OR groups
const filter = `
    (brand = "nvidia" OR brand = "amd")
    AND (memory >= 8 OR (memory >= 4 AND price < 300))
`;
```

### Combining String and Numeric Filters

```javascript
// Full-featured search
const filter = `
    title CONTAINS "machine learning"
    AND year >= 2020
    AND citations > 100
    AND NOT (status = "retracted")
`;
```

---

## Operator Precedence

From highest to lowest precedence:

1. `NOT` (highest)
2. `AND`
3. `OR` (lowest)

**Example:**

```javascript
// This expression:
a = 1 OR b = 2 AND c = 3

// Is parsed as:
a = 1 OR (b = 2 AND c = 3)

// Use parentheses for explicit grouping:
(a = 1 OR b = 2) AND c = 3
```

---

## TypeScript Types

```typescript
// Filter expression is always a string
const filter: string = 'category = "gpu"';

// Search options
interface SearchOptions {
    filter?: string;           // Filter expression
    strategy?: 'auto' | 'pre' | 'post' | 'hybrid';
    oversampleFactor?: number; // For post/hybrid (default: 3.0)
    includeMetadata?: boolean; // Include metadata in results
    includeVectors?: boolean;  // Include vectors in results
}

// Usage with searchFiltered
const result = JSON.parse(index.searchFiltered(query, k, JSON.stringify({
    filter: 'category = "gpu" AND price < 500',
    strategy: 'auto'
})));
```

---

## Error Messages

### Common Mistakes and Fixes

| Error | Cause | Fix |
|:------|:------|:----|
| `E001: Syntax error` | Missing quotes around string | `category = gpu` → `category = "gpu"` |
| `E001: Syntax error` | Using `==` instead of `=` | `price == 100` → `price = 100` |
| `E001: Syntax error` | Missing operator | `category "gpu"` → `category = "gpu"` |
| `E001: Syntax error` | Unclosed string | `name = "test` → `name = "test"` |
| `E001: Syntax error` | Unclosed parenthesis | `(a = 1` → `(a = 1)` |
| `E301: Nesting too deep` | Too many nested expressions | Simplify expression (max 50 levels) |
| `E303: Input too long` | Filter expression too large | Max 65,536 bytes |

### Invalid Escape Sequences

Valid escape sequences in strings:
- `\"` - Double quote
- `\\` - Backslash
- `\n` - Newline
- `\r` - Carriage return
- `\t` - Tab

```javascript
// Correct
const filter = 'title = "Hello \"World\""';

// Incorrect - \x is not valid
const filter = 'title = "test\xvalue"';  // Error: Invalid escape
```

---

## Limits

| Limit | Value | Error Code |
|:------|:------|:-----------|
| Max input length | 65,536 bytes | E303 |
| Max nesting depth | 50 levels | E301 |
| Max array elements | 1,000 | E304 |
| Max expression nodes | 1,000 | E302 |

---

## Best Practices

1. **Use parentheses** for clarity in complex expressions
2. **Put the most selective condition first** for better performance
3. **Use `IN` instead of multiple `OR`** for membership tests
4. **Avoid deeply nested expressions** (max 50 levels)
5. **Quote all string values** with double quotes
6. **Test filters** with `validate_filter_js()` before search

---

## See Also

- [Database Operations](DATABASE_OPERATIONS.md)
- [TypeScript API](TYPESCRIPT_API.md)
- [Error Reference](ERROR_REFERENCE.md)
