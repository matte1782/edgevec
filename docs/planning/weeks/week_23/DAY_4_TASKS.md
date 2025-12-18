# Week 23 Day 4: WASM Bindings

**Date:** Day 4 of Week 23
**Focus:** Rust-to-JavaScript WASM Bindings for Filter System
**Agent:** WASM_SPECIALIST
**Total Hours:** 12h
**Status:** [PLANNED]

---

## Executive Summary

Day 4 exposes the Rust filter system to JavaScript through WASM bindings. This is the bridge that allows browser and Node.js applications to use EdgeVec's filtering capabilities.

**Prerequisites:**
- W23.3.3 (search_filtered API) COMPLETE
- All Rust filter components functional
- FILTERING_WASM_API.md specification approved

---

## Tasks Overview

| Task ID | Description | Hours | Priority |
|:--------|:------------|:------|:---------|
| W23.4.1 | parse_filter_js() WASM export | 3h | P0 |
| W23.4.2 | search_filtered_js() WASM export | 4h | P0 |
| W23.4.3 | FilterError → JsValue serialization | 2h | P0 |
| W23.4.4 | JSON serialization for FilterExpr | 3h | P0 |

---

## Task W23.4.1: parse_filter_js() WASM Export

### Description
Export filter parsing function to JavaScript via wasm-bindgen.

### Hours: 3h

### Specification

**File:** `src/wasm/filter.rs`

