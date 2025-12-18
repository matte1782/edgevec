//! WASM Bindings for EdgeVec Filter System.
//!
//! This module exposes the Rust filter system to JavaScript via wasm-bindgen.
//! It provides filter parsing, validation, and filtered search capabilities.
//!
//! # Week 23 Day 4 (W23.4) Implementation
//!
//! - W23.4.1: `parse_filter_js()` - Parse filter strings to JSON AST
//! - W23.4.2: `search_filtered_js()` - Filtered vector search
//! - W23.4.3: FilterError → JsValue serialization
//! - W23.4.4: JSON serialization helpers
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! import { parse_filter_js, validate_filter_js } from 'edgevec';
//!
//! // Parse a filter expression
//! const filterJson = parse_filter_js('category = "gpu" AND price < 500');
//! console.log(JSON.parse(filterJson));
//!
//! // Validate a filter
//! const result = JSON.parse(validate_filter_js('price > 100'));
//! if (result.valid) {
//!     console.log('Filter is valid');
//! }
//! ```

use crate::filter::{parse, FilterError, FilterExpr};
use serde::Serialize;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

// =============================================================================
// W23.4.1: parse_filter_js() WASM Export
// =============================================================================

/// Parse a filter expression string into a compiled filter.
///
/// # Arguments
///
/// * `filter_str` - Filter expression in EdgeVec syntax
///
/// # Returns
///
/// JSON string representation of the parsed filter AST.
///
/// # Errors
///
/// Returns a JsValue error with structured JSON containing:
/// - `code`: Error code (e.g., "E001")
/// - `message`: Human-readable error message
/// - `position`: Position information (if available)
/// - `suggestion`: Fix suggestion (if available)
///
/// # Example (JavaScript)
///
/// ```javascript
/// try {
///     const filterJson = parse_filter_js('category = "gpu" AND price < 500');
///     console.log(JSON.parse(filterJson));
/// } catch (e) {
///     console.error('Parse error:', JSON.parse(e).message);
/// }
/// ```
#[wasm_bindgen]
pub fn parse_filter_js(filter_str: &str) -> Result<String, JsValue> {
    let ast = parse(filter_str).map_err(|e| filter_error_to_jsvalue(&e))?;

    serde_json::to_string(&ast).map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
}

/// Validate a filter string without fully returning the AST.
///
/// # Arguments
///
/// * `filter_str` - Filter expression to validate
///
/// # Returns
///
/// JSON string with validation result:
/// ```json
/// {
///   "valid": true,
///   "errors": [],
///   "warnings": []
/// }
/// ```
///
/// # Example (JavaScript)
///
/// ```javascript
/// const result = JSON.parse(validate_filter_js('price <'));
/// if (!result.valid) {
///     console.log('Errors:', result.errors);
/// }
/// ```
#[must_use]
#[wasm_bindgen]
pub fn validate_filter_js(filter_str: &str) -> String {
    let result = parse(filter_str);

    let validation = ValidationResult {
        valid: result.is_ok(),
        errors: match &result {
            Err(e) => vec![error_to_json(e)],
            Ok(_) => vec![],
        },
        warnings: vec![], // Future: add warnings for suspicious patterns
    };

    serde_json::to_string(&validation).unwrap_or_else(|_| {
        r#"{"valid":false,"errors":[{"code":"E000","message":"Internal serialization error"}]}"#
            .to_string()
    })
}

/// Try to parse a filter string, returning null on error.
///
/// # Arguments
///
/// * `filter_str` - Filter expression to parse
///
/// # Returns
///
/// JSON string of parsed filter, or null if invalid.
///
/// # Example (JavaScript)
///
/// ```javascript
/// const filter = try_parse_filter_js(userInput);
/// if (filter !== null) {
///     // Valid filter
/// }
/// ```
#[must_use]
#[wasm_bindgen]
pub fn try_parse_filter_js(filter_str: &str) -> JsValue {
    match parse(filter_str) {
        Ok(ast) => match serde_json::to_string(&ast) {
            Ok(json) => JsValue::from_str(&json),
            Err(_) => JsValue::NULL,
        },
        Err(_) => JsValue::NULL,
    }
}

