//! Filter expression AST for EdgeVec.
//!
//! This module defines the `FilterExpr` enum with all 27 AST node variants
//! as specified in FILTER_EVALUATOR.md.
//!
//! # Variant Categories
//!
//! | Category | Count | Variants |
//! |:---------|:------|:---------|
//! | Literals | 5 | String, Int, Float, Bool, Array |
//! | Field Reference | 1 | Field |
//! | Comparison | 6 | Eq, Ne, Lt, Le, Gt, Ge |
//! | String Operators | 4 | Contains, StartsWith, EndsWith, Like |
//! | Array/Set Operators | 5 | In, NotIn, Any, All, None |
//! | Range | 1 | Between |
//! | Logical | 3 | And, Or, Not |
//! | Null Checks | 2 | IsNull, IsNotNull |
//!
//! # Memory Layout
//!
//! All recursive variants use `Box<FilterExpr>` to ensure:
//! - Fixed-size enum (single pointer width per child)
//! - Heap allocation for nested expressions
//! - Prevents stack overflow with deep nesting
//!
//! # Serialization
//!
//! The AST is serializable with serde for:
//! - WASM boundary crossing (JSON serialization)
//! - Debug logging and visualization
//! - Potential caching of parsed expressions

use serde::{Deserialize, Serialize};

