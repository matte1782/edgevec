//! Core metadata types for EdgeVec.
//!
//! This module defines the `MetadataValue` enum which represents all supported
//! metadata value types in EdgeVec's vector database.
//!
//! # Supported Types
//!
//! EdgeVec metadata supports 5 value types optimized for common RAG and
//! vector search use cases:
//!
//! | Rust Type | JSON Type | TypeScript Type | Use Case |
//! |:----------|:----------|:----------------|:---------|
//! | `String` | `string` | `string` | Titles, descriptions |
//! | `Integer` | `number` | `number` | Counts, IDs, timestamps |
//! | `Float` | `number` | `number` | Scores, weights |
//! | `Boolean` | `boolean` | `boolean` | Flags, filters |
//! | `StringArray` | `string[]` | `string[]` | Tags, categories |
//!
//! # Serialization Format
//!
//! Values serialize to JSON using adjacently-tagged representation:
//!
//! ```json
//! {"type": "string", "value": "hello"}
//! {"type": "integer", "value": 42}
//! {"type": "float", "value": 2.5}
//! {"type": "boolean", "value": true}
//! {"type": "string_array", "value": ["a", "b", "c"]}
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported metadata value types.
///
/// EdgeVec metadata supports 5 value types optimized for common
/// RAG and vector search use cases.
///
/// # Type Mapping
///
/// | Rust Type | JSON Type | TypeScript Type |
/// |:----------|:----------|:----------------|
/// | String | string | string |
/// | Integer | number | number |
/// | Float | number | number |
/// | Boolean | boolean | boolean |
/// | StringArray | string[] | string[] |
///
/// # Serialization
///
/// Uses adjacently-tagged representation for unambiguous type preservation:
///
/// ```rust
/// use edgevec::metadata::MetadataValue;
///
/// let value = MetadataValue::Integer(42);
/// let json = serde_json::to_string(&value).unwrap();
/// assert_eq!(json, r#"{"type":"integer","value":42}"#);
/// ```
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::MetadataValue;
///
/// let title = MetadataValue::String("Document Title".to_string());
/// let page_count = MetadataValue::Integer(42);
/// let relevance = MetadataValue::Float(0.95);
/// let is_verified = MetadataValue::Boolean(true);
/// let tags = MetadataValue::StringArray(vec!["rust".to_string(), "wasm".to_string()]);
///
/// // Type checking
/// assert!(title.is_string());
/// assert!(page_count.is_integer());
/// assert!(relevance.is_float());
/// assert!(is_verified.is_boolean());
/// assert!(tags.is_string_array());
///
/// // Value extraction
/// assert_eq!(title.as_string(), Some("Document Title"));
/// assert_eq!(page_count.as_integer(), Some(42));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MetadataValue {
    /// UTF-8 string value (max 65,536 bytes).
    ///
    /// Use for titles, descriptions, content, and any text data.
    String(String),

    /// 64-bit signed integer.
    ///
    /// Use for counts, IDs, timestamps (Unix epoch), and numeric identifiers.
    Integer(i64),

    /// 64-bit IEEE 754 floating point.
    ///
    /// Use for scores, weights, probabilities, and continuous values.
    /// Note: NaN and Infinity are rejected during validation.
    Float(f64),

    /// Boolean true/false.
    ///
    /// Use for flags, binary filters, and on/off states.
    Boolean(bool),

    /// Array of UTF-8 strings (max 1,024 elements).
    ///
    /// Use for tags, categories, labels, and multi-value fields.
    StringArray(Vec<String>),
}

