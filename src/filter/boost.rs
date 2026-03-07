//! Metadata boost module for entity-enhanced RAG.
//!
//! Provides multiplicative distance boosting based on metadata field matches.
//! A matched boost reduces distance (promoting the result), while negative
//! weights increase distance (penalizing the result).
//!
//! # Formula
//!
//! ```text
//! boost_factor = sum(boost_i.compute_boost(metadata)).clamp(-1.0, 0.99)
//! final_distance = raw_distance * (1.0 - boost_factor)
//! ```
//!
//! This is **scale-independent**: a weight of 0.3 reduces distance by 30%
//! regardless of whether L2 distances are 0.001 or 50000.

use crate::metadata::MetadataValue;
use std::collections::HashMap;
use std::fmt;

/// Error type for boost construction.
#[derive(Debug, Clone, PartialEq)]
pub enum BoostError {
    /// Weight is NaN or infinite.
    InvalidWeight(f32),
}

impl fmt::Display for BoostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoostError::InvalidWeight(w) => write!(f, "invalid boost weight: {w} (must be finite)"),
        }
    }
}

impl std::error::Error for BoostError {}

/// A metadata-based distance boost.
///
/// When the specified `field` in a vector's metadata matches `value`,
/// the boost's `weight` is contributed to the total boost factor.
/// Type mismatches silently return 0.0 (no effect).
///
/// For `StringArray` fields, matching uses `contains()` — the boost
/// fires if the target value appears anywhere in the array.
///
/// # Example
///
/// ```rust
/// use edgevec::filter::boost::MetadataBoost;
/// use edgevec::metadata::MetadataValue;
///
/// let boost = MetadataBoost::new(
///     "entity_type".to_string(),
///     MetadataValue::String("ORG".to_string()),
///     0.3,
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct MetadataBoost {
    /// Metadata field name to match against.
    pub field: String,
    /// Value to match. For StringArray fields, matches via contains().
    pub value: MetadataValue,
    /// Boost weight. Positive = promote (reduce distance), negative = penalize.
    pub weight: f32,
}

impl MetadataBoost {
    /// Create a new metadata boost.
    ///
    /// # Errors
    ///
    /// Returns `BoostError::InvalidWeight` if weight is NaN or infinite.
    pub fn new(field: String, value: MetadataValue, weight: f32) -> Result<Self, BoostError> {
        if !weight.is_finite() {
            return Err(BoostError::InvalidWeight(weight));
        }
        Ok(Self {
            field,
            value,
            weight,
        })
    }

    /// Compute the boost contribution for a given metadata map.
    ///
    /// Returns `weight` if the metadata field matches `value`, otherwise `0.0`.
    /// Type mismatches return `0.0` (no error, no effect).
    /// For `StringArray` fields, matches if the array contains the target string.
    #[must_use]
    pub fn compute_boost(&self, metadata: &HashMap<String, MetadataValue>) -> f32 {
        let Some(field_value) = metadata.get(&self.field) else {
            return 0.0;
        };

        if self.matches(field_value) {
            self.weight
        } else {
            0.0
        }
    }

    /// Check if a metadata value matches the boost target.
    fn matches(&self, field_value: &MetadataValue) -> bool {
        match (&self.value, field_value) {
            // Exact match for same types
            (MetadataValue::String(target), MetadataValue::String(actual)) => target == actual,
            (MetadataValue::Integer(target), MetadataValue::Integer(actual)) => target == actual,
            (MetadataValue::Boolean(target), MetadataValue::Boolean(actual)) => target == actual,
            // Float comparison with epsilon
            (MetadataValue::Float(target), MetadataValue::Float(actual)) => {
                (target - actual).abs() < f64::EPSILON
            }
            // StringArray: check if array contains the target string
            (MetadataValue::String(target), MetadataValue::StringArray(arr)) => {
                arr.iter().any(|s| s == target)
            }
            // Type mismatch: no effect
            _ => false,
        }
    }
}

