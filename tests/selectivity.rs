//! Unit tests for heuristic-based selectivity estimation (W26.3.1)
//!
//! Tests static selectivity heuristics per RFC-002 §3.2.

use edgevec::filter::{parse, FilterExpr};

// Import the heuristic selectivity function (to be implemented)
use edgevec::filter::strategy::estimate_filter_selectivity;

// =============================================================================
// Heuristic Selectivity Tests
// =============================================================================

mod heuristic_selectivity {
    use super::*;

    /// Test: Equality filter is highly selective (~10%)
    #[test]
    fn test_equality_is_selective() {
        let filter = parse("category = \"books\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.10).abs() < 0.01,
            "Equality should have ~10% selectivity, got {}",
            selectivity
        );
    }

    /// Test: Not-equals is inverse of equality (~90%)
    #[test]
    fn test_not_equals_is_not_selective() {
        let filter = parse("category != \"books\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.90).abs() < 0.01,
            "NotEquals should have ~90% selectivity, got {}",
            selectivity
        );
    }

    /// Test: Range filters are moderately selective (~30%)
    #[test]
    fn test_range_is_moderately_selective() {
        let filter_lt = parse("price < 100").unwrap();
        let filter_gt = parse("price > 50").unwrap();

        let selectivity_lt = estimate_filter_selectivity(&filter_lt);
        let selectivity_gt = estimate_filter_selectivity(&filter_gt);

        assert!(
            (selectivity_lt - 0.30).abs() < 0.01,
            "LessThan should have ~30% selectivity, got {}",
            selectivity_lt
        );
        assert!(
            (selectivity_gt - 0.30).abs() < 0.01,
            "GreaterThan should have ~30% selectivity, got {}",
            selectivity_gt
        );
    }

    /// Test: LessOrEqual/GreaterOrEqual are slightly less selective (~35%)
    #[test]
    fn test_range_inclusive_slightly_less_selective() {
        let filter_le = parse("price <= 100").unwrap();
        let filter_ge = parse("price >= 50").unwrap();

        let selectivity_le = estimate_filter_selectivity(&filter_le);
        let selectivity_ge = estimate_filter_selectivity(&filter_ge);

        assert!(
            (selectivity_le - 0.35).abs() < 0.01,
            "LessOrEqual should have ~35% selectivity, got {}",
            selectivity_le
        );
        assert!(
            (selectivity_ge - 0.35).abs() < 0.01,
            "GreaterOrEqual should have ~35% selectivity, got {}",
            selectivity_ge
        );
    }

    /// Test: AND multiplies selectivities
    #[test]
    fn test_and_multiplies_selectivities() {
        // category = "books" (0.10) AND price < 100 (0.30)
        // Expected: 0.10 * 0.30 = 0.03
        let filter = parse("category = \"books\" AND price < 100").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.03).abs() < 0.01,
            "AND should multiply: 0.10 * 0.30 = 0.03, got {}",
            selectivity
        );
    }

    /// Test: OR uses union formula (sum - product)
    #[test]
    fn test_or_uses_union_formula() {
        // category = "books" (0.10) OR category = "movies" (0.10)
        // Expected: 0.10 + 0.10 - (0.10 * 0.10) = 0.19
        let filter = parse("category = \"books\" OR category = \"movies\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.19).abs() < 0.02,
            "OR should use union: 0.10 + 0.10 - 0.01 = 0.19, got {}",
            selectivity
        );
    }

    /// Test: NOT inverts selectivity
    #[test]
    fn test_not_inverts_selectivity() {
        // NOT (category = "books")
        // Expected: 1.0 - 0.10 = 0.90
        let filter = parse("NOT category = \"books\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.90).abs() < 0.01,
            "NOT should invert: 1.0 - 0.10 = 0.90, got {}",
            selectivity
        );
    }

    /// Test: Contains is moderately selective (~20%)
    #[test]
    fn test_contains_is_moderately_selective() {
        let filter = parse("description CONTAINS \"great\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.20).abs() < 0.01,
            "Contains should have ~20% selectivity, got {}",
            selectivity
        );
    }

    /// Test: Selectivity is always in [0.0, 1.0]
    #[test]
    fn test_selectivity_always_bounded() {
        // Complex filter that could exceed bounds without clamping
        let filter =
            parse("a = \"x\" OR b = \"y\" OR c = \"z\" OR d = \"w\" OR e = \"v\"").unwrap();
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (0.0..=1.0).contains(&selectivity),
            "Selectivity must be in [0.0, 1.0], got {}",
            selectivity
        );
    }

    /// Test: Default/unknown filter type uses conservative 0.50
    #[test]
    fn test_default_is_conservative() {
        // Literals are unknown filter patterns - should default to 0.50
        let filter = FilterExpr::LiteralBool(true);
        let selectivity = estimate_filter_selectivity(&filter);

        assert!(
            (selectivity - 0.50).abs() < 0.01,
            "Unknown patterns should default to 0.50, got {}",
            selectivity
        );
    }
}

