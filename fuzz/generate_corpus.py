#!/usr/bin/env python3
"""
Corpus Seed Generator for EdgeVec HNSW Fuzz Targets

This script generates initial corpus seeds for fuzzing. Each seed is designed
to exercise specific code paths and edge cases.

Input Formats:
- hnsw_insert: [cmd: u8][vector: 4 x f32 (16 bytes)] repeated
- hnsw_search: arbitrary::Unstructured data (random bytes)
- graph_ops: arbitrary::Arbitrary derive for Vec<Op>
- search_robustness: [entry_point: u32 (4 bytes)][query: N x f32]
"""

import struct
import os
from pathlib import Path

BASE_DIR = Path(__file__).parent / "corpus"


def write_seed(target: str, name: str, data: bytes):
    """Write a seed file to the target's corpus directory."""
    target_dir = BASE_DIR / target
    target_dir.mkdir(parents=True, exist_ok=True)
    seed_path = target_dir / name
    seed_path.write_bytes(data)
    print(f"  Created: {seed_path.name} ({len(data)} bytes)")


def f32_bytes(value: float) -> bytes:
    """Convert float to little-endian bytes."""
    return struct.pack("<f", value)


def u32_bytes(value: int) -> bytes:
    """Convert u32 to little-endian bytes."""
    return struct.pack("<I", value)


def u8_bytes(value: int) -> bytes:
    """Convert u8 to bytes."""
    return struct.pack("<B", value)


# =============================================================================
# hnsw_insert seeds
# Format: [cmd: u8][vector: 4 x f32] repeated
# cmd < 220 = INSERT (86%), cmd >= 220 = SEARCH (14%)
# =============================================================================

def generate_hnsw_insert_seeds():
    print("\nGenerating hnsw_insert seeds...")

    # Seed 1: Single insert - zero vector
    cmd_insert = u8_bytes(0)  # INSERT
    zero_vec = f32_bytes(0.0) * 4
    write_seed("hnsw_insert", "01_single_insert_zero", cmd_insert + zero_vec)

    # Seed 2: Single insert - unit vector
    unit_vec = f32_bytes(1.0) * 4
    write_seed("hnsw_insert", "02_single_insert_unit", cmd_insert + unit_vec)

    # Seed 3: Multiple inserts
    data = b""
    for i in range(5):
        data += cmd_insert
        data += f32_bytes(float(i)) * 4
    write_seed("hnsw_insert", "03_multiple_inserts", data)

    # Seed 4: Insert then search
    data = cmd_insert + unit_vec  # Insert
    cmd_search = u8_bytes(250)  # SEARCH (>= 220)
    data += cmd_search + zero_vec  # Search
    write_seed("hnsw_insert", "04_insert_then_search", data)

    # Seed 5: Many inserts (stress test)
    data = b""
    for i in range(20):
        data += cmd_insert
        data += f32_bytes(float(i * 0.1)) * 4
    write_seed("hnsw_insert", "05_many_inserts", data)

    # Seed 6: Alternating insert/search
    data = b""
    for i in range(5):
        data += cmd_insert + f32_bytes(float(i)) * 4
        data += cmd_search + f32_bytes(float(i)) * 4
    write_seed("hnsw_insert", "06_alternating_ops", data)

    # Seed 7: Edge case - large values
    large_vec = f32_bytes(1e30) + f32_bytes(-1e30) + f32_bytes(1e-30) + f32_bytes(-1e-30)
    write_seed("hnsw_insert", "07_large_values", cmd_insert + large_vec)

    # Seed 8: Edge case - all same value
    same_vec = f32_bytes(0.5) * 4
    write_seed("hnsw_insert", "08_same_values", cmd_insert + same_vec)

    # Seed 9: Negative values
    neg_vec = f32_bytes(-1.0) + f32_bytes(-2.0) + f32_bytes(-3.0) + f32_bytes(-4.0)
    write_seed("hnsw_insert", "09_negative_values", cmd_insert + neg_vec)

    # Seed 10: Mixed positive/negative
    mixed_vec = f32_bytes(1.0) + f32_bytes(-1.0) + f32_bytes(2.0) + f32_bytes(-2.0)
    write_seed("hnsw_insert", "10_mixed_signs", cmd_insert + mixed_vec)

    # Seed 11: Search on empty graph (before any inserts)
    write_seed("hnsw_insert", "11_search_empty", cmd_search + zero_vec)

    # Seed 12: Dense cluster - many similar vectors
    data = b""
    for i in range(10):
        data += cmd_insert
        data += f32_bytes(1.0 + i * 0.01) * 4
    write_seed("hnsw_insert", "12_dense_cluster", data)


