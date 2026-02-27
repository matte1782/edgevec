#![no_main]
//! Fuzz target for SparseVector::new().
//!
//! Feeds arbitrary indices, values, and dimension to SparseVector::new().
//! Invariant: MUST return Ok or Err, NEVER panic.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // We need at least 4 bytes for dim + 1 element (4 bytes index + 4 bytes value) = 12 bytes
    if data.len() < 12 {
        return;
    }

    // Parse dimension from first 4 bytes
    let dim = u32::from_le_bytes(data[0..4].try_into().unwrap());

    // Avoid unreasonably large dimensions that would just waste time
    if dim == 0 || dim > 1_000_000 {
        return;
    }

    let remaining = &data[4..];

    // Each element needs 4 bytes (u32 index) + 4 bytes (f32 value) = 8 bytes
    let num_elements = remaining.len() / 8;
    if num_elements == 0 {
        return;
    }

    let mut indices = Vec::with_capacity(num_elements);
    let mut values = Vec::with_capacity(num_elements);

    for i in 0..num_elements {
        let offset = i * 8;
        if offset + 8 > remaining.len() {
            break;
        }
        let idx = u32::from_le_bytes(remaining[offset..offset + 4].try_into().unwrap());
        let val = f32::from_le_bytes(remaining[offset + 4..offset + 8].try_into().unwrap());
        indices.push(idx);
        values.push(val);
    }

    // This must never panic, only return Ok or Err
    let result = edgevec::sparse::SparseVector::new(indices.clone(), values.clone(), dim);

    // If construction succeeded, exercise additional methods without panicking
    if let Ok(sv) = result {
        let _ = sv.nnz();
        let _ = sv.dim();
        let _ = sv.indices();
        let _ = sv.values();
        let _ = sv.norm();
        let _ = sv.normalize();
        let _ = sv.to_pairs();

        // Try dot product with itself
        let _ = sv.dot(&sv);
        let _ = sv.cosine(&sv);

        // Try get on first index
        if let Some(&first_idx) = sv.indices().first() {
            let _ = sv.get(first_idx);
        }
    }

    // Also test from_pairs which sorts internally
    let pairs: Vec<(u32, f32)> = indices.into_iter().zip(values).collect();
    let _ = edgevec::sparse::SparseVector::from_pairs(&pairs, dim);
});
