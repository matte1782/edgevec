//! Filter error types for EdgeVec.
//!
//! This module defines comprehensive error types with position information
//! for the filter parsing and evaluation subsystem.
//!
//! # Error Categories
//!
//! | Category | Prefix | Description |
//! |:---------|:-------|:------------|
//! | Syntax | E0xx | Parse errors, invalid tokens |
//! | Type | E1xx | Type mismatches, invalid casts |
//! | Evaluation | E2xx | Runtime evaluation errors |
//! | Limit | E3xx | Resource limits exceeded |
//!
//! # Position Information
//!
//! All syntax errors include:
//! - `position`: Byte offset in input string
//! - `line`: 1-indexed line number
//! - `column`: 1-indexed column number
//!
//! This enables precise error highlighting in IDEs and editors.

use thiserror::Error;

/// Filter error with position information.
///
/// Represents all possible errors that can occur during filter parsing
/// and evaluation. Each error variant includes contextual information
/// to help diagnose and fix the issue.
///
/// # Error Codes
///
/// Error codes follow a structured pattern for easy categorization:
/// - `E001-E099`: Syntax errors (parsing)
/// - `E101-E199`: Type errors (type checking)
/// - `E201-E299`: Evaluation errors (runtime)
/// - `E301-E399`: Limit errors (resource constraints)
///
/// # Example
///
/// ```rust
/// use edgevec::filter::FilterError;
///
/// let error = FilterError::SyntaxError {
///     position: 10,
///     line: 1,
///     column: 11,
///     message: "Expected operator".to_string(),
///     suggestion: Some("Did you mean '=' instead of ':'?".to_string()),
/// };
///
/// assert_eq!(error.code(), "E001");
/// ```
#[derive(Debug, Clone, Error, PartialEq)]
pub enum FilterError {
    // ═══════════════════════════════════════════════════════════════════════
    // SYNTAX ERRORS (E0xx)
    // ═══════════════════════════════════════════════════════════════════════
    /// General syntax error during parsing.
    ///
    /// This is the catch-all error for parser failures that don't fit
    /// into more specific categories.
    #[error("Syntax error at position {position} (line {line}, column {column}): {message}")]
    SyntaxError {
        /// Byte offset in input string where error occurred.
        position: usize,
        /// 1-indexed line number.
        line: usize,
        /// 1-indexed column number.
        column: usize,
        /// Human-readable error message.
        message: String,
        /// Optional suggestion for fixing the error.
        suggestion: Option<String>,
    },

    /// Unexpected end of input while parsing.
    ///
    /// The parser expected more tokens but reached the end of the string.
    #[error("Unexpected end of input at position {position}: expected {expected}")]
    UnexpectedEof {
        /// Byte offset where input ended.
        position: usize,
        /// Description of what was expected.
        expected: String,
    },

    /// Invalid character encountered during parsing.
    ///
    /// The parser found a character that isn't valid in filter expressions.
    #[error("Invalid character '{char}' at position {position}")]
    InvalidChar {
        /// The invalid character.
        char: char,
        /// Byte offset where character was found.
        position: usize,
    },

    /// Unclosed string literal.
    ///
    /// A string literal was started but never closed with a matching quote.
    #[error("Unclosed string starting at position {position}")]
    UnclosedString {
        /// Byte offset where string started.
        position: usize,
    },

    /// Unclosed parenthesis.
    ///
    /// An opening parenthesis was found but never closed.
    #[error("Unclosed parenthesis at position {position}")]
    UnclosedParen {
        /// Byte offset where opening paren was found.
        position: usize,
    },

    /// Invalid escape sequence in string.
    ///
    /// An escape sequence like `\x` was found that isn't supported.
    #[error("Invalid escape sequence '\\{char}' at position {position}")]
    InvalidEscape {
        /// The character after the backslash.
        char: char,
        /// Byte offset of the backslash.
        position: usize,
    },

