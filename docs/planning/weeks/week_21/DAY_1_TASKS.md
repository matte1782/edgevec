# Week 21, Day 1: Metadata API Architecture & Core Types

**Date:** 2025-12-30
**Sprint:** Week 21 (v0.5.0 Phase)
**Day Theme:** Foundation Types & Module Structure
**Status:** PLANNED

---

## Task W21.1: Metadata API Core Types & Module Structure

**Priority:** CRITICAL (P0)
**Estimated Effort:** 8 hours (3x rule: 2h optimistic × 3 = 6h + 2h buffer)
**Status:** PLANNED
**Depends On:** Week 20 complete (GATE_W20_PLANNING_COMPLETE.md exists)
**Blocks:** W21.2, W21.3, W21.4, W21.5

---

### Context

The v0.4.0 external review identified "missing metadata storage" as the #1 user complaint blocking production RAG use cases. Week 21 Day 1 establishes the foundational types for the Metadata Storage API — the most critical deliverable of v0.5.0.

**Strategic Importance:**
- Metadata API is the FOUNDATION for Week 22-23 Filtering implementation
- Schema decisions made today will be FROZEN after Week 21
- This is a non-negotiable prerequisite for v0.5.0 release

**Reference Documents:**
- `docs/planning/V0.5.0_STRATEGIC_ROADMAP.md` (BINDING)
- `docs/reviews/2025-12-16_V0.5.0_PHASED_APPROACH_APPROVED.md`

---

### Objective

Create the `src/metadata/` module with all core types, implementing a type-safe metadata storage system that supports 5 value types (String, Integer, Float, Boolean, StringArray).

---

### Technical Approach

#### 1. Module Structure

Create the following file structure:
```
src/
├── metadata/
│   ├── mod.rs          # Module exports, re-exports
│   ├── types.rs        # MetadataValue enum, type definitions
│   ├── store.rs        # MetadataStore struct (stub for Day 2)
│   ├── error.rs        # Metadata-specific errors
│   └── validation.rs   # Key/value validation rules
├── lib.rs              # Add `pub mod metadata;`
```

#### 2. Core Type Definitions

**File: `src/metadata/types.rs`**
```rust
use serde::{Deserialize, Serialize};

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
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetadataValue {
    /// UTF-8 string value (max 65,536 bytes)
    String(String),

    /// 64-bit signed integer
    Integer(i64),

    /// 64-bit IEEE 754 floating point
    Float(f64),

    /// Boolean true/false
    Boolean(bool),

    /// Array of UTF-8 strings (max 1,024 elements)
    StringArray(Vec<String>),
}

impl MetadataValue {
    /// Returns the type name as a static string.
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
    pub fn is_string(&self) -> bool {
        matches!(self, MetadataValue::String(_))
    }

    /// Returns true if this value is an Integer type.
    pub fn is_integer(&self) -> bool {
        matches!(self, MetadataValue::Integer(_))
    }

    /// Returns true if this value is a Float type.
    pub fn is_float(&self) -> bool {
        matches!(self, MetadataValue::Float(_))
    }

    /// Returns true if this value is a Boolean type.
    pub fn is_boolean(&self) -> bool {
        matches!(self, MetadataValue::Boolean(_))
    }

    /// Returns true if this value is a StringArray type.
    pub fn is_string_array(&self) -> bool {
        matches!(self, MetadataValue::StringArray(_))
    }

    /// Attempts to extract the String value.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetadataValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Attempts to extract the Integer value.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            MetadataValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Attempts to extract the Float value.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            MetadataValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Attempts to extract the Boolean value.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            MetadataValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Attempts to extract the StringArray value.
    pub fn as_string_array(&self) -> Option<&[String]> {
        match self {
            MetadataValue::StringArray(arr) => Some(arr),
            _ => None,
        }
    }
}
```

#### 3. Validation Constants

