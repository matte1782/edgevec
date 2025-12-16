# Week 21, Day 3: Metadata WASM Bindings & TypeScript Types

**Date:** 2026-01-01
**Sprint:** Week 21 (v0.5.0 Phase)
**Day Theme:** Browser API & TypeScript Integration
**Status:** PLANNED

---

## Task W21.3: Metadata WASM Bindings & TypeScript Definitions

**Priority:** CRITICAL (P0)
**Estimated Effort:** 8 hours (3x rule: 2h optimistic × 3 = 6h + 2h buffer)
**Status:** PLANNED
**Depends On:** W21.2 complete (CRUD operations working)
**Blocks:** W21.4, W21.5

---

### Context

Day 3 creates the browser-facing API for metadata operations. The WASM bindings must be ergonomic for JavaScript/TypeScript developers while maintaining the safety guarantees of the Rust implementation.

**Strategic Importance:**
- WASM API is the primary interface for browser users
- TypeScript types enable IDE autocomplete and type safety
- Bundle size must stay under 500KB gzipped

**Reference Documents:**
- `src/wasm/lib.rs` (existing WASM bindings)
- `pkg/edgevec.d.ts` (existing TypeScript definitions)

---

### Objective

Create complete WASM bindings for the Metadata API with:
1. All CRUD operations exported to JavaScript
2. TypeScript type definitions generated
3. Ergonomic JavaScript API design
4. No bundle size regression

---

### Technical Approach

#### 1. WASM Bindings Implementation

