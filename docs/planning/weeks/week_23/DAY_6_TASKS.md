# Week 23 Day 6: Testing Sprint

**Date:** Day 6 of Week 23
**Focus:** Comprehensive Testing - Unit, Property, and Fuzz
**Agent:** TEST_ENGINEER
**Total Hours:** 18h (was 14h - addresses [C4], [C5], [M3])
**Status:** [REVISED]
**Revision:** Addresses HOSTILE_REVIEWER issues [C4], [C5], [M3]

---

## Executive Summary

Day 6 is a dedicated testing sprint implementing the test strategy defined in FILTER_TEST_STRATEGY.md. This includes 1856+ unit tests, 17 property test invariants, and 6 fuzz targets (split per [C4]).

**Prerequisites:**
- W23.1-W23.5 (all implementation) COMPLETE
- `cargo build --release` succeeds
- `wasm-pack build --target web` succeeds

**Key Deliverables:**
- 1,856+ unit tests (verified per [M3])
- 17 property test invariants
- 6 fuzz targets: 5 original + 1 deep nesting ([C4])
- Multi-field filter integration tests ([C5])
- >90% code coverage

---

## Tasks Overview

| Task ID | Description | Hours | Priority |
|:--------|:------------|:------|:---------|
| W23.6.1 | Parser unit tests (344 tests) | 3h | P0 |
| W23.6.2 | Evaluator unit tests (804 tests) | 4h | P0 |
| W23.6.3 | Strategy unit tests (408 tests) | 2h | P0 |
| W23.6.4 | Property tests (17 invariants) | 2h | P0 |
| W23.6.5a | **[C4]** Fuzz target - simple expressions | 2h | P0 |
| W23.6.5b | **[C4]** Fuzz target - deeply nested (max depth 50) | 2h | P0 |
| W23.6.6 | Integration tests (HNSW + filter) | 1h | P0 |
| W23.6.7 | **[C5]** Multi-field filter integration tests | 1h | P0 |
| W23.6.8 | **[M3]** Test count verification (≥1856) | 1h | P0 |

---

## Task W23.6.1: Parser Unit Tests (344 tests)

### Description
Implement comprehensive unit tests for the filter parser.

### Hours: 3h

### Test Categories

| Category | Count | Description |
|:---------|:------|:------------|
| Literal parsing | 32 | string, int, float, bool |
| Operator parsing | 48 | =, !=, <, >, <=, >= |
| String operators | 24 | CONTAINS, LIKE, etc. |
| Array operators | 32 | IN, ANY, ALL, NONE |
| Logical operators | 24 | AND, OR, NOT |
| NULL checks | 12 | IS NULL, IS NOT NULL |
| Precedence | 36 | Operator precedence |
| Parentheses | 24 | Grouping |
| Error cases | 64 | Invalid syntax |
| Edge cases | 48 | Unicode, escapes, limits |
| **Total** | **344** | |

### Specification

**File:** `tests/filter_parser_tests.rs`