**File: `src/metadata/validation.rs`**
```rust
/// Maximum number of metadata keys per vector.
pub const MAX_KEYS_PER_VECTOR: usize = 64;

/// Maximum length of a metadata key in bytes.
pub const MAX_KEY_LENGTH: usize = 256;

/// Maximum length of a String metadata value in bytes.
pub const MAX_STRING_VALUE_LENGTH: usize = 65_536; // 64KB

/// Maximum number of elements in a StringArray.
pub const MAX_STRING_ARRAY_LENGTH: usize = 1_024;

/// Validates a metadata key.
pub fn validate_key(key: &str) -> Result<(), MetadataError> {
    if key.is_empty() {
        return Err(MetadataError::EmptyKey);
    }
    if key.len() > MAX_KEY_LENGTH {
        return Err(MetadataError::KeyTooLong {
            length: key.len(),
            max: MAX_KEY_LENGTH,
        });
    }
    // Keys must be valid UTF-8 (guaranteed by &str)
    // Keys should be alphanumeric + underscore for compatibility
    if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(MetadataError::InvalidKeyFormat {
            key: key.to_string(),
        });
    }
    Ok(())
}

/// Validates a metadata value.
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
            if arr.len() > MAX_STRING_ARRAY_LENGTH {
                return Err(MetadataError::ArrayTooLong {
                    length: arr.len(),
                    max: MAX_STRING_ARRAY_LENGTH,
                });
            }
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
                return Err(MetadataError::InvalidFloat { reason: "NaN not allowed" });
            }
            if f.is_infinite() {
                return Err(MetadataError::InvalidFloat { reason: "Infinity not allowed" });
            }
        }
        // Integer and Boolean have no validation constraints
        MetadataValue::Integer(_) | MetadataValue::Boolean(_) => {}
    }
    Ok(())
}
```

#### 4. Error Types

**File: `src/metadata/error.rs`**
```rust
use thiserror::Error;

/// Errors that can occur during metadata operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MetadataError {
    #[error("metadata key cannot be empty")]
    EmptyKey,

    #[error("metadata key too long: {length} bytes (max {max})")]
    KeyTooLong { length: usize, max: usize },

    #[error("invalid key format: '{key}' (must be alphanumeric + underscore)")]
    InvalidKeyFormat { key: String },

    #[error("string value too long: {length} bytes (max {max})")]
    StringValueTooLong { length: usize, max: usize },

    #[error("string array too long: {length} elements (max {max})")]
    ArrayTooLong { length: usize, max: usize },

    #[error("invalid float value: {reason}")]
    InvalidFloat { reason: &'static str },

    #[error("too many keys for vector {vector_id}: {count} (max {max})")]
    TooManyKeys { vector_id: u32, count: usize, max: usize },

    #[error("vector {vector_id} not found")]
    VectorNotFound { vector_id: u32 },

    #[error("key '{key}' not found for vector {vector_id}")]
    KeyNotFound { vector_id: u32, key: String },

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),
}
```

---

### Acceptance Criteria

**CRITICAL (Must Pass):**
- [ ] `src/metadata/mod.rs` exists and exports all public types
- [ ] `MetadataValue` enum has exactly 5 variants: String, Integer, Float, Boolean, StringArray
- [ ] `MetadataValue` derives `Clone, Debug, PartialEq, Serialize, Deserialize`
- [ ] `#[serde(tag = "type", content = "value")]` annotation for JSON compatibility
- [ ] All validation constants defined with documented rationale
- [ ] `MetadataError` enum covers all error cases
- [ ] Module compiles with `cargo check` (exit code 0)

**MAJOR (Should Pass):**
- [ ] Type accessor methods (`as_string()`, `as_integer()`, etc.) implemented
- [ ] Type check methods (`is_string()`, `is_integer()`, etc.) implemented
- [ ] `type_name()` method returns lowercase string for JSON compatibility
- [ ] `validate_key()` and `validate_value()` functions implemented
- [ ] Doc comments with examples for all public items
- [ ] Unit tests for serialization roundtrip (all 5 types)

**MINOR (Nice to Have):**
- [ ] `From<T>` trait implementations for common conversions
- [ ] `Display` trait implementation for `MetadataValue`
- [ ] Benchmark for serialization performance

---

### Implementation Checklist

- [ ] Create `src/metadata/` directory
- [ ] Create `src/metadata/mod.rs` with module structure
- [ ] Create `src/metadata/types.rs` with `MetadataValue` enum
- [ ] Create `src/metadata/validation.rs` with constants and validators
- [ ] Create `src/metadata/error.rs` with `MetadataError` enum
- [ ] Create `src/metadata/store.rs` with `MetadataStore` struct stub
- [ ] Add `pub mod metadata;` to `src/lib.rs`
- [ ] Verify `cargo check` passes
- [ ] Verify `cargo clippy -- -D warnings` passes
- [ ] Write unit tests for `MetadataValue` serialization
- [ ] Write unit tests for validation functions
- [ ] Write doc tests for public API examples