/// Filter expression AST node.
///
/// Represents a parsed filter expression that can be evaluated
/// against vector metadata. The enum has exactly 27 variants
/// covering all supported filter operations.
///
/// # Recursive Structure
///
/// Binary operators store left and right operands as `Box<FilterExpr>`,
/// enabling arbitrarily nested expressions like:
/// ```text
/// (category = "gpu" AND price < 500) OR rating >= 4.5
/// ```
///
/// # Example
///
/// ```rust
/// use edgevec::filter::FilterExpr;
///
/// // Simple equality check
/// let expr = FilterExpr::Eq(
///     Box::new(FilterExpr::Field("category".to_string())),
///     Box::new(FilterExpr::LiteralString("gpu".to_string())),
/// );
///
/// // Compound expression
/// let compound = FilterExpr::And(
///     Box::new(expr),
///     Box::new(FilterExpr::Lt(
///         Box::new(FilterExpr::Field("price".to_string())),
///         Box::new(FilterExpr::LiteralInt(500)),
///     )),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FilterExpr {
    // ═══════════════════════════════════════════════════════════════════════
    // LITERALS (5 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// String literal: `"hello"`, `"gpu"`, `"category name"`
    ///
    /// Supports escape sequences: `\"`, `\\`, `\n`, `\r`, `\t`
    LiteralString(String),

    /// Integer literal: `42`, `-1`, `0`, `9999`
    ///
    /// Stored as i64 for maximum range compatibility.
    LiteralInt(i64),

    /// Float literal: `3.14159`, `-0.5`, `100.0`
    ///
    /// Stored as f64 (IEEE 754 double precision).
    /// Note: NaN and Infinity are not valid in filter expressions.
    LiteralFloat(f64),

    /// Boolean literal: `true`, `false`
    ///
    /// Case-insensitive parsing: `TRUE`, `True`, `true` all valid.
    LiteralBool(bool),

    /// Array literal: `[1, 2, 3]`, `["a", "b"]`, `[true, false]`
    ///
    /// Used with `IN`, `NOT IN`, `ANY`, `ALL`, `NONE` operators.
    /// Elements must be homogeneous in type.
    LiteralArray(Vec<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // FIELD REFERENCE (1 variant)
    // ═══════════════════════════════════════════════════════════════════════
    /// Field reference: `category`, `price`, `tags`, `created_at`
    ///
    /// References a metadata field by name. Field names must:
    /// - Start with a letter or underscore
    /// - Contain only alphanumeric characters and underscores
    /// - Be case-sensitive (recommended: snake_case)
    Field(String),

    // ═══════════════════════════════════════════════════════════════════════
    // COMPARISON OPERATORS (6 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// Equality: `field = value`
    ///
    /// Supports all comparable types: string, int, float, bool.
    /// String comparison is case-sensitive.
    Eq(Box<FilterExpr>, Box<FilterExpr>),

    /// Inequality: `field != value`
    ///
    /// Logically equivalent to `NOT (field = value)`.
    Ne(Box<FilterExpr>, Box<FilterExpr>),

    /// Less than: `field < value`
    ///
    /// Supports numeric types (int, float) and strings (lexicographic).
    Lt(Box<FilterExpr>, Box<FilterExpr>),

    /// Less than or equal: `field <= value`
    ///
    /// Supports numeric types (int, float) and strings (lexicographic).
    Le(Box<FilterExpr>, Box<FilterExpr>),

    /// Greater than: `field > value`
    ///
    /// Supports numeric types (int, float) and strings (lexicographic).
    Gt(Box<FilterExpr>, Box<FilterExpr>),

    /// Greater than or equal: `field >= value`
    ///
    /// Supports numeric types (int, float) and strings (lexicographic).
    Ge(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // STRING OPERATORS (4 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// Contains substring: `field CONTAINS "value"`
    ///
    /// Case-sensitive substring match.
    /// Example: `description CONTAINS "fast"` matches "ultra-fast processor"
    Contains(Box<FilterExpr>, Box<FilterExpr>),

    /// Starts with prefix: `field STARTS_WITH "prefix"`
    ///
    /// Case-sensitive prefix match.
    /// Example: `name STARTS_WITH "GPU"` matches "GPU-1080"
    StartsWith(Box<FilterExpr>, Box<FilterExpr>),

    /// Ends with suffix: `field ENDS_WITH "suffix"`
    ///
    /// Case-sensitive suffix match.
    /// Example: `filename ENDS_WITH ".pdf"` matches "document.pdf"
    EndsWith(Box<FilterExpr>, Box<FilterExpr>),

    /// SQL-style LIKE pattern: `field LIKE "pattern%"`
    ///
    /// Supports wildcards:
    /// - `%` matches zero or more characters
    /// - `_` matches exactly one character
    ///
    /// Example: `name LIKE "GPU_%"` matches "GPU_1080", "GPU_3090"
    Like(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // ARRAY/SET OPERATORS (5 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// In set: `field IN [v1, v2, v3]`
    ///
    /// True if field value equals any element in the array.
    /// Example: `category IN ["gpu", "cpu", "ram"]`
    In(Box<FilterExpr>, Box<FilterExpr>),

    /// Not in set: `field NOT IN [v1, v2, v3]`
    ///
    /// True if field value does not equal any element in the array.
    /// Logically equivalent to `NOT (field IN [v1, v2, v3])`.
    NotIn(Box<FilterExpr>, Box<FilterExpr>),

    /// Any match: `field ANY [v1, v2]`
    ///
    /// For array-valued fields: true if field contains any of the specified values.
    /// Example: `tags ANY ["rust", "wasm"]` matches if tags contains "rust" OR "wasm"
    Any(Box<FilterExpr>, Box<FilterExpr>),

    /// All match: `field ALL [v1, v2]`
    ///
    /// For array-valued fields: true if field contains all of the specified values.
    /// Example: `tags ALL ["rust", "wasm"]` matches if tags contains "rust" AND "wasm"
    All(Box<FilterExpr>, Box<FilterExpr>),

    /// None match: `field NONE [v1, v2]`
    ///
    /// For array-valued fields: true if field contains none of the specified values.
    /// Example: `tags NONE ["deprecated"]` matches if tags does not contain "deprecated"
    None(Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // RANGE OPERATOR (1 variant)
    // ═══════════════════════════════════════════════════════════════════════
    /// Between range: `field BETWEEN low AND high` (inclusive)
    ///
    /// Logically equivalent to `field >= low AND field <= high`.
    /// Supports numeric types and strings.
    ///
    /// Example: `price BETWEEN 100 AND 500`
    Between(Box<FilterExpr>, Box<FilterExpr>, Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // LOGICAL OPERATORS (3 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// Logical AND: `expr1 AND expr2`
    ///
    /// True if both operands are true. Short-circuit evaluation.
    /// Alternative syntax: `&&`
    And(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical OR: `expr1 OR expr2`
    ///
    /// True if either operand is true. Short-circuit evaluation.
    /// Alternative syntax: `||`
    Or(Box<FilterExpr>, Box<FilterExpr>),

    /// Logical NOT: `NOT expr`
    ///
    /// Inverts the boolean result of the operand.
    /// Alternative syntax: `!`
    Not(Box<FilterExpr>),

    // ═══════════════════════════════════════════════════════════════════════
    // NULL OPERATORS (2 variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// Is null check: `field IS NULL`
    ///
    /// True if the field does not exist or has a null value.
    IsNull(Box<FilterExpr>),

    /// Is not null check: `field IS NOT NULL`
    ///
    /// True if the field exists and has a non-null value.
    /// Logically equivalent to `NOT (field IS NULL)`.
    IsNotNull(Box<FilterExpr>),
}

impl FilterExpr {
    // ═══════════════════════════════════════════════════════════════════════
    // TYPE CHECKING METHODS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns true if this is a literal value.
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            FilterExpr::LiteralString(_)
                | FilterExpr::LiteralInt(_)
                | FilterExpr::LiteralFloat(_)
                | FilterExpr::LiteralBool(_)
                | FilterExpr::LiteralArray(_)
        )
    }

    /// Returns true if this is a field reference.
    #[must_use]
    pub fn is_field(&self) -> bool {
        matches!(self, FilterExpr::Field(_))
    }

    /// Returns true if this is a comparison operator.
    #[must_use]
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            FilterExpr::Eq(_, _)
                | FilterExpr::Ne(_, _)
                | FilterExpr::Lt(_, _)
                | FilterExpr::Le(_, _)
                | FilterExpr::Gt(_, _)
                | FilterExpr::Ge(_, _)
        )
    }

    /// Returns true if this is a string operator.
    #[must_use]
    pub fn is_string_op(&self) -> bool {
        matches!(
            self,
            FilterExpr::Contains(_, _)
                | FilterExpr::StartsWith(_, _)
                | FilterExpr::EndsWith(_, _)
                | FilterExpr::Like(_, _)
        )
    }

    /// Returns true if this is an array/set operator.
    #[must_use]
    pub fn is_array_op(&self) -> bool {
        matches!(
            self,
            FilterExpr::In(_, _)
                | FilterExpr::NotIn(_, _)
                | FilterExpr::Any(_, _)
                | FilterExpr::All(_, _)
                | FilterExpr::None(_, _)
        )
    }

    /// Returns true if this is a logical operator.
    #[must_use]
    pub fn is_logical(&self) -> bool {
        matches!(
            self,
            FilterExpr::And(_, _) | FilterExpr::Or(_, _) | FilterExpr::Not(_)
        )
    }

    /// Returns true if this is a null check operator.
    #[must_use]
    pub fn is_null_check(&self) -> bool {
        matches!(self, FilterExpr::IsNull(_) | FilterExpr::IsNotNull(_))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VALUE EXTRACTION METHODS
    // ═══════════════════════════════════════════════════════════════════════

    /// Extracts string literal value if this is a LiteralString.
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FilterExpr::LiteralString(s) => Some(s),
            _ => Option::None,
        }
    }

    /// Extracts integer literal value if this is a LiteralInt.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            FilterExpr::LiteralInt(i) => Some(*i),
            _ => Option::None,
        }
    }

    /// Extracts float literal value if this is a LiteralFloat.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            FilterExpr::LiteralFloat(f) => Some(*f),
            _ => Option::None,
        }
    }

    /// Extracts boolean literal value if this is a LiteralBool.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FilterExpr::LiteralBool(b) => Some(*b),
            _ => Option::None,
        }
    }

    /// Extracts field name if this is a Field reference.
    #[must_use]
    pub fn as_field(&self) -> Option<&str> {
        match self {
            FilterExpr::Field(name) => Some(name),
            _ => Option::None,
        }
    }

    /// Extracts array elements if this is a LiteralArray.
    #[must_use]
    pub fn as_array(&self) -> Option<&[FilterExpr]> {
        match self {
            FilterExpr::LiteralArray(arr) => Some(arr),
            _ => Option::None,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // UTILITY METHODS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns the operator name for display/debugging.
    #[must_use]
    pub fn operator_name(&self) -> &'static str {
        match self {
            FilterExpr::LiteralString(_) => "LiteralString",
            FilterExpr::LiteralInt(_) => "LiteralInt",
            FilterExpr::LiteralFloat(_) => "LiteralFloat",
            FilterExpr::LiteralBool(_) => "LiteralBool",
            FilterExpr::LiteralArray(_) => "LiteralArray",
            FilterExpr::Field(_) => "Field",
            FilterExpr::Eq(_, _) => "Eq",
            FilterExpr::Ne(_, _) => "Ne",
            FilterExpr::Lt(_, _) => "Lt",
            FilterExpr::Le(_, _) => "Le",
            FilterExpr::Gt(_, _) => "Gt",
            FilterExpr::Ge(_, _) => "Ge",
            FilterExpr::Contains(_, _) => "Contains",
            FilterExpr::StartsWith(_, _) => "StartsWith",
            FilterExpr::EndsWith(_, _) => "EndsWith",
            FilterExpr::Like(_, _) => "Like",
            FilterExpr::In(_, _) => "In",
            FilterExpr::NotIn(_, _) => "NotIn",
            FilterExpr::Any(_, _) => "Any",
            FilterExpr::All(_, _) => "All",
            FilterExpr::None(_, _) => "None",
            FilterExpr::Between(_, _, _) => "Between",
            FilterExpr::And(_, _) => "And",
            FilterExpr::Or(_, _) => "Or",
            FilterExpr::Not(_) => "Not",
            FilterExpr::IsNull(_) => "IsNull",
            FilterExpr::IsNotNull(_) => "IsNotNull",
        }
    }

    /// Counts the depth of nesting in this expression tree.
    ///
    /// Used for enforcing maximum nesting limits to prevent stack overflow.
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            // Leaf nodes have depth 1
            FilterExpr::LiteralString(_)
            | FilterExpr::LiteralInt(_)
            | FilterExpr::LiteralFloat(_)
            | FilterExpr::LiteralBool(_)
            | FilterExpr::Field(_) => 1,

            // Array depth is max of element depths + 1
            FilterExpr::LiteralArray(arr) => {
                1 + arr.iter().map(FilterExpr::depth).max().unwrap_or(0)
            }

            // Unary operators: child depth + 1
            FilterExpr::Not(expr) | FilterExpr::IsNull(expr) | FilterExpr::IsNotNull(expr) => {
                1 + expr.depth()
            }

            // Binary operators: max of children + 1
            FilterExpr::Eq(l, r)
            | FilterExpr::Ne(l, r)
            | FilterExpr::Lt(l, r)
            | FilterExpr::Le(l, r)
            | FilterExpr::Gt(l, r)
            | FilterExpr::Ge(l, r)
            | FilterExpr::Contains(l, r)
            | FilterExpr::StartsWith(l, r)
            | FilterExpr::EndsWith(l, r)
            | FilterExpr::Like(l, r)
            | FilterExpr::In(l, r)
            | FilterExpr::NotIn(l, r)
            | FilterExpr::Any(l, r)
            | FilterExpr::All(l, r)
            | FilterExpr::None(l, r)
            | FilterExpr::And(l, r)
            | FilterExpr::Or(l, r) => 1 + l.depth().max(r.depth()),

            // Ternary operator (Between)
            FilterExpr::Between(field, low, high) => {
                1 + field.depth().max(low.depth()).max(high.depth())
            }
        }
    }

    /// Collects all field names referenced in this expression.
    ///
    /// Useful for determining which metadata fields are needed for evaluation.
    #[must_use]
    pub fn referenced_fields(&self) -> Vec<&str> {
        let mut fields = Vec::new();
        self.collect_fields(&mut fields);
        fields
    }

    fn collect_fields<'a>(&'a self, fields: &mut Vec<&'a str>) {
        match self {
            FilterExpr::Field(name) => fields.push(name),
            FilterExpr::LiteralArray(arr) => {
                for elem in arr {
                    elem.collect_fields(fields);
                }
            }
            FilterExpr::Not(expr) | FilterExpr::IsNull(expr) | FilterExpr::IsNotNull(expr) => {
                expr.collect_fields(fields);
            }
            FilterExpr::Eq(l, r)
            | FilterExpr::Ne(l, r)
            | FilterExpr::Lt(l, r)
            | FilterExpr::Le(l, r)
            | FilterExpr::Gt(l, r)
            | FilterExpr::Ge(l, r)
            | FilterExpr::Contains(l, r)
            | FilterExpr::StartsWith(l, r)
            | FilterExpr::EndsWith(l, r)
            | FilterExpr::Like(l, r)
            | FilterExpr::In(l, r)
            | FilterExpr::NotIn(l, r)
            | FilterExpr::Any(l, r)
            | FilterExpr::All(l, r)
            | FilterExpr::None(l, r)
            | FilterExpr::And(l, r)
            | FilterExpr::Or(l, r) => {
                l.collect_fields(fields);
                r.collect_fields(fields);
            }
            FilterExpr::Between(field, low, high) => {
                field.collect_fields(fields);
                low.collect_fields(fields);
                high.collect_fields(fields);
            }
            // Literals don't have fields
            FilterExpr::LiteralString(_)
            | FilterExpr::LiteralInt(_)
            | FilterExpr::LiteralFloat(_)
            | FilterExpr::LiteralBool(_) => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)] // Test values like 3.14 are intentionally approximate
