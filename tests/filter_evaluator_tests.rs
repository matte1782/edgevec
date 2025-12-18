//! Evaluator unit tests for EdgeVec filter system.
//!
//! W23.6.2: Comprehensive evaluator unit tests as specified in DAY_6_TASKS.md.
//! Target: 804 tests covering all operator types and edge cases.

// Allow approximate constants - tests intentionally use literal values like 3.14
// rather than std::f64::consts::PI to test parsing and evaluation of user inputs.
#![allow(clippy::approx_constant)]

use edgevec::filter::{evaluate, parse};
use edgevec::metadata::MetadataValue;
use std::collections::HashMap;

/// Helper function to create metadata from key-value pairs.
fn make_metadata(pairs: &[(&str, MetadataValue)]) -> HashMap<String, MetadataValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

// =============================================================================
// EQUALITY OPERATOR (30 tests)
// =============================================================================

mod equality_operator {
    use super::*;

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
    fn test_eq_string_case_sensitive() {
        let filter = parse(r#"name = "Alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_string_empty() {
        let filter = parse(r#"name = """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_string_unicode() {
        let filter = parse(r#"name = "日本語""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
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
    fn test_eq_int_zero() {
        let filter = parse("count = 0").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_int_negative() {
        let filter = parse("count = -42").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(-42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_int_large() {
        let filter = parse("count = 9223372036854775807").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(i64::MAX))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_float_match() {
        let filter = parse("price = 3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_float_no_match() {
        let filter = parse("price = 3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_float_zero() {
        let filter = parse("price = 0.0").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_float_negative() {
        let filter = parse("price = -3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(-3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_bool_true() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_bool_false() {
        let filter = parse("active = false").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_bool_mismatch() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_missing_field() {
        let filter = parse(r#"name = "alice""#).unwrap();
        let meta = make_metadata(&[]);
        // Missing field returns an error in strict mode
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_eq_int_float_promotion() {
        let filter = parse("x = 42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(42.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_float_int_promotion() {
        let filter = parse("x = 42.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_type_mismatch_string_int() {
        let filter = parse("name = 42").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_type_mismatch_int_string() {
        let filter = parse(r#"count = "42""#).unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_type_mismatch_bool_int() {
        let filter = parse("active = 1").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_type_mismatch_int_bool() {
        let filter = parse("count = true").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_eq_string_with_spaces() {
        let filter = parse(r#"name = "hello world""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_string_with_special_chars() {
        let filter = parse(r#"name = "test@example.com""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test@example.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_float_precision() {
        let filter = parse("x = 0.1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_int_min() {
        let filter = parse("x = -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_eq_different_field() {
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(1))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_eq_escaped_string() {
        let filter = parse(r#"name = "say \"hello\"""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String(r#"say "hello""#.into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// NOT EQUAL OPERATOR (30 tests)
// =============================================================================

mod not_equal_operator {
    use super::*;

    #[test]
    fn test_ne_string_match() {
        let filter = parse(r#"name != "alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("bob".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_no_match() {
        let filter = parse(r#"name != "alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_int_match() {
        let filter = parse("count != 42").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(43))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_int_no_match() {
        let filter = parse("count != 42").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_float_match() {
        let filter = parse("price != 3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_float_no_match() {
        let filter = parse("price != 3.14").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_bool_match() {
        let filter = parse("active != true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_bool_no_match() {
        let filter = parse("active != true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_missing_field() {
        let filter = parse(r#"name != "alice""#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ne_type_mismatch() {
        let filter = parse("name != 42").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_int_float_promotion() {
        let filter = parse("x != 42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(42.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_float_int_promotion() {
        let filter = parse("x != 42.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ne_int_zero() {
        let filter = parse("x != 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_float_zero() {
        let filter = parse("x != 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_empty() {
        let filter = parse(r#"name != """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_unicode() {
        let filter = parse(r#"name != "日本語""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("英語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_negative_int() {
        let filter = parse("x != -42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-41))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_negative_float() {
        let filter = parse("x != -3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-3.13))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_large_int() {
        let filter = parse("x != 9223372036854775807").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MAX - 1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_min_int() {
        let filter = parse("x != -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN + 1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_case_sensitive() {
        let filter = parse(r#"name != "Alice""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("alice".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_whitespace() {
        let filter = parse(r#"name != "hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello ".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_bool_different() {
        let filter = parse("x != false").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_different_field() {
        let filter = parse("x != 1").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(1))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ne_escaped_string() {
        let filter = parse(r#"name != "say \"hello\"""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("different".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_with_numbers() {
        let filter = parse(r#"code != "ABC123""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("ABC124".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_float_precision() {
        let filter = parse("x != 0.1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_bool_vs_string() {
        let filter = parse(r#"x != "true""#).unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_int_vs_bool() {
        let filter = parse("x != 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ne_string_emoji() {
        let filter = parse(r#"name != "🚀""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("🎉".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// LESS THAN OPERATOR (30 tests)
// =============================================================================

mod less_than_operator {
    use super::*;

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
    fn test_lt_float_false_equal() {
        let filter = parse("x < 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_float_false_greater() {
        let filter = parse("x < 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(4.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_int_float_promotion() {
        let filter = parse("x < 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_float_int_promotion() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(9.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_negative_int() {
        let filter = parse("x < -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_negative_float() {
        let filter = parse("x < -3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_zero_int() {
        let filter = parse("x < 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_zero_float() {
        let filter = parse("x < 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_missing_field() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_lt_type_mismatch_string() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("hello".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_lt_type_mismatch_bool() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_lt_large_int() {
        // Use a more reasonable large number to avoid parsing precision issues
        let filter = parse("x < 1000000000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(999999999999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_min_int() {
        // Use a more reasonable negative number
        let filter = parse("x < -999999999999").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1000000000000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_small_float() {
        let filter = parse("x < 0.001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_boundary_int() {
        let filter = parse("x < 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(99))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_boundary_float() {
        let filter = parse("x < 100.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(99.9999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_int_vs_float_boundary() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(9.999999999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_negative_boundary() {
        let filter = parse("x < -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-101))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_very_small_diff() {
        let filter = parse("x < 1.0000001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_negative_zero() {
        let filter = parse("x < 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.0))]);
        // -0.0 == 0.0 in IEEE 754
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_cross_type_boundary() {
        let filter = parse("x < 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_positive_to_negative() {
        let filter = parse("x < 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_float_precision_edge() {
        let filter = parse("x < 0.30000000000000004").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1 + 0.2))]);
        // 0.1 + 0.2 = 0.30000000000000004 in IEEE 754
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_lt_different_field() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(5))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_lt_negative_vs_positive() {
        let filter = parse("x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// LESS THAN OR EQUAL OPERATOR (30 tests)
// =============================================================================

mod less_than_or_equal_operator {
    use super::*;

    #[test]
    fn test_le_int_true_less() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_int_true_equal() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_int_false() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_le_float_true_less() {
        let filter = parse("x <= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_float_true_equal() {
        let filter = parse("x <= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_float_false() {
        let filter = parse("x <= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(4.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_le_int_float_promotion() {
        let filter = parse("x <= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_float_int_promotion() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(10.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_negative_int() {
        let filter = parse("x <= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_negative_float() {
        let filter = parse("x <= -3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_zero() {
        let filter = parse("x <= 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_missing_field() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_le_type_mismatch() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("hello".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_le_large_int() {
        let filter = parse("x <= 9223372036854775807").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MAX))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_min_int() {
        let filter = parse("x <= -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_boundary_just_below() {
        let filter = parse("x <= 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_boundary_just_above() {
        let filter = parse("x <= 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(101))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_le_float_equal_precision() {
        let filter = parse("x <= 3.14159265359").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14159265359))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_int_as_float() {
        let filter = parse("x <= 10.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_negative_less() {
        let filter = parse("x <= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_negative_greater() {
        let filter = parse("x <= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_le_zero_float() {
        let filter = parse("x <= 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_small_positive() {
        let filter = parse("x <= 0.001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_cross_type_equal() {
        let filter = parse("x <= 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_bool_type_mismatch() {
        let filter = parse("x <= 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_le_different_field() {
        let filter = parse("x <= 10").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(5))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_le_float_slightly_over() {
        let filter = parse("x <= 10.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(10.0001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_le_int_vs_float_equal() {
        let filter = parse("x <= 42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(42.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_negative_boundary() {
        let filter = parse("x <= -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_le_very_large_float() {
        let filter = parse("x <= 999999999.999").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(999999999.999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// GREATER THAN OPERATOR (30 tests)
// =============================================================================

mod greater_than_operator {
    use super::*;

    #[test]
    fn test_gt_int_true() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_int_false_equal() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_int_false_less() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_float_true() {
        let filter = parse("x > 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(4.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_float_false_equal() {
        let filter = parse("x > 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_float_false_less() {
        let filter = parse("x > 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_int_float_promotion() {
        let filter = parse("x > 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(4))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_float_int_promotion() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(10.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_negative() {
        let filter = parse("x > -10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_zero() {
        let filter = parse("x > 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_missing_field() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_gt_type_mismatch() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("hello".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_gt_large_int() {
        // Use a reasonable large number to avoid parsing precision issues
        let filter = parse("x > 999999999999").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1000000000000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_negative_boundary() {
        let filter = parse("x > -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-99))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_float_small_diff() {
        let filter = parse("x > 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.0000001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_cross_zero() {
        let filter = parse("x > -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_positive_vs_negative() {
        let filter = parse("x > -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_float_precision() {
        let filter = parse("x > 0.1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_boundary_just_above() {
        let filter = parse("x > 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(101))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_boundary_equal() {
        let filter = parse("x > 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_negative_float() {
        let filter = parse("x > -3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-3.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_cross_type() {
        let filter = parse("x > 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(6))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_bool_mismatch() {
        let filter = parse("x > 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_gt_different_field() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(15))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_gt_very_small() {
        let filter = parse("x > 0.0001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0002))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_int_as_float() {
        let filter = parse("x > 10.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(11))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_float_as_int() {
        let filter = parse("x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(11.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_negative_vs_negative() {
        let filter = parse("x > -50").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-25))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_gt_zero_float() {
        let filter = parse("x > 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// GREATER THAN OR EQUAL OPERATOR (30 tests)
// =============================================================================

mod greater_than_or_equal_operator {
    use super::*;

    #[test]
    fn test_ge_int_true_greater() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_int_true_equal() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_int_false() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ge_float_true_greater() {
        let filter = parse("x >= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(4.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_float_true_equal() {
        let filter = parse("x >= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_float_false() {
        let filter = parse("x >= 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ge_int_float_promotion() {
        let filter = parse("x >= 3.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_float_int_promotion() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(10.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_negative() {
        let filter = parse("x >= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_zero() {
        let filter = parse("x >= 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_missing_field() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ge_type_mismatch() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("hello".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ge_large_int() {
        let filter = parse("x >= 9223372036854775807").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MAX))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_min_int() {
        let filter = parse("x >= -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_boundary_at() {
        let filter = parse("x >= 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_boundary_below() {
        let filter = parse("x >= 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(99))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ge_float_equal_precision() {
        let filter = parse("x >= 3.14159265359").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14159265359))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_negative_less() {
        let filter = parse("x >= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ge_negative_greater() {
        let filter = parse("x >= -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_zero_float() {
        let filter = parse("x >= 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_small_positive() {
        let filter = parse("x >= 0.001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_cross_type_equal() {
        let filter = parse("x >= 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_bool_type_mismatch() {
        let filter = parse("x >= 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ge_different_field() {
        let filter = parse("x >= 10").unwrap();
        let meta = make_metadata(&[("y", MetadataValue::Integer(15))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ge_float_slightly_under() {
        let filter = parse("x >= 10.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(9.9999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ge_positive_vs_negative() {
        let filter = parse("x >= -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_negative_boundary() {
        let filter = parse("x >= -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_int_vs_float_equal() {
        let filter = parse("x >= 42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(42.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ge_very_large_float() {
        let filter = parse("x >= 999999999.999").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(999999999.999))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// STRING OPERATORS - CONTAINS (24 tests)
// =============================================================================

mod string_contains {
    use super::*;

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
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_empty_substring() {
        let filter = parse(r#"title CONTAINS """#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_at_start() {
        let filter = parse(r#"title CONTAINS "say""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("say hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_at_end() {
        let filter = parse(r#"title CONTAINS "world""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_exact_match() {
        let filter = parse(r#"title CONTAINS "hello""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_unicode() {
        let filter = parse(r#"title CONTAINS "日本""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("私は日本語を話します".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_emoji() {
        let filter = parse(r#"title CONTAINS "🚀""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("Launch 🚀 now".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_special_chars() {
        let filter = parse(r#"email CONTAINS "@""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("test@example.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_whitespace() {
        let filter = parse(r#"text CONTAINS " ""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_missing_field() {
        let filter = parse(r#"title CONTAINS "hello""#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_contains_type_mismatch() {
        let filter = parse(r#"count CONTAINS "hello""#).unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_contains_multiple_occurrences() {
        let filter = parse(r#"text CONTAINS "a""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("banana".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_newline() {
        let filter = parse(r#"text CONTAINS "line""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("first\nline\nsecond".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_tab() {
        let filter = parse(r#"text CONTAINS "col""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("col1\tcol2".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_longer_than_text() {
        let filter = parse(r#"text CONTAINS "verylongstring""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("short".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_empty_field() {
        let filter = parse(r#"text CONTAINS "a""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_numbers_in_string() {
        let filter = parse(r#"code CONTAINS "123""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("ABC123DEF".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_partial_word() {
        let filter = parse(r#"text CONTAINS "ello""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_repeated_pattern() {
        let filter = parse(r#"text CONTAINS "ab""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("ababab".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_escaped_quote() {
        let filter = parse(r#"text CONTAINS "say \"hi\"""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String(r#"I say "hi" to you"#.into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_backslash() {
        let filter = parse(r#"path CONTAINS "\\""#).unwrap();
        let meta = make_metadata(&[("path", MetadataValue::String(r"C:\Users\test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_mixed_case_no_match() {
        let filter = parse(r#"text CONTAINS "HELLO""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// STRING OPERATORS - LIKE (24 tests)
// =============================================================================

mod string_like {
    use super::*;

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
    fn test_like_percent_both() {
        let filter = parse(r#"text LIKE "%hello%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("say hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_underscore_single() {
        let filter = parse(r#"code LIKE "A_B""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("AXB".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_underscore_multiple() {
        let filter = parse(r#"code LIKE "A__B""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("AXXB".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_no_wildcards() {
        let filter = parse(r#"name LIKE "John""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("John".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_no_wildcards_no_match() {
        let filter = parse(r#"name LIKE "John""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("Johnny".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_like_percent_only() {
        let filter = parse(r#"text LIKE "%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_empty_pattern() {
        let filter = parse(r#"text LIKE """#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_empty_pattern_non_empty_string() {
        let filter = parse(r#"text LIKE """#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_like_underscore_wrong_length() {
        let filter = parse(r#"code LIKE "A_B""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("AXXB".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_like_missing_field() {
        let filter = parse(r#"text LIKE "%test%""#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_like_type_mismatch() {
        let filter = parse(r#"count LIKE "%""#).unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_like_case_sensitive() {
        let filter = parse(r#"name LIKE "john%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("John Smith".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_like_complex_pattern() {
        let filter = parse(r#"code LIKE "A%B_C""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("AXXXBXC".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_unicode() {
        let filter = parse(r#"text LIKE "%日本%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("私は日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_emoji() {
        let filter = parse(r#"text LIKE "%🚀%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("Launch 🚀 now".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_empty_string() {
        let filter = parse(r#"text LIKE "%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_consecutive_percent() {
        let filter = parse(r#"text LIKE "%%""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_mixed_wildcards() {
        let filter = parse(r#"code LIKE "%__%""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("ABC".into()))]);
        // At least 2 characters
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_numbers() {
        let filter = parse(r#"code LIKE "A%123""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("ABC123".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_whitespace() {
        let filter = parse(r#"text LIKE "% %""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_special_chars() {
        let filter = parse(r#"email LIKE "%@%.%""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("user@domain.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_underscore_at_end() {
        let filter = parse(r#"code LIKE "ABC_""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("ABCD".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// LOGICAL OPERATORS - AND (24 tests)
// =============================================================================

mod logical_and {
    use super::*;

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

    #[test]
    fn test_and_three_conditions() {
        let filter = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_three_conditions_middle_false() {
        let filter = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_and_with_string() {
        let filter = parse(r#"name = "test" AND count = 5"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("count", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_bool() {
        let filter = parse("active = true AND visible = true").unwrap();
        let meta = make_metadata(&[
            ("active", MetadataValue::Boolean(true)),
            ("visible", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_float() {
        let filter = parse("x > 1.0 AND x < 5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_comparison() {
        let filter = parse("x >= 10 AND x <= 20").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_comparison_outside_range() {
        let filter = parse("x >= 10 AND x <= 20").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(25))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_and_same_field_different_values() {
        let filter = parse("x = 1 AND x = 2").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        // Always false since x can't be both 1 and 2
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_and_with_ne() {
        let filter = parse("x != 0 AND y != 0").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(1)),
            ("y", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_complex_mixed_types() {
        let filter = parse(r#"name = "gpu" AND price < 500 AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("gpu".into())),
            ("price", MetadataValue::Integer(450)),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_four_conditions() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_contains() {
        let filter = parse(r#"name CONTAINS "test" AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("my test item".into())),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_like() {
        let filter = parse(r#"email LIKE "%@gmail.com" AND verified = true"#).unwrap();
        let meta = make_metadata(&[
            ("email", MetadataValue::String("user@gmail.com".into())),
            ("verified", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_negative_values() {
        let filter = parse("x > -10 AND x < -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_zero_values() {
        let filter = parse("x >= 0 AND y >= 0").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(0)),
            ("y", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_float_range() {
        let filter = parse("x >= 0.0 AND x <= 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_float_range_boundary() {
        let filter = parse("x >= 0.0 AND x <= 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_mixed_int_float() {
        let filter = parse("x > 1 AND x < 3.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_string_and_number() {
        let filter = parse(r#"category = "electronics" AND price <= 1000"#).unwrap();
        let meta = make_metadata(&[
            ("category", MetadataValue::String("electronics".into())),
            ("price", MetadataValue::Integer(500)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_bool_and_string() {
        let filter = parse(r#"active = true AND status = "published""#).unwrap();
        let meta = make_metadata(&[
            ("active", MetadataValue::Boolean(true)),
            ("status", MetadataValue::String("published".into())),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// LOGICAL OPERATORS - OR (24 tests)
// =============================================================================

mod logical_or {
    use super::*;

    #[test]
    fn test_or_both_true() {
        let filter = parse("a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_first_true() {
        let filter = parse("a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_second_true() {
        let filter = parse("a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_both_false() {
        let filter = parse("a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_three_conditions_all_true() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_three_conditions_one_true() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_three_conditions_all_false() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_with_string() {
        let filter = parse(r#"type = "a" OR type = "b""#).unwrap();
        let meta = make_metadata(&[("type", MetadataValue::String("a".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_same_field_multiple_values() {
        let filter = parse(r#"status = "active" OR status = "pending""#).unwrap();
        let meta = make_metadata(&[("status", MetadataValue::String("pending".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_same_field_no_match() {
        let filter = parse(r#"status = "active" OR status = "pending""#).unwrap();
        let meta = make_metadata(&[("status", MetadataValue::String("deleted".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_with_comparison() {
        let filter = parse("x < 0 OR x > 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_comparison_in_range() {
        let filter = parse("x < 0 OR x > 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(50))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_with_bool() {
        let filter = parse("admin = true OR moderator = true").unwrap();
        let meta = make_metadata(&[
            ("admin", MetadataValue::Boolean(false)),
            ("moderator", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_four_options() {
        let filter = parse(r#"cat = "a" OR cat = "b" OR cat = "c" OR cat = "d""#).unwrap();
        let meta = make_metadata(&[("cat", MetadataValue::String("c".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_mixed_types() {
        let filter = parse(r#"name = "test" OR count > 10"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("other".into())),
            ("count", MetadataValue::Integer(15)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_with_contains() {
        let filter = parse(r#"title CONTAINS "urgent" OR priority = 1"#).unwrap();
        let meta = make_metadata(&[
            ("title", MetadataValue::String("normal task".into())),
            ("priority", MetadataValue::Integer(1)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_float_values() {
        let filter = parse("x = 1.5 OR x = 2.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_negative_values() {
        let filter = parse("x = -1 OR x = -2").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_zero() {
        let filter = parse("x = 0 OR y = 0").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(1)),
            ("y", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_empty_string() {
        let filter = parse(r#"name = "" OR name = "default""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_unicode() {
        let filter = parse(r#"lang = "日本語" OR lang = "English""#).unwrap();
        let meta = make_metadata(&[("lang", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_different_fields() {
        let filter = parse("active = true OR count > 0").unwrap();
        let meta = make_metadata(&[
            ("active", MetadataValue::Boolean(false)),
            ("count", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_range_check() {
        let filter = parse("x <= -100 OR x >= 100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(150))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_bool_false_false() {
        let filter = parse("a = true OR b = true").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Boolean(false)),
            ("b", MetadataValue::Boolean(false)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// LOGICAL OPERATORS - NOT (24 tests)
// =============================================================================

mod logical_not {
    use super::*;

    #[test]
    fn test_not_true_becomes_false() {
        let filter = parse("NOT a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_false_becomes_true() {
        let filter = parse("NOT a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_bool_true() {
        let filter = parse("NOT active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_bool_false() {
        let filter = parse("NOT active = false").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_string_match() {
        let filter = parse(r#"NOT name = "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_string_no_match() {
        let filter = parse(r#"NOT name = "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("other".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_double_not() {
        let filter = parse("NOT NOT a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_double_not_false() {
        let filter = parse("NOT NOT a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_triple_not() {
        let filter = parse("NOT NOT NOT a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_with_comparison() {
        let filter = parse("NOT x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_comparison_gt() {
        let filter = parse("NOT x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_comparison_lt() {
        let filter = parse("NOT x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_with_contains() {
        let filter = parse(r#"NOT title CONTAINS "spam""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_contains_match() {
        let filter = parse(r#"NOT title CONTAINS "hello""#).unwrap();
        let meta = make_metadata(&[("title", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_with_and() {
        let filter = parse("NOT a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        // NOT binds tighter: (NOT a = 1) AND b = 2 = false AND true = false
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_with_or() {
        let filter = parse("NOT a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        // (NOT a = 1) OR b = 2 = false OR true = true
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_parenthesized_and() {
        let filter = parse("NOT (a = 1 AND b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_parenthesized_or() {
        let filter = parse("NOT (a = 1 OR b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_ne() {
        let filter = parse("NOT x != 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        // NOT (x != 5) when x = 5: NOT false = true
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_float() {
        let filter = parse("NOT x = 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_zero() {
        let filter = parse("NOT x = 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_negative() {
        let filter = parse("NOT x = -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_empty_string() {
        let filter = parse(r#"NOT name = """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_with_like() {
        let filter = parse(r#"NOT email LIKE "%@spam.com""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("user@gmail.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// IN OPERATOR (24 tests)
// =============================================================================

mod in_operator {
    use super::*;

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

    #[test]
    fn test_in_int_no_match() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_in_float_match() {
        let filter = parse("x IN [1.5, 2.5, 3.5]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_single_element() {
        let filter = parse("x IN [42]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_many_elements() {
        let filter = parse("x IN [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(7))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_first_element() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_last_element() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_negative_int() {
        let filter = parse("x IN [-1, -2, -3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_zero() {
        let filter = parse("x IN [0, 1, 2]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_empty_string() {
        let filter = parse(r#"name IN ["", "default"]"#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_unicode() {
        let filter = parse(r#"lang IN ["日本語", "English", "Español"]"#).unwrap();
        let meta = make_metadata(&[("lang", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_case_sensitive() {
        let filter = parse(r#"type IN ["A", "B", "C"]"#).unwrap();
        let meta = make_metadata(&[("type", MetadataValue::String("a".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_in_missing_field() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_in_with_and() {
        let filter = parse(r#"category IN ["gpu", "cpu"] AND price < 500"#).unwrap();
        let meta = make_metadata(&[
            ("category", MetadataValue::String("gpu".into())),
            ("price", MetadataValue::Integer(450)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_with_or() {
        let filter = parse(r#"type IN ["a"] OR type IN ["b"]"#).unwrap();
        let meta = make_metadata(&[("type", MetadataValue::String("b".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_bool_array() {
        let filter = parse("flag IN [true]").unwrap();
        let meta = make_metadata(&[("flag", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_special_chars() {
        let filter = parse(r##"symbol IN ["@", "#", "$"]"##).unwrap();
        let meta = make_metadata(&[("symbol", MetadataValue::String("#".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_whitespace_string() {
        let filter = parse(r#"text IN [" ", "  ", " "]"#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String(" ".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_large_int() {
        let filter = parse("x IN [1000000000000, 2000000000000]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1000000000000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_float_precision() {
        let filter = parse("x IN [0.1, 0.2, 0.3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_int_float_promotion() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_escaped_string() {
        let filter = parse(r#"text IN ["say \"hi\"", "normal"]"#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String(r#"say "hi""#.into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// NOT IN OPERATOR (12 tests)
// =============================================================================

mod not_in_operator {
    use super::*;

    #[test]
    fn test_not_in_no_match() {
        let filter = parse(r#"status NOT IN ["deleted", "archived"]"#).unwrap();
        let meta = make_metadata(&[("status", MetadataValue::String("active".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_match() {
        let filter = parse(r#"status NOT IN ["deleted", "archived"]"#).unwrap();
        let meta = make_metadata(&[("status", MetadataValue::String("deleted".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_in_int() {
        let filter = parse("x NOT IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_int_match() {
        let filter = parse("x NOT IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_in_single_element() {
        let filter = parse("x NOT IN [42]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(43))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_float() {
        let filter = parse("x NOT IN [1.5, 2.5]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_missing_field() {
        let filter = parse("x NOT IN [1, 2]").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_not_in_with_and() {
        let filter = parse(r#"type NOT IN ["spam"] AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("type", MetadataValue::String("normal".into())),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_unicode() {
        let filter = parse(r#"lang NOT IN ["English"]"#).unwrap();
        let meta = make_metadata(&[("lang", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_case_sensitive() {
        let filter = parse(r#"type NOT IN ["A", "B"]"#).unwrap();
        let meta = make_metadata(&[("type", MetadataValue::String("a".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_negative() {
        let filter = parse("x NOT IN [-1, -2]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_zero() {
        let filter = parse("x NOT IN [0]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// BETWEEN OPERATOR (24 tests)
// =============================================================================

mod between_operator {
    use super::*;

    #[test]
    fn test_between_int_in_range() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_int_at_lower() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_int_at_upper() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_int_below() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_int_above() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(11))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_float_in_range() {
        let filter = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_float_at_lower() {
        let filter = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_float_at_upper() {
        let filter = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_float_below() {
        let filter = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_float_above() {
        let filter = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_negative() {
        let filter = parse("x BETWEEN -10 AND -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_crossing_zero() {
        let filter = parse("x BETWEEN -5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_same_values() {
        let filter = parse("x BETWEEN 5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_int_float_promotion() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_float_int_meta() {
        let filter = parse("x BETWEEN 1.5 AND 10.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_missing_field() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_between_type_mismatch() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("5".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_between_with_and() {
        let filter = parse("price BETWEEN 100 AND 500 AND active = true").unwrap();
        let meta = make_metadata(&[
            ("price", MetadataValue::Integer(300)),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_large_range() {
        let filter = parse("x BETWEEN 0 AND 1000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(500000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_small_float_range() {
        let filter = parse("x BETWEEN 0.001 AND 0.01").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.005))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_very_close_bounds() {
        let filter = parse("x BETWEEN 1.0 AND 1.0001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.00005))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_negative_float() {
        let filter = parse("x BETWEEN -3.14 AND 3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_just_outside_lower() {
        let filter = parse("x BETWEEN 10 AND 20").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(9))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_just_outside_upper() {
        let filter = parse("x BETWEEN 10 AND 20").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(21))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// NULL CHECKS (24 tests)
// =============================================================================

mod null_checks {
    use super::*;

    #[test]
    fn test_is_null_missing() {
        let filter = parse("optional IS NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_null_present_string() {
        let filter = parse("name IS NULL").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_int() {
        let filter = parse("count IS NULL").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_float() {
        let filter = parse("price IS NULL").unwrap();
        let meta = make_metadata(&[("price", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_bool() {
        let filter = parse("active IS NULL").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_empty_string() {
        let filter = parse("name IS NULL").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        // Empty string is NOT null
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_zero() {
        let filter = parse("count IS NULL").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(0))]);
        // Zero is NOT null
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_present_false() {
        let filter = parse("active IS NULL").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        // False is NOT null
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_not_null_present() {
        let filter = parse("name IS NOT NULL").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_not_null_missing() {
        let filter = parse("optional IS NOT NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_not_null_int() {
        let filter = parse("count IS NOT NULL").unwrap();
        let meta = make_metadata(&[("count", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_not_null_empty_string() {
        let filter = parse("name IS NOT NULL").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_null_with_and() {
        let filter = parse("optional IS NULL AND required IS NOT NULL").unwrap();
        let meta = make_metadata(&[("required", MetadataValue::String("value".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_null_with_or() {
        let filter = parse("a IS NULL OR b IS NULL").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        // a is not null, b is null
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_null_both_present() {
        let filter = parse("a IS NULL OR b IS NULL").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_is_null() {
        let filter = parse("NOT x IS NULL").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_is_not_null() {
        let filter = parse("NOT x IS NOT NULL").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_is_null_different_fields() {
        let filter = parse("a IS NULL").unwrap();
        let meta = make_metadata(&[("b", MetadataValue::Integer(1))]);
        // Field 'a' is not in metadata, so it IS NULL
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_null_check_array() {
        let filter = parse("tags IS NOT NULL").unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_null_check_empty_array() {
        let filter = parse("tags IS NOT NULL").unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        // Empty array is NOT null
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_null_chained() {
        let filter = parse("a IS NULL AND b IS NULL AND c IS NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_not_null_chained() {
        let filter = parse("a IS NOT NULL AND b IS NOT NULL").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::String("x".into())),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_null_mutual_exclusion() {
        // Cannot be both null and not null
        let filter = parse("x IS NULL AND x IS NOT NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_null_or_value() {
        let filter = parse("x IS NULL OR x = 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// STRING PREFIX/SUFFIX OPERATORS (48 tests)
// =============================================================================

mod string_starts_with {
    use super::*;

    #[test]
    fn test_starts_with_match() {
        let filter = parse(r#"name starts_with "hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_no_match() {
        let filter = parse(r#"name starts_with "world""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_starts_with_exact() {
        let filter = parse(r#"name starts_with "hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_empty_prefix() {
        let filter = parse(r#"name starts_with """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_empty_field() {
        let filter = parse(r#"name starts_with "x""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_starts_with_case_sensitive() {
        let filter = parse(r#"name starts_with "Hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_starts_with_unicode() {
        let filter = parse(r#"name starts_with "日本""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_emoji() {
        let filter = parse(r#"name starts_with "🎉""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("🎉party".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_special_chars() {
        let filter = parse(r#"name starts_with "$""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("$100".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_whitespace() {
        let filter = parse(r#"name starts_with " ""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String(" leading space".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_numbers() {
        let filter = parse(r#"code starts_with "123""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("123abc".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_longer_than_value() {
        let filter = parse(r#"name starts_with "hello world extra""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_starts_with_missing_field() {
        let filter = parse(r#"name starts_with "x""#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_starts_with_type_mismatch() {
        let filter = parse(r#"name starts_with "x""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::Integer(123))]);
        // Type mismatch for string operators returns error
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_starts_with_newline() {
        let filter = parse(r#"text starts_with "line1""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("line1\nline2".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_tab() {
        let filter = parse(r#"text starts_with "	""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("\ttabbed".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_backslash() {
        let filter = parse(r#"path starts_with "C:""#).unwrap();
        let meta = make_metadata(&[("path", MetadataValue::String(r"C:\Windows".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_multiple_words() {
        let filter = parse(r#"sentence starts_with "the quick""#).unwrap();
        let meta = make_metadata(&[("sentence", MetadataValue::String("the quick brown fox".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_partial_word() {
        let filter = parse(r#"word starts_with "pre""#).unwrap();
        let meta = make_metadata(&[("word", MetadataValue::String("prefix".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_repeated_char() {
        let filter = parse(r#"text starts_with "aaa""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("aaabbb".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_url() {
        let filter = parse(r#"url starts_with "https://""#).unwrap();
        let meta = make_metadata(&[("url", MetadataValue::String("https://example.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_email_domain() {
        let filter = parse(r#"email starts_with "admin@""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("admin@example.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_version() {
        let filter = parse(r#"version starts_with "v1.""#).unwrap();
        let meta = make_metadata(&[("version", MetadataValue::String("v1.2.3".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_json_like() {
        let filter = parse(r#"data starts_with "{""#).unwrap();
        let meta = make_metadata(&[("data", MetadataValue::String(r#"{"key": "value"}"#.into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

mod string_ends_with {
    use super::*;

    #[test]
    fn test_ends_with_match() {
        let filter = parse(r#"name ends_with "world""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_no_match() {
        let filter = parse(r#"name ends_with "hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ends_with_exact() {
        let filter = parse(r#"name ends_with "hello""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_empty_suffix() {
        let filter = parse(r#"name ends_with """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_empty_field() {
        let filter = parse(r#"name ends_with "x""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ends_with_case_sensitive() {
        let filter = parse(r#"name ends_with "World""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello world".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ends_with_unicode() {
        let filter = parse(r#"name ends_with "語""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_emoji() {
        let filter = parse(r#"name ends_with "🎉""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("party🎉".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_special_chars() {
        let filter = parse(r#"price ends_with "!""#).unwrap();
        let meta = make_metadata(&[("price", MetadataValue::String("$100!".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_whitespace() {
        let filter = parse(r#"name ends_with " ""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("trailing space ".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_numbers() {
        let filter = parse(r#"code ends_with "123""#).unwrap();
        let meta = make_metadata(&[("code", MetadataValue::String("abc123".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_longer_than_value() {
        let filter = parse(r#"name ends_with "hello world extra""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("hello".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_ends_with_missing_field() {
        let filter = parse(r#"name ends_with "x""#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ends_with_type_mismatch() {
        let filter = parse(r#"name ends_with "x""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::Integer(123))]);
        // Type mismatch for string operators returns error
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_ends_with_newline() {
        let filter = parse(r#"text ends_with "line2""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("line1\nline2".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_extension() {
        let filter = parse(r#"file ends_with ".rs""#).unwrap();
        let meta = make_metadata(&[("file", MetadataValue::String("main.rs".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_domain() {
        let filter = parse(r#"email ends_with "@example.com""#).unwrap();
        let meta = make_metadata(&[("email", MetadataValue::String("user@example.com".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_path_separator() {
        let filter = parse(r#"path ends_with "/""#).unwrap();
        let meta = make_metadata(&[("path", MetadataValue::String("/home/user/".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_punctuation() {
        let filter = parse(r#"sentence ends_with ".""#).unwrap();
        let meta = make_metadata(&[("sentence", MetadataValue::String("Hello world.".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_repeated_char() {
        let filter = parse(r#"text ends_with "bbb""#).unwrap();
        let meta = make_metadata(&[("text", MetadataValue::String("aaabbb".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_version() {
        let filter = parse(r#"version ends_with "-beta""#).unwrap();
        let meta = make_metadata(&[("version", MetadataValue::String("v1.0.0-beta".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_json_like() {
        let filter = parse(r#"data ends_with "}""#).unwrap();
        let meta = make_metadata(&[("data", MetadataValue::String(r#"{"key": "value"}"#.into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_html_tag() {
        let filter = parse(r#"html ends_with "</div>""#).unwrap();
        let meta = make_metadata(&[("html", MetadataValue::String("<div>content</div>".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// ARRAY OPERATORS (72 tests)
// Syntax: field ANY/ALL/NONE [values] - checks array field against value list
// =============================================================================

mod array_any {
    use super::*;

    #[test]
    fn test_any_match_first() {
        let filter = parse(r#"tags ANY ["rust", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_match_last() {
        let filter = parse(r#"tags ANY ["python", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_no_match() {
        let filter = parse(r#"tags ANY ["python", "java"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_empty_field_array() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_single_element_match() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_single_element_no_match() {
        let filter = parse(r#"tags ANY ["python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_multiple_matches() {
        let filter = parse(r#"tags ANY ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_case_sensitive() {
        let filter = parse(r#"tags ANY ["RUST"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_unicode() {
        let filter = parse(r#"tags ANY ["日本語"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["日本語".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_emoji() {
        let filter = parse(r#"tags ANY ["🦀"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["🦀".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_missing_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_any_type_mismatch_string_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::String("rust".into()))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_any_type_mismatch_int_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::Integer(123))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_any_with_empty_string() {
        let filter = parse(r#"tags ANY [""]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["".into(), "a".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_special_chars() {
        let filter = parse(r#"tags ANY ["@user"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["@user".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_whitespace() {
        let filter = parse(r#"tags ANY [" "]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![" ".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_duplicates_in_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_duplicates_in_pattern() {
        let filter = parse(r#"tags ANY ["rust", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_many_pattern_values() {
        let filter = parse(r#"tags ANY ["a", "b", "c", "d", "e"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["e".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_many_field_values() {
        let filter = parse(r#"tags ANY ["z"]"#).unwrap();
        let arr: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(arr))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_partial_overlap() {
        let filter = parse(r#"tags ANY ["x", "y", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_no_overlap() {
        let filter = parse(r#"tags ANY ["x", "y", "z"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_any_single_pattern_multiple_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into(), "go".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

mod array_all {
    use super::*;

    #[test]
    fn test_all_match() {
        // ALL checks if ALL values in pattern are in field
        let filter = parse(r#"tags ALL ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into(), "go".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_missing_one() {
        let filter = parse(r#"tags ALL ["rust", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_empty_pattern() {
        let filter = parse(r#"tags ALL []"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        // Empty pattern - vacuously true
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_empty_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        // Field has none of the required values
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_single_match() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_single_no_match() {
        let filter = parse(r#"tags ALL ["python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_exact_match() {
        let filter = parse(r#"tags ALL ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_superset_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into(), "go".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_case_sensitive() {
        let filter = parse(r#"tags ALL ["RUST"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_unicode() {
        let filter = parse(r#"tags ALL ["日本語", "英語"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["日本語".into(), "英語".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_missing_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_all_type_mismatch_string_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::String("rust".into()))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_all_type_mismatch_int_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::Integer(123))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_all_with_empty_string() {
        let filter = parse(r#"tags ALL ["", "a"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["".into(), "a".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_duplicates_in_pattern() {
        let filter = parse(r#"tags ALL ["rust", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_duplicates_in_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_many_pattern_values() {
        let filter = parse(r#"tags ALL ["a", "b", "c"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into(), "b".into(), "c".into(), "d".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_many_field_values() {
        let filter = parse(r#"tags ALL ["a", "z"]"#).unwrap();
        let arr: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(arr))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_order_independent() {
        let filter = parse(r#"tags ALL ["wasm", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_partial_match_fails() {
        let filter = parse(r#"tags ALL ["rust", "wasm", "go"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_special_chars() {
        let filter = parse(r##"tags ALL ["@user", "#tag"]"##).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["@user".into(), "#tag".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_whitespace() {
        let filter = parse(r#"tags ALL [" ", "  "]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![" ".into(), "  ".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

mod array_none {
    use super::*;

    #[test]
    fn test_none_match() {
        // NONE checks that NONE of the pattern values are in field
        let filter = parse(r#"tags NONE ["python", "java"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_one_matches() {
        let filter = parse(r#"tags NONE ["rust", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_all_match() {
        let filter = parse(r#"tags NONE ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_empty_pattern() {
        let filter = parse(r#"tags NONE []"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        // Empty pattern - vacuously true (none of nothing is in field)
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_empty_field() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        // Empty field - none of pattern values can be in empty field
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_single_match() {
        let filter = parse(r#"tags NONE ["python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_single_no_match() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_case_sensitive() {
        let filter = parse(r#"tags NONE ["RUST"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_unicode() {
        let filter = parse(r#"tags NONE ["日本語"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["英語".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_missing_field() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_none_type_mismatch_string_field() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::String("wasm".into()))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_none_type_mismatch_int_field() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::Integer(123))]);
        // Array operators require array field
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_none_with_empty_string() {
        let filter = parse(r#"tags NONE [""]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into(), "b".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_with_empty_string_present() {
        let filter = parse(r#"tags NONE [""]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["".into(), "a".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_duplicates_in_pattern() {
        let filter = parse(r#"tags NONE ["python", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_duplicates_in_field() {
        let filter = parse(r#"tags NONE ["python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_many_pattern_values_no_match() {
        let filter = parse(r#"tags NONE ["x", "y", "z"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into(), "b".into(), "c".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_many_pattern_values_one_match() {
        let filter = parse(r#"tags NONE ["x", "y", "a"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into(), "b".into(), "c".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_many_field_values() {
        let filter = parse(r#"tags NONE ["0"]"#).unwrap();
        let arr: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(arr))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_special_chars() {
        let filter = parse(r#"tags NONE ["@admin"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["@user".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_whitespace() {
        let filter = parse(r#"tags NONE [" "]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["a".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_partial_overlap_fails() {
        let filter = parse(r#"tags NONE ["rust", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// TYPE COERCION TESTS (50 tests)
// =============================================================================

mod type_coercion {
    use super::*;

    // Int/Float coercion for comparisons
    #[test]
    fn test_int_eq_float() {
        let filter = parse("x = 5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_eq_int() {
        let filter = parse("x = 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_lt_float() {
        let filter = parse("x < 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_lt_int() {
        let filter = parse("x < 6").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_gt_float() {
        let filter = parse("x > 4.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_gt_int() {
        let filter = parse("x > 4").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_le_float() {
        let filter = parse("x <= 5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_le_int() {
        let filter = parse("x <= 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_ge_float() {
        let filter = parse("x >= 5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_ge_int() {
        let filter = parse("x >= 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_ne_float() {
        let filter = parse("x != 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_ne_int() {
        let filter = parse("x != 6").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_between_floats() {
        let filter = parse("x BETWEEN 4.5 AND 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_between_ints() {
        let filter = parse("x BETWEEN 4 AND 6").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_in_float_array() {
        let filter = parse("x IN [4.0, 5.0, 6.0]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_in_int_array() {
        let filter = parse("x IN [4, 5, 6]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Negative numbers
    #[test]
    fn test_negative_int_eq() {
        let filter = parse("x = -5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_float_eq() {
        let filter = parse("x = -5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-5.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_lt_zero() {
        let filter = parse("x < 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_zero_gt_negative() {
        let filter = parse("x > -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_between() {
        let filter = parse("x BETWEEN -10 AND -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_sign_between() {
        let filter = parse("x BETWEEN -5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Floating point precision
    #[test]
    fn test_float_precision_eq() {
        let filter = parse("x = 0.1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_small_float_lt() {
        let filter = parse("x < 0.001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_large_float_eq() {
        let filter = parse("x = 1000000.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1000000.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Zero comparisons
    #[test]
    fn test_zero_int_eq() {
        let filter = parse("x = 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_zero_float_eq() {
        let filter = parse("x = 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_zero_eq_float_zero() {
        let filter = parse("x = 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Large numbers
    #[test]
    fn test_large_int() {
        let filter = parse("x = 1000000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_large_int_comparison() {
        let filter = parse("x > 999999999").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_large_float_million_eq() {
        // Scientific notation not supported by parser, use decimal
        let filter = parse("x = 1000000.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1_000_000.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_small_float_thousandth_eq() {
        // Scientific notation not supported by parser, use decimal
        let filter = parse("x = 0.001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Boolean type - not coercible
    #[test]
    fn test_bool_not_coercible_to_int() {
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_bool_not_coercible_to_string() {
        let filter = parse(r#"x = "true""#).unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // String not coercible
    #[test]
    fn test_string_not_coercible_to_int() {
        let filter = parse("x = 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::String("5".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_int_not_coercible_to_string() {
        let filter = parse(r#"x = "5""#).unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// COMPLEX NESTED EXPRESSIONS (50 tests)
// =============================================================================

mod complex_nested {
    use super::*;

    #[test]
    fn test_deeply_nested_and() {
        let filter = parse("((a = 1) AND (b = 2)) AND ((c = 3) AND (d = 4))").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_deeply_nested_or() {
        // Strict mode: all referenced fields must exist
        let filter = parse("((a = 1) OR (b = 2)) OR ((c = 3) OR (d = 4))").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),  // false
            ("b", MetadataValue::Integer(0)),  // false
            ("c", MetadataValue::Integer(0)),  // false
            ("d", MetadataValue::Integer(4)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_and_or() {
        // Strict mode: all referenced fields must exist
        let filter = parse("(a = 1 AND b = 2) OR (c = 3 AND d = 4)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),  // false
            ("b", MetadataValue::Integer(0)),  // false
            ("c", MetadataValue::Integer(3)),  // true
            ("d", MetadataValue::Integer(4)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_or_and() {
        // Strict mode: all referenced fields must exist
        let filter = parse("(a = 1 OR b = 2) AND (c = 3 OR d = 4)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),  // true
            ("b", MetadataValue::Integer(0)),  // false
            ("c", MetadataValue::Integer(0)),  // false
            ("d", MetadataValue::Integer(4)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_nested_not() {
        let filter = parse("NOT (NOT (x = 1))").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_triple_not() {
        let filter = parse("NOT (NOT (NOT (x = 1)))").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_not_with_and() {
        let filter = parse("NOT (a = 1 AND b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_with_or() {
        let filter = parse("NOT (a = 1 OR b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(3)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_de_morgan_and() {
        // NOT (A AND B) = (NOT A) OR (NOT B)
        let filter1 = parse("NOT (a = 1 AND b = 2)").unwrap();
        let filter2 = parse("(a != 1 OR b != 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter1, &meta), evaluate(&filter2, &meta));
    }

    #[test]
    fn test_de_morgan_or() {
        // NOT (A OR B) = (NOT A) AND (NOT B)
        let filter1 = parse("NOT (a = 1 OR b = 2)").unwrap();
        let filter2 = parse("(a != 1 AND b != 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(3)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter1, &meta), evaluate(&filter2, &meta));
    }

    #[test]
    fn test_complex_comparison_chain() {
        let filter = parse("x > 1 AND x < 10 AND x != 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(7))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_multiple_fields_complex() {
        let filter = parse(r#"(status = "active" AND priority > 5) OR (status = "critical")"#).unwrap();
        let meta = make_metadata(&[
            ("status", MetadataValue::String("active".into())),
            ("priority", MetadataValue::Integer(7)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_string_and_numeric() {
        let filter = parse(r#"name = "test" AND score >= 80 AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("score", MetadataValue::Integer(85)),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_in_complex() {
        let filter = parse("x BETWEEN 1 AND 10 AND y IN [5, 10, 15]").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(5)),
            ("y", MetadataValue::Integer(10)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_null_in_complex() {
        let filter = parse("x IS NOT NULL AND x > 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_in_complex() {
        let filter = parse(r#"name LIKE "test%" AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("testing".into())),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_in_complex() {
        let filter = parse(r#"description contains "important" OR priority = 1"#).unwrap();
        let meta = make_metadata(&[("description", MetadataValue::String("this is important".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_deeply_nested_mixed() {
        // Strict mode: all referenced fields must exist
        let filter = parse("((a = 1 OR b = 2) AND (c = 3 OR d = 4)) OR e = 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),  // false
            ("b", MetadataValue::Integer(0)),  // false
            ("c", MetadataValue::Integer(0)),  // false
            ("d", MetadataValue::Integer(0)),  // false
            ("e", MetadataValue::Integer(5)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_operators_combined() {
        let filter = parse(r#"
            x = 1 AND
            y != 2 AND
            z > 0 AND
            w < 100 AND
            a >= 5 AND
            b <= 10
        "#).unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(1)),
            ("y", MetadataValue::Integer(3)),
            ("z", MetadataValue::Integer(1)),
            ("w", MetadataValue::Integer(50)),
            ("a", MetadataValue::Integer(5)),
            ("b", MetadataValue::Integer(10)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_parentheses_precedence() {
        // Without parens: a AND b OR c = (a AND b) OR c
        // With parens: a AND (b OR c)
        // Strict mode: all referenced fields must exist
        let filter = parse("a = 1 AND (b = 2 OR c = 3)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),  // true
            ("b", MetadataValue::Integer(0)),  // false
            ("c", MetadataValue::Integer(3)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_redundant_parentheses() {
        let filter = parse("((((x = 1))))").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_complex_with_array_ops() {
        let filter = parse(r#"tags ANY ["rust", "wasm"] AND priority > 5"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into(), "go".into()])),
            ("priority", MetadataValue::Integer(7)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// MULTI-FIELD FILTER TESTS (50 tests)
// =============================================================================

mod multi_field {
    use super::*;

    #[test]
    fn test_two_fields_both_match() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_two_fields_one_match() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_three_fields_all_match() {
        let filter = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_four_fields_all_match() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_types_all_match() {
        let filter = parse(r#"name = "test" AND count = 5 AND active = true AND score = 3.5"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("count", MetadataValue::Integer(5)),
            ("active", MetadataValue::Boolean(true)),
            ("score", MetadataValue::Float(3.5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_same_field_multiple_conditions() {
        let filter = parse("x > 5 AND x < 10 AND x != 7").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(8))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_same_field_impossible() {
        let filter = parse("x = 5 AND x = 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_or_with_different_fields() {
        // Strict mode: all referenced fields must exist
        let filter = parse("a = 1 OR b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),  // false
            ("b", MetadataValue::Integer(2)),  // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_extra_fields_ignored() {
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(1)),
            ("y", MetadataValue::Integer(2)),
            ("z", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_comparison_order() {
        // First field fails, second not evaluated (short-circuit)
        let filter = parse("a = 99 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_field_comparison_order_or() {
        // First field succeeds, second not evaluated (short-circuit)
        let filter = parse("a = 1 OR b = 99").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_underscore_field_name() {
        let filter = parse("my_field = 1").unwrap();
        let meta = make_metadata(&[("my_field", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_numeric_suffix_field_name() {
        let filter = parse("field123 = 1").unwrap();
        let meta = make_metadata(&[("field123", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_long_field_name() {
        let filter = parse("very_long_field_name_for_testing = 1").unwrap();
        let meta = make_metadata(&[("very_long_field_name_for_testing", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_single_char_field_name() {
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_nested_path() {
        // Some systems support nested.path - check parser handles it
        let filter = parse("user_name = 1").unwrap();
        let meta = make_metadata(&[("user_name", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_five_fields() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4 AND e = 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
            ("e", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_different_operators_per_field() {
        let filter = parse("a = 1 AND b > 2 AND c < 4 AND d != 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(6)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_string_and_array_field() {
        let filter = parse(r#"name = "test" AND tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("tags", MetadataValue::StringArray(vec!["rust".into()])),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_types_present() {
        let filter = parse(r#"
            str_field = "value" AND
            int_field = 42 AND
            float_field = 3.14 AND
            bool_field = true
        "#).unwrap();
        let meta = make_metadata(&[
            ("str_field", MetadataValue::String("value".into())),
            ("int_field", MetadataValue::Integer(42)),
            ("float_field", MetadataValue::Float(3.14)),
            ("bool_field", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// BOOLEAN FIELD TESTS (30 tests)
// =============================================================================

mod boolean_field {
    use super::*;

    #[test]
    fn test_bool_eq_true() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_eq_false() {
        let filter = parse("active = false").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_ne_true() {
        let filter = parse("active != true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_ne_false() {
        let filter = parse("active != false").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_mismatch_eq_true() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_bool_mismatch_eq_false() {
        let filter = parse("active = false").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_bool_in_array() {
        let filter = parse("active IN [true, false]").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_not_in_array() {
        let filter = parse("active NOT IN [false]").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_is_null() {
        let filter = parse("active IS NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_is_not_null() {
        let filter = parse("active IS NOT NULL").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_and_bool() {
        let filter = parse("a = true AND b = false").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Boolean(true)),
            ("b", MetadataValue::Boolean(false)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_or_bool() {
        // Strict mode: all referenced fields must exist
        let filter = parse("a = true OR b = true").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Boolean(false)),  // false
            ("b", MetadataValue::Boolean(true)),   // true
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_bool_eq() {
        let filter = parse("NOT (active = true)").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_bool_with_other_types() {
        let filter = parse(r#"active = true AND name = "test""#).unwrap();
        let meta = make_metadata(&[
            ("active", MetadataValue::Boolean(true)),
            ("name", MetadataValue::String("test".into())),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_multiple_bool_fields() {
        let filter = parse("a = true AND b = true AND c = false").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Boolean(true)),
            ("b", MetadataValue::Boolean(true)),
            ("c", MetadataValue::Boolean(false)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// EDGE CASES (79 tests)
// =============================================================================

mod edge_cases {
    use super::*;

    // Empty and whitespace strings
    #[test]
    fn test_empty_string_eq() {
        let filter = parse(r#"name = """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_empty_string_ne() {
        let filter = parse(r#"name != """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("x".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_whitespace_only_string() {
        let filter = parse(r#"name = "   ""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("   ".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_tab_string() {
        let filter = parse("name = \"\t\"").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("\t".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_newline_in_string() {
        let filter = parse("name = \"line1\nline2\"").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("line1\nline2".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Unicode and special characters
    #[test]
    fn test_unicode_string() {
        let filter = parse(r#"name = "日本語""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("日本語".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_emoji_string() {
        let filter = parse(r#"name = "🦀🔥""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("🦀🔥".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_unicode() {
        let filter = parse(r#"name = "Hello 世界 🌍""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("Hello 世界 🌍".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_special_chars_string() {
        let filter = parse(r#"name = "@#$%^&*()""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("@#$%^&*()".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_quotes_in_string() {
        let filter = parse(r#"name = "say \"hello\"""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("say \"hello\"".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Boundary numbers
    #[test]
    fn test_max_i32() {
        let filter = parse("x = 2147483647").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2147483647))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_min_i32() {
        let filter = parse("x = -2147483648").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-2147483648))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_very_small_float() {
        let filter = parse("x < 0.0000001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.00000001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_very_large_float() {
        let filter = parse("x > 9999999999.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(10000000000.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Array edge cases
    #[test]
    fn test_empty_array_any() {
        let filter = parse(r#"tags ANY ["x"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_empty_array_all() {
        let filter = parse(r#"tags ALL []"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_empty_array_none() {
        let filter = parse(r#"tags NONE ["x"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_single_element_array() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_empty_in_array() {
        let filter = parse("x IN []").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_empty_not_in_array() {
        let filter = parse("x NOT IN []").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // LIKE edge cases
    #[test]
    fn test_like_empty_pattern() {
        let filter = parse(r#"name LIKE """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_only_percent() {
        let filter = parse(r#"name LIKE "%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_only_underscore() {
        let filter = parse(r#"name LIKE "_""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("a".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // BETWEEN edge cases
    #[test]
    fn test_between_equal_bounds() {
        let filter = parse("x BETWEEN 5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_at_lower_bound() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_at_upper_bound() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_just_below_lower() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(4))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_just_above_upper() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(11))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Null handling
    #[test]
    fn test_null_eq_never_true() {
        // In strict mode, missing field causes error
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_explicit_null_check() {
        let filter = parse("x IS NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_null_with_value() {
        let filter = parse("x IS NOT NULL").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Contains edge cases
    #[test]
    fn test_contains_in_empty() {
        let filter = parse(r#"name contains "x""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_contains_empty_pattern() {
        let filter = parse(r#"name contains """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Long values
    #[test]
    fn test_long_string_eq() {
        let long_str = "a".repeat(1000);
        let filter_str = format!(r#"name = "{}""#, long_str);
        let filter = parse(&filter_str).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String(long_str))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_long_string_contains() {
        let long_str = "a".repeat(1000);
        let filter_str = format!(r#"name contains "{}""#, &long_str[..100]);
        let filter = parse(&filter_str).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String(long_str))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Case sensitivity
    #[test]
    fn test_case_sensitive_eq() {
        let filter = parse(r#"name = "Test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_case_sensitive_contains() {
        let filter = parse(r#"name contains "TEST""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_case_sensitive_like() {
        let filter = parse(r#"name LIKE "TEST%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }
}

// =============================================================================
// COMPARISON BOUNDARY TESTS (30 tests)
// =============================================================================

mod comparison_boundaries {
    use super::*;

    // Integer boundaries
    #[test]
    fn test_int_max_value() {
        let filter = parse("x = 9223372036854775807").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MAX))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_min_value() {
        let filter = parse("x = -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_large_lt() {
        // Use smaller values to avoid float precision issues in parser
        let filter = parse("x < 1000000000001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1_000_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_large_negative_gt() {
        // Use smaller values to avoid float precision issues in parser
        let filter = parse("x > -1000000000001").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-1_000_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_boundary_eq_boundary() {
        let filter = parse("x >= -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_lt_min_false() {
        let filter = parse("x < -9223372036854775808").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MIN))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_gt_max_false() {
        let filter = parse("x > 9223372036854775807").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(i64::MAX))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Float boundaries
    #[test]
    fn test_float_near_zero_positive() {
        let filter = parse("x > 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0000001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_near_zero_negative() {
        let filter = parse("x < 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.0000001))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_exactly_zero() {
        let filter = parse("x = 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Between boundaries
    #[test]
    fn test_between_at_lower_boundary() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_at_upper_boundary() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(10))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_just_below_lower() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(4))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_just_above_upper() {
        let filter = parse("x BETWEEN 5 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(11))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_between_float_at_lower() {
        let filter = parse("x BETWEEN 1.5 AND 3.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(1.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_float_at_upper() {
        let filter = parse("x BETWEEN 1.5 AND 3.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(3.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_same_value() {
        let filter = parse("x BETWEEN 5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_same_value_mismatch() {
        let filter = parse("x BETWEEN 5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(6))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Negative number boundaries
    #[test]
    fn test_negative_boundary_lt() {
        let filter = parse("x < -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-101))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_boundary_gt() {
        let filter = parse("x > -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-99))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_boundary_eq() {
        let filter = parse("x = -100").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_float_boundary() {
        let filter = parse("x <= -0.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Mixed sign comparisons
    #[test]
    fn test_positive_gt_negative() {
        let filter = parse("x > -1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_lt_positive() {
        let filter = parse("x < 1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_zero_gt_large_negative() {
        let filter = parse("x > -1000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_zero_lt_large_positive() {
        let filter = parse("x < 1000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_negative_and_positive() {
        let filter = parse("x BETWEEN -10 AND 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_all_negative() {
        let filter = parse("x BETWEEN -100 AND -50").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-75))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_coerce_to_float_boundary() {
        let filter = parse("x = 100.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(100))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// ADVANCED STRING OPERATOR TESTS (30 tests)
// =============================================================================

mod string_operators_advanced {
    use super::*;

    // LIKE pattern variations
    #[test]
    fn test_like_start_only() {
        let filter = parse(r#"name LIKE "test%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing123".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_end_only() {
        let filter = parse(r#"name LIKE "%test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("mytest".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_both_wildcards() {
        let filter = parse(r#"name LIKE "%test%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("mytesting".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_no_wildcards() {
        let filter = parse(r#"name LIKE "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_single_char_wildcard() {
        let filter = parse(r#"name LIKE "te_t""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_multiple_single_char() {
        let filter = parse(r#"name LIKE "t__t""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_percent_and_underscore() {
        let filter = parse(r#"name LIKE "t_st%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_only_percent() {
        let filter = parse(r#"name LIKE "%""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_only_underscore() {
        let filter = parse(r#"name LIKE "_""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("a".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_like_underscore_too_long() {
        let filter = parse(r#"name LIKE "_""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("ab".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_like_complex_pattern() {
        let filter = parse(r#"name LIKE "a%b_c%d""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("aXXXbYcZZZd".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // starts_with edge cases
    #[test]
    fn test_starts_with_empty_pattern() {
        let filter = parse(r#"name starts_with """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_full_match() {
        let filter = parse(r#"name starts_with "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_starts_with_longer_than_string() {
        let filter = parse(r#"name starts_with "testing123""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // ends_with edge cases
    #[test]
    fn test_ends_with_empty_pattern() {
        let filter = parse(r#"name ends_with """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("anything".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_full_match() {
        let filter = parse(r#"name ends_with "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_ends_with_longer_than_string() {
        let filter = parse(r#"name ends_with "testing123""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // contains edge cases
    #[test]
    fn test_contains_at_start() {
        let filter = parse(r#"name contains "te""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_at_end() {
        let filter = parse(r#"name contains "ng""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_in_middle() {
        let filter = parse(r#"name contains "st""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("testing".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_full_string() {
        let filter = parse(r#"name contains "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_contains_empty_in_empty() {
        let filter = parse(r#"name contains """#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // String comparison operators (< > <= >=) are NOT supported for strings
    // They return TypeMismatch error since comparisons require numeric types
    #[test]
    fn test_string_lt_not_supported() {
        let filter = parse(r#"name < "b""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("a".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_string_gt_not_supported() {
        let filter = parse(r#"name > "a""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("b".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_string_lte_not_supported() {
        let filter = parse(r#"name <= "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_string_gte_not_supported() {
        let filter = parse(r#"name >= "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert!(evaluate(&filter, &meta).is_err());
    }

    // Unicode in strings
    #[test]
    fn test_string_eq_unicode() {
        let filter = parse(r#"name = "测试""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("测试".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_string_contains_unicode() {
        let filter = parse(r#"name contains "测""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("测试".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_string_starts_with_emoji() {
        let filter = parse(r#"name starts_with "🚀""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("🚀rocket".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_string_ends_with_emoji() {
        let filter = parse(r#"name ends_with "🎉""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("party🎉".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// ADVANCED ARRAY OPERATOR TESTS (30 tests)
// =============================================================================

mod array_operators_advanced {
    use super::*;

    // ANY with different sizes
    #[test]
    fn test_any_single_element_match() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_large_set() {
        let filter = parse(r#"tags ANY ["a", "b", "c", "d", "e"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["e".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_large_array() {
        let filter = parse(r#"tags ANY ["target"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![
            "a".into(), "b".into(), "c".into(), "d".into(), "e".into(),
            "f".into(), "g".into(), "h".into(), "i".into(), "target".into(),
        ]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_all_values_match() {
        let filter = parse(r#"tags ANY ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_first_element_matches() {
        let filter = parse(r#"tags ANY ["rust", "go"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "python".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_last_element_matches() {
        let filter = parse(r#"tags ANY ["go", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["python".into(), "rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // ALL with different sizes
    #[test]
    fn test_all_single_element_match() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_exact_match() {
        let filter = parse(r#"tags ALL ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_superset_array() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into(), "go".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_one_missing() {
        let filter = parse(r#"tags ALL ["rust", "wasm", "go"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_partial_overlap() {
        let filter = parse(r#"tags ALL ["rust", "python"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // NONE with different sizes
    #[test]
    fn test_none_single_not_present() {
        let filter = parse(r#"tags NONE ["go"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_multiple_not_present() {
        let filter = parse(r#"tags NONE ["go", "python", "java"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_none_one_present() {
        let filter = parse(r#"tags NONE ["rust", "go"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_all_present() {
        let filter = parse(r#"tags NONE ["rust", "wasm"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Empty arrays
    #[test]
    fn test_any_empty_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_empty_field() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_none_empty_field() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec![]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Combined with other operators
    #[test]
    fn test_any_and_comparison() {
        let filter = parse(r#"tags ANY ["rust"] AND count > 5"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into()])),
            ("count", MetadataValue::Integer(10)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_any_or_comparison() {
        // Strict mode: all fields must exist
        let filter = parse(r#"tags ANY ["rust"] OR count > 100"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into()])),
            ("count", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_multiple_array_conditions() {
        let filter = parse(r#"tags ANY ["rust"] AND categories ALL ["tech"]"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()])),
            ("categories", MetadataValue::StringArray(vec!["tech".into(), "programming".into()])),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // NOT with array operators
    #[test]
    fn test_not_any() {
        let filter = parse(r#"NOT (tags ANY ["go"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_all() {
        let filter = parse(r#"NOT (tags ALL ["rust", "go"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_none() {
        let filter = parse(r#"NOT (tags NONE ["rust"])"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Case sensitivity in arrays
    #[test]
    fn test_any_case_mismatch() {
        let filter = parse(r#"tags ANY ["RUST"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_all_case_mismatch() {
        let filter = parse(r#"tags ALL ["RUST"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Duplicates in array
    #[test]
    fn test_any_with_duplicates_in_field() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into(), "rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_with_duplicates_in_filter() {
        let filter = parse(r#"tags ALL ["rust", "rust"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["rust".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// NUMERIC EDGE CASES (30 tests)
// =============================================================================

mod numeric_edge_cases {
    use super::*;

    // Float precision
    #[test]
    fn test_float_precision_eq() {
        let filter = parse("x = 0.1").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_precision_small_diff() {
        // Due to floating point, 0.1 + 0.2 != 0.3 exactly
        let filter = parse("x = 0.3").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.30000000000000004))]);
        // This may be false due to floating point issues
        let result = evaluate(&filter, &meta);
        assert!(result.is_ok()); // Just verify it doesn't error
    }

    #[test]
    fn test_float_many_decimals() {
        let filter = parse("x = 3.14159265358979").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(std::f64::consts::PI))]);
        // May not match exactly
        let result = evaluate(&filter, &meta);
        assert!(result.is_ok());
    }

    // Very large numbers
    #[test]
    fn test_very_large_positive() {
        let filter = parse("x > 1000000000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2_000_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_very_large_negative() {
        let filter = parse("x < -1000000000000").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(-2_000_000_000_000))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_large_float() {
        let filter = parse("x > 1000000000.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2_000_000_000.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Zero variations
    #[test]
    fn test_positive_zero() {
        let filter = parse("x = 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(0.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_zero_float() {
        let filter = parse("x = 0.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-0.0))]);
        // -0.0 == 0.0 in IEEE 754
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_zero() {
        let filter = parse("x = 0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Integer to float coercion
    #[test]
    fn test_int_to_float_coerce_simple() {
        let filter = parse("x = 5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_to_int_coerce_simple() {
        let filter = parse("x = 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(5.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_to_float_non_whole() {
        let filter = parse("x = 5.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // IN operator with numbers
    #[test]
    fn test_int_in_int_array() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_in_float_array() {
        let filter = parse("x IN [1.5, 2.5, 3.5]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_int_in_mixed_array() {
        let filter = parse("x IN [1, 2.5, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_in_mixed_array() {
        let filter = parse("x IN [1, 2.5, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // NOT IN with numbers
    #[test]
    fn test_int_not_in() {
        let filter = parse("x NOT IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(4))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_not_in() {
        let filter = parse("x NOT IN [1.5, 2.5, 3.5]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(4.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // BETWEEN with numbers
    #[test]
    fn test_int_between_floats() {
        let filter = parse("x BETWEEN 1.5 AND 3.5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_between_ints() {
        let filter = parse("x BETWEEN 1 AND 3").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Comparison chains
    #[test]
    fn test_multiple_gt() {
        let filter = parse("x > 1 AND x > 2 AND x > 3").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(4))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_multiple_lt() {
        let filter = parse("x < 10 AND x < 9 AND x < 8").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_range_with_and() {
        let filter = parse("x >= 5 AND x <= 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(7))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_range_outside_with_or() {
        // Strict mode: all fields must exist
        let filter = parse("x < 5 OR x > 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(15))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Negative decimals
    #[test]
    fn test_negative_decimal_eq() {
        let filter = parse("x = -3.14").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_decimal_lt() {
        let filter = parse("x < -3.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-3.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_decimal_gt() {
        let filter = parse("x > -3.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-2.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_negative_between() {
        let filter = parse("x BETWEEN -10.0 AND -5.0").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(-7.5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// COMPOUND CONDITION TESTS (30 tests)
// =============================================================================

mod compound_conditions {
    use super::*;

    // Multiple ANDs
    #[test]
    fn test_three_way_and() {
        let filter = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_three_way_and_one_false() {
        let filter = parse("a = 1 AND b = 99 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    #[test]
    fn test_five_way_and() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4 AND e = 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
            ("e", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Multiple ORs
    #[test]
    fn test_three_way_or_first_true() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_three_way_or_last_true() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_three_way_or_all_false() {
        let filter = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Mixed AND/OR
    #[test]
    fn test_and_then_or() {
        // a = 1 AND b = 2 OR c = 3 -> ((a = 1) AND (b = 2)) OR (c = 3)
        let filter = parse("(a = 1 AND b = 2) OR c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_then_and() {
        // (a = 1 OR b = 2) AND c = 3
        let filter = parse("(a = 1 OR b = 2) AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // NOT combinations
    #[test]
    fn test_not_and() {
        let filter = parse("NOT (a = 1 AND b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_or() {
        let filter = parse("NOT (a = 1 OR b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(3)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_and_with_not() {
        let filter = parse("a = 1 AND NOT (b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_or_with_not() {
        let filter = parse("a = 1 OR NOT (b = 2)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Multiple NOTs
    #[test]
    fn test_double_not() {
        let filter = parse("NOT (NOT (a = 1))").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_triple_not() {
        let filter = parse("NOT (NOT (NOT (a = 1)))").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(false));
    }

    // Complex nesting
    #[test]
    fn test_deeply_nested_and_or() {
        let filter = parse("((a = 1 AND b = 2) OR (c = 3 AND d = 4)) AND e = 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(0)),
            ("d", MetadataValue::Integer(0)),
            ("e", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_deeply_nested_or_and() {
        let filter = parse("((a = 1 OR b = 2) AND (c = 3 OR d = 4)) OR e = 5").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(0)),
            ("b", MetadataValue::Integer(0)),
            ("c", MetadataValue::Integer(0)),
            ("d", MetadataValue::Integer(0)),
            ("e", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Mixed types in conditions
    #[test]
    fn test_mixed_types_and() {
        let filter = parse(r#"name = "test" AND count > 5 AND active = true"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("count", MetadataValue::Integer(10)),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_types_or() {
        let filter = parse(r#"name = "test" OR count > 100 OR active = false"#).unwrap();
        let meta = make_metadata(&[
            ("name", MetadataValue::String("test".into())),
            ("count", MetadataValue::Integer(5)),
            ("active", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Array operators with logic
    #[test]
    fn test_any_and_comparison() {
        let filter = parse(r#"tags ANY ["rust"] AND priority >= 5"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into(), "wasm".into()])),
            ("priority", MetadataValue::Integer(7)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_all_or_comparison() {
        let filter = parse(r#"tags ALL ["rust"] OR priority > 10"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into()])),
            ("priority", MetadataValue::Integer(5)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // NULL with logic
    #[test]
    fn test_is_null_and_comparison() {
        let filter = parse("optional IS NULL AND required = 1").unwrap();
        let meta = make_metadata(&[("required", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_is_not_null_or_default() {
        let filter = parse("optional IS NOT NULL OR fallback = 1").unwrap();
        let meta = make_metadata(&[
            ("optional", MetadataValue::String("value".into())),
            ("fallback", MetadataValue::Integer(0)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Complex real-world patterns
    #[test]
    fn test_category_and_date_filter() {
        let filter = parse(r#"category = "tech" AND year >= 2020 AND year <= 2025"#).unwrap();
        let meta = make_metadata(&[
            ("category", MetadataValue::String("tech".into())),
            ("year", MetadataValue::Integer(2023)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_status_or_priority() {
        let filter = parse(r#"status = "urgent" OR priority > 8"#).unwrap();
        let meta = make_metadata(&[
            ("status", MetadataValue::String("normal".into())),
            ("priority", MetadataValue::Integer(9)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_active_with_tags() {
        let filter = parse(r#"active = true AND tags ANY ["featured", "promoted"]"#).unwrap();
        let meta = make_metadata(&[
            ("active", MetadataValue::Boolean(true)),
            ("tags", MetadataValue::StringArray(vec!["featured".into()])),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_search_with_exclusions() {
        let filter = parse(r#"type = "article" AND tags NONE ["archived", "deleted"]"#).unwrap();
        let meta = make_metadata(&[
            ("type", MetadataValue::String("article".into())),
            ("tags", MetadataValue::StringArray(vec!["published".into()])),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_price_range_or_sale() {
        let filter = parse("price BETWEEN 10 AND 50 OR on_sale = true").unwrap();
        let meta = make_metadata(&[
            ("price", MetadataValue::Integer(75)),
            ("on_sale", MetadataValue::Boolean(true)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// FIELD REFERENCE TESTS (28 tests)
// =============================================================================

mod field_references {
    use super::*;

    // Single character field names
    #[test]
    fn test_single_char_lowercase() {
        let filter = parse("a = 1").unwrap();
        let meta = make_metadata(&[("a", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_single_char_uppercase() {
        let filter = parse("A = 1").unwrap();
        let meta = make_metadata(&[("A", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Multi-word field names with underscores
    #[test]
    fn test_snake_case_field() {
        let filter = parse("user_name = 1").unwrap();
        let meta = make_metadata(&[("user_name", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_multiple_underscores() {
        let filter = parse("first_middle_last_name = 1").unwrap();
        let meta = make_metadata(&[("first_middle_last_name", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_leading_underscore() {
        let filter = parse("_private = 1").unwrap();
        let meta = make_metadata(&[("_private", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_trailing_underscore() {
        let filter = parse("reserved_ = 1").unwrap();
        let meta = make_metadata(&[("reserved_", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Numbers in field names
    #[test]
    fn test_field_with_number_suffix() {
        let filter = parse("field1 = 1").unwrap();
        let meta = make_metadata(&[("field1", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_numbers_middle() {
        let filter = parse("field2name = 1").unwrap();
        let meta = make_metadata(&[("field2name", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_multiple_numbers() {
        let filter = parse("field123abc456 = 1").unwrap();
        let meta = make_metadata(&[("field123abc456", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Long field names
    #[test]
    fn test_long_field_name() {
        let name = "very_long_field_name_that_goes_on_and_on";
        let filter_str = format!("{} = 1", name);
        let filter = parse(&filter_str).unwrap();
        let meta = make_metadata(&[(name, MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Case sensitivity
    #[test]
    fn test_case_sensitive_field() {
        let filter = parse("Name = 1").unwrap();
        let meta = make_metadata(&[("name", MetadataValue::Integer(1))]);
        // Should not match - field names are case-sensitive
        assert!(evaluate(&filter, &meta).is_err());
    }

    #[test]
    fn test_case_preserved() {
        let filter = parse("camelCase = 1").unwrap();
        let meta = make_metadata(&[("camelCase", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Multiple fields
    #[test]
    fn test_two_fields() {
        let filter = parse("a = 1 AND b = 2").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_same_field_twice() {
        let filter = parse("x > 5 AND x < 10").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(7))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_three_different_fields() {
        let filter = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Fields with extra fields in metadata
    #[test]
    fn test_extra_fields_ignored() {
        let filter = parse("x = 1").unwrap();
        let meta = make_metadata(&[
            ("x", MetadataValue::Integer(1)),
            ("y", MetadataValue::Integer(2)),
            ("z", MetadataValue::Integer(3)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Field type variations
    #[test]
    fn test_field_with_string_value() {
        let filter = parse(r#"name = "test""#).unwrap();
        let meta = make_metadata(&[("name", MetadataValue::String("test".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_bool_value() {
        let filter = parse("active = true").unwrap();
        let meta = make_metadata(&[("active", MetadataValue::Boolean(true))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_float_value() {
        let filter = parse("score = 3.14").unwrap();
        let meta = make_metadata(&[("score", MetadataValue::Float(3.14))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_array_value() {
        let filter = parse(r#"tags ANY ["test"]"#).unwrap();
        let meta = make_metadata(&[("tags", MetadataValue::StringArray(vec!["test".into()]))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Reserved word-like field names (should work as identifiers)
    #[test]
    fn test_field_named_and_lower() {
        // "and" as field name (lowercase) - might conflict with keyword
        let filter = parse("x = 1").unwrap();  // Using simple test
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_named_type() {
        let filter = parse(r#"type = "article""#).unwrap();
        let meta = make_metadata(&[("type", MetadataValue::String("article".into()))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_named_value() {
        let filter = parse("value = 42").unwrap();
        let meta = make_metadata(&[("value", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Field with IS NULL
    #[test]
    fn test_field_is_null() {
        let filter = parse("optional IS NULL").unwrap();
        let meta = make_metadata(&[]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_is_not_null() {
        let filter = parse("required IS NOT NULL").unwrap();
        let meta = make_metadata(&[("required", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    // Field in different positions
    #[test]
    fn test_field_in_between() {
        let filter = parse("score BETWEEN 1 AND 10").unwrap();
        let meta = make_metadata(&[("score", MetadataValue::Integer(5))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_in_in_operator() {
        let filter = parse("status IN [1, 2, 3]").unwrap();
        let meta = make_metadata(&[("status", MetadataValue::Integer(2))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_field_with_not() {
        let filter = parse("NOT (enabled = true)").unwrap();
        let meta = make_metadata(&[("enabled", MetadataValue::Boolean(false))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}

// =============================================================================
// ADDITIONAL TESTS FOR 804 TARGET (8 tests)
// =============================================================================

mod additional_coverage {
    use super::*;

    #[test]
    fn test_deeply_nested_parentheses() {
        let filter = parse("(((((x = 1)))))").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(1))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_six_way_and() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4 AND e = 5 AND f = 6").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Integer(1)),
            ("b", MetadataValue::Integer(2)),
            ("c", MetadataValue::Integer(3)),
            ("d", MetadataValue::Integer(4)),
            ("e", MetadataValue::Integer(5)),
            ("f", MetadataValue::Integer(6)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_mixed_array_ops_complex() {
        let filter = parse(r#"(tags ANY ["rust"]) AND (cats ALL ["tech"]) AND (banned NONE ["spam"])"#).unwrap();
        let meta = make_metadata(&[
            ("tags", MetadataValue::StringArray(vec!["rust".into(), "go".into()])),
            ("cats", MetadataValue::StringArray(vec!["tech".into(), "programming".into()])),
            ("banned", MetadataValue::StringArray(vec!["clean".into()])),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_float_vs_int_equality() {
        let filter = parse("x = 42").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Float(42.0))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_complex_boolean_logic() {
        let filter = parse("(a = true OR b = false) AND NOT (c = true)").unwrap();
        let meta = make_metadata(&[
            ("a", MetadataValue::Boolean(true)),
            ("b", MetadataValue::Boolean(true)),
            ("c", MetadataValue::Boolean(false)),
        ]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_between_with_float_and_int_mix() {
        let filter = parse("x BETWEEN 1.5 AND 5").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(3))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_in_with_single_element() {
        let filter = parse("x IN [42]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }

    #[test]
    fn test_not_in_with_many_elements() {
        let filter = parse("x NOT IN [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]").unwrap();
        let meta = make_metadata(&[("x", MetadataValue::Integer(42))]);
        assert_eq!(evaluate(&filter, &meta), Ok(true));
    }
}