    /// Invalid number literal.
    ///
    /// A number was malformed (e.g., multiple decimal points).
    #[error("Invalid number '{value}' at position {position}")]
    InvalidNumber {
        /// The malformed number text.
        value: String,
        /// Byte offset where number started.
        position: usize,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // TYPE ERRORS (E1xx)
    // ═══════════════════════════════════════════════════════════════════════
    /// Type mismatch in operation.
    ///
    /// An operation was attempted with incompatible types.
    #[error("Type mismatch: expected {expected}, got {actual} for field '{field}'")]
    TypeMismatch {
        /// Name of the field being operated on.
        field: String,
        /// Expected type name.
        expected: String,
        /// Actual type found.
        actual: String,
    },

    /// Incompatible types for comparison.
    ///
    /// Two values of incompatible types were compared.
    #[error("Cannot compare {left_type} with {right_type}")]
    IncompatibleTypes {
        /// Type of left operand.
        left_type: String,
        /// Type of right operand.
        right_type: String,
    },

    /// Invalid operator for type.
    ///
    /// An operator was used with a type that doesn't support it.
    #[error("Operator '{operator}' is not valid for type '{value_type}'")]
    InvalidOperatorForType {
        /// The operator that was used.
        operator: String,
        /// The type it was used with.
        value_type: String,
    },

    /// Unknown field reference.
    ///
    /// A field was referenced that doesn't exist in the metadata schema.
    #[error("Unknown field '{field}'")]
    UnknownField {
        /// Name of the unknown field.
        field: String,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // EVALUATION ERRORS (E2xx)
    // ═══════════════════════════════════════════════════════════════════════
    /// Division by zero during evaluation.
    #[error("Division by zero")]
    DivisionByZero,

    /// Null value in non-nullable context.
    #[error("Null value encountered for field '{field}' in non-nullable context")]
    NullValue {
        /// Name of the field with null value.
        field: String,
    },

    /// Array index out of bounds.
    #[error("Array index {index} out of bounds (length: {length})")]
    IndexOutOfBounds {
        /// The invalid index.
        index: usize,
        /// Length of the array.
        length: usize,
    },

    /// Invalid expression for evaluation.
    ///
    /// A literal was used where a boolean expression was expected.
    #[error("Invalid expression: {message}")]
    InvalidExpression {
        /// Description of what's wrong with the expression.
        message: String,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // LIMIT ERRORS (E3xx)
    // ═══════════════════════════════════════════════════════════════════════
    /// Expression nesting too deep.
    ///
    /// The filter expression exceeds the maximum allowed nesting depth.
    /// This limit prevents stack overflow during evaluation.
    #[error("Nesting too deep (max {max_depth} levels, found {actual_depth})")]
    NestingTooDeep {
        /// Maximum allowed depth.
        max_depth: usize,
        /// Actual depth found.
        actual_depth: usize,
    },

    /// Expression too complex.
    ///
    /// The filter expression has too many nodes/operations.
    #[error("Expression too complex (max {max_nodes} nodes, found {actual_nodes})")]
    ExpressionTooComplex {
        /// Maximum allowed nodes.
        max_nodes: usize,
        /// Actual nodes found.
        actual_nodes: usize,
    },

    /// Input string too long.
    ///
    /// The filter expression string exceeds the maximum allowed length.
    #[error("Filter expression too long (max {max_length} bytes, found {actual_length})")]
    InputTooLong {
        /// Maximum allowed length in bytes.
        max_length: usize,
        /// Actual length in bytes.
        actual_length: usize,
    },

    /// Array literal too large.
    ///
    /// An array literal in the filter has too many elements.
    #[error("Array literal too large (max {max_elements} elements, found {actual_elements})")]
    ArrayTooLarge {
        /// Maximum allowed elements.
        max_elements: usize,
        /// Actual elements found.
        actual_elements: usize,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // STRATEGY ERRORS (E4xx)
    // ═══════════════════════════════════════════════════════════════════════
    /// Invalid filter strategy configuration.
    ///
    /// The filter strategy parameters are invalid (e.g., oversample < 1.0).
    #[error("Invalid filter strategy: {0}")]
    InvalidStrategy(String),
}

impl FilterError {
    /// Get the error code for WASM serialization.
    ///
    /// Error codes follow a structured pattern:
    /// - `E001-E099`: Syntax errors
    /// - `E101-E199`: Type errors
    /// - `E201-E299`: Evaluation errors
    /// - `E301-E399`: Limit errors
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgevec::filter::FilterError;
    ///
    /// let error = FilterError::SyntaxError {
    ///     position: 0,
    ///     line: 1,
    ///     column: 1,
    ///     message: "test".to_string(),
    ///     suggestion: None,
    /// };
    /// assert_eq!(error.code(), "E001");
    /// ```
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            // Syntax errors (E0xx)
            FilterError::SyntaxError { .. } => "E001",
            FilterError::UnexpectedEof { .. } => "E002",
            FilterError::InvalidChar { .. } => "E003",
            FilterError::UnclosedString { .. } => "E004",
            FilterError::UnclosedParen { .. } => "E005",
            FilterError::InvalidEscape { .. } => "E006",
            FilterError::InvalidNumber { .. } => "E007",
            // Type errors (E1xx)
            FilterError::TypeMismatch { .. } => "E101",
            FilterError::IncompatibleTypes { .. } => "E102",
            FilterError::InvalidOperatorForType { .. } => "E103",
            FilterError::UnknownField { .. } => "E105",
            // Evaluation errors (E2xx)
            FilterError::DivisionByZero => "E201",
            FilterError::NullValue { .. } => "E202",
            FilterError::IndexOutOfBounds { .. } => "E203",
            FilterError::InvalidExpression { .. } => "E204",
            // Limit errors (E3xx)
            FilterError::NestingTooDeep { .. } => "E301",
            FilterError::ExpressionTooComplex { .. } => "E302",
            FilterError::InputTooLong { .. } => "E303",
            FilterError::ArrayTooLarge { .. } => "E304",
            // Strategy errors (E4xx)
            FilterError::InvalidStrategy(_) => "E401",
        }
    }

    /// Check if this is a syntax error.
    #[must_use]
    pub fn is_syntax_error(&self) -> bool {
        matches!(
            self,
            FilterError::SyntaxError { .. }
                | FilterError::UnexpectedEof { .. }
                | FilterError::InvalidChar { .. }
                | FilterError::UnclosedString { .. }
                | FilterError::UnclosedParen { .. }
                | FilterError::InvalidEscape { .. }
                | FilterError::InvalidNumber { .. }
        )
    }

    /// Check if this is a type error.
    #[must_use]
    pub fn is_type_error(&self) -> bool {
        matches!(
            self,
            FilterError::TypeMismatch { .. }
                | FilterError::IncompatibleTypes { .. }
                | FilterError::InvalidOperatorForType { .. }
                | FilterError::UnknownField { .. }
        )
    }

    /// Check if this is an evaluation error.
    #[must_use]
    pub fn is_evaluation_error(&self) -> bool {
        matches!(
            self,
            FilterError::DivisionByZero
                | FilterError::NullValue { .. }
                | FilterError::IndexOutOfBounds { .. }
                | FilterError::InvalidExpression { .. }
        )
    }

    /// Check if this is a limit error.
    #[must_use]
    pub fn is_limit_error(&self) -> bool {
        matches!(
            self,
            FilterError::NestingTooDeep { .. }
                | FilterError::ExpressionTooComplex { .. }
                | FilterError::InputTooLong { .. }
                | FilterError::ArrayTooLarge { .. }
        )
    }

    /// Get the position information if available.
    ///
    /// Returns `(position, line, column)` for errors that have position info.
    #[must_use]
    pub fn position(&self) -> Option<(usize, usize, usize)> {
        match self {
            FilterError::SyntaxError {
                position,
                line,
                column,
                ..
            } => Some((*position, *line, *column)),
            FilterError::UnexpectedEof { position, .. }
            | FilterError::InvalidChar { position, .. }
            | FilterError::UnclosedString { position }
            | FilterError::UnclosedParen { position }
            | FilterError::InvalidEscape { position, .. }
            | FilterError::InvalidNumber { position, .. } => Some((*position, 1, *position + 1)),
            _ => Option::None,
        }
    }

    /// Generate a helpful suggestion based on error type.
    ///
    /// Returns a suggestion string that can help the user fix the error.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match self {
            FilterError::SyntaxError { suggestion, .. } => suggestion.clone(),
            FilterError::UnclosedString { .. } => {
                Some("Did you forget the closing quote?".to_string())
            }
            FilterError::UnclosedParen { .. } => {
                Some("Did you forget the closing parenthesis?".to_string())
            }
            FilterError::InvalidEscape { char, .. } => Some(format!(
                "Valid escape sequences are: \\\" \\\\ \\n \\r \\t. '\\{char}' is not valid."
            )),
            FilterError::TypeMismatch {
                field,
                expected,
                actual,
            } => Some(format!(
                "Field '{field}' is of type '{actual}', but '{expected}' was expected. \
                 Check that you're using the right operator for this field type."
            )),
            FilterError::UnknownField { field } => Some(format!(
                "Field '{field}' does not exist. Check the field name for typos."
            )),
            FilterError::NestingTooDeep { max_depth, .. } => Some(format!(
                "Simplify your filter expression. Maximum nesting depth is {max_depth}."
            )),
            FilterError::ArrayTooLarge { max_elements, .. } => Some(format!(
                "Use smaller arrays in IN/ANY/ALL operators. Maximum is {max_elements} elements."
            )),
            _ => Option::None,
        }
    }
}

