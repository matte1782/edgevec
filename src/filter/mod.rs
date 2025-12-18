//! Filter expression module for EdgeVec.
//!
//! This module provides a powerful filtering system for vector search queries,
//! allowing users to filter results based on metadata fields.
//!
//! # Architecture
//!
//! The filter subsystem consists of:
//! - **AST (`ast.rs`)**: 27-variant `FilterExpr` enum representing parsed filter expressions
//! - **Parser (`parser.rs`)**: Pest-based parser converting filter strings to AST
//! - **Evaluator (`evaluator.rs`)**: Recursive evaluator with short-circuit optimization
//! - **Error (`error.rs`)**: Comprehensive error types with position information
//!
//! # Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use edgevec::filter::{parse, evaluate, FilterExpr};
//! use edgevec::metadata::MetadataValue;
//!
//! // Parse a filter expression
//! let expr = parse("category = \"gpu\" AND price < 500").unwrap();
//!
//! // Evaluate against metadata
//! let mut metadata = HashMap::new();
//! metadata.insert("category".to_string(), MetadataValue::String("gpu".to_string()));
//! metadata.insert("price".to_string(), MetadataValue::Integer(450));
//!
//! let result = evaluate(&expr, &metadata).unwrap();
//! assert!(result);
//! ```
//!
//! # Grammar
//!
//! The filter syntax supports:
//! - Comparison operators: `=`, `!=`, `<`, `<=`, `>`, `>=`
//! - String operators: `CONTAINS`, `STARTS_WITH`, `ENDS_WITH`, `LIKE`
//! - Array operators: `IN`, `NOT IN`, `ANY`, `ALL`, `NONE`
//! - Range operator: `BETWEEN`
//! - Logical operators: `AND`, `OR`, `NOT`
//! - Null checks: `IS NULL`, `IS NOT NULL`
//!
//! # Implementation Status
//!
//! - [x] W23.1.1: FilterExpr AST enum (27 variants)
//! - [x] W23.1.2: Pest grammar file
//! - [x] W23.1.3: AST builder from parse tree
//! - [x] W23.1.4: Error handling with positions
//! - [x] W23.2.1: Core evaluate() function
//! - [x] W23.2.2: Comparison operators
//! - [x] W23.2.3: String operators
//! - [x] W23.2.4: Array operators
//! - [x] W23.3.1: FilterStrategy enum
//! - [x] W23.3.2: Selectivity estimation (estimate_selectivity)
//! - [x] W23.3.3: FilteredSearcher API (search_filtered)
//! - [x] W23.3.4: Tautology/contradiction detection

pub mod ast;
pub mod error;
pub mod evaluator;
pub mod filtered_search;
pub mod parser;
pub mod strategy;

// Re-exports for convenience
pub use ast::FilterExpr;
pub use error::FilterError;
pub use evaluator::evaluate;
pub use filtered_search::{
    FilteredSearchError, FilteredSearchResult, FilteredSearcher, VectorMetadataStore,
};
pub use parser::parse;
pub use strategy::{estimate_selectivity, FilterStrategy, MetadataStore, SelectivityEstimate};
