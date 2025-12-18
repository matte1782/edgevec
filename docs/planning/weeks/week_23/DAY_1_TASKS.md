# Week 23, Day 1: Parser Foundation

**Date:** 2025-12-18
**Sprint:** Week 23 (v0.5.0 Implementation)
**Day Theme:** Filter Parser Implementation + Baseline Benchmark
**Status:** [REVISED]
**Revision:** Addresses [M1] and [M5] from HOSTILE_REVIEWER

---

## Daily Summary

| Metric | Value |
|:-------|:------|
| Tasks | 5 (was 4) |
| Total Hours | 13h (was 12h) |
| Agents | BENCHMARK_SCIENTIST + RUST_ENGINEER |
| Priority | P0 CRITICAL |

---

## Task W23.0.1: [M5] Baseline Performance Recording

**Priority:** P0 CRITICAL (Pre-requisite)
**Estimated Effort:** 1 hour
**Agent:** BENCHMARK_SCIENTIST
**Status:** PLANNED

### Objective

Record current system performance before filter implementation to enable regression detection.

### Technical Specification

**Output File:** `docs/benchmarks/week_23_baseline.md`

```markdown
# Week 23 Baseline Performance

**Date:** 2025-12-18
**Pre-Filter Version:** v0.4.x

## Search Performance (100k vectors, 128 dimensions)

| Metric | Value | Notes |
|:-------|:------|:------|
| k=10 search latency (mean) | XXX ms | |
| k=10 search latency (P99) | XXX ms | |
| k=100 search latency (mean) | XXX ms | |
| Memory usage | XXX MB | |
| WASM bundle size | XXX KB | gzipped |

## Command Log

\`\`\`bash
cargo bench --bench search_bench
wasm-pack build --target web --release
gzip -c pkg/edgevec_bg.wasm | wc -c
\`\`\`

## Notes

- No filtering subsystem present
- Baseline for Week 23 regression detection
```

### Acceptance Criteria

- [ ] Baseline benchmark recorded
- [ ] File created at `docs/benchmarks/week_23_baseline.md`
- [ ] Current WASM bundle size documented
- [ ] Ready for W23.7.1 comparison

---

## Task W23.1.1: Define FilterExpr AST Enum

**Priority:** P0 CRITICAL
**Estimated Effort:** 3 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Create the `FilterExpr` enum with all 27 AST node variants as specified in FILTER_EVALUATOR.md.

### Technical Specification

```rust
// src/filter/ast.rs

use serde::{Serialize, Deserialize};

/// Filter expression AST node.
///
/// Represents a parsed filter expression that can be evaluated
/// against vector metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    // ═══════════════════════════════════════════════════════════════
    // LITERALS (5 variants)
    // ═══════════════════════════════════════════════════════════════

    /// String literal: "hello"
    LiteralString(String),

    /// Integer literal: 42
    LiteralInt(i64),

    /// Float literal: 3.14159
    LiteralFloat(f64),

    /// Boolean literal: true, false
    LiteralBool(bool),

    /// Array literal: [1, 2, 3] or ["a", "b"]
    LiteralArray(Vec<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // FIELD REFERENCE (1 variant)
    // ═══════════════════════════════════════════════════════════════

    /// Field reference: category, price, tags
    Field(String),

    // ═══════════════════════════════════════════════════════════════
    // COMPARISON OPERATORS (6 variants)
    // ═══════════════════════════════════════════════════════════════

    /// Equality: field = value
    Eq(Box<FilterExpr>, Box<FilterExpr>),

    /// Inequality: field != value
    Ne(Box<FilterExpr>, Box<FilterExpr>),

    /// Less than: field < value
    Lt(Box<FilterExpr>, Box<FilterExpr>),

    /// Less than or equal: field <= value
    Le(Box<FilterExpr>, Box<FilterExpr>),

    /// Greater than: field > value
    Gt(Box<FilterExpr>, Box<FilterExpr>),

    /// Greater than or equal: field >= value
    Ge(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // STRING OPERATORS (4 variants)
    // ═══════════════════════════════════════════════════════════════

    /// Contains substring: field CONTAINS "value"
    Contains(Box<FilterExpr>, Box<FilterExpr>),

    /// Starts with: field STARTS_WITH "prefix"
    StartsWith(Box<FilterExpr>, Box<FilterExpr>),

    /// Ends with: field ENDS_WITH "suffix"
    EndsWith(Box<FilterExpr>, Box<FilterExpr>),

    /// LIKE pattern: field LIKE "pattern%"
    Like(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // ARRAY/SET OPERATORS (5 variants)
    // ═══════════════════════════════════════════════════════════════

    /// In set: field IN [v1, v2, v3]
    In(Box<FilterExpr>, Box<FilterExpr>),

    /// Not in set: field NOT IN [v1, v2, v3]
    NotIn(Box<FilterExpr>, Box<FilterExpr>),

    /// Any match: field ANY [v1, v2] (array field contains any)
    Any(Box<FilterExpr>, Box<FilterExpr>),

    /// All match: field ALL [v1, v2] (array field contains all)
    All(Box<FilterExpr>, Box<FilterExpr>),

    /// None match: field NONE [v1, v2] (array field contains none)
    None(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // RANGE OPERATOR (1 variant)
    // ═══════════════════════════════════════════════════════════════

    /// Between range: field BETWEEN low high
    Between(Box<FilterExpr>, Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // LOGICAL OPERATORS (3 variants)
    // ═══════════════════════════════════════════════════════════════

    /// Logical AND: expr1 AND expr2
    And(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical OR: expr1 OR expr2
    Or(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical NOT: NOT expr
    Not(Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════
    // NULL OPERATORS (2 variants)
    // ═══════════════════════════════════════════════════════════════

    /// Is null: field IS NULL
    IsNull(Box<FilterExpr>),

    /// Is not null: field IS NOT NULL
    IsNotNull(Box<FilterExpr>),
}
```