/// Maximum nesting depth for filter expressions.
///
/// Prevents stack overflow during recursive evaluation.
pub const MAX_NESTING_DEPTH: usize = 50;

/// Maximum number of nodes in a filter expression.
///
/// Prevents denial-of-service via overly complex expressions.
pub const MAX_EXPRESSION_NODES: usize = 1000;

/// Maximum length of a filter expression string in bytes.
///
/// Prevents denial-of-service via extremely long inputs.
pub const MAX_INPUT_LENGTH: usize = 65536;

/// Maximum number of elements in an array literal.
///
/// Limits memory usage for IN/ANY/ALL operations.
pub const MAX_ARRAY_ELEMENTS: usize = 1000;

#[cfg(test)]
#[allow(clippy::unreadable_literal)] // Large test literals like 100000 are fine
mod tests {
    use super::*;

    // =========================================================================
    // ERROR CODE TESTS
    // =========================================================================

    #[test]
    fn test_syntax_error_codes() {
        assert_eq!(
            FilterError::SyntaxError {
                position: 0,
                line: 1,
                column: 1,
                message: "test".to_string(),
                suggestion: None
            }
            .code(),
            "E001"
        );
        assert_eq!(
            FilterError::UnexpectedEof {
                position: 0,
                expected: "value".to_string()
            }
            .code(),
            "E002"
        );
        assert_eq!(
            FilterError::InvalidChar {
                char: '@',
                position: 0
            }
            .code(),
            "E003"
        );
        assert_eq!(FilterError::UnclosedString { position: 0 }.code(), "E004");
        assert_eq!(FilterError::UnclosedParen { position: 0 }.code(), "E005");
        assert_eq!(
            FilterError::InvalidEscape {
                char: 'x',
                position: 0
            }
            .code(),
            "E006"
        );
        assert_eq!(
            FilterError::InvalidNumber {
                value: "1.2.3".to_string(),
                position: 0
            }
            .code(),
            "E007"
        );
    }

