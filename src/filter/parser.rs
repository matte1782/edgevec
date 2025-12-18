// Allow missing docs for the generated pest Rule enum
#![allow(missing_docs)]

//! Filter expression parser for EdgeVec.
//!
//! This module provides the `parse()` function that converts filter expression
//! strings into `FilterExpr` AST nodes using a pest-based parser.
//!
//! # Grammar
//!
//! The filter grammar supports:
//! - Comparison: `=`, `!=`, `<`, `<=`, `>`, `>=`
//! - String: `CONTAINS`, `STARTS_WITH`, `ENDS_WITH`, `LIKE`
//! - Array: `IN`, `NOT IN`, `ANY`, `ALL`, `NONE`
//! - Range: `BETWEEN`
//! - Logical: `AND`, `OR`, `NOT`
//! - Null: `IS NULL`, `IS NOT NULL`
//!
//! # Example
//!
//! ```rust
//! use edgevec::filter::parse;
//!
//! // Simple comparison
//! let expr = parse("category = \"gpu\"").unwrap();
//!
//! // Complex expression
//! let expr = parse("price < 500 AND rating >= 4.0").unwrap();
//!
//! // With array operators
//! let expr = parse("tags ANY [\"rust\", \"wasm\"]").unwrap();
//! ```

use pest::Parser;
use pest_derive::Parser;

use super::ast::FilterExpr;
use super::error::{FilterError, MAX_INPUT_LENGTH, MAX_NESTING_DEPTH};

/// The pest parser for filter expressions.
///
/// This parser is generated from the grammar in `filter.pest` and
/// implements the `pest::Parser` trait for the `Rule` enum.
#[derive(Parser)]
#[grammar = "filter/filter.pest"]
#[allow(missing_docs)]
pub struct FilterParser;

/// Parse a filter expression string into an AST.
///
/// # Arguments
/// * `input` - The filter expression string
///
/// # Returns
/// * `Ok(FilterExpr)` - The parsed AST
/// * `Err(FilterError)` - Parse error with position information
///
/// # Examples
///
/// ```rust
/// use edgevec::filter::parse;
///
/// // Simple equality
/// let expr = parse("category = \"gpu\"").unwrap();
///
/// // Compound expression
/// let expr = parse("price < 500 AND rating >= 4.0").unwrap();
///
/// // With null check
/// let expr = parse("description IS NOT NULL").unwrap();
/// ```
///
/// # Errors
///
/// Returns `FilterError` for:
/// - Syntax errors (invalid tokens, unclosed strings, etc.)
/// - Input too long (exceeds `MAX_INPUT_LENGTH`)
/// - Nesting too deep (exceeds `MAX_NESTING_DEPTH`)
pub fn parse(input: &str) -> Result<FilterExpr, FilterError> {
    // Check input length
    if input.len() > MAX_INPUT_LENGTH {
        return Err(FilterError::InputTooLong {
            max_length: MAX_INPUT_LENGTH,
            actual_length: input.len(),
        });
    }

    // Parse with pest
    let pairs = FilterParser::parse(Rule::filter, input).map_err(|e| from_pest_error(&e, input))?;

    // Build AST from parse tree
    let expr = build_ast(pairs)?;

    // Check nesting depth
    let depth = expr.depth();
    if depth > MAX_NESTING_DEPTH {
        return Err(FilterError::NestingTooDeep {
            max_depth: MAX_NESTING_DEPTH,
            actual_depth: depth,
        });
    }

    Ok(expr)
}

/// Convert pest error to FilterError with position information.
fn from_pest_error(e: &pest::error::Error<Rule>, input: &str) -> FilterError {
    let (position, line, column) = match e.line_col {
        pest::error::LineColLocation::Pos((line, col))
        | pest::error::LineColLocation::Span((line, col), _) => {
            let pos = line_col_to_position(input, line, col);
            (pos, line, col)
        }
    };

    let message = format!("{}", e.variant.message());

    // Try to generate helpful suggestions
    let suggestion = generate_suggestion(&message, input, position);

    FilterError::SyntaxError {
        position,
        line,
        column,
        message,
        suggestion,
    }
}