```rust
//! Parser unit tests for EdgeVec filter system.
//!
//! Tests organized by category as specified in FILTER_TEST_STRATEGY.md.

use edgevec::filter::{parse, FilterExpr, FilterError};

mod literal_parsing {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // STRING LITERALS (8 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_simple_string() {
        let result = parse(r#"x = "hello""#).unwrap();
        // Verify AST structure
    }

    #[test]
    fn test_empty_string() {
        let result = parse(r#"x = """#).unwrap();
    }

    #[test]
    fn test_string_with_spaces() {
        let result = parse(r#"x = "hello world""#).unwrap();
    }

    #[test]
    fn test_string_with_unicode() {
        let result = parse(r#"x = "日本語""#).unwrap();
    }

    #[test]
    fn test_string_with_emoji() {
        let result = parse(r#"x = "🚀 rocket""#).unwrap();
    }

    #[test]
    fn test_string_escaped_quote() {
        let result = parse(r#"x = "say \"hello\"""#).unwrap();
    }

    #[test]
    fn test_string_escaped_backslash() {
        let result = parse(r#"x = "path\\to\\file""#).unwrap();
    }

    #[test]
    fn test_string_newline_escape() {
        let result = parse(r#"x = "line1\nline2""#).unwrap();
    }

    // ═══════════════════════════════════════════════════════════════════════
    // INTEGER LITERALS (8 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_positive_int() {
        let result = parse("x = 42").unwrap();
    }

    #[test]
    fn test_zero() {
        let result = parse("x = 0").unwrap();
    }

    #[test]
    fn test_negative_int() {
        let result = parse("x = -42").unwrap();
    }

    #[test]
    fn test_large_int() {
        let result = parse("x = 9223372036854775807").unwrap();
    }

    #[test]
    fn test_negative_large_int() {
        let result = parse("x = -9223372036854775808").unwrap();
    }

    #[test]
    fn test_int_overflow() {
        let result = parse("x = 99999999999999999999999");
        assert!(matches!(result, Err(FilterError::Overflow { .. })));
    }

    #[test]
    fn test_leading_zeros() {
        // Leading zeros should be valid (lexer normalizes)
        let result = parse("x = 007");
        assert!(result.is_ok());
    }

    #[test]
    fn test_plus_sign() {
        let result = parse("x = +42");
        // Could be valid or error depending on grammar
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FLOAT LITERALS (8 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_simple_float() {
        let result = parse("x = 3.14").unwrap();
    }

    #[test]
    fn test_negative_float() {
        let result = parse("x = -3.14").unwrap();
    }

    #[test]
    fn test_float_no_leading_digit() {
        let result = parse("x = .5");
        // May be valid or invalid
    }

    #[test]
    fn test_float_no_trailing_digit() {
        let result = parse("x = 5.");
        // May be valid or invalid
    }

    #[test]
    fn test_float_exponent() {
        let result = parse("x = 1.5e10").unwrap();
    }

    #[test]
    fn test_float_negative_exponent() {
        let result = parse("x = 1.5e-10").unwrap();
    }

    #[test]
    fn test_float_infinity() {
        // Should probably error
        let result = parse("x = inf");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_nan() {
        // Should probably error
        let result = parse("x = NaN");
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // BOOLEAN LITERALS (8 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_true_lowercase() {
        let result = parse("x = true").unwrap();
    }

    #[test]
    fn test_false_lowercase() {
        let result = parse("x = false").unwrap();
    }

    #[test]
    fn test_true_uppercase() {
        let result = parse("x = TRUE").unwrap();
    }

    #[test]
    fn test_false_uppercase() {
        let result = parse("x = FALSE").unwrap();
    }

    #[test]
    fn test_true_mixed_case() {
        let result = parse("x = True").unwrap();
    }

    #[test]
    fn test_false_mixed_case() {
        let result = parse("x = False").unwrap();
    }

    #[test]
    fn test_bool_as_string() {
        // "true" (with quotes) should be string, not bool
        let result = parse(r#"x = "true""#).unwrap();
        // Verify it's a string literal, not bool
    }

    #[test]
    fn test_bool_1_0() {
        // 1 and 0 should be integers, not bools
        let result = parse("x = 1").unwrap();
        // Verify it's an int literal, not bool
    }
}

mod operator_parsing {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // COMPARISON OPERATORS (48 tests - 8 per operator)
    // ═══════════════════════════════════════════════════════════════════════

    // ... test each operator with various value types ...
}

mod string_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // STRING OPERATORS (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_contains_simple() {
        let result = parse(r#"name CONTAINS "test""#).unwrap();
    }

    #[test]
    fn test_contains_case_insensitive_keyword() {
        let result = parse(r#"name contains "test""#).unwrap();
    }

    #[test]
    fn test_starts_with() {
        let result = parse(r#"name STARTS_WITH "pre""#).unwrap();
    }

    #[test]
    fn test_ends_with() {
        let result = parse(r#"name ENDS_WITH "suf""#).unwrap();
    }

    #[test]
    fn test_like_simple() {
        let result = parse(r#"email LIKE "%@%.com""#).unwrap();
    }

    #[test]
    fn test_like_underscore() {
        let result = parse(r#"code LIKE "A_B""#).unwrap();
    }

    // ... more string operator tests ...
}

mod array_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // ARRAY OPERATORS (32 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_in_strings() {
        let result = parse(r#"category IN ["a", "b", "c"]"#).unwrap();
    }

    #[test]
    fn test_in_numbers() {
        let result = parse("x IN [1, 2, 3]").unwrap();
    }

    #[test]
    fn test_in_empty_array() {
        let result = parse("x IN []");
        assert!(matches!(result, Err(FilterError::EmptyArray { .. })));
    }

    #[test]
    fn test_not_in() {
        let result = parse(r#"status NOT IN ["deleted", "archived"]"#).unwrap();
    }

    #[test]
    fn test_any_scalar() {
        let result = parse(r#"ANY(tags, "nvidia")"#).unwrap();
    }

    #[test]
    fn test_all_array() {
        let result = parse(r#"ALL(tags, ["a", "b"])"#).unwrap();
    }

    #[test]
    fn test_none_array() {
        let result = parse(r#"NONE(tags, ["spam", "nsfw"])"#).unwrap();
    }

    // ... more array operator tests ...
}

mod logical_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // LOGICAL OPERATORS (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_and_simple() {
        let result = parse("a = 1 AND b = 2").unwrap();
    }

    #[test]
    fn test_or_simple() {
        let result = parse("a = 1 OR b = 2").unwrap();
    }

    #[test]
    fn test_not_simple() {
        let result = parse("NOT a = 1").unwrap();
    }

    #[test]
    fn test_and_chain() {
        let result = parse("a = 1 AND b = 2 AND c = 3").unwrap();
    }

    #[test]
    fn test_or_chain() {
        let result = parse("a = 1 OR b = 2 OR c = 3").unwrap();
    }

    #[test]
    fn test_mixed_and_or() {
        let result = parse("a = 1 AND b = 2 OR c = 3").unwrap();
        // Verify precedence: AND binds tighter than OR
    }

    #[test]
    fn test_double_not() {
        let result = parse("NOT NOT a = 1").unwrap();
    }

    // ... more logical operator tests ...
}

mod precedence {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // PRECEDENCE (36 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_and_before_or() {
        // a AND b OR c should parse as (a AND b) OR c
        let result = parse("a = 1 AND b = 2 OR c = 3").unwrap();
        // Verify structure
    }

    #[test]
    fn test_not_highest() {
        // NOT a AND b should parse as (NOT a) AND b
        let result = parse("NOT a = 1 AND b = 2").unwrap();
    }

    #[test]
    fn test_comparison_before_logical() {
        // a = 1 AND b = 2 should parse comparisons first
        let result = parse("a = 1 AND b = 2").unwrap();
    }

    // ... more precedence tests ...
}

mod parentheses {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // PARENTHESES (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_simple_parens() {
        let result = parse("(a = 1)").unwrap();
    }

    #[test]
    fn test_nested_parens() {
        let result = parse("((a = 1))").unwrap();
    }

    #[test]
    fn test_parens_override_precedence() {
        // a AND (b OR c) should keep OR grouped
        let result = parse("a = 1 AND (b = 2 OR c = 3)").unwrap();
    }

    #[test]
    fn test_unclosed_paren() {
        let result = parse("(a = 1");
        assert!(matches!(result, Err(FilterError::UnclosedParen { .. })));
    }

    #[test]
    fn test_extra_close_paren() {
        let result = parse("a = 1)");
        assert!(result.is_err());
    }

    // ... more parentheses tests ...
}

mod error_cases {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // ERROR CASES (64 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_empty_input() {
        let result = parse("");
        assert!(matches!(result, Err(FilterError::UnexpectedEof { .. })));
    }

    #[test]
    fn test_only_whitespace() {
        let result = parse("   ");
        assert!(matches!(result, Err(FilterError::UnexpectedEof { .. })));
    }

    #[test]
    fn test_missing_value() {
        let result = parse("x =");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_field() {
        let result = parse("= 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_operator() {
        let result = parse("x >> 1");
        assert!(matches!(result, Err(FilterError::SyntaxError { .. })));
    }

    #[test]
    fn test_unclosed_string() {
        let result = parse(r#"x = "hello"#);
        assert!(matches!(result, Err(FilterError::UnclosedString { .. })));
    }

    #[test]
    fn test_invalid_escape() {
        let result = parse(r#"x = "hello\z""#);
        assert!(matches!(result, Err(FilterError::InvalidEscape { .. })));
    }

    // ... more error cases ...
}

mod edge_cases {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // EDGE CASES (48 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_field_name_underscore() {
        let result = parse("_field = 1").unwrap();
    }

    #[test]
    fn test_field_name_numbers() {
        let result = parse("field123 = 1").unwrap();
    }

    #[test]
    fn test_field_name_reserved_word() {
        // "and" as field name should work if quoted or escaped
        let result = parse(r#""and" = 1"#);
        // Or might not be supported
    }

    #[test]
    fn test_whitespace_variations() {
        let a = parse("x=1").unwrap();
        let b = parse("x = 1").unwrap();
        let c = parse("x  =  1").unwrap();
        let d = parse("  x = 1  ").unwrap();
        // All should produce equivalent AST
    }

    #[test]
    fn test_very_long_string() {
        let long_string = "a".repeat(10000);
        let result = parse(&format!(r#"x = "{}""#, long_string));
        // Should work or error gracefully
    }

    #[test]
    fn test_deeply_nested() {
        let mut expr = "x = 1".to_string();
        for _ in 0..10 {
            expr = format!("({})", expr);
        }
        let result = parse(&expr);
        // Should work or error with TooDeep
    }

    // ... more edge cases ...
}
```