    #[test]
    fn test_type_error_codes() {
        assert_eq!(
            FilterError::TypeMismatch {
                field: "f".to_string(),
                expected: "int".to_string(),
                actual: "string".to_string()
            }
            .code(),
            "E101"
        );
        assert_eq!(
            FilterError::IncompatibleTypes {
                left_type: "int".to_string(),
                right_type: "string".to_string()
            }
            .code(),
            "E102"
        );
        assert_eq!(
            FilterError::InvalidOperatorForType {
                operator: "<".to_string(),
                value_type: "boolean".to_string()
            }
            .code(),
            "E103"
        );
        assert_eq!(
            FilterError::UnknownField {
                field: "x".to_string()
            }
            .code(),
            "E105"
        );
    }

    #[test]
    fn test_evaluation_error_codes() {
        assert_eq!(FilterError::DivisionByZero.code(), "E201");
        assert_eq!(
            FilterError::NullValue {
                field: "x".to_string()
            }
            .code(),
            "E202"
        );
        assert_eq!(
            FilterError::IndexOutOfBounds {
                index: 5,
                length: 3
            }
            .code(),
            "E203"
        );
    }

    #[test]
    fn test_limit_error_codes() {
        assert_eq!(
            FilterError::NestingTooDeep {
                max_depth: 50,
                actual_depth: 100
            }
            .code(),
            "E301"
        );
        assert_eq!(
            FilterError::ExpressionTooComplex {
                max_nodes: 1000,
                actual_nodes: 2000
            }
            .code(),
            "E302"
        );
        assert_eq!(
            FilterError::InputTooLong {
                max_length: 65536,
                actual_length: 100000
            }
            .code(),
            "E303"
        );
        assert_eq!(
            FilterError::ArrayTooLarge {
                max_elements: 1000,
                actual_elements: 2000
            }
            .code(),
            "E304"
        );
    }

