//! WASM Bindings for EdgeVec Metadata API.
//!
//! This module provides JavaScript-friendly wrappers for the metadata system,
//! allowing browser applications to attach, query, and manage metadata on vectors.
//!
//! # JavaScript Usage
//!
//! ```javascript
//! import { EdgeVec, JsMetadataValue } from 'edgevec';
//!
//! const db = new EdgeVec(config);
//! const id = db.insert(vector);
//!
//! // Attach metadata
//! db.setMetadata(id, 'title', JsMetadataValue.fromString('My Document'));
//! db.setMetadata(id, 'page_count', JsMetadataValue.fromInteger(42));
//!
//! // Retrieve metadata
//! const title = db.getMetadata(id, 'title');
//! console.log(title.asString()); // 'My Document'
//!
//! // Get all metadata as JS object
//! const all = db.getAllMetadata(id);
//! console.log(all); // { title: 'My Document', page_count: 42 }
//! ```

use crate::metadata::{MetadataError, MetadataStore, MetadataValue};
use js_sys::Array;
use wasm_bindgen::prelude::*;

// =============================================================================
// JavaScript Safe Integer Constants
// =============================================================================

/// Maximum safe integer in JavaScript (2^53 - 1).
/// Values larger than this may lose precision when stored in f64.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Minimum safe integer in JavaScript (-(2^53 - 1)).
const MIN_SAFE_INTEGER: f64 = -9_007_199_254_740_991.0;

// =============================================================================
// JsMetadataValue - JavaScript-friendly wrapper for MetadataValue
// =============================================================================

/// JavaScript-friendly metadata value representation.
///
/// This type bridges Rust's `MetadataValue` enum to JavaScript objects.
/// Use the static factory methods (`fromString`, `fromInteger`, etc.) to create
/// values from JavaScript.
///
/// # Example (JavaScript)
///
/// ```javascript
/// const strValue = JsMetadataValue.fromString('hello');
/// const intValue = JsMetadataValue.fromInteger(42);
/// const floatValue = JsMetadataValue.fromFloat(3.14);
/// const boolValue = JsMetadataValue.fromBoolean(true);
/// const arrValue = JsMetadataValue.fromStringArray(['a', 'b', 'c']);
///
/// console.log(strValue.getType()); // 'string'
/// console.log(intValue.toJS());    // 42
/// ```
#[wasm_bindgen]
pub struct JsMetadataValue {
    pub(crate) inner: MetadataValue,
}

#[wasm_bindgen]
impl JsMetadataValue {
    // =========================================================================
    // Factory Methods (Static Constructors)
    // =========================================================================

    /// Creates a string metadata value.
    ///
    /// @param value - The string value
    /// @returns A new JsMetadataValue containing a string
    #[wasm_bindgen(js_name = "fromString")]
    #[must_use]
    pub fn from_string(value: String) -> Self {
        Self {
            inner: MetadataValue::String(value),
        }
    }

    /// Creates an integer metadata value.
    ///
    /// JavaScript numbers are always f64, so this method validates the input
    /// to ensure it's a valid integer within JavaScript's safe integer range.
    ///
    /// @param value - The integer value (must be within ±(2^53 - 1))
    /// @returns A new JsMetadataValue containing an integer
    /// @throws {Error} If value is outside safe integer range or has fractional part
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Value exceeds JavaScript's safe integer range (±9007199254740991)
    /// - Value has a fractional part (e.g., 3.14)
    /// - Value is NaN or Infinity
    #[wasm_bindgen(js_name = "fromInteger")]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_integer(value: f64) -> Result<Self, JsError> {
        // Check for NaN or Infinity
        if !value.is_finite() {
            return Err(JsError::new(
                "Integer value must be finite (not NaN or Infinity)",
            ));
        }

        // Check for fractional part
        if value.fract() != 0.0 {
            return Err(JsError::new(&format!(
                "Value {value} is not an integer (has fractional part)"
            )));
        }

        // Check safe integer range
        if !(MIN_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
            return Err(JsError::new(&format!(
                "Integer value {value} exceeds JavaScript safe integer range (±{MAX_SAFE_INTEGER})"
            )));
        }

