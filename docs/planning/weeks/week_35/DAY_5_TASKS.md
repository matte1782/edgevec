# Week 35 Day 5: Test Clippy + Comparison Doc

**Date:** 2026-01-31
**Focus:** Clean test warnings + EdgeVec vs pgvector comparison
**Hours:** 1h
**Status:** [x] COMPLETE

---

## Context

Two tasks for Day 5:
1. Clean clippy warnings in `tests/` and `benches/` directories
2. Write "EdgeVec vs pgvector" comparison document (Milestone 8.3)

**Priority:** P3 (test cleanup) + P2 (documentation)

---

## Tasks

### W35.4: Test Code Clippy Warnings (1h)

**Goal:** Zero clippy warnings in test code.

**Subtasks:**

- [x] **5.1** Audit test warnings (15min) COMPLETE
  - Ran `cargo clippy --tests -- -D warnings`
  - Ran `cargo clippy --benches -- -D warnings`
  - Found 7 warnings total

- [x] **5.2** Fix test warnings (30min) COMPLETE
  - Fixed binary literals in simd.rs (0b10101010 -> 0b1010_1010)
  - Fixed uninlined_format_args in simd.rs and chunking.rs
  - Added #[allow(cast_precision_loss)] for test data generation
  - Inlined format args in assert messages

- [x] **5.3** Verify clean (15min) COMPLETE
  - `cargo clippy --tests` zero warnings
  - `cargo clippy --benches` zero warnings
  - All 700 tests pass

---

### W35.5: EdgeVec vs pgvector Comparison (1h)

**Goal:** Create comparison document for Milestone 8.3.

**Location:** `docs/guides/COMPARISON_PGVECTOR.md`

**Subtasks:**

- [x] **5.4** Research pgvector features (15min) COMPLETE
  - Reviewed pgvector 0.7.x features
  - Index types: IVFFlat, HNSW
  - Quantization: half-precision (fp16)

- [x] **5.5** Write comparison document (45min) COMPLETE
  - Created `docs/guides/COMPARISON_PGVECTOR.md`
  - Feature comparison table
  - Architecture diagrams
  - Use case recommendations
  - Migration considerations

---

## Comparison Document Outline

```markdown
# EdgeVec vs pgvector: Choosing the Right Vector Database

## Overview
- pgvector: PostgreSQL extension for vector similarity search
- EdgeVec: Embedded WASM-native vector database

## Feature Comparison

| Feature | EdgeVec | pgvector |
|:--------|:--------|:---------|
| Deployment | Embedded/Browser | PostgreSQL |
| Index Types | HNSW, Flat | IVFFlat, HNSW |
| Quantization | Binary, Scalar | Half-precision |
| Max Dimensions | Unlimited | 2000 (default) |
| Filtering | Integrated | SQL WHERE |
| Language | Rust/WASM | C/SQL |
| Persistence | File/IndexedDB | PostgreSQL |

## Architecture Comparison

### EdgeVec
- Runs in-process (no network latency)
- WASM-native for browser deployment
- Zero-copy operations where possible

### pgvector
- Requires PostgreSQL server
- Leverages PostgreSQL's ACID guarantees
- Rich SQL ecosystem integration

## Performance Comparison

[Note: Fair comparison requires same hardware/dataset]

| Metric | EdgeVec (target) | pgvector (typical) |
|:-------|:-----------------|:-------------------|
| Search 100k | <10ms | ~10-50ms |
| Insert | <5ms | ~1-10ms |
| Memory/vector | <100 bytes | ~128+ bytes |

## When to Choose

### Choose EdgeVec when:
- Browser/edge deployment required
- Embedded use case (no server)
- Real-time applications needing <10ms latency
- Offline-first applications

### Choose pgvector when:
- Already using PostgreSQL
- Need ACID transactions
- Complex SQL queries required
- Team expertise in PostgreSQL

## Migration Considerations

### pgvector → EdgeVec
1. Export vectors as JSON/binary
2. Import using EdgeVec bulk loader
3. Recreate filters as EdgeVec expressions

### EdgeVec → pgvector
1. Export using EdgeVec's persistence format
2. Convert to pgvector INSERT statements
3. Create appropriate indexes
```

---

## Acceptance Criteria

### Test Cleanup
- [x] `cargo clippy --tests` zero warnings
- [x] `cargo clippy --benches` zero warnings
- [x] All tests still pass (700 tests)

### Comparison Document
- [x] Feature table complete and accurate
- [x] Use case guidance clear
- [x] No marketing language (objective facts)
- [x] Cross-referenced from appropriate docs

---

## Exit Criteria

Day 5 is complete when:
- [x] Test clippy clean
- [x] Comparison doc created
- [x] Both committed

---

## Commit Message Template

```
chore(tests): clean clippy warnings in tests and benches

- Fix N warnings in tests/
- Fix M warnings in benches/
- Add #[allow] for test helpers

docs(comparison): add EdgeVec vs pgvector guide

- Feature comparison table
- Architecture differences
- Use case recommendations

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Day 5 Total:** 2 hours
**Agents:** RUST_ENGINEER (1h), DOCWRITER (1h)
