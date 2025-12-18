# Week 23, Day 2: Evaluator Core

**Date:** 2025-12-19
**Sprint:** Week 23 (v0.5.0 Implementation)
**Day Theme:** Filter Evaluator Implementation
**Status:** [REVISED]
**Revision:** Addresses [M2] from HOSTILE_REVIEWER

---

## Daily Summary

| Metric | Value |
|:-------|:------|
| Tasks | 4 |
| Total Hours | 12h |
| Agent | RUST_ENGINEER |
| Priority | P0 CRITICAL |
| Depends On | W23.1 (Parser) |

**[M2] Dependency Update:**
- W23.2.4 (Array operators) now explicitly depends on W23.2.3 (String operators)
- Rationale: Array operators reuse pattern matching foundation from string operators

---

## Task W23.2.1: Core Recursive Evaluate Function

**Priority:** P0 CRITICAL
**Estimated Effort:** 4 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Implement the core `evaluate()` function that recursively evaluates a `FilterExpr` against metadata.

### Technical Specification

```rust
// src/filter/evaluator.rs

use crate::filter::ast::FilterExpr;
use crate::filter::error::FilterError;
use crate::metadata::MetadataValue;
use std::collections::HashMap;

/// Evaluate a filter expression against metadata.
///
/// # Arguments
/// * `expr` - The filter expression AST
/// * `metadata` - The metadata map for a single vector
///
/// # Returns
/// * `Ok(true)` - The filter matches
/// * `Ok(false)` - The filter doesn't match
/// * `Err(FilterError)` - Evaluation error (type mismatch, etc.)
///
/// # Short-Circuit Behavior
/// - AND: Returns false immediately on first false operand
/// - OR: Returns true immediately on first true operand
/// - Errors propagate immediately
pub fn evaluate(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError> {
    match expr {
        // ═══════════════════════════════════════════════════════════════
        // LOGICAL OPERATORS (with short-circuit)
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::And(left, right) => {
            // Short-circuit: if left is false, return false immediately
            if !evaluate(left, metadata)? {
                return Ok(false);
            }
            evaluate(right, metadata)
        }

        FilterExpr::Or(left, right) => {
            // Short-circuit: if left is true, return true immediately
            if evaluate(left, metadata)? {
                return Ok(true);
            }
            evaluate(right, metadata)
        }

        FilterExpr::Not(inner) => {
            Ok(!evaluate(inner, metadata)?)
        }

        // ═══════════════════════════════════════════════════════════════
        // COMPARISON OPERATORS
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::Eq(left, right) => {
            let left_val = resolve_value(left, metadata)?;
            let right_val = resolve_value(right, metadata)?;
            Ok(values_equal(&left_val, &right_val))
        }

        FilterExpr::Ne(left, right) => {
            let left_val = resolve_value(left, metadata)?;
            let right_val = resolve_value(right, metadata)?;
            Ok(!values_equal(&left_val, &right_val))
        }

        FilterExpr::Lt(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a < b)
        }

        FilterExpr::Le(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a <= b)
        }

        FilterExpr::Gt(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a > b)
        }

        FilterExpr::Ge(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a >= b)
        }

        // ═══════════════════════════════════════════════════════════════
        // STRING OPERATORS
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::Contains(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.contains(p))
        }

        FilterExpr::StartsWith(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.starts_with(p))
        }

        FilterExpr::EndsWith(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.ends_with(p))
        }

        FilterExpr::Like(field, pattern) => {
            eval_like_pattern(field, pattern, metadata)
        }

        // ═══════════════════════════════════════════════════════════════
        // ARRAY/SET OPERATORS
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::In(field, array) => {
            eval_set_membership(field, array, metadata, true)
        }

        FilterExpr::NotIn(field, array) => {
            eval_set_membership(field, array, metadata, false)
        }

        FilterExpr::Any(field, array) => {
            eval_array_op(field, array, metadata, ArrayOp::Any)
        }

        FilterExpr::All(field, array) => {
            eval_array_op(field, array, metadata, ArrayOp::All)
        }

        FilterExpr::None(field, array) => {
            eval_array_op(field, array, metadata, ArrayOp::None)
        }

        // ═══════════════════════════════════════════════════════════════
        // RANGE OPERATOR
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::Between(field, low, high) => {
            eval_between(field, low, high, metadata)
        }

        // ═══════════════════════════════════════════════════════════════
        // NULL OPERATORS
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::IsNull(field) => {
            eval_is_null(field, metadata, true)
        }

        FilterExpr::IsNotNull(field) => {
            eval_is_null(field, metadata, false)
        }

        // ═══════════════════════════════════════════════════════════════
        // LITERALS (should not be evaluated directly)
        // ═══════════════════════════════════════════════════════════════

        FilterExpr::LiteralString(_)
        | FilterExpr::LiteralInt(_)
        | FilterExpr::LiteralFloat(_)
        | FilterExpr::LiteralBool(_)
        | FilterExpr::LiteralArray(_)
        | FilterExpr::Field(_) => {
            Err(FilterError::InvalidExpression {
                message: "Cannot evaluate literal as boolean".into(),
            })
        }
    }
}
```

