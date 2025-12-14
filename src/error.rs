//! Unified error hierarchy for EdgeVec.
//!
//! This module defines the error types used throughout EdgeVec:
//!
//! - `EdgeVecError` — Top-level error type wrapping all component errors
//! - [`BatchError`] — Errors specific to batch insertion operations
//!
//! # Error Mapping
//!
//! All errors automatically convert to JavaScript objects when used in WASM,
//! with `code` and `message` properties for structured error handling.
//!
//! # Batch Error Handling
//!
//! [`BatchError`] supports **best-effort semantics**:
//! - Fatal errors (dimension mismatch on first vector, capacity exceeded) abort immediately
//! - Non-fatal errors (duplicates, invalid vectors mid-batch) are skipped
//! - Partial success is returned via `Ok(Vec<u64>)`
//!
//! # Example
//!
//! ```ignore
//! use edgevec::error::BatchError;
//!
//! fn handle_batch_error(err: BatchError) {
//!     match err {
//!         BatchError::DimensionMismatch { expected, actual, vector_id } => {
//!             eprintln!("Vector {} has {} dims, expected {}", vector_id, actual, expected);
//!         }
//!         BatchError::DuplicateId { vector_id } => {
//!             eprintln!("Duplicate ID: {}", vector_id);
//!         }
//!         BatchError::CapacityExceeded { current, max } => {
//!             eprintln!("Index full: {}/{}", current, max);
//!         }
//!         _ => eprintln!("Other error: {}", err),
//!     }
//! }
//! ```

use crate::hnsw::GraphError;
use crate::persistence::PersistenceError;
use thiserror::Error;

/// The Unified EdgeVec Error type.
#[derive(Debug, Error)]
pub enum EdgeVecError {
    /// Input/Output errors (filesystem, network, etc).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Persistence and storage errors.
    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    /// Graph algorithm and index errors.
    #[error(transparent)]
    Graph(#[from] GraphError),

    /// Validation errors (invalid arguments, dimensions, etc).
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Errors that can occur during batch insertion operations.
///
/// This type represents errors specific to batch insertion workflows.
/// Unlike `GraphError`, which handles single-vector operations,
/// `BatchError` provides context about which vector in a batch failed
/// and supports best-effort semantics (partial success).
///
/// # WASM Error Codes
///
/// When used via WASM, these errors map to JavaScript error objects with `code` property:
/// - `EmptyBatch` → `EMPTY_BATCH`
/// - `DimensionMismatch` → `DIMENSION_MISMATCH`
/// - `DuplicateId` → `DUPLICATE_ID`
/// - `InvalidVector` → `INVALID_VECTOR`
/// - `CapacityExceeded` → `CAPACITY_EXCEEDED`
/// - `InternalError` → `INTERNAL_ERROR`
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BatchError {
    /// Empty batch provided (no vectors to insert).
    #[error("Empty batch: cannot insert zero vectors")]
    EmptyBatch,

    /// Vector dimensionality does not match index configuration.
    #[error("Dimension mismatch for vector {vector_id}: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension from index
        expected: usize,
        /// Actual dimension of rejected vector
        actual: usize,
        /// ID of the problematic vector
        vector_id: u64,
    },

    /// Vector ID already exists in the index.
    #[error("Duplicate vector ID: {vector_id}")]
    DuplicateId {
        /// Duplicate vector ID
        vector_id: u64,
    },

    /// Vector contains invalid floating-point values (NaN, Infinity).
    #[error("Invalid vector {vector_id}: {reason}")]
    InvalidVector {
        /// ID of the invalid vector
        vector_id: u64,
        /// Description of the invalid value
        reason: String,
    },

    /// Index has reached maximum capacity.
    #[error("Capacity exceeded: current={current}, max={max}")]
    CapacityExceeded {
        /// Current number of vectors
        current: usize,
        /// Maximum allowed vectors
        max: usize,
    },

    /// Internal HNSW invariant violated during insertion.
    #[error("Internal error: {message}")]
    InternalError {
        /// Description of the violated invariant
        message: String,
    },
}

use wasm_bindgen::prelude::*;

impl From<EdgeVecError> for JsValue {
    fn from(err: EdgeVecError) -> Self {
        let (code, msg) = match &err {
            EdgeVecError::Io(e) => ("ERR_IO", e.to_string()),

            EdgeVecError::Persistence(pe) => match pe {
                PersistenceError::Io(e) => ("ERR_IO", e.to_string()),
                PersistenceError::ChecksumMismatch { .. }
                | PersistenceError::Corrupted(_)
                | PersistenceError::InvalidMagic { .. } => ("ERR_CORRUPTION", pe.to_string()),
                _ => ("ERR_PERSISTENCE", pe.to_string()),
            },

            EdgeVecError::Graph(ge) => match ge {
                GraphError::ConfigMismatch { .. } | GraphError::DimensionMismatch { .. } => {
                    ("ERR_DIMENSION", ge.to_string())
                }
                GraphError::CapacityExceeded => ("ERR_CAPACITY", ge.to_string()),
                _ => ("ERR_GRAPH", ge.to_string()),
            },

            EdgeVecError::Validation(msg) => ("ERR_VALIDATION", msg.clone()),
        };

        // Create JS Error object with code property
        // Note: In a real implementation we might use a helper to create
        // a custom object with code/message, or just an Error with prefix.
        // Here we return a plain Error object, but we could attach `code`.
        // Ideally: new Error(msg); err.code = code;

        // Simple string for now as fallback, but ideally Object:
        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"code".into(), &code.into());
        let _ = js_sys::Reflect::set(&obj, &"message".into(), &msg.into());
        obj.into()
    }
}

impl From<BatchError> for JsValue {
    fn from(err: BatchError) -> Self {
        let (code, msg) = match &err {
            BatchError::EmptyBatch => ("EMPTY_BATCH", err.to_string()),
            BatchError::DimensionMismatch { .. } => ("DIMENSION_MISMATCH", err.to_string()),
            BatchError::DuplicateId { .. } => ("DUPLICATE_ID", err.to_string()),
            BatchError::InvalidVector { .. } => ("INVALID_VECTOR", err.to_string()),
            BatchError::CapacityExceeded { .. } => ("CAPACITY_EXCEEDED", err.to_string()),
            BatchError::InternalError { .. } => ("INTERNAL_ERROR", err.to_string()),
        };

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"code".into(), &code.into());
        let _ = js_sys::Reflect::set(&obj, &"message".into(), &msg.into());
        obj.into()
    }
}