/// Convert line/column to byte position.
fn line_col_to_position(input: &str, line: usize, col: usize) -> usize {
    let mut pos = 0;
    for (i, l) in input.lines().enumerate() {
        if i + 1 == line {
            return pos + col.saturating_sub(1);
        }
        pos += l.len() + 1; // +1 for newline
    }
    pos
}

/// Generate helpful suggestions based on error context.
fn generate_suggestion(message: &str, input: &str, position: usize) -> Option<String> {
    // Check for common typos
    if message.contains("expected") {
        // Look for common operator typos
        if position < input.len() {
            let remaining = &input[position..];
            if remaining.starts_with(':') {
                return Some("Did you mean '=' instead of ':'?".to_string());
            }
            if remaining.starts_with("==") {
                return Some("Use '=' for equality, not '=='".to_string());
            }
            if remaining.starts_with("&&") || remaining.starts_with("||") {
                return Some(
                    "Symbolic operators && and || are supported, or use AND/OR keywords"
                        .to_string(),
                );
            }
        }
    }

    None
}

/// Build AST from pest parse tree.
fn build_ast(pairs: pest::iterators::Pairs<Rule>) -> Result<FilterExpr, FilterError> {
    for pair in pairs {
        if pair.as_rule() == Rule::filter {
            // filter = { SOI ~ logical_expr ~ EOI }
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::logical_expr {
                    return build_logical_expr(inner);
                }
            }
        }
    }

    // Should not reach here if grammar is correct
    Err(FilterError::SyntaxError {
        position: 0,
        line: 1,
        column: 1,
        message: "Empty or invalid filter expression".to_string(),
        suggestion: None,
    })
}

/// Build logical expression (OR chain).
fn build_logical_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // logical_expr = { or_expr }
    // SAFETY: Grammar guarantees logical_expr always contains exactly one or_expr child.
    // The pest grammar rule `logical_expr = { or_expr }` ensures this invariant.
    let inner = pair.into_inner().next().unwrap();
    build_or_expr(inner)
}

/// Build OR expression.
fn build_or_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // or_expr = { and_expr ~ (or_op ~ and_expr)* }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees or_expr always contains at least one and_expr.
    // The rule `or_expr = { and_expr ~ (or_op ~ and_expr)* }` requires the first and_expr.
    let first = inner.next().unwrap();
    let mut left = build_and_expr(first)?;

    for next in inner {
        // or_op is silent, so we get and_expr directly
        let right = build_and_expr(next)?;
        left = FilterExpr::Or(Box::new(left), Box::new(right));
    }

    Ok(left)
}

/// Build AND expression.
fn build_and_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // and_expr = { not_expr ~ (and_op ~ not_expr)* }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees and_expr always contains at least one not_expr.
    // The rule `and_expr = { not_expr ~ (and_op ~ not_expr)* }` requires the first not_expr.
    let first = inner.next().unwrap();
    let mut left = build_not_expr(first)?;

    for next in inner {
        // and_op is silent, so we get not_expr directly
        let right = build_not_expr(next)?;
        left = FilterExpr::And(Box::new(left), Box::new(right));
    }

    Ok(left)
}

/// Build NOT expression.
fn build_not_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // not_expr = { not_op ~ not_expr | primary_expr }
    // Since not_op is silent (_), when we have "NOT x = 1", we get:
    // - not_expr which contains just not_expr (the inner recursion)
    // When we have just "x = 1", we get:
    // - not_expr which contains primary_expr

    let inner: Vec<_> = pair.into_inner().collect();

    if inner.is_empty() {
        // Defensive check: should not happen if grammar is correct
        return Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: "Empty not_expr".to_string(),
            suggestion: None,
        });
    }

    // SAFETY: We've already checked inner.is_empty() above, so this unwrap is safe.
    // The grammar rule `not_expr = { not_op ~ not_expr | primary_expr }` guarantees
    // at least one child node exists.
    let first = inner.into_iter().next().unwrap();

    match first.as_rule() {
        Rule::not_expr => {
            // This is a NOT operation: not_op (silent) followed by not_expr
            let operand = build_not_expr(first)?;
            Ok(FilterExpr::Not(Box::new(operand)))
        }
        Rule::primary_expr => {
            // This is just a primary expression (no NOT)
            build_primary_expr(first)
        }
        _ => {
            // Fallback: try to build as primary expression
            build_primary_expr(first)
        }
    }
}