    // =========================================================================
    // ERROR CATEGORY TESTS
    // =========================================================================

    #[test]
    fn test_is_syntax_error() {
        assert!(FilterError::SyntaxError {
            position: 0,
            line: 1,
            column: 1,
            message: "test".to_string(),
            suggestion: None
        }
        .is_syntax_error());
        assert!(FilterError::UnclosedString { position: 0 }.is_syntax_error());
        assert!(!FilterError::DivisionByZero.is_syntax_error());
    }

    #[test]
    fn test_is_type_error() {
        assert!(FilterError::TypeMismatch {
            field: "f".to_string(),
            expected: "int".to_string(),
            actual: "string".to_string()
        }
        .is_type_error());
        assert!(FilterError::UnknownField {
            field: "x".to_string()
        }
        .is_type_error());
        assert!(!FilterError::DivisionByZero.is_type_error());
    }

    #[test]
    fn test_is_evaluation_error() {
        assert!(FilterError::DivisionByZero.is_evaluation_error());
        assert!(FilterError::NullValue {
            field: "x".to_string()
        }
        .is_evaluation_error());
        assert!(!FilterError::UnclosedString { position: 0 }.is_evaluation_error());
    }

    #[test]
    fn test_is_limit_error() {
        assert!(FilterError::NestingTooDeep {
            max_depth: 50,
            actual_depth: 100
        }
        .is_limit_error());
        assert!(FilterError::ArrayTooLarge {
            max_elements: 1000,
            actual_elements: 2000
        }
        .is_limit_error());
        assert!(!FilterError::DivisionByZero.is_limit_error());
    }

    // =========================================================================
    // POSITION TESTS
    // =========================================================================

    #[test]
    fn test_position_syntax_error() {
        let error = FilterError::SyntaxError {
            position: 10,
            line: 2,
            column: 5,
            message: "test".to_string(),
            suggestion: None,
        };
        assert_eq!(error.position(), Some((10, 2, 5)));
    }

    #[test]
    fn test_position_other_syntax_errors() {
        assert_eq!(
            FilterError::UnclosedString { position: 5 }.position(),
            Some((5, 1, 6))
        );
        assert_eq!(
            FilterError::InvalidChar {
                char: '@',
                position: 3
            }
            .position(),
            Some((3, 1, 4))
        );
    }