### Acceptance Criteria
- [ ] All 344 tests pass
- [ ] Each test category complete
- [ ] Error messages verified
- [ ] Position info verified for errors
- [ ] `cargo test filter::parser` passes

---

## Task W23.6.2: Evaluator Unit Tests (804 tests)

### Description
Implement comprehensive unit tests for the filter evaluator.

### Hours: 4h

### Test Categories

| Category | Count | Description |
|:---------|:------|:------------|
| Comparison (6 ops × 5 types × 6 cases) | 180 | =, !=, <, <=, >, >= |
| String operators | 96 | CONTAINS, LIKE, etc. |
| Array operators | 120 | IN, ANY, ALL, NONE |
| Logical operators | 72 | AND, OR, NOT |
| BETWEEN | 48 | Range checks |
| NULL checks | 36 | IS NULL, IS NOT NULL |
| Type coercion | 48 | int/float promotion |
| Error conditions | 96 | Type errors, etc. |
| Short-circuit | 36 | AND/OR optimization |
| Edge cases | 72 | Empty, unicode, limits |
| **Total** | **804** | |

### Specification

**File:** `tests/filter_evaluator_tests.rs`

```rust
//! Evaluator unit tests for EdgeVec filter system.

use edgevec::filter::{parse, evaluate, FilterExpr, FilterError, MetadataValue};
use std::collections::HashMap;

fn make_metadata(pairs: &[(&str, MetadataValue)]) -> HashMap<String, MetadataValue> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

mod comparison_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // EQUALITY (30 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_eq_string_match() {
        let filter = parse(r#"name = "alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_string_no_match() {
        let filter = parse(r#"name = "alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("bob".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_int_match() {
        let filter = parse("count = 42").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_int_no_match() {
        let filter = parse("count = 42").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(43))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_float_match() {
        let filter = parse("price = 3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_bool_match() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Bool(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_missing_field() {
        let filter = parse("name = \"alice\"").unwrap();
        let meta = make_metadata(&[]);
        // Missing field should return false, not error (lenient mode)
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_type_mismatch_string_int() {
        let filter = parse("name = 42").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        // Type mismatch should return false (lenient mode)
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // NOT EQUAL (30 tests - similar structure)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_ne_string() {
        let filter = parse(r#"name != "alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("bob".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ... more ne tests ...

    // ═══════════════════════════════════════════════════════════════════════
    // LESS THAN (30 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lt_int_true() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_int_false_equal() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_int_false_greater() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_float_true() {
        let filter = parse("x < 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_int_float_promotion() {
        // Compare int field with float literal
        let filter = parse("x < 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ... more comparison tests for <=, >, >= ...
}

mod string_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // CONTAINS (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_contains_match() {
        let filter = parse(r#"title CONTAINS "hello""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("say hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_no_match() {
        let filter = parse(r#"title CONTAINS "goodbye""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("say hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_case_sensitive() {
        let filter = parse(r#"title CONTAINS "Hello""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("say hello world".into()))]);
        // Default is case-sensitive, so this should be false
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_empty_substring() {
        let filter = parse(r#"title CONTAINS """#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("anything".into()))]);
        // Empty string is contained in everything
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // LIKE PATTERN (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_like_percent_prefix() {
        let filter = parse(r#"email LIKE "%@gmail.com""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("user@gmail.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_percent_suffix() {
        let filter = parse(r#"name LIKE "John%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("John Smith".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_underscore() {
        let filter = parse(r#"code LIKE "A_B""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("AXB".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_no_wildcards() {
        let filter = parse(r#"name LIKE "John""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("John".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_escaped_percent() {
        // Escaping % to match literal percent
        let filter = parse(r#"text LIKE "50\% off""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("50% off".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ... more LIKE tests, STARTS_WITH, ENDS_WITH ...
}

mod array_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // IN OPERATOR (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_in_string_match() {
        let filter = parse(r#"category IN ["gpu", "cpu", "ram"]"#).unwrap();
        let meta = make_metadata(&[("category", MetadataValue::String("gpu".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_string_no_match() {
        let filter = parse(r#"category IN ["gpu", "cpu", "ram"]"#).unwrap();
        let meta = make_metadata(&[("category", MetadataValue::String("ssd".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_in_int_match() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ANY OPERATOR (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_any_match() {
        let filter = parse(r#"ANY(tags, "nvidia")"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["nvidia".into(), "gaming".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_no_match() {
        let filter = parse(r#"ANY(tags, "amd")"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["nvidia".into(), "gaming".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_empty_array() {
        let filter = parse(r#"ANY(tags, "nvidia")"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ALL OPERATOR (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_all_match() {
        let filter = parse(r#"ALL(tags, ["gpu", "gaming"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["gpu".into(), "gaming".into(), "nvidia".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_partial_match() {
        let filter = parse(r#"ALL(tags, ["gpu", "amd"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["gpu".into(), "nvidia".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // NONE OPERATOR (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_none_match() {
        let filter = parse(r#"NONE(tags, ["spam", "nsfw"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["gpu".into(), "nvidia".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_has_one() {
        let filter = parse(r#"NONE(tags, ["spam", "nsfw"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["gpu".into(), "spam".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

mod logical_operators {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // AND (24 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_and_both_true() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_first_false() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_and_second_false() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_and_both_false() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // ... OR, NOT tests ...
}

mod short_circuit {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // SHORT-CIRCUIT BEHAVIOR (36 tests)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_and_short_circuit_false() {
        // If first is false, second should not be evaluated
        let filter = parse("false_field = true AND error_field = 1").unwrap();
        let meta = make_metadata(&[("false_field", MetadataValue::Bool(false))]);
        // Should return false without erroring on missing error_field
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_short_circuit_true() {
        // If first is true, second should not be evaluated
        let filter = parse("true_field = true OR error_field = 1").unwrap();
        let meta = make_metadata(&[("true_field", MetadataValue::Bool(true))]);
        // Should return true without evaluating second
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// ... more test modules for BETWEEN, NULL checks, type coercion, errors, edge cases ...
```