/// Build primary expression.
fn build_primary_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // primary_expr = { grouped_expr | null_check | between_expr | ... }
    // SAFETY: Grammar guarantees primary_expr always contains exactly one child.
    // The rule `primary_expr = { grouped_expr | null_check | ... }` is a choice
    // expression that always matches exactly one alternative.
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::grouped_expr => build_grouped_expr(inner),
        Rule::null_check => build_null_check(inner),
        Rule::between_expr => build_between_expr(inner),
        Rule::array_op_expr => build_array_op_expr(inner),
        Rule::string_op_expr => build_string_op_expr(inner),
        Rule::set_op_expr => build_set_op_expr(inner),
        Rule::comparison_expr => build_comparison_expr(inner),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Unexpected rule: {:?}", inner.as_rule()),
            suggestion: None,
        }),
    }
}

/// Build grouped expression (parentheses).
fn build_grouped_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // grouped_expr = { "(" ~ logical_expr ~ ")" }
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::logical_expr {
            return build_logical_expr(inner);
        }
    }

    Err(FilterError::SyntaxError {
        position: 0,
        line: 1,
        column: 1,
        message: "Empty grouped expression".to_string(),
        suggestion: None,
    })
}

/// Build null check expression.
fn build_null_check(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // null_check = { field ~ is_null_op }
    let mut inner = pair.into_inner();
    // SAFETY: Grammar guarantees null_check contains field followed by is_null_op.
    // The rule `null_check = { field ~ is_null_op }` requires both children.
    let field_pair = inner.next().unwrap();
    let field_name = field_pair.as_str().to_string();
    let field_expr = FilterExpr::Field(field_name);

    // SAFETY: Second child (is_null_op) is guaranteed by grammar.
    let op_pair = inner.next().unwrap();
    // SAFETY: is_null_op = { is_not_null_op | is_null_only_op } always has one child.
    let op_inner = op_pair.into_inner().next().unwrap();

    match op_inner.as_rule() {
        Rule::is_not_null_op => Ok(FilterExpr::IsNotNull(Box::new(field_expr))),
        Rule::is_null_only_op => Ok(FilterExpr::IsNull(Box::new(field_expr))),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: "Invalid null check operator".to_string(),
            suggestion: None,
        }),
    }
}

/// Build between expression.
fn build_between_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // between_expr = { field ~ between_op ~ value ~ and_keyword ~ value }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees between_expr has exactly 5 children in order:
    // field, between_op, value, and_keyword, value
    let field_pair = inner.next().unwrap();
    let field_expr = FilterExpr::Field(field_pair.as_str().to_string());

    // Skip between_op (SAFETY: guaranteed by grammar)
    inner.next();

    // SAFETY: Third child (first value) guaranteed by grammar
    let low_pair = inner.next().unwrap();
    let low_expr = build_value(low_pair)?;

    // Skip and_keyword (SAFETY: guaranteed by grammar)
    inner.next();

    // SAFETY: Fifth child (second value) guaranteed by grammar
    let high_pair = inner.next().unwrap();
    let high_expr = build_value(high_pair)?;

    Ok(FilterExpr::Between(
        Box::new(field_expr),
        Box::new(low_expr),
        Box::new(high_expr),
    ))
}

/// Build array operation expression (ANY, ALL, NONE).
fn build_array_op_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // array_op_expr = { field ~ array_op ~ array_literal }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees array_op_expr has exactly 3 children:
    // field, array_op, array_literal
    let field_pair = inner.next().unwrap();
    let field_expr = FilterExpr::Field(field_pair.as_str().to_string());

    // SAFETY: Second child (array_op) guaranteed by grammar
    let op_pair = inner.next().unwrap();
    let op_str = op_pair.as_str().to_lowercase();

    // SAFETY: Third child (array_literal) guaranteed by grammar
    let array_pair = inner.next().unwrap();
    let array_expr = build_array_literal(array_pair)?;

    match op_str.as_str() {
        "any" => Ok(FilterExpr::Any(Box::new(field_expr), Box::new(array_expr))),
        "all" => Ok(FilterExpr::All(Box::new(field_expr), Box::new(array_expr))),
        "none" => Ok(FilterExpr::None(Box::new(field_expr), Box::new(array_expr))),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Unknown array operator: {op_str}"),
            suggestion: None,
        }),
    }
}

