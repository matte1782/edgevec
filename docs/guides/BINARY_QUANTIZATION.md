# Binary Quantization Guide

**Version:** EdgeVec v0.6.0
**Last Updated:** 2025-12-22

---

## Overview

Binary Quantization (BQ) encodes each dimension as a single bit, making binary
codes **32x smaller** than the original F32 payload. Enabling BQ on a float index
retains the F32 vectors for rescoring; the code cache is additional storage.

---

## How It Works

### Binarization Process

1. **Threshold:** Each dimension is compared to zero; there is no training step
2. **Encoding:** Positive values become `1`; zero and negative values become `0`
3. **Packing:** 8 dimensions are packed into a byte, dimension 0 in the least significant bit (128D vector → 16 bytes)

```
F32 Vector:    [0.2, -0.5, 0.8, 0.1, -0.3, 0.9, -0.1, 0.4]
Binary:        [  1,    0,   1,   1,    0,   1,    0,   1]
Packed Byte:   0b10101101 = 0xAD  (dimension 0 is bit 0)
```

### Distance Calculation

BQ uses **Hamming distance** (count of differing bits) instead of L2/cosine:

```
Query:     0b10110101
Candidate: 0b10010111
XOR:       0b00100010
Popcount:  2 (bits that differ)
```

Lower Hamming distance = more similar vectors.

---

## Memory Savings

These sizes compare vector encodings, excluding graph, metadata, and allocation
overhead. A BQ-enabled float index keeps **both** encodings: F32 payloads plus the
binary code cache, about 103% of the original float payload size.

| Dimensions | F32 Size | BQ Size | Reduction |
|:-----------|:---------|:--------|:----------|
| 128D | 512 bytes | 16 bytes | 32x |
| 384D | 1,536 bytes | 48 bytes | 32x |
| 768D | 3,072 bytes | 96 bytes | 32x |
| 1536D | 6,144 bytes | 192 bytes | 32x |

### At Scale (768D vectors)

| Vector Count | F32 Payload | Binary Codes | Encoding Reduction |
|:-------------|:-----------|:----------|:--------|
| 10K | 29.30 MiB | 0.92 MiB | 32x |
| 100K | 292.97 MiB | 9.16 MiB | 32x |
| 1M | 2929.69 MiB | 91.55 MiB | 32x |

---

## API Usage

### JavaScript/TypeScript

```typescript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Explicitly enable BQ on a compatible HNSW float index
const config = new EdgeVecConfig(768);
const db = new EdgeVec(config);
config.free();
db.enableBQ();

// Insert vectors (automatically quantized)
for (const vector of vectors) {
    db.insert(vector);
}

// Search options:

// 1. Raw BQ search (~85% recall, ~5x faster)
const bqResults = db.searchBQ(query, 10);

// 2. BQ + rescore (~95% recall, ~3x faster)
const rescoredResults = db.searchBQRescored(query, 10, 5);

// 3. Hybrid: BQ + metadata filtering
const hybridResults = db.searchHybrid(query, {
    k: 10,
    filter: 'category = "news"',
    useBQ: true,
    rescoreFactor: 5
});
```

### Rust

```rust
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

// Create BQ-enabled index
let config = HnswConfig::new(768);
let mut storage = VectorStorage::new(&config, None);
let mut index = HnswIndex::with_bq(config, &storage)?;

// Insert (auto-quantized)
let id = index.insert_bq(&vector, &mut storage)?;

// Search
let results = index.search_bq(&query, 10, &storage)?;

// Rescored search
let results = index.search_bq_rescored(&query, 10, 5, &storage)?;
```

---

## Performance Characteristics

### Search Speed

| Mode | Relative Speed | Notes |
|:-----|:---------------|:------|
| F32 HNSW | 1x (baseline) | Full precision |
| BQ raw | 5x faster | Hamming distance only |
| BQ + rescore(3) | 3x faster | Good balance |
| BQ + rescore(5) | 2.5x faster | High recall |
| BQ + rescore(10) | 2x faster | Near-F32 quality |