    #[test]
    fn test_position_non_positional() {
        assert_eq!(FilterError::DivisionByZero.position(), Option::None);
        assert_eq!(
            FilterError::TypeMismatch {
                field: "f".to_string(),
                expected: "int".to_string(),
                actual: "string".to_string()
            }
            .position(),
            Option::None
        );
    }

    // =========================================================================
    // SUGGESTION TESTS
    // =========================================================================

    #[test]
    fn test_suggestion_unclosed_string() {
        let error = FilterError::UnclosedString { position: 0 };
        assert!(error.suggestion().is_some());
        assert!(error.suggestion().unwrap().contains("closing quote"));
    }

    #[test]
    fn test_suggestion_unclosed_paren() {
        let error = FilterError::UnclosedParen { position: 0 };
        assert!(error.suggestion().is_some());
        assert!(error.suggestion().unwrap().contains("closing parenthesis"));
    }

    #[test]
    fn test_suggestion_invalid_escape() {
        let error = FilterError::InvalidEscape {
            char: 'x',
            position: 0,
        };
        let suggestion = error.suggestion().unwrap();
        assert!(suggestion.contains("\\\""));
        assert!(suggestion.contains("\\\\"));
    }

    #[test]
    fn test_suggestion_type_mismatch() {
        let error = FilterError::TypeMismatch {
            field: "price".to_string(),
            expected: "integer".to_string(),
            actual: "string".to_string(),
        };
        let suggestion = error.suggestion().unwrap();
        assert!(suggestion.contains("price"));
        assert!(suggestion.contains("string"));
        assert!(suggestion.contains("integer"));
    }

    #[test]
    fn test_suggestion_unknown_field() {
        let error = FilterError::UnknownField {
            field: "categry".to_string(),
        };
        let suggestion = error.suggestion().unwrap();
        assert!(suggestion.contains("categry"));
        assert!(suggestion.contains("typos"));
    }

    // =========================================================================
    // DISPLAY TESTS
    // =========================================================================

    #[test]
    fn test_display_syntax_error() {
        let error = FilterError::SyntaxError {
            position: 10,
            line: 1,
            column: 11,
            message: "Expected operator".to_string(),
            suggestion: None,
        };
        let display = format!("{error}");
        assert!(display.contains("position 10"));
        assert!(display.contains("line 1"));
        assert!(display.contains("column 11"));
        assert!(display.contains("Expected operator"));
    }

    #[test]
    fn test_display_type_mismatch() {
        let error = FilterError::TypeMismatch {
            field: "price".to_string(),
            expected: "integer".to_string(),
            actual: "string".to_string(),
        };
        let display = format!("{error}");
        assert!(display.contains("Type mismatch"));
        assert!(display.contains("price"));
        assert!(display.contains("integer"));
        assert!(display.contains("string"));
    }

    // =========================================================================
    // CONSTANT TESTS
    // =========================================================================

    #[test]
    fn test_constants() {
        assert_eq!(MAX_NESTING_DEPTH, 50);
        assert_eq!(MAX_EXPRESSION_NODES, 1000);
        assert_eq!(MAX_INPUT_LENGTH, 65536);
        assert_eq!(MAX_ARRAY_ELEMENTS, 1000);
    }

    // =========================================================================
    // CLONE AND PARTIAL_EQ TESTS
    // =========================================================================

    #[test]
    fn test_clone() {
        let error = FilterError::SyntaxError {
            position: 10,
            line: 1,
            column: 11,
            message: "test".to_string(),
            suggestion: Some("hint".to_string()),
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_partial_eq() {
        let error1 = FilterError::InvalidChar {
            char: '@',
            position: 5,
        };
        let error2 = FilterError::InvalidChar {
            char: '@',
            position: 5,
        };
        let error3 = FilterError::InvalidChar {
            char: '#',
            position: 5,
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }
}