/// Get filter information (complexity, fields, operators).
///
/// # Arguments
///
/// * `filter_str` - Filter expression to analyze
///
/// # Returns
///
/// JSON string with filter info:
/// ```json
/// {
///   "nodeCount": 5,
///   "depth": 3,
///   "fields": ["category", "price"],
///   "operators": ["eq", "lt", "and"],
///   "complexity": 3
/// }
/// ```
///
/// # Errors
///
/// Returns error if filter parsing fails.
#[wasm_bindgen]
pub fn get_filter_info_js(filter_str: &str) -> Result<String, JsValue> {
    let ast = parse(filter_str).map_err(|e| filter_error_to_jsvalue(&e))?;

    let info = FilterInfo {
        node_count: count_nodes(&ast),
        depth: ast.depth(),
        fields: collect_fields(&ast),
        operators: collect_operators(&ast),
        complexity: estimate_complexity(&ast),
    };

    serde_json::to_string(&info)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
}

// =============================================================================
// W23.4.3: FilterError → JsValue Serialization
// =============================================================================

/// Structured error for JavaScript consumption.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FilterExceptionJs {
    /// Error code for programmatic handling (e.g., "E001")
    code: String,
    /// Human-readable error message
    message: String,
    /// Position in filter string (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<PositionJs>,
    /// Suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

#[derive(Serialize)]
struct PositionJs {
    line: usize,
    column: usize,
    offset: usize,
}

/// Convert FilterError to JsValue for throwing.
///
/// This function is public so it can be used by other WASM modules
/// (e.g., searchFiltered in mod.rs).
#[must_use]
pub fn filter_error_to_jsvalue(error: &FilterError) -> JsValue {
    let exception = filter_error_to_exception(error);

    // Serialize to JSON and return as JsValue
    match serde_json::to_string(&exception) {
        Ok(json) => JsValue::from_str(&json),
        Err(_) => JsValue::from_str(&format!(
            r#"{{"code":"{}","message":"{}"}}"#,
            exception.code, exception.message
        )),
    }
}

