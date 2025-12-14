# Unsafe Block Audit: Persistence Module

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-13
**Scope:** `src/persistence/*.rs`
**Task:** W13.1a
**Status:** COMPLETE

---

## Executive Summary

The persistence module contains **2 unsafe blocks** that perform pointer casts between byte slices and `HnswNode` structs. Both blocks are classified as **UNSOUND** due to missing alignment verification.

| File | Line | Operation | Classification | Risk Level |
|:-----|:-----|:----------|:---------------|:-----------|
| `snapshot.rs` | 236-239 | `&[u8]` → `&[HnswNode]` | **UNSOUND** | HIGH |
| `chunking.rs` | 227-232 | `&[HnswNode]` → `&[u8]` | **POTENTIALLY_UNSOUND** | MEDIUM |

**Recommendation:** Replace both unsafe blocks with `bytemuck::cast_slice` / `bytemuck::try_cast_slice` in W13.2.

---

## Detailed Analysis

### Block 1: `snapshot.rs:236-239` (UNSOUND)

**Location:** `src/persistence/snapshot.rs` lines 236-239

**Code:**
```rust
// SAFETY: HnswNode is repr(C) and properly aligned. The bytes came from
// a valid serialization of HnswNode structs. We use `_mm_loadu_*` style
// operations which handle alignment, and the original data was aligned.
#[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
let nodes: &[HnswNode] = unsafe {
    let ptr = nodes_bytes.as_ptr() as *const HnswNode;
    std::slice::from_raw_parts(ptr, vec_count)
};
```

**Operation:** Deserialize bytes read from storage backend into `&[HnswNode]` slice.

**Invariants Required:**
1. `nodes_bytes.as_ptr()` must be aligned to `align_of::<HnswNode>()` (8 bytes, due to `VectorId(u64)`)
2. `nodes_bytes.len() >= vec_count * size_of::<HnswNode>()` (16 bytes per node)
3. Bytes must represent valid `HnswNode` structs (no invalid enum variants, etc.)

**Invariant Verification:**

| Invariant | Verified? | Evidence |
|:----------|:----------|:---------|
| Alignment (8 bytes) | **NO** | `nodes_bytes` comes from `data[index_offset_local..]` which is arbitrary byte slice from storage |
| Size requirement | PARTIAL | Length check exists at line 205-209, but doesn't use `size_of::<HnswNode>()` |
| Valid representation | **NO** | No validation that bytes represent valid structs |

**Why This Is UB:**

1. **Alignment Violation:** The `nodes_bytes` slice originates from a `Vec<u8>` read from the storage backend. While `Vec<u8>` is heap-allocated, its pointer has alignment 1 (byte alignment). When the slice is taken at an arbitrary offset (`index_offset_local`), there is NO guarantee it aligns to 8 bytes.

2. **Clippy Suppression:** The `#[allow(clippy::cast_ptr_alignment)]` explicitly suppresses the warning Clippy raises about this exact issue. This is a **red flag**.

3. **SAFETY Comment Is Incorrect:** The comment claims "the original data was aligned" but this is false. The data comes from arbitrary I/O which does not guarantee alignment.

**Architecture Impact:**
- **x86_64:** May appear to work (x86_64 is permissive about unaligned access)
- **ARM:** Will cause SIGBUS or incorrect behavior
- **WASM:** Behavior depends on engine; Chrome is permissive, strict WASM mode faults

**Classification:** **UNSOUND** - This code has undefined behavior when alignment is not met.

**Risk Level:** **HIGH** - Can cause crashes or silent data corruption.

**Mitigation:** Replace with `bytemuck::try_cast_slice()` which verifies alignment at runtime and returns `Err` on failure.

---

### Block 2: `chunking.rs:227-232` (POTENTIALLY_UNSOUND)

**Location:** `src/persistence/chunking.rs` lines 227-232

**Code:**
```rust
// SAFETY: HnswNode is #[repr(C)] with fixed layout 16 bytes.
// It contains simple primitives (u64, u32, u16, u8).
unsafe {
    let ptr = slice.as_ptr().cast::<u8>();
    let len = nodes_to_copy * 16; // 16 bytes per node
    let byte_slice = std::slice::from_raw_parts(ptr, len);
    self.buffer.extend_from_slice(byte_slice);
}
```

**Operation:** Serialize `&[HnswNode]` slice to bytes for storage.

**Invariants Required:**
1. `slice` must be a valid `&[HnswNode]` reference
2. `size_of::<HnswNode>()` must equal 16 bytes
3. No padding bytes contain uninitialized memory

**Invariant Verification:**

| Invariant | Verified? | Evidence |
|:----------|:----------|:---------|
| Valid reference | YES | `slice` comes from `&self.index.nodes[..]` |
| Size = 16 bytes | PARTIAL | Hardcoded as 16, but should use `size_of::<HnswNode>()` |
| Padding initialized | **UNCERTAIN** | `HnswNode` has `pad: u8` field which may not be initialized |

**Why This Is Potentially Unsound:**

1. **Hardcoded Size:** Uses `16` instead of `std::mem::size_of::<HnswNode>()`. If the struct layout changes, this becomes incorrect.

2. **Padding Bytes:** The `HnswNode` struct has a `pad: u8` field. Reading uninitialized padding is technically UB (though practically benign on most platforms). The struct SHOULD be zero-initialized when created.