/// Build string operation expression.
fn build_string_op_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // string_op_expr = { field ~ string_op ~ string_literal }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees string_op_expr has exactly 3 children:
    // field, string_op, string_literal
    let field_pair = inner.next().unwrap();
    let field_expr = FilterExpr::Field(field_pair.as_str().to_string());

    // SAFETY: Second child (string_op) guaranteed by grammar
    let op_pair = inner.next().unwrap();
    let op_str = op_pair.as_str().to_lowercase();

    // SAFETY: Third child (string_literal) guaranteed by grammar
    let string_pair = inner.next().unwrap();
    let string_expr = build_string_literal(&string_pair)?;

    match op_str.as_str() {
        "contains" => Ok(FilterExpr::Contains(
            Box::new(field_expr),
            Box::new(string_expr),
        )),
        "starts_with" => Ok(FilterExpr::StartsWith(
            Box::new(field_expr),
            Box::new(string_expr),
        )),
        "ends_with" => Ok(FilterExpr::EndsWith(
            Box::new(field_expr),
            Box::new(string_expr),
        )),
        "like" => Ok(FilterExpr::Like(
            Box::new(field_expr),
            Box::new(string_expr),
        )),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Unknown string operator: {op_str}"),
            suggestion: None,
        }),
    }
}

/// Build set operation expression (IN, NOT IN).
fn build_set_op_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // set_op_expr = { field ~ set_op ~ array_literal }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees set_op_expr has exactly 3 children:
    // field, set_op, array_literal
    let field_pair = inner.next().unwrap();
    let field_expr = FilterExpr::Field(field_pair.as_str().to_string());

    // SAFETY: Second child (set_op) guaranteed by grammar
    let op_pair = inner.next().unwrap();
    // SAFETY: set_op = { not_in_op | in_op } always has exactly one child
    let op_inner = op_pair.into_inner().next().unwrap();
    let is_not_in = op_inner.as_rule() == Rule::not_in_op;

    // SAFETY: Third child (array_literal) guaranteed by grammar
    let array_pair = inner.next().unwrap();
    let array_expr = build_array_literal(array_pair)?;

    if is_not_in {
        Ok(FilterExpr::NotIn(
            Box::new(field_expr),
            Box::new(array_expr),
        ))
    } else {
        Ok(FilterExpr::In(Box::new(field_expr), Box::new(array_expr)))
    }
}

/// Build comparison expression.
fn build_comparison_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // comparison_expr = { field ~ comp_op ~ value }
    let mut inner = pair.into_inner();

    // SAFETY: Grammar guarantees comparison_expr has exactly 3 children:
    // field, comp_op, value
    let field_pair = inner.next().unwrap();
    let field_expr = FilterExpr::Field(field_pair.as_str().to_string());

    // SAFETY: Second child (comp_op) guaranteed by grammar
    let op_pair = inner.next().unwrap();
    let op_str = op_pair.as_str();

    // SAFETY: Third child (value) guaranteed by grammar
    let value_pair = inner.next().unwrap();
    let value_expr = build_value(value_pair)?;

    match op_str {
        "=" => Ok(FilterExpr::Eq(Box::new(field_expr), Box::new(value_expr))),
        "!=" => Ok(FilterExpr::Ne(Box::new(field_expr), Box::new(value_expr))),
        "<" => Ok(FilterExpr::Lt(Box::new(field_expr), Box::new(value_expr))),
        "<=" => Ok(FilterExpr::Le(Box::new(field_expr), Box::new(value_expr))),
        ">" => Ok(FilterExpr::Gt(Box::new(field_expr), Box::new(value_expr))),
        ">=" => Ok(FilterExpr::Ge(Box::new(field_expr), Box::new(value_expr))),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Unknown comparison operator: {op_str}"),
            suggestion: None,
        }),
    }
}

