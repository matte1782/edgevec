//! Filter expression evaluator for EdgeVec.
//!
//! This module provides the core `evaluate()` function that recursively evaluates
//! a `FilterExpr` AST against metadata values.
//!
//! # Architecture
//!
//! The evaluator follows a simple recursive descent pattern:
//! - Logical operators (AND, OR, NOT) short-circuit when possible
//! - Comparison operators use type coercion (Int/Float interop)
//! - String operators provide pattern matching (LIKE, CONTAINS, etc.)
//! - Array operators support set membership and quantifiers
//!
//! # Short-Circuit Behavior
//!
//! - `AND`: Returns `false` immediately on first `false` operand
//! - `OR`: Returns `true` immediately on first `true` operand
//! - Errors propagate immediately (no error swallowing)
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use edgevec::filter::{evaluate, parse, FilterExpr};
//! use edgevec::metadata::MetadataValue;
//!
//! let mut metadata = HashMap::new();
//! metadata.insert("category".to_string(), MetadataValue::String("gpu".to_string()));
//! metadata.insert("price".to_string(), MetadataValue::Integer(450));
//!
//! let expr = parse("category = \"gpu\" AND price < 500").unwrap();
//! let result = evaluate(&expr, &metadata).unwrap();
//! assert!(result);
//! ```

use crate::filter::ast::FilterExpr;
use crate::filter::error::FilterError;
use crate::metadata::MetadataValue;
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════════
// RESOLVED VALUE TYPE
// ═══════════════════════════════════════════════════════════════════════════════

/// Intermediate value type for evaluation.
///
/// This type represents resolved values during filter evaluation,
/// supporting the same types as `MetadataValue` plus explicit `Null`.
#[derive(Debug, Clone, PartialEq)]
enum ResolvedValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    /// Represents a missing field (field not in metadata map).
    Null,
}

impl ResolvedValue {
    /// Returns the type name for error messages.
    fn type_name(&self) -> &'static str {
        match self {
            ResolvedValue::String(_) => "string",
            ResolvedValue::Int(_) => "integer",
            ResolvedValue::Float(_) => "float",
            ResolvedValue::Bool(_) => "boolean",
            ResolvedValue::StringArray(_) => "string_array",
            ResolvedValue::Null => "null",
        }
    }
}