---

### Test Requirements

**Unit Tests (Required):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

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
        let value = MetadataValue::Float(3.14159);
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

    #[test]
    fn test_validate_key_empty() {
        assert!(validate_key("").is_err());
    }

    #[test]
    fn test_validate_key_too_long() {
        let long_key = "a".repeat(MAX_KEY_LENGTH + 1);
        assert!(validate_key(&long_key).is_err());
    }

    #[test]
    fn test_validate_key_valid() {
        assert!(validate_key("my_key_123").is_ok());
    }

    #[test]
    fn test_validate_value_nan() {
        let value = MetadataValue::Float(f64::NAN);
        assert!(validate_value(&value).is_err());
    }

    #[test]
    fn test_validate_value_infinity() {
        let value = MetadataValue::Float(f64::INFINITY);
        assert!(validate_value(&value).is_err());
    }
}
```

**Coverage Target:** >90% line coverage for types.rs, validation.rs, error.rs

---

### Performance Targets

| Operation | Target | Notes |
|:----------|:-------|:------|
| `MetadataValue` serialization | <1µs per value | Small values |
| `MetadataValue` deserialization | <1µs per value | Small values |
| `validate_key()` | <100ns | String validation |
| `validate_value()` | <1µs | Depends on type |

---

### Documentation Requirements

- [ ] Module-level doc comment explaining metadata system
- [ ] Doc comments for `MetadataValue` enum with type mapping table
- [ ] Doc comments for each variant explaining constraints
- [ ] Doc comments for all validation constants
- [ ] Doc examples that compile and run

---

### Dependencies

**Blocks:**
- W21.2 (Implementation needs types defined first)
- W21.3 (WASM bindings need types)
- W21.4 (Mobile testing needs working API)
- W21.5 (CI needs everything working)

**Blocked By:**
- Week 20 complete (GATE_W20_PLANNING_COMPLETE.md) ✅

**External Dependencies:**
- `serde` (already in Cargo.toml)
- `serde_json` (already in Cargo.toml)
- `thiserror` (already in Cargo.toml)

---

### Verification Method

**Day 1 is COMPLETE when:**

1. Run verification commands:
   ```bash
   cargo check
   cargo test --lib metadata
   cargo clippy -- -D warnings
   cargo doc --no-deps
   ```

2. All commands exit with code 0

3. JSON serialization produces expected format:
   ```json
   {"type":"string","value":"hello"}
   {"type":"integer","value":42}
   {"type":"float","value":3.14159}
   {"type":"boolean","value":true}
   {"type":"string_array","value":["a","b","c"]}
   ```

---

### Rollback Plan

If Day 1 encounters blocking issues:

1. **Serialization format issues:** Fall back to `#[serde(untagged)]` if tagged unions cause problems
2. **Validation too strict:** Relax constraints (can always tighten later)
3. **Module structure issues:** Flatten into single file if necessary
4. **Type system issues:** Remove `StringArray` variant (least common use case)

---

### Estimated Timeline

| Phase | Time | Cumulative |
|:------|:-----|:-----------|
| Module scaffolding | 1h | 1h |
| `MetadataValue` implementation | 2h | 3h |
| Validation functions | 1.5h | 4.5h |
| Error types | 1h | 5.5h |
| Unit tests | 1.5h | 7h |
| Documentation | 0.5h | 7.5h |
| Buffer | 0.5h | 8h |

---

### Hostile Review Checkpoint

**End of Day 1:** Submit for `/review` with:
- `src/metadata/mod.rs`
- `src/metadata/types.rs`
- `src/metadata/validation.rs`
- `src/metadata/error.rs`
- Unit tests in `src/metadata/types.rs`

**Expected Review Focus:**
- Type system correctness
- Serialization format compatibility
- Validation completeness
- API ergonomics

---

**Task Owner:** RUST_ENGINEER
**Review Required:** HOSTILE_REVIEWER
**Next Task:** W21.2 (Metadata Storage Implementation)

---

*"Types first. Validation second. Implementation last."*