**File: `src/wasm/metadata.rs`**
```rust
use wasm_bindgen::prelude::*;
use crate::metadata::{MetadataStore, MetadataValue, MetadataError};

/// JavaScript-friendly metadata value representation.
///
/// This type bridges Rust's `MetadataValue` enum to JavaScript objects.
#[wasm_bindgen]
pub struct JsMetadataValue {
    inner: MetadataValue,
}

#[wasm_bindgen]
impl JsMetadataValue {
    /// Creates a string metadata value.
    #[wasm_bindgen(js_name = "fromString")]
    pub fn from_string(value: String) -> Self {
        Self { inner: MetadataValue::String(value) }
    }

    /// Creates an integer metadata value.
    #[wasm_bindgen(js_name = "fromInteger")]
    pub fn from_integer(value: i64) -> Self {
        Self { inner: MetadataValue::Integer(value) }
    }

    /// Creates a float metadata value.
    #[wasm_bindgen(js_name = "fromFloat")]
    pub fn from_float(value: f64) -> Self {
        Self { inner: MetadataValue::Float(value) }
    }

    /// Creates a boolean metadata value.
    #[wasm_bindgen(js_name = "fromBoolean")]
    pub fn from_boolean(value: bool) -> Self {
        Self { inner: MetadataValue::Boolean(value) }
    }

    /// Creates a string array metadata value.
    #[wasm_bindgen(js_name = "fromStringArray")]
    pub fn from_string_array(value: Vec<String>) -> Self {
        Self { inner: MetadataValue::StringArray(value) }
    }

    /// Returns the type of this value.
    #[wasm_bindgen(js_name = "getType")]
    pub fn get_type(&self) -> String {
        self.inner.type_name().to_string()
    }

    /// Gets the value as a string (returns undefined if not a string).
    #[wasm_bindgen(js_name = "asString")]
    pub fn as_string(&self) -> Option<String> {
        self.inner.as_string().map(|s| s.to_string())
    }

    /// Gets the value as an integer (returns undefined if not an integer).
    #[wasm_bindgen(js_name = "asInteger")]
    pub fn as_integer(&self) -> Option<i64> {
        self.inner.as_integer()
    }

    /// Gets the value as a float (returns undefined if not a float).
    #[wasm_bindgen(js_name = "asFloat")]
    pub fn as_float(&self) -> Option<f64> {
        self.inner.as_float()
    }

    /// Gets the value as a boolean (returns undefined if not a boolean).
    #[wasm_bindgen(js_name = "asBoolean")]
    pub fn as_boolean(&self) -> Option<bool> {
        self.inner.as_boolean()
    }

    /// Gets the value as a string array (returns undefined if not an array).
    #[wasm_bindgen(js_name = "asStringArray")]
    pub fn as_string_array(&self) -> Option<Vec<String>> {
        self.inner.as_string_array().map(|arr| arr.to_vec())
    }

    /// Converts to a JavaScript-native value.
    ///
    /// Returns:
    /// - string for String
    /// - number for Integer and Float
    /// - boolean for Boolean
    /// - string[] for StringArray
    #[wasm_bindgen(js_name = "toJS")]
    pub fn to_js(&self) -> JsValue {
        match &self.inner {
            MetadataValue::String(s) => JsValue::from_str(s),
            MetadataValue::Integer(i) => JsValue::from_f64(*i as f64),
            MetadataValue::Float(f) => JsValue::from_f64(*f),
            MetadataValue::Boolean(b) => JsValue::from_bool(*b),
            MetadataValue::StringArray(arr) => {
                let js_array = js_sys::Array::new();
                for s in arr {
                    js_array.push(&JsValue::from_str(s));
                }
                js_array.into()
            }
        }
    }
}

/// Extend HnswIndex with metadata operations.
#[wasm_bindgen]
impl crate::wasm::HnswIndexWasm {
    /// Sets metadata for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @param key - The metadata key (alphanumeric + underscore, max 256 chars)
    /// @param value - The metadata value
    /// @throws Error if validation fails or key limit exceeded
    #[wasm_bindgen(js_name = "setMetadata")]
    pub fn set_metadata(
        &mut self,
        vector_id: u32,
        key: &str,
        value: &JsMetadataValue,
    ) -> Result<(), JsError> {
        self.inner
            .metadata
            .insert(vector_id, key, value.inner.clone())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Gets metadata for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @param key - The metadata key
    /// @returns The metadata value, or undefined if not found
    #[wasm_bindgen(js_name = "getMetadata")]
    pub fn get_metadata(&self, vector_id: u32, key: &str) -> Option<JsMetadataValue> {
        self.inner
            .metadata
            .get(vector_id, key)
            .map(|v| JsMetadataValue { inner: v.clone() })
    }

    /// Gets all metadata for a vector as a JavaScript object.
    ///
    /// @param vectorId - The vector ID
    /// @returns An object mapping keys to values, or undefined if no metadata
    #[wasm_bindgen(js_name = "getAllMetadata")]
    pub fn get_all_metadata(&self, vector_id: u32) -> JsValue {
        match self.inner.metadata.get_all(vector_id) {
            Some(metadata) => {
                let obj = js_sys::Object::new();
                for (key, value) in metadata {
                    let js_value = JsMetadataValue { inner: value.clone() };
                    js_sys::Reflect::set(&obj, &JsValue::from_str(key), &js_value.to_js())
                        .expect("setting property should succeed");
                }
                obj.into()
            }
            None => JsValue::UNDEFINED,
        }
    }

    /// Deletes metadata for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @param key - The metadata key to delete
    /// @returns true if the key existed and was deleted
    #[wasm_bindgen(js_name = "deleteMetadata")]
    pub fn delete_metadata(&mut self, vector_id: u32, key: &str) -> Result<bool, JsError> {
        self.inner
            .metadata
            .delete(vector_id, key)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Deletes all metadata for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @returns true if the vector had metadata that was deleted
    #[wasm_bindgen(js_name = "deleteAllMetadata")]
    pub fn delete_all_metadata(&mut self, vector_id: u32) -> Result<bool, JsError> {
        self.inner
            .metadata
            .delete_all(vector_id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Checks if a metadata key exists for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @param key - The metadata key to check
    /// @returns true if the key exists
    #[wasm_bindgen(js_name = "hasMetadata")]
    pub fn has_metadata(&self, vector_id: u32, key: &str) -> bool {
        self.inner.metadata.has_key(vector_id, key)
    }

    /// Returns the number of metadata keys for a vector.
    ///
    /// @param vectorId - The vector ID
    /// @returns The number of metadata keys
    #[wasm_bindgen(js_name = "metadataKeyCount")]
    pub fn metadata_key_count(&self, vector_id: u32) -> usize {
        self.inner.metadata.key_count(vector_id)
    }
}
```