### Acceptance Criteria

- [ ] All 27 AST variants handled in match
- [ ] AND/OR short-circuit correctly
- [ ] Errors propagate correctly
- [ ] `cargo test filter::evaluator::core` passes

---

## Task W23.2.2: Comparison Operators

**Priority:** P0 CRITICAL
**Estimated Effort:** 3 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Implement comparison operators with type coercion.

### Technical Specification

```rust
/// Resolve a FilterExpr to a concrete value.
fn resolve_value(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<ResolvedValue, FilterError> {
    match expr {
        FilterExpr::LiteralString(s) => Ok(ResolvedValue::String(s.clone())),
        FilterExpr::LiteralInt(i) => Ok(ResolvedValue::Int(*i)),
        FilterExpr::LiteralFloat(f) => Ok(ResolvedValue::Float(*f)),
        FilterExpr::LiteralBool(b) => Ok(ResolvedValue::Bool(*b)),
        FilterExpr::Field(name) => {
            metadata.get(name)
                .map(|v| ResolvedValue::from(v))
                .ok_or_else(|| FilterError::UnknownField { field: name.clone() })
        }
        _ => Err(FilterError::InvalidExpression {
            message: "Expected value expression".into(),
        }),
    }
}

/// Compare two values for equality with type coercion.
fn values_equal(left: &ResolvedValue, right: &ResolvedValue) -> bool {
    match (left, right) {
        (ResolvedValue::String(a), ResolvedValue::String(b)) => a == b,
        (ResolvedValue::Int(a), ResolvedValue::Int(b)) => a == b,
        (ResolvedValue::Float(a), ResolvedValue::Float(b)) => a == b,
        (ResolvedValue::Bool(a), ResolvedValue::Bool(b)) => a == b,

        // Int/Float coercion
        (ResolvedValue::Int(a), ResolvedValue::Float(b)) => (*a as f64) == *b,
        (ResolvedValue::Float(a), ResolvedValue::Int(b)) => *a == (*b as f64),

        // Null comparisons
        (ResolvedValue::Null, ResolvedValue::Null) => true,
        (ResolvedValue::Null, _) | (_, ResolvedValue::Null) => false,

        // Type mismatch
        _ => false,
    }
}

/// Evaluate numeric comparison with coercion.
fn eval_numeric_comparison<F>(
    left: &FilterExpr,
    right: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    cmp: F,
) -> Result<bool, FilterError>
where
    F: Fn(f64, f64) -> bool,
{
    let left_val = resolve_value(left, metadata)?;
    let right_val = resolve_value(right, metadata)?;

    let left_num = to_numeric(&left_val)?;
    let right_num = to_numeric(&right_val)?;

    Ok(cmp(left_num, right_num))
}

fn to_numeric(val: &ResolvedValue) -> Result<f64, FilterError> {
    match val {
        ResolvedValue::Int(i) => Ok(*i as f64),
        ResolvedValue::Float(f) => Ok(*f),
        _ => Err(FilterError::TypeMismatch {
            field: "".into(),
            expected: "numeric".into(),
            actual: val.type_name().into(),
        }),
    }
}
```

### Acceptance Criteria

- [ ] =, !=, <, <=, >, >= all work correctly
- [ ] Int/Float coercion works (5 == 5.0)
- [ ] Type mismatches return appropriate errors
- [ ] `cargo test filter::evaluator::comparison` passes

---

## Task W23.2.3: String Operators

**Priority:** P0 CRITICAL
**Estimated Effort:** 3 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Implement string operators including LIKE pattern matching.

### Technical Specification

```rust
/// Evaluate string operation.
fn eval_string_op<F>(
    field: &FilterExpr,
    pattern: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    op: F,
) -> Result<bool, FilterError>
where
    F: Fn(&str, &str) -> bool,
{
    let field_val = resolve_value(field, metadata)?;
    let pattern_val = resolve_value(pattern, metadata)?;

    match (field_val, pattern_val) {
        (ResolvedValue::String(s), ResolvedValue::String(p)) => Ok(op(&s, &p)),
        (ResolvedValue::Null, _) => Ok(false),
        _ => Err(FilterError::TypeMismatch {
            field: get_field_name(field),
            expected: "string".into(),
            actual: "non-string".into(),
        }),
    }
}

/// Evaluate LIKE pattern matching.
///
/// Supports SQL LIKE syntax:
/// - % matches any sequence of characters
/// - _ matches exactly one character
/// - Use \% and \_ for literal % and _
fn eval_like_pattern(
    field: &FilterExpr,
    pattern: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let pattern_val = resolve_value(pattern, metadata)?;

    match (field_val, pattern_val) {
        (ResolvedValue::String(s), ResolvedValue::String(p)) => {
            Ok(like_match(&s, &p))
        }
        (ResolvedValue::Null, _) => Ok(false),
        _ => Err(FilterError::TypeMismatch {
            field: get_field_name(field),
            expected: "string".into(),
            actual: "non-string".into(),
        }),
    }
}

/// LIKE pattern matching implementation.
///
/// Uses iterative algorithm to avoid stack overflow on pathological patterns.
fn like_match(value: &str, pattern: &str) -> bool {
    let value: Vec<char> = value.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();

    let mut vi = 0; // Value index
    let mut pi = 0; // Pattern index
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while vi < value.len() {
        if pi < pattern.len() && (pattern[pi] == '_' || pattern[pi] == value[vi]) {
            vi += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == '%' {
            star_idx = Some(pi);
            match_idx = vi;
            pi += 1;
        } else if let Some(star) = star_idx {
            pi = star + 1;
            match_idx += 1;
            vi = match_idx;
        } else {
            return false;
        }
    }

    // Consume remaining % in pattern
    while pi < pattern.len() && pattern[pi] == '%' {
        pi += 1;
    }

    pi == pattern.len()
}
```

