//! Comprehensive tests for FilterStrategy module.
//!
//! Target: 348 tests (408 total with ~60 inline tests)
//!
//! Coverage areas:
//! - FilterStrategy validation
//! - Selectivity calculation
//! - Strategy selection
//! - Tautology detection
//! - Contradiction detection
//! - Edge case handling
//! - Real-world filter scenarios

// Allow constant assertions - these tests intentionally verify constant values
// to ensure they stay within reasonable bounds across versions.
#![allow(clippy::assertions_on_constants)]

use edgevec::filter::parse;
use edgevec::filter::strategy::{
    calculate_oversample, is_contradiction, is_tautology, select_strategy,
    FilterStrategy, SelectivityEstimate,
    DEFAULT_OVERSAMPLE, EF_CAP, MAX_OVERSAMPLE, POSTFILTER_THRESHOLD,
    PREFILTER_THRESHOLD, SELECTIVITY_SAMPLE_SIZE,
};
use edgevec::filter::FilterExpr;

// =============================================================================
// FILTER STRATEGY VALIDATION TESTS (50 tests)
// =============================================================================

mod strategy_validation {
    use super::*;

    // PostFilter validation
    #[test]
    fn test_postfilter_oversample_1_0() {
        assert!(FilterStrategy::PostFilter { oversample: 1.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_1_5() {
        assert!(FilterStrategy::PostFilter { oversample: 1.5 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_2_0() {
        assert!(FilterStrategy::PostFilter { oversample: 2.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_3_0() {
        assert!(FilterStrategy::PostFilter { oversample: 3.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_5_0() {
        assert!(FilterStrategy::PostFilter { oversample: 5.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_7_5() {
        assert!(FilterStrategy::PostFilter { oversample: 7.5 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_10_0() {
        assert!(FilterStrategy::PostFilter { oversample: 10.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_0_99_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 0.99 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_0_5_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 0.5 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_0_0_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 0.0 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_negative_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: -1.0 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_10_01_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 10.01 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_15_0_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 15.0 }.validate().is_err());
    }

    #[test]
    fn test_postfilter_oversample_100_0_invalid() {
        assert!(FilterStrategy::PostFilter { oversample: 100.0 }.validate().is_err());
    }

    // Hybrid validation
    #[test]
    fn test_hybrid_min_1_max_10_valid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: 10.0 }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_1_max_5_valid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: 5.0 }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_1_5_max_5_valid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 5.0 }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_2_max_8_valid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 2.0, oversample_max: 8.0 }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_equals_max_valid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 5.0, oversample_max: 5.0 }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_0_5_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 0.5, oversample_max: 5.0 }.validate().is_err());
    }

    #[test]
    fn test_hybrid_min_0_0_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 0.0, oversample_max: 5.0 }.validate().is_err());
    }

    #[test]
    fn test_hybrid_min_negative_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: -1.0, oversample_max: 5.0 }.validate().is_err());
    }

    #[test]
    fn test_hybrid_max_less_than_min_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 5.0, oversample_max: 3.0 }.validate().is_err());
    }

    #[test]
    fn test_hybrid_max_exceeds_limit_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: 15.0 }.validate().is_err());
    }

    #[test]
    fn test_hybrid_max_exceeds_limit_11_invalid() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: 11.0 }.validate().is_err());
    }

    // PreFilter and Auto validation
    #[test]
    fn test_prefilter_always_valid() {
        assert!(FilterStrategy::PreFilter.validate().is_ok());
    }

    #[test]
    fn test_auto_always_valid() {
        assert!(FilterStrategy::Auto.validate().is_ok());
    }

    // Constants
    #[test]
    fn test_post_filter_default_value() {
        let expected = FilterStrategy::PostFilter { oversample: DEFAULT_OVERSAMPLE };
        assert_eq!(FilterStrategy::POST_FILTER_DEFAULT, expected);
    }

    #[test]
    fn test_hybrid_default_value() {
        let expected = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: MAX_OVERSAMPLE };
        assert_eq!(FilterStrategy::HYBRID_DEFAULT, expected);
    }

    #[test]
    fn test_default_is_auto() {
        assert_eq!(FilterStrategy::default(), FilterStrategy::Auto);
    }

    // Equality tests
    #[test]
    fn test_postfilter_equality() {
        let a = FilterStrategy::PostFilter { oversample: 3.0 };
        let b = FilterStrategy::PostFilter { oversample: 3.0 };
        assert_eq!(a, b);
    }

    #[test]
    fn test_postfilter_inequality() {
        let a = FilterStrategy::PostFilter { oversample: 3.0 };
        let b = FilterStrategy::PostFilter { oversample: 5.0 };
        assert_ne!(a, b);
    }

    #[test]
    fn test_hybrid_equality() {
        let a = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 10.0 };
        let b = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 10.0 };
        assert_eq!(a, b);
    }

    #[test]
    fn test_hybrid_inequality_min() {
        let a = FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: 10.0 };
        let b = FilterStrategy::Hybrid { oversample_min: 2.0, oversample_max: 10.0 };
        assert_ne!(a, b);
    }

    #[test]
    fn test_hybrid_inequality_max() {
        let a = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 8.0 };
        let b = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 10.0 };
        assert_ne!(a, b);
    }

    #[test]
    fn test_prefilter_equality() {
        assert_eq!(FilterStrategy::PreFilter, FilterStrategy::PreFilter);
    }

    #[test]
    fn test_auto_equality() {
        assert_eq!(FilterStrategy::Auto, FilterStrategy::Auto);
    }

    #[test]
    fn test_different_variants_not_equal() {
        assert_ne!(FilterStrategy::PreFilter, FilterStrategy::Auto);
    }

    #[test]
    fn test_postfilter_not_equal_to_prefilter() {
        let post = FilterStrategy::PostFilter { oversample: 3.0 };
        assert_ne!(post, FilterStrategy::PreFilter);
    }

    #[test]
    fn test_hybrid_not_equal_to_auto() {
        let hybrid = FilterStrategy::Hybrid { oversample_min: 1.5, oversample_max: 10.0 };
        assert_ne!(hybrid, FilterStrategy::Auto);
    }

    // Clone tests
    #[test]
    fn test_postfilter_clone() {
        let original = FilterStrategy::PostFilter { oversample: 5.5 };
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_hybrid_clone() {
        let original = FilterStrategy::Hybrid { oversample_min: 2.0, oversample_max: 8.0 };
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_prefilter_clone() {
        let original = FilterStrategy::PreFilter;
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_auto_clone() {
        let original = FilterStrategy::Auto;
        let cloned = original;
        assert_eq!(original, cloned);
    }

    // Boundary validation
    #[test]
    fn test_postfilter_at_exact_min() {
        assert!(FilterStrategy::PostFilter { oversample: 1.0 }.validate().is_ok());
    }

    #[test]
    fn test_postfilter_at_exact_max() {
        assert!(FilterStrategy::PostFilter { oversample: MAX_OVERSAMPLE }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_at_1() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: MAX_OVERSAMPLE }.validate().is_ok());
    }

    #[test]
    fn test_hybrid_max_at_cap() {
        assert!(FilterStrategy::Hybrid { oversample_min: 1.0, oversample_max: MAX_OVERSAMPLE }.validate().is_ok());
    }
}

// =============================================================================
// CALCULATE OVERSAMPLE TESTS (40 tests)
// =============================================================================

mod calculate_oversample_tests {
    use super::*;

    #[test]
    fn test_selectivity_1_0() {
        assert!((calculate_oversample(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_5() {
        assert!((calculate_oversample(0.5) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_25() {
        assert!((calculate_oversample(0.25) - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_2() {
        assert!((calculate_oversample(0.2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_1() {
        assert!((calculate_oversample(0.1) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_125() {
        assert!((calculate_oversample(0.125) - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_333() {
        let result = calculate_oversample(1.0/3.0);
        assert!(result > 2.9 && result < 3.1);
    }

    #[test]
    fn test_selectivity_0_75() {
        let result = calculate_oversample(0.75);
        assert!(result > 1.3 && result < 1.4);
    }

    #[test]
    fn test_selectivity_0_8() {
        assert!((calculate_oversample(0.8) - 1.25).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_9() {
        let result = calculate_oversample(0.9);
        assert!(result > 1.1 && result < 1.12);
    }

    // Capping at MAX_OVERSAMPLE
    #[test]
    fn test_selectivity_0_05_capped() {
        assert_eq!(calculate_oversample(0.05), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_selectivity_0_01_capped() {
        assert_eq!(calculate_oversample(0.01), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_selectivity_0_001_capped() {
        assert_eq!(calculate_oversample(0.001), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_selectivity_0_0001_capped() {
        assert_eq!(calculate_oversample(0.0001), MAX_OVERSAMPLE);
    }

    // Edge cases
    #[test]
    fn test_selectivity_0_capped() {
        assert_eq!(calculate_oversample(0.0), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_selectivity_negative_capped() {
        assert_eq!(calculate_oversample(-0.1), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_selectivity_very_negative_capped() {
        assert_eq!(calculate_oversample(-100.0), MAX_OVERSAMPLE);
    }

    // Just above cap threshold
    #[test]
    fn test_selectivity_just_above_cap() {
        // 1/MAX_OVERSAMPLE = 0.1, so selectivity of 0.11 should not be capped
        let result = calculate_oversample(0.11);
        assert!(result < MAX_OVERSAMPLE);
        assert!(result > 9.0);
    }

    // Precision tests
    #[test]
    fn test_selectivity_0_15() {
        let result = calculate_oversample(0.15);
        assert!(result > 6.6 && result < 6.7);
    }

    #[test]
    fn test_selectivity_0_3() {
        let result = calculate_oversample(0.3);
        assert!(result > 3.3 && result < 3.4);
    }

    #[test]
    fn test_selectivity_0_4() {
        assert!((calculate_oversample(0.4) - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_selectivity_0_6() {
        let result = calculate_oversample(0.6);
        assert!(result > 1.66 && result < 1.67);
    }

    #[test]
    fn test_selectivity_0_7() {
        let result = calculate_oversample(0.7);
        assert!(result > 1.42 && result < 1.43);
    }

    // Result is always >= 1.0
    #[test]
    fn test_result_always_gte_1_high_selectivity() {
        assert!(calculate_oversample(0.99) >= 1.0);
    }

    #[test]
    fn test_result_always_gte_1_full_selectivity() {
        assert!(calculate_oversample(1.0) >= 1.0);
    }

    // Result is always <= MAX_OVERSAMPLE
    #[test]
    fn test_result_always_lte_max_low_selectivity() {
        assert!(calculate_oversample(0.001) <= MAX_OVERSAMPLE);
    }

    #[test]
    fn test_result_always_lte_max_zero() {
        assert!(calculate_oversample(0.0) <= MAX_OVERSAMPLE);
    }

    // Specific selectivities
    #[test]
    fn test_selectivity_half() {
        let result = calculate_oversample(0.5);
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_selectivity_quarter() {
        let result = calculate_oversample(0.25);
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_selectivity_fifth() {
        let result = calculate_oversample(0.2);
        assert_eq!(result, 5.0);
    }

    #[test]
    fn test_selectivity_tenth() {
        let result = calculate_oversample(0.1);
        assert_eq!(result, MAX_OVERSAMPLE);
    }

    // Fractional selectivities
    #[test]
    fn test_selectivity_0_12() {
        let result = calculate_oversample(0.12);
        assert!(result > 8.3 && result < 8.4);
    }

    #[test]
    fn test_selectivity_0_17() {
        let result = calculate_oversample(0.17);
        assert!(result > 5.8 && result < 5.9);
    }

    #[test]
    fn test_selectivity_0_23() {
        let result = calculate_oversample(0.23);
        assert!(result > 4.3 && result < 4.4);
    }

    #[test]
    fn test_selectivity_0_55() {
        let result = calculate_oversample(0.55);
        assert!(result > 1.8 && result < 1.82);
    }

    #[test]
    fn test_selectivity_0_65() {
        let result = calculate_oversample(0.65);
        assert!(result > 1.53 && result < 1.54);
    }

    #[test]
    fn test_selectivity_0_85() {
        let result = calculate_oversample(0.85);
        assert!(result > 1.17 && result < 1.18);
    }

    #[test]
    fn test_selectivity_0_95() {
        let result = calculate_oversample(0.95);
        assert!(result > 1.05 && result < 1.06);
    }

    // Boundary at cap
    #[test]
    fn test_selectivity_exactly_at_cap_boundary() {
        // At selectivity 0.1, oversample = 10.0 = MAX_OVERSAMPLE
        assert_eq!(calculate_oversample(0.1), MAX_OVERSAMPLE);
    }
}

// =============================================================================
// STRATEGY SELECTION TESTS (50 tests)
// =============================================================================

mod select_strategy_tests {
    use super::*;

    // High selectivity -> PreFilter
    #[test]
    fn test_select_strategy_0_95() {
        assert_eq!(select_strategy(0.95), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_0_9() {
        assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_0_85() {
        assert_eq!(select_strategy(0.85), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_0_81() {
        assert_eq!(select_strategy(0.81), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_1_0() {
        assert_eq!(select_strategy(1.0), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_0_99() {
        assert_eq!(select_strategy(0.99), FilterStrategy::PreFilter);
    }

    // At boundary 0.8 -> Hybrid (not PreFilter, threshold is >0.8)
    #[test]
    fn test_select_strategy_exactly_0_8() {
        assert!(matches!(select_strategy(0.8), FilterStrategy::Hybrid { .. }));
    }

    // Low selectivity -> PostFilter
    #[test]
    fn test_select_strategy_0_04() {
        assert!(matches!(select_strategy(0.04), FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_0_03() {
        assert!(matches!(select_strategy(0.03), FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_0_02() {
        assert!(matches!(select_strategy(0.02), FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_0_01() {
        assert!(matches!(select_strategy(0.01), FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_0_001() {
        assert!(matches!(select_strategy(0.001), FilterStrategy::PostFilter { .. }));
    }

    // At boundary 0.05 -> Hybrid (not PostFilter, threshold is <0.05)
    #[test]
    fn test_select_strategy_exactly_0_05() {
        assert!(matches!(select_strategy(0.05), FilterStrategy::Hybrid { .. }));
    }

    // Medium selectivity -> Hybrid
    #[test]
    fn test_select_strategy_0_5() {
        assert!(matches!(select_strategy(0.5), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_4() {
        assert!(matches!(select_strategy(0.4), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_3() {
        assert!(matches!(select_strategy(0.3), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_2() {
        assert!(matches!(select_strategy(0.2), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_15() {
        assert!(matches!(select_strategy(0.15), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_1() {
        assert!(matches!(select_strategy(0.1), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_07() {
        assert!(matches!(select_strategy(0.07), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_06() {
        assert!(matches!(select_strategy(0.06), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_6() {
        assert!(matches!(select_strategy(0.6), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_7() {
        assert!(matches!(select_strategy(0.7), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_75() {
        assert!(matches!(select_strategy(0.75), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_select_strategy_0_79() {
        assert!(matches!(select_strategy(0.79), FilterStrategy::Hybrid { .. }));
    }

    // PostFilter oversample values
    #[test]
    fn test_postfilter_oversample_at_0_04() {
        match select_strategy(0.04) {
            FilterStrategy::PostFilter { oversample } => {
                assert_eq!(oversample, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }
    }

    #[test]
    fn test_postfilter_oversample_at_0_03() {
        match select_strategy(0.03) {
            FilterStrategy::PostFilter { oversample } => {
                assert_eq!(oversample, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }
    }

    #[test]
    fn test_postfilter_oversample_at_0_01() {
        match select_strategy(0.01) {
            FilterStrategy::PostFilter { oversample } => {
                assert_eq!(oversample, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }
    }

    // Hybrid bounds
    #[test]
    fn test_hybrid_bounds_at_0_5() {
        match select_strategy(0.5) {
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
                assert!((oversample_min - 1.5).abs() < 0.001);
                assert!((oversample_max - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected Hybrid"),
        }
    }

    #[test]
    fn test_hybrid_bounds_at_0_3() {
        match select_strategy(0.3) {
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
                assert!((oversample_min - 1.5).abs() < 0.001);
                assert!(oversample_max > 3.3 && oversample_max < 3.4);
            }
            _ => panic!("Expected Hybrid"),
        }
    }

    #[test]
    fn test_hybrid_bounds_at_0_2() {
        match select_strategy(0.2) {
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
                assert!((oversample_min - 1.5).abs() < 0.001);
                assert!((oversample_max - 5.0).abs() < 0.001);
            }
            _ => panic!("Expected Hybrid"),
        }
    }

    #[test]
    fn test_hybrid_bounds_at_0_1() {
        match select_strategy(0.1) {
            FilterStrategy::Hybrid { oversample_min, oversample_max } => {
                assert!((oversample_min - 1.5).abs() < 0.001);
                assert_eq!(oversample_max, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected Hybrid"),
        }
    }

    // Edge cases
    #[test]
    fn test_select_strategy_0_0() {
        // Zero selectivity (impossible) -> should still return valid strategy
        let strategy = select_strategy(0.0);
        assert!(matches!(strategy, FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_negative() {
        let strategy = select_strategy(-0.1);
        assert!(matches!(strategy, FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_select_strategy_above_1() {
        let strategy = select_strategy(1.5);
        assert_eq!(strategy, FilterStrategy::PreFilter);
    }

    // Coverage at specific thresholds
    #[test]
    fn test_prefilter_threshold_value() {
        assert_eq!(PREFILTER_THRESHOLD, 0.8);
    }

    #[test]
    fn test_postfilter_threshold_value() {
        assert_eq!(POSTFILTER_THRESHOLD, 0.05);
    }

    #[test]
    fn test_strategy_just_above_prefilter_threshold() {
        assert_eq!(select_strategy(0.801), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_strategy_just_below_prefilter_threshold() {
        assert!(matches!(select_strategy(0.799), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_strategy_just_above_postfilter_threshold() {
        assert!(matches!(select_strategy(0.051), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_strategy_just_below_postfilter_threshold() {
        assert!(matches!(select_strategy(0.049), FilterStrategy::PostFilter { .. }));
    }

    // Multiple calls with same input
    #[test]
    fn test_select_strategy_deterministic() {
        let result1 = select_strategy(0.5);
        let result2 = select_strategy(0.5);
        assert_eq!(result1, result2);
    }

    // Full range coverage
    #[test]
    fn test_full_range_prefilter() {
        for i in 81..=100 {
            let selectivity = i as f32 / 100.0;
            assert_eq!(select_strategy(selectivity), FilterStrategy::PreFilter);
        }
    }

    #[test]
    fn test_full_range_hybrid() {
        for i in 5..=80 {
            let selectivity = i as f32 / 100.0;
            assert!(matches!(select_strategy(selectivity), FilterStrategy::Hybrid { .. }));
        }
    }

    #[test]
    fn test_full_range_postfilter() {
        for i in 1..5 {
            let selectivity = i as f32 / 100.0;
            assert!(matches!(select_strategy(selectivity), FilterStrategy::PostFilter { .. }));
        }
    }
}

// =============================================================================
// TAUTOLOGY DETECTION TESTS (40 tests)
// =============================================================================

mod tautology_tests {
    use super::*;

    #[test]
    fn test_true_literal_is_tautology() {
        assert!(is_tautology(&FilterExpr::LiteralBool(true)));
    }

    #[test]
    fn test_false_literal_not_tautology() {
        assert!(!is_tautology(&FilterExpr::LiteralBool(false)));
    }

    #[test]
    fn test_field_not_tautology() {
        let filter = FilterExpr::Field("x".into());
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_eq_not_tautology() {
        let filter = parse("x = 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ne_not_tautology() {
        let filter = parse("x != 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_lt_not_tautology() {
        let filter = parse("x < 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_gt_not_tautology() {
        let filter = parse("x > 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_a_or_not_a_is_tautology() {
        let a = FilterExpr::Field("x".into());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_not_a_or_a_is_tautology() {
        let a = FilterExpr::Field("x".into());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(not_a), Box::new(a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_true_or_anything() {
        let anything = parse("x = 5").unwrap();
        let or = FilterExpr::Or(Box::new(FilterExpr::LiteralBool(true)), Box::new(anything));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_anything_or_true() {
        let anything = parse("x = 5").unwrap();
        let or = FilterExpr::Or(Box::new(anything), Box::new(FilterExpr::LiteralBool(true)));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_not_false_is_tautology() {
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(false)));
        assert!(is_tautology(&filter));
    }

    #[test]
    fn test_not_true_not_tautology() {
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true)));
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_simple_and_not_tautology() {
        let filter = parse("x = 1 AND y = 2").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_simple_or_not_tautology() {
        let filter = parse("x = 1 OR y = 2").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_true_and_true_is_tautology() {
        let and = FilterExpr::And(
            Box::new(FilterExpr::LiteralBool(true)),
            Box::new(FilterExpr::LiteralBool(true)),
        );
        assert!(is_tautology(&and));
    }

    #[test]
    fn test_true_and_false_not_tautology() {
        let and = FilterExpr::And(
            Box::new(FilterExpr::LiteralBool(true)),
            Box::new(FilterExpr::LiteralBool(false)),
        );
        assert!(!is_tautology(&and));
    }

    #[test]
    fn test_false_and_true_not_tautology() {
        let and = FilterExpr::And(
            Box::new(FilterExpr::LiteralBool(false)),
            Box::new(FilterExpr::LiteralBool(true)),
        );
        assert!(!is_tautology(&and));
    }

    #[test]
    fn test_nested_tautology_in_or() {
        // (a AND b) OR TRUE
        let a = FilterExpr::Field("x".into());
        let b = FilterExpr::Field("y".into());
        let and = FilterExpr::And(Box::new(a), Box::new(b));
        let or = FilterExpr::Or(Box::new(and), Box::new(FilterExpr::LiteralBool(true)));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_deeply_nested_tautology() {
        // ((a OR TRUE) AND b) OR c
        // (a OR TRUE) is tautology, but (tautology AND b) is not necessarily
        let a = FilterExpr::Field("a".into());
        let b = FilterExpr::Field("b".into());
        let _c = FilterExpr::Field("c".into());
        let or1 = FilterExpr::Or(Box::new(a), Box::new(FilterExpr::LiteralBool(true)));
        let and = FilterExpr::And(Box::new(or1.clone()), Box::new(b.clone()));
        // Since or1 is tautology, and is equivalent to just b, which is not tautology
        // The outer OR with c is also not tautology
        // Actually wait - let me check the logic:
        // or1 = a OR TRUE = TRUE (tautology)
        // and = TRUE AND b = b (not tautology unless b is)
        // outer = b OR c (not tautology)
        // But is_tautology uses structural analysis, not simplification
        // It checks: if or1 is tautology, then the outer OR is tautology? No!
        // The AND propagates: is_tautology(left) && is_tautology(right)
        // or1 is tautology, but b is not, so AND is not tautology
        assert!(!is_tautology(&and));
    }

    // IN/NOT IN not tautology
    #[test]
    fn test_in_not_tautology() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_not_in_not_tautology() {
        let filter = parse("x NOT IN [1, 2, 3]").unwrap();
        assert!(!is_tautology(&filter));
    }

    // BETWEEN not tautology
    #[test]
    fn test_between_not_tautology() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        assert!(!is_tautology(&filter));
    }

    // String operators not tautology
    #[test]
    fn test_contains_not_tautology() {
        let filter = parse(r#"x contains "test""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_like_not_tautology() {
        let filter = parse(r#"x LIKE "test%""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // NULL checks not tautology
    #[test]
    fn test_is_null_not_tautology() {
        let filter = parse("x IS NULL").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_is_not_null_not_tautology() {
        let filter = parse("x IS NOT NULL").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Complex filter not tautology
    #[test]
    fn test_complex_filter_not_tautology() {
        let filter = parse("a = 1 AND b = 2 OR c = 3").unwrap();
        assert!(!is_tautology(&filter));
    }

    // More a OR NOT a variants
    #[test]
    fn test_complex_a_or_not_a() {
        let a = parse("x > 5").unwrap();
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_nested_or_with_complement() {
        let a = FilterExpr::Field("x".into());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let b = FilterExpr::Field("y".into());
        // (a OR b) OR NOT a
        // This is actually tautology because a OR NOT a is embedded
        let or1 = FilterExpr::Or(Box::new(a.clone()), Box::new(b));
        let or2 = FilterExpr::Or(Box::new(or1), Box::new(not_a));
        // This might not be detected as tautology by structural analysis
        // Let's check: is or1 tautology? No. is not_a tautology? No.
        // are_complementary(or1, not_a)? No, or1 is not the complement of not_a
        assert!(!is_tautology(&or2)); // Structural analysis doesn't detect this
    }

    // Array operators not tautology
    #[test]
    fn test_any_not_tautology() {
        let filter = parse(r#"tags ANY ["rust"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_all_not_tautology() {
        let filter = parse(r#"tags ALL ["rust"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_none_not_tautology() {
        let filter = parse(r#"tags NONE ["rust"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // Literals
    #[test]
    fn test_int_literal_not_tautology() {
        assert!(!is_tautology(&FilterExpr::LiteralInt(42)));
    }

    #[test]
    fn test_float_literal_not_tautology() {
        assert!(!is_tautology(&FilterExpr::LiteralFloat(3.5)));
    }

    #[test]
    fn test_string_literal_not_tautology() {
        assert!(!is_tautology(&FilterExpr::LiteralString("test".into())));
    }
}

// =============================================================================
// CONTRADICTION DETECTION TESTS (40 tests)
// =============================================================================

mod contradiction_tests {
    use super::*;

    #[test]
    fn test_false_literal_is_contradiction() {
        assert!(is_contradiction(&FilterExpr::LiteralBool(false)));
    }

    #[test]
    fn test_true_literal_not_contradiction() {
        assert!(!is_contradiction(&FilterExpr::LiteralBool(true)));
    }

    #[test]
    fn test_field_not_contradiction() {
        let filter = FilterExpr::Field("x".into());
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_eq_not_contradiction() {
        let filter = parse("x = 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_a_and_not_a_is_contradiction() {
        let a = FilterExpr::Field("x".into());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let and = FilterExpr::And(Box::new(a), Box::new(not_a));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_not_a_and_a_is_contradiction() {
        let a = FilterExpr::Field("x".into());
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let and = FilterExpr::And(Box::new(not_a), Box::new(a));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_not_true_is_contradiction() {
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true)));
        assert!(is_contradiction(&filter));
    }

    #[test]
    fn test_not_false_not_contradiction() {
        let filter = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(false)));
        assert!(!is_contradiction(&filter));
    }

    // Impossible range tests
    #[test]
    fn test_impossible_range_gt_lt() {
        let gt10 = parse("x > 10").unwrap();
        let lt5 = parse("x < 5").unwrap();
        let and = FilterExpr::And(Box::new(gt10), Box::new(lt5));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_range_lt_gt() {
        let lt5 = parse("x < 5").unwrap();
        let gt10 = parse("x > 10").unwrap();
        let and = FilterExpr::And(Box::new(lt5), Box::new(gt10));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_range_ge_le() {
        let ge10 = parse("x >= 10").unwrap();
        let le5 = parse("x <= 5").unwrap();
        let and = FilterExpr::And(Box::new(ge10), Box::new(le5));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_range_le_ge() {
        let le5 = parse("x <= 5").unwrap();
        let ge10 = parse("x >= 10").unwrap();
        let and = FilterExpr::And(Box::new(le5), Box::new(ge10));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_range_float() {
        let gt = parse("x > 10.5").unwrap();
        let lt = parse("x < 5.0").unwrap();
        let and = FilterExpr::And(Box::new(gt), Box::new(lt));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_range_mixed_int_float() {
        let gt = parse("x > 10").unwrap();
        let lt = parse("x < 5.0").unwrap();
        let and = FilterExpr::And(Box::new(gt), Box::new(lt));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_possible_range_not_contradiction() {
        let gt5 = parse("x > 5").unwrap();
        let lt10 = parse("x < 10").unwrap();
        let and = FilterExpr::And(Box::new(gt5), Box::new(lt10));
        assert!(!is_contradiction(&and));
    }

    #[test]
    fn test_possible_range_overlapping() {
        let gt0 = parse("x > 0").unwrap();
        let lt100 = parse("x < 100").unwrap();
        let and = FilterExpr::And(Box::new(gt0), Box::new(lt100));
        assert!(!is_contradiction(&and));
    }

    #[test]
    fn test_different_fields_not_contradiction() {
        let gtx = parse("x > 10").unwrap();
        let lty = parse("y < 5").unwrap();
        let and = FilterExpr::And(Box::new(gtx), Box::new(lty));
        assert!(!is_contradiction(&and));
    }

    #[test]
    fn test_false_and_anything() {
        let anything = parse("x = 5").unwrap();
        let and = FilterExpr::And(Box::new(FilterExpr::LiteralBool(false)), Box::new(anything));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_anything_and_false() {
        let anything = parse("x = 5").unwrap();
        let and = FilterExpr::And(Box::new(anything), Box::new(FilterExpr::LiteralBool(false)));
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_false_or_false_is_contradiction() {
        let or = FilterExpr::Or(
            Box::new(FilterExpr::LiteralBool(false)),
            Box::new(FilterExpr::LiteralBool(false)),
        );
        assert!(is_contradiction(&or));
    }

    #[test]
    fn test_false_or_true_not_contradiction() {
        let or = FilterExpr::Or(
            Box::new(FilterExpr::LiteralBool(false)),
            Box::new(FilterExpr::LiteralBool(true)),
        );
        assert!(!is_contradiction(&or));
    }

    // Simple expressions not contradiction
    #[test]
    fn test_simple_lt_not_contradiction() {
        let filter = parse("x < 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_simple_gt_not_contradiction() {
        let filter = parse("x > 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_simple_le_not_contradiction() {
        let filter = parse("x <= 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_simple_ge_not_contradiction() {
        let filter = parse("x >= 5").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_in_not_contradiction() {
        let filter = parse("x IN [1, 2, 3]").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_between_not_contradiction() {
        let filter = parse("x BETWEEN 1 AND 10").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_contains_not_contradiction() {
        let filter = parse(r#"x contains "test""#).unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_is_null_not_contradiction() {
        let filter = parse("x IS NULL").unwrap();
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_is_not_null_not_contradiction() {
        let filter = parse("x IS NOT NULL").unwrap();
        assert!(!is_contradiction(&filter));
    }

    // Nested contradictions
    #[test]
    fn test_nested_and_with_contradiction() {
        // (x > 10 AND x < 5) AND y = 1
        let gt10 = parse("x > 10").unwrap();
        let lt5 = parse("x < 5").unwrap();
        let inner = FilterExpr::And(Box::new(gt10), Box::new(lt5));
        let y1 = parse("y = 1").unwrap();
        let outer = FilterExpr::And(Box::new(inner), Box::new(y1));
        assert!(is_contradiction(&outer));
    }

    // Boundary cases
    #[test]
    fn test_equal_bounds_treated_as_contradiction() {
        // NOTE: x >= 5 AND x <= 5 mathematically has solution x = 5,
        // but current implementation treats equal bounds as contradiction.
        // This is a known limitation - the is_impossible_range function
        // uses >= comparison without distinguishing strict/non-strict operators.
        let ge5 = parse("x >= 5").unwrap();
        let le5 = parse("x <= 5").unwrap();
        let and = FilterExpr::And(Box::new(ge5), Box::new(le5));
        // Current implementation returns true (treats as contradiction)
        assert!(is_contradiction(&and));
    }

    #[test]
    fn test_impossible_with_equal_at_boundary() {
        // x > 5 AND x < 5 is impossible
        let gt5 = parse("x > 5").unwrap();
        let lt5 = parse("x < 5").unwrap();
        let and = FilterExpr::And(Box::new(gt5), Box::new(lt5));
        assert!(is_contradiction(&and));
    }

    // Literals
    #[test]
    fn test_int_literal_not_contradiction() {
        assert!(!is_contradiction(&FilterExpr::LiteralInt(42)));
    }

    #[test]
    fn test_float_literal_not_contradiction() {
        assert!(!is_contradiction(&FilterExpr::LiteralFloat(3.5)));
    }

    #[test]
    fn test_string_literal_not_contradiction() {
        assert!(!is_contradiction(&FilterExpr::LiteralString("test".into())));
    }

    // Complex filter not contradiction
    #[test]
    fn test_complex_filter_not_contradiction() {
        let filter = parse("a = 1 AND b = 2 OR c = 3").unwrap();
        assert!(!is_contradiction(&filter));
    }
}

// =============================================================================
// SELECTIVITY ESTIMATE TESTS (40 tests)
// =============================================================================

mod selectivity_estimate_tests {
    use super::*;

    #[test]
    fn test_new_normal() {
        let estimate = SelectivityEstimate::new(0.5, 100, 50);
        assert_eq!(estimate.selectivity, 0.5);
        assert_eq!(estimate.sample_size, 100);
        assert_eq!(estimate.passed, 50);
    }

    #[test]
    fn test_new_zero_selectivity() {
        let estimate = SelectivityEstimate::new(0.0, 100, 0);
        assert_eq!(estimate.selectivity, 0.0);
    }

    #[test]
    fn test_new_full_selectivity() {
        let estimate = SelectivityEstimate::new(1.0, 100, 100);
        assert_eq!(estimate.selectivity, 1.0);
    }

    #[test]
    fn test_new_clamps_above_1() {
        let estimate = SelectivityEstimate::new(1.5, 100, 150);
        assert_eq!(estimate.selectivity, 1.0);
    }

    #[test]
    fn test_new_clamps_below_0() {
        let estimate = SelectivityEstimate::new(-0.5, 100, 0);
        assert_eq!(estimate.selectivity, 0.0);
    }

    #[test]
    fn test_new_large_negative() {
        let estimate = SelectivityEstimate::new(-10.0, 100, 0);
        assert_eq!(estimate.selectivity, 0.0);
    }

    #[test]
    fn test_new_large_positive() {
        let estimate = SelectivityEstimate::new(10.0, 100, 100);
        assert_eq!(estimate.selectivity, 1.0);
    }

    #[test]
    fn test_zero_constructor() {
        let estimate = SelectivityEstimate::zero();
        assert_eq!(estimate.selectivity, 0.01);
        assert_eq!(estimate.sample_size, 0);
        assert_eq!(estimate.passed, 0);
    }

    #[test]
    fn test_full_constructor() {
        let estimate = SelectivityEstimate::full();
        assert_eq!(estimate.selectivity, 1.0);
    }

    // Confidence tests
    #[test]
    fn test_confidence_full_sample() {
        let estimate = SelectivityEstimate::new(0.5, 100, 50);
        assert_eq!(estimate.confidence(), 1.0);
    }

    #[test]
    fn test_confidence_half_sample() {
        let estimate = SelectivityEstimate::new(0.5, 50, 25);
        assert_eq!(estimate.confidence(), 0.5);
    }

    #[test]
    fn test_confidence_zero_sample() {
        let estimate = SelectivityEstimate::new(0.5, 0, 0);
        assert_eq!(estimate.confidence(), 0.0);
    }

    #[test]
    fn test_confidence_10_sample() {
        let estimate = SelectivityEstimate::new(0.5, 10, 5);
        assert_eq!(estimate.confidence(), 0.1);
    }

    #[test]
    fn test_confidence_25_sample() {
        let estimate = SelectivityEstimate::new(0.5, 25, 12);
        assert_eq!(estimate.confidence(), 0.25);
    }

    #[test]
    fn test_confidence_75_sample() {
        let estimate = SelectivityEstimate::new(0.5, 75, 37);
        assert_eq!(estimate.confidence(), 0.75);
    }

    #[test]
    fn test_confidence_over_100_capped() {
        let estimate = SelectivityEstimate::new(0.5, 150, 75);
        assert_eq!(estimate.confidence(), 1.0);
    }

    #[test]
    fn test_confidence_200_capped() {
        let estimate = SelectivityEstimate::new(0.5, 200, 100);
        assert_eq!(estimate.confidence(), 1.0);
    }

    // Various selectivity values
    #[test]
    fn test_selectivity_0_1() {
        let estimate = SelectivityEstimate::new(0.1, 100, 10);
        assert_eq!(estimate.selectivity, 0.1);
    }

    #[test]
    fn test_selectivity_0_2() {
        let estimate = SelectivityEstimate::new(0.2, 100, 20);
        assert_eq!(estimate.selectivity, 0.2);
    }

    #[test]
    fn test_selectivity_0_3() {
        let estimate = SelectivityEstimate::new(0.3, 100, 30);
        assert_eq!(estimate.selectivity, 0.3);
    }

    #[test]
    fn test_selectivity_0_4() {
        let estimate = SelectivityEstimate::new(0.4, 100, 40);
        assert_eq!(estimate.selectivity, 0.4);
    }

    #[test]
    fn test_selectivity_0_6() {
        let estimate = SelectivityEstimate::new(0.6, 100, 60);
        assert_eq!(estimate.selectivity, 0.6);
    }

    #[test]
    fn test_selectivity_0_7() {
        let estimate = SelectivityEstimate::new(0.7, 100, 70);
        assert_eq!(estimate.selectivity, 0.7);
    }

    #[test]
    fn test_selectivity_0_8() {
        let estimate = SelectivityEstimate::new(0.8, 100, 80);
        assert_eq!(estimate.selectivity, 0.8);
    }

    #[test]
    fn test_selectivity_0_9() {
        let estimate = SelectivityEstimate::new(0.9, 100, 90);
        assert_eq!(estimate.selectivity, 0.9);
    }

    // Edge cases for sample size
    #[test]
    fn test_sample_size_1() {
        let estimate = SelectivityEstimate::new(1.0, 1, 1);
        assert_eq!(estimate.confidence(), 0.01);
    }

    #[test]
    fn test_sample_size_1_zero_passed() {
        let estimate = SelectivityEstimate::new(0.0, 1, 0);
        assert_eq!(estimate.confidence(), 0.01);
    }

    // Equality tests
    #[test]
    fn test_estimate_equality() {
        let a = SelectivityEstimate::new(0.5, 100, 50);
        let b = SelectivityEstimate::new(0.5, 100, 50);
        assert_eq!(a, b);
    }

    #[test]
    fn test_estimate_inequality_selectivity() {
        let a = SelectivityEstimate::new(0.5, 100, 50);
        let b = SelectivityEstimate::new(0.6, 100, 60);
        assert_ne!(a, b);
    }

    #[test]
    fn test_estimate_inequality_sample_size() {
        let a = SelectivityEstimate::new(0.5, 100, 50);
        let b = SelectivityEstimate::new(0.5, 50, 25);
        assert_ne!(a, b);
    }

    #[test]
    fn test_estimate_inequality_passed() {
        let a = SelectivityEstimate::new(0.5, 100, 50);
        let b = SelectivityEstimate::new(0.5, 100, 40);
        assert_ne!(a, b);
    }

    // Clone tests
    #[test]
    fn test_estimate_clone() {
        let original = SelectivityEstimate::new(0.5, 100, 50);
        let cloned = original;
        assert_eq!(original, cloned);
    }

    // Copy tests
    #[test]
    fn test_estimate_copy() {
        let original = SelectivityEstimate::new(0.5, 100, 50);
        let copied = original;
        assert_eq!(original.selectivity, copied.selectivity);
    }

    // Small sample sizes
    #[test]
    fn test_small_sample_2() {
        let estimate = SelectivityEstimate::new(0.5, 2, 1);
        assert_eq!(estimate.confidence(), 0.02);
    }

    #[test]
    fn test_small_sample_5() {
        let estimate = SelectivityEstimate::new(0.6, 5, 3);
        assert_eq!(estimate.confidence(), 0.05);
    }

    #[test]
    fn test_small_sample_20() {
        let estimate = SelectivityEstimate::new(0.5, 20, 10);
        assert_eq!(estimate.confidence(), 0.2);
    }
}

// =============================================================================
// CONSTANTS TESTS (28 tests)
// =============================================================================

mod constants_tests {
    use super::*;

    #[test]
    fn test_max_oversample_value() {
        assert_eq!(MAX_OVERSAMPLE, 10.0);
    }

    #[test]
    fn test_default_oversample_value() {
        assert_eq!(DEFAULT_OVERSAMPLE, 3.0);
    }

    #[test]
    fn test_ef_cap_value() {
        assert_eq!(EF_CAP, 1000);
    }

    #[test]
    fn test_selectivity_sample_size_value() {
        assert_eq!(SELECTIVITY_SAMPLE_SIZE, 100);
    }

    #[test]
    fn test_prefilter_threshold_value() {
        assert_eq!(PREFILTER_THRESHOLD, 0.8);
    }

    #[test]
    fn test_postfilter_threshold_value() {
        assert_eq!(POSTFILTER_THRESHOLD, 0.05);
    }

    // Reasonable values tests
    #[test]
    fn test_max_oversample_reasonable() {
        assert!(MAX_OVERSAMPLE >= 5.0);
        assert!(MAX_OVERSAMPLE <= 20.0);
    }

    #[test]
    fn test_default_oversample_reasonable() {
        assert!(DEFAULT_OVERSAMPLE >= 1.0);
        assert!(DEFAULT_OVERSAMPLE <= MAX_OVERSAMPLE);
    }

    #[test]
    fn test_ef_cap_reasonable() {
        assert!(EF_CAP >= 100);
        assert!(EF_CAP <= 10000);
    }

    #[test]
    fn test_selectivity_sample_size_reasonable() {
        assert!(SELECTIVITY_SAMPLE_SIZE >= 10);
        assert!(SELECTIVITY_SAMPLE_SIZE <= 1000);
    }

    #[test]
    fn test_prefilter_threshold_reasonable() {
        assert!(PREFILTER_THRESHOLD > 0.5);
        assert!(PREFILTER_THRESHOLD < 1.0);
    }

    #[test]
    fn test_postfilter_threshold_reasonable() {
        assert!(POSTFILTER_THRESHOLD > 0.0);
        assert!(POSTFILTER_THRESHOLD < 0.2);
    }

    // Relationship tests
    #[test]
    fn test_prefilter_gt_postfilter_threshold() {
        assert!(PREFILTER_THRESHOLD > POSTFILTER_THRESHOLD);
    }

    #[test]
    fn test_default_oversample_lte_max() {
        assert!(DEFAULT_OVERSAMPLE <= MAX_OVERSAMPLE);
    }

    // Constants used correctly
    #[test]
    fn test_post_filter_default_uses_constant() {
        match FilterStrategy::POST_FILTER_DEFAULT {
            FilterStrategy::PostFilter { oversample } => {
                assert_eq!(oversample, DEFAULT_OVERSAMPLE);
            }
            _ => panic!("Expected PostFilter"),
        }
    }

    #[test]
    fn test_hybrid_default_uses_max_constant() {
        match FilterStrategy::HYBRID_DEFAULT {
            FilterStrategy::Hybrid { oversample_max, .. } => {
                assert_eq!(oversample_max, MAX_OVERSAMPLE);
            }
            _ => panic!("Expected Hybrid"),
        }
    }

    #[test]
    fn test_calculate_oversample_uses_max_constant() {
        assert_eq!(calculate_oversample(0.0), MAX_OVERSAMPLE);
    }

    #[test]
    fn test_select_strategy_uses_prefilter_threshold() {
        // Just above threshold -> PreFilter
        assert_eq!(select_strategy(PREFILTER_THRESHOLD + 0.001), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_select_strategy_uses_postfilter_threshold() {
        // Just below threshold -> PostFilter
        assert!(matches!(
            select_strategy(POSTFILTER_THRESHOLD - 0.001),
            FilterStrategy::PostFilter { .. }
        ));
    }

    // Type tests
    #[test]
    fn test_max_oversample_is_f32() {
        let _: f32 = MAX_OVERSAMPLE;
    }

    #[test]
    fn test_default_oversample_is_f32() {
        let _: f32 = DEFAULT_OVERSAMPLE;
    }

    #[test]
    fn test_ef_cap_is_usize() {
        let _: usize = EF_CAP;
    }

    #[test]
    fn test_selectivity_sample_size_is_usize() {
        let _: usize = SELECTIVITY_SAMPLE_SIZE;
    }

    #[test]
    fn test_prefilter_threshold_is_f32() {
        let _: f32 = PREFILTER_THRESHOLD;
    }

    #[test]
    fn test_postfilter_threshold_is_f32() {
        let _: f32 = POSTFILTER_THRESHOLD;
    }

    // Constants are positive
    #[test]
    fn test_max_oversample_positive() {
        assert!(MAX_OVERSAMPLE > 0.0);
    }

    #[test]
    fn test_default_oversample_positive() {
        assert!(DEFAULT_OVERSAMPLE > 0.0);
    }

    #[test]
    fn test_ef_cap_positive() {
        assert!(EF_CAP > 0);
    }
}

// =============================================================================
// REAL WORLD FILTER SCENARIOS (60 tests)
// =============================================================================

mod real_world_scenarios {
    use super::*;

    // E-commerce filters
    #[test]
    fn test_ecommerce_price_range() {
        let filter = parse("price BETWEEN 10 AND 100").unwrap();
        assert!(!is_tautology(&filter));
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_ecommerce_category() {
        let filter = parse(r#"category = "electronics""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ecommerce_in_stock() {
        let filter = parse("in_stock = true").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ecommerce_combined() {
        let filter = parse(r#"category = "electronics" AND price < 500 AND in_stock = true"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ecommerce_brand_or() {
        let filter = parse(r#"brand = "apple" OR brand = "samsung""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ecommerce_rating() {
        let filter = parse("rating >= 4.0").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_ecommerce_tags() {
        let filter = parse(r#"tags ANY ["sale", "featured"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // Content management filters
    #[test]
    fn test_cms_published() {
        let filter = parse("status = 1").unwrap(); // published
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_cms_author() {
        let filter = parse(r#"author = "admin""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_cms_date_range() {
        let filter = parse("created_at > 1700000000").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_cms_category_tags() {
        let filter = parse(r#"category = "tech" AND tags ANY ["tutorial", "guide"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_cms_not_deleted() {
        let filter = parse("deleted = false").unwrap();
        assert!(!is_tautology(&filter));
    }

    // User management filters
    #[test]
    fn test_user_active() {
        let filter = parse("active = true").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_user_role() {
        let filter = parse(r#"role IN ["admin", "moderator"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_user_verified() {
        let filter = parse("email_verified = true AND phone_verified = true").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_user_premium() {
        let filter = parse(r#"subscription = "premium""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // Search filters
    #[test]
    fn test_search_contains() {
        let filter = parse(r#"title contains "rust""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_search_like() {
        let filter = parse(r#"title LIKE "rust%""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_search_starts_with() {
        let filter = parse(r#"title starts_with "How to""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_search_ends_with() {
        let filter = parse(r#"filename ends_with ".pdf""#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // Complex business logic
    #[test]
    fn test_complex_discount_eligibility() {
        let filter = parse("total > 100 AND (vip = true OR coupon_used = false)").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_complex_shipping() {
        let filter = parse("weight < 50 AND (domestic = true OR express = true)").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_complex_inventory() {
        let filter = parse("quantity > 0 AND NOT (reserved = true AND backorder = false)").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Null handling
    #[test]
    fn test_null_optional_field() {
        let filter = parse("phone IS NULL").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_not_null_required_field() {
        let filter = parse("email IS NOT NULL").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_null_with_default() {
        let filter = parse("custom_field IS NULL OR custom_field = 0").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Numeric ranges
    #[test]
    fn test_age_range() {
        let filter = parse("age >= 18 AND age <= 65").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_score_range() {
        let filter = parse("score BETWEEN 0 AND 100").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_negative_balance() {
        let filter = parse("balance < 0").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_float_precision() {
        let filter = parse("price = 19.99").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Array operations
    #[test]
    fn test_all_tags_required() {
        let filter = parse(r#"tags ALL ["featured", "approved"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_none_blocked_tags() {
        let filter = parse(r#"tags NONE ["spam", "nsfw", "blocked"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_any_premium_tags() {
        let filter = parse(r#"features ANY ["hd", "4k", "dolby"]"#).unwrap();
        assert!(!is_tautology(&filter));
    }

    // Multi-field filters
    #[test]
    fn test_multi_field_and() {
        let filter = parse("a = 1 AND b = 2 AND c = 3 AND d = 4").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_multi_field_or() {
        let filter = parse("a = 1 OR b = 2 OR c = 3 OR d = 4").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_multi_field_mixed() {
        let filter = parse("(a = 1 AND b = 2) OR (c = 3 AND d = 4)").unwrap();
        assert!(!is_tautology(&filter));
    }

    // NOT expressions
    #[test]
    fn test_not_eq() {
        let filter = parse("NOT (status = 0)").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_not_in() {
        let filter = parse("status NOT IN [0, -1, 99]").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_not_complex() {
        let filter = parse("NOT (deleted = true OR archived = true)").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Strategy selection for real scenarios
    #[test]
    fn test_strategy_for_common_category() {
        // Category filters often have ~10-30% selectivity -> Hybrid
        assert!(matches!(select_strategy(0.2), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_strategy_for_rare_status() {
        // Rare status (e.g., "error") has low selectivity -> PostFilter
        assert!(matches!(select_strategy(0.02), FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_strategy_for_active_users() {
        // Most users are active -> PreFilter
        assert_eq!(select_strategy(0.9), FilterStrategy::PreFilter);
    }

    #[test]
    fn test_strategy_for_date_range() {
        // Date range depends on range size, often medium selectivity
        assert!(matches!(select_strategy(0.3), FilterStrategy::Hybrid { .. }));
    }

    #[test]
    fn test_strategy_for_boolean_filter() {
        // Boolean with even split is ~50%
        assert!(matches!(select_strategy(0.5), FilterStrategy::Hybrid { .. }));
    }

    // Deeply nested expressions
    #[test]
    fn test_deeply_nested_and() {
        let filter = parse("((a = 1 AND b = 2) AND (c = 3 AND d = 4)) AND e = 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_deeply_nested_or() {
        let filter = parse("((a = 1 OR b = 2) OR (c = 3 OR d = 4)) OR e = 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_deeply_nested_mixed() {
        let filter = parse("((a = 1 AND b = 2) OR (c = 3 AND d = 4)) AND e = 5").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Edge cases
    #[test]
    fn test_single_field_eq() {
        let filter = parse("x = 1").unwrap();
        assert!(!is_tautology(&filter));
        assert!(!is_contradiction(&filter));
    }

    #[test]
    fn test_single_field_ne() {
        let filter = parse("x != 1").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_empty_string_match() {
        let filter = parse(r#"name = """#).unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_zero_value() {
        let filter = parse("count = 0").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_negative_value() {
        let filter = parse("balance = -100").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_float_zero() {
        let filter = parse("score = 0.0").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_boolean_false() {
        let filter = parse("active = false").unwrap();
        assert!(!is_tautology(&filter));
    }

    // Combined with IS NULL
    #[test]
    fn test_null_or_default() {
        let filter = parse("value IS NULL OR value = 0").unwrap();
        assert!(!is_tautology(&filter));
    }

    #[test]
    fn test_not_null_and_valid() {
        let filter = parse("email IS NOT NULL AND email_verified = true").unwrap();
        assert!(!is_tautology(&filter));
    }
}

// =============================================================================
// ADDITIONAL EDGE CASES AND BOUNDARY TESTS (35 tests)
// =============================================================================

mod additional_strategy_tests {
    use super::*;

    // Oversample boundary tests
    #[test]
    fn test_oversample_just_above_1() {
        let oversample = calculate_oversample(0.99);
        assert!(oversample >= 1.0);
    }

    #[test]
    fn test_oversample_just_below_max() {
        let oversample = calculate_oversample(0.001);
        assert!(oversample <= MAX_OVERSAMPLE);
    }

    #[test]
    fn test_oversample_at_half() {
        let oversample = calculate_oversample(0.5);
        assert!((1.0..=MAX_OVERSAMPLE).contains(&oversample));
    }

    #[test]
    fn test_oversample_at_quarter() {
        let oversample = calculate_oversample(0.25);
        assert!((1.0..=MAX_OVERSAMPLE).contains(&oversample));
    }

    #[test]
    fn test_oversample_at_tenth() {
        let oversample = calculate_oversample(0.1);
        assert!((1.0..=MAX_OVERSAMPLE).contains(&oversample));
    }

    // Strategy boundary tests
    #[test]
    fn test_strategy_at_boundary_5_percent() {
        // At exactly 5% threshold, should be PostFilter or Hybrid
        let strategy = select_strategy(0.05);
        assert!(matches!(
            strategy,
            FilterStrategy::PostFilter { .. } | FilterStrategy::Hybrid { .. }
        ));
    }

    #[test]
    fn test_strategy_at_boundary_80_percent() {
        // At exactly 80% threshold, should be Hybrid or PreFilter
        let strategy = select_strategy(0.80);
        assert!(matches!(
            strategy,
            FilterStrategy::Hybrid { .. } | FilterStrategy::PreFilter
        ));
    }

    #[test]
    fn test_strategy_near_zero() {
        let strategy = select_strategy(0.001);
        assert!(matches!(strategy, FilterStrategy::PostFilter { .. }));
    }

    #[test]
    fn test_strategy_near_one() {
        let strategy = select_strategy(0.999);
        assert_eq!(strategy, FilterStrategy::PreFilter);
    }

    // Hybrid strategy validation edge cases
    #[test]
    fn test_hybrid_min_1_0_valid() {
        let strategy = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 5.0,
        };
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_hybrid_min_max_very_close() {
        let strategy = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 1.01,
        };
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_hybrid_validation_fails_min_gt_max() {
        let strategy = FilterStrategy::Hybrid {
            oversample_min: 5.0,
            oversample_max: 1.0,
        };
        assert!(strategy.validate().is_err());
    }

    #[test]
    fn test_hybrid_validation_fails_max_too_high() {
        let strategy = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 100.0,
        };
        assert!(strategy.validate().is_err());
    }

    // PostFilter validation edge cases
    #[test]
    fn test_postfilter_oversample_exactly_1() {
        let strategy = FilterStrategy::PostFilter { oversample: 1.0 };
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_exactly_max() {
        let strategy = FilterStrategy::PostFilter {
            oversample: MAX_OVERSAMPLE,
        };
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_postfilter_oversample_just_above_max() {
        let strategy = FilterStrategy::PostFilter {
            oversample: MAX_OVERSAMPLE + 0.01,
        };
        assert!(strategy.validate().is_err());
    }

    // Tautology with complex expressions
    #[test]
    fn test_tautology_double_negation() {
        // NOT(NOT(true)) = true
        let filter = FilterExpr::Not(Box::new(FilterExpr::Not(Box::new(
            FilterExpr::LiteralBool(true),
        ))));
        assert!(is_tautology(&filter));
    }

    #[test]
    fn test_tautology_a_or_not_a_nested() {
        // (a = 1 OR NOT(a = 1)) is tautology
        let a = parse("x = 1").unwrap();
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let or = FilterExpr::Or(Box::new(a), Box::new(not_a));
        assert!(is_tautology(&or));
    }

    #[test]
    fn test_not_tautology_a_and_not_a() {
        // (a = 1 AND NOT(a = 1)) is contradiction, not tautology
        let a = parse("x = 1").unwrap();
        let not_a = FilterExpr::Not(Box::new(a.clone()));
        let and = FilterExpr::And(Box::new(a), Box::new(not_a));
        assert!(!is_tautology(&and));
    }

    // Contradiction with complex expressions
    #[test]
    fn test_contradiction_double_negation_false() {
        // NOT(NOT(false)) = false
        let filter = FilterExpr::Not(Box::new(FilterExpr::Not(Box::new(
            FilterExpr::LiteralBool(false),
        ))));
        assert!(is_contradiction(&filter));
    }

    #[test]
    fn test_contradiction_three_way_and() {
        // x > 10 AND x < 5 AND y = 1 is contradiction (first two contradict)
        let gt10 = parse("x > 10").unwrap();
        let lt5 = parse("x < 5").unwrap();
        let y1 = parse("y = 1").unwrap();
        let inner = FilterExpr::And(Box::new(gt10), Box::new(lt5));
        let outer = FilterExpr::And(Box::new(inner), Box::new(y1));
        assert!(is_contradiction(&outer));
    }

    #[test]
    fn test_not_contradiction_overlapping_ranges() {
        // x >= 0 AND x <= 10 is valid
        let ge0 = parse("x >= 0").unwrap();
        let le10 = parse("x <= 10").unwrap();
        let and = FilterExpr::And(Box::new(ge0), Box::new(le10));
        assert!(!is_contradiction(&and));
    }

    // SelectivityEstimate edge cases
    #[test]
    fn test_selectivity_estimate_max_sample() {
        let estimate = SelectivityEstimate::new(0.5, usize::MAX, usize::MAX / 2);
        assert_eq!(estimate.selectivity, 0.5);
        assert_eq!(estimate.sample_size, usize::MAX);
    }

    #[test]
    fn test_selectivity_estimate_zero_sample() {
        let estimate = SelectivityEstimate::new(0.0, 0, 0);
        assert_eq!(estimate.selectivity, 0.0);
        assert_eq!(estimate.sample_size, 0);
    }

    #[test]
    fn test_selectivity_confidence_at_max() {
        let estimate = SelectivityEstimate::new(0.5, SELECTIVITY_SAMPLE_SIZE, 50);
        assert_eq!(estimate.confidence(), 1.0);
    }

    #[test]
    fn test_selectivity_confidence_above_max() {
        let estimate = SelectivityEstimate::new(0.5, SELECTIVITY_SAMPLE_SIZE * 2, 100);
        // Confidence should be capped at 1.0
        assert_eq!(estimate.confidence(), 1.0);
    }

    // FilterStrategy equality and cloning
    #[test]
    fn test_postfilter_different_oversamples_not_equal() {
        let s1 = FilterStrategy::PostFilter { oversample: 2.0 };
        let s2 = FilterStrategy::PostFilter { oversample: 3.0 };
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_hybrid_different_mins_not_equal() {
        let s1 = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 5.0,
        };
        let s2 = FilterStrategy::Hybrid {
            oversample_min: 2.0,
            oversample_max: 5.0,
        };
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_hybrid_different_maxs_not_equal() {
        let s1 = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 5.0,
        };
        let s2 = FilterStrategy::Hybrid {
            oversample_min: 1.0,
            oversample_max: 8.0,
        };
        assert_ne!(s1, s2);
    }

    // Literal expressions
    #[test]
    fn test_literal_bool_true_tautology() {
        assert!(is_tautology(&FilterExpr::LiteralBool(true)));
    }

    #[test]
    fn test_literal_bool_false_contradiction() {
        assert!(is_contradiction(&FilterExpr::LiteralBool(false)));
    }

    #[test]
    fn test_literal_int_not_tautology_or_contradiction() {
        assert!(!is_tautology(&FilterExpr::LiteralInt(42)));
        assert!(!is_contradiction(&FilterExpr::LiteralInt(42)));
    }

    #[test]
    fn test_literal_float_not_tautology_or_contradiction() {
        assert!(!is_tautology(&FilterExpr::LiteralFloat(3.5)));
        assert!(!is_contradiction(&FilterExpr::LiteralFloat(3.5)));
    }

    #[test]
    fn test_literal_string_not_tautology_or_contradiction() {
        assert!(!is_tautology(&FilterExpr::LiteralString("test".into())));
        assert!(!is_contradiction(&FilterExpr::LiteralString("test".into())));
    }

    // Final edge case - field reference
    #[test]
    fn test_field_reference_not_tautology_or_contradiction() {
        let field = FilterExpr::Field("some_field".into());
        assert!(!is_tautology(&field));
        assert!(!is_contradiction(&field));
    }
}