// =============================================================================
// Overfetch Factor Tests
// =============================================================================

mod overfetch_factor {
    use edgevec::filter::strategy::overfetch_from_selectivity;

    /// Test: High selectivity (most pass) = low overfetch
    #[test]
    fn test_high_selectivity_low_overfetch() {
        // 100% selectivity = 1x overfetch (minimum 2)
        assert_eq!(overfetch_from_selectivity(1.0), 2);

        // 50% selectivity = 2x overfetch
        assert_eq!(overfetch_from_selectivity(0.50), 2);
    }

    /// Test: Low selectivity = high overfetch (capped at 10)
    #[test]
    fn test_low_selectivity_high_overfetch() {
        // 10% selectivity = 10x overfetch
        assert_eq!(overfetch_from_selectivity(0.10), 10);

        // 5% selectivity = capped at 10
        assert_eq!(overfetch_from_selectivity(0.05), 10);

        // 1% selectivity = capped at 10
        assert_eq!(overfetch_from_selectivity(0.01), 10);
    }

    /// Test: Medium selectivity = proportional overfetch
    #[test]
    fn test_medium_selectivity_proportional() {
        // 25% selectivity = 4x overfetch
        assert_eq!(overfetch_from_selectivity(0.25), 4);

        // 20% selectivity = 5x overfetch
        assert_eq!(overfetch_from_selectivity(0.20), 5);
    }

    /// Test: Overfetch is always in [2, 10]
    #[test]
    fn test_overfetch_always_bounded() {
        assert!(overfetch_from_selectivity(0.0) <= 10);
        assert!(overfetch_from_selectivity(0.001) <= 10);
        assert!(overfetch_from_selectivity(1.0) >= 2);
        assert!(overfetch_from_selectivity(2.0) >= 2);
    }
}

// =============================================================================
// Integration with search_filtered Tests
// =============================================================================

mod search_filtered_integration {
    use edgevec::hnsw::{HnswConfig, HnswIndex};
    use edgevec::metadata::MetadataValue;
    use edgevec::storage::VectorStorage;
    use std::collections::HashMap;

    fn create_test_index(dim: u32) -> (HnswIndex, VectorStorage) {
        let config = HnswConfig::new(dim);
        let storage = VectorStorage::new(&config, None);
        let index = HnswIndex::new(config, &storage).unwrap();
        (index, storage)
    }

    /// Test: search_filtered works with various filter types
    #[test]
    fn test_search_filtered_uses_heuristic() {
        let (mut index, mut storage) = create_test_index(4);

        // Insert 20 vectors with different categories
        for i in 0..20 {
            let category = if i < 2 {
                "rare" // 10% of vectors
            } else {
                "common"
            };
            let mut metadata = HashMap::new();
            metadata.insert(
                "category".to_string(),
                MetadataValue::String(category.to_string()),
            );
            index
                .insert_with_metadata(&mut storage, &[i as f32, 0.0, 0.0, 0.0], metadata)
                .unwrap();
        }

        // Search for rare category (10% selectivity should trigger higher overfetch)
        let query = [1.0, 0.0, 0.0, 0.0];
        let results = index
            .search_filtered(&storage, &query, "category = \"rare\"", 2)
            .unwrap();

        // Should find both rare vectors
        assert_eq!(results.len(), 2);
    }
}