### Acceptance Criteria

- [ ] `FilterExpr` enum defined with all 27 variants
- [ ] Derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- [ ] All variants use `Box<FilterExpr>` for recursion
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` passes with no warnings

### Verification

```bash
cargo build
cargo clippy -- -D warnings
cargo test filter::ast
```

---

## Task W23.1.2: Implement Pest Grammar File

**Priority:** P0 CRITICAL
**Estimated Effort:** 4 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Create `src/filter/filter.pest` with complete EBNF grammar for filter expressions.

### Technical Specification

```pest
// src/filter/filter.pest

// ═══════════════════════════════════════════════════════════════════════════
// TOP-LEVEL RULES
// ═══════════════════════════════════════════════════════════════════════════

filter = { SOI ~ logical_expr ~ EOI }

// ═══════════════════════════════════════════════════════════════════════════
// LOGICAL EXPRESSIONS (Precedence: OR < AND < NOT)
// ═══════════════════════════════════════════════════════════════════════════

logical_expr = { or_expr }

or_expr = { and_expr ~ (or_op ~ and_expr)* }

and_expr = { not_expr ~ (and_op ~ not_expr)* }

not_expr = { not_op ~ not_expr | primary_expr }

// ═══════════════════════════════════════════════════════════════════════════
// PRIMARY EXPRESSIONS
// ═══════════════════════════════════════════════════════════════════════════

primary_expr = {
    grouped_expr
    | null_check
    | between_expr
    | array_op_expr
    | string_op_expr
    | set_op_expr
    | comparison_expr
}

grouped_expr = { "(" ~ logical_expr ~ ")" }

// ═══════════════════════════════════════════════════════════════════════════
// COMPARISON EXPRESSIONS
// ═══════════════════════════════════════════════════════════════════════════

comparison_expr = { field ~ comp_op ~ value }

comp_op = { "<=" | ">=" | "!=" | "<" | ">" | "=" }

// ═══════════════════════════════════════════════════════════════════════════
// STRING OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

string_op_expr = { field ~ string_op ~ string_literal }

string_op = {
    ^"contains"
    | ^"starts_with"
    | ^"ends_with"
    | ^"like"
}

// ═══════════════════════════════════════════════════════════════════════════
// SET OPERATIONS (IN, NOT IN)
// ═══════════════════════════════════════════════════════════════════════════

set_op_expr = { field ~ set_op ~ array_literal }

set_op = { ^"not" ~ ^"in" | ^"in" }

// ═══════════════════════════════════════════════════════════════════════════
// ARRAY OPERATIONS (ANY, ALL, NONE)
// ═══════════════════════════════════════════════════════════════════════════

array_op_expr = { field ~ array_op ~ array_literal }

array_op = { ^"any" | ^"all" | ^"none" }

// ═══════════════════════════════════════════════════════════════════════════
// RANGE OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