impl MetadataValue {
    /// Returns the type name as a static string.
    ///
    /// This matches the JSON serialization type field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// assert_eq!(MetadataValue::String("hi".into()).type_name(), "string");
    /// assert_eq!(MetadataValue::Integer(42).type_name(), "integer");
    /// assert_eq!(MetadataValue::Float(2.5).type_name(), "float");
    /// assert_eq!(MetadataValue::Boolean(true).type_name(), "boolean");
    /// assert_eq!(MetadataValue::StringArray(vec![]).type_name(), "string_array");
    /// ```
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            MetadataValue::String(_) => "string",
            MetadataValue::Integer(_) => "integer",
            MetadataValue::Float(_) => "float",
            MetadataValue::Boolean(_) => "boolean",
            MetadataValue::StringArray(_) => "string_array",
        }
    }

    /// Returns true if this value is a String type.
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, MetadataValue::String(_))
    }

    /// Returns true if this value is an Integer type.
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, MetadataValue::Integer(_))
    }

    /// Returns true if this value is a Float type.
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, MetadataValue::Float(_))
    }

    /// Returns true if this value is a Boolean type.
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, MetadataValue::Boolean(_))
    }

    /// Returns true if this value is a StringArray type.
    #[must_use]
    pub fn is_string_array(&self) -> bool {
        matches!(self, MetadataValue::StringArray(_))
    }

    /// Attempts to extract the String value.
    ///
    /// Returns `None` if this is not a String variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// let value = MetadataValue::String("hello".to_string());
    /// assert_eq!(value.as_string(), Some("hello"));
    ///
    /// let value = MetadataValue::Integer(42);
    /// assert_eq!(value.as_string(), None);
    /// ```
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetadataValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Attempts to extract the Integer value.
    ///
    /// Returns `None` if this is not an Integer variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// let value = MetadataValue::Integer(42);
    /// assert_eq!(value.as_integer(), Some(42));
    ///
    /// let value = MetadataValue::Float(2.5);
    /// assert_eq!(value.as_integer(), None);
    /// ```
    #[must_use]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            MetadataValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Attempts to extract the Float value.
    ///
    /// Returns `None` if this is not a Float variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// let value = MetadataValue::Float(2.5);
    /// assert_eq!(value.as_float(), Some(2.5));
    ///
    /// let value = MetadataValue::Integer(42);
    /// assert_eq!(value.as_float(), None);
    /// ```
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            MetadataValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Attempts to extract the Boolean value.
    ///
    /// Returns `None` if this is not a Boolean variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// let value = MetadataValue::Boolean(true);
    /// assert_eq!(value.as_boolean(), Some(true));
    ///
    /// let value = MetadataValue::Integer(1);
    /// assert_eq!(value.as_boolean(), None);
    /// ```
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            MetadataValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Attempts to extract the StringArray value.
    ///
    /// Returns `None` if this is not a StringArray variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::metadata::MetadataValue;
    ///
    /// let value = MetadataValue::StringArray(vec!["a".into(), "b".into()]);
    /// assert_eq!(value.as_string_array(), Some(&["a".to_string(), "b".to_string()][..]));
    ///
    /// let value = MetadataValue::String("not an array".to_string());
    /// assert_eq!(value.as_string_array(), None);
    /// ```
    #[must_use]
    pub fn as_string_array(&self) -> Option<&[String]> {
        match self {
            MetadataValue::StringArray(arr) => Some(arr),
            _ => None,
        }
    }
}

// Implement Display for human-readable output
impl fmt::Display for MetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataValue::String(s) => write!(f, "\"{s}\""),
            MetadataValue::Integer(i) => write!(f, "{i}"),
            MetadataValue::Float(fl) => write!(f, "{fl}"),
            MetadataValue::Boolean(b) => write!(f, "{b}"),
            MetadataValue::StringArray(arr) => {
                write!(f, "[")?;
                for (i, s) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{s}\"")?;
                }
                write!(f, "]")
            }
        }
    }
}

// Implement From<T> for common conversions
impl From<String> for MetadataValue {
    fn from(s: String) -> Self {
        MetadataValue::String(s)
    }
}

impl From<&str> for MetadataValue {
    fn from(s: &str) -> Self {
        MetadataValue::String(s.to_string())
    }
}

impl From<i64> for MetadataValue {
    fn from(i: i64) -> Self {
        MetadataValue::Integer(i)
    }
}

impl From<i32> for MetadataValue {
    fn from(i: i32) -> Self {
        MetadataValue::Integer(i64::from(i))
    }
}

impl From<f64> for MetadataValue {
    fn from(f: f64) -> Self {
        MetadataValue::Float(f)
    }
}

impl From<f32> for MetadataValue {
    fn from(f: f32) -> Self {
        MetadataValue::Float(f64::from(f))
    }
}

impl From<bool> for MetadataValue {
    fn from(b: bool) -> Self {
        MetadataValue::Boolean(b)
    }
}

impl From<Vec<String>> for MetadataValue {
    fn from(arr: Vec<String>) -> Self {
        MetadataValue::StringArray(arr)
    }
}