3. **Direction of Cast:** `&[HnswNode]` → `&[u8]` is safer than the reverse because:
   - The source is known to be properly aligned (it's a Rust slice)
   - `u8` has alignment 1, so any pointer is valid
   - However, reading padding bytes is still technically UB

**Classification:** **POTENTIALLY_UNSOUND** - Safer than Block 1, but still has issues.

**Risk Level:** **MEDIUM** - Less likely to cause problems, but technically incorrect.

**Mitigation:** Replace with `bytemuck::cast_slice()` after deriving `Pod` for `HnswNode`. This requires ensuring the struct has no padding or that padding is always zero-initialized.

---

## HnswNode Struct Analysis

**Location:** `src/hnsw/graph.rs` lines 124-140

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct HnswNode {
    pub vector_id: VectorId,     // u64, 8 bytes, offset 0
    pub neighbor_offset: u32,    // u32, 4 bytes, offset 8
    pub neighbor_len: u16,       // u16, 2 bytes, offset 12
    pub max_layer: u8,           // u8, 1 byte, offset 14
    pub pad: u8,                 // u8, 1 byte, offset 15
}
```

**Layout Analysis:**

| Field | Type | Size | Offset | Alignment |
|:------|:-----|:-----|:-------|:----------|
| `vector_id` | `VectorId(u64)` | 8 bytes | 0 | 8 |
| `neighbor_offset` | `u32` | 4 bytes | 8 | 4 |
| `neighbor_len` | `u16` | 2 bytes | 12 | 2 |
| `max_layer` | `u8` | 1 byte | 14 | 1 |
| `pad` | `u8` | 1 byte | 15 | 1 |
| **Total** | | **16 bytes** | | **8** |

**Struct Requirements for `Pod`:**

1. **`#[repr(C)]`** ✅ - Already present
2. **No implicit padding** ✅ - Layout is exactly 16 bytes with explicit `pad` field
3. **All fields are `Pod`** ✅ - `u64`, `u32`, `u16`, `u8` are all `Pod`
4. **No interior mutability** ✅ - No `Cell`, `RefCell`, etc.
5. **`Copy`** ❌ - Missing `Copy` derive (required for `Pod`)

**Pod Compatibility:** `HnswNode` is **almost** `Pod`-compatible. It only needs `#[derive(Copy)]` added.

---

## Files Without Unsafe Blocks

The following persistence module files were audited and found to contain **no unsafe blocks**:

| File | Status |
|:-----|:-------|
| `src/persistence/mod.rs` | ✅ Safe |
| `src/persistence/header.rs` | ✅ Safe |
| `src/persistence/entry.rs` | ✅ Safe |
| `src/persistence/reader.rs` | ✅ Safe |
| `src/persistence/writer.rs` | ✅ Safe |
| `src/persistence/wal.rs` | ✅ Safe |
| `src/persistence/storage.rs` | ✅ Safe |

---

## Recommendations

### Immediate Actions (W13.2)

1. **Add bytemuck dependency:**
   ```toml
   bytemuck = { version = "1.14", features = ["derive"] }
   ```

2. **Derive `Pod` and `Zeroable` for `HnswNode`:**
   ```rust
   #[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
   #[repr(C)]
   pub struct HnswNode { ... }
   ```

3. **Replace `snapshot.rs:224-227`:**
   ```rust
   // BEFORE (unsafe):
   #[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
   let nodes: &[HnswNode] = unsafe {
       let ptr = nodes_bytes.as_ptr() as *const HnswNode;
       std::slice::from_raw_parts(ptr, vec_count)
   };

   // AFTER (safe):
   let nodes: &[HnswNode] = bytemuck::try_cast_slice(nodes_bytes)
       .map_err(|e| PersistenceError::Alignment {
           source: format!("{:?}", e),
           context: "HnswNode deserialization".to_string(),
       })?;
   ```

4. **Replace `chunking.rs:216-221`:**
   ```rust
   // BEFORE (unsafe):
   unsafe {
       let ptr = slice.as_ptr().cast::<u8>();
       let len = nodes_to_copy * 16;
       let byte_slice = std::slice::from_raw_parts(ptr, len);
       self.buffer.extend_from_slice(byte_slice);
   }

   // AFTER (safe):
   let byte_slice: &[u8] = bytemuck::cast_slice(slice);
   self.buffer.extend_from_slice(byte_slice);
   ```

5. **Remove Clippy suppressions:**
   - Delete `#[allow(clippy::cast_ptr_alignment)]`
   - Delete `#[allow(clippy::ptr_as_ptr)]`

### Testing Requirements

1. **Alignment test:** Create test with intentionally misaligned data to verify `try_cast_slice` returns error
2. **Roundtrip test:** Verify serialize → deserialize preserves data
3. **Property test:** Use proptest to verify arbitrary `HnswNode` values roundtrip correctly

---

## Verification Commands

```bash
# Count unsafe blocks (should be 2 before fix, 0 after)
grep -rn "unsafe" src/persistence/

# Find Clippy suppressions (should be 0 after fix)
grep -rn "cast_ptr_alignment" src/persistence/

# Build and test
cargo build
cargo test --lib
cargo clippy -- -D warnings
```

---

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|:----------|:-------|:---------|
| AC13.1a.1: All unsafe blocks documented | ✅ | 2 blocks listed above |
| AC13.1a.2: snapshot.rs issue documented | ✅ | Block 1 analysis |
| AC13.1a.3: chunking.rs issue documented | ✅ | Block 2 analysis |
| AC13.1a.4: SAFETY comments added | ⏳ | Pending source updates |

---

## Audit Conclusion

The persistence module contains **2 unsafe blocks** that violate Rust's safety guarantees:

1. **`snapshot.rs:236-239`** - UNSOUND (alignment not verified on deserialization)
2. **`chunking.rs:227-232`** - POTENTIALLY_UNSOUND (padding/size assumptions)

Both issues will be resolved in W13.2 by integrating `bytemuck` as specified in RFC-001.

---

**Auditor:** RUST_ENGINEER
**Date:** 2025-12-13
**Status:** AUDIT COMPLETE — Ready for W13.2 Implementation