### Acceptance Criteria
- [ ] All 804 tests pass
- [ ] Each operator fully tested
- [ ] Type coercion behavior verified
- [ ] Short-circuit behavior verified
- [ ] `cargo test filter::evaluator` passes

---

## Task W23.6.3: Strategy Unit Tests (408 tests)

### Description
Implement unit tests for filter strategy selection and execution.

### Hours: 2h

### Test Categories

| Category | Count | Description |
|:---------|:------|:------------|
| PostFilter | 48 | Various oversample factors |
| PreFilter | 48 | Various selectivity levels |
| Hybrid | 72 | Selectivity ranges |
| Auto-selection | 96 | All scenarios |
| Selectivity estimation | 36 | Sampling accuracy |
| Edge cases | 48 | Empty, tautology, contradiction |
| EF cap | 24 | Limit enforcement |
| Result completeness | 36 | k vs actual |
| **Total** | **408** | |

**File:** `tests/filter_strategy_tests.rs`

### Acceptance Criteria
- [ ] All 408 tests pass
- [ ] Each strategy fully tested
- [ ] Edge cases covered
- [ ] `cargo test filter::strategy` passes

---

## Task W23.6.4: Property Tests (17 invariants)

### Description
Implement the 17 property test invariants specified in FILTER_TEST_STRATEGY.md.