/// Compute the total boost factor from a slice of boosts.
///
/// Individual boost contributions are **additively stacked**: if two boosts
/// of weight 0.3 and 0.2 both match, the total factor is 0.5 (not 0.3×0.2).
/// The sum is clamped to `[-1.0, 0.99]`.
/// The upper clamp at 0.99 prevents distance from going to zero or negative.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn compute_boost_factor(
    boosts: &[MetadataBoost],
    metadata: &HashMap<String, MetadataValue>,
) -> f32 {
    let sum: f32 = boosts.iter().map(|b| b.compute_boost(metadata)).sum();
    sum.clamp(-1.0, 0.99)
}

/// Apply boost factor to a raw distance.
///
/// `final_distance = raw_distance * (1.0 - boost_factor)`
///
/// A positive boost_factor reduces distance (promotes result).
/// A negative boost_factor increases distance (penalizes result).
#[inline]
#[must_use]
pub fn apply_boost(raw_distance: f32, boost_factor: f32) -> f32 {
    raw_distance * (1.0 - boost_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metadata(pairs: Vec<(&str, MetadataValue)>) -> HashMap<String, MetadataValue> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn test_boost_single_field_match() {
        let boost = MetadataBoost::new(
            "entity_type".into(),
            MetadataValue::String("ORG".into()),
            0.3,
        )
        .unwrap();
        let meta = make_metadata(vec![("entity_type", MetadataValue::String("ORG".into()))]);
        assert!((boost.compute_boost(&meta) - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_multiple_fields() {
        let b1 = MetadataBoost::new(
            "entity_type".into(),
            MetadataValue::String("ORG".into()),
            0.3,
        )
        .unwrap();
        let b2 =
            MetadataBoost::new("is_verified".into(), MetadataValue::Boolean(true), 0.2).unwrap();
        let meta = make_metadata(vec![
            ("entity_type", MetadataValue::String("ORG".into())),
            ("is_verified", MetadataValue::Boolean(true)),
        ]);
        let factor = compute_boost_factor(&[b1, b2], &meta);
        assert!((factor - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_no_match_no_effect() {
        let boost = MetadataBoost::new(
            "entity_type".into(),
            MetadataValue::String("ORG".into()),
            0.3,
        )
        .unwrap();
        let meta = make_metadata(vec![(
            "entity_type",
            MetadataValue::String("PERSON".into()),
        )]);
        assert!((boost.compute_boost(&meta)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_all_match_reranks() {
        let boost =
            MetadataBoost::new("topic".into(), MetadataValue::String("space".into()), 0.4).unwrap();
        let meta = make_metadata(vec![("topic", MetadataValue::String("space".into()))]);
        let raw_distance = 10.0;
        let factor = compute_boost_factor(&[boost], &meta);
        let final_d = apply_boost(raw_distance, factor);
        // 10.0 * (1.0 - 0.4) = 6.0
        assert!((final_d - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_weight_zero_neutral() {
        let boost = MetadataBoost::new("x".into(), MetadataValue::Integer(1), 0.0).unwrap();
        let meta = make_metadata(vec![("x", MetadataValue::Integer(1))]);
        assert!((boost.compute_boost(&meta)).abs() < f32::EPSILON);
        let final_d = apply_boost(5.0, compute_boost_factor(&[boost], &meta));
        assert!((final_d - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_negative_weight_penalty() {
        let boost = MetadataBoost::new("spam".into(), MetadataValue::Boolean(true), -0.5).unwrap();
        let meta = make_metadata(vec![("spam", MetadataValue::Boolean(true))]);
        let factor = compute_boost_factor(&[boost], &meta);
        assert!((factor - (-0.5)).abs() < f32::EPSILON);
        let final_d = apply_boost(10.0, factor);
        // 10.0 * (1.0 - (-0.5)) = 10.0 * 1.5 = 15.0
        assert!((final_d - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_match_vs_no_match_independent() {
        // Verify same boost returns weight on match and 0.0 on mismatch
        let boost = MetadataBoost::new(
            "category".into(),
            MetadataValue::String("tech".into()),
            0.25,
        )
        .unwrap();
        let meta_match = make_metadata(vec![("category", MetadataValue::String("tech".into()))]);
        let meta_no = make_metadata(vec![("category", MetadataValue::String("sports".into()))]);
        assert!((boost.compute_boost(&meta_match) - 0.25).abs() < f32::EPSILON);
        assert!((boost.compute_boost(&meta_no)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_large_weight_dominates() {
        // Large positive weight gets clamped to 0.99
        let boost = MetadataBoost::new("x".into(), MetadataValue::Integer(1), 5.0).unwrap();
        let meta = make_metadata(vec![("x", MetadataValue::Integer(1))]);
        let factor = compute_boost_factor(&[boost], &meta);
        assert!((factor - 0.99).abs() < f32::EPSILON);
        let final_d = apply_boost(100.0, factor);
        // 100.0 * (1.0 - 0.99) = 1.0 (f32 rounding tolerance)
        assert!((final_d - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_boost_type_mismatch_ignored() {
        // Boost expects String, metadata has Integer — should return 0.0
        let boost =
            MetadataBoost::new("field".into(), MetadataValue::String("value".into()), 0.5).unwrap();
        let meta = make_metadata(vec![("field", MetadataValue::Integer(42))]);
        assert!((boost.compute_boost(&meta)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_string_array_contains() {
        let boost =
            MetadataBoost::new("entities".into(), MetadataValue::String("NASA".into()), 0.3)
                .unwrap();
        let meta = make_metadata(vec![(
            "entities",
            MetadataValue::StringArray(vec!["SpaceX".into(), "NASA".into(), "ESA".into()]),
        )]);
        assert!((boost.compute_boost(&meta) - 0.3).abs() < f32::EPSILON);

        // Non-matching array
        let meta_no = make_metadata(vec![(
            "entities",
            MetadataValue::StringArray(vec!["SpaceX".into(), "ESA".into()]),
        )]);
        assert!((boost.compute_boost(&meta_no)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_construction_validation() {
        // Verify MetadataBoost construction, Clone, and error handling
        let boost = MetadataBoost::new(
            "entity_type".into(),
            MetadataValue::String("ORG".into()),
            0.3,
        )
        .unwrap();
        // Struct fields are accessible and Clone works (needed for WASM boundary)
        let cloned = boost.clone();
        assert_eq!(cloned.field, "entity_type");
        assert!((cloned.weight - 0.3).abs() < f32::EPSILON);

        // BoostError Display works
        let err = MetadataBoost::new("x".into(), MetadataValue::Integer(1), f32::NAN);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("invalid boost weight"));

        // Inf also rejected
        assert!(MetadataBoost::new("x".into(), MetadataValue::Integer(1), f32::INFINITY).is_err());
        assert!(
            MetadataBoost::new("x".into(), MetadataValue::Integer(1), f32::NEG_INFINITY).is_err()
        );
    }

    #[test]
    fn test_boost_empty_metadata_map() {
        let boost = MetadataBoost::new(
            "entity_type".into(),
            MetadataValue::String("ORG".into()),
            0.3,
        )
        .unwrap();
        let empty_meta: HashMap<String, MetadataValue> = HashMap::new();
        assert!((boost.compute_boost(&empty_meta)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_boost_absent_field() {
        let boost = MetadataBoost::new(
            "missing_field".into(),
            MetadataValue::String("value".into()),
            0.5,
        )
        .unwrap();
        let meta = make_metadata(vec![("other_field", MetadataValue::String("value".into()))]);
        assert!((boost.compute_boost(&meta)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_boost_factor_empty_boosts() {
        let meta = make_metadata(vec![("x", MetadataValue::Integer(1))]);
        let factor = compute_boost_factor(&[], &meta);
        assert!((factor).abs() < f32::EPSILON);
    }
}
