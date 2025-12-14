//! Alignment Safety Tests for bytemuck Integration
//!
//! These tests verify that our Pod/Zeroable implementations are correct
//! and that bytemuck catches alignment issues at runtime.
//!
//! Created: W13.2 (bytemuck integration)
//! Purpose: Address Reddit community feedback on undefined behavior

use bytemuck::{Pod, Zeroable};
use edgevec::hnsw::graph::{HnswNode, VectorId};

/// Verifies at compile time that HnswNode implements Pod.
/// If this test compiles, Pod derive is correct.
#[test]
fn hnsw_node_is_pod() {
    fn assert_pod<T: Pod>() {}
    assert_pod::<HnswNode>();
}

/// Verifies at compile time that HnswNode implements Zeroable.
#[test]
fn hnsw_node_is_zeroable() {
    fn assert_zeroable<T: Zeroable>() {}
    assert_zeroable::<HnswNode>();
}

/// Verifies at compile time that VectorId implements Pod.
#[test]
fn vector_id_is_pod() {
    fn assert_pod<T: Pod>() {}
    assert_pod::<VectorId>();
}

/// Verifies at compile time that VectorId implements Zeroable.
#[test]
fn vector_id_is_zeroable() {
    fn assert_zeroable<T: Zeroable>() {}
    assert_zeroable::<VectorId>();
}

/// Tests roundtrip serialization of HnswNode via bytemuck.
#[test]
fn hnsw_node_roundtrip() {
    let original = HnswNode {
        vector_id: VectorId(42),
        neighbor_offset: 100,
        neighbor_len: 16,
        max_layer: 3,
        pad: 0,
    };

    // Serialize to bytes
    let bytes: &[u8] = bytemuck::bytes_of(&original);

    // Verify size matches expectation (16 bytes)
    assert_eq!(bytes.len(), std::mem::size_of::<HnswNode>());
    assert_eq!(bytes.len(), 16);

    // Deserialize back
    let recovered: &HnswNode = bytemuck::from_bytes(bytes);

    assert_eq!(original.vector_id, recovered.vector_id);
    assert_eq!(original.neighbor_offset, recovered.neighbor_offset);
    assert_eq!(original.neighbor_len, recovered.neighbor_len);
    assert_eq!(original.max_layer, recovered.max_layer);
    assert_eq!(original.pad, recovered.pad);
}

/// Tests roundtrip serialization of VectorId via bytemuck.
#[test]
fn vector_id_roundtrip() {
    let original = VectorId(0xDEAD_BEEF_CAFE_BABE);

    let bytes: &[u8] = bytemuck::bytes_of(&original);
    assert_eq!(bytes.len(), 8); // u64 = 8 bytes

    let recovered: &VectorId = bytemuck::from_bytes(bytes);
    assert_eq!(original, *recovered);
}

/// Tests slice roundtrip for multiple HnswNodes.
#[test]
fn hnsw_node_slice_roundtrip() {
    let nodes = vec![
        HnswNode {
            vector_id: VectorId(1),
            neighbor_offset: 0,
            neighbor_len: 8,
            max_layer: 0,
            pad: 0,
        },
        HnswNode {
            vector_id: VectorId(2),
            neighbor_offset: 8,
            neighbor_len: 16,
            max_layer: 1,
            pad: 0,
        },
        HnswNode {
            vector_id: VectorId(3),
            neighbor_offset: 24,
            neighbor_len: 24,
            max_layer: 2,
            pad: 0,
        },
    ];

    // Serialize slice to bytes
    let bytes: &[u8] = bytemuck::cast_slice(&nodes);
    assert_eq!(bytes.len(), 48); // 3 * 16 bytes

    // Deserialize back
    let recovered: &[HnswNode] = bytemuck::cast_slice(bytes);
    assert_eq!(recovered.len(), 3);

    for (orig, rec) in nodes.iter().zip(recovered.iter()) {
        assert_eq!(orig.vector_id, rec.vector_id);
        assert_eq!(orig.neighbor_offset, rec.neighbor_offset);
        assert_eq!(orig.neighbor_len, rec.neighbor_len);
        assert_eq!(orig.max_layer, rec.max_layer);
    }
}

/// Tests that try_cast_slice detects misaligned data.
/// This is the key safety check that prevents the UB we had before.
#[test]
fn try_cast_slice_detects_misalignment() {
    // Create a buffer larger than HnswNode so we can create misaligned slice
    let buffer = vec![0u8; std::mem::size_of::<HnswNode>() * 2 + 1];

    // Slice starting at offset 1 is likely misaligned for HnswNode (needs 8-byte alignment)
    let misaligned = &buffer[1..1 + std::mem::size_of::<HnswNode>()];

    let result: Result<&[HnswNode], _> = bytemuck::try_cast_slice(misaligned);

    // Should fail for misaligned data on most platforms
    // Note: This might succeed on some platforms with relaxed alignment, but the
    // important thing is bytemuck checks alignment instead of causing UB.
    if std::mem::align_of::<HnswNode>() > 1 {
        assert!(
            result.is_err(),
            "Expected alignment error for misaligned slice"
        );
    }
}

/// Tests that properly aligned data casts successfully.
#[test]
fn try_cast_slice_accepts_aligned() {
    // Create aligned nodes
    let nodes = vec![HnswNode {
        vector_id: VectorId(1),
        neighbor_offset: 0,
        neighbor_len: 0,
        max_layer: 0,
        pad: 0,
    }];

    // Cast to bytes
    let bytes: &[u8] = bytemuck::cast_slice(&nodes);

    // Cast back - should succeed because source was properly aligned
    let result: Result<&[HnswNode], _> = bytemuck::try_cast_slice(bytes);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

/// Verifies HnswNode has the expected size (16 bytes).
/// This is critical for binary compatibility.
#[test]
fn hnsw_node_size() {
    assert_eq!(std::mem::size_of::<HnswNode>(), 16);
}

/// Verifies HnswNode has the expected alignment (8 bytes due to VectorId containing u64).
#[test]
fn hnsw_node_alignment() {
    assert_eq!(std::mem::align_of::<HnswNode>(), 8);
}

/// Verifies VectorId has the expected size and alignment.
#[test]
fn vector_id_size_and_alignment() {
    assert_eq!(std::mem::size_of::<VectorId>(), 8);
    assert_eq!(std::mem::align_of::<VectorId>(), 8);
}

/// Tests Zeroable by creating zeroed instances.
#[test]
fn zeroable_creates_valid_zero() {
    let zeroed_node: HnswNode = bytemuck::Zeroable::zeroed();
    assert_eq!(zeroed_node.vector_id, VectorId(0));
    assert_eq!(zeroed_node.neighbor_offset, 0);
    assert_eq!(zeroed_node.neighbor_len, 0);
    assert_eq!(zeroed_node.max_layer, 0);
    assert_eq!(zeroed_node.pad, 0);

    let zeroed_id: VectorId = bytemuck::Zeroable::zeroed();
    assert_eq!(zeroed_id, VectorId(0));
}
