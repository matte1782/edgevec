//! Error types for sparse vector operations.

use thiserror::Error;

/// Errors that can occur during sparse vector operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SparseError {
    /// Indices are not sorted in ascending order.
    #[error("indices must be sorted in ascending order")]
    UnsortedIndices,

    /// Duplicate index found at the specified position.
    #[error("duplicate index at position {0}")]
    DuplicateIndex(usize),

    /// Index exceeds the vector dimension.
    #[error("index {index} exceeds dimension {dim}")]
    IndexOutOfBounds {
        /// The invalid index value.
        index: u32,
        /// The maximum allowed dimension.
        dim: u32,
    },

    /// Value at index is NaN or Infinity.
    #[error("value at index {0} is NaN or Infinity")]
    InvalidValue(usize),

    /// Sparse vector must have at least one element.
    #[error("sparse vector must have at least one element")]
    EmptyVector,

    /// Indices and values arrays have different lengths.
    #[error("indices and values length mismatch: {indices} vs {values}")]
    LengthMismatch {
        /// Length of indices array.
        indices: usize,
        /// Length of values array.
        values: usize,
    },

    /// Sparse ID not found in storage.
    #[error("sparse ID {0} not found")]
    IdNotFound(u64),

    /// Cannot normalize a zero vector.
    #[error("cannot normalize zero vector")]
    ZeroNorm,

    /// IO error during save/load operations.
    #[error("IO error: {0}")]
    Io(String),

    /// Invalid magic number in file header.
    #[error("invalid magic number: expected {expected:?}, found {found:?}")]
    InvalidMagic {
        /// Expected magic bytes.
        expected: [u8; 4],
        /// Found magic bytes.
        found: [u8; 4],
    },

    /// Unsupported format version.
    #[error("unsupported format version: expected {expected}, found {found}")]
    UnsupportedVersion {
        /// Expected version number.
        expected: u32,
        /// Found version number.
        found: u32,
    },

    /// Data corruption detected during load.
    #[error("corrupted data: {0}")]
    CorruptedData(String),

    /// ID counter overflow (u64::MAX reached).
    #[error("ID counter overflow: cannot assign more IDs (u64::MAX reached)")]
    IdOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_unsorted() {
        let err = SparseError::UnsortedIndices;
        assert!(err.to_string().contains("sorted"));
    }

    #[test]
    fn test_error_display_duplicate() {
        let err = SparseError::DuplicateIndex(5);
        assert!(err.to_string().contains('5'));
    }

    #[test]
    fn test_error_display_out_of_bounds() {
        let err = SparseError::IndexOutOfBounds {
            index: 100,
            dim: 50,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_error_display_invalid_value() {
        let err = SparseError::InvalidValue(3);
        assert!(err.to_string().contains('3'));
    }

    #[test]
    fn test_error_display_empty() {
        let err = SparseError::EmptyVector;
        assert!(err.to_string().contains("at least one"));
    }

    #[test]
    fn test_error_display_length_mismatch() {
        let err = SparseError::LengthMismatch {
            indices: 5,
            values: 3,
        };
        assert!(err.to_string().contains('5'));
        assert!(err.to_string().contains('3'));
    }

    #[test]
    fn test_error_display_id_not_found() {
        let err = SparseError::IdNotFound(42);
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn test_error_display_zero_norm() {
        let err = SparseError::ZeroNorm;
        assert!(err.to_string().contains("zero"));
    }

    #[test]
    fn test_error_display_id_overflow() {
        let err = SparseError::IdOverflow;
        assert!(err.to_string().contains("overflow"));
    }
}
