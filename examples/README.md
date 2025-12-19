# EdgeVec Examples

This directory contains runnable examples demonstrating EdgeVec features.

## Filter Examples (v0.5.0)

### Basic Filtered Search

```bash
cargo run --example filter_basic
```

Demonstrates:
- Creating an index with metadata
- Simple equality filters (`category = "fruit"`)
- Range filters (`price < 2.0`)
- Combined AND/OR filters

### E-commerce Product Search

```bash
cargo run --example filter_ecommerce
```

Demonstrates:
- Product catalog with rich metadata
- Price range filtering
- Category filtering
- Rating filtering
- Combined multi-attribute queries
- IN operator for multiple categories

### Document Similarity Search

```bash
cargo run --example filter_documents
```

Demonstrates:
- Document corpus with metadata
- String operators (CONTAINS, STARTS_WITH)
- NULL checks for optional fields
- Author/date filtering

### Real-time Filtering

```bash
cargo run --example filter_realtime
```

Demonstrates:
- Building filters programmatically
- Comparing filter strategies (PreFilter, PostFilter, Hybrid, Auto)
- Performance characteristics
- Strategy selection based on selectivity

### Persistence with Filters

```bash
cargo run --example filter_persistence
```

Demonstrates:
- Creating an index with filtered data
- Soft delete workflow
- Compaction
- Save/load with filters preserved

## Core Examples

### Batch Insert

```bash
cargo run --example batch_insert
```

Demonstrates:
- Batch insertion API
- Progress tracking
- Performance comparison

### Memory Profile

```bash
cargo run --example memory_profile
```

Demonstrates:
- Memory usage tracking
- Quantization effects

### SIMD Check

```bash
cargo run --example simd_check
```

Demonstrates:
- SIMD capability detection
- Backend selection

## Running All Examples

```bash
# Run all filter examples
cargo run --example filter_basic
cargo run --example filter_ecommerce
cargo run --example filter_documents
cargo run --example filter_realtime
cargo run --example filter_persistence

# Run with release mode for accurate timing
cargo run --release --example filter_realtime
```

## Requirements

- Rust 1.70+
- EdgeVec crate

## See Also

- [Filter Syntax Reference](../docs/api/FILTER_SYNTAX.md)
- [Database Operations Guide](../docs/api/DATABASE_OPERATIONS.md)
- [TypeScript API](../docs/api/TYPESCRIPT_API.md)