/// Convert FilterError to structured exception.
fn filter_error_to_exception(error: &FilterError) -> FilterExceptionJs {
    match error {
        FilterError::SyntaxError {
            position,
            line,
            column,
            message,
            suggestion,
        } => FilterExceptionJs {
            code: "E001".to_string(),
            message: message.clone(),
            position: Some(PositionJs {
                line: *line,
                column: *column,
                offset: *position,
            }),
            suggestion: suggestion.clone(),
        },
        FilterError::UnexpectedEof { position, expected } => FilterExceptionJs {
            code: "E002".to_string(),
            message: format!("Unexpected end of input: expected {expected}"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: Some("The filter expression appears to be incomplete".to_string()),
        },
        FilterError::InvalidChar { char, position } => FilterExceptionJs {
            code: "E003".to_string(),
            message: format!("Invalid character '{char}' at position {position}"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: None,
        },
        FilterError::UnclosedString { position } => FilterExceptionJs {
            code: "E004".to_string(),
            message: format!("Unclosed string literal starting at position {position}"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: Some("Add a closing quote (\") to complete the string".to_string()),
        },
        FilterError::UnclosedParen { position } => FilterExceptionJs {
            code: "E005".to_string(),
            message: format!("Unclosed parenthesis starting at position {position}"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: Some("Add a closing parenthesis )".to_string()),
        },
        FilterError::InvalidEscape { char, position } => FilterExceptionJs {
            code: "E006".to_string(),
            message: format!("Invalid escape sequence: '\\{char}'"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: Some("Valid escapes: \\n, \\t, \\r, \\\", \\\\".to_string()),
        },
        FilterError::InvalidNumber { value, position } => FilterExceptionJs {
            code: "E007".to_string(),
            message: format!("Invalid number format: '{value}'"),
            position: Some(PositionJs {
                line: 1,
                column: *position + 1,
                offset: *position,
            }),
            suggestion: Some("Numbers must be valid integers or decimals".to_string()),
        },
        FilterError::TypeMismatch {
            field,
            expected,
            actual,
        } => FilterExceptionJs {
            code: "E101".to_string(),
            message: format!(
                "Type mismatch: expected {expected} but got {actual} for field '{field}'"
            ),
            position: None,
            suggestion: Some(format!("Ensure '{field}' contains {expected} values")),
        },
        FilterError::UnknownField { field } => FilterExceptionJs {
            code: "E102".to_string(),
            message: format!("Unknown field: '{field}'"),
            position: None,
            suggestion: None,
        },
        FilterError::IncompatibleTypes {
            left_type,
            right_type,
        } => FilterExceptionJs {
            code: "E103".to_string(),
            message: format!("Cannot compare {left_type} with {right_type}"),
            position: None,
            suggestion: Some("Ensure both values have compatible types".to_string()),
        },
        FilterError::InvalidOperatorForType {
            operator,
            value_type,
        } => FilterExceptionJs {
            code: "E104".to_string(),
            message: format!("Operator '{operator}' is not valid for type '{value_type}'"),
            position: None,
            suggestion: None,
        },
        FilterError::DivisionByZero => FilterExceptionJs {
            code: "E201".to_string(),
            message: "Division by zero".to_string(),
            position: None,
            suggestion: None,
        },
        FilterError::NullValue { field } => FilterExceptionJs {
            code: "E202".to_string(),
            message: format!("Null value for field '{field}'"),
            position: None,
            suggestion: Some("Use IS NULL / IS NOT NULL to handle null values".to_string()),
        },
        FilterError::IndexOutOfBounds { index, length } => FilterExceptionJs {
            code: "E203".to_string(),
            message: format!("Array index {index} out of bounds (length: {length})"),
            position: None,
            suggestion: None,
        },
        FilterError::InvalidExpression { message } => FilterExceptionJs {
            code: "E204".to_string(),
            message: message.clone(),
            position: None,
            suggestion: None,
        },
        FilterError::NestingTooDeep {
            max_depth,
            actual_depth,
        } => FilterExceptionJs {
            code: "E301".to_string(),
            message: format!(
                "Filter nesting too deep: depth {actual_depth} exceeds limit of {max_depth}"
            ),
            position: None,
            suggestion: Some("Reduce parenthesis nesting depth".to_string()),
        },
        FilterError::InputTooLong {
            max_length,
            actual_length,
        } => FilterExceptionJs {
            code: "E302".to_string(),
            message: format!("Input too long: {actual_length} bytes exceeds limit of {max_length}"),
            position: None,
            suggestion: Some("Simplify the filter or split into multiple queries".to_string()),
        },
        FilterError::ExpressionTooComplex {
            max_nodes,
            actual_nodes,
        } => FilterExceptionJs {
            code: "E303".to_string(),
            message: format!("Too many nodes: {actual_nodes} exceeds limit of {max_nodes}"),
            position: None,
            suggestion: Some("Simplify the filter expression".to_string()),
        },
        FilterError::ArrayTooLarge {
            max_elements,
            actual_elements,
        } => FilterExceptionJs {
            code: "E304".to_string(),
            message: format!(
                "Array too large: {actual_elements} elements exceeds limit of {max_elements}"
            ),
            position: None,
            suggestion: None,
        },
        FilterError::InvalidStrategy(message) => FilterExceptionJs {
            code: "E401".to_string(),
            message: message.clone(),
            position: None,
            suggestion: None,
        },
    }
}

/// Convert error to JSON value (for validation results).
fn error_to_json(error: &FilterError) -> serde_json::Value {
    let exception = filter_error_to_exception(error);
    serde_json::to_value(exception).unwrap_or_else(|_| {
        serde_json::json!({
            "code": "E000",
            "message": error.to_string()
        })
    })
}

// =============================================================================
// W23.4.4: Helper Structs and Functions
// =============================================================================

/// Validation result for JavaScript.
#[derive(Serialize)]
struct ValidationResult {
    valid: bool,
    errors: Vec<serde_json::Value>,
    warnings: Vec<serde_json::Value>,
}

/// Filter information for JavaScript.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FilterInfo {
    node_count: usize,
    depth: usize,
    fields: Vec<String>,
    operators: Vec<String>,
    complexity: usize,
}

/// Count total nodes in the AST.
fn count_nodes(expr: &FilterExpr) -> usize {
    match expr {
        FilterExpr::And(l, r)
        | FilterExpr::Or(l, r)
        | FilterExpr::Eq(l, r)
        | FilterExpr::Ne(l, r)
        | FilterExpr::Lt(l, r)
        | FilterExpr::Le(l, r)
        | FilterExpr::Gt(l, r)
        | FilterExpr::Ge(l, r)
        | FilterExpr::Contains(l, r)
        | FilterExpr::StartsWith(l, r)
        | FilterExpr::EndsWith(l, r)
        | FilterExpr::Like(l, r)
        | FilterExpr::In(l, r)
        | FilterExpr::NotIn(l, r)
        | FilterExpr::Any(l, r)
        | FilterExpr::All(l, r)
        | FilterExpr::None(l, r) => 1 + count_nodes(l) + count_nodes(r),
        FilterExpr::Between(field, low, high) => {
            1 + count_nodes(field) + count_nodes(low) + count_nodes(high)
        }
        FilterExpr::Not(inner) | FilterExpr::IsNull(inner) | FilterExpr::IsNotNull(inner) => {
            1 + count_nodes(inner)
        }
        FilterExpr::LiteralArray(items) => 1 + items.iter().map(count_nodes).sum::<usize>(),
        _ => 1,
    }
}

/// Collect all field names referenced in the expression.
fn collect_fields(expr: &FilterExpr) -> Vec<String> {
    let mut fields = HashSet::new();
    collect_fields_recursive(expr, &mut fields);
    let mut result: Vec<String> = fields.into_iter().collect();
    result.sort();
    result
}

fn collect_fields_recursive(expr: &FilterExpr, fields: &mut HashSet<String>) {
    match expr {
        FilterExpr::Field(name) => {
            fields.insert(name.clone());
        }
        FilterExpr::And(l, r)
        | FilterExpr::Or(l, r)
        | FilterExpr::Eq(l, r)
        | FilterExpr::Ne(l, r)
        | FilterExpr::Lt(l, r)
        | FilterExpr::Le(l, r)
        | FilterExpr::Gt(l, r)
        | FilterExpr::Ge(l, r)
        | FilterExpr::Contains(l, r)
        | FilterExpr::StartsWith(l, r)
        | FilterExpr::EndsWith(l, r)
        | FilterExpr::Like(l, r)
        | FilterExpr::In(l, r)
        | FilterExpr::NotIn(l, r)
        | FilterExpr::Any(l, r)
        | FilterExpr::All(l, r)
        | FilterExpr::None(l, r) => {
            collect_fields_recursive(l, fields);
            collect_fields_recursive(r, fields);
        }
        FilterExpr::Between(field, low, high) => {
            collect_fields_recursive(field, fields);
            collect_fields_recursive(low, fields);
            collect_fields_recursive(high, fields);
        }
        FilterExpr::Not(inner) | FilterExpr::IsNull(inner) | FilterExpr::IsNotNull(inner) => {
            collect_fields_recursive(inner, fields);
        }
        FilterExpr::LiteralArray(items) => {
            for item in items {
                collect_fields_recursive(item, fields);
            }
        }
        _ => {}
    }
}

/// Collect all operators used in the expression.
fn collect_operators(expr: &FilterExpr) -> Vec<String> {
    let mut ops = HashSet::new();
    collect_operators_recursive(expr, &mut ops);
    let mut result: Vec<String> = ops.into_iter().collect();
    result.sort();
    result
}

fn collect_operators_recursive(expr: &FilterExpr, ops: &mut HashSet<String>) {
    let op_name = match expr {
        FilterExpr::And(_, _) => Some("and"),
        FilterExpr::Or(_, _) => Some("or"),
        FilterExpr::Not(_) => Some("not"),
        FilterExpr::Eq(_, _) => Some("eq"),
        FilterExpr::Ne(_, _) => Some("ne"),
        FilterExpr::Lt(_, _) => Some("lt"),
        FilterExpr::Le(_, _) => Some("le"),
        FilterExpr::Gt(_, _) => Some("gt"),
        FilterExpr::Ge(_, _) => Some("ge"),
        FilterExpr::Between(_, _, _) => Some("between"),
        FilterExpr::Contains(_, _) => Some("contains"),
        FilterExpr::StartsWith(_, _) => Some("starts_with"),
        FilterExpr::EndsWith(_, _) => Some("ends_with"),
        FilterExpr::Like(_, _) => Some("like"),
        FilterExpr::In(_, _) => Some("in"),
        FilterExpr::NotIn(_, _) => Some("not_in"),
        FilterExpr::Any(_, _) => Some("any"),
        FilterExpr::All(_, _) => Some("all"),
        FilterExpr::None(_, _) => Some("none"),
        FilterExpr::IsNull(_) => Some("is_null"),
        FilterExpr::IsNotNull(_) => Some("is_not_null"),
        _ => None,
    };

    if let Some(name) = op_name {
        ops.insert(name.to_string());
    }

    // Recurse into children
    match expr {
        FilterExpr::And(l, r)
        | FilterExpr::Or(l, r)
        | FilterExpr::Eq(l, r)
        | FilterExpr::Ne(l, r)
        | FilterExpr::Lt(l, r)
        | FilterExpr::Le(l, r)
        | FilterExpr::Gt(l, r)
        | FilterExpr::Ge(l, r)
        | FilterExpr::Contains(l, r)
        | FilterExpr::StartsWith(l, r)
        | FilterExpr::EndsWith(l, r)
        | FilterExpr::Like(l, r)
        | FilterExpr::In(l, r)
        | FilterExpr::NotIn(l, r)
        | FilterExpr::Any(l, r)
        | FilterExpr::All(l, r)
        | FilterExpr::None(l, r) => {
            collect_operators_recursive(l, ops);
            collect_operators_recursive(r, ops);
        }
        FilterExpr::Between(field, low, high) => {
            collect_operators_recursive(field, ops);
            collect_operators_recursive(low, ops);
            collect_operators_recursive(high, ops);
        }
        FilterExpr::Not(inner) | FilterExpr::IsNull(inner) | FilterExpr::IsNotNull(inner) => {
            collect_operators_recursive(inner, ops);
        }
        FilterExpr::LiteralArray(items) => {
            for item in items {
                collect_operators_recursive(item, ops);
            }
        }
        _ => {}
    }
}

/// Estimate filter complexity (1-10 scale).
fn estimate_complexity(expr: &FilterExpr) -> usize {
    let nodes = count_nodes(expr);
    let depth = expr.depth();

    // Simple heuristic: nodes + depth * 2, clamped to 1-10
    let raw = nodes + depth * 2;
    raw.clamp(1, 10)
}

// =============================================================================
// W23.4.2: search_filtered_js() WASM Export
// =============================================================================

// NOTE: search_filtered_js is implemented in src/wasm/mod.rs as a method on EdgeVec
// because it requires access to the index, storage, and metadata which are
// encapsulated in the EdgeVec struct.

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Native Tests (work on all targets)
    // These test pure Rust logic without JsValue
    // =========================================================================

    #[test]
    fn test_parse_and_serialize_simple() {
        // Test parsing via native Rust API
        let ast = parse("category = \"gpu\"").unwrap();
        let json = serde_json::to_string(&ast).unwrap();
        assert!(json.contains("LiteralString"));
        assert!(json.contains("gpu"));
    }

    #[test]
    fn test_parse_invalid_syntax() {
        // Test that parsing fails on invalid input
        let result = parse("invalid >>");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_filter_valid() {
        let result = validate_filter_js("price < 500");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["valid"], true);
        assert!(parsed["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_validate_filter_invalid() {
        let result = validate_filter_js("price <");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["valid"], false);
        assert!(!parsed["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_get_filter_info_native() {
        // Parse and analyze directly
        let ast = parse("category = \"gpu\" AND price < 500").unwrap();
        let info = FilterInfo {
            node_count: count_nodes(&ast),
            depth: ast.depth(),
            fields: collect_fields(&ast),
            operators: collect_operators(&ast),
            complexity: estimate_complexity(&ast),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["nodeCount"].as_u64().unwrap() >= 5);
        assert!(parsed["fields"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("category")));
        assert!(parsed["fields"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("price")));
    }

    // =========================================================================
    // WASM-specific tests (only run on wasm32 target)
    // These tests use JsValue which only works in WASM runtime
    // =========================================================================

    #[cfg(target_arch = "wasm32")]
    mod wasm_tests {
        use super::*;
        use wasm_bindgen_test::*;

        #[wasm_bindgen_test]
        fn test_parse_filter_js_success() {
            let result = parse_filter_js("category = \"gpu\"");
            assert!(result.is_ok());
            let json = result.unwrap();
            assert!(json.contains("LiteralString"));
        }

        #[wasm_bindgen_test]
        fn test_parse_filter_js_error() {
            let result = parse_filter_js("invalid >>");
            assert!(result.is_err());
        }

        #[wasm_bindgen_test]
        fn test_try_parse_filter_success() {
            let result = try_parse_filter_js("x = 1");
            assert!(!result.is_null());
        }

        #[wasm_bindgen_test]
        fn test_try_parse_filter_failure() {
            let result = try_parse_filter_js("<<<invalid>>>");
            assert!(result.is_null());
        }

        #[wasm_bindgen_test]
        fn test_get_filter_info_js_success() {
            let result = get_filter_info_js("category = \"gpu\" AND price < 500");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_count_nodes_simple() {
        let expr = FilterExpr::Eq(
            Box::new(FilterExpr::Field("x".to_string())),
            Box::new(FilterExpr::LiteralInt(1)),
        );
        assert_eq!(count_nodes(&expr), 3);
    }

    #[test]
    fn test_count_nodes_complex() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Eq(
                Box::new(FilterExpr::Field("a".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            )),
            Box::new(FilterExpr::Lt(
                Box::new(FilterExpr::Field("b".to_string())),
                Box::new(FilterExpr::LiteralInt(2)),
            )),
        );
        // and(eq(field, int), lt(field, int)) = 1 + 3 + 3 = 7
        assert_eq!(count_nodes(&expr), 7);
    }

    #[test]
    fn test_collect_fields() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Eq(
                Box::new(FilterExpr::Field("category".to_string())),
                Box::new(FilterExpr::LiteralString("gpu".to_string())),
            )),
            Box::new(FilterExpr::Lt(
                Box::new(FilterExpr::Field("price".to_string())),
                Box::new(FilterExpr::LiteralInt(500)),
            )),
        );
        let fields = collect_fields(&expr);
        assert_eq!(fields, vec!["category", "price"]);
    }

    #[test]
    fn test_collect_operators() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Eq(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            )),
            Box::new(FilterExpr::Lt(
                Box::new(FilterExpr::Field("y".to_string())),
                Box::new(FilterExpr::LiteralInt(2)),
            )),
        );
        let ops = collect_operators(&expr);
        assert!(ops.contains(&"and".to_string()));
        assert!(ops.contains(&"eq".to_string()));
        assert!(ops.contains(&"lt".to_string()));
    }

    #[test]
    fn test_estimate_complexity() {
        // Simple expression
        let simple = FilterExpr::Eq(
            Box::new(FilterExpr::Field("x".to_string())),
            Box::new(FilterExpr::LiteralInt(1)),
        );
        let complexity = estimate_complexity(&simple);
        assert!((1..=10).contains(&complexity));

        // Complex expression should have higher complexity
        let complex = FilterExpr::And(
            Box::new(FilterExpr::Or(
                Box::new(simple.clone()),
                Box::new(simple.clone()),
            )),
            Box::new(FilterExpr::Not(Box::new(simple))),
        );
        let complex_score = estimate_complexity(&complex);
        assert!(complex_score > complexity);
    }

    #[test]
    fn test_error_exception_syntax() {
        let error = FilterError::SyntaxError {
            position: 10,
            line: 1,
            column: 11,
            message: "Expected operator".to_string(),
            suggestion: Some("Did you mean '='?".to_string()),
        };

        // Test the exception struct directly (works on native)
        let exception = filter_error_to_exception(&error);
        let json = serde_json::to_string(&exception).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["code"], "E001");
        assert!(parsed["message"]
            .as_str()
            .unwrap()
            .contains("Expected operator"));
        assert_eq!(parsed["position"]["line"], 1);
        assert_eq!(parsed["position"]["column"], 11);
        assert!(parsed["suggestion"].as_str().unwrap().contains('='));
    }

    #[test]
    fn test_error_exception_type_mismatch() {
        let error = FilterError::TypeMismatch {
            field: "price".to_string(),
            expected: "integer".to_string(),
            actual: "string".to_string(),
        };

        let exception = filter_error_to_exception(&error);
        let json = serde_json::to_string(&exception).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["code"], "E101");
        assert!(parsed["message"]
            .as_str()
            .unwrap()
            .contains("Type mismatch"));
        assert!(parsed["position"].is_null());
    }

    #[test]
    fn test_validation_result_structure() {
        let result = validate_filter_js("category = \"gpu\"");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.get("valid").is_some());
        assert!(parsed.get("errors").is_some());
        assert!(parsed.get("warnings").is_some());
        assert!(parsed["errors"].is_array());
        assert!(parsed["warnings"].is_array());
    }
}
