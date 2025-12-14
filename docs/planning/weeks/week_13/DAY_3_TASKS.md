# Week 13 — Day 3 Tasks (Wednesday, Dec 18)

**Date:** 2025-12-18
**Focus:** Complete bytemuck Integration + Start Benchmark Setup
**Agent:** RUST_ENGINEER, BENCHMARK_SCIENTIST
**Status:** DRAFT

---

## Day Objective

Complete bytemuck integration by replacing ALL unsafe pointer casts with safe bytemuck operations. Add alignment tests and verify performance overhead. Begin setting up the competitive benchmark harness.

**Success Criteria:**
- All `from_raw_parts` calls replaced with `try_cast_slice`
- All `#[allow(clippy::cast_ptr_alignment)]` removed
- Alignment tests passing
- Performance overhead measured (<1%)
- Benchmark harness skeleton created

---

## Tasks

### W13.2: Integrate bytemuck Crate (COMPLETE)

**Priority:** P0 (Critical Path)
**Estimate:** 8h remaining (14h total)
**Agent:** RUST_ENGINEER
**Status:** COMPLETE on Day 3

#### Day 3 Scope (Complete Implementation)

- [ ] **AC2.6:** Replace unsafe cast in snapshot.rs:223-227
- [ ] **AC2.7:** Replace unsafe cast in chunking.rs:216-220
- [ ] **AC2.8:** Remove ALL `#[allow(clippy::cast_ptr_alignment)]`
- [ ] **AC2.9:** Add AlignmentError to PersistenceError enum
- [ ] **AC2.10:** Create alignment test file
- [ ] **AC2.11:** Add roundtrip property test
- [ ] **AC2.12:** Benchmark serialization overhead
- [ ] **AC2.13:** All existing tests pass
- [ ] **AC2.14:** Clippy passes (no new warnings)

#### Implementation Specification

**Step 1: Replace snapshot.rs:223-227**

```rust
// BEFORE (unsafe):
#[allow(clippy::cast_ptr_alignment, clippy::ptr_as_ptr)]
let nodes: &[HnswNode] = unsafe {
    let ptr = nodes_bytes.as_ptr() as *const HnswNode;
    std::slice::from_raw_parts(ptr, vec_count)
};

// AFTER (safe):
use bytemuck::try_cast_slice;

let nodes: &[HnswNode] = try_cast_slice(nodes_bytes)
    .map_err(|e| PersistenceError::Alignment {
        source: format!("{:?}", e),
        context: "HnswNode deserialization".to_string(),
    })?;

// Verify count matches
if nodes.len() != vec_count {
    return Err(PersistenceError::Corrupted(format!(
        "Node count mismatch: expected {}, got {}",
        vec_count, nodes.len()
    )));
}
```

**Step 2: Add Error Variant**

```rust
// src/error.rs (or src/persistence/mod.rs)
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    // ... existing variants ...

    #[error("Alignment error in {context}: {source}")]
    Alignment {
        source: String,
        context: String,
    },
}
```

**Step 3: Create Alignment Tests**

```rust
// tests/alignment_safety.rs
use edgevec::hnsw::graph::HnswNode;
use bytemuck::{Pod, Zeroable};

#[test]
fn hnsw_node_is_pod() {
    // This test verifies at compile time that HnswNode implements Pod
    fn assert_pod<T: Pod>() {}
    assert_pod::<HnswNode>();
}

#[test]
fn hnsw_node_is_zeroable() {
    fn assert_zeroable<T: Zeroable>() {}
    assert_zeroable::<HnswNode>();
}

#[test]
fn hnsw_node_roundtrip() {
    let original = HnswNode {
        id: 42,
        max_layer: 3,
        _padding: [0; 3],
        neighbor_start: 100,
        neighbor_counts: [4, 8, 12, 0],
    };

    // Serialize to bytes
    let bytes: &[u8] = bytemuck::bytes_of(&original);

    // Deserialize back
    let recovered: &HnswNode = bytemuck::from_bytes(bytes);

    assert_eq!(original.id, recovered.id);
    assert_eq!(original.max_layer, recovered.max_layer);
    assert_eq!(original.neighbor_start, recovered.neighbor_start);
}

#[test]
fn try_cast_slice_detects_misalignment() {
    // Create intentionally misaligned buffer
    let mut buffer = vec![0u8; std::mem::size_of::<HnswNode>() + 1];
    let misaligned = &buffer[1..];  // Start at offset 1 (likely misaligned)

    let result: Result<&[HnswNode], _> = bytemuck::try_cast_slice(misaligned);

    // Should fail for misaligned data
    assert!(result.is_err());
}
```

**Step 4: Benchmark Overhead**