/// Build value expression.
fn build_value(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // value = { string_literal | number | boolean | field }
    // SAFETY: Grammar guarantees value always contains exactly one child.
    // The rule `value = { string_literal | number | boolean | field }` is a choice
    // expression that always matches exactly one alternative.
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::string_literal => build_string_literal(&inner),
        Rule::number => build_number(&inner),
        Rule::boolean => build_boolean(&inner),
        Rule::field => Ok(FilterExpr::Field(inner.as_str().to_string())),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Unexpected value type: {:?}", inner.as_rule()),
            suggestion: None,
        }),
    }
}

/// Build string literal.
fn build_string_literal(pair: &pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // string_literal = @{ "\"" ~ inner_string ~ "\"" }
    let raw = pair.as_str();
    // Remove quotes
    let content = &raw[1..raw.len() - 1];
    // Process escape sequences
    let processed = process_escapes(content)?;
    Ok(FilterExpr::LiteralString(processed))
}

/// Process escape sequences in string.
fn process_escapes(s: &str) -> Result<String, FilterError> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some(other) => {
                    return Err(FilterError::InvalidEscape {
                        char: other,
                        position: 0, // Position not easily available here
                    });
                }
                None => {
                    return Err(FilterError::SyntaxError {
                        position: 0,
                        line: 1,
                        column: 1,
                        message: "Trailing backslash in string".to_string(),
                        suggestion: Some("Escape the backslash with \\\\".to_string()),
                    });
                }
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// Build number literal.
fn build_number(pair: &pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    let s = pair.as_str();

    // Try parsing as integer first
    if !s.contains('.') {
        if let Ok(i) = s.parse::<i64>() {
            return Ok(FilterExpr::LiteralInt(i));
        }
    }

    // Parse as float
    if let Ok(f) = s.parse::<f64>() {
        return Ok(FilterExpr::LiteralFloat(f));
    }

    Err(FilterError::InvalidNumber {
        value: s.to_string(),
        position: 0,
    })
}

/// Build boolean literal.
fn build_boolean(pair: &pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    let s = pair.as_str().to_lowercase();
    match s.as_str() {
        "true" => Ok(FilterExpr::LiteralBool(true)),
        "false" => Ok(FilterExpr::LiteralBool(false)),
        _ => Err(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: format!("Invalid boolean: {s}"),
            suggestion: None,
        }),
    }
}

/// Build array literal.
fn build_array_literal(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // array_literal = { "[" ~ (value ~ ("," ~ value)*)? ~ "]" }
    let mut elements = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::value {
            elements.push(build_value(inner)?);
        }
    }

    Ok(FilterExpr::LiteralArray(elements))
}

#[cfg(test)]
#[allow(clippy::redundant_closure_for_method_calls)] // Test assertions use explicit closures for clarity
mod tests {
    use super::*;

    // =========================================================================
    // BASIC PARSING TESTS
    // =========================================================================

    #[test]
    fn test_parse_simple_eq() {
        let expr = parse("category = \"gpu\"").unwrap();
        assert!(matches!(expr, FilterExpr::Eq(_, _)));
        if let FilterExpr::Eq(left, right) = expr {
            assert_eq!(left.as_field(), Some("category"));
            assert_eq!(right.as_string(), Some("gpu"));
        }
    }

