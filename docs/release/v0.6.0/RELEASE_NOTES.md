# EdgeVec v0.6.0 - RFC-002 Complete

This release implements RFC-002: Metadata & Binary Quantization.

## Highlights

- **32x Memory Reduction** - Binary Quantization compresses vectors from 3KB to 96 bytes
- **Metadata Filtering** - Filter search by category, tags, and numeric ranges
- **Memory Monitoring** - Track pressure levels and prevent OOM
- **Hybrid Search** - Combine BQ speed with F32 accuracy via rescoring

## New WASM Exports

| Function | Description |
|:---------|:------------|
| `insertWithMetadata()` | Insert vectors with JSON metadata |
| `searchFiltered()` | Search with filter expressions |
| `searchBQ()` | Fast binary quantized search |
| `searchBQRescored()` | BQ search with F32 rescoring |
| `searchHybrid()` | Adaptive hybrid search |
| `getMemoryPressure()` | Get current memory status |

## Performance

| Metric | v0.5.x | v0.6.0 | Improvement |
|:-------|:-------|:-------|:------------|
| Memory/vector (BQ) | 3KB | 96B | 32x reduction |
| Search latency (BQ) | - | 2-5ms | NEW |
| Recall@10 (BQ+rescore) | - | 0.936 | NEW |

## Installation

```bash
# Rust
cargo add edgevec

# npm
npm install edgevec
```

## Filter Syntax Examples

```javascript
// Simple equality
searchFiltered(query, 10, { filter: "category = 'docs'" })

// Numeric comparison
searchFiltered(query, 10, { filter: "score > 0.8" })

// Compound expressions
searchFiltered(query, 10, { filter: "category = 'tech' AND year >= 2024" })

// Array membership
searchFiltered(query, 10, { filter: "tag IN ('rust', 'wasm')" })
```

## Breaking Changes

None. v0.6.0 is backwards compatible with v0.5.x.

## Links

- [Documentation](https://docs.rs/edgevec)
- [CHANGELOG](./CHANGELOG.md)
- [Cyberpunk Demo](./wasm/examples/v060_cyberpunk_demo.html)