```rust
//! WASM bindings for EdgeVec filter system.
//!
//! Exposes filter parsing, validation, and search to JavaScript.

use wasm_bindgen::prelude::*;
use crate::filter::{parse, validate, FilterExpr, FilterError};

/// Parse a filter expression string into a compiled filter.
///
/// # Arguments
/// * `filter_str` - Filter expression in EdgeVec syntax
///
/// # Returns
/// * JSON string representation of the parsed filter AST
///
/// # Throws
/// * FilterException with code, message, position, and suggestion
///
/// # Example (JavaScript)
/// ```javascript
/// try {
///     const filterJson = parse_filter_js('category = "gpu" AND price < 500');
///     console.log(JSON.parse(filterJson));
/// } catch (e) {
///     console.error('Parse error:', e.message);
/// }
/// ```
#[wasm_bindgen]
pub fn parse_filter_js(filter_str: &str) -> Result<String, JsValue> {
    let ast = parse(filter_str).map_err(|e| filter_error_to_jsvalue(&e))?;

    serde_json::to_string(&ast)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Validate a filter string without fully compiling.
///
/// # Arguments
/// * `filter_str` - Filter expression to validate
///
/// # Returns
/// * JSON string with validation result:
///   ```json
///   {
///     "valid": true/false,
///     "errors": [...],
///     "warnings": [...]
///   }
///   ```
///
/// # Example (JavaScript)
/// ```javascript
/// const result = JSON.parse(validate_filter_js('price <'));
/// if (!result.valid) {
///     console.log('Errors:', result.errors);
/// }
/// ```
#[wasm_bindgen]
pub fn validate_filter_js(filter_str: &str) -> String {
    let result = validate(filter_str);

    serde_json::to_string(&ValidationResult {
        valid: result.is_ok(),
        errors: match &result {
            Err(e) => vec![error_to_json(e)],
            Ok(_) => vec![],
        },
        warnings: vec![], // Future: add warnings for suspicious patterns
        filter: result.ok().map(|f| serde_json::to_value(&f).ok()).flatten(),
    }).unwrap_or_else(|_| r#"{"valid":false,"errors":["Internal serialization error"]}"#.to_string())
}

/// Try to parse a filter string, returning null on error.
///
/// # Arguments
/// * `filter_str` - Filter expression to parse
///
/// # Returns
/// * JSON string of parsed filter, or null if invalid
///
/// # Example (JavaScript)
/// ```javascript
/// const filter = try_parse_filter_js(userInput);
/// if (filter !== null) {
///     // Valid filter
/// }
/// ```
#[wasm_bindgen]
pub fn try_parse_filter_js(filter_str: &str) -> JsValue {
    match parse(filter_str) {
        Ok(ast) => {
            match serde_json::to_string(&ast) {
                Ok(json) => JsValue::from_str(&json),
                Err(_) => JsValue::NULL,
            }
        }
        Err(_) => JsValue::NULL,
    }
}

/// Get filter information (complexity, fields, operators).
///
/// # Arguments
/// * `filter_str` - Filter expression to analyze
///
/// # Returns
/// * JSON string with filter info:
///   ```json
///   {
///     "nodeCount": 5,
///     "depth": 3,
///     "fields": ["category", "price"],
///     "operators": ["eq", "lt", "and"],
///     "complexity": 3
///   }
///   ```
#[wasm_bindgen]
pub fn get_filter_info_js(filter_str: &str) -> Result<String, JsValue> {
    let ast = parse(filter_str).map_err(|e| filter_error_to_jsvalue(&e))?;

    let info = FilterInfo {
        node_count: count_nodes(&ast),
        depth: calculate_depth(&ast),
        fields: collect_fields(&ast),
        operators: collect_operators(&ast),
        complexity: estimate_complexity(&ast),
    };

    serde_json::to_string(&info)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// Helper structs for JSON serialization
#[derive(serde::Serialize)]
struct ValidationResult {
    valid: bool,
    errors: Vec<serde_json::Value>,
    warnings: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct FilterInfo {
    node_count: usize,
    depth: usize,
    fields: Vec<String>,
    operators: Vec<String>,
    complexity: usize,
}

// Helper functions
fn count_nodes(expr: &FilterExpr) -> usize {
    match expr {
        FilterExpr::And(l, r) | FilterExpr::Or(l, r) => 1 + count_nodes(l) + count_nodes(r),
        FilterExpr::Not(inner) => 1 + count_nodes(inner),
        _ => 1,
    }
}

fn calculate_depth(expr: &FilterExpr) -> usize {
    match expr {
        FilterExpr::And(l, r) | FilterExpr::Or(l, r) => {
            1 + calculate_depth(l).max(calculate_depth(r))
        }
        FilterExpr::Not(inner) => 1 + calculate_depth(inner),
        _ => 1,
    }
}

fn collect_fields(expr: &FilterExpr) -> Vec<String> {
    let mut fields = Vec::new();
    collect_fields_recursive(expr, &mut fields);
    fields.sort();
    fields.dedup();
    fields
}

fn collect_fields_recursive(expr: &FilterExpr, fields: &mut Vec<String>) {
    match expr {
        FilterExpr::Field(name) => fields.push(name.clone()),
        FilterExpr::And(l, r) | FilterExpr::Or(l, r) => {
            collect_fields_recursive(l, fields);
            collect_fields_recursive(r, fields);
        }
        FilterExpr::Not(inner) => collect_fields_recursive(inner, fields),
        FilterExpr::Eq(l, r) | FilterExpr::Ne(l, r) |
        FilterExpr::Lt(l, r) | FilterExpr::Le(l, r) |
        FilterExpr::Gt(l, r) | FilterExpr::Ge(l, r) => {
            collect_fields_recursive(l, fields);
            collect_fields_recursive(r, fields);
        }
        _ => {}
    }
}

fn collect_operators(expr: &FilterExpr) -> Vec<String> {
    let mut ops = Vec::new();
    collect_operators_recursive(expr, &mut ops);
    ops.sort();
    ops.dedup();
    ops
}

fn collect_operators_recursive(expr: &FilterExpr, ops: &mut Vec<String>) {
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
        ops.push(name.to_string());
    }

    // Recurse
    match expr {
        FilterExpr::And(l, r) | FilterExpr::Or(l, r) => {
            collect_operators_recursive(l, ops);
            collect_operators_recursive(r, ops);
        }
        FilterExpr::Not(inner) => collect_operators_recursive(inner, ops),
        _ => {}
    }
}

fn estimate_complexity(expr: &FilterExpr) -> usize {
    // Simple complexity score: 1-10 based on node count and depth
    let nodes = count_nodes(expr);
    let depth = calculate_depth(expr);

    let raw = nodes + depth * 2;
    raw.min(10).max(1)
}
```

### Acceptance Criteria
- [ ] `parse_filter_js()` callable from JavaScript
- [ ] Returns JSON string of AST on success
- [ ] Throws structured error on parse failure
- [ ] `validate_filter_js()` returns validation result
- [ ] `try_parse_filter_js()` returns null on error
- [ ] `get_filter_info_js()` returns filter metadata
- [ ] `wasm-pack test --node` passes

### Test Cases
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_parse_filter_simple() {
        let result = parse_filter_js("category = \"gpu\"");
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"op\":\"eq\""));
    }

    #[wasm_bindgen_test]
    fn test_parse_filter_invalid() {
        let result = parse_filter_js("invalid >>");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_validate_filter_valid() {
        let result = validate_filter_js("price < 500");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["valid"], true);
    }

    #[wasm_bindgen_test]
    fn test_validate_filter_invalid() {
        let result = validate_filter_js("price <");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["valid"], false);
        assert!(!parsed["errors"].as_array().unwrap().is_empty());
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
    fn test_get_filter_info() {
        let result = get_filter_info_js("category = \"gpu\" AND price < 500");
        assert!(result.is_ok());
        let info: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(info["nodeCount"].as_u64().unwrap() >= 3);
        assert!(info["fields"].as_array().unwrap().contains(&serde_json::json!("category")));
    }
}
```

---

## Task W23.4.2: search_filtered_js() WASM Export

### Description
Export filtered search function to JavaScript with full options support.

### Hours: 4h

### Specification

**File:** `src/wasm/filter.rs` (continued)

```rust
use crate::hnsw::HnswIndex;
use crate::filter::{FilterStrategy, FilteredSearchResult};

/// Execute a filtered search on the index.
///
/// # Arguments
/// * `index_ptr` - Pointer to HnswIndex (from create_index_js)
/// * `query_json` - JSON array of query vector floats
/// * `k` - Number of results to return
/// * `options_json` - JSON object with search options:
///   ```json
///   {
///     "filter": "category = \"gpu\"",  // or null
///     "strategy": "auto",              // "auto" | "pre" | "post" | "hybrid"
///     "oversampleFactor": 3.0,         // for post/hybrid
///     "includeMetadata": true,
///     "includeVectors": false
///   }
///   ```
///
/// # Returns
/// * JSON string with search results:
///   ```json
///   {
///     "results": [
///       { "id": 42, "score": 0.95, "metadata": {...} }
///     ],
///     "complete": true,
///     "observedSelectivity": 0.15,
///     "strategyUsed": "hybrid",
///     "vectorsEvaluated": 150,
///     "filterTimeMs": 2.5,
///     "totalTimeMs": 8.3
///   }
///   ```
///
/// # Throws
/// * FilterException on filter error
/// * Error on search failure
#[wasm_bindgen]
pub fn search_filtered_js(
    index_ptr: *mut HnswIndex,
    query_json: &str,
    k: usize,
    options_json: &str,
) -> Result<String, JsValue> {
    let start_time = instant::Instant::now();

    // Parse query vector
    let query: Vec<f32> = serde_json::from_str(query_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid query vector: {}", e)))?;

    // Parse options
    let options: SearchOptionsJs = serde_json::from_str(options_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid options: {}", e)))?;

    // Get index reference (unsafe but necessary for WASM)
    let index = unsafe {
        if index_ptr.is_null() {
            return Err(JsValue::from_str("Index pointer is null"));
        }
        &*index_ptr
    };

    // Parse filter if provided
    let filter = match &options.filter {
        Some(filter_str) => Some(parse(filter_str).map_err(|e| filter_error_to_jsvalue(&e))?),
        None => None,
    };

    // Convert strategy
    let strategy = match options.strategy.as_deref() {
        Some("pre") => FilterStrategy::PreFilter,
        Some("post") => FilterStrategy::PostFilter {
            oversample: options.oversample_factor.unwrap_or(3.0),
        },
        Some("hybrid") => FilterStrategy::Hybrid {
            oversample_min: 1.5,
            oversample_max: options.oversample_factor.unwrap_or(10.0),
        },
        _ => FilterStrategy::Auto,
    };

    let filter_start = instant::Instant::now();

    // Execute search
    let result = index.search_filtered(
        &query,
        k,
        filter.as_ref(),
        strategy,
    ).map_err(|e| filter_error_to_jsvalue(&e))?;

    let filter_time = filter_start.elapsed();
    let total_time = start_time.elapsed();

    // Build response
    let response = SearchResultJs {
        results: result.results.iter().map(|r| {
            let mut res = SearchResultItemJs {
                id: r.id,
                score: r.score,
                metadata: None,
                vector: None,
            };

            if options.include_metadata.unwrap_or(false) {
                if let Some(meta) = index.get_metadata(r.id) {
                    res.metadata = Some(serde_json::to_value(&meta).ok());
                }
            }

            if options.include_vectors.unwrap_or(false) {
                if let Some(vec) = index.get_vector(r.id) {
                    res.vector = Some(vec.to_vec());
                }
            }

            res
        }).collect(),
        complete: result.complete,
        observed_selectivity: result.observed_selectivity,
        strategy_used: strategy_to_string(&result.strategy_used),
        vectors_evaluated: result.vectors_evaluated,
        filter_time_ms: filter_time.as_secs_f64() * 1000.0,
        total_time_ms: total_time.as_secs_f64() * 1000.0,
    };

    serde_json::to_string(&response)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Simple search without filter (convenience wrapper).
#[wasm_bindgen]
pub fn search_js(
    index_ptr: *mut HnswIndex,
    query_json: &str,
    k: usize,
) -> Result<String, JsValue> {
    search_filtered_js(index_ptr, query_json, k, "{}")
}

// Helper structs for JSON serialization
#[derive(serde::Deserialize)]
struct SearchOptionsJs {
    filter: Option<String>,
    strategy: Option<String>,
    #[serde(rename = "oversampleFactor")]
    oversample_factor: Option<f32>,
    #[serde(rename = "includeMetadata")]
    include_metadata: Option<bool>,
    #[serde(rename = "includeVectors")]
    include_vectors: Option<bool>,
}

#[derive(serde::Serialize)]
struct SearchResultJs {
    results: Vec<SearchResultItemJs>,
    complete: bool,
    #[serde(rename = "observedSelectivity")]
    observed_selectivity: f32,
    #[serde(rename = "strategyUsed")]
    strategy_used: String,
    #[serde(rename = "vectorsEvaluated")]
    vectors_evaluated: usize,
    #[serde(rename = "filterTimeMs")]
    filter_time_ms: f64,
    #[serde(rename = "totalTimeMs")]
    total_time_ms: f64,
}

#[derive(serde::Serialize)]
struct SearchResultItemJs {
    id: usize,
    score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Option<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector: Option<Vec<f32>>,
}

fn strategy_to_string(strategy: &FilterStrategy) -> String {
    match strategy {
        FilterStrategy::PreFilter => "pre".to_string(),
        FilterStrategy::PostFilter { .. } => "post".to_string(),
        FilterStrategy::Hybrid { .. } => "hybrid".to_string(),
        FilterStrategy::Auto => "auto".to_string(),
    }
}
```

### Acceptance Criteria
- [ ] `search_filtered_js()` callable from JavaScript
- [ ] Accepts query as JSON array
- [ ] Accepts options with filter, strategy, etc.
- [ ] Returns results with timing diagnostics
- [ ] includeMetadata and includeVectors work
- [ ] All strategy options work
- [ ] `wasm-pack test --node` passes

### Test Cases
```rust
#[wasm_bindgen_test]
fn test_search_filtered_no_filter() {
    let index = create_test_wasm_index();
    let query = serde_json::to_string(&vec![0.1f32; 384]).unwrap();

    let result = search_filtered_js(
        index as *mut _,
        &query,
        5,
        "{}"
    );

    assert!(result.is_ok());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(parsed["results"].as_array().unwrap().len() <= 5);
}

#[wasm_bindgen_test]
fn test_search_filtered_with_filter() {
    let index = create_test_wasm_index_with_metadata();
    let query = serde_json::to_string(&vec![0.1f32; 384]).unwrap();
    let options = r#"{"filter": "category = \"gpu\"", "strategy": "auto"}"#;

    let result = search_filtered_js(
        index as *mut _,
        &query,
        5,
        options
    );

    assert!(result.is_ok());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(parsed["complete"].is_boolean());
    assert!(parsed["strategyUsed"].is_string());
}

#[wasm_bindgen_test]
fn test_search_filtered_include_metadata() {
    let index = create_test_wasm_index_with_metadata();
    let query = serde_json::to_string(&vec![0.1f32; 384]).unwrap();
    let options = r#"{"includeMetadata": true}"#;

    let result = search_filtered_js(
        index as *mut _,
        &query,
        5,
        options
    );

    assert!(result.is_ok());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    // Check that results have metadata
    for r in parsed["results"].as_array().unwrap() {
        assert!(r.get("metadata").is_some());
    }
}

#[wasm_bindgen_test]
fn test_search_filtered_strategy_override() {
    let index = create_test_wasm_index_with_metadata();
    let query = serde_json::to_string(&vec![0.1f32; 384]).unwrap();
    let options = r#"{"filter": "active = true", "strategy": "pre"}"#;

    let result = search_filtered_js(
        index as *mut _,
        &query,
        5,
        options
    );

    assert!(result.is_ok());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed["strategyUsed"], "pre");
}
```

---

## Task W23.4.3: FilterError → JsValue Serialization

### Description
Create rich error serialization that provides JavaScript with structured error information.

### Hours: 2h

### Specification

**File:** `src/wasm/filter.rs` (continued)

```rust
use serde::Serialize;

/// Structured error for JavaScript consumption.
#[derive(Serialize)]
struct FilterExceptionJs {
    /// Error code for programmatic handling (e.g., "E100")
    code: String,
    /// Human-readable error message
    message: String,
    /// Position in filter string (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<PositionJs>,
    /// Suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    /// The filter string that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "filterString")]
    filter_string: Option<String>,
    /// Formatted error with source snippet
    #[serde(skip_serializing_if = "Option::is_none")]
    formatted: Option<String>,
}

#[derive(Serialize)]
struct PositionJs {
    line: usize,
    column: usize,
    offset: usize,
}

/// Convert FilterError to JsValue for throwing.
fn filter_error_to_jsvalue(error: &FilterError) -> JsValue {
    let (code, message, position, suggestion) = match error {
        FilterError::SyntaxError { message, position, suggestion } => (
            "E100",
            message.clone(),
            position.clone(),
            suggestion.clone(),
        ),
        FilterError::UnexpectedEof { position } => (
            "E101",
            "Unexpected end of input".to_string(),
            Some(*position),
            Some("The filter expression appears to be incomplete".to_string()),
        ),
        FilterError::InvalidChar { char, position } => (
            "E102",
            format!("Invalid character '{}' at position {}", char, position),
            Some(*position),
            None,
        ),
        FilterError::UnclosedString { start_position } => (
            "E103",
            format!("Unclosed string literal starting at position {}", start_position),
            Some(*start_position),
            Some("Add a closing quote (\") to complete the string".to_string()),
        ),
        FilterError::UnclosedParen { start_position } => (
            "E104",
            format!("Unclosed parenthesis starting at position {}", start_position),
            Some(*start_position),
            Some("Add a closing parenthesis )".to_string()),
        ),
        FilterError::InvalidNumber { value, position } => (
            "E105",
            format!("Invalid number format: '{}'", value),
            Some(*position),
            Some("Numbers must be valid integers or decimals".to_string()),
        ),
        FilterError::InvalidEscape { sequence, position } => (
            "E106",
            format!("Invalid escape sequence: '{}'", sequence),
            Some(*position),
            Some("Valid escapes: \\n, \\t, \\r, \\\", \\\\".to_string()),
        ),
        FilterError::TypeMismatch { expected, actual, field, position } => (
            "E200",
            format!("Type mismatch: expected {} but got {} for field '{}'", expected, actual, field),
            position.clone(),
            Some(format!("Ensure '{}' contains {} values", field, expected)),
        ),
        FilterError::UnknownField { field, suggestions } => (
            "E201",
            format!("Unknown field: '{}'", field),
            None,
            if suggestions.is_empty() {
                None
            } else {
                Some(format!("Did you mean: {}?", suggestions.join(", ")))
            },
        ),
        FilterError::Overflow { value, max } => (
            "E202",
            format!("Integer overflow: {} exceeds maximum {}", value, max),
            None,
            Some("Use a smaller number or a float".to_string()),
        ),
        FilterError::NotAnArray { field } => (
            "E203",
            format!("Field '{}' is not an array", field),
            None,
            Some("Array operators (ANY, ALL, NONE) require array-valued fields".to_string()),
        ),
        FilterError::EmptyArray { position } => (
            "E204",
            "Empty array in IN/NOT IN clause".to_string(),
            Some(*position),
            Some("Provide at least one value in the array".to_string()),
        ),
        FilterError::TooComplex { nodes, max } => (
            "E300",
            format!("Filter too complex: {} nodes exceeds limit of {}", nodes, max),
            None,
            Some("Simplify the filter or break it into multiple queries".to_string()),
        ),
        FilterError::TooDeep { depth, max } => (
            "E301",
            format!("Filter nesting too deep: depth {} exceeds limit of {}", depth, max),
            None,
            Some("Reduce parenthesis nesting depth".to_string()),
        ),
        FilterError::StringTooLong { length, max } => (
            "E302",
            format!("String too long: {} characters exceeds limit of {}", length, max),
            None,
            None,
        ),
        FilterError::ArrayTooLarge { size, max } => (
            "E303",
            format!("Array too large: {} elements exceeds limit of {}", size, max),
            None,
            None,
        ),
        FilterError::EvaluationError { message } => (
            "E400",
            message.clone(),
            None,
            None,
        ),
        FilterError::MetadataError { message } => (
            "E401",
            message.clone(),
            None,
            None,
        ),
        FilterError::InvalidStrategy(message) => (
            "E500",
            message.clone(),
            None,
            None,
        ),
    };

    let exception = FilterExceptionJs {
        code: code.to_string(),
        message: message.clone(),
        position: position.map(|p| PositionJs {
            line: p.line,
            column: p.column,
            offset: p.offset,
        }),
        suggestion,
        filter_string: None, // Could be added if we track the original string
        formatted: Some(format_error_with_snippet(error)),
    };

    // Serialize to JSON and return as JsValue
    match serde_json::to_string(&exception) {
        Ok(json) => JsValue::from_str(&json),
        Err(_) => JsValue::from_str(&format!(r#"{{"code":"{}","message":"{}"}}"#, code, message)),
    }
}

/// Convert error to JSON value (for validation results).
fn error_to_json(error: &FilterError) -> serde_json::Value {
    match serde_json::to_value(filter_error_to_exception_struct(error)) {
        Ok(v) => v,
        Err(_) => serde_json::json!({
            "code": "E000",
            "message": error.to_string()
        }),
    }
}

fn filter_error_to_exception_struct(error: &FilterError) -> FilterExceptionJs {
    // Reuse logic from filter_error_to_jsvalue
    // ... (same mapping as above)
}

/// Format error with source snippet for display.
fn format_error_with_snippet(error: &FilterError) -> String {
    let (message, position) = match error {
        FilterError::SyntaxError { message, position, .. } => (message.clone(), position.clone()),
        FilterError::UnexpectedEof { position } => ("Unexpected end of input".to_string(), Some(*position)),
        _ => (error.to_string(), None),
    };

    if let Some(pos) = position {
        format!(
            "Error: {}\n  at line {}, column {}\n  {}^\n",
            message,
            pos.line,
            pos.column,
            " ".repeat(pos.column.saturating_sub(1))
        )
    } else {
        format!("Error: {}", message)
    }
}
```

### Acceptance Criteria
- [ ] All FilterError variants map to error codes
- [ ] Position information preserved where available
- [ ] Suggestions provided for common errors
- [ ] Formatted output includes source snippet
- [ ] JSON serialization always succeeds
- [ ] JavaScript can catch and parse errors

### Test Cases
```rust
#[test]
fn test_error_code_mapping() {
    let error = FilterError::SyntaxError {
        message: "test".to_string(),
        position: Some(Position { line: 1, column: 5, offset: 4 }),
        suggestion: Some("hint".to_string()),
    };

    let js_value = filter_error_to_jsvalue(&error);
    let json: String = js_value.as_string().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["code"], "E100");
    assert_eq!(parsed["position"]["line"], 1);
    assert_eq!(parsed["suggestion"], "hint");
}

#[test]
fn test_all_error_types_serialize() {
    // Test that every error variant can be serialized
    let errors = vec![
        FilterError::SyntaxError { message: "test".into(), position: None, suggestion: None },
        FilterError::UnexpectedEof { position: Position { line: 1, column: 1, offset: 0 } },
        FilterError::TypeMismatch { expected: "int".into(), actual: "string".into(), field: "x".into(), position: None },
        FilterError::TooComplex { nodes: 100, max: 50 },
        // ... add all variants
    ];

    for error in errors {
        let result = filter_error_to_jsvalue(&error);
        assert!(result.is_string());
        let json: serde_json::Value = serde_json::from_str(&result.as_string().unwrap()).unwrap();
        assert!(json.get("code").is_some());
        assert!(json.get("message").is_some());
    }
}
```

---

## Task W23.4.4: JSON Serialization for FilterExpr

### Description
Implement complete JSON serialization for FilterExpr AST to enable JavaScript interop.

### Hours: 3h

### Specification

**File:** `src/filter/ast.rs` (add serde derives and custom serialization)

```rust
use serde::{Deserialize, Serialize};

/// Filter expression AST node.
///
/// All variants serialize to a consistent JSON format with "op" discriminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FilterExpr {
    // Literals
    #[serde(rename = "literal_string")]
    LiteralString(String),

    #[serde(rename = "literal_int")]
    LiteralInt(i64),

    #[serde(rename = "literal_float")]
    LiteralFloat(f64),

    #[serde(rename = "literal_bool")]
    LiteralBool(bool),

    #[serde(rename = "literal_array")]
    LiteralArray(Vec<FilterExpr>),

    // Field reference
    #[serde(rename = "field")]
    Field(String),

    // Comparison operators
    #[serde(rename = "eq")]
    Eq(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "ne")]
    Ne(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "lt")]
    Lt(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "le")]
    Le(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "gt")]
    Gt(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "ge")]
    Ge(Box<FilterExpr>, Box<FilterExpr>),

    // Range
    #[serde(rename = "between")]
    Between(Box<FilterExpr>, Box<FilterExpr>, Box<FilterExpr>),

    // String operators
    #[serde(rename = "contains")]
    Contains(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "starts_with")]
    StartsWith(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "ends_with")]
    EndsWith(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "like")]
    Like(Box<FilterExpr>, Box<FilterExpr>),

    // Set operators
    #[serde(rename = "in")]
    In(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "not_in")]
    NotIn(Box<FilterExpr>, Box<FilterExpr>),

    // Array operators
    #[serde(rename = "any")]
    Any(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "all")]
    All(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "none_of")] // "none" conflicts with Option::None
    None(Box<FilterExpr>, Box<FilterExpr>),

    // NULL checks
    #[serde(rename = "is_null")]
    IsNull(Box<FilterExpr>),

    #[serde(rename = "is_not_null")]
    IsNotNull(Box<FilterExpr>),

    // Logical operators
    #[serde(rename = "and")]
    And(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "or")]
    Or(Box<FilterExpr>, Box<FilterExpr>),

    #[serde(rename = "not")]
    Not(Box<FilterExpr>),
}