    #[test]
    fn test_parse_simple_ne() {
        let expr = parse("status != \"deleted\"").unwrap();
        assert!(matches!(expr, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_parse_simple_lt() {
        let expr = parse("price < 500").unwrap();
        assert!(matches!(expr, FilterExpr::Lt(_, _)));
        if let FilterExpr::Lt(left, right) = expr {
            assert_eq!(left.as_field(), Some("price"));
            assert_eq!(right.as_int(), Some(500));
        }
    }

    #[test]
    fn test_parse_simple_le() {
        let expr = parse("rating <= 4.5").unwrap();
        assert!(matches!(expr, FilterExpr::Le(_, _)));
    }

    #[test]
    fn test_parse_simple_gt() {
        let expr = parse("count > 0").unwrap();
        assert!(matches!(expr, FilterExpr::Gt(_, _)));
    }

    #[test]
    fn test_parse_simple_ge() {
        let expr = parse("score >= 90").unwrap();
        assert!(matches!(expr, FilterExpr::Ge(_, _)));
    }

    // =========================================================================
    // LOGICAL OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_parse_and() {
        let expr = parse("a = 1 AND b = 2").unwrap();
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_or() {
        let expr = parse("a = 1 OR b = 2").unwrap();
        assert!(matches!(expr, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_not() {
        let expr = parse("NOT active = true").unwrap();
        assert!(matches!(expr, FilterExpr::Not(_)));
    }

    #[test]
    fn test_parse_symbolic_and() {
        let expr = parse("a = 1 && b = 2").unwrap();
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_symbolic_or() {
        let expr = parse("a = 1 || b = 2").unwrap();
        assert!(matches!(expr, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_symbolic_not() {
        let expr = parse("!active = true").unwrap();
        assert!(matches!(expr, FilterExpr::Not(_)));
    }

    // =========================================================================
    // PRECEDENCE TESTS
    // =========================================================================

    #[test]
    fn test_precedence_and_binds_tighter_than_or() {
        // a OR b AND c should parse as a OR (b AND c)
        let expr = parse("a = 1 OR b = 2 AND c = 3").unwrap();
        if let FilterExpr::Or(left, right) = expr {
            assert!(left.as_field().is_none()); // left is Eq, not field
            assert!(matches!(*right, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Or at top level");
        }
    }

    #[test]
    fn test_precedence_parentheses() {
        // (a OR b) AND c
        let expr = parse("(a = 1 OR b = 2) AND c = 3").unwrap();
        assert!(matches!(expr, FilterExpr::And(_, _)));
        if let FilterExpr::And(left, _) = expr {
            assert!(matches!(*left, FilterExpr::Or(_, _)));
        }
    }

    // =========================================================================
    // STRING OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_parse_contains() {
        let expr = parse("description CONTAINS \"fast\"").unwrap();
        assert!(matches!(expr, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_parse_starts_with() {
        let expr = parse("name STARTS_WITH \"GPU\"").unwrap();
        assert!(matches!(expr, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_parse_ends_with() {
        let expr = parse("filename ENDS_WITH \".pdf\"").unwrap();
        assert!(matches!(expr, FilterExpr::EndsWith(_, _)));
    }

    #[test]
    fn test_parse_like() {
        let expr = parse("name LIKE \"GPU_%\"").unwrap();
        assert!(matches!(expr, FilterExpr::Like(_, _)));
    }

    // =========================================================================
    // SET OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_parse_in() {
        let expr = parse("category IN [\"gpu\", \"cpu\"]").unwrap();
        assert!(matches!(expr, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_parse_not_in() {
        let expr = parse("status NOT IN [\"deleted\", \"archived\"]").unwrap();
        assert!(matches!(expr, FilterExpr::NotIn(_, _)));
    }

    // =========================================================================
    // ARRAY OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_parse_any() {
        let expr = parse("tags ANY [\"rust\", \"wasm\"]").unwrap();
        assert!(matches!(expr, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_parse_all() {
        let expr = parse("tags ALL [\"rust\", \"wasm\"]").unwrap();
        assert!(matches!(expr, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_parse_none() {
        let expr = parse("tags NONE [\"deprecated\"]").unwrap();
        assert!(matches!(expr, FilterExpr::None(_, _)));
    }

    // =========================================================================
    // RANGE OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_parse_between() {
        let expr = parse("price BETWEEN 100 AND 500").unwrap();
        assert!(matches!(expr, FilterExpr::Between(_, _, _)));
        if let FilterExpr::Between(field, low, high) = expr {
            assert_eq!(field.as_field(), Some("price"));
            assert_eq!(low.as_int(), Some(100));
            assert_eq!(high.as_int(), Some(500));
        }
    }

    // =========================================================================
    // NULL CHECK TESTS
    // =========================================================================

    #[test]
    fn test_parse_is_null() {
        let expr = parse("description IS NULL").unwrap();
        assert!(matches!(expr, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_parse_is_not_null() {
        let expr = parse("description IS NOT NULL").unwrap();
        assert!(matches!(expr, FilterExpr::IsNotNull(_)));
    }

    // =========================================================================
    // CASE INSENSITIVITY TESTS
    // =========================================================================

    #[test]
    fn test_case_insensitive_and() {
        assert!(parse("a = 1 and b = 2").is_ok());
        assert!(parse("a = 1 AND b = 2").is_ok());
        assert!(parse("a = 1 And b = 2").is_ok());
    }

    #[test]
    fn test_case_insensitive_or() {
        assert!(parse("a = 1 or b = 2").is_ok());
        assert!(parse("a = 1 OR b = 2").is_ok());
    }

    #[test]
    fn test_case_insensitive_contains() {
        assert!(parse("name contains \"test\"").is_ok());
        assert!(parse("name CONTAINS \"test\"").is_ok());
    }

    #[test]
    fn test_case_insensitive_boolean() {
        assert!(parse("active = TRUE").is_ok());
        assert!(parse("active = true").is_ok());
        assert!(parse("active = True").is_ok());
    }

    // =========================================================================
    // LITERAL TESTS
    // =========================================================================

    #[test]
    fn test_parse_integer() {
        let expr = parse("count = 42").unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_int(), Some(42));
        }
    }

    #[test]
    fn test_parse_negative_integer() {
        let expr = parse("temp = -10").unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_int(), Some(-10));
        }
    }

    #[test]
    fn test_parse_float() {
        let expr = parse("rating = 4.5").unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_float(), Some(4.5));
        }
    }

    #[test]
    fn test_parse_negative_float() {
        let expr = parse("temp = -2.5").unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_float(), Some(-2.5));
        }
    }

    #[test]
    fn test_parse_string_escapes() {
        let expr = parse(r#"msg = "hello \"world\"""#).unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_string(), Some("hello \"world\""));
        }
    }

    #[test]
    fn test_parse_string_escape_newline() {
        let expr = parse(r#"msg = "line1\nline2""#).unwrap();
        if let FilterExpr::Eq(_, right) = expr {
            assert_eq!(right.as_string(), Some("line1\nline2"));
        }
    }

    #[test]
    fn test_parse_empty_array() {
        let expr = parse("tags IN []").unwrap();
        if let FilterExpr::In(_, right) = expr {
            assert_eq!(right.as_array().map(|a| a.len()), Some(0));
        }
    }

    // =========================================================================
    // COMPLEX EXPRESSION TESTS
    // =========================================================================

    #[test]
    fn test_parse_complex_expression() {
        let input = "category = \"gpu\" AND (price < 500 OR rating >= 4.5)";
        let expr = parse(input).unwrap();
        assert!(matches!(expr, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parse_deeply_nested() {
        let input = "((a = 1) AND (b = 2)) OR ((c = 3) AND (d = 4))";
        let expr = parse(input).unwrap();
        assert!(matches!(expr, FilterExpr::Or(_, _)));
    }

    // =========================================================================
    // ERROR TESTS
    // =========================================================================

    #[test]
    fn test_parse_error_empty() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_string() {
        let result = parse("name = \"unclosed");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unclosed_paren() {
        let result = parse("(a = 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_invalid_operator() {
        let result = parse("a == 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_input_too_long() {
        let long_input = "a".repeat(MAX_INPUT_LENGTH + 1);
        let result = parse(&long_input);
        assert!(matches!(result, Err(FilterError::InputTooLong { .. })));
    }

    // =========================================================================
    // WHITESPACE TESTS
    // =========================================================================

    #[test]
    fn test_whitespace_handling() {
        assert!(parse("  a  =  1  ").is_ok());
        assert!(parse("a=1").is_ok());
        assert!(parse("a = 1 AND b = 2").is_ok());
        // Note: a=1AND b=2 actually parses correctly because:
        // - number parsing stops at 'A' (non-digit)
        // - AND is recognized as a keyword
        // This is valid in the grammar (though not recommended style)
        assert!(parse("a=1AND b=2").is_ok());
    }

    #[test]
    fn test_newline_handling() {
        let input = "a = 1\nAND\nb = 2";
        assert!(parse(input).is_ok());
    }
}
