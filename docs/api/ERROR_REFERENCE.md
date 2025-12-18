# Error Reference

**Version:** EdgeVec v0.5.0
**Last Updated:** 2025-12-18

---

## Error Code Categories

| Range | Category | Description |
|:------|:---------|:------------|
| `E001-E099` | Syntax Errors | Parse errors, invalid tokens |
| `E101-E199` | Type Errors | Type mismatches, invalid casts |
| `E201-E299` | Evaluation Errors | Runtime evaluation errors |
| `E301-E399` | Limit Errors | Resource limits exceeded |
| `E401-E499` | Strategy Errors | Filter strategy configuration |

---

## Syntax Errors (E0xx)

### E001: SyntaxError

**Cause:** General syntax error during parsing.

**Example:**
```
category gpu           // Missing operator
```

**Fix:** Add the missing operator:
```
category = "gpu"
```

**Position Info:** Yes (position, line, column)

---

### E002: UnexpectedEof

**Cause:** Unexpected end of input while parsing.

**Example:**
```
category =            // Value expected
```

**Fix:** Complete the expression:
```
category = "gpu"
```

**Position Info:** Yes (position)

---

### E003: InvalidChar

**Cause:** Invalid character encountered during parsing.

**Example:**
```
category @ "gpu"      // '@' is not valid
```

**Fix:** Use valid operators (=, !=, <, >, etc.):
```
category = "gpu"
```

**Position Info:** Yes (character, position)

---

### E004: UnclosedString

**Cause:** String literal started but never closed.

**Example:**
```
category = "gpu       // Missing closing quote
```

**Fix:** Close the string:
```
category = "gpu"
```

**Suggestion:** "Did you forget the closing quote?"

**Position Info:** Yes (position where string started)

---

### E005: UnclosedParen

**Cause:** Opening parenthesis without matching close.

**Example:**
```
(a = 1 AND b = 2      // Missing )
```

**Fix:** Close the parenthesis:
```
(a = 1 AND b = 2)
```

**Suggestion:** "Did you forget the closing parenthesis?"

**Position Info:** Yes (position of opening paren)

---

### E006: InvalidEscape

**Cause:** Invalid escape sequence in string.

**Example:**
```
name = "test\xvalue"  // \x is not valid
```

**Fix:** Use valid escape sequences:
```
name = "test\\xvalue"  // Escaped backslash
```

**Valid Escapes:** `\"`, `\\`, `\n`, `\r`, `\t`

**Position Info:** Yes (position of backslash)

---

### E007: InvalidNumber

**Cause:** Malformed number literal.

**Example:**
```
price = 1.2.3         // Multiple decimal points
```

**Fix:** Use valid number format:
```
price = 1.23
```

**Position Info:** Yes (position where number started)

---

## Type Errors (E1xx)

### E101: TypeMismatch

**Cause:** Operation with incompatible types.

**Example:**
```
// Field 'count' is integer, but compared with string
count = "five"
```

**Fix:** Use matching type:
```
count = 5
```

**Suggestion:** "Field 'count' is of type 'integer', but 'string' was expected. Check that you're using the right operator for this field type."

---

### E102: IncompatibleTypes

**Cause:** Two values of incompatible types were compared.

**Example:**
```
// Can't compare string with integer
category = 123
```

**Fix:** Use compatible types:
```
category = "123"      // String comparison
// OR
count = 123           // Integer comparison
```

---

### E103: InvalidOperatorForType

**Cause:** Operator used with unsupported type.

**Example:**
```
// Can't use < with boolean
active < true
```

**Fix:** Use valid operator for type:
```
active = true         // Boolean equality
```

---

### E105: UnknownField

**Cause:** Field referenced that doesn't exist.

**Example:**
```
categry = "gpu"       // Typo: should be "category"
```

**Fix:** Correct the field name:
```
category = "gpu"
```

**Suggestion:** "Field 'categry' does not exist. Check the field name for typos."

---

## Evaluation Errors (E2xx)

### E201: DivisionByZero

**Cause:** Division by zero during evaluation.

**Note:** This error is rare in filter expressions as division is not commonly used.

---

### E202: NullValue

**Cause:** Null value in non-nullable context.

**Example:**
```
// Field 'description' is null, but comparison requires a value
description > "test"
```

**Fix:** Check for null first:
```
description IS NOT NULL AND description > "test"
```

---

### E203: IndexOutOfBounds

**Cause:** Array index out of bounds.

**Note:** This error is rare in filter expressions.

---

### E204: InvalidExpression

**Cause:** Literal used where boolean expression expected.

