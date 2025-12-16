//! Metadata-specific error types.
//!
//! This module defines all error types that can occur during metadata operations.
//! These errors are designed to be descriptive and actionable.

use thiserror::Error;

/// Errors that can occur during metadata operations.
///
/// Each variant provides detailed context about what went wrong and includes
/// relevant values to help diagnose issues.
///
/// # Example
///
/// ```rust
/// use edgevec::metadata::MetadataError;
///
/// let error = MetadataError::KeyTooLong { length: 300, max: 256 };
/// assert!(error.to_string().contains("300"));
/// ```
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MetadataError {
    /// Metadata key cannot be empty.
    #[error("metadata key cannot be empty")]
    EmptyKey,

    /// Metadata key exceeds maximum length.
    #[error("metadata key too long: {length} bytes (max {max})")]
    KeyTooLong {
        /// Actual length of the key in bytes.
        length: usize,
        /// Maximum allowed length in bytes.
        max: usize,
    },

    /// Metadata key contains invalid characters.
    #[error("invalid key format: '{key}' (must be alphanumeric + underscore)")]
    InvalidKeyFormat {
        /// The invalid key.
        key: String,
    },

    /// String value exceeds maximum length.
    #[error("string value too long: {length} bytes (max {max})")]
    StringValueTooLong {
        /// Actual length of the string in bytes.
        length: usize,
        /// Maximum allowed length in bytes.
        max: usize,
    },

    /// String array exceeds maximum element count.
    #[error("string array too long: {length} elements (max {max})")]
    ArrayTooLong {
        /// Actual number of elements.
        length: usize,
        /// Maximum allowed elements.
        max: usize,
    },

    /// Float value is invalid (NaN or Infinity).
    #[error("invalid float value: {reason}")]
    InvalidFloat {
        /// Reason why the float is invalid.
        reason: &'static str,
    },

    /// Vector has too many metadata keys.
    #[error("too many keys for vector {vector_id}: {count} (max {max})")]
    TooManyKeys {
        /// The vector ID.
        vector_id: u32,
        /// Current number of keys.
        count: usize,
        /// Maximum allowed keys.
        max: usize,
    },

    /// Vector not found in metadata store.
    #[error("vector {vector_id} not found")]
    VectorNotFound {
        /// The vector ID that was not found.
        vector_id: u32,
    },

    /// Key not found for a specific vector.
    #[error("key '{key}' not found for vector {vector_id}")]
    KeyNotFound {
        /// The vector ID.
        vector_id: u32,
        /// The key that was not found.
        key: String,
    },

    /// Serialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization failed.
    #[error("deserialization error: {0}")]
    Deserialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages_are_descriptive() {
        let error = MetadataError::EmptyKey;
        assert!(error.to_string().contains("empty"));

        let error = MetadataError::KeyTooLong {
            length: 300,
            max: 256,
        };
        assert!(error.to_string().contains("300"));
        assert!(error.to_string().contains("256"));

        let error = MetadataError::InvalidKeyFormat {
            key: "bad-key".to_string(),
        };
        assert!(error.to_string().contains("bad-key"));

        let error = MetadataError::StringValueTooLong {
            length: 70000,
            max: 65536,
        };
        assert!(error.to_string().contains("70000"));

        let error = MetadataError::ArrayTooLong {
            length: 2000,
            max: 1024,
        };
        assert!(error.to_string().contains("2000"));

        let error = MetadataError::InvalidFloat {
            reason: "NaN not allowed",
        };
        assert!(error.to_string().contains("NaN"));

        let error = MetadataError::TooManyKeys {
            vector_id: 42,
            count: 100,
            max: 64,
        };
        assert!(error.to_string().contains("42"));
        assert!(error.to_string().contains("100"));

        let error = MetadataError::VectorNotFound { vector_id: 99 };
        assert!(error.to_string().contains("99"));

        let error = MetadataError::KeyNotFound {
            vector_id: 5,
            key: "missing_key".to_string(),
        };
        assert!(error.to_string().contains("5"));
        assert!(error.to_string().contains("missing_key"));

        let error = MetadataError::Serialization("failed".to_string());
        assert!(error.to_string().contains("failed"));

        let error = MetadataError::Deserialization("corrupt".to_string());
        assert!(error.to_string().contains("corrupt"));
    }

    #[test]
    fn test_error_equality() {
        let e1 = MetadataError::EmptyKey;
        let e2 = MetadataError::EmptyKey;
        assert_eq!(e1, e2);

        let e3 = MetadataError::KeyTooLong {
            length: 100,
            max: 50,
        };
        let e4 = MetadataError::KeyTooLong {
            length: 100,
            max: 50,
        };
        assert_eq!(e3, e4);
    }

    #[test]
    fn test_error_clone() {
        let error = MetadataError::InvalidKeyFormat {
            key: "test".to_string(),
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_error_debug() {
        let error = MetadataError::EmptyKey;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("EmptyKey"));
    }
}