        // JavaScript doesn't have a native i64 type, so we receive f64
        // and convert to i64. This is safe within the validated range.
        Ok(Self {
            inner: MetadataValue::Integer(value as i64),
        })
    }

    /// Creates a float metadata value.
    ///
    /// @param value - The float value (must not be NaN or Infinity)
    /// @returns A new JsMetadataValue containing a float
    #[wasm_bindgen(js_name = "fromFloat")]
    #[must_use]
    pub fn from_float(value: f64) -> Self {
        Self {
            inner: MetadataValue::Float(value),
        }
    }

    /// Creates a boolean metadata value.
    ///
    /// @param value - The boolean value
    /// @returns A new JsMetadataValue containing a boolean
    #[wasm_bindgen(js_name = "fromBoolean")]
    #[must_use]
    pub fn from_boolean(value: bool) -> Self {
        Self {
            inner: MetadataValue::Boolean(value),
        }
    }

    /// Creates a string array metadata value.
    ///
    /// @param value - An array of strings
    /// @returns A new JsMetadataValue containing a string array
    ///
    /// # Errors
    ///
    /// Returns an error if any array element is not a string.
    #[wasm_bindgen(js_name = "fromStringArray")]
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_string_array(value: Array) -> Result<Self, JsError> {
        let mut strings = Vec::with_capacity(value.length() as usize);

        for i in 0..value.length() {
            let item = value.get(i);
            let s = item
                .as_string()
                .ok_or_else(|| JsError::new("Array elements must be strings"))?;
            strings.push(s);
        }

        Ok(Self {
            inner: MetadataValue::StringArray(strings),
        })
    }

    // =========================================================================
    // Type Inspection
    // =========================================================================

    /// Returns the type of this value.
    ///
    /// @returns One of: 'string', 'integer', 'float', 'boolean', 'string_array'
    #[wasm_bindgen(js_name = "getType")]
    #[must_use]
    pub fn get_type(&self) -> String {
        self.inner.type_name().to_string()
    }

    /// Checks if this value is a string.
    #[wasm_bindgen(js_name = "isString")]
    #[must_use]
    pub fn is_string(&self) -> bool {
        self.inner.is_string()
    }

    /// Checks if this value is an integer.
    #[wasm_bindgen(js_name = "isInteger")]
    #[must_use]
    pub fn is_integer(&self) -> bool {
        self.inner.is_integer()
    }

    /// Checks if this value is a float.
    #[wasm_bindgen(js_name = "isFloat")]
    #[must_use]
    pub fn is_float(&self) -> bool {
        self.inner.is_float()
    }

    /// Checks if this value is a boolean.
    #[wasm_bindgen(js_name = "isBoolean")]
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        self.inner.is_boolean()
    }

    /// Checks if this value is a string array.
    #[wasm_bindgen(js_name = "isStringArray")]
    #[must_use]
    pub fn is_string_array(&self) -> bool {
        self.inner.is_string_array()
    }

    // =========================================================================
    // Value Extraction
    // =========================================================================

    /// Gets the value as a string.
    ///
    /// @returns The string value, or undefined if not a string
    #[wasm_bindgen(js_name = "asString")]
    #[must_use]
    pub fn as_string(&self) -> Option<String> {
        self.inner.as_string().map(String::from)
    }

    /// Gets the value as an integer.
    ///
    /// Note: Returns as f64 for JavaScript compatibility. Safe for integers up to ±2^53.
    ///
    /// @returns The integer value as a number, or undefined if not an integer
    #[wasm_bindgen(js_name = "asInteger")]
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_integer(&self) -> Option<f64> {
        // Return as f64 for JavaScript compatibility
        self.inner.as_integer().map(|i| i as f64)
    }

    /// Gets the value as a float.
    ///
    /// @returns The float value, or undefined if not a float
    #[wasm_bindgen(js_name = "asFloat")]
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        self.inner.as_float()
    }

    /// Gets the value as a boolean.
    ///
    /// @returns The boolean value, or undefined if not a boolean
    #[wasm_bindgen(js_name = "asBoolean")]
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        self.inner.as_boolean()
    }

    /// Gets the value as a string array.
    ///
    /// @returns The string array, or undefined if not a string array
    #[wasm_bindgen(js_name = "asStringArray")]
    #[must_use]
    pub fn as_string_array(&self) -> JsValue {
        match self.inner.as_string_array() {
            Some(arr) => {
                let js_array = Array::new();
                for s in arr {
                    js_array.push(&JsValue::from_str(s));
                }
                js_array.into()
            }
            None => JsValue::UNDEFINED,
        }
    }

    // =========================================================================
    // JavaScript Conversion
    // =========================================================================

    /// Converts to a JavaScript-native value.
    ///
    /// Returns:
    /// - `string` for String values
    /// - `number` for Integer and Float values
    /// - `boolean` for Boolean values
    /// - `string[]` for StringArray values
    ///
    /// @returns The JavaScript-native value
    #[wasm_bindgen(js_name = "toJS")]
    #[must_use]
    pub fn to_js(&self) -> JsValue {
        match &self.inner {
            MetadataValue::String(s) => JsValue::from_str(s),
            MetadataValue::Integer(i) => {
                // Safe cast for values up to ±2^53
                #[allow(clippy::cast_precision_loss)]
                JsValue::from_f64(*i as f64)
            }
            MetadataValue::Float(f) => JsValue::from_f64(*f),
            MetadataValue::Boolean(b) => JsValue::from_bool(*b),
            MetadataValue::StringArray(arr) => {
                let js_array = Array::new();
                for s in arr {
                    js_array.push(&JsValue::from_str(s));
                }
                js_array.into()
            }
        }
    }
}

