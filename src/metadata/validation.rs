//! Metadata validation rules and constants.
//!
//! This module defines validation rules for metadata keys and values to ensure
//! data integrity and prevent resource exhaustion.
//!
//! # Validation Constants
//!
//! | Constant | Value | Rationale |
//! |:---------|:------|:----------|
//! | `MAX_KEYS_PER_VECTOR` | 64 | Prevents memory bloat per vector |
//! | `MAX_KEY_LENGTH` | 256 | Reasonable for field names |
//! | `MAX_STRING_VALUE_LENGTH` | 65,536 | 64KB limit for text content |
//! | `MAX_STRING_ARRAY_LENGTH` | 1,024 | Prevents excessive arrays |
//!
//! # Validation Rules
//!
//! ## Keys
//! - Must not be empty
//! - Must not exceed 256 bytes
//! - Must not contain NULL bytes (security: prevents C-string/IndexedDB issues)
//! - Must be ASCII-only (security: prevents homoglyph attacks)
//! - Must contain only ASCII alphanumeric characters and underscores (`[a-zA-Z0-9_]`)
//!
//! ## Values
//! - Strings: Max 64KB
//! - String arrays: Max 1,024 elements, each max 64KB
//! - Floats: NaN and Infinity rejected
//! - Integers and Booleans: No constraints

use super::error::MetadataError;
use super::types::MetadataValue;

// =============================================================================
// VALIDATION CONSTANTS
// =============================================================================

/// Maximum number of metadata keys per vector.
///
/// This limit prevents memory bloat and ensures predictable performance.
/// 64 keys should be sufficient for most use cases while keeping memory
/// usage reasonable.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::validation::MAX_KEYS_PER_VECTOR;
///
/// assert_eq!(MAX_KEYS_PER_VECTOR, 64);
/// ```
pub const MAX_KEYS_PER_VECTOR: usize = 64;

/// Maximum length of a metadata key in bytes.
///
/// Keys are typically field names, so 256 bytes is generous.
/// This prevents memory exhaustion from extremely long keys.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::validation::MAX_KEY_LENGTH;
///
/// let valid_key = "my_metadata_field";
/// assert!(valid_key.len() <= MAX_KEY_LENGTH);
/// ```
pub const MAX_KEY_LENGTH: usize = 256;

/// Maximum length of a String metadata value in bytes.
///
/// 64KB allows substantial text content while preventing abuse.
/// For larger content, consider storing references/IDs instead.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::validation::MAX_STRING_VALUE_LENGTH;
///
/// // 64KB limit
/// assert_eq!(MAX_STRING_VALUE_LENGTH, 65_536);
/// ```
pub const MAX_STRING_VALUE_LENGTH: usize = 65_536; // 64KB

/// Maximum number of elements in a StringArray.
///
/// 1,024 elements is sufficient for tags, categories, and similar
/// use cases while preventing memory bloat.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::validation::MAX_STRING_ARRAY_LENGTH;
///
/// let tags = vec!["tag1".to_string(); 100];
/// assert!(tags.len() <= MAX_STRING_ARRAY_LENGTH);
/// ```
pub const MAX_STRING_ARRAY_LENGTH: usize = 1_024;

// =============================================================================
// VALIDATION FUNCTIONS
// =============================================================================

