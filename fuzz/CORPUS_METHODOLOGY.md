# EdgeVec Fuzz Corpus Methodology

**Version:** 1.0.0
**Created:** 2025-12-13
**Author:** RUST_ENGINEER

---

## Overview

This document describes the methodology for generating initial corpus seeds for EdgeVec's HNSW fuzz targets. Well-crafted seed corpora significantly improve fuzzing effectiveness by:

1. Providing valid starting points that pass input validation
2. Covering known edge cases and boundary conditions
3. Exercising different code paths from the start
4. Reducing time-to-first-interesting-input

---

## Corpus Generation

Seeds are generated using `generate_corpus.py`, which creates binary files tailored to each fuzz target's input format.

### Running the Generator

```bash
cd fuzz/
python generate_corpus.py
```

### Regenerating Seeds

To regenerate all seeds:

```bash
rm -rf corpus/*/*.seed  # Clear existing seeds
python generate_corpus.py
```

---

## Target-Specific Input Formats

### 1. hnsw_insert

**Format:** Stream of operations
```
[cmd: u8][vector: 4 x f32 (16 bytes)] repeated
```

**Command Interpretation:**
- `cmd < 220` (86% probability): INSERT operation
- `cmd >= 220` (14% probability): SEARCH operation

**Seeds Generated (12):**

| Seed | Description | Edge Case |
|:-----|:------------|:----------|
| 01_single_insert_zero | Single zero vector | Empty/zero input |
| 02_single_insert_unit | Single unit vector | Normalized input |
| 03_multiple_inserts | 5 sequential inserts | Basic workflow |
| 04_insert_then_search | Insert + search | Operation mixing |
| 05_many_inserts | 20 inserts | Stress test |
| 06_alternating_ops | I/S/I/S pattern | Interleaved ops |
| 07_large_values | Extreme floats | Overflow bounds |
| 08_same_values | Identical components | Degenerate case |
| 09_negative_values | All negative | Sign handling |
| 10_mixed_signs | Pos/neg mix | Sign boundaries |
| 11_search_empty | Search before insert | Empty graph |
| 12_dense_cluster | Similar vectors | Clustering |

---

### 2. hnsw_search

**Format:** `arbitrary::Unstructured` raw bytes

The fuzzer uses `arbitrary` crate to parse structured data:
- Dimensions (2-16)
- Node count (5-50)
- Vectors for each node
- Neighbor links
- Query vector
- Entry points
- ef parameter

**Seeds Generated (12):**

| Seed | Description | Edge Case |
|:-----|:------------|:----------|
| 01_minimal | 64 bytes | Minimum viable |
| 02_medium | 256 bytes | Medium complexity |
| 03_large | 512 bytes | More nodes/links |
| 04_zeros | All zero bytes | Edge: zero parsing |
| 05_all_ff | All 0xFF | Edge: max values |
| 06_alternating | 0x55/0xAA pattern | Bit patterns |
| 07_incrementing | 0-255 sequence | Sequential |
| 08_decrementing | 255-0 sequence | Reverse |
| 09_small_config | Low dim/nodes | Simple graph |
| 10_structured | Config-focused | Valid config |
| 11_prime_offset | Prime multiplier | Coverage |
| 12_binary_count | 0-1023 pattern | Large input |

---

### 3. graph_ops

**Format:** `arbitrary::Arbitrary` derive for `Vec<Op>`

The `Op` enum:
```rust
enum Op {
    Insert { vector: Vec<f32> },  // variant 0
    Delete { id: u64 },           // variant 1
    Search { vector: Vec<f32>, k: u8 },  // variant 2
    SaveLoad,                     // variant 3
}
```

**Seeds Generated (12):**

| Seed | Description | Edge Case |
|:-----|:------------|:----------|
| 01_minimal | 32 bytes | Minimum |
| 02_insert_heavy | All zeros | Insert bias |
| 03_delete_heavy | All ones | Delete bias |
| 04_search_heavy | All twos | Search bias |
| 05_saveload_heavy | All threes | Persistence |
| 06_mixed_ops | Interleaved ops | Mixed workflow |
| 07_empty | 4 bytes | Empty ops |
| 08_long_sequence | 500 ops | Stress |
| 09_incrementing | Sequential | Varied ops |
| 10_float_data | f32 encoded | Vector data |
| 11_delete_ids | Various IDs | ID handling |
| 12_search_k | Search with k | k parameter |

---

### 4. search_robustness

**Format:** Raw binary
```
[entry_point: u32 (4 bytes, little-endian)]
[query: N x f32 (N * 4 bytes, little-endian)]
```

Tests `search_layer` robustness with random entry points against a fixed 4-node diamond graph.

**Seeds Generated (20):**

| Seed | Description | Edge Case |
|:-----|:------------|:----------|
| 01_valid_entry | NodeId 0 | Valid entry |
| 02_entry_1 | NodeId 1 | Valid entry |
| 03_entry_2 | NodeId 2 | Valid entry |
| 04_entry_3 | NodeId 3 | Valid entry |
| 05_invalid_large | NodeId 0xFFFFFFFF | Invalid: max |
| 06_invalid_100 | NodeId 100 | Invalid: OOB |
| 07_zero_query | All zero query | Zero vector |
| 08_unit_query | All ones query | Unit vector |
| 09_8d_query | 8-dimension | Large dim |
| 10_1d_query | 1-dimension | Small dim |
| 11_mixed_query | Pos/neg mix | Sign mix |
| 12_exact_match | Match vector 0 | Distance 0 |
| 13_boundary_N | Various boundaries | Boundary sweep |

---

## Edge Cases Covered

### Numeric Edge Cases
- Zero vectors
- Unit vectors (all 1.0)
- Large values (1e30, -1e30)
- Small values (1e-30, -1e-30)
- Negative values
- Mixed positive/negative

### Structural Edge Cases
- Empty graph (search before insert)
- Single element
- Dense clusters (similar vectors)
- Maximum neighbors
- Invalid node IDs (out of bounds)

### Operation Edge Cases
- Search on empty graph
- Delete non-existent ID
- SaveLoad roundtrip
- Many sequential operations

---

## CI Integration

The corpus seeds are checked into git at `fuzz/corpus/<target>/`. CI can run fuzzing with these seeds:

```bash
# Run all fuzz targets with existing corpus
cargo +nightly fuzz run hnsw_insert -- -max_total_time=60
cargo +nightly fuzz run hnsw_search -- -max_total_time=60
cargo +nightly fuzz run graph_ops -- -max_total_time=60
cargo +nightly fuzz run search_robustness -- -max_total_time=60
```

The fuzzer will:
1. Load existing corpus seeds
2. Mutate seeds to find new coverage
3. Save any new interesting inputs to the corpus

---

## Maintenance

### Adding New Seeds

1. Add generation logic to `generate_corpus.py`
2. Run `python generate_corpus.py`
3. Commit the new seed files

### Cleaning the Corpus

After fuzzing, the corpus may grow large. To minimize:

```bash
cargo +nightly fuzz cmin <target>
```

This keeps only seeds that provide unique coverage.

---

## Statistics

| Target | Seeds | Acceptance Criteria |
|:-------|------:|:--------------------|
| hnsw_insert | 12 | >= 10 |
| hnsw_search | 12 | >= 10 |
| graph_ops | 12 | >= 10 |
| search_robustness | 20 | >= 10 |
| **Total** | **56** | >= 40 |

All targets exceed the minimum requirement of 10 seeds.

---

*Version: 1.0.0*
*Last Updated: 2025-12-13*