# =============================================================================
# hnsw_search seeds
# Uses arbitrary::Unstructured - random bytes that get parsed
# The target parses: dimensions, node_count, vectors, links, query, entry_points, ef
# =============================================================================

def generate_hnsw_search_seeds():
    print("\nGenerating hnsw_search seeds...")

    # These are raw bytes that arbitrary::Unstructured will parse
    # The exact interpretation depends on the fuzzer's parsing logic

    # Seed 1: Minimal valid input (small)
    write_seed("hnsw_search", "01_minimal", bytes(range(64)))

    # Seed 2: Medium size random-like
    data = bytes([(i * 17 + 31) % 256 for i in range(256)])
    write_seed("hnsw_search", "02_medium", data)

    # Seed 3: Larger input for more nodes
    data = bytes([(i * 23 + 47) % 256 for i in range(512)])
    write_seed("hnsw_search", "03_large", data)

    # Seed 4: All zeros (edge case)
    write_seed("hnsw_search", "04_zeros", bytes(128))

    # Seed 5: All 0xFF (edge case)
    write_seed("hnsw_search", "05_all_ff", bytes([0xFF] * 128))

    # Seed 6: Alternating pattern
    write_seed("hnsw_search", "06_alternating", bytes([0x55, 0xAA] * 64))

    # Seed 7: Incrementing bytes
    write_seed("hnsw_search", "07_incrementing", bytes(range(256)))

    # Seed 8: Decrementing bytes
    write_seed("hnsw_search", "08_decrementing", bytes(range(255, -1, -1)))

    # Seed 9: Small values (likely valid dimensions 2-16)
    data = bytes([4, 10] + [0] * 200)  # dim=4, nodes=10
    write_seed("hnsw_search", "09_small_config", data)

    # Seed 10: Structured to encourage specific paths
    # First few bytes control config, rest is vector/link data
    data = bytes([8] + [20] + list(range(100, 200)))
    write_seed("hnsw_search", "10_structured", data)

    # Seed 11: Prime-offset pattern (good for coverage)
    data = bytes([(i * 7 + 11) % 256 for i in range(384)])
    write_seed("hnsw_search", "11_prime_offset", data)

    # Seed 12: Binary counting
    data = bytes([i % 256 for i in range(1024)])
    write_seed("hnsw_search", "12_binary_count", data)


# =============================================================================
# graph_ops seeds
# Uses arbitrary::Arbitrary derive for Vec<Op>
# Op enum: Insert { vector: Vec<f32> }, Delete { id: u64 }, Search { vector, k }, SaveLoad
# =============================================================================

def generate_graph_ops_seeds():
    print("\nGenerating graph_ops seeds...")

    # arbitrary parses these bytes into Vec<Op>
    # The exact mapping depends on the derive implementation

    # Seed 1: Minimal
    write_seed("graph_ops", "01_minimal", bytes(32))

    # Seed 2: Insert-heavy pattern
    data = bytes([0] * 100)  # Op::Insert is likely variant 0
    write_seed("graph_ops", "02_insert_heavy", data)

    # Seed 3: Delete-heavy pattern
    data = bytes([1] * 100)  # Op::Delete is likely variant 1
    write_seed("graph_ops", "03_delete_heavy", data)

    # Seed 4: Search-heavy pattern
    data = bytes([2] * 100)  # Op::Search is likely variant 2
    write_seed("graph_ops", "04_search_heavy", data)

    # Seed 5: SaveLoad-heavy pattern
    data = bytes([3] * 50)  # Op::SaveLoad is likely variant 3
    write_seed("graph_ops", "05_saveload_heavy", data)

    # Seed 6: Mixed operations
    data = bytes([0, 0, 0, 2, 0, 0, 1, 0, 3, 0, 2] * 10)
    write_seed("graph_ops", "06_mixed_ops", data)

    # Seed 7: Empty ops list (short input)
    write_seed("graph_ops", "07_empty", bytes(4))

    # Seed 8: Large operations sequence
    data = bytes([(i % 4) for i in range(500)])
    write_seed("graph_ops", "08_long_sequence", data)

    # Seed 9: Incrementing (exercises various vector data)
    write_seed("graph_ops", "09_incrementing", bytes(range(256)))

    # Seed 10: With float-like data for vectors
    # Each insert needs a Vec<f32>, so include 4-byte aligned data
    data = f32_bytes(1.0) + f32_bytes(2.0) + f32_bytes(3.0) + f32_bytes(4.0)
    write_seed("graph_ops", "10_float_data", bytes([0]) + data * 5)

    # Seed 11: Delete with various IDs
    data = bytes([1]) + u32_bytes(0) + bytes([1]) + u32_bytes(1) + bytes([1]) + u32_bytes(0xFFFFFFFF)
    write_seed("graph_ops", "11_delete_ids", data * 3)

    # Seed 12: Search with k values
    data = bytes([2]) + f32_bytes(0.0) * 4 + bytes([10])  # k=10
    write_seed("graph_ops", "12_search_k", data * 5)