### Hours: 2h

### Invariants

**Parser (P1-P5):**
- P1: Parse-serialize roundtrip
- P2: Parser never panics
- P3: Empty input returns error
- P4: Whitespace normalization
- P5: Keyword case insensitivity

**Evaluator (E1-E7):**
- E1: NOT involution (NOT NOT x == x)
- E2: AND commutativity
- E3: OR commutativity
- E4: De Morgan's laws
- E5: Short-circuit correctness
- E6: Type consistency
- E7: NULL semantics (XOR)

**Strategy (S1-S3):**
- S1: Strategy result equivalence
- S2: Auto selection stability
- S3: Oversample bounds

**WASM (W1-W2):**
- W1: JSON roundtrip
- W2: TypeScript type safety

**File:** `tests/filter_property_tests.rs`

```rust
use proptest::prelude::*;
use edgevec::filter::*;

// ... property test implementations as specified in FILTER_TEST_STRATEGY.md ...
```

### Acceptance Criteria
- [ ] All 17 invariants implemented
- [ ] `PROPTEST_CASES=10000 cargo test --features proptest` passes
- [ ] No invariant violations found

---

## Task W23.6.5a: [C4] Fuzz Target - Simple Expressions

### Description
Implement fuzz target for simple filter expressions (basic patterns, shallow nesting).

### Hours: 2h

### Targets