**Example:**
```
// Just a value, not a boolean expression
"gpu"
```

**Fix:** Use a complete comparison:
```
category = "gpu"
```

---

## Limit Errors (E3xx)

### E301: NestingTooDeep

**Cause:** Expression exceeds maximum nesting depth (50 levels).

**Example:**
```
(((((... 51 levels ...)))))
```

**Fix:** Simplify the expression by:
- Splitting into multiple queries
- Reducing redundant grouping
- Combining conditions

**Suggestion:** "Simplify your filter expression. Maximum nesting depth is 50."

**Limits:**
- Max depth: 50 levels

---

### E302: ExpressionTooComplex

**Cause:** Expression has too many nodes/operations (>1000).

**Example:**
```
a = 1 AND b = 2 AND c = 3 AND ... (1001 conditions)
```

**Fix:** Break into multiple queries or use IN operator:
```
// Instead of many OR conditions:
id IN [1, 2, 3, 4, 5, ...]
```

**Limits:**
- Max nodes: 1,000

---

### E303: InputTooLong

**Cause:** Filter expression string exceeds maximum length.

**Example:**
```
// Filter string > 65,536 bytes
```

**Fix:** Shorten the expression or use parameterized queries.

**Limits:**
- Max length: 65,536 bytes

---

### E304: ArrayTooLarge

**Cause:** Array literal in IN/ANY/ALL has too many elements.

**Example:**
```
id IN [1, 2, 3, ..., 1001]  // Too many elements
```

**Fix:** Reduce array size or use multiple queries:
```
// Split into chunks
id IN [1, 2, ..., 500]   // First query
id IN [501, 502, ..., 1000]  // Second query
```

**Suggestion:** "Use smaller arrays in IN/ANY/ALL operators. Maximum is 1000 elements."

**Limits:**
- Max elements: 1,000

---

## Strategy Errors (E4xx)

### E401: InvalidStrategy

**Cause:** Invalid filter strategy configuration.

**Example:**
```json
{
    "strategy": "invalid",
    "oversampleFactor": 0.5  // Must be >= 1.0
}
```

**Fix:** Use valid strategy and parameters:
```json
{
    "strategy": "auto",
    "oversampleFactor": 3.0
}
```

**Valid Strategies:** `auto`, `pre`, `post`, `hybrid`

---

## Error Handling in JavaScript

### Catching Parse Errors

```javascript
import { parse_filter_js, validate_filter_js } from 'edgevec';

// Method 1: Try-catch
try {
    const ast = parse_filter_js('category = "gpu"');
} catch (e) {
    const error = JSON.parse(e);
    console.log('Code:', error.code);
    console.log('Message:', error.message);
    console.log('Position:', error.position);
    console.log('Suggestion:', error.suggestion);
}

// Method 2: Validate first
const result = JSON.parse(validate_filter_js(userInput));
if (!result.valid) {
    console.log('Errors:', result.errors);
    console.log('Warnings:', result.warnings);
}
```

### Error Object Structure

```typescript
interface FilterError {
    code: string;           // "E001", "E301", etc.
    message: string;        // Human-readable message
    position?: number;      // Byte offset (for syntax errors)
    line?: number;          // Line number (1-indexed)
    column?: number;        // Column number (1-indexed)
    suggestion?: string;    // Fix suggestion
}
```

---

## Error Prevention Best Practices

1. **Validate before use:**
   ```javascript
   const validation = JSON.parse(validate_filter_js(userInput));
   if (validation.valid) {
       // Safe to use
   }
   ```

2. **Quote all string values:**
   ```javascript
   // Good
   'category = "gpu"'

   // Bad - will cause E001
   'category = gpu'
   ```

3. **Use parentheses for clarity:**
   ```javascript
   // Good
   '(a = 1 OR b = 2) AND c = 3'

   // Ambiguous
   'a = 1 OR b = 2 AND c = 3'
   ```

4. **Keep arrays small:**
   ```javascript
   // Good (< 1000 elements)
   'id IN [1, 2, 3, 4, 5]'

   // Bad - may cause E304
   'id IN [1, 2, ..., 2000]'
   ```

5. **Avoid deep nesting:**
   ```javascript
   // Good (flat structure)
   'a = 1 AND b = 2 AND c = 3'

   // Avoid (unnecessary nesting)
   '(((a = 1) AND (b = 2)) AND (c = 3))'
   ```

---

## See Also

- [Filter Syntax Reference](FILTER_SYNTAX.md)
- [Database Operations Guide](DATABASE_OPERATIONS.md)
- [TypeScript API](TYPESCRIPT_API.md)