# =============================================================================
# search_robustness seeds
# Format: [entry_point: u32 (4 bytes)][query: N x f32]
# Tests random entry points against a fixed small graph
# =============================================================================

def generate_search_robustness_seeds():
    print("\nGenerating search_robustness seeds...")

    # Seed 1: Valid entry point (NodeId 0), simple query
    entry = u32_bytes(0)
    query = f32_bytes(0.5) * 4  # 4D query
    write_seed("search_robustness", "01_valid_entry", entry + query)

    # Seed 2: Entry point 1
    entry = u32_bytes(1)
    write_seed("search_robustness", "02_entry_1", entry + query)

    # Seed 3: Entry point 2
    entry = u32_bytes(2)
    write_seed("search_robustness", "03_entry_2", entry + query)

    # Seed 4: Entry point 3
    entry = u32_bytes(3)
    write_seed("search_robustness", "04_entry_3", entry + query)

    # Seed 5: Invalid entry point (very large)
    entry = u32_bytes(0xFFFFFFFF)
    write_seed("search_robustness", "05_invalid_large", entry + query)

    # Seed 6: Invalid entry point (100)
    entry = u32_bytes(100)
    write_seed("search_robustness", "06_invalid_100", entry + query)

    # Seed 7: Zero query vector
    entry = u32_bytes(0)
    zero_query = f32_bytes(0.0) * 4
    write_seed("search_robustness", "07_zero_query", entry + zero_query)

    # Seed 8: Unit query vector
    unit_query = f32_bytes(1.0) * 4
    write_seed("search_robustness", "08_unit_query", entry + unit_query)

    # Seed 9: Large dimension query (8D)
    large_query = f32_bytes(0.5) * 8
    write_seed("search_robustness", "09_8d_query", entry + large_query)

    # Seed 10: Single float query (1D)
    single_query = f32_bytes(0.5)
    write_seed("search_robustness", "10_1d_query", entry + single_query)

    # Seed 11: Mixed negative query
    mixed_query = f32_bytes(-1.0) + f32_bytes(1.0) + f32_bytes(-0.5) + f32_bytes(0.5)
    write_seed("search_robustness", "11_mixed_query", entry + mixed_query)

    # Seed 12: Exact match to vector 0 ([0,0,0,0])
    exact_query = f32_bytes(0.0) * 4
    write_seed("search_robustness", "12_exact_match", entry + exact_query)

    # Seed 13: Boundary entry point values
    for i, ep in enumerate([0, 1, 2, 3, 4, 5, 10, 255]):
        entry = u32_bytes(ep)
        write_seed("search_robustness", f"13_boundary_{i}", entry + query)


def main():
    print("EdgeVec Corpus Seed Generator")
    print("=" * 50)

    generate_hnsw_insert_seeds()
    generate_hnsw_search_seeds()
    generate_graph_ops_seeds()
    generate_search_robustness_seeds()

    print("\n" + "=" * 50)
    print("Corpus generation complete!")

    # Count total seeds
    total = 0
    for target in ["hnsw_insert", "hnsw_search", "graph_ops", "search_robustness"]:
        target_dir = BASE_DIR / target
        if target_dir.exists():
            count = len(list(target_dir.glob("*")))
            total += count
            print(f"  {target}: {count} seeds")
    print(f"  Total: {total} seeds")


if __name__ == "__main__":
    main()