/// Validates a metadata key.
///
/// # Rules
///
/// - Key must not be empty
/// - Key must not exceed `MAX_KEY_LENGTH` bytes
/// - Key must not contain NULL bytes (security)
/// - Key must be ASCII-only (security: prevents homoglyph attacks)
/// - Key must contain only ASCII alphanumeric characters and underscores (`[a-zA-Z0-9_]`)
///
/// # Arguments
///
/// * `key` - The metadata key to validate
///
/// # Returns
///
/// * `Ok(())` if the key is valid
/// * `Err(MetadataError)` if validation fails
///
/// # Errors
///
/// Returns an error if:
/// - Key is empty ([`MetadataError::EmptyKey`])
/// - Key exceeds `MAX_KEY_LENGTH` ([`MetadataError::KeyTooLong`])
/// - Key contains invalid characters ([`MetadataError::InvalidKeyFormat`])
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::validation::validate_key;
///
/// // Valid keys
/// assert!(validate_key("title").is_ok());
/// assert!(validate_key("page_count").is_ok());
/// assert!(validate_key("field123").is_ok());
///
/// // Invalid keys
/// assert!(validate_key("").is_err());  // Empty
/// assert!(validate_key("bad-key").is_err());  // Contains hyphen
/// assert!(validate_key("with spaces").is_err());  // Contains space
/// ```
pub fn validate_key(key: &str) -> Result<(), MetadataError> {
    // Rule 1: Key cannot be empty
    if key.is_empty() {
        return Err(MetadataError::EmptyKey);
    }

    // Rule 2: Key cannot exceed max length
    if key.len() > MAX_KEY_LENGTH {
        return Err(MetadataError::KeyTooLong {
            length: key.len(),
            max: MAX_KEY_LENGTH,
        });
    }

    // Rule 3: SECURITY - Reject NULL bytes (prevents C-string/IndexedDB issues)
    if key.contains('\0') {
        return Err(MetadataError::InvalidKeyFormat {
            key: key.replace('\0', "\\0"),
        });
    }

    // Rule 4: SECURITY - Keys must be ASCII-only (prevents homoglyph attacks)
    // This is stricter than is_alphanumeric() which allows Unicode letters
    if !key.is_ascii() {
        return Err(MetadataError::InvalidKeyFormat {
            key: key.to_string(),
        });
    }

    // Rule 5: Key must be ASCII alphanumeric + underscore only
    if !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(MetadataError::InvalidKeyFormat {
            key: key.to_string(),
        });
    }

    Ok(())
}

/// Validates a metadata value.
///
/// # Rules
///
/// - Strings: Must not exceed `MAX_STRING_VALUE_LENGTH` bytes
/// - String arrays: Must not exceed `MAX_STRING_ARRAY_LENGTH` elements
/// - String array elements: Must not exceed `MAX_STRING_VALUE_LENGTH` bytes
/// - Floats: Must not be NaN or Infinity
/// - Integers and Booleans: Always valid
///
/// # Arguments
///
/// * `value` - The metadata value to validate
///
/// # Returns
///
/// * `Ok(())` if the value is valid
/// * `Err(MetadataError)` if validation fails
///
/// # Errors
///
/// Returns an error if:
/// - String value exceeds `MAX_STRING_VALUE_LENGTH` ([`MetadataError::StringValueTooLong`])
/// - String array exceeds `MAX_STRING_ARRAY_LENGTH` ([`MetadataError::ArrayTooLong`])
/// - Float is NaN or Infinity ([`MetadataError::InvalidFloat`])
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::{MetadataValue, validation::validate_value};
///
/// // Valid values
/// assert!(validate_value(&MetadataValue::String("hello".into())).is_ok());
/// assert!(validate_value(&MetadataValue::Integer(42)).is_ok());
/// assert!(validate_value(&MetadataValue::Float(2.5)).is_ok());
/// assert!(validate_value(&MetadataValue::Boolean(true)).is_ok());
///
/// // Invalid values
/// assert!(validate_value(&MetadataValue::Float(f64::NAN)).is_err());
/// assert!(validate_value(&MetadataValue::Float(f64::INFINITY)).is_err());
/// ```
pub fn validate_value(value: &MetadataValue) -> Result<(), MetadataError> {
    match value {
        MetadataValue::String(s) => {
            if s.len() > MAX_STRING_VALUE_LENGTH {
                return Err(MetadataError::StringValueTooLong {
                    length: s.len(),
                    max: MAX_STRING_VALUE_LENGTH,
                });
            }
        }
        MetadataValue::StringArray(arr) => {
            // Check array length
            if arr.len() > MAX_STRING_ARRAY_LENGTH {
                return Err(MetadataError::ArrayTooLong {
                    length: arr.len(),
                    max: MAX_STRING_ARRAY_LENGTH,
                });
            }
            // Check each element's length
            for s in arr {
                if s.len() > MAX_STRING_VALUE_LENGTH {
                    return Err(MetadataError::StringValueTooLong {
                        length: s.len(),
                        max: MAX_STRING_VALUE_LENGTH,
                    });
                }
            }
        }
        MetadataValue::Float(f) => {
            if f.is_nan() {
                return Err(MetadataError::InvalidFloat {
                    reason: "NaN not allowed",
                });
            }
            if f.is_infinite() {
                return Err(MetadataError::InvalidFloat {
                    reason: "Infinity not allowed",
                });
            }
        }
        // Integer and Boolean have no validation constraints
        MetadataValue::Integer(_) | MetadataValue::Boolean(_) => {}
    }
    Ok(())
}