### Acceptance Criteria

- [ ] CONTAINS, STARTS_WITH, ENDS_WITH work correctly
- [ ] LIKE pattern matching handles %, _, escapes
- [ ] No ReDoS vulnerability (iterative algorithm)
- [ ] Null returns false, not error
- [ ] `cargo test filter::evaluator::string` passes

---

## Task W23.2.4: Array Operators [M2 DEPENDENCY ADDED]

**Priority:** P0 CRITICAL
**Estimated Effort:** 2 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED
**Dependencies:** W23.2.3 (String operators - pattern matching foundation)

### Objective

Implement IN, NOT IN, ANY, ALL, NONE operators.

### Technical Specification

```rust
/// Evaluate set membership (IN / NOT IN).
fn eval_set_membership(
    field: &FilterExpr,
    array: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    should_contain: bool,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let array_vals = resolve_array(array, metadata)?;

    let is_member = array_vals.iter().any(|v| values_equal(&field_val, v));

    Ok(if should_contain { is_member } else { !is_member })
}

/// Evaluate array operations (ANY, ALL, NONE).
///
/// For array field containing [a, b, c] and pattern [x, y]:
/// - ANY: true if any element of field is in pattern (a in [x,y] OR b in [x,y] OR ...)
/// - ALL: true if all elements of pattern are in field (x in [a,b,c] AND y in [a,b,c])
/// - NONE: true if no element of field is in pattern (not ANY)
fn eval_array_op(
    field: &FilterExpr,
    pattern: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    op: ArrayOp,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let pattern_vals = resolve_array(pattern, metadata)?;

    // Field must be a string array
    let field_array = match field_val {
        ResolvedValue::StringArray(arr) => arr,
        ResolvedValue::Null => return Ok(false),
        _ => {
            return Err(FilterError::TypeMismatch {
                field: get_field_name(field),
                expected: "string array".into(),
                actual: field_val.type_name().into(),
            });
        }
    };

    // Convert pattern to strings
    let pattern_strings: Vec<String> = pattern_vals
        .iter()
        .filter_map(|v| match v {
            ResolvedValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect();

    match op {
        ArrayOp::Any => {
            // Any element of field is in pattern
            Ok(field_array.iter().any(|f| pattern_strings.contains(f)))
        }
        ArrayOp::All => {
            // All elements of pattern are in field
            Ok(pattern_strings.iter().all(|p| field_array.contains(p)))
        }
        ArrayOp::None => {
            // No element of field is in pattern
            Ok(!field_array.iter().any(|f| pattern_strings.contains(f)))
        }
    }
}

enum ArrayOp {
    Any,
    All,
    None,
}

fn resolve_array(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<Vec<ResolvedValue>, FilterError> {
    match expr {
        FilterExpr::LiteralArray(items) => {
            items.iter().map(|i| resolve_value(i, metadata)).collect()
        }
        _ => Err(FilterError::InvalidExpression {
            message: "Expected array literal".into(),
        }),
    }
}
```

### Acceptance Criteria

- [ ] IN/NOT IN work for scalar values against arrays
- [ ] ANY/ALL/NONE work for array fields against pattern arrays
- [ ] Empty arrays handled correctly
- [ ] Null fields return false
- [ ] `cargo test filter::evaluator::array` passes

---

## Day 2 Completion Checklist

- [ ] W23.2.1: Core evaluate() function complete
- [ ] W23.2.2: Comparison operators complete
- [ ] W23.2.3: String operators complete (including LIKE)
- [ ] W23.2.4: Array operators complete
- [ ] All unit tests pass
- [ ] Short-circuit optimization verified
- [ ] Performance: <5μs per vector evaluation

---

## Handoff to Day 3

**Day 2 Output:**
- `src/filter/evaluator.rs` - Complete evaluator implementation
- `src/filter/resolved_value.rs` - Value resolution types (optional)

**Day 3 Depends On:**
- `evaluate()` function (W23.2.1)
- All operator implementations (W23.2.2-2.4)

---

*"The evaluator is the heart. Every vector passes through it."*
