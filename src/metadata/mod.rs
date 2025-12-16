//! Metadata storage system for EdgeVec.
//!
//! This module provides a type-safe metadata storage system for attaching
//! key-value pairs to vectors. Metadata enables filtering, sorting, and
//! enriching vector search results with additional context.
//!
//! # Features
//!
//! - **5 Value Types**: String, Integer, Float, Boolean, StringArray
//! - **Type-Safe**: Strongly typed values with accessor methods
//! - **Validated**: Keys and values are validated against limits
//! - **Serializable**: JSON serialization with clear type tags
//!
//! # Quick Start
//!
//! ```rust
//! use edgevec::metadata::{MetadataValue, MetadataStore};
//! use edgevec::metadata::validation::validate_key_value;
//!
//! // Create values using constructors
//! let title = MetadataValue::String("Hello World".to_string());
//! let count = MetadataValue::Integer(42);
//! let score = MetadataValue::Float(0.95);
//! let active = MetadataValue::Boolean(true);
//! let tags = MetadataValue::StringArray(vec!["rust".into(), "wasm".into()]);
//!
//! // Create values using From trait
//! let title: MetadataValue = "Hello World".into();
//! let count: MetadataValue = 42i64.into();
//! let active: MetadataValue = true.into();
//!
//! // Validate before storing
//! assert!(validate_key_value("title", &title).is_ok());
//!
//! // Check types
//! assert!(title.is_string());
//! assert!(count.is_integer());
//!
//! // Extract values
//! assert_eq!(title.as_string(), Some("Hello World"));
//! assert_eq!(count.as_integer(), Some(42));
//! ```
//!
//! # Value Types
//!
//! | Type | Rust | JSON | Use Case |
//! |:-----|:-----|:-----|:---------|
//! | String | `String` | `string` | Titles, descriptions |
//! | Integer | `i64` | `number` | Counts, IDs, timestamps |
//! | Float | `f64` | `number` | Scores, weights |
//! | Boolean | `bool` | `boolean` | Flags, filters |
//! | StringArray | `Vec<String>` | `string[]` | Tags, categories |
//!
//! # Validation Limits
//!
//! | Limit | Value | Description |
//! |:------|:------|:------------|
//! | Max keys per vector | 64 | Prevents memory bloat |
//! | Max key length | 256 bytes | Reasonable for field names |
//! | Max string value | 64KB | Prevents abuse |
//! | Max array elements | 1,024 | Prevents excessive arrays |
//!
//! # JSON Serialization
//!
//! Values serialize with adjacently-tagged representation:
//!
//! ```json
//! {"type": "string", "value": "hello"}
//! {"type": "integer", "value": 42}
//! {"type": "float", "value": 3.14}
//! {"type": "boolean", "value": true}
//! {"type": "string_array", "value": ["a", "b"]}
//! ```
//!
//! # Module Structure
//!
//! - `types` - Core `MetadataValue` enum
//! - `error` - `MetadataError` enum
//! - `validation` - Validation constants and functions
//! - `store` - `MetadataStore` for CRUD operations (Day 2)

mod error;
mod store;
mod types;
pub mod validation;

// Re-export public types at module level
pub use error::MetadataError;
pub use store::MetadataStore;
pub use types::MetadataValue;

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: verify all types can be created and serialized.
    #[test]
    fn test_all_types_serialize() {
        let values = vec![
            MetadataValue::String("hello".to_string()),
            MetadataValue::Integer(42),
            MetadataValue::Float(3.14159),
            MetadataValue::Boolean(true),
            MetadataValue::StringArray(vec!["a".into(), "b".into()]),
        ];

        for value in &values {
            let json = serde_json::to_string(value).unwrap();
            let parsed: MetadataValue = serde_json::from_str(&json).unwrap();
            assert_eq!(*value, parsed);
        }
    }

    /// Integration test: verify validation works with types.
    #[test]
    fn test_validation_integration() {
        use validation::{validate_key, validate_value};

        // Valid key
        assert!(validate_key("my_field").is_ok());

        // Valid values
        assert!(validate_value(&MetadataValue::String("test".into())).is_ok());
        assert!(validate_value(&MetadataValue::Integer(123)).is_ok());
        assert!(validate_value(&MetadataValue::Float(1.5)).is_ok());
        assert!(validate_value(&MetadataValue::Boolean(false)).is_ok());
        assert!(validate_value(&MetadataValue::StringArray(vec![])).is_ok());

        // Invalid float
        assert!(validate_value(&MetadataValue::Float(f64::NAN)).is_err());
    }

    /// Integration test: verify store can be created.
    #[test]
    fn test_store_creation() {
        let store = MetadataStore::new();
        assert!(store.is_empty());
    }

    /// Integration test: verify error types are accessible.
    #[test]
    fn test_error_types() {
        let error = MetadataError::EmptyKey;
        assert!(error.to_string().contains("empty"));
    }
}