#### 2. TypeScript Type Definitions

**File: `pkg/edgevec.d.ts` (additions)**
```typescript
/**
 * Supported metadata value types.
 */
export type MetadataValueType = 'string' | 'integer' | 'float' | 'boolean' | 'string_array';

/**
 * Represents a metadata value that can be attached to vectors.
 */
export class JsMetadataValue {
  /**
   * Creates a string metadata value.
   */
  static fromString(value: string): JsMetadataValue;

  /**
   * Creates an integer metadata value.
   */
  static fromInteger(value: number): JsMetadataValue;

  /**
   * Creates a float metadata value.
   */
  static fromFloat(value: number): JsMetadataValue;

  /**
   * Creates a boolean metadata value.
   */
  static fromBoolean(value: boolean): JsMetadataValue;

  /**
   * Creates a string array metadata value.
   */
  static fromStringArray(value: string[]): JsMetadataValue;

  /**
   * Returns the type of this value.
   */
  getType(): MetadataValueType;

  /**
   * Gets the value as a string (returns undefined if not a string).
   */
  asString(): string | undefined;

  /**
   * Gets the value as an integer (returns undefined if not an integer).
   */
  asInteger(): number | undefined;

  /**
   * Gets the value as a float (returns undefined if not a float).
   */
  asFloat(): number | undefined;

  /**
   * Gets the value as a boolean (returns undefined if not a boolean).
   */
  asBoolean(): boolean | undefined;

  /**
   * Gets the value as a string array (returns undefined if not an array).
   */
  asStringArray(): string[] | undefined;

  /**
   * Converts to a JavaScript-native value.
   */
  toJS(): string | number | boolean | string[];
}

/**
 * Metadata operations on HnswIndex.
 */
export interface HnswIndexMetadata {
  /**
   * Sets metadata for a vector.
   * @throws Error if validation fails or key limit exceeded
   */
  setMetadata(vectorId: number, key: string, value: JsMetadataValue): void;

  /**
   * Gets metadata for a vector.
   * @returns The metadata value, or undefined if not found
   */
  getMetadata(vectorId: number, key: string): JsMetadataValue | undefined;

  /**
   * Gets all metadata for a vector.
   * @returns An object mapping keys to values, or undefined if no metadata
   */
  getAllMetadata(vectorId: number): Record<string, string | number | boolean | string[]> | undefined;

  /**
   * Deletes metadata for a vector.
   * @returns true if the key existed and was deleted
   */
  deleteMetadata(vectorId: number, key: string): boolean;

  /**
   * Deletes all metadata for a vector.
   * @returns true if the vector had metadata that was deleted
   */
  deleteAllMetadata(vectorId: number): boolean;

  /**
   * Checks if a metadata key exists for a vector.
   */
  hasMetadata(vectorId: number, key: string): boolean;

  /**
   * Returns the number of metadata keys for a vector.
   */
  metadataKeyCount(vectorId: number): number;
}
```

#### 3. JavaScript Usage Example

**File: `examples/metadata_example.js`**
```javascript
import init, { HnswIndex, JsMetadataValue } from 'edgevec';

async function main() {
  await init();

  // Create index
  const index = new HnswIndex(128);

  // Insert a vector
  const vector = new Float32Array(128).fill(0.5);
  const vectorId = index.insert(vector);

  // Attach metadata
  index.setMetadata(vectorId, 'title', JsMetadataValue.fromString('My Document'));
  index.setMetadata(vectorId, 'page_count', JsMetadataValue.fromInteger(42));
  index.setMetadata(vectorId, 'relevance', JsMetadataValue.fromFloat(0.95));
  index.setMetadata(vectorId, 'is_verified', JsMetadataValue.fromBoolean(true));
  index.setMetadata(vectorId, 'tags', JsMetadataValue.fromStringArray(['rust', 'wasm']));

  // Retrieve metadata
  const title = index.getMetadata(vectorId, 'title');
  console.log('Title:', title?.asString());

  // Get all metadata as JS object
  const allMetadata = index.getAllMetadata(vectorId);
  console.log('All metadata:', allMetadata);
  // Output: { title: 'My Document', page_count: 42, relevance: 0.95, ... }

  // Check existence
  console.log('Has title:', index.hasMetadata(vectorId, 'title'));
  console.log('Key count:', index.metadataKeyCount(vectorId));

  // Delete metadata
  index.deleteMetadata(vectorId, 'page_count');
  index.deleteAllMetadata(vectorId);
}

main();
```