between_expr = { field ~ ^"between" ~ value ~ value }

// ═══════════════════════════════════════════════════════════════════════════
// NULL CHECKS
// ═══════════════════════════════════════════════════════════════════════════

null_check = { field ~ is_null_op }

is_null_op = { ^"is" ~ ^"not" ~ ^"null" | ^"is" ~ ^"null" }

// ═══════════════════════════════════════════════════════════════════════════
// LOGICAL OPERATORS
// ═══════════════════════════════════════════════════════════════════════════

or_op = _{ ^"or" | "||" }
and_op = _{ ^"and" | "&&" }
not_op = _{ ^"not" | "!" }

// ═══════════════════════════════════════════════════════════════════════════
// VALUES AND LITERALS
// ═══════════════════════════════════════════════════════════════════════════

value = { string_literal | number | boolean | field }

string_literal = @{ "\"" ~ inner_string ~ "\"" }
inner_string = @{ (!("\"" | "\\") ~ ANY | escape_seq)* }
escape_seq = @{ "\\" ~ ("\"" | "\\" | "n" | "r" | "t") }

number = @{ "-"? ~ ("0" | ASCII_NONZERO_DIGIT ~ ASCII_DIGIT*) ~ ("." ~ ASCII_DIGIT+)? }

boolean = { ^"true" | ^"false" }

array_literal = { "[" ~ (value ~ ("," ~ value)*)? ~ "]" }

field = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }

// ═══════════════════════════════════════════════════════════════════════════
// WHITESPACE (implicit)
// ═══════════════════════════════════════════════════════════════════════════

WHITESPACE = _{ " " | "\t" | "\n" | "\r" }
```

### Acceptance Criteria

- [ ] Grammar file parses all 28 example queries from FILTERING_SYNTAX.md
- [ ] Operator precedence: NOT > AND > OR
- [ ] Case-insensitive keywords (AND, and, And all work)
- [ ] String escapes handled (\", \\, \n)
- [ ] Numbers: integers and floats
- [ ] Arrays: `[1, 2, 3]` and `["a", "b"]`
- [ ] `cargo build` succeeds with pest_derive

### Verification

```bash
cargo build
# Manual test with pest playground or unit tests
```

---

## Task W23.1.3: Build AST from Pest Parse Tree

**Priority:** P0 CRITICAL
**Estimated Effort:** 3 hours
**Agent:** RUST_ENGINEER
**Status:** PLANNED

### Objective

Implement `parse()` function that converts pest parse tree to `FilterExpr` AST.

### Technical Specification

```rust
// src/filter/parser.rs

use pest::Parser;
use pest_derive::Parser;
use crate::filter::ast::FilterExpr;
use crate::filter::error::FilterError;

#[derive(Parser)]
#[grammar = "filter/filter.pest"]
pub struct FilterParser;

/// Parse a filter expression string into an AST.
///
/// # Arguments
/// * `input` - The filter expression string
///
/// # Returns
/// * `Ok(FilterExpr)` - The parsed AST
/// * `Err(FilterError)` - Parse error with position
///
/// # Examples
/// ```
/// let expr = parse("category = \"gpu\"")?;
/// let expr = parse("price < 500 AND rating >= 4.0")?;
/// ```
pub fn parse(input: &str) -> Result<FilterExpr, FilterError> {
    let pairs = FilterParser::parse(Rule::filter, input)
        .map_err(|e| FilterError::from_pest_error(e, input))?;

    build_ast(pairs)
}

fn build_ast(pairs: pest::iterators::Pairs<Rule>) -> Result<FilterExpr, FilterError> {
    // Implementation: walk parse tree and build FilterExpr
    // Handle each Rule type and construct corresponding variant
}

fn build_logical_expr(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // Handle or_expr, and_expr, not_expr
}

fn build_comparison(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // Handle comparison_expr with comp_op
}

fn build_value(pair: pest::iterators::Pair<Rule>) -> Result<FilterExpr, FilterError> {
    // Handle string_literal, number, boolean, field
}
```

### Acceptance Criteria

- [ ] `parse("category = \"gpu\"")` returns correct AST
- [ ] Nested expressions build correctly
- [ ] All 27 operator types construct correctly
- [ ] `cargo test filter::parser` passes

### Verification

```rust
#[test]
fn test_parse_simple_eq() {
    let expr = parse("category = \"gpu\"").unwrap();
    assert!(matches!(expr, FilterExpr::Eq(_, _)));
}