| Target | File | Duration | Goal |
|:-------|:-----|:---------|:-----|
| F1 | `fuzz_parser.rs` | 10 min | Parser crashes |
| F2 | `fuzz_evaluator.rs` | 20 min | Evaluator crashes |
| F3 | `fuzz_like_pattern.rs` | 15 min | ReDoS |
| F4 | `fuzz_json_roundtrip.rs` | 10 min | Serialization bugs |
| F5 | `fuzz_wasm_boundary.rs` | 10 min | WASM boundary issues |

**Directory:** `fuzz/fuzz_targets/`

### Commands
```bash
cargo +nightly fuzz run fuzz_parser -- -max_total_time=600
cargo +nightly fuzz run fuzz_evaluator -- -max_total_time=1200
cargo +nightly fuzz run fuzz_like_pattern -- -max_total_time=900
cargo +nightly fuzz run fuzz_json_roundtrip -- -max_total_time=600
cargo +nightly fuzz run fuzz_wasm_boundary -- -max_total_time=600
```

### Acceptance Criteria
- [ ] All 5 fuzz targets implemented
- [ ] Each target runs for minimum duration without findings
- [ ] No crashes or panics found

---

## Task W23.6.5b: [C4] Fuzz Target - Deeply Nested Expressions

### Description
Implement fuzz target specifically for deeply nested filter expressions to catch stack overflow and parser edge cases.

### Hours: 2h

**[C4] Critical Issue Addressed:**
The original plan had no coverage for adversarial deeply nested expressions like:
`A AND (B OR (C AND (D OR (E AND ...))))` (depth 50+)

### Technical Specification

**File:** `fuzz/fuzz_targets/fuzz_deep_nesting.rs`

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use edgevec::filter::parse;

// Maximum AST depth to test
const MAX_DEPTH: usize = 50;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Generate deeply nested expression from fuzz input
        let expr = generate_nested_expr(s, MAX_DEPTH);

        // Parser should either succeed or return clean error
        // Must NOT:
        // - Stack overflow
        // - Panic
        // - Hang indefinitely
        match parse(&expr) {
            Ok(_) => { /* Valid parse */ }
            Err(_) => { /* Clean error */ }
        }
    }
});

fn generate_nested_expr(seed: &str, max_depth: usize) -> String {
    let depth = seed.len() % max_depth + 1;
    let mut expr = "x = 1".to_string();

    for i in 0..depth {
        let op = if seed.as_bytes().get(i).unwrap_or(&0) % 2 == 0 {
            "AND"
        } else {
            "OR"
        };
        expr = format!("({}) {} y{} = {}", expr, op, i, i);
    }

    expr
}
```

### Test Cases to Generate

1. **Pathological AND chains:** `((((x=1) AND y=2) AND z=3) AND ...)`
2. **Pathological OR chains:** `((((x=1) OR y=2) OR z=3) OR ...)`
3. **Mixed deep nesting:** `(a AND (b OR (c AND (d OR ...))))`
4. **NOT chains:** `NOT NOT NOT NOT ... x = 1`
5. **Parenthesis overload:** `((((((((((x=1))))))))))`

### Acceptance Criteria

- [ ] Fuzz target handles expressions up to depth 50
- [ ] 10M iterations without crash/panic
- [ ] No stack overflow detected
- [ ] Parser returns clean error for too-deep expressions
- [ ] Max depth limit enforced: `FilterError::NestingTooDeep`

### Commands
```bash
cargo +nightly fuzz run fuzz_deep_nesting -- -max_total_time=600 -max_len=10000
```

---

## Task W23.6.7: [C5] Multi-Field Filter Integration Tests

### Description
Integration tests verifying multi-field filters work correctly with HNSW search and ranking.

### Hours: 1h

**[C5] Critical Issue Addressed:**
Original plan didn't verify that complex multi-field filters work with search ranking.

### Technical Specification

**File:** `tests/filter_multifield_integration_tests.rs`

```rust
//! Multi-field filter integration tests for EdgeVec.
//! Addresses HOSTILE_REVIEWER critical issue [C5].

use edgevec::{HnswIndex, Filter, FilterStrategy, MetadataValue};
use std::collections::HashMap;

fn random_vector(dim: usize) -> Vec<f32> {
    (0..dim).map(|i| ((i * 17) % 100) as f32 / 100.0).collect()
}