---

### Acceptance Criteria

**CRITICAL (Must Pass):**
- [ ] `JsMetadataValue` class exported with all 5 factory methods
- [ ] `HnswIndex.setMetadata()` works from JavaScript
- [ ] `HnswIndex.getMetadata()` returns correct values
- [ ] `HnswIndex.getAllMetadata()` returns JavaScript object
- [ ] `HnswIndex.deleteMetadata()` removes keys
- [ ] `HnswIndex.hasMetadata()` returns correct boolean
- [ ] `wasm-pack build` succeeds with exit code 0
- [ ] Bundle size < 500KB gzipped (no regression)

**MAJOR (Should Pass):**
- [ ] TypeScript types generated correctly
- [ ] `toJS()` returns JavaScript-native types
- [ ] Error messages propagate to JavaScript
- [ ] Node.js tests pass
- [ ] Browser tests pass (basic sanity check)

**MINOR (Nice to Have):**
- [ ] `examples/metadata_example.js` included
- [ ] API documented in README
- [ ] Performance benchmarks for WASM calls

---

### Implementation Checklist

- [ ] Create `src/wasm/metadata.rs` with `JsMetadataValue`
- [ ] Add metadata methods to `HnswIndexWasm`
- [ ] Update `src/wasm/mod.rs` to include metadata module
- [ ] Run `wasm-pack build --target web`
- [ ] Verify TypeScript definitions in `pkg/edgevec.d.ts`
- [ ] Write Node.js tests for WASM bindings
- [ ] Verify bundle size (measure with `gzip -c pkg/edgevec_bg.wasm | wc -c`)
- [ ] Create usage example
- [ ] Test in browser (manual or automated)

---

### Test Requirements

**Node.js Tests (Required):**
```javascript
// tests/wasm_metadata.test.js
const { HnswIndex, JsMetadataValue } = require('../pkg/edgevec.js');

describe('Metadata WASM Bindings', () => {
  let index;

  beforeEach(() => {
    index = new HnswIndex(4);
    const vector = new Float32Array([1, 0, 0, 0]);
    index.insert(vector);
  });

  test('setMetadata and getMetadata work for string', () => {
    index.setMetadata(0, 'title', JsMetadataValue.fromString('Test'));
    const value = index.getMetadata(0, 'title');
    expect(value.asString()).toBe('Test');
  });

  test('setMetadata and getMetadata work for integer', () => {
    index.setMetadata(0, 'count', JsMetadataValue.fromInteger(42));
    const value = index.getMetadata(0, 'count');
    expect(value.asInteger()).toBe(42);
  });

  test('setMetadata and getMetadata work for float', () => {
    index.setMetadata(0, 'score', JsMetadataValue.fromFloat(3.14));
    const value = index.getMetadata(0, 'score');
    expect(value.asFloat()).toBeCloseTo(3.14);
  });

  test('setMetadata and getMetadata work for boolean', () => {
    index.setMetadata(0, 'verified', JsMetadataValue.fromBoolean(true));
    const value = index.getMetadata(0, 'verified');
    expect(value.asBoolean()).toBe(true);
  });

  test('setMetadata and getMetadata work for string array', () => {
    index.setMetadata(0, 'tags', JsMetadataValue.fromStringArray(['a', 'b']));
    const value = index.getMetadata(0, 'tags');
    expect(value.asStringArray()).toEqual(['a', 'b']);
  });

  test('getAllMetadata returns JS object', () => {
    index.setMetadata(0, 'title', JsMetadataValue.fromString('Test'));
    index.setMetadata(0, 'count', JsMetadataValue.fromInteger(42));
    const all = index.getAllMetadata(0);
    expect(all.title).toBe('Test');
    expect(all.count).toBe(42);
  });

  test('deleteMetadata removes key', () => {
    index.setMetadata(0, 'title', JsMetadataValue.fromString('Test'));
    expect(index.deleteMetadata(0, 'title')).toBe(true);
    expect(index.hasMetadata(0, 'title')).toBe(false);
  });

  test('metadataKeyCount returns correct count', () => {
    index.setMetadata(0, 'a', JsMetadataValue.fromInteger(1));
    index.setMetadata(0, 'b', JsMetadataValue.fromInteger(2));
    expect(index.metadataKeyCount(0)).toBe(2);
  });

  test('invalid key throws error', () => {
    expect(() => {
      index.setMetadata(0, '', JsMetadataValue.fromString('value'));
    }).toThrow();
  });
});
```