impl From<&MetadataValue> for ResolvedValue {
    fn from(value: &MetadataValue) -> Self {
        match value {
            MetadataValue::String(s) => ResolvedValue::String(s.clone()),
            MetadataValue::Integer(i) => ResolvedValue::Int(*i),
            MetadataValue::Float(f) => ResolvedValue::Float(*f),
            MetadataValue::Boolean(b) => ResolvedValue::Bool(*b),
            MetadataValue::StringArray(arr) => ResolvedValue::StringArray(arr.clone()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN EVALUATE FUNCTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Evaluate a filter expression against metadata.
///
/// # Arguments
///
/// * `expr` - The filter expression AST to evaluate
/// * `metadata` - The metadata map for a single vector
///
/// # Returns
///
/// * `Ok(true)` - The filter matches
/// * `Ok(false)` - The filter doesn't match
/// * `Err(FilterError)` - Evaluation error (type mismatch, etc.)
///
/// # Short-Circuit Behavior
///
/// - `AND`: Returns `false` immediately on first `false` operand
/// - `OR`: Returns `true` immediately on first `true` operand
/// - Errors propagate immediately
///
/// # Errors
///
/// Returns [`FilterError::TypeMismatch`] when:
/// - A string operator is used on a non-string field
/// - A numeric comparison is used on a non-numeric field
/// - An array operator is used on a non-array field
///
/// Returns [`FilterError::UnknownField`] when:
/// - A field reference doesn't exist in the metadata map
///
/// Returns [`FilterError::InvalidExpression`] when:
/// - A literal or field is evaluated directly instead of within an operator
///
/// # Example
///
/// ```rust
/// use std::collections::HashMap;
/// use edgevec::filter::{evaluate, parse};
/// use edgevec::metadata::MetadataValue;
///
/// let mut metadata = HashMap::new();
/// metadata.insert("active".to_string(), MetadataValue::Boolean(true));
/// metadata.insert("count".to_string(), MetadataValue::Integer(10));
///
/// // Simple equality
/// let expr = parse("active = true").unwrap();
/// assert!(evaluate(&expr, &metadata).unwrap());
///
/// // Numeric comparison
/// let expr = parse("count >= 5").unwrap();
/// assert!(evaluate(&expr, &metadata).unwrap());
/// ```
#[allow(clippy::implicit_hasher)]
pub fn evaluate(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError> {
    match expr {
        // ═══════════════════════════════════════════════════════════════════════
        // LOGICAL OPERATORS (with short-circuit)
        // ═══════════════════════════════════════════════════════════════════════
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

        FilterExpr::Not(inner) => Ok(!evaluate(inner, metadata)?),

        // ═══════════════════════════════════════════════════════════════════════
        // COMPARISON OPERATORS
        // ═══════════════════════════════════════════════════════════════════════
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

        FilterExpr::Lt(left, right) => eval_numeric_comparison(left, right, metadata, |a, b| a < b),

        FilterExpr::Le(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a <= b)
        }

        FilterExpr::Gt(left, right) => eval_numeric_comparison(left, right, metadata, |a, b| a > b),

        FilterExpr::Ge(left, right) => {
            eval_numeric_comparison(left, right, metadata, |a, b| a >= b)
        }

        // ═══════════════════════════════════════════════════════════════════════
        // STRING OPERATORS
        // ═══════════════════════════════════════════════════════════════════════
        FilterExpr::Contains(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.contains(p))
        }

        FilterExpr::StartsWith(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.starts_with(p))
        }

        FilterExpr::EndsWith(field, pattern) => {
            eval_string_op(field, pattern, metadata, |s, p| s.ends_with(p))
        }

        FilterExpr::Like(field, pattern) => eval_like_pattern(field, pattern, metadata),

        // ═══════════════════════════════════════════════════════════════════════
        // ARRAY/SET OPERATORS
        // ═══════════════════════════════════════════════════════════════════════
        FilterExpr::In(field, array) => eval_set_membership(field, array, metadata, true),

        FilterExpr::NotIn(field, array) => eval_set_membership(field, array, metadata, false),

        FilterExpr::Any(field, array) => eval_array_op(field, array, metadata, ArrayOp::Any),

        FilterExpr::All(field, array) => eval_array_op(field, array, metadata, ArrayOp::All),

        FilterExpr::None(field, array) => eval_array_op(field, array, metadata, ArrayOp::None),

        // ═══════════════════════════════════════════════════════════════════════
        // RANGE OPERATOR
        // ═══════════════════════════════════════════════════════════════════════
        FilterExpr::Between(field, low, high) => eval_between(field, low, high, metadata),

        // ═══════════════════════════════════════════════════════════════════════
        // NULL OPERATORS
        // ═══════════════════════════════════════════════════════════════════════
        FilterExpr::IsNull(field) => Ok(eval_is_null(field, metadata, true)),

        FilterExpr::IsNotNull(field) => Ok(eval_is_null(field, metadata, false)),

        // ═══════════════════════════════════════════════════════════════════════
        // LITERALS (should not be evaluated directly)
        // ═══════════════════════════════════════════════════════════════════════
        FilterExpr::LiteralString(_)
        | FilterExpr::LiteralInt(_)
        | FilterExpr::LiteralFloat(_)
        | FilterExpr::LiteralBool(_)
        | FilterExpr::LiteralArray(_)
        | FilterExpr::Field(_) => Err(FilterError::InvalidExpression {
            message: "Cannot evaluate literal or field as boolean expression".into(),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALUE RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════════

/// Resolve a `FilterExpr` to a concrete value.
///
/// For literals, returns the literal value directly.
/// For fields, looks up the value in metadata.
fn resolve_value(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<ResolvedValue, FilterError> {
    match expr {
        FilterExpr::LiteralString(s) => Ok(ResolvedValue::String(s.clone())),
        FilterExpr::LiteralInt(i) => Ok(ResolvedValue::Int(*i)),
        FilterExpr::LiteralFloat(f) => Ok(ResolvedValue::Float(*f)),
        FilterExpr::LiteralBool(b) => Ok(ResolvedValue::Bool(*b)),
        FilterExpr::LiteralArray(items) => {
            // Resolve each item in the array
            let resolved: Result<Vec<ResolvedValue>, FilterError> = items
                .iter()
                .map(|item| resolve_value(item, metadata))
                .collect();
            Ok(ResolvedValue::StringArray(
                resolved?
                    .into_iter()
                    .filter_map(|v| match v {
                        ResolvedValue::String(s) => Some(s),
                        _ => None,
                    })
                    .collect(),
            ))
        }
        FilterExpr::Field(name) => {
            metadata
                .get(name)
                .map(ResolvedValue::from)
                .ok_or_else(|| FilterError::UnknownField {
                    field: name.clone(),
                })
        }
        _ => Err(FilterError::InvalidExpression {
            message: "Expected value expression (literal or field)".into(),
        }),
    }
}

/// Try to resolve a field, returning `Null` if not found (for null checks).
fn resolve_field_nullable(
    expr: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> ResolvedValue {
    match expr {
        FilterExpr::Field(name) => metadata
            .get(name)
            .map_or(ResolvedValue::Null, ResolvedValue::from),
        _ => ResolvedValue::Null,
    }
}

/// Get field name from a `FilterExpr::Field`, or empty string.
fn get_field_name(expr: &FilterExpr) -> String {
    match expr {
        FilterExpr::Field(name) => name.clone(),
        _ => String::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPARISON HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Compare two values for equality with type coercion.
///
/// Supports Int/Float coercion: `5 == 5.0` returns `true`.
#[allow(clippy::cast_precision_loss)]
fn values_equal(left: &ResolvedValue, right: &ResolvedValue) -> bool {
    match (left, right) {
        (ResolvedValue::String(a), ResolvedValue::String(b)) => a == b,
        (ResolvedValue::Int(a), ResolvedValue::Int(b)) => a == b,
        (ResolvedValue::Float(a), ResolvedValue::Float(b)) => (a - b).abs() < f64::EPSILON,
        (ResolvedValue::Bool(a), ResolvedValue::Bool(b)) => a == b,
        (ResolvedValue::StringArray(a), ResolvedValue::StringArray(b)) => a == b,

        // Int/Float coercion (precision loss is acceptable for filter comparisons)
        (ResolvedValue::Int(a), ResolvedValue::Float(b)) => ((*a as f64) - *b).abs() < f64::EPSILON,
        (ResolvedValue::Float(a), ResolvedValue::Int(b)) => (*a - (*b as f64)).abs() < f64::EPSILON,

        // Null comparisons and type mismatch
        (ResolvedValue::Null, ResolvedValue::Null) => true,
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

    let left_num = to_numeric(&left_val, &get_field_name(left))?;
    let right_num = to_numeric(&right_val, &get_field_name(right))?;

    Ok(cmp(left_num, right_num))
}

/// Convert a resolved value to f64 for numeric comparison.
#[allow(clippy::cast_precision_loss)]
fn to_numeric(val: &ResolvedValue, field_hint: &str) -> Result<f64, FilterError> {
    match val {
        ResolvedValue::Int(i) => Ok(*i as f64),
        ResolvedValue::Float(f) => Ok(*f),
        _ => Err(FilterError::TypeMismatch {
            field: field_hint.to_string(),
            expected: "numeric (integer or float)".into(),
            actual: val.type_name().into(),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STRING OPERATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Evaluate a string operation (CONTAINS, STARTS_WITH, ENDS_WITH).
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

    match (&field_val, &pattern_val) {
        (ResolvedValue::String(s), ResolvedValue::String(p)) => Ok(op(s, p)),
        (ResolvedValue::Null, _) => Ok(false),
        _ => Err(FilterError::TypeMismatch {
            field: get_field_name(field),
            expected: "string".into(),
            actual: field_val.type_name().into(),
        }),
    }
}

/// Evaluate LIKE pattern matching.
///
/// Supports SQL LIKE syntax:
/// - `%` matches any sequence of characters (including empty)
/// - `_` matches exactly one character
fn eval_like_pattern(
    field: &FilterExpr,
    pattern: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let pattern_val = resolve_value(pattern, metadata)?;

    match (&field_val, &pattern_val) {
        (ResolvedValue::String(s), ResolvedValue::String(p)) => Ok(like_match(s, p)),
        (ResolvedValue::Null, _) => Ok(false),
        _ => Err(FilterError::TypeMismatch {
            field: get_field_name(field),
            expected: "string".into(),
            actual: field_val.type_name().into(),
        }),
    }
}

/// LIKE pattern matching implementation.
///
/// Uses an iterative algorithm to avoid stack overflow on pathological patterns.
/// - `%` matches any sequence of characters
/// - `_` matches exactly one character
fn like_match(value: &str, pattern: &str) -> bool {
    let value: Vec<char> = value.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();

    let mut vi = 0; // Value index
    let mut pi = 0; // Pattern index
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while vi < value.len() {
        if pi < pattern.len() && (pattern[pi] == '_' || pattern[pi] == value[vi]) {
            // Match single character or underscore wildcard
            vi += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == '%' {
            // Found a %, record position for backtracking
            star_idx = Some(pi);
            match_idx = vi;
            pi += 1;
        } else if let Some(star) = star_idx {
            // Backtrack to last % and try matching one more character
            pi = star + 1;
            match_idx += 1;
            vi = match_idx;
        } else {
            return false;
        }
    }

    // Consume remaining % characters in pattern
    while pi < pattern.len() && pattern[pi] == '%' {
        pi += 1;
    }

    pi == pattern.len()
}

// ═══════════════════════════════════════════════════════════════════════════════
// ARRAY/SET OPERATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Enum for array quantifier operations.
#[derive(Clone, Copy)]
enum ArrayOp {
    /// True if ANY element of field is in pattern array
    Any,
    /// True if ALL elements of pattern array are in field
    All,
    /// True if NO element of field is in pattern array
    None,
}

/// Evaluate set membership (IN / NOT IN).
///
/// Checks if a scalar field value is contained in an array literal.
fn eval_set_membership(
    field: &FilterExpr,
    array: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    should_contain: bool,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let array_vals = resolve_array(array, metadata)?;

    let is_member = array_vals.iter().any(|v| values_equal(&field_val, v));

    Ok(if should_contain {
        is_member
    } else {
        !is_member
    })
}

/// Evaluate array operations (ANY, ALL, NONE).
///
/// For a field containing `[a, b, c]` and pattern `[x, y]`:
/// - `ANY`: true if any element of field is in pattern (a in [x,y] OR b in [x,y] OR ...)
/// - `ALL`: true if all elements of pattern are in field (x in field AND y in field)
/// - `NONE`: true if no element of field is in pattern (!ANY)
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
                expected: "string_array".into(),
                actual: field_val.type_name().into(),
            });
        }
    };

    // Convert pattern to strings
    let pattern_strings: Vec<String> = pattern_vals
        .into_iter()
        .filter_map(|v| match v {
            ResolvedValue::String(s) => Some(s),
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

/// Resolve an array literal to a vector of resolved values.
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

// ═══════════════════════════════════════════════════════════════════════════════
// RANGE OPERATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Evaluate BETWEEN operator.
///
/// `field BETWEEN low AND high` is equivalent to `field >= low AND field <= high`.
fn eval_between(
    field: &FilterExpr,
    low: &FilterExpr,
    high: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
) -> Result<bool, FilterError> {
    let field_val = resolve_value(field, metadata)?;
    let low_val = resolve_value(low, metadata)?;
    let high_val = resolve_value(high, metadata)?;

    let field_name = get_field_name(field);
    let field_num = to_numeric(&field_val, &field_name)?;
    let low_num = to_numeric(&low_val, "")?;
    let high_num = to_numeric(&high_val, "")?;

    Ok(field_num >= low_num && field_num <= high_num)
}

// ═══════════════════════════════════════════════════════════════════════════════
// NULL CHECK HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Evaluate IS NULL / IS NOT NULL.
fn eval_is_null(
    field: &FilterExpr,
    metadata: &HashMap<String, MetadataValue>,
    should_be_null: bool,
) -> bool {
    let field_val = resolve_field_nullable(field, metadata);
    let is_null = matches!(field_val, ResolvedValue::Null);

    if should_be_null {
        is_null
    } else {
        !is_null
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[allow(clippy::uninlined_format_args)]
#[allow(clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use crate::filter::parse;

    /// Helper to create metadata map
    fn make_metadata() -> HashMap<String, MetadataValue> {
        let mut m = HashMap::new();
        m.insert(
            "category".to_string(),
            MetadataValue::String("gpu".to_string()),
        );
        m.insert(
            "brand".to_string(),
            MetadataValue::String("nvidia".to_string()),
        );
        m.insert("price".to_string(), MetadataValue::Integer(450));
        m.insert("rating".to_string(), MetadataValue::Float(4.5));
        m.insert("in_stock".to_string(), MetadataValue::Boolean(true));
        m.insert(
            "tags".to_string(),
            MetadataValue::StringArray(vec![
                "gaming".to_string(),
                "graphics".to_string(),
                "high-end".to_string(),
            ]),
        );
        m
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CORE EVALUATE TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod core {
        use super::*;

        #[test]
        fn test_literal_cannot_be_evaluated() {
            let metadata = HashMap::new();
            let expr = FilterExpr::LiteralInt(42);
            let result = evaluate(&expr, &metadata);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                FilterError::InvalidExpression { .. }
            ));
        }

        #[test]
        fn test_field_cannot_be_evaluated() {
            let metadata = make_metadata();
            let expr = FilterExpr::Field("category".to_string());
            let result = evaluate(&expr, &metadata);
            assert!(result.is_err());
        }

        #[test]
        fn test_empty_metadata() {
            let metadata = HashMap::new();
            let expr = parse("category = \"gpu\"").unwrap();
            let result = evaluate(&expr, &metadata);
            // Unknown field error
            assert!(result.is_err());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // COMPARISON TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod comparison {
        use super::*;

        #[test]
        fn test_eq_string() {
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("category = \"cpu\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_eq_integer() {
            let metadata = make_metadata();
            let expr = parse("price = 450").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price = 500").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_eq_boolean() {
            let metadata = make_metadata();
            let expr = parse("in_stock = true").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("in_stock = false").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_ne_string() {
            let metadata = make_metadata();
            let expr = parse("category != \"cpu\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("category != \"gpu\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_lt() {
            let metadata = make_metadata();
            let expr = parse("price < 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price < 450").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_le() {
            let metadata = make_metadata();
            let expr = parse("price <= 450").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price <= 449").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_gt() {
            let metadata = make_metadata();
            let expr = parse("price > 400").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price > 450").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_ge() {
            let metadata = make_metadata();
            let expr = parse("price >= 450").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price >= 451").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_int_float_coercion() {
            let metadata = make_metadata();
            // price is Integer(450), compare with float
            let expr = parse("price = 450.0").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price < 450.5").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_float_comparison() {
            let metadata = make_metadata();
            let expr = parse("rating > 4.0").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("rating < 5.0").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_type_mismatch_comparison() {
            let metadata = make_metadata();
            // Comparing string field with numeric operator
            let expr = parse("category < 100").unwrap();
            let result = evaluate(&expr, &metadata);
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                FilterError::TypeMismatch { .. }
            ));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STRING OPERATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod string_ops {
        use super::*;

        #[test]
        fn test_contains() {
            let metadata = make_metadata();
            let expr = parse("brand CONTAINS \"vid\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("brand CONTAINS \"amd\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_starts_with() {
            let metadata = make_metadata();
            let expr = parse("brand STARTS_WITH \"nv\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("brand STARTS_WITH \"am\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_ends_with() {
            let metadata = make_metadata();
            let expr = parse("brand ENDS_WITH \"dia\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("brand ENDS_WITH \"amd\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_exact() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"nvidia\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_percent_start() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"%dia\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_percent_end() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"nv%\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_percent_both() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"%vid%\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_underscore() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"n_idia\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("brand LIKE \"n__dia\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_like_no_match() {
            let metadata = make_metadata();
            let expr = parse("brand LIKE \"amd%\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_string_op_type_mismatch() {
            let metadata = make_metadata();
            let expr = parse("price CONTAINS \"00\"").unwrap();
            let result = evaluate(&expr, &metadata);
            assert!(result.is_err());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ARRAY OPERATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod array_ops {
        use super::*;

        #[test]
        fn test_in_match() {
            let metadata = make_metadata();
            let expr = parse("category IN [\"gpu\", \"cpu\", \"ram\"]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_in_no_match() {
            let metadata = make_metadata();
            let expr = parse("category IN [\"cpu\", \"ram\", \"ssd\"]").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_not_in_match() {
            let metadata = make_metadata();
            let expr = parse("category NOT IN [\"cpu\", \"ram\", \"ssd\"]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_not_in_no_match() {
            let metadata = make_metadata();
            let expr = parse("category NOT IN [\"gpu\", \"cpu\", \"ram\"]").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_any_match() {
            let metadata = make_metadata();
            // tags = ["gaming", "graphics", "high-end"]
            let expr = parse("tags ANY [\"gaming\", \"budget\"]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_any_no_match() {
            let metadata = make_metadata();
            let expr = parse("tags ANY [\"budget\", \"low-end\"]").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_all_match() {
            let metadata = make_metadata();
            // All of ["gaming", "graphics"] are in tags
            let expr = parse("tags ALL [\"gaming\", \"graphics\"]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_all_no_match() {
            let metadata = make_metadata();
            // "budget" is not in tags
            let expr = parse("tags ALL [\"gaming\", \"budget\"]").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_none_match() {
            let metadata = make_metadata();
            let expr = parse("tags NONE [\"budget\", \"low-end\"]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_none_no_match() {
            let metadata = make_metadata();
            let expr = parse("tags NONE [\"gaming\", \"budget\"]").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_array_op_type_mismatch() {
            let metadata = make_metadata();
            // category is String, not StringArray
            let expr = parse("category ANY [\"gpu\", \"cpu\"]").unwrap();
            let result = evaluate(&expr, &metadata);
            assert!(result.is_err());
        }

        #[test]
        fn test_in_with_integer() {
            let metadata = make_metadata();
            let expr = parse("price IN [400, 450, 500]").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_empty_array() {
            let metadata = make_metadata();
            let expr = parse("category IN []").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());

            let expr = parse("category NOT IN []").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RANGE OPERATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod range_ops {
        use super::*;

        #[test]
        fn test_between_in_range() {
            let metadata = make_metadata();
            let expr = parse("price BETWEEN 400 AND 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_between_at_boundary() {
            let metadata = make_metadata();
            let expr = parse("price BETWEEN 450 AND 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("price BETWEEN 400 AND 450").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_between_out_of_range() {
            let metadata = make_metadata();
            let expr = parse("price BETWEEN 500 AND 600").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_between_float() {
            let metadata = make_metadata();
            let expr = parse("rating BETWEEN 4.0 AND 5.0").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NULL CHECK TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod null_checks {
        use super::*;

        #[test]
        fn test_is_null_missing_field() {
            let metadata = make_metadata();
            let expr = parse("missing_field IS NULL").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_is_null_existing_field() {
            let metadata = make_metadata();
            let expr = parse("category IS NULL").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_is_not_null_missing_field() {
            let metadata = make_metadata();
            let expr = parse("missing_field IS NOT NULL").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_is_not_null_existing_field() {
            let metadata = make_metadata();
            let expr = parse("category IS NOT NULL").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LOGICAL OPERATOR TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    mod logical_ops {
        use super::*;

        #[test]
        fn test_and_both_true() {
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\" AND price < 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_and_left_false() {
            let metadata = make_metadata();
            let expr = parse("category = \"cpu\" AND price < 500").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_and_right_false() {
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\" AND price > 500").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_or_both_true() {
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\" OR price < 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_or_left_true() {
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\" OR price > 1000").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_or_right_true() {
            let metadata = make_metadata();
            let expr = parse("category = \"cpu\" OR price < 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_or_both_false() {
            let metadata = make_metadata();
            let expr = parse("category = \"cpu\" OR price > 1000").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_not_true() {
            let metadata = make_metadata();
            let expr = parse("NOT category = \"cpu\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_not_false() {
            let metadata = make_metadata();
            let expr = parse("NOT category = \"gpu\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_complex_expression() {
            let metadata = make_metadata();
            let expr = parse("(category = \"gpu\" OR category = \"cpu\") AND price < 500").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            let expr = parse("NOT (category = \"cpu\" AND price > 1000)").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_short_circuit_and() {
            // Test that AND short-circuits on first false
            // If short-circuit works, second operand with unknown field won't error
            let metadata = make_metadata();
            let expr = parse("category = \"cpu\" AND unknown_field = 1").unwrap();
            // First operand is false, should return false without evaluating second
            assert!(!evaluate(&expr, &metadata).unwrap());
        }

        #[test]
        fn test_short_circuit_or() {
            // Test that OR short-circuits on first true
            let metadata = make_metadata();
            let expr = parse("category = \"gpu\" OR unknown_field = 1").unwrap();
            // First operand is true, should return true without evaluating second
            assert!(evaluate(&expr, &metadata).unwrap());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LIKE PATTERN MATCHING EDGE CASES
    // ═══════════════════════════════════════════════════════════════════════════

    mod like_edge_cases {
        use super::*;

        #[test]
        fn test_like_empty_pattern() {
            assert!(like_match("", ""));
            assert!(!like_match("a", ""));
        }

        #[test]
        fn test_like_empty_value() {
            assert!(like_match("", ""));
            assert!(like_match("", "%"));
            assert!(!like_match("", "_"));
        }

        #[test]
        fn test_like_only_percent() {
            assert!(like_match("anything", "%"));
            assert!(like_match("", "%"));
            assert!(like_match("abc", "%%%"));
        }

        #[test]
        fn test_like_only_underscore() {
            assert!(like_match("a", "_"));
            assert!(!like_match("ab", "_"));
            assert!(like_match("ab", "__"));
        }

        #[test]
        fn test_like_mixed_wildcards() {
            assert!(like_match("abcdef", "a%f"));
            assert!(like_match("abcdef", "a_c%f"));
            assert!(like_match("abcdef", "%c_e%"));
        }

        #[test]
        fn test_like_unicode() {
            assert!(like_match("日本語", "日%"));
            assert!(like_match("日本語", "_本_"));
            assert!(like_match("Hello世界", "Hello%"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EDGE CASE TESTS (M2 Remediation)
    // ═══════════════════════════════════════════════════════════════════════════

    mod edge_cases {
        use super::*;

        /// Test ALL operator with empty pattern array (vacuous truth).
        ///
        /// In logic, "for all x in empty set, P(x)" is vacuously true because
        /// there are no counterexamples. This matches SQL behavior.
        #[test]
        fn test_all_empty_array_vacuous_truth() {
            let metadata = make_metadata();
            // "All elements of [] are in tags" is vacuously true
            let expr = parse("tags ALL []").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }

        /// Test string equality with unicode characters.
        ///
        /// Ensures that non-ASCII characters are compared correctly byte-for-byte
        /// without normalization issues.
        #[test]
        fn test_unicode_string_equality() {
            let mut metadata = HashMap::new();
            metadata.insert(
                "greeting".to_string(),
                MetadataValue::String("你好世界".to_string()),
            );
            metadata.insert(
                "emoji".to_string(),
                MetadataValue::String("🦀🚀".to_string()),
            );

            // Exact match with Chinese characters
            let expr = parse("greeting = \"你好世界\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            // Non-match with similar but different characters
            let expr = parse("greeting = \"你好世界!\"").unwrap();
            assert!(!evaluate(&expr, &metadata).unwrap());

            // Emoji equality
            let expr = parse("emoji = \"🦀🚀\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());

            // Contains with unicode
            let expr = parse("greeting CONTAINS \"好\"").unwrap();
            assert!(evaluate(&expr, &metadata).unwrap());
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PROPERTY TESTS (M3 Remediation - "Nvidia Grade" verification)
    // ═══════════════════════════════════════════════════════════════════════════
    //
    // Property-based tests using proptest to verify invariants across randomly
    // generated inputs. These tests complement the unit tests above by exploring
    // edge cases that manual test authoring might miss.

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: Logical operator laws
        // ═══════════════════════════════════════════════════════════════════════

        /// Property: NOT(NOT(x)) == x (double negation elimination)
        ///
        /// For any filter expression that evaluates to a boolean result,
        /// applying NOT twice should return the original result.
        #[test]
        fn prop_double_negation_elimination() {
            let metadata = make_metadata();

            // Test with concrete expressions instead of generated AST
            let test_cases = [
                "category = \"gpu\"",
                "price < 500",
                "in_stock = true",
                "rating >= 4.0",
            ];

            for filter_str in &test_cases {
                let expr = parse(filter_str).unwrap();
                let original = evaluate(&expr, &metadata).unwrap();

                // NOT(NOT(expr))
                let double_neg = FilterExpr::Not(Box::new(FilterExpr::Not(Box::new(expr))));
                let result = evaluate(&double_neg, &metadata).unwrap();

                assert_eq!(
                    original, result,
                    "Double negation should be eliminated for: {}",
                    filter_str
                );
            }
        }

        /// Property: AND is commutative (a AND b) == (b AND a)
        ///
        /// The order of operands in AND should not affect the result.
        #[test]
        fn prop_and_commutativity() {
            let metadata = make_metadata();

            let pairs = [
                ("category = \"gpu\"", "price < 500"),
                ("in_stock = true", "rating > 4.0"),
                ("brand = \"nvidia\"", "category = \"gpu\""),
            ];

            for (a, b) in &pairs {
                let expr_a = parse(a).unwrap();
                let expr_b = parse(b).unwrap();

                let and_ab =
                    FilterExpr::And(Box::new(expr_a.clone()), Box::new(expr_b.clone()));
                let and_ba = FilterExpr::And(Box::new(expr_b), Box::new(expr_a));

                let result_ab = evaluate(&and_ab, &metadata).unwrap();
                let result_ba = evaluate(&and_ba, &metadata).unwrap();

                assert_eq!(
                    result_ab, result_ba,
                    "AND should be commutative for ({}, {})",
                    a, b
                );
            }
        }

        /// Property: OR is commutative (a OR b) == (b OR a)
        ///
        /// The order of operands in OR should not affect the result.
        #[test]
        fn prop_or_commutativity() {
            let metadata = make_metadata();

            let pairs = [
                ("category = \"gpu\"", "category = \"cpu\""),
                ("price > 1000", "price < 100"),
                ("in_stock = true", "rating > 4.5"),
            ];

            for (a, b) in &pairs {
                let expr_a = parse(a).unwrap();
                let expr_b = parse(b).unwrap();

                let or_ab = FilterExpr::Or(Box::new(expr_a.clone()), Box::new(expr_b.clone()));
                let or_ba = FilterExpr::Or(Box::new(expr_b), Box::new(expr_a));

                let result_ab = evaluate(&or_ab, &metadata).unwrap();
                let result_ba = evaluate(&or_ba, &metadata).unwrap();

                assert_eq!(
                    result_ab, result_ba,
                    "OR should be commutative for ({}, {})",
                    a, b
                );
            }
        }

        /// Property: De Morgan's laws
        ///
        /// NOT(a AND b) == (NOT a) OR (NOT b)
        /// NOT(a OR b) == (NOT a) AND (NOT b)
        #[test]
        fn prop_de_morgan_laws() {
            let metadata = make_metadata();

            let pairs = [
                ("category = \"gpu\"", "price < 500"),
                ("in_stock = true", "rating > 4.0"),
            ];

            for (a, b) in &pairs {
                let expr_a = parse(a).unwrap();
                let expr_b = parse(b).unwrap();

                // NOT(a AND b)
                let not_and = FilterExpr::Not(Box::new(FilterExpr::And(
                    Box::new(expr_a.clone()),
                    Box::new(expr_b.clone()),
                )));

                // (NOT a) OR (NOT b)
                let or_not = FilterExpr::Or(
                    Box::new(FilterExpr::Not(Box::new(expr_a.clone()))),
                    Box::new(FilterExpr::Not(Box::new(expr_b.clone()))),
                );

                let result_not_and = evaluate(&not_and, &metadata).unwrap();
                let result_or_not = evaluate(&or_not, &metadata).unwrap();

                assert_eq!(
                    result_not_and, result_or_not,
                    "De Morgan's law: NOT(a AND b) == (NOT a) OR (NOT b) for ({}, {})",
                    a, b
                );

                // NOT(a OR b)
                let not_or = FilterExpr::Not(Box::new(FilterExpr::Or(
                    Box::new(expr_a.clone()),
                    Box::new(expr_b.clone()),
                )));

                // (NOT a) AND (NOT b)
                let and_not = FilterExpr::And(
                    Box::new(FilterExpr::Not(Box::new(expr_a))),
                    Box::new(FilterExpr::Not(Box::new(expr_b))),
                );

                let result_not_or = evaluate(&not_or, &metadata).unwrap();
                let result_and_not = evaluate(&and_not, &metadata).unwrap();

                assert_eq!(
                    result_not_or, result_and_not,
                    "De Morgan's law: NOT(a OR b) == (NOT a) AND (NOT b) for ({}, {})",
                    a, b
                );
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: Numeric comparison transitivity
        // ═══════════════════════════════════════════════════════════════════════

        proptest! {
            /// Property: If a < b and b < c, then a < c (transitivity of <)
            ///
            /// Tests that numeric comparisons maintain transitivity.
            #[test]
            fn prop_numeric_transitivity(
                a in -1000i64..1000i64,
                b in -1000i64..1000i64,
                c in -1000i64..1000i64
            ) {
                let mut metadata = HashMap::new();
                metadata.insert("val".to_string(), MetadataValue::Integer(b));

                // If a < b (testing against the metadata value)
                let expr_a_lt_val = FilterExpr::Lt(
                    Box::new(FilterExpr::LiteralInt(a)),
                    Box::new(FilterExpr::Field("val".to_string())),
                );

                let expr_val_lt_c = FilterExpr::Lt(
                    Box::new(FilterExpr::Field("val".to_string())),
                    Box::new(FilterExpr::LiteralInt(c)),
                );

                let a_lt_b = evaluate(&expr_a_lt_val, &metadata).unwrap();
                let b_lt_c = evaluate(&expr_val_lt_c, &metadata).unwrap();

                if a_lt_b && b_lt_c {
                    // Then a < c must be true
                    prop_assert!(a < c, "Transitivity violated: {} < {} < {} but {} >= {}", a, b, c, a, c);
                }
            }

            /// Property: x == x (reflexivity of equality)
            ///
            /// Any value compared to itself should be equal.
            #[test]
            fn prop_equality_reflexivity(val in -1000i64..1000i64) {
                let mut metadata = HashMap::new();
                metadata.insert("num".to_string(), MetadataValue::Integer(val));

                // num = num (comparing field to itself via same literal)
                let expr = FilterExpr::Eq(
                    Box::new(FilterExpr::Field("num".to_string())),
                    Box::new(FilterExpr::LiteralInt(val)),
                );

                let result = evaluate(&expr, &metadata).unwrap();
                prop_assert!(result, "Reflexivity violated: {} != {}", val, val);
            }

            /// Property: BETWEEN is equivalent to >= AND <=
            ///
            /// "x BETWEEN a AND b" should be equivalent to "x >= a AND x <= b"
            #[test]
            fn prop_between_equivalence(
                x in -1000i64..1000i64,
                a in -1000i64..1000i64,
                b in -1000i64..1000i64
            ) {
                // Ensure a <= b for BETWEEN semantics
                let (low, high) = if a <= b { (a, b) } else { (b, a) };

                let mut metadata = HashMap::new();
                metadata.insert("val".to_string(), MetadataValue::Integer(x));

                // val BETWEEN low AND high
                let expr_between = FilterExpr::Between(
                    Box::new(FilterExpr::Field("val".to_string())),
                    Box::new(FilterExpr::LiteralInt(low)),
                    Box::new(FilterExpr::LiteralInt(high)),
                );

                // val >= low AND val <= high
                let expr_equivalent = FilterExpr::And(
                    Box::new(FilterExpr::Ge(
                        Box::new(FilterExpr::Field("val".to_string())),
                        Box::new(FilterExpr::LiteralInt(low)),
                    )),
                    Box::new(FilterExpr::Le(
                        Box::new(FilterExpr::Field("val".to_string())),
                        Box::new(FilterExpr::LiteralInt(high)),
                    )),
                );

                let result_between = evaluate(&expr_between, &metadata).unwrap();
                let result_equivalent = evaluate(&expr_equivalent, &metadata).unwrap();

                prop_assert_eq!(
                    result_between, result_equivalent,
                    "BETWEEN {} AND {} should equal >= {} AND <= {} for val={}",
                    low, high, low, high, x
                );
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: Set membership consistency
        // ═══════════════════════════════════════════════════════════════════════

        proptest! {
            /// Property: IN and NOT IN are mutually exclusive
            ///
            /// "x IN [a,b,c]" XOR "x NOT IN [a,b,c]" should always be true
            #[test]
            fn prop_in_not_in_mutual_exclusion(val in 0i64..10i64) {
                let mut metadata = HashMap::new();
                metadata.insert("num".to_string(), MetadataValue::Integer(val));

                let array = FilterExpr::LiteralArray(vec![
                    FilterExpr::LiteralInt(0),
                    FilterExpr::LiteralInt(3),
                    FilterExpr::LiteralInt(6),
                    FilterExpr::LiteralInt(9),
                ]);

                let expr_in = FilterExpr::In(
                    Box::new(FilterExpr::Field("num".to_string())),
                    Box::new(array.clone()),
                );

                let expr_not_in = FilterExpr::NotIn(
                    Box::new(FilterExpr::Field("num".to_string())),
                    Box::new(array),
                );

                let result_in = evaluate(&expr_in, &metadata).unwrap();
                let result_not_in = evaluate(&expr_not_in, &metadata).unwrap();

                // Exactly one should be true (XOR)
                prop_assert_ne!(
                    result_in, result_not_in,
                    "IN and NOT IN should be mutually exclusive for val={}",
                    val
                );
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: NULL handling consistency
        // ═══════════════════════════════════════════════════════════════════════

        /// Property: IS NULL and IS NOT NULL are mutually exclusive
        ///
        /// For any field, exactly one of IS NULL or IS NOT NULL should be true.
        #[test]
        fn prop_null_mutual_exclusion() {
            let test_cases = [
                // Metadata with field present
                {
                    let mut m = HashMap::new();
                    m.insert("field".to_string(), MetadataValue::Integer(42));
                    m
                },
                // Metadata with field absent
                HashMap::new(),
            ];

            for metadata in &test_cases {
                let expr_is_null = FilterExpr::IsNull(Box::new(FilterExpr::Field(
                    "field".to_string(),
                )));
                let expr_is_not_null = FilterExpr::IsNotNull(Box::new(FilterExpr::Field(
                    "field".to_string(),
                )));

                let result_is_null = evaluate(&expr_is_null, metadata).unwrap();
                let result_is_not_null = evaluate(&expr_is_not_null, metadata).unwrap();

                assert_ne!(
                    result_is_null, result_is_not_null,
                    "IS NULL and IS NOT NULL should be mutually exclusive"
                );
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: Type coercion consistency
        // ═══════════════════════════════════════════════════════════════════════

        proptest! {
            /// Property: Int/Float comparison coercion is symmetric
            ///
            /// Comparing int to float should give same result as float to int.
            #[test]
            fn prop_int_float_coercion_symmetric(int_val in -100i64..100i64) {
                let float_val = int_val as f64;

                let mut metadata_int = HashMap::new();
                metadata_int.insert("val".to_string(), MetadataValue::Integer(int_val));

                let mut metadata_float = HashMap::new();
                metadata_float.insert("val".to_string(), MetadataValue::Float(float_val));

                // Compare int field to float literal
                let expr_int_eq_float = FilterExpr::Eq(
                    Box::new(FilterExpr::Field("val".to_string())),
                    Box::new(FilterExpr::LiteralFloat(float_val)),
                );

                // Compare float field to int literal
                let expr_float_eq_int = FilterExpr::Eq(
                    Box::new(FilterExpr::Field("val".to_string())),
                    Box::new(FilterExpr::LiteralInt(int_val)),
                );

                let result_int = evaluate(&expr_int_eq_float, &metadata_int).unwrap();
                let result_float = evaluate(&expr_float_eq_int, &metadata_float).unwrap();

                // Both should be true since int_val == float_val
                prop_assert!(result_int, "Int {} should equal Float {}", int_val, float_val);
                prop_assert!(result_float, "Float {} should equal Int {}", float_val, int_val);
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // PROPERTY: LIKE pattern matching
        // ═══════════════════════════════════════════════════════════════════════

        proptest! {
            /// Property: "%" matches any string
            ///
            /// The pattern "%" should match any non-null string value.
            #[test]
            fn prop_like_percent_matches_all(s in "[a-zA-Z0-9]{0,20}") {
                let mut metadata = HashMap::new();
                metadata.insert("str".to_string(), MetadataValue::String(s.clone()));

                let expr = FilterExpr::Like(
                    Box::new(FilterExpr::Field("str".to_string())),
                    Box::new(FilterExpr::LiteralString("%".to_string())),
                );

                let result = evaluate(&expr, &metadata).unwrap();
                prop_assert!(result, "'%' should match '{}'", s);
            }

            /// Property: Exact pattern matches exact string
            ///
            /// A pattern with no wildcards should only match the exact string.
            #[test]
            fn prop_like_exact_match(s in "[a-zA-Z]{1,10}") {
                let mut metadata = HashMap::new();
                metadata.insert("str".to_string(), MetadataValue::String(s.clone()));

                // Same string should match
                let expr_same = FilterExpr::Like(
                    Box::new(FilterExpr::Field("str".to_string())),
                    Box::new(FilterExpr::LiteralString(s.clone())),
                );
                let result_same = evaluate(&expr_same, &metadata).unwrap();
                prop_assert!(result_same, "Exact pattern '{}' should match '{}'", s, s);

                // Different string should not match (add 'X' suffix)
                let different = format!("{}X", s);
                let expr_diff = FilterExpr::Like(
                    Box::new(FilterExpr::Field("str".to_string())),
                    Box::new(FilterExpr::LiteralString(different.clone())),
                );
                let result_diff = evaluate(&expr_diff, &metadata).unwrap();
                prop_assert!(!result_diff, "Pattern '{}' should not match '{}'", different, s);
            }
        }
    }
}