/// [C5] Test: Multi-field filter with search ranking
/// Verifies results are BOTH filtered AND ranked by similarity.
#[test]
fn test_multifield_filter_with_ranking() {
    let mut index = HnswIndex::new(128);

    // Add 1000 vectors with multi-field metadata
    for i in 0..1000 {
        let vector = random_vector(128);
        let mut metadata = HashMap::new();

        // category: alternates between "electronics", "clothing", "food", "other"
        metadata.insert("category".to_string(),
            MetadataValue::String(["electronics", "clothing", "food", "other"][i % 4].to_string()));

        // price: ranges from 0 to 9999
        metadata.insert("price".to_string(),
            MetadataValue::Float((i * 10) as f64));

        // tags: array of strings
        let tags = vec![
            format!("tag{}", i % 10),
            if i % 3 == 0 { "sale".to_string() } else { "regular".to_string() },
        ];
        metadata.insert("tags".to_string(),
            MetadataValue::StringArray(tags));

        // active: boolean
        metadata.insert("active".to_string(),
            MetadataValue::Boolean(i % 2 == 0));

        index.add_with_metadata(vector, metadata).unwrap();
    }

    // Query vector
    let query = random_vector(128);

    // Multi-field filter: category = "electronics" AND price < 100 AND tags ANY ["sale"]
    let filter = Filter::parse(
        r#"category = "electronics" AND price < 100 AND tags ANY ["sale"]"#
    ).unwrap();

    // Search with filter
    let results = index.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto).unwrap();

    // CRITICAL VERIFICATION: All results must match ALL filter conditions
    for result in &results.results {
        let meta = index.get_metadata(result.id).unwrap();

        // Verify category
        assert_eq!(
            meta.get("category").unwrap().as_str().unwrap(),
            "electronics",
            "Result {} has wrong category", result.id
        );

        // Verify price
        let price = meta.get("price").unwrap().as_f64().unwrap();
        assert!(
            price < 100.0,
            "Result {} has price {} >= 100", result.id, price
        );

        // Verify tags contain "sale"
        let tags = meta.get("tags").unwrap().as_string_array().unwrap();
        assert!(
            tags.contains(&"sale".to_string()),
            "Result {} tags {:?} don't contain 'sale'", result.id, tags
        );
    }

    // CRITICAL VERIFICATION: Results are ranked by similarity (descending)
    let mut prev_distance = 0.0f32;
    for result in &results.results {
        assert!(
            result.distance >= prev_distance,
            "Results not ranked by similarity: {} < {}", result.distance, prev_distance
        );
        prev_distance = result.distance;
    }
}

/// Test: 4-field conjunction filter
#[test]
fn test_four_field_conjunction() {
    // ... similar setup ...
    let filter = Filter::parse(
        r#"category = "electronics" AND price BETWEEN 50 AND 200 AND active = true AND tags ALL ["sale", "tag0"]"#
    ).unwrap();
    // ... verify all 4 conditions ...
}

/// Test: Mixed conjunction/disjunction multi-field
#[test]
fn test_mixed_logic_multifield() {
    let filter = Filter::parse(
        r#"(category = "electronics" OR category = "clothing") AND price < 500 AND NOT (active = false)"#
    ).unwrap();
    // ... verify results ...
}

/// Test: String operators on multiple fields
#[test]
fn test_string_ops_multifield() {
    let filter = Filter::parse(
        r#"name CONTAINS "Pro" AND description STARTS_WITH "High" AND category = "electronics""#
    ).unwrap();
    // ... verify results ...
}
```

### Acceptance Criteria

- [ ] Multi-field filter test passes with category + price + tags
- [ ] All results verified to match ALL filter conditions
- [ ] Results verified to be ranked by similarity
- [ ] 4+ field conjunction tested
- [ ] Mixed AND/OR logic tested
- [ ] String operators on multiple fields tested

---

## Task W23.6.8: [M3] Test Count Verification

### Description
Verify the test suite contains ≥1856 tests as specified.

### Hours: 1h

**[M3] Major Issue Addressed:**
Original plan had no explicit verification of the 1856 test target.

### Commands

```bash
# Count unit tests
echo "=== Unit Test Count ==="
cargo test --lib -- --list 2>/dev/null | grep -c "test$"

# Expected: ≥1856

# Breakdown
echo "=== Parser Tests ==="
cargo test --lib filter::parser -- --list 2>/dev/null | grep -c "test$"
# Expected: 344

echo "=== Evaluator Tests ==="
cargo test --lib filter::evaluator -- --list 2>/dev/null | grep -c "test$"
# Expected: 804

echo "=== Strategy Tests ==="
cargo test --lib filter::strategy -- --list 2>/dev/null | grep -c "test$"
# Expected: 408