#[allow(clippy::manual_string_new)] // Tests use "".to_string() for clarity
mod tests {
    use super::*;

    // =========================================================================
    // VARIANT COUNT TEST (Required by W23.1.1 Acceptance Criteria)
    // =========================================================================

    #[test]
    fn test_filter_expr_has_27_variants() {
        // Create one instance of each variant to verify they all exist
        let variants: Vec<FilterExpr> = vec![
            // Literals (5)
            FilterExpr::LiteralString("test".to_string()),
            FilterExpr::LiteralInt(42),
            FilterExpr::LiteralFloat(3.14),
            FilterExpr::LiteralBool(true),
            FilterExpr::LiteralArray(vec![]),
            // Field (1)
            FilterExpr::Field("name".to_string()),
            // Comparison (6)
            FilterExpr::Eq(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            FilterExpr::Ne(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            FilterExpr::Lt(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            FilterExpr::Le(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            FilterExpr::Gt(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            FilterExpr::Ge(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(1)),
            ),
            // String operators (4)
            FilterExpr::Contains(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralString("a".to_string())),
            ),
            FilterExpr::StartsWith(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralString("a".to_string())),
            ),
            FilterExpr::EndsWith(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralString("a".to_string())),
            ),
            FilterExpr::Like(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralString("a%".to_string())),
            ),
            // Array/Set operators (5)
            FilterExpr::In(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralArray(vec![])),
            ),
            FilterExpr::NotIn(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralArray(vec![])),
            ),
            FilterExpr::Any(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralArray(vec![])),
            ),
            FilterExpr::All(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralArray(vec![])),
            ),
            FilterExpr::None(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralArray(vec![])),
            ),
            // Range (1)
            FilterExpr::Between(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(0)),
                Box::new(FilterExpr::LiteralInt(100)),
            ),
            // Logical (3)
            FilterExpr::And(
                Box::new(FilterExpr::LiteralBool(true)),
                Box::new(FilterExpr::LiteralBool(false)),
            ),
            FilterExpr::Or(
                Box::new(FilterExpr::LiteralBool(true)),
                Box::new(FilterExpr::LiteralBool(false)),
            ),
            FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true))),
            // Null checks (2)
            FilterExpr::IsNull(Box::new(FilterExpr::Field("x".to_string()))),
            FilterExpr::IsNotNull(Box::new(FilterExpr::Field("x".to_string()))),
        ];

        // Verify we have exactly 27 variants
        assert_eq!(
            variants.len(),
            27,
            "FilterExpr should have exactly 27 variants"
        );
    }

    // =========================================================================
    // DERIVE TRAIT TESTS (Required by W23.1.1 Acceptance Criteria)
    // =========================================================================

    #[test]
    fn test_derive_debug() {
        let expr = FilterExpr::LiteralString("test".to_string());
        let debug_str = format!("{expr:?}");
        assert!(debug_str.contains("LiteralString"));
    }

    #[test]
    fn test_derive_clone() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Field("category".to_string())),
            Box::new(FilterExpr::LiteralBool(true)),
        );
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_derive_partial_eq() {
        let expr1 = FilterExpr::LiteralInt(42);
        let expr2 = FilterExpr::LiteralInt(42);
        let expr3 = FilterExpr::LiteralInt(43);

        assert_eq!(expr1, expr2);
        assert_ne!(expr1, expr3);
    }

    #[test]
    fn test_derive_serialize_deserialize() {
        let expr = FilterExpr::Eq(
            Box::new(FilterExpr::Field("category".to_string())),
            Box::new(FilterExpr::LiteralString("gpu".to_string())),
        );

        let json = serde_json::to_string(&expr).unwrap();
        let parsed: FilterExpr = serde_json::from_str(&json).unwrap();

        assert_eq!(expr, parsed);
    }

    // =========================================================================
    // BOX USAGE TESTS (Required by W23.1.1 Acceptance Criteria)
    // =========================================================================

    #[test]
    fn test_box_recursion() {
        // Test deep nesting doesn't cause issues
        let mut expr = FilterExpr::LiteralBool(true);
        for _ in 0..100 {
            expr = FilterExpr::Not(Box::new(expr));
        }

        assert_eq!(expr.depth(), 101);
    }

    // =========================================================================
    // TYPE CHECKING TESTS
    // =========================================================================

    #[test]
    fn test_is_literal() {
        assert!(FilterExpr::LiteralString("test".to_string()).is_literal());
        assert!(FilterExpr::LiteralInt(42).is_literal());
        assert!(FilterExpr::LiteralFloat(3.14).is_literal());
        assert!(FilterExpr::LiteralBool(true).is_literal());
        assert!(FilterExpr::LiteralArray(vec![]).is_literal());
        assert!(!FilterExpr::Field("x".to_string()).is_literal());
    }

    #[test]
    fn test_is_field() {
        assert!(FilterExpr::Field("name".to_string()).is_field());
        assert!(!FilterExpr::LiteralString("name".to_string()).is_field());
    }

    #[test]
    fn test_is_comparison() {
        let field = Box::new(FilterExpr::Field("x".to_string()));
        let value = Box::new(FilterExpr::LiteralInt(1));

        assert!(FilterExpr::Eq(field.clone(), value.clone()).is_comparison());
        assert!(FilterExpr::Ne(field.clone(), value.clone()).is_comparison());
        assert!(FilterExpr::Lt(field.clone(), value.clone()).is_comparison());
        assert!(FilterExpr::Le(field.clone(), value.clone()).is_comparison());
        assert!(FilterExpr::Gt(field.clone(), value.clone()).is_comparison());
        assert!(FilterExpr::Ge(field, value).is_comparison());
        assert!(!FilterExpr::LiteralBool(true).is_comparison());
    }

    #[test]
    fn test_is_string_op() {
        let field = Box::new(FilterExpr::Field("x".to_string()));
        let value = Box::new(FilterExpr::LiteralString("a".to_string()));

        assert!(FilterExpr::Contains(field.clone(), value.clone()).is_string_op());
        assert!(FilterExpr::StartsWith(field.clone(), value.clone()).is_string_op());
        assert!(FilterExpr::EndsWith(field.clone(), value.clone()).is_string_op());
        assert!(FilterExpr::Like(field, value).is_string_op());
    }

    #[test]
    fn test_is_array_op() {
        let field = Box::new(FilterExpr::Field("x".to_string()));
        let arr = Box::new(FilterExpr::LiteralArray(vec![]));

        assert!(FilterExpr::In(field.clone(), arr.clone()).is_array_op());
        assert!(FilterExpr::NotIn(field.clone(), arr.clone()).is_array_op());
        assert!(FilterExpr::Any(field.clone(), arr.clone()).is_array_op());
        assert!(FilterExpr::All(field.clone(), arr.clone()).is_array_op());
        assert!(FilterExpr::None(field, arr).is_array_op());
    }

    #[test]
    fn test_is_logical() {
        assert!(FilterExpr::And(
            Box::new(FilterExpr::LiteralBool(true)),
            Box::new(FilterExpr::LiteralBool(false))
        )
        .is_logical());
        assert!(FilterExpr::Or(
            Box::new(FilterExpr::LiteralBool(true)),
            Box::new(FilterExpr::LiteralBool(false))
        )
        .is_logical());
        assert!(FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true))).is_logical());
    }

    #[test]
    fn test_is_null_check() {
        assert!(FilterExpr::IsNull(Box::new(FilterExpr::Field("x".to_string()))).is_null_check());
        assert!(
            FilterExpr::IsNotNull(Box::new(FilterExpr::Field("x".to_string()))).is_null_check()
        );
    }

    // =========================================================================
    // VALUE EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_as_string() {
        assert_eq!(
            FilterExpr::LiteralString("hello".to_string()).as_string(),
            Some("hello")
        );
        assert_eq!(FilterExpr::LiteralInt(42).as_string(), Option::None);
    }

    #[test]
    fn test_as_int() {
        assert_eq!(FilterExpr::LiteralInt(42).as_int(), Some(42));
        assert_eq!(FilterExpr::LiteralFloat(42.0).as_int(), Option::None);
    }

    #[test]
    fn test_as_float() {
        assert_eq!(FilterExpr::LiteralFloat(3.14).as_float(), Some(3.14));
        assert_eq!(FilterExpr::LiteralInt(3).as_float(), Option::None);
    }

    #[test]
    fn test_as_bool() {
        assert_eq!(FilterExpr::LiteralBool(true).as_bool(), Some(true));
        assert_eq!(FilterExpr::LiteralInt(1).as_bool(), Option::None);
    }

    #[test]
    fn test_as_field() {
        assert_eq!(
            FilterExpr::Field("category".to_string()).as_field(),
            Some("category")
        );
        assert_eq!(
            FilterExpr::LiteralString("category".to_string()).as_field(),
            Option::None
        );
    }

    #[test]
    fn test_as_array() {
        let arr = vec![FilterExpr::LiteralInt(1), FilterExpr::LiteralInt(2)];
        let expr = FilterExpr::LiteralArray(arr.clone());
        assert_eq!(expr.as_array(), Some(&arr[..]));
        assert_eq!(FilterExpr::LiteralInt(1).as_array(), Option::None);
    }

    // =========================================================================
    // OPERATOR NAME TESTS
    // =========================================================================

    #[test]
    fn test_operator_name() {
        assert_eq!(
            FilterExpr::LiteralString("".to_string()).operator_name(),
            "LiteralString"
        );
        assert_eq!(FilterExpr::LiteralInt(0).operator_name(), "LiteralInt");
        assert_eq!(FilterExpr::Field("x".to_string()).operator_name(), "Field");
        assert_eq!(
            FilterExpr::And(
                Box::new(FilterExpr::LiteralBool(true)),
                Box::new(FilterExpr::LiteralBool(false))
            )
            .operator_name(),
            "And"
        );
    }

    // =========================================================================
    // DEPTH CALCULATION TESTS
    // =========================================================================

    #[test]
    fn test_depth_literal() {
        assert_eq!(FilterExpr::LiteralInt(42).depth(), 1);
        assert_eq!(FilterExpr::LiteralString("test".to_string()).depth(), 1);
    }

    #[test]
    fn test_depth_unary() {
        let expr = FilterExpr::Not(Box::new(FilterExpr::LiteralBool(true)));
        assert_eq!(expr.depth(), 2);
    }

    #[test]
    fn test_depth_binary() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::LiteralBool(true)),
            Box::new(FilterExpr::Not(Box::new(FilterExpr::LiteralBool(false)))),
        );
        // Left: depth 1, Right: depth 2, +1 for And = 3
        assert_eq!(expr.depth(), 3);
    }

    #[test]
    fn test_depth_ternary() {
        let expr = FilterExpr::Between(
            Box::new(FilterExpr::Field("x".to_string())),
            Box::new(FilterExpr::LiteralInt(0)),
            Box::new(FilterExpr::LiteralInt(100)),
        );
        assert_eq!(expr.depth(), 2);
    }

    #[test]
    fn test_depth_array() {
        let expr =
            FilterExpr::LiteralArray(vec![FilterExpr::LiteralInt(1), FilterExpr::LiteralInt(2)]);
        // Elements are depth 1, array adds 1 = 2
        assert_eq!(expr.depth(), 2);
    }

    #[test]
    fn test_depth_empty_array() {
        let expr = FilterExpr::LiteralArray(vec![]);
        assert_eq!(expr.depth(), 1);
    }

    // =========================================================================
    // REFERENCED FIELDS TESTS
    // =========================================================================

    #[test]
    fn test_referenced_fields_single() {
        let expr = FilterExpr::Eq(
            Box::new(FilterExpr::Field("category".to_string())),
            Box::new(FilterExpr::LiteralString("gpu".to_string())),
        );
        assert_eq!(expr.referenced_fields(), vec!["category"]);
    }

    #[test]
    fn test_referenced_fields_multiple() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Eq(
                Box::new(FilterExpr::Field("category".to_string())),
                Box::new(FilterExpr::LiteralString("gpu".to_string())),
            )),
            Box::new(FilterExpr::Lt(
                Box::new(FilterExpr::Field("price".to_string())),
                Box::new(FilterExpr::LiteralInt(500)),
            )),
        );
        let fields = expr.referenced_fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"category"));
        assert!(fields.contains(&"price"));
    }

    #[test]
    fn test_referenced_fields_no_fields() {
        let expr = FilterExpr::LiteralBool(true);
        assert!(expr.referenced_fields().is_empty());
    }

    // =========================================================================
    // SERIALIZATION ROUNDTRIP TESTS
    // =========================================================================

    #[test]
    fn test_serialization_complex() {
        let expr = FilterExpr::And(
            Box::new(FilterExpr::Eq(
                Box::new(FilterExpr::Field("category".to_string())),
                Box::new(FilterExpr::LiteralString("gpu".to_string())),
            )),
            Box::new(FilterExpr::Or(
                Box::new(FilterExpr::Lt(
                    Box::new(FilterExpr::Field("price".to_string())),
                    Box::new(FilterExpr::LiteralInt(500)),
                )),
                Box::new(FilterExpr::In(
                    Box::new(FilterExpr::Field("brand".to_string())),
                    Box::new(FilterExpr::LiteralArray(vec![
                        FilterExpr::LiteralString("nvidia".to_string()),
                        FilterExpr::LiteralString("amd".to_string()),
                    ])),
                )),
            )),
        );

        let json = serde_json::to_string(&expr).unwrap();
        let parsed: FilterExpr = serde_json::from_str(&json).unwrap();

        assert_eq!(expr, parsed);
    }

    #[test]
    fn test_serialization_all_literals() {
        let exprs = vec![
            FilterExpr::LiteralString("hello".to_string()),
            FilterExpr::LiteralInt(i64::MAX),
            FilterExpr::LiteralInt(i64::MIN),
            FilterExpr::LiteralFloat(f64::MAX),
            FilterExpr::LiteralFloat(f64::MIN_POSITIVE),
            FilterExpr::LiteralBool(true),
            FilterExpr::LiteralBool(false),
            FilterExpr::LiteralArray(vec![FilterExpr::LiteralInt(1), FilterExpr::LiteralInt(2)]),
        ];

        for expr in exprs {
            let json = serde_json::to_string(&expr).unwrap();
            let parsed: FilterExpr = serde_json::from_str(&json).unwrap();
            assert_eq!(expr, parsed);
        }
    }
}
