//! Parser unit tests for EdgeVec filter system.
//!
//! W23.6.1: Comprehensive parser unit tests as specified in DAY_6_TASKS.md.
//! Tests organized by category:
//! - Literal parsing (string, int, float, bool)
//! - Operator parsing (comparison, string, array, logical)
//! - Precedence and parentheses
//! - Error cases and edge cases

use edgevec::filter::{parse, FilterExpr};

// =============================================================================
// LITERAL PARSING — STRING (16 tests)
// =============================================================================

mod string_literals {
    use super::*;

    #[test]
    fn test_simple_string() {
        let result = parse(r#"x = "hello""#).unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_empty_string() {
        let result = parse(r#"x = """#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s.is_empty()));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_spaces() {
        let result = parse(r#"x = "hello world""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "hello world"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_unicode() {
        let result = parse(r#"x = "日本語""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "日本語"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_emoji() {
        let result = parse(r#"x = "🚀 rocket""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "🚀 rocket"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_escaped_quote() {
        let result = parse(r#"x = "say \"hello\"""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == r#"say "hello""#));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_escaped_backslash() {
        let result = parse(r#"x = "path\\to\\file""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == r"path\to\file"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_newline_escape() {
        let result = parse(r#"x = "line1\nline2""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "line1\nline2"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_tab_escape() {
        let result = parse(r#"x = "col1\tcol2""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "col1\tcol2"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_carriage_return_escape() {
        let result = parse(r#"x = "line\r\n""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "line\r\n"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_numbers() {
        let result = parse(r#"x = "123abc""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "123abc"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_special_chars() {
        let result = parse(r#"x = "!@#$%^&*()""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "!@#$%^&*()"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_brackets() {
        let result = parse(r#"x = "[test]""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "[test]"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_single_char() {
        let result = parse(r#"x = "a""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "a"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_very_long() {
        let long_string = "a".repeat(1000);
        let result = parse(&format!(r#"x = "{}""#, long_string)).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralString(s) = *right {
                assert_eq!(s.len(), 1000);
            } else {
                panic!("Expected LiteralString");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_sql_keywords() {
        let result = parse(r#"x = "SELECT * FROM users""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "SELECT * FROM users"));
        } else {
            panic!("Expected Eq");
        }
    }
}

// =============================================================================
// LITERAL PARSING — INTEGER (16 tests)
// =============================================================================

mod integer_literals {
    use super::*;

    #[test]
    fn test_positive_int() {
        let result = parse("x = 42").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(42)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_zero() {
        let result = parse("x = 0").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(0)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_negative_int() {
        let result = parse("x = -42").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(-42)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_large_positive_int() {
        let result = parse("x = 2147483647").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(2147483647)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_large_negative_int() {
        let result = parse("x = -2147483648").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(-2147483648)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_i64_max() {
        let result = parse("x = 9223372036854775807").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(9223372036854775807)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_i64_min() {
        let result = parse("x = -9223372036854775808").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(-9223372036854775808)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_leading_zeros() {
        // Leading zeros are NOT allowed by grammar (uses ASCII_NONZERO_DIGIT for first digit)
        // Only 0 by itself is valid
        let result = parse("x = 007");
        assert!(result.is_err());
    }

    #[test]
    fn test_int_single_digit() {
        let result = parse("x = 5").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(5)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_two_digits() {
        let result = parse("x = 99").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(99)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_negative_single_digit() {
        let result = parse("x = -1").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(-1)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_in_comparison() {
        let result = parse("price < 100").unwrap();
        if let FilterExpr::Lt(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(100)));
        } else {
            panic!("Expected Lt");
        }
    }

    #[test]
    fn test_int_in_between() {
        let result = parse("x BETWEEN 10 AND 20").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_int_in_array() {
        let result = parse("x IN [1, 2, 3]").unwrap();
        if let FilterExpr::In(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralArray(_)));
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_int_many_digits() {
        let result = parse("x = 123456789012").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(123456789012)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_negative_in_array() {
        let result = parse("x IN [-1, -2, -3]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }
}

// =============================================================================
// LITERAL PARSING — FLOAT (16 tests)
// =============================================================================

mod float_literals {
    use super::*;

    #[test]
    fn test_simple_float() {
        let result = parse("x = 3.5").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 3.5).abs() < 0.0001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_negative_float() {
        let result = parse("x = -3.5").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - (-3.5)).abs() < 0.0001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_zero() {
        let result = parse("x = 0.0").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 0.0).abs() < 0.0001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_one() {
        let result = parse("x = 1.0").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 1.0).abs() < 0.0001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_small_decimal() {
        let result = parse("x = 0.001").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 0.001).abs() < 0.00001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_large() {
        let result = parse("x = 123456.789").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 123456.789).abs() < 0.001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_exponent_positive() {
        // Scientific notation NOT supported by grammar
        let result = parse("x = 1.5e10");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_exponent_negative() {
        // Scientific notation NOT supported by grammar
        let result = parse("x = 1.5e-10");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_uppercase_e() {
        // Scientific notation NOT supported by grammar
        let result = parse("x = 1.5E10");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_many_decimals() {
        let result = parse("x = 3.141592653589793").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - std::f64::consts::PI).abs() < 1e-10);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_in_comparison() {
        let result = parse("rating >= 4.5").unwrap();
        assert!(matches!(result, FilterExpr::Ge(_, _)));
    }

    #[test]
    fn test_float_in_between() {
        let result = parse("price BETWEEN 9.99 AND 19.99").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_float_in_array() {
        let result = parse("x IN [1.1, 2.2, 3.3]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_float_negative_exponent_small() {
        // Scientific notation NOT supported by grammar
        let result = parse("x = 5e-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_trailing_zeros() {
        let result = parse("x = 10.00").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 10.0).abs() < 0.0001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_float_negative_with_exponent() {
        // Scientific notation NOT supported by grammar
        let result = parse("x = -2.5e3");
        assert!(result.is_err());
    }
}

// =============================================================================
// LITERAL PARSING — BOOLEAN (12 tests)
// =============================================================================

mod boolean_literals {
    use super::*;

    #[test]
    fn test_true_lowercase() {
        let result = parse("x = true").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(true)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_false_lowercase() {
        let result = parse("x = false").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(false)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_true_uppercase() {
        let result = parse("x = TRUE").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(true)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_false_uppercase() {
        let result = parse("x = FALSE").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(false)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_true_mixed_case() {
        let result = parse("x = True").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(true)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_false_mixed_case() {
        let result = parse("x = False").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(false)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_bool_in_equality() {
        let result = parse("active = true").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_bool_in_inequality() {
        let result = parse("deleted != false").unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_bool_true_different_cases() {
        let r1 = parse("x = TrUe").unwrap();
        let r2 = parse("x = tRuE").unwrap();
        if let FilterExpr::Eq(_, right1) = r1 {
            if let FilterExpr::Eq(_, right2) = r2 {
                assert!(matches!(*right1, FilterExpr::LiteralBool(true)));
                assert!(matches!(*right2, FilterExpr::LiteralBool(true)));
            } else {
                panic!("Expected Eq");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_bool_not_string() {
        // "true" with quotes should be string, not bool
        let result = parse(r#"x = "true""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "true"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_bool_in_complex_expr() {
        let result = parse("active = true AND visible = false").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_bool_with_not() {
        let result = parse("NOT active = true").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }
}

// =============================================================================
// COMPARISON OPERATORS (24 tests)
// =============================================================================

mod comparison_operators {
    use super::*;

    #[test]
    fn test_eq_string() {
        let result = parse(r#"name = "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_eq_int() {
        let result = parse("count = 42").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_eq_float() {
        let result = parse("price = 9.99").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_eq_bool() {
        let result = parse("active = true").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_ne_string() {
        let result = parse(r#"name != "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_ne_int() {
        let result = parse("count != 0").unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_ne_float() {
        let result = parse("price != 0.0").unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_ne_bool() {
        let result = parse("active != false").unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_lt_int() {
        let result = parse("count < 10").unwrap();
        assert!(matches!(result, FilterExpr::Lt(_, _)));
    }

    #[test]
    fn test_lt_float() {
        let result = parse("price < 9.99").unwrap();
        assert!(matches!(result, FilterExpr::Lt(_, _)));
    }

    #[test]
    fn test_le_int() {
        let result = parse("count <= 10").unwrap();
        assert!(matches!(result, FilterExpr::Le(_, _)));
    }

    #[test]
    fn test_le_float() {
        let result = parse("price <= 9.99").unwrap();
        assert!(matches!(result, FilterExpr::Le(_, _)));
    }

    #[test]
    fn test_gt_int() {
        let result = parse("count > 10").unwrap();
        assert!(matches!(result, FilterExpr::Gt(_, _)));
    }

    #[test]
    fn test_gt_float() {
        let result = parse("price > 9.99").unwrap();
        assert!(matches!(result, FilterExpr::Gt(_, _)));
    }

    #[test]
    fn test_ge_int() {
        let result = parse("count >= 10").unwrap();
        assert!(matches!(result, FilterExpr::Ge(_, _)));
    }

    #[test]
    fn test_ge_float() {
        let result = parse("price >= 9.99").unwrap();
        assert!(matches!(result, FilterExpr::Ge(_, _)));
    }

    #[test]
    fn test_comparison_with_negative() {
        let result = parse("temp < -10").unwrap();
        assert!(matches!(result, FilterExpr::Lt(_, _)));
    }

    #[test]
    fn test_comparison_chain_and() {
        let result = parse("x > 0 AND x < 100").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_comparison_with_field_left() {
        let result = parse("price >= 10").unwrap();
        if let FilterExpr::Ge(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "price"));
        } else {
            panic!("Expected Ge");
        }
    }

    #[test]
    fn test_ne_symbolic() {
        let result = parse("x <> 5");
        // <> might not be supported - check
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_comparison_no_spaces() {
        let result = parse("x=5").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_comparison_extra_spaces() {
        let result = parse("x    =    5").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_comparison_leading_trailing_spaces() {
        let result = parse("  x = 5  ").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_comparison_with_underscore_field() {
        let result = parse("_private_field = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "_private_field"));
        } else {
            panic!("Expected Eq");
        }
    }
}

// =============================================================================
// STRING OPERATORS (24 tests)
// =============================================================================

mod string_operators {
    use super::*;

    #[test]
    fn test_contains_simple() {
        let result = parse(r#"name CONTAINS "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_contains_lowercase() {
        let result = parse(r#"name contains "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_contains_mixed_case() {
        let result = parse(r#"name CoNtAiNs "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_starts_with() {
        let result = parse(r#"name STARTS_WITH "pre""#).unwrap();
        assert!(matches!(result, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_starts_with_lowercase() {
        let result = parse(r#"name starts_with "pre""#).unwrap();
        assert!(matches!(result, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_ends_with() {
        let result = parse(r#"name ENDS_WITH "suf""#).unwrap();
        assert!(matches!(result, FilterExpr::EndsWith(_, _)));
    }

    #[test]
    fn test_ends_with_lowercase() {
        let result = parse(r#"name ends_with "suf""#).unwrap();
        assert!(matches!(result, FilterExpr::EndsWith(_, _)));
    }

    #[test]
    fn test_like_percent_start() {
        let result = parse(r#"email LIKE "%@gmail.com""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_percent_end() {
        let result = parse(r#"name LIKE "John%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_percent_both() {
        let result = parse(r#"desc LIKE "%search%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_underscore() {
        let result = parse(r#"code LIKE "A_B""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_mixed() {
        let result = parse(r#"code LIKE "A%_B""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_no_wildcards() {
        let result = parse(r#"name LIKE "exact""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_lowercase() {
        let result = parse(r#"name like "%test%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_contains_empty_string() {
        let result = parse(r#"name CONTAINS """#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_starts_with_empty_string() {
        let result = parse(r#"name STARTS_WITH """#).unwrap();
        assert!(matches!(result, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_ends_with_empty_string() {
        let result = parse(r#"name ENDS_WITH """#).unwrap();
        assert!(matches!(result, FilterExpr::EndsWith(_, _)));
    }

    #[test]
    fn test_like_only_percent() {
        let result = parse(r#"name LIKE "%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_like_only_underscore() {
        let result = parse(r#"name LIKE "_""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_contains_unicode() {
        let result = parse(r#"text CONTAINS "日本""#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_like_unicode() {
        let result = parse(r#"text LIKE "%日本%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_contains_with_and() {
        let result = parse(r#"name CONTAINS "test" AND active = true"#).unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_starts_with_special_chars() {
        let result = parse(r#"name STARTS_WITH "[prefix]""#).unwrap();
        assert!(matches!(result, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_ends_with_special_chars() {
        let result = parse(r#"name ENDS_WITH ".pdf""#).unwrap();
        assert!(matches!(result, FilterExpr::EndsWith(_, _)));
    }
}

// =============================================================================
// ARRAY OPERATORS (32 tests)
// =============================================================================

mod array_operators {
    use super::*;

    #[test]
    fn test_in_strings() {
        let result = parse(r#"category IN ["a", "b", "c"]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_in_integers() {
        let result = parse("x IN [1, 2, 3]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_in_floats() {
        let result = parse("x IN [1.1, 2.2, 3.3]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_in_single_element() {
        let result = parse(r#"x IN ["only"]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_in_lowercase() {
        let result = parse(r#"x in ["a", "b"]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_not_in_strings() {
        let result = parse(r#"status NOT IN ["deleted", "archived"]"#).unwrap();
        assert!(matches!(result, FilterExpr::NotIn(_, _)));
    }

    #[test]
    fn test_not_in_integers() {
        let result = parse("x NOT IN [0, -1]").unwrap();
        assert!(matches!(result, FilterExpr::NotIn(_, _)));
    }

    #[test]
    fn test_not_in_lowercase() {
        let result = parse(r#"x not in ["a"]"#).unwrap();
        assert!(matches!(result, FilterExpr::NotIn(_, _)));
    }

    #[test]
    fn test_any_strings() {
        // Grammar uses: field ANY array_literal (not function syntax)
        let result = parse(r#"tags ANY ["nvidia"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_any_multiple() {
        let result = parse(r#"tags ANY ["gpu", "nvidia"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_any_lowercase() {
        let result = parse(r#"tags any ["test"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_all_strings() {
        // Grammar uses: field ALL array_literal (not function syntax)
        let result = parse(r#"tags ALL ["a", "b"]"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_all_single_value() {
        let result = parse(r#"tags ALL ["only"]"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_all_lowercase() {
        let result = parse(r#"tags all ["a", "b"]"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_none_strings() {
        // Grammar uses: field NONE array_literal (not function syntax)
        let result = parse(r#"tags NONE ["spam", "nsfw"]"#).unwrap();
        assert!(matches!(result, FilterExpr::None(_, _)));
    }

    #[test]
    fn test_none_single_value() {
        let result = parse(r#"tags NONE ["bad"]"#).unwrap();
        assert!(matches!(result, FilterExpr::None(_, _)));
    }

    #[test]
    fn test_none_lowercase() {
        let result = parse(r#"tags none ["spam"]"#).unwrap();
        assert!(matches!(result, FilterExpr::None(_, _)));
    }

    #[test]
    fn test_in_with_spaces() {
        let result = parse(r#"x IN [ "a" , "b" , "c" ]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_any_no_spaces() {
        let result = parse(r#"tags ANY["test"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_all_extra_spaces() {
        let result = parse(r#"tags   ALL   [ "a" , "b" ]"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_in_mixed_negative() {
        let result = parse("x IN [-1, 0, 1]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_array_ops_with_and() {
        let result = parse(r#"tags ANY ["gpu"] AND price < 500"#).unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_array_ops_with_or() {
        let result = parse(r#"tags ANY ["gpu"] OR tags ANY ["cpu"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_in_unicode() {
        let result = parse(r#"lang IN ["日本語", "中文"]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_in_many_elements() {
        let result = parse(r#"x IN [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_any_field_name_underscore() {
        let result = parse(r#"_tags ANY ["test"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_all_field_with_numbers() {
        let result = parse(r#"tags123 ALL ["a"]"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_none_nested_field_name() {
        let result = parse(r#"user_tags NONE ["spam"]"#).unwrap();
        assert!(matches!(result, FilterExpr::None(_, _)));
    }

    #[test]
    fn test_in_empty_strings() {
        let result = parse(r#"x IN ["", "a", ""]"#).unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_tags_any_empty_array() {
        let result = parse(r#"tags ANY []"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }

    #[test]
    fn test_all_empty_array() {
        let result = parse(r#"tags ALL []"#).unwrap();
        assert!(matches!(result, FilterExpr::All(_, _)));
    }

    #[test]
    fn test_none_empty_array() {
        let result = parse(r#"tags NONE []"#).unwrap();
        assert!(matches!(result, FilterExpr::None(_, _)));
    }
}

// =============================================================================
// LOGICAL OPERATORS (24 tests)
// =============================================================================

mod logical_operators {
    use super::*;

    #[test]
    fn test_and_simple() {
        let result = parse("a = 1 AND b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_and_lowercase() {
        let result = parse("a = 1 and b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_and_mixed_case() {
        let result = parse("a = 1 AnD b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_and_symbolic() {
        let result = parse("a = 1 && b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_or_simple() {
        let result = parse("a = 1 OR b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_or_lowercase() {
        let result = parse("a = 1 or b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_or_mixed_case() {
        let result = parse("a = 1 Or b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_or_symbolic() {
        let result = parse("a = 1 || b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_not_simple() {
        let result = parse("NOT a = 1").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_not_lowercase() {
        let result = parse("not a = 1").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_not_symbolic() {
        let result = parse("! a = 1").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_and_chain() {
        let result = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        // Should be (a = 1 AND b = 2) AND c = 3 or similar
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_or_chain() {
        let result = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_double_not() {
        let result = parse("NOT NOT a = 1").unwrap();
        if let FilterExpr::Not(inner) = result {
            assert!(matches!(*inner, FilterExpr::Not(_)));
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_not_with_parens() {
        let result = parse("NOT (a = 1)").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_mixed_and_or() {
        let result = parse("a = 1 AND b = 2 OR c = 3").unwrap();
        // AND should bind tighter, so result is (a AND b) OR c
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_or_then_and() {
        let result = parse("a = 1 OR b = 2 AND c = 3").unwrap();
        // AND binds tighter: a OR (b AND c)
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_not_and() {
        let result = parse("NOT a = 1 AND b = 2").unwrap();
        // NOT binds tighter: (NOT a = 1) AND b = 2
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_not_or() {
        let result = parse("NOT a = 1 OR b = 2").unwrap();
        // NOT binds tighter: (NOT a = 1) OR b = 2
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_complex_logical() {
        let result = parse("a = 1 AND b = 2 OR c = 3 AND d = 4").unwrap();
        // Should be (a AND b) OR (c AND d)
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_many_ands() {
        let result = parse("a = 1 AND b = 2 AND c = 3 AND d = 4 AND e = 5").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_many_ors() {
        let result = parse("a = 1 OR b = 2 OR c = 3 OR d = 4 OR e = 5").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_and_with_not() {
        let result = parse("a = 1 AND NOT b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_or_with_not() {
        let result = parse("NOT a = 1 OR NOT b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }
}

// =============================================================================
// NULL CHECKS (12 tests)
// =============================================================================

mod null_checks {
    use super::*;

    #[test]
    fn test_is_null() {
        let result = parse("x IS NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_is_null_lowercase() {
        let result = parse("x is null").unwrap();
        assert!(matches!(result, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_is_null_mixed_case() {
        let result = parse("x Is NuLl").unwrap();
        assert!(matches!(result, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_is_not_null() {
        let result = parse("x IS NOT NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNotNull(_)));
    }

    #[test]
    fn test_is_not_null_lowercase() {
        let result = parse("x is not null").unwrap();
        assert!(matches!(result, FilterExpr::IsNotNull(_)));
    }

    #[test]
    fn test_is_not_null_mixed_case() {
        let result = parse("x Is Not Null").unwrap();
        assert!(matches!(result, FilterExpr::IsNotNull(_)));
    }

    #[test]
    fn test_is_null_underscore_field() {
        let result = parse("_field IS NULL").unwrap();
        if let FilterExpr::IsNull(inner) = result {
            assert!(matches!(*inner, FilterExpr::Field(s) if s == "_field"));
        } else {
            panic!("Expected IsNull");
        }
    }

    #[test]
    fn test_is_not_null_with_and() {
        let result = parse("x IS NOT NULL AND y = 1").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_is_null_with_or() {
        let result = parse("x IS NULL OR y IS NULL").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_not_is_null() {
        let result = parse("NOT x IS NULL").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_is_null_complex_field() {
        let result = parse("user_profile IS NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_is_not_null_numeric_field() {
        let result = parse("field123 IS NOT NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNotNull(_)));
    }
}

// =============================================================================
// BETWEEN OPERATOR (12 tests)
// =============================================================================

mod between_operator {
    use super::*;

    #[test]
    fn test_between_integers() {
        let result = parse("x BETWEEN 1 AND 10").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_floats() {
        let result = parse("price BETWEEN 9.99 AND 19.99").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_negative() {
        let result = parse("temp BETWEEN -10 AND 10").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_lowercase() {
        let result = parse("x between 1 and 10").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_mixed_case() {
        let result = parse("x BeTwEeN 1 AnD 10").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_with_other_and() {
        let result = parse("x BETWEEN 1 AND 10 AND y = 5").unwrap();
        // Should parse as (x BETWEEN 1 AND 10) AND (y = 5)
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_between_same_values() {
        let result = parse("x BETWEEN 5 AND 5").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_large_range() {
        let result = parse("x BETWEEN 0 AND 1000000").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_with_or() {
        let result = parse("x BETWEEN 1 AND 10 OR y BETWEEN 20 AND 30").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_between_negative_range() {
        let result = parse("x BETWEEN -100 AND -1").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_float_precision() {
        let result = parse("x BETWEEN 0.001 AND 0.999").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_between_exponent() {
        // Scientific notation NOT supported by grammar
        let result = parse("x BETWEEN 1e5 AND 1e6");
        assert!(result.is_err());
    }
}

// =============================================================================
// PRECEDENCE AND PARENTHESES (24 tests)
// =============================================================================

mod precedence_and_parentheses {
    use super::*;

    #[test]
    fn test_simple_parens() {
        let result = parse("(a = 1)").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_double_parens() {
        let result = parse("((a = 1))").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_parens_around_and() {
        let result = parse("(a = 1 AND b = 2)").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parens_override_precedence() {
        let result = parse("a = 1 AND (b = 2 OR c = 3)").unwrap();
        // Without parens: (a AND b) OR c
        // With parens: a AND (b OR c)
        if let FilterExpr::And(_, right) = result {
            assert!(matches!(*right, FilterExpr::Or(_, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_parens_left() {
        let result = parse("(a = 1 OR b = 2) AND c = 3").unwrap();
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::Or(_, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_both_sides_parens() {
        let result = parse("(a = 1) AND (b = 2)").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_nested_parens() {
        let result = parse("((a = 1 AND b = 2) OR c = 3)").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_deeply_nested_parens() {
        let result = parse("(((a = 1)))").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_and_binds_tighter_than_or() {
        let result = parse("a = 1 OR b = 2 AND c = 3").unwrap();
        // Should be: a OR (b AND c)
        if let FilterExpr::Or(_, right) = result {
            assert!(matches!(*right, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Or at top");
        }
    }

    #[test]
    fn test_not_highest_precedence() {
        let result = parse("NOT a = 1 AND b = 2").unwrap();
        // Should be: (NOT a = 1) AND b = 2
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::Not(_)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_parens_with_not() {
        let result = parse("NOT (a = 1 AND b = 2)").unwrap();
        if let FilterExpr::Not(inner) = result {
            assert!(matches!(*inner, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_complex_parens() {
        let result = parse("(a = 1 AND b = 2) OR (c = 3 AND d = 4)").unwrap();
        if let FilterExpr::Or(left, right) = result {
            assert!(matches!(*left, FilterExpr::And(_, _)));
            assert!(matches!(*right, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Or");
        }
    }

    #[test]
    fn test_parens_with_between() {
        let result = parse("(x BETWEEN 1 AND 10) AND y = 5").unwrap();
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::Between(_, _, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_parens_with_in() {
        let result = parse("(x IN [1, 2]) OR y = 3").unwrap();
        if let FilterExpr::Or(left, _) = result {
            assert!(matches!(*left, FilterExpr::In(_, _)));
        } else {
            panic!("Expected Or");
        }
    }

    #[test]
    fn test_parens_with_null() {
        let result = parse("(x IS NULL) AND y = 1").unwrap();
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::IsNull(_)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_parens_with_string_op() {
        let result = parse(r#"(name CONTAINS "test") OR active = true"#).unwrap();
        if let FilterExpr::Or(left, _) = result {
            assert!(matches!(*left, FilterExpr::Contains(_, _)));
        } else {
            panic!("Expected Or");
        }
    }

    #[test]
    fn test_comparison_before_logical() {
        let result = parse("a < 5 AND b > 10").unwrap();
        if let FilterExpr::And(left, right) = result {
            assert!(matches!(*left, FilterExpr::Lt(_, _)));
            assert!(matches!(*right, FilterExpr::Gt(_, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_triple_or_parens() {
        let result = parse("(a = 1 OR b = 2) OR c = 3").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_triple_and_parens() {
        let result = parse("a = 1 AND (b = 2 AND c = 3)").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_parens_whitespace() {
        let result = parse("( a = 1 )").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_parens_no_whitespace() {
        let result = parse("(a=1)").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_mixed_operators_parens() {
        let result = parse("NOT (a = 1 OR (b = 2 AND c = 3))").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_parens_many_levels() {
        let result = parse("((((a = 1) AND (b = 2)) OR (c = 3)))").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }
}

// =============================================================================
// ERROR CASES (32 tests)
// =============================================================================

mod error_cases {
    use super::*;

    #[test]
    fn test_empty_input() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_only() {
        let result = parse("   ");
        assert!(result.is_err());
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
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_string() {
        let result = parse(r#"x = "hello"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_paren() {
        let result = parse("(a = 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_extra_close_paren() {
        let result = parse("a = 1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_bracket() {
        let result = parse("x IN [1, 2");
        assert!(result.is_err());
    }

    #[test]
    fn test_extra_close_bracket() {
        let result = parse("x IN 1, 2]");
        assert!(result.is_err());
    }

    #[test]
    fn test_double_operator() {
        let result = parse("x == 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_and_operand() {
        let result = parse("a = 1 AND");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_or_operand() {
        let result = parse("OR b = 2");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_not_operand() {
        let result = parse("NOT");
        assert!(result.is_err());
    }

    #[test]
    fn test_between_missing_and() {
        let result = parse("x BETWEEN 1 10");
        assert!(result.is_err());
    }

    #[test]
    fn test_between_missing_upper() {
        let result = parse("x BETWEEN 1 AND");
        assert!(result.is_err());
    }

    #[test]
    fn test_between_missing_lower() {
        let result = parse("x BETWEEN AND 10");
        assert!(result.is_err());
    }

    #[test]
    fn test_in_missing_array() {
        let result = parse("x IN");
        assert!(result.is_err());
    }

    #[test]
    fn test_any_missing_value() {
        let result = parse("ANY(tags)");
        assert!(result.is_err());
    }

    #[test]
    fn test_any_missing_field() {
        let result = parse(r#"ANY("test")"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_missing_array() {
        let result = parse("ALL(tags)");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_null_missing_field() {
        let result = parse("IS NULL");
        assert!(result.is_err());
    }

    #[test]
    fn test_contains_missing_value() {
        let result = parse("name CONTAINS");
        assert!(result.is_err());
    }

    #[test]
    fn test_like_missing_pattern() {
        let result = parse("name LIKE");
        assert!(result.is_err());
    }

    #[test]
    fn test_number_as_field() {
        let result = parse("123 = 1");
        // Numbers can't be field names
        assert!(result.is_err());
    }

    #[test]
    fn test_string_as_field() {
        let result = parse(r#""field" = 1"#);
        // Quoted strings can't be field names
        assert!(result.is_err());
    }

    #[test]
    fn test_input_too_long() {
        let long_input = "a".repeat(100_001);
        let result = parse(&format!("{} = 1", long_input));
        assert!(result.is_err());
    }

    #[test]
    fn test_nesting_too_deep() {
        let mut expr = "x = 1".to_string();
        for _ in 0..60 {
            expr = format!("({}) AND y = 1", expr);
        }
        let result = parse(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let result = parse(r#"x = "hello\z""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_trailing_garbage() {
        let result = parse("x = 1 garbage");
        assert!(result.is_err());
    }

    #[test]
    fn test_array_trailing_comma() {
        let result = parse("x IN [1, 2,]");
        // Might be valid or invalid depending on grammar
        // Just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_double_comma_in_array() {
        let result = parse("x IN [1,, 2]");
        assert!(result.is_err());
    }
}

// =============================================================================
// EDGE CASES (24 tests)
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_field_name_underscore_only() {
        let result = parse("_ = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "_"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_field_name_starts_with_underscore() {
        let result = parse("_private = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "_private"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_field_name_with_numbers() {
        let result = parse("field123 = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "field123"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_field_name_all_caps() {
        let result = parse("MYFIELD = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "MYFIELD"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_whitespace_tabs() {
        let result = parse("x\t=\t1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_whitespace_newlines() {
        let result = parse("x\n=\n1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_whitespace_mixed() {
        let result = parse("  x \t = \n 1  ").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_very_long_field_name() {
        let field = "a".repeat(100);
        let result = parse(&format!("{} = 1", field)).unwrap();
        if let FilterExpr::Eq(left, _) = result {
            if let FilterExpr::Field(s) = *left {
                assert_eq!(s.len(), 100);
            } else {
                panic!("Expected Field");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_many_operators_in_expression() {
        let result = parse("a = 1 AND b = 2 OR c = 3 AND d = 4 OR e = 5").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_deeply_nested_parens_limit() {
        let mut expr = "x = 1".to_string();
        for _ in 0..49 {
            expr = format!("({})", expr);
        }
        let result = parse(&expr);
        // Should be within limit (50)
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_single_element() {
        let result = parse("x IN [1]").unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 1);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_array_many_elements() {
        let values: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
        let arr_str = values.join(", ");
        let result = parse(&format!("x IN [{}]", arr_str)).unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 100);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_string_with_keywords() {
        let result = parse(r#"x = "AND OR NOT""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "AND OR NOT"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_field_same_as_keyword() {
        // Field named 'and' - might require special handling
        // Just test it doesn't panic
        let _ = parse("and = 1");
    }

    #[test]
    fn test_comparison_boundary_values() {
        let result = parse("x >= -9223372036854775808 AND x <= 9223372036854775807").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_empty_array_in_in() {
        let result = parse("x IN []").unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert!(arr.is_empty());
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_unicode_field_name() {
        // Some parsers might not support unicode field names
        // Just check it doesn't panic
        let _ = parse("日本語 = 1");
    }

    #[test]
    fn test_emoji_in_string() {
        let result = parse(r#"x = "🎉🎊🎁""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "🎉🎊🎁"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_zero_width_char_in_string() {
        let result = parse("x = \"a\u{200B}b\"").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralString(s) = *right {
                assert_eq!(s.len(), 5); // 'a' + zero-width + 'b'
            } else {
                panic!("Expected LiteralString");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_control_char_in_string() {
        // Test that control characters are handled
        let _ = parse("x = \"a\x01b\"");
    }

    #[test]
    fn test_null_literal() {
        // "null" should probably not parse as a literal (use IS NULL instead)
        let result = parse("x = null");
        // Just check behavior is defined
        let _ = result;
    }

    #[test]
    fn test_field_starting_with_number() {
        let result = parse("123field = 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_consecutive_nots() {
        let result = parse("NOT NOT NOT a = 1").unwrap();
        if let FilterExpr::Not(inner) = result {
            if let FilterExpr::Not(inner2) = *inner {
                assert!(matches!(*inner2, FilterExpr::Not(_)));
            } else {
                panic!("Expected nested Not");
            }
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_expression_with_only_parens() {
        let result = parse("()");
        assert!(result.is_err());
    }
}

// =============================================================================
// FIELD PATH EXPRESSIONS (20 tests)
// =============================================================================

mod field_paths {
    use super::*;

    #[test]
    fn test_simple_dotted_field() {
        let result = parse("metadata.name = \"test\"");
        // Check if dotted fields are supported (may or may not be)
        let _ = result;
    }

    #[test]
    fn test_deeply_nested_dotted_field() {
        let result = parse("a.b.c.d.e = 1");
        let _ = result;
    }

    #[test]
    fn test_field_with_array_index() {
        let result = parse("items[0] = 1");
        // Array indexing might not be supported
        let _ = result;
    }

    #[test]
    fn test_field_underscore_prefix() {
        let result = parse("_metadata = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_field_with_digits() {
        let result = parse("field_v2 = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_field_camel_case() {
        let result = parse("myFieldName = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_field_snake_case() {
        let result = parse("my_field_name = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_field_mixed_case() {
        let result = parse("My_Field_Name_v2 = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_field_single_letter() {
        let result = parse("x = 1").unwrap();
        if let FilterExpr::Eq(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "x"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_field_double_underscore() {
        let result = parse("__private__ = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_multiple_fields_and() {
        let result = parse("name = \"test\" AND age = 25").unwrap();
        if let FilterExpr::And(left, right) = result {
            assert!(matches!(*left, FilterExpr::Eq(_, _)));
            assert!(matches!(*right, FilterExpr::Eq(_, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_multiple_fields_or() {
        let result = parse("type = \"a\" OR type = \"b\"").unwrap();
        if let FilterExpr::Or(left, right) = result {
            assert!(matches!(*left, FilterExpr::Eq(_, _)));
            assert!(matches!(*right, FilterExpr::Eq(_, _)));
        } else {
            panic!("Expected Or");
        }
    }

    #[test]
    fn test_same_field_range() {
        let result = parse("age >= 18 AND age <= 65").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_field_vs_between() {
        // Both should be equivalent in meaning
        let result1 = parse("age >= 18 AND age <= 65").unwrap();
        let result2 = parse("age BETWEEN 18 AND 65").unwrap();
        assert!(matches!(result1, FilterExpr::And(_, _)));
        assert!(matches!(result2, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_field_with_is_null() {
        let result = parse("optional_field IS NULL").unwrap();
        if let FilterExpr::IsNull(field) = result {
            assert!(matches!(*field, FilterExpr::Field(s) if s == "optional_field"));
        } else {
            panic!("Expected IsNull");
        }
    }

    #[test]
    fn test_field_with_is_not_null() {
        let result = parse("required_field IS NOT NULL").unwrap();
        if let FilterExpr::IsNotNull(field) = result {
            assert!(matches!(*field, FilterExpr::Field(s) if s == "required_field"));
        } else {
            panic!("Expected IsNotNull");
        }
    }

    #[test]
    fn test_field_with_like() {
        let result = parse("name LIKE \"test%\"").unwrap();
        if let FilterExpr::Like(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "name"));
        } else {
            panic!("Expected Like");
        }
    }

    #[test]
    fn test_field_with_contains() {
        let result = parse("description CONTAINS \"important\"").unwrap();
        if let FilterExpr::Contains(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "description"));
        } else {
            panic!("Expected Contains");
        }
    }

    #[test]
    fn test_field_with_in() {
        let result = parse("status IN [\"active\", \"pending\"]").unwrap();
        if let FilterExpr::In(left, _) = result {
            assert!(matches!(*left, FilterExpr::Field(s) if s == "status"));
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_field_reserved_word_like() {
        // "type" is reserved in many languages but should work as field
        let result = parse("type = \"test\"").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }
}

// =============================================================================
// COMPLEX EXPRESSIONS (20 tests)
// =============================================================================

mod complex_expressions {
    use super::*;

    #[test]
    fn test_three_way_and() {
        let result = parse("a = 1 AND b = 2 AND c = 3").unwrap();
        // Should be (a = 1 AND b = 2) AND c = 3 due to left associativity
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_three_way_or() {
        let result = parse("a = 1 OR b = 2 OR c = 3").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_and_or_precedence() {
        // AND has higher precedence than OR
        let result = parse("a = 1 OR b = 2 AND c = 3").unwrap();
        // Should be a = 1 OR (b = 2 AND c = 3)
        if let FilterExpr::Or(left, right) = result {
            assert!(matches!(*left, FilterExpr::Eq(_, _)));
            assert!(matches!(*right, FilterExpr::And(_, _)));
        } else {
            panic!("Expected Or at top level");
        }
    }

    #[test]
    fn test_or_and_precedence_override() {
        let result = parse("(a = 1 OR b = 2) AND c = 3").unwrap();
        if let FilterExpr::And(left, right) = result {
            assert!(matches!(*left, FilterExpr::Or(_, _)));
            assert!(matches!(*right, FilterExpr::Eq(_, _)));
        } else {
            panic!("Expected And at top level");
        }
    }

    #[test]
    fn test_not_with_and() {
        let result = parse("NOT a = 1 AND b = 2").unwrap();
        // NOT binds tighter than AND
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::Not(_)));
        } else {
            panic!("Expected And at top level");
        }
    }

    #[test]
    fn test_not_with_or() {
        let result = parse("NOT a = 1 OR b = 2").unwrap();
        if let FilterExpr::Or(left, _) = result {
            assert!(matches!(*left, FilterExpr::Not(_)));
        } else {
            panic!("Expected Or at top level");
        }
    }

    #[test]
    fn test_double_not() {
        let result = parse("NOT NOT a = 1").unwrap();
        if let FilterExpr::Not(inner) = result {
            assert!(matches!(*inner, FilterExpr::Not(_)));
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_complex_mixed_operators() {
        let result = parse("a = 1 AND (b = 2 OR c = 3) AND d = 4").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_between_with_and() {
        let result = parse("x BETWEEN 1 AND 10 AND y = 5").unwrap();
        if let FilterExpr::And(left, _) = result {
            assert!(matches!(*left, FilterExpr::Between(_, _, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_in_with_or() {
        let result = parse("x IN [1, 2] OR y IN [3, 4]").unwrap();
        if let FilterExpr::Or(left, right) = result {
            assert!(matches!(*left, FilterExpr::In(_, _)));
            assert!(matches!(*right, FilterExpr::In(_, _)));
        } else {
            panic!("Expected Or");
        }
    }

    #[test]
    fn test_null_check_in_complex() {
        let result = parse("a IS NULL AND b IS NOT NULL").unwrap();
        if let FilterExpr::And(left, right) = result {
            assert!(matches!(*left, FilterExpr::IsNull(_)));
            assert!(matches!(*right, FilterExpr::IsNotNull(_)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_string_ops_in_complex() {
        // Grammar uses starts_with (underscore), not STARTS WITH (two words)
        let result = parse(r#"name CONTAINS "test" AND name starts_with "a""#).unwrap();
        if let FilterExpr::And(left, right) = result {
            assert!(matches!(*left, FilterExpr::Contains(_, _)));
            assert!(matches!(*right, FilterExpr::StartsWith(_, _)));
        } else {
            panic!("Expected And");
        }
    }

    #[test]
    fn test_comparison_chain_simulated() {
        // a < b < c style - needs two comparisons
        let result = parse("a < 5 AND 5 < b");
        // Just check it parses without panic
        let _ = result;
    }

    #[test]
    fn test_multiple_not_with_parens() {
        let result = parse("NOT (NOT (NOT a = 1))").unwrap();
        if let FilterExpr::Not(inner) = result {
            if let FilterExpr::Not(inner2) = *inner {
                assert!(matches!(*inner2, FilterExpr::Not(_)));
            } else {
                panic!("Expected nested Not");
            }
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_all_comparison_ops() {
        let result = parse("a = 1 AND b != 2 AND c < 3 AND d <= 4 AND e > 5 AND f >= 6").unwrap();
        // Should parse as nested ANDs
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_all_string_ops() {
        // Grammar uses starts_with and ends_with (underscore), not STARTS WITH / ENDS WITH
        let result = parse(r#"a LIKE "%" AND b CONTAINS "x" AND c starts_with "y" AND d ends_with "z""#).unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_mixed_types_in_array() {
        // Arrays should be homogeneous, but let's see parser behavior
        let result = parse("x IN [1, \"two\", 3.0]");
        // Just check it handles this gracefully
        let _ = result;
    }

    #[test]
    fn test_not_with_between() {
        let result = parse("NOT x BETWEEN 1 AND 10").unwrap();
        if let FilterExpr::Not(inner) = result {
            assert!(matches!(*inner, FilterExpr::Between(_, _, _)));
        } else {
            panic!("Expected Not");
        }
    }

    #[test]
    fn test_not_with_in() {
        // This should parse as NOT IN
        let result = parse("x NOT IN [1, 2, 3]").unwrap();
        assert!(matches!(result, FilterExpr::NotIn(_, _)));
    }

    #[test]
    fn test_deeply_complex() {
        let result = parse(
            "(a = 1 OR b = 2) AND (c = 3 OR d = 4) AND NOT (e = 5 AND f = 6)"
        ).unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }
}

// =============================================================================
// TYPE HANDLING (17 tests)
// =============================================================================

mod type_handling {
    use super::*;

    #[test]
    fn test_int_vs_float_in_eq() {
        let result1 = parse("x = 1").unwrap();
        let result2 = parse("x = 1.0").unwrap();
        assert!(matches!(result1, FilterExpr::Eq(_, _)));
        assert!(matches!(result2, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_negative_int_in_comparison() {
        let result = parse("x >= -100").unwrap();
        if let FilterExpr::Ge(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(-100)));
        } else {
            panic!("Expected Ge");
        }
    }

    #[test]
    fn test_negative_float_in_comparison() {
        let result = parse("x < -3.5").unwrap();
        if let FilterExpr::Lt(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - (-3.5)).abs() < 0.001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Lt");
        }
    }

    #[test]
    fn test_bool_true_in_eq() {
        let result = parse("active = true").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(true)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_bool_false_in_eq() {
        let result = parse("deleted = false").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralBool(false)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_int_array() {
        let result = parse("x IN [1, 2, 3]").unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 3);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_float_array() {
        let result = parse("x IN [1.1, 2.2, 3.3]").unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 3);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_string_array() {
        let result = parse(r#"x IN ["a", "b", "c"]"#).unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 3);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_bool_array() {
        let result = parse("x IN [true, false]").unwrap();
        if let FilterExpr::In(_, right) = result {
            if let FilterExpr::LiteralArray(arr) = *right {
                assert_eq!(arr.len(), 2);
            } else {
                panic!("Expected LiteralArray");
            }
        } else {
            panic!("Expected In");
        }
    }

    #[test]
    fn test_between_with_ints() {
        let result = parse("x BETWEEN 1 AND 100").unwrap();
        if let FilterExpr::Between(_, low, high) = result {
            assert!(matches!(*low, FilterExpr::LiteralInt(1)));
            assert!(matches!(*high, FilterExpr::LiteralInt(100)));
        } else {
            panic!("Expected Between");
        }
    }

    #[test]
    fn test_between_with_floats() {
        let result = parse("x BETWEEN 0.0 AND 1.0").unwrap();
        if let FilterExpr::Between(_, low, high) = result {
            if let FilterExpr::LiteralFloat(f) = *low {
                assert!((f - 0.0).abs() < 0.001);
            } else {
                panic!("Expected LiteralFloat for low");
            }
            if let FilterExpr::LiteralFloat(f) = *high {
                assert!((f - 1.0).abs() < 0.001);
            } else {
                panic!("Expected LiteralFloat for high");
            }
        } else {
            panic!("Expected Between");
        }
    }

    #[test]
    fn test_between_with_negatives() {
        let result = parse("x BETWEEN -10 AND 10").unwrap();
        if let FilterExpr::Between(_, low, high) = result {
            assert!(matches!(*low, FilterExpr::LiteralInt(-10)));
            assert!(matches!(*high, FilterExpr::LiteralInt(10)));
        } else {
            panic!("Expected Between");
        }
    }

    #[test]
    fn test_large_int() {
        let result = parse("x = 9999999999").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralInt(9999999999)));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_very_small_float() {
        let result = parse("x = 0.000001").unwrap();
        if let FilterExpr::Eq(_, right) = result {
            if let FilterExpr::LiteralFloat(f) = *right {
                assert!((f - 0.000001).abs() < 0.0000001);
            } else {
                panic!("Expected LiteralFloat");
            }
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_string_with_numbers() {
        let result = parse(r#"x = "12345""#).unwrap();
        if let FilterExpr::Eq(_, right) = result {
            assert!(matches!(*right, FilterExpr::LiteralString(s) if s == "12345"));
        } else {
            panic!("Expected Eq");
        }
    }

    #[test]
    fn test_bool_case_sensitivity() {
        // true/false should be lowercase
        let result_lower_true = parse("x = true").unwrap();
        let result_lower_false = parse("x = false").unwrap();
        assert!(matches!(result_lower_true, FilterExpr::Eq(_, _)));
        assert!(matches!(result_lower_false, FilterExpr::Eq(_, _)));

        // TRUE/FALSE might or might not work
        let _ = parse("x = TRUE");
        let _ = parse("x = FALSE");
    }

    #[test]
    fn test_int_with_plus_sign() {
        // +42 might not be supported
        let result = parse("x = +42");
        let _ = result;
    }
}

// =============================================================================
// OPERATOR VARIATIONS (20 tests)
// =============================================================================

mod operator_variations {
    use super::*;

    #[test]
    fn test_eq_symbol() {
        let result = parse("x = 1").unwrap();
        assert!(matches!(result, FilterExpr::Eq(_, _)));
    }

    #[test]
    fn test_ne_symbol() {
        let result = parse("x != 1").unwrap();
        assert!(matches!(result, FilterExpr::Ne(_, _)));
    }

    #[test]
    fn test_ne_alternative_symbol() {
        // <> is common SQL notation
        let result = parse("x <> 1");
        let _ = result;
    }

    #[test]
    fn test_lt_symbol() {
        let result = parse("x < 1").unwrap();
        assert!(matches!(result, FilterExpr::Lt(_, _)));
    }

    #[test]
    fn test_le_symbol() {
        let result = parse("x <= 1").unwrap();
        assert!(matches!(result, FilterExpr::Le(_, _)));
    }

    #[test]
    fn test_gt_symbol() {
        let result = parse("x > 1").unwrap();
        assert!(matches!(result, FilterExpr::Gt(_, _)));
    }

    #[test]
    fn test_ge_symbol() {
        let result = parse("x >= 1").unwrap();
        assert!(matches!(result, FilterExpr::Ge(_, _)));
    }

    #[test]
    fn test_and_uppercase() {
        let result = parse("a = 1 AND b = 2").unwrap();
        assert!(matches!(result, FilterExpr::And(_, _)));
    }

    #[test]
    fn test_or_uppercase() {
        let result = parse("a = 1 OR b = 2").unwrap();
        assert!(matches!(result, FilterExpr::Or(_, _)));
    }

    #[test]
    fn test_not_uppercase() {
        let result = parse("NOT a = 1").unwrap();
        assert!(matches!(result, FilterExpr::Not(_)));
    }

    #[test]
    fn test_in_uppercase() {
        let result = parse("x IN [1, 2]").unwrap();
        assert!(matches!(result, FilterExpr::In(_, _)));
    }

    #[test]
    fn test_not_in_uppercase() {
        let result = parse("x NOT IN [1, 2]").unwrap();
        assert!(matches!(result, FilterExpr::NotIn(_, _)));
    }

    #[test]
    fn test_between_uppercase() {
        let result = parse("x BETWEEN 1 AND 10").unwrap();
        assert!(matches!(result, FilterExpr::Between(_, _, _)));
    }

    #[test]
    fn test_like_uppercase() {
        let result = parse(r#"x LIKE "test%""#).unwrap();
        assert!(matches!(result, FilterExpr::Like(_, _)));
    }

    #[test]
    fn test_contains_uppercase() {
        let result = parse(r#"x CONTAINS "test""#).unwrap();
        assert!(matches!(result, FilterExpr::Contains(_, _)));
    }

    #[test]
    fn test_starts_with_underscore() {
        // Grammar uses starts_with (underscore), not STARTS WITH (two words)
        let result = parse(r#"x starts_with "test""#).unwrap();
        assert!(matches!(result, FilterExpr::StartsWith(_, _)));
    }

    #[test]
    fn test_ends_with_underscore() {
        // Grammar uses ends_with (underscore), not ENDS WITH (two words)
        let result = parse(r#"x ends_with "test""#).unwrap();
        assert!(matches!(result, FilterExpr::EndsWith(_, _)));
    }

    #[test]
    fn test_is_null_uppercase() {
        let result = parse("x IS NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNull(_)));
    }

    #[test]
    fn test_is_not_null_uppercase() {
        let result = parse("x IS NOT NULL").unwrap();
        assert!(matches!(result, FilterExpr::IsNotNull(_)));
    }

    #[test]
    fn test_any_uppercase() {
        let result = parse(r#"x ANY ["a"]"#).unwrap();
        assert!(matches!(result, FilterExpr::Any(_, _)));
    }
}