impl<const N: usize> From<[&str; N]> for MetadataValue {
    fn from(arr: [&str; N]) -> Self {
        MetadataValue::StringArray(arr.iter().map(|s| (*s).to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Serialization Roundtrip Tests (Required by DAY_1_TASKS.md)
    // =========================================================================

    #[test]
    fn test_metadata_value_string_roundtrip() {
        let value = MetadataValue::String("hello".to_string());
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_metadata_value_integer_roundtrip() {
        let value = MetadataValue::Integer(42);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_metadata_value_float_roundtrip() {
        let value = MetadataValue::Float(9.87654);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_metadata_value_boolean_roundtrip() {
        let value = MetadataValue::Boolean(true);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_metadata_value_string_array_roundtrip() {
        let value = MetadataValue::StringArray(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    // =========================================================================
    // JSON Format Verification (per DAY_1_TASKS.md:492-497)
    // =========================================================================

    #[test]
    fn test_json_format_string() {
        let value = MetadataValue::String("hello".to_string());
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"type":"string","value":"hello"}"#);
    }

    #[test]
    fn test_json_format_integer() {
        let value = MetadataValue::Integer(42);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"type":"integer","value":42}"#);
    }

    #[test]
    fn test_json_format_float() {
        let value = MetadataValue::Float(9.87654);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"type":"float","value":9.87654}"#);
    }

    #[test]
    fn test_json_format_boolean() {
        let value = MetadataValue::Boolean(true);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"type":"boolean","value":true}"#);
    }

    #[test]
    fn test_json_format_string_array() {
        let value =
            MetadataValue::StringArray(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"type":"string_array","value":["a","b","c"]}"#);
    }

    // =========================================================================
    // Type Check Methods Tests
    // =========================================================================

    #[test]
    fn test_is_string() {
        assert!(MetadataValue::String("test".into()).is_string());
        assert!(!MetadataValue::Integer(42).is_string());
        assert!(!MetadataValue::Float(2.5).is_string());
        assert!(!MetadataValue::Boolean(true).is_string());
        assert!(!MetadataValue::StringArray(vec![]).is_string());
    }

    #[test]
    fn test_is_integer() {
        assert!(!MetadataValue::String("test".into()).is_integer());
        assert!(MetadataValue::Integer(42).is_integer());
        assert!(!MetadataValue::Float(2.5).is_integer());
        assert!(!MetadataValue::Boolean(true).is_integer());
        assert!(!MetadataValue::StringArray(vec![]).is_integer());
    }

    #[test]
    fn test_is_float() {
        assert!(!MetadataValue::String("test".into()).is_float());
        assert!(!MetadataValue::Integer(42).is_float());
        assert!(MetadataValue::Float(2.5).is_float());
        assert!(!MetadataValue::Boolean(true).is_float());
        assert!(!MetadataValue::StringArray(vec![]).is_float());
    }

    #[test]
    fn test_is_boolean() {
        assert!(!MetadataValue::String("test".into()).is_boolean());
        assert!(!MetadataValue::Integer(42).is_boolean());
        assert!(!MetadataValue::Float(2.5).is_boolean());
        assert!(MetadataValue::Boolean(true).is_boolean());
        assert!(!MetadataValue::StringArray(vec![]).is_boolean());
    }

    #[test]
    fn test_is_string_array() {
        assert!(!MetadataValue::String("test".into()).is_string_array());
        assert!(!MetadataValue::Integer(42).is_string_array());
        assert!(!MetadataValue::Float(2.5).is_string_array());
        assert!(!MetadataValue::Boolean(true).is_string_array());
        assert!(MetadataValue::StringArray(vec![]).is_string_array());
    }

    // =========================================================================
    // Type Accessor Methods Tests
    // =========================================================================

    #[test]
    fn test_as_string() {
        assert_eq!(
            MetadataValue::String("hello".into()).as_string(),
            Some("hello")
        );
        assert_eq!(MetadataValue::Integer(42).as_string(), None);
    }

    #[test]
    fn test_as_integer() {
        assert_eq!(MetadataValue::Integer(42).as_integer(), Some(42));
        assert_eq!(MetadataValue::String("42".into()).as_integer(), None);
    }

    #[test]
    fn test_as_float() {
        assert_eq!(MetadataValue::Float(2.5).as_float(), Some(2.5));
        assert_eq!(MetadataValue::Integer(3).as_float(), None);
    }

    #[test]
    fn test_as_boolean() {
        assert_eq!(MetadataValue::Boolean(true).as_boolean(), Some(true));
        assert_eq!(MetadataValue::Integer(1).as_boolean(), None);
    }

    #[test]
    fn test_as_string_array() {
        let arr = vec!["a".to_string(), "b".to_string()];
        let value = MetadataValue::StringArray(arr.clone());
        assert_eq!(value.as_string_array(), Some(&arr[..]));
        assert_eq!(MetadataValue::String("[]".into()).as_string_array(), None);
    }

    // =========================================================================
    // Type Name Tests
    // =========================================================================

    #[test]
    fn test_type_name() {
        assert_eq!(MetadataValue::String(String::new()).type_name(), "string");
        assert_eq!(MetadataValue::Integer(0).type_name(), "integer");
        assert_eq!(MetadataValue::Float(0.0).type_name(), "float");
        assert_eq!(MetadataValue::Boolean(false).type_name(), "boolean");
        assert_eq!(
            MetadataValue::StringArray(vec![]).type_name(),
            "string_array"
        );
    }

    // =========================================================================
    // Display Trait Tests
    // =========================================================================

    #[test]
    fn test_display_string() {
        let value = MetadataValue::String("hello".into());
        assert_eq!(format!("{value}"), "\"hello\"");
    }

    #[test]
    fn test_display_integer() {
        let value = MetadataValue::Integer(42);
        assert_eq!(format!("{value}"), "42");
    }

    #[test]
    fn test_display_float() {
        let value = MetadataValue::Float(2.5);
        assert_eq!(format!("{value}"), "2.5");
    }

    #[test]
    fn test_display_boolean() {
        let value_true = MetadataValue::Boolean(true);
        let value_false = MetadataValue::Boolean(false);
        assert_eq!(format!("{value_true}"), "true");
        assert_eq!(format!("{value_false}"), "false");
    }

    #[test]
    fn test_display_string_array() {
        let value = MetadataValue::StringArray(vec!["a".into(), "b".into()]);
        assert_eq!(format!("{value}"), "[\"a\", \"b\"]");
    }

    #[test]
    fn test_display_empty_array() {
        let value = MetadataValue::StringArray(vec![]);
        assert_eq!(format!("{value}"), "[]");
    }

    // =========================================================================
    // From Trait Tests
    // =========================================================================

    #[test]
    fn test_from_string() {
        let value: MetadataValue = String::from("test").into();
        assert_eq!(value, MetadataValue::String("test".into()));
    }

    #[test]
    fn test_from_str() {
        let value: MetadataValue = "test".into();
        assert_eq!(value, MetadataValue::String("test".into()));
    }

    #[test]
    fn test_from_i64() {
        let value: MetadataValue = 42i64.into();
        assert_eq!(value, MetadataValue::Integer(42));
    }

    #[test]
    fn test_from_i32() {
        let value: MetadataValue = 42i32.into();
        assert_eq!(value, MetadataValue::Integer(42));
    }

    #[test]
    fn test_from_f64() {
        let value: MetadataValue = 2.5f64.into();
        assert_eq!(value, MetadataValue::Float(2.5));
    }

    #[test]
    fn test_from_f32() {
        let value: MetadataValue = 2.5f32.into();
        assert!(matches!(value, MetadataValue::Float(f) if (f - 2.5).abs() < 0.001));
    }

    #[test]
    fn test_from_bool() {
        let value: MetadataValue = true.into();
        assert_eq!(value, MetadataValue::Boolean(true));
    }

    #[test]
    fn test_from_vec_string() {
        let value: MetadataValue = vec!["a".to_string(), "b".to_string()].into();
        assert_eq!(
            value,
            MetadataValue::StringArray(vec!["a".into(), "b".into()])
        );
    }

    #[test]
    fn test_from_str_array() {
        let value: MetadataValue = ["a", "b", "c"].into();
        assert_eq!(
            value,
            MetadataValue::StringArray(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_empty_string() {
        let value = MetadataValue::String(String::new());
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
        assert_eq!(value.as_string(), Some(""));
    }

    #[test]
    fn test_empty_array() {
        let value = MetadataValue::StringArray(vec![]);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
        assert!(value.as_string_array().unwrap().is_empty());
    }

    #[test]
    fn test_negative_integer() {
        let value = MetadataValue::Integer(-999);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_large_integer() {
        let value = MetadataValue::Integer(i64::MAX);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_negative_float() {
        let value = MetadataValue::Float(-9.87654);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_zero_float() {
        let value = MetadataValue::Float(0.0);
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_unicode_string() {
        let value = MetadataValue::String("Hello, 世界! 🚀".to_string());
        let json = serde_json::to_string(&value).unwrap();
        let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_clone() {
        let original = MetadataValue::StringArray(vec!["a".into(), "b".into()]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