```rust
// benches/bytemuck_overhead.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_serialization(c: &mut Criterion) {
    // Setup: create nodes
    let nodes: Vec<HnswNode> = (0..10000)
        .map(|i| HnswNode { id: i, ..Default::default() })
        .collect();

    c.bench_function("serialize_bytemuck", |b| {
        b.iter(|| {
            let bytes: &[u8] = bytemuck::cast_slice(&nodes);
            std::hint::black_box(bytes.len())
        })
    });

    c.bench_function("deserialize_bytemuck", |b| {
        let bytes: &[u8] = bytemuck::cast_slice(&nodes);
        b.iter(|| {
            let recovered: &[HnswNode] = bytemuck::cast_slice(bytes);
            std::hint::black_box(recovered.len())
        })
    });
}

criterion_group!(benches, benchmark_serialization);
criterion_main!(benches);
```

#### Files to Create

- `tests/alignment_safety.rs` (new)
- `benches/bytemuck_overhead.rs` (new, optional)

#### Files to Modify

- `src/persistence/snapshot.rs` (replace unsafe)
- `src/persistence/chunking.rs` (replace unsafe)
- `src/error.rs` or `src/persistence/mod.rs` (add error variant)

#### Verification Commands

```bash
# Verify no unsafe casts remain
grep -rn "as \*const HnswNode" src/
# Should return 0 results

# Verify clippy allows removed
grep -rn "cast_ptr_alignment" src/
# Should return 0 results

# Run alignment tests
cargo test alignment

# Run all tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Run benchmark (if created)
cargo bench --bench bytemuck_overhead
```

---

### W13.3a: Competitive Benchmark Setup (START)

**Priority:** P1
**Estimate:** 2h on Day 3 (8h total)
**Agent:** BENCHMARK_SCIENTIST
**Status:** START Day 3 (2h), CONTINUE Day 4 (6h)

#### Day 3 Scope (Partial)

- [ ] **AC3a.1:** Create benchmark directory structure
- [ ] **AC3a.2:** Document hardware specifications
- [ ] **AC3a.3:** Create harness skeleton
- [ ] **AC3a.4:** Install first competitor library (hnswlib-wasm)

#### Implementation Specification

**Directory Structure:**

```
benches/
└── competitive/
    ├── harness.js           # Main benchmark runner
    ├── package.json         # Node.js dependencies
    ├── adapters/
    │   ├── edgevec.js       # EdgeVec WASM adapter
    │   ├── hnswlib.js       # hnswlib-wasm adapter
    │   ├── voy.js           # voy adapter
    │   ├── usearch.js       # usearch-wasm adapter
    │   └── vectra.js        # vectra adapter
    ├── data/
    │   └── sift_100k.json   # Test vectors (generated)
    └── results/
        └── .gitkeep
```

**Hardware Documentation Template:**

```markdown
# docs/benchmarks/hardware_specs.md

## Test Environment

**CPU:** [e.g., Intel Core i7-12700K @ 3.6GHz]
**RAM:** [e.g., 32GB DDR5-4800]
**OS:** [e.g., Windows 11 Pro 23H2]
**Browser:** Chrome 120.0.6099.109
**Node.js:** v20.10.0
**WASM Engine:** V8 (via Chrome/Node)

## Test Date

2025-12-18

## Reproducibility Notes

[Any relevant settings, background processes stopped, etc.]
```

#### Files to Create

- `benches/competitive/package.json`
- `benches/competitive/harness.js` (skeleton)
- `docs/benchmarks/hardware_specs.md`

#### Verification Commands

```bash
# Verify directory created
test -d benches/competitive && echo "OK"

# Initialize npm project
cd benches/competitive && npm init -y

# Install first competitor
npm install hnswlib-wasm
```

---

## Day 3 Summary

**Total Effort:** 6h (W13.2 complete) + 2h (W13.3a start) = **8h scheduled**

**Deliverables:**
1. ✅ All unsafe casts replaced with bytemuck
2. ✅ AlignmentError added to PersistenceError
3. ✅ Alignment tests passing
4. ✅ Performance overhead documented
5. ✅ Benchmark harness skeleton created

**Carryover to Day 4:**
- W13.3a: Complete competitor library setup (6h)
- W13.3b: Run EdgeVec benchmarks

**Blockers Removed:**
- W13.3b can proceed (EdgeVec stable build available)
- Safety hardening COMPLETE

**Status Validation:**
```bash
# Run before end of day
cargo test
cargo clippy -- -D warnings
grep -c "try_cast_slice" src/persistence/snapshot.rs  # Should be >= 1
grep -c "cast_ptr_alignment" src/persistence/  # Should be 0
test -d benches/competitive && echo "Benchmark dir exists"
```

---

## HOSTILE_REVIEWER Pre-Flight Checklist

Before submitting Day 3 work:

- [ ] W13.2 complete (all unsafe replaced)
- [ ] No `#[allow(clippy::cast_ptr_alignment)]` in codebase
- [ ] Alignment tests pass
- [ ] All existing tests still pass
- [ ] Benchmark harness skeleton created
- [ ] Hardware specs documented

---

**PLANNER Notes:**
- Day 3 is the "safety completion" milestone
- After Day 3, NO unsafe pointer alignment issues remain
- This directly addresses Reddit community feedback
- Benchmark setup can proceed in parallel with final testing

**Status:** DRAFT
**Next:** Execute W13.2 completion, W13.3a start