/// Validates both key and value together.
///
/// This is a convenience function that calls both `validate_key` and
/// `validate_value`.
///
/// # Arguments
///
/// * `key` - The metadata key to validate
/// * `value` - The metadata value to validate
///
/// # Returns
///
/// * `Ok(())` if both key and value are valid
/// * `Err(MetadataError)` if either validation fails
///
/// # Errors
///
/// Returns an error if key validation fails (see [`validate_key`]) or
/// if value validation fails (see [`validate_value`]).
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::{MetadataValue, validation::validate_key_value};
///
/// // Valid key-value pair
/// assert!(validate_key_value("title", &MetadataValue::String("Hello".into())).is_ok());
///
/// // Invalid key
/// assert!(validate_key_value("", &MetadataValue::String("Hello".into())).is_err());
///
/// // Invalid value
/// assert!(validate_key_value("score", &MetadataValue::Float(f64::NAN)).is_err());
/// ```
pub fn validate_key_value(key: &str, value: &MetadataValue) -> Result<(), MetadataError> {
    validate_key(key)?;
    validate_value(value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Key Validation Tests (Required by DAY_1_TASKS.md)
    // =========================================================================

    #[test]
    fn test_validate_key_empty() {
        assert!(validate_key("").is_err());
        assert!(matches!(validate_key(""), Err(MetadataError::EmptyKey)));
    }

    #[test]
    fn test_validate_key_too_long() {
        let long_key = "a".repeat(MAX_KEY_LENGTH + 1);
        assert!(validate_key(&long_key).is_err());
        assert!(matches!(
            validate_key(&long_key),
            Err(MetadataError::KeyTooLong { .. })
        ));
    }

    #[test]
    fn test_validate_key_valid() {
        assert!(validate_key("my_key_123").is_ok());
        assert!(validate_key("a").is_ok());
        assert!(validate_key("_").is_ok());
        assert!(validate_key("CamelCase").is_ok());
        assert!(validate_key("snake_case").is_ok());
        assert!(validate_key("UPPER_CASE").is_ok());
    }

    #[test]
    fn test_validate_key_max_length() {
        let max_key = "a".repeat(MAX_KEY_LENGTH);
        assert!(validate_key(&max_key).is_ok());
    }

    #[test]
    fn test_validate_key_invalid_chars() {
        assert!(validate_key("bad-key").is_err());
        assert!(validate_key("with spaces").is_err());
        assert!(validate_key("with.dot").is_err());
        assert!(validate_key("key@symbol").is_err());
        assert!(validate_key("key/slash").is_err());
    }

    #[test]
    fn test_validate_key_unicode_rejected() {
        // SECURITY: Unicode letters are rejected to prevent homoglyph attacks
        // Only ASCII alphanumeric + underscore allowed
        assert!(validate_key("日本語").is_err()); // Japanese - REJECTED
        assert!(validate_key("émoji").is_err()); // Accented chars - REJECTED
        assert!(validate_key("café").is_err()); // Latin with accent - REJECTED
        assert!(validate_key("usеr").is_err()); // Cyrillic 'е' homoglyph - REJECTED
    }

    #[test]
    fn test_validate_key_null_byte_rejected() {
        // SECURITY: NULL bytes rejected to prevent C-string/IndexedDB issues
        assert!(validate_key("valid\0key").is_err());
        assert!(validate_key("\0").is_err());
        assert!(validate_key("key\0").is_err());
    }

    #[test]
    fn test_validate_key_ascii_only() {
        // Only ASCII alphanumeric + underscore allowed
        assert!(validate_key("valid_key_123").is_ok());
        assert!(validate_key("UPPERCASE").is_ok());
        assert!(validate_key("lowercase").is_ok());
        assert!(validate_key("_leading_underscore").is_ok());
        assert!(validate_key("trailing_underscore_").is_ok());
        assert!(validate_key("__double__underscore__").is_ok());
    }

    // =========================================================================
    // Value Validation Tests (Required by DAY_1_TASKS.md)
    // =========================================================================

    #[test]
    fn test_validate_value_nan() {
        let value = MetadataValue::Float(f64::NAN);
        assert!(validate_value(&value).is_err());
        assert!(matches!(
            validate_value(&value),
            Err(MetadataError::InvalidFloat {
                reason: "NaN not allowed"
            })
        ));
    }

    #[test]
    fn test_validate_value_infinity() {
        let value = MetadataValue::Float(f64::INFINITY);
        assert!(validate_value(&value).is_err());
        assert!(matches!(
            validate_value(&value),
            Err(MetadataError::InvalidFloat {
                reason: "Infinity not allowed"
            })
        ));
    }

    #[test]
    fn test_validate_value_neg_infinity() {
        let value = MetadataValue::Float(f64::NEG_INFINITY);
        assert!(validate_value(&value).is_err());
    }

    #[test]
    fn test_validate_value_valid_float() {
        assert!(validate_value(&MetadataValue::Float(0.0)).is_ok());
        assert!(validate_value(&MetadataValue::Float(-1.5)).is_ok());
        assert!(validate_value(&MetadataValue::Float(f64::MAX)).is_ok());
        assert!(validate_value(&MetadataValue::Float(f64::MIN)).is_ok());
        assert!(validate_value(&MetadataValue::Float(f64::MIN_POSITIVE)).is_ok());
    }

    #[test]
    fn test_validate_value_string_too_long() {
        let long_string = "a".repeat(MAX_STRING_VALUE_LENGTH + 1);
        let value = MetadataValue::String(long_string);
        assert!(validate_value(&value).is_err());
    }

    #[test]
    fn test_validate_value_string_max_length() {
        let max_string = "a".repeat(MAX_STRING_VALUE_LENGTH);
        let value = MetadataValue::String(max_string);
        assert!(validate_value(&value).is_ok());
    }

    #[test]
    fn test_validate_value_array_too_long() {
        let long_array = vec!["a".to_string(); MAX_STRING_ARRAY_LENGTH + 1];
        let value = MetadataValue::StringArray(long_array);
        assert!(validate_value(&value).is_err());
    }

    #[test]
    fn test_validate_value_array_max_length() {
        let max_array = vec!["a".to_string(); MAX_STRING_ARRAY_LENGTH];
        let value = MetadataValue::StringArray(max_array);
        assert!(validate_value(&value).is_ok());
    }

    #[test]
    fn test_validate_value_array_element_too_long() {
        let long_element = "a".repeat(MAX_STRING_VALUE_LENGTH + 1);
        let value = MetadataValue::StringArray(vec![long_element]);
        assert!(validate_value(&value).is_err());
    }

    #[test]
    fn test_validate_value_integer_always_valid() {
        assert!(validate_value(&MetadataValue::Integer(0)).is_ok());
        assert!(validate_value(&MetadataValue::Integer(i64::MAX)).is_ok());
        assert!(validate_value(&MetadataValue::Integer(i64::MIN)).is_ok());
        assert!(validate_value(&MetadataValue::Integer(-1)).is_ok());
    }

    #[test]
    fn test_validate_value_boolean_always_valid() {
        assert!(validate_value(&MetadataValue::Boolean(true)).is_ok());
        assert!(validate_value(&MetadataValue::Boolean(false)).is_ok());
    }

    // =========================================================================
    // Combined Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_key_value_both_valid() {
        assert!(validate_key_value("title", &MetadataValue::String("Hello".into())).is_ok());
        assert!(validate_key_value("count", &MetadataValue::Integer(42)).is_ok());
    }

    #[test]
    fn test_validate_key_value_invalid_key() {
        assert!(validate_key_value("", &MetadataValue::String("Hello".into())).is_err());
    }

    #[test]
    fn test_validate_key_value_invalid_value() {
        assert!(validate_key_value("score", &MetadataValue::Float(f64::NAN)).is_err());
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_constants_have_reasonable_values() {
        // These tests document expected values
        assert_eq!(MAX_KEYS_PER_VECTOR, 64);
        assert_eq!(MAX_KEY_LENGTH, 256);
        assert_eq!(MAX_STRING_VALUE_LENGTH, 65_536);
        assert_eq!(MAX_STRING_ARRAY_LENGTH, 1_024);
    }
}