echo "=== WASM Tests ==="
cargo test --lib wasm -- --list 2>/dev/null | grep -c "test$"
# Expected: 300

# Coverage
echo "=== Coverage Report ==="
cargo tarpaulin --out Stdout --skip-clean
# Expected: ≥90%
```

### Acceptance Criteria

- [ ] `cargo test --lib -- --list | wc -l` outputs ≥1856
- [ ] Coverage ≥90%: `cargo tarpaulin --out Stdout`
- [ ] Test count breakdown matches specification
- [ ] All tests pass: `cargo test --all-features`

---

## Task W23.6.6: Integration Tests

### Description
Implement integration tests for complete filter + HNSW search flow.

### Hours: 1h

### Specification

**File:** `tests/filter_integration_tests.rs`

```rust
//! Integration tests for filter + HNSW search.

use edgevec::{HnswIndex, Filter, FilterStrategy};

#[test]
fn test_e2e_simple_filter_search() {
    let mut index = HnswIndex::new(384);

    // Add vectors with metadata
    for i in 0..1000 {
        let vector = random_vector(384);
        let metadata = hashmap! {
            "category" => if i % 2 == 0 { "gpu" } else { "cpu" },
            "price" => (i * 10) as f64,
        };
        index.add_with_metadata(vector, metadata).unwrap();
    }

    // Search with filter
    let query = random_vector(384);
    let filter = Filter::parse("category = \"gpu\" AND price < 500").unwrap();

    let result = index.search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto).unwrap();

    // Verify all results match filter
    for r in &result.results {
        let meta = index.get_metadata(r.id).unwrap();
        assert_eq!(meta.get("category").unwrap().as_str(), "gpu");
        assert!(meta.get("price").unwrap().as_f64() < 500.0);
    }
}

// ... more integration tests ...
```

### Acceptance Criteria
- [ ] E2E tests pass
- [ ] All strategies tested
- [ ] Results verified against filter

---

## Test Execution Summary

### Commands

```bash
# All unit tests
cargo test --all-features

# Parser tests
cargo test filter::parser -- --nocapture

# Evaluator tests
cargo test filter::evaluator

# Strategy tests
cargo test filter::strategy

# Property tests (extended iterations)
PROPTEST_CASES=10000 cargo test --features proptest

# Fuzz targets (in sequence, ~65 min total)
cargo +nightly fuzz run fuzz_parser -- -max_total_time=600
cargo +nightly fuzz run fuzz_evaluator -- -max_total_time=1200
cargo +nightly fuzz run fuzz_like_pattern -- -max_total_time=900
cargo +nightly fuzz run fuzz_json_roundtrip -- -max_total_time=600
cargo +nightly fuzz run fuzz_wasm_boundary -- -max_total_time=600

# WASM tests
wasm-pack test --node
wasm-pack test --headless --chrome

# Coverage
cargo tarpaulin --out Html --ignore-tests
```

### Expected Results

| Category | Count | Status |
|:---------|:------|:-------|
| Parser unit tests | 344 | [ ] PASS |
| Evaluator unit tests | 804 | [ ] PASS |
| Strategy unit tests | 408 | [ ] PASS |
| WASM unit tests | 300 | [ ] PASS |
| **Unit Total** | **1,856** | |
| Property tests | 17 | [ ] PASS |
| Fuzz targets | 5 | [ ] CLEAN |
| Integration tests | ~50 | [ ] PASS |
| Coverage | >90% | [ ] VERIFIED |

---

## End of Day 6 Gate

**Pass Criteria:**
- [ ] 1,856+ unit tests pass
- [ ] 17 property invariants verified
- [ ] 5 fuzz targets run clean
- [ ] Coverage >90%
- [ ] `cargo test` all green
- [ ] `wasm-pack test` passes

**Handoff:**
```
[TEST_ENGINEER]: Day 6 Complete

Artifacts generated:
- tests/filter_parser_tests.rs (344 tests)
- tests/filter_evaluator_tests.rs (804 tests)
- tests/filter_strategy_tests.rs (408 tests)
- tests/filter_property_tests.rs (17 invariants)
- tests/filter_integration_tests.rs (~50 tests)
- fuzz/fuzz_targets/*.rs (5 targets)

Test Results:
- Unit tests: 1,856 PASS
- Property tests: 17 PASS
- Fuzz: 5 targets clean (65 min total)
- Coverage: XX%

Status: READY_FOR_DAY_7

Next: W23.7.1 (Performance benchmarks)
```

---

**Day 6 Total: 14 hours | 6 tasks | 1,856+ tests + 17 props + 5 fuzz**