impl FilterExpr {
    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// Custom serialization for complex types with better JSON structure
mod custom_serde {
    use super::*;
    use serde::{Serializer, Deserializer};

    /// Serialize binary operator with named fields.
    pub fn serialize_binary<S: Serializer>(
        op: &str,
        left: &FilterExpr,
        right: &FilterExpr,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("op", op)?;
        map.serialize_entry("left", left)?;
        map.serialize_entry("right", right)?;
        map.end()
    }
}
```

**JSON Output Examples:**

```json
// category = "gpu"
{
    "op": "eq",
    "left": { "op": "field", "name": "category" },
    "right": { "op": "literal_string", "value": "gpu" }
}

// price >= 100 AND price < 500
{
    "op": "and",
    "left": {
        "op": "ge",
        "left": { "op": "field", "name": "price" },
        "right": { "op": "literal_int", "value": 100 }
    },
    "right": {
        "op": "lt",
        "left": { "op": "field", "name": "price" },
        "right": { "op": "literal_int", "value": 500 }
    }
}

// tags ANY ["nvidia", "gaming"]
{
    "op": "any",
    "field": { "op": "field", "name": "tags" },
    "values": {
        "op": "literal_array",
        "items": [
            { "op": "literal_string", "value": "nvidia" },
            { "op": "literal_string", "value": "gaming" }
        ]
    }
}
```

### Acceptance Criteria
- [ ] All 27 FilterExpr variants serialize to JSON
- [ ] JSON format follows FILTERING_WASM_API.md schema
- [ ] Roundtrip (serialize → deserialize) preserves equality
- [ ] Pretty printing works
- [ ] NaN/Infinity floats handled gracefully
- [ ] `cargo test filter::ast::serde` passes

### Test Cases
```rust
#[test]
fn test_serialize_simple_eq() {
    let expr = FilterExpr::Eq(
        Box::new(FilterExpr::Field("category".into())),
        Box::new(FilterExpr::LiteralString("gpu".into())),
    );

    let json = expr.to_json().unwrap();
    assert!(json.contains("\"op\":\"eq\""));
    assert!(json.contains("\"category\""));
    assert!(json.contains("\"gpu\""));
}

#[test]
fn test_serialize_complex_nested() {
    let expr = FilterExpr::And(
        Box::new(FilterExpr::Eq(
            Box::new(FilterExpr::Field("a".into())),
            Box::new(FilterExpr::LiteralInt(1)),
        )),
        Box::new(FilterExpr::Or(
            Box::new(FilterExpr::Lt(
                Box::new(FilterExpr::Field("b".into())),
                Box::new(FilterExpr::LiteralFloat(2.5)),
            )),
            Box::new(FilterExpr::Not(
                Box::new(FilterExpr::Field("c".into())),
            )),
        )),
    );

    let json = expr.to_json().unwrap();
    let reparsed = FilterExpr::from_json(&json).unwrap();
    assert_eq!(expr, reparsed);
}

#[test]
fn test_roundtrip_all_variants() {
    let test_cases = vec![
        FilterExpr::LiteralString("test".into()),
        FilterExpr::LiteralInt(42),
        FilterExpr::LiteralFloat(3.14),
        FilterExpr::LiteralBool(true),
        FilterExpr::Field("name".into()),
        FilterExpr::Eq(
            Box::new(FilterExpr::Field("x".into())),
            Box::new(FilterExpr::LiteralInt(1)),
        ),
        // ... test all 27 variants
    ];

    for expr in test_cases {
        let json = expr.to_json().unwrap();
        let reparsed = FilterExpr::from_json(&json).unwrap();
        assert_eq!(expr, reparsed, "Roundtrip failed for {:?}", expr);
    }
}

#[test]
fn test_pretty_print() {
    let expr = FilterExpr::And(
        Box::new(FilterExpr::Field("a".into())),
        Box::new(FilterExpr::Field("b".into())),
    );

    let pretty = expr.to_json_pretty().unwrap();
    assert!(pretty.contains('\n')); // Has line breaks
}
```

---

## WASM Build and Test Commands

```bash
# Build WASM package
wasm-pack build --target web --release

# Test in Node.js
wasm-pack test --node

# Test in headless browser
wasm-pack test --headless --chrome
wasm-pack test --headless --firefox

# Check bundle size
ls -la pkg/*.wasm
```

---

## Deliverables Checklist

| Artifact | Path | Status |
|:---------|:-----|:-------|
| parse_filter_js() | `src/wasm/filter.rs` | [ ] |
| search_filtered_js() | `src/wasm/filter.rs` | [ ] |
| Error serialization | `src/wasm/filter.rs` | [ ] |
| FilterExpr JSON | `src/filter/ast.rs` | [ ] |
| WASM tests | `src/wasm/filter.rs` | [ ] |

---

## End of Day 4 Gate

**Pass Criteria:**
- [ ] All 4 tasks complete
- [ ] `wasm-pack build --target web` succeeds
- [ ] `wasm-pack test --node` passes
- [ ] Bundle size < 350KB (base + filter module)
- [ ] No clippy warnings

**Handoff:**
```
[WASM_SPECIALIST]: Day 4 Complete

Artifacts generated:
- src/wasm/filter.rs (WASM bindings)
- src/filter/ast.rs (JSON serialization)
- pkg/* (built WASM package)

Status: READY_FOR_DAY_5

Next: W23.5.1 (TypeScript Filter class)
```

---

**Day 4 Total: 12 hours | 4 tasks | WASM boundary complete**