#[test]
fn test_parse_complex() {
    let expr = parse("price < 500 AND rating >= 4.0").unwrap();
    assert!(matches!(expr, FilterExpr::And(_, _)));
}
```

---

## Task W23.1.4: Parser Error Handling [M1 REVISED]

**Priority:** P0 CRITICAL
**Estimated Effort:** 3 hours (was 2h - adjusted per HOSTILE_REVIEWER [M1])
**Agent:** RUST_ENGINEER
**Status:** PLANNED

**[M1] Estimate Adjustment Rationale:**
- FILTER_PARSER.md specifies 9 error variants
- Each variant needs: message, position extraction, suggestion generation
- 3h more realistic for comprehensive error handling

### Objective

Implement comprehensive error handling with position information and helpful suggestions.

### Technical Specification

```rust
// src/filter/error.rs

use thiserror::Error;

/// Filter error with position information.
#[derive(Debug, Clone, Error)]
pub enum FilterError {
    #[error("Syntax error at position {position}: {message}")]
    SyntaxError {
        position: usize,
        line: usize,
        column: usize,
        message: String,
        suggestion: Option<String>,
    },

    #[error("Unexpected end of input at position {position}")]
    UnexpectedEof {
        position: usize,
        expected: String,
    },

    #[error("Invalid character '{char}' at position {position}")]
    InvalidChar {
        char: char,
        position: usize,
    },

    #[error("Unclosed string starting at position {position}")]
    UnclosedString {
        position: usize,
    },

    #[error("Unclosed parenthesis at position {position}")]
    UnclosedParen {
        position: usize,
    },

    #[error("Type mismatch: expected {expected}, got {actual} for field '{field}'")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Unknown field '{field}'")]
    UnknownField {
        field: String,
    },

    #[error("Nesting too deep (max {max_depth})")]
    NestingTooDeep {
        max_depth: usize,
    },
}

impl FilterError {
    /// Create error from pest parse error.
    pub fn from_pest_error(e: pest::error::Error<Rule>, input: &str) -> Self {
        // Extract position, convert to line/column, generate suggestion
    }

    /// Get error code for WASM serialization.
    pub fn code(&self) -> &'static str {
        match self {
            FilterError::SyntaxError { .. } => "E001",
            FilterError::UnexpectedEof { .. } => "E002",
            FilterError::InvalidChar { .. } => "E003",
            FilterError::UnclosedString { .. } => "E004",
            FilterError::UnclosedParen { .. } => "E005",
            FilterError::TypeMismatch { .. } => "E101",
            FilterError::UnknownField { .. } => "E105",
            FilterError::NestingTooDeep { .. } => "E301",
        }
    }

    /// Generate helpful suggestion based on error type.
    fn suggest(&self) -> Option<String> {
        // Context-aware suggestions
    }
}
```

### Acceptance Criteria

- [ ] Error messages include line, column, position
- [ ] Suggestions provided for common errors
- [ ] Error codes match FILTER_EVALUATOR.md catalog
- [ ] `cargo test filter::error` passes

### Verification

```rust
#[test]
fn test_error_position() {
    let err = parse("price >> 100").unwrap_err();
    assert!(matches!(err, FilterError::SyntaxError { position: 6, .. }));
}

#[test]
fn test_error_suggestion() {
    let err = parse("price >> 100").unwrap_err();
    if let FilterError::SyntaxError { suggestion, .. } = err {
        assert!(suggestion.is_some());
    }
}
```

---

## Day 1 Completion Checklist

- [ ] W23.1.1: FilterExpr AST enum complete
- [ ] W23.1.2: Pest grammar file complete
- [ ] W23.1.3: AST builder complete
- [ ] W23.1.4: Error handling complete
- [ ] All unit tests pass
- [ ] `cargo clippy` clean
- [ ] Code documented

---

## Handoff to Day 2

**Day 1 Output:**
- `src/filter/mod.rs`
- `src/filter/ast.rs`
- `src/filter/filter.pest`
- `src/filter/parser.rs`
- `src/filter/error.rs`

**Day 2 Depends On:**
- `FilterExpr` enum (W23.1.1)
- `parse()` function (W23.1.3)
- `FilterError` type (W23.1.4)

---

*"The parser is the gateway. If it fails, nothing works."*