// =============================================================================
// Helper Functions for EdgeVec Integration
// =============================================================================

/// Internal helper to convert MetadataError to JsError.
///
/// Takes ownership of the error since it's typically used in `.map_err()`
/// where the error is consumed anyway.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn metadata_error_to_js(e: MetadataError) -> JsError {
    JsError::new(&e.to_string())
}

/// Internal helper to convert Option<&MetadataValue> to Option<JsMetadataValue>.
pub(crate) fn metadata_value_to_js(value: Option<&MetadataValue>) -> Option<JsMetadataValue> {
    value.map(|v| JsMetadataValue { inner: v.clone() })
}

/// Internal helper to convert all metadata for a vector to a JS object.
pub(crate) fn metadata_to_js_object(store: &MetadataStore, vector_id: u32) -> JsValue {
    match store.get_all(vector_id) {
        Some(metadata) => {
            let obj = js_sys::Object::new();
            for (key, value) in metadata {
                let js_value = JsMetadataValue {
                    inner: value.clone(),
                };
                // Silently ignore errors (shouldn't happen for valid keys)
                let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(key), &js_value.to_js());
            }
            obj.into()
        }
        None => JsValue::UNDEFINED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_metadata_value_string() {
        let value = JsMetadataValue::from_string("hello".to_string());
        assert!(value.is_string());
        assert_eq!(value.get_type(), "string");
        assert_eq!(value.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_js_metadata_value_integer() {
        let value = JsMetadataValue::from_integer(42.0).unwrap();
        assert!(value.is_integer());
        assert_eq!(value.get_type(), "integer");
        assert_eq!(value.as_integer(), Some(42.0));
    }

    #[test]
    fn test_from_integer_valid_range() {
        // Valid integers should work (these don't call JsError::new)
        assert!(JsMetadataValue::from_integer(0.0).is_ok());
        assert!(JsMetadataValue::from_integer(-1.0).is_ok());
        assert!(JsMetadataValue::from_integer(1000000.0).is_ok());

        // MAX_SAFE_INTEGER should work
        let max_safe = 9_007_199_254_740_991.0;
        assert!(JsMetadataValue::from_integer(max_safe).is_ok());
        assert!(JsMetadataValue::from_integer(-max_safe).is_ok());
    }

    // Note: Tests for invalid integers (fractional, NaN, Infinity, out of range)
    // cannot be run on non-wasm targets because JsError::new is wasm-only.
    // These validations are tested via wasm-pack integration tests.

    #[test]
    fn test_js_metadata_value_float() {
        let value = JsMetadataValue::from_float(3.14);
        assert!(value.is_float());
        assert_eq!(value.get_type(), "float");
        assert_eq!(value.as_float(), Some(3.14));
    }

    #[test]
    fn test_js_metadata_value_boolean() {
        let value = JsMetadataValue::from_boolean(true);
        assert!(value.is_boolean());
        assert_eq!(value.get_type(), "boolean");
        assert_eq!(value.as_boolean(), Some(true));
    }

    // Note: test_metadata_error_to_js is not included because JsError::new
    // can only be called on wasm targets. The function is tested via
    // integration tests in wasm-pack test.
}