**Coverage Target:** 100% of WASM binding methods tested

---

### Performance Targets

| Operation | Target | Notes |
|:----------|:-------|:------|
| WASM `setMetadata` | <10µs | Single call |
| WASM `getMetadata` | <5µs | Single call |
| WASM `getAllMetadata` | <50µs | 10 keys |
| Bundle size (gzipped) | <500KB | No regression |

---

### Bundle Size Verification

```bash
# Build WASM bundle
wasm-pack build --target web --release

# Measure sizes
wc -c pkg/edgevec_bg.wasm                    # Uncompressed
gzip -c pkg/edgevec_bg.wasm | wc -c          # Gzipped

# Compare to baseline
# v0.4.0: 227,037 bytes uncompressed, 93,579 bytes gzipped
# Target: No more than 10% increase
```

---

### Documentation Requirements

- [ ] JSDoc comments on all exported functions
- [ ] TypeScript interface documentation
- [ ] Usage example in README or examples/
- [ ] Error handling documentation

---

### Dependencies

**Blocks:**
- W21.4 (Mobile testing needs WASM bindings)
- W21.5 (CI needs everything working)

**Blocked By:**
- W21.2 complete (CRUD must work in Rust first)

**External Dependencies:**
- `wasm-bindgen` (already in Cargo.toml)
- `js-sys` (already in Cargo.toml)
- `wasm-pack` (build tool)

---

### Verification Method

**Day 3 is COMPLETE when:**

1. Run verification commands:
   ```bash
   wasm-pack build --target web --release
   wasm-pack test --node
   gzip -c pkg/edgevec_bg.wasm | wc -c  # Must be < 500KB
   ```

2. All commands exit with code 0

3. Node.js tests pass:
   ```bash
   npm test tests/wasm_metadata.test.js
   ```

4. TypeScript types are valid:
   ```bash
   npx tsc --noEmit pkg/edgevec.d.ts
   ```

---

### Rollback Plan

If Day 3 encounters blocking issues:

1. **wasm-bindgen issues:** Simplify API (fewer methods)
2. **Bundle size regression:** Defer `StringArray` support
3. **TypeScript issues:** Manual type definitions
4. **Performance issues:** Profile and optimize hot paths

---

### Estimated Timeline

| Phase | Time | Cumulative |
|:------|:-----|:-----------|
| JsMetadataValue implementation | 2h | 2h |
| HnswIndex bindings | 2h | 4h |
| TypeScript definitions | 1h | 5h |
| Node.js tests | 1.5h | 6.5h |
| Bundle size verification | 0.5h | 7h |
| Documentation | 0.5h | 7.5h |
| Buffer | 0.5h | 8h |

---

### Hostile Review Checkpoint

**End of Day 3:** Submit for `/review` with:
- `src/wasm/metadata.rs`
- Updated `pkg/edgevec.d.ts`
- Node.js test file
- Bundle size report (before/after)

**Expected Review Focus:**
- API ergonomics for JavaScript developers
- Type safety in TypeScript definitions
- Bundle size impact
- Error handling propagation

---

**Task Owner:** WASM_SPECIALIST
**Review Required:** HOSTILE_REVIEWER
**Next Task:** W21.4 (Mobile Browser Testing)

---

*"Type-safe in Rust. Ergonomic in JavaScript. Fast everywhere."*