### Recall vs Rescore Factor

| Rescore Factor | Recall@10 | Use Case |
|:---------------|:----------|:---------|
| 1 | ~70% | Speed-critical, low quality OK |
| 3 | ~90% | Balanced (recommended default) |
| 5 | ~95% | Quality-focused |
| 10 | ~98% | Near-F32 quality |
| 15+ | >99% | Maximum quality |

---

## When to Use BQ

### Good Use Cases

- **Large datasets** (100K+ vectors) — Memory savings are significant
- **Browser environments** — Limited memory makes BQ essential
- **Semantic search** — Embedding similarity is robust to quantization
- **Real-time applications** — Faster search latency

### Less Ideal Use Cases

- **Small datasets** (<10K vectors) — Memory savings less important
- **Precision-critical** — Financial, medical where exact similarity matters
- **Very low dimensions** (<64) — Quantization noise more significant

---

## Rescoring Explained

Rescoring improves recall by:

1. **Over-fetch:** Get k × rescoreFactor candidates using BQ
2. **Rerank:** Calculate true F32 distance for candidates
3. **Return:** Top k by F32 distance

```javascript
// Request 10 results, but fetch 50 BQ candidates first
const results = db.searchBQRescored(query, 10, 5);  // 10 * 5 = 50 candidates
```

**Trade-off:** Higher rescore factor = better recall but more F32 computations.

---

## SIMD Optimization

EdgeVec uses SIMD (Single Instruction Multiple Data) for Hamming distance:

| Platform | Instruction | Speedup |
|:---------|:------------|:--------|
| x86-64 | AVX2 POPCNT | 6.9x |
| ARM | NEON CNT | ~5x |
| WASM | Scalar fallback | 1x |

SIMD is automatically detected and used when available.

---

## Persistence

BQ vectors are **not persisted** in snapshots. When loading:

1. Load F32 vectors from snapshot
2. Regenerate BQ vectors from F32

This ensures consistency and allows algorithm improvements.

```javascript
// Save
const bytes = db.toBytes();  // F32 vectors + "BQ enabled" flag

// Load - BQ regenerated automatically
const restored = EdgeVec.fromBytes(bytes);
console.log(restored.hasBQ());  // true if BQ was enabled when saved
```

---

## Troubleshooting

### "BQ not enabled"

```javascript
// Error: BQ methods fail
const config = new EdgeVecConfig(768);
const db = new EdgeVec(config);
config.free();
db.insert(vector);
db.searchBQ(query, 10);  // Throws: BQ has not been enabled
db.enableBQ();
const results = db.searchBQ(query, 10);
```

**Fix:** Call `enableBQ()` explicitly. It requires a HNSW float index with
dimensions divisible by 8 (128, 384, 768, 1536); incompatible indexes reject it.

### Low Recall

If recall is lower than expected:

1. Increase rescore factor: `searchBQRescored(query, k, 10)`
2. Verify query vectors are normalized (for cosine similarity)
3. Check vector distribution — highly clustered data may need higher rescore

### Memory Still High

BQ stores both F32 and BQ vectors for rescoring. Total memory:

```
F32 storage + BQ storage = (1 + 1/32) × F32 storage = ~103% F32 storage
```

For BQ-only (no rescoring), future versions may support discarding F32.

---

## Best Practices

1. **Always use rescoring** — Raw BQ has ~85% recall which may not be acceptable
2. **Start with rescore=5** — Good balance of speed and quality
3. **Benchmark your data** — Recall varies by dataset characteristics
4. **Use 768D or higher** — Lower dimensions have more quantization noise
5. **Monitor recall** — Compare BQ results to F32 periodically

---

## See Also

- [Memory Management Guide](../api/MEMORY.md)
- [Performance Tuning](../PERFORMANCE_TUNING.md)
- [WASM API Reference](../api/WASM_INDEX.md)
