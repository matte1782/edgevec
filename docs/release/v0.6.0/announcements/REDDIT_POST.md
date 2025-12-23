# EdgeVec v0.6.0: Browser-Native Vector Database with 32x Memory Reduction

I just released EdgeVec v0.6.0, implementing RFC-002 (Metadata & Binary Quantization).

## What is EdgeVec?

A vector database that runs entirely in the browser via WebAssembly. No server required - your vectors stay on-device.

## What's New in v0.6.0?

1. **Binary Quantization** - Compress vectors 32x (768-dim: 3KB -> 96 bytes)
2. **Metadata Filtering** - Query with expressions: `category = 'docs' AND year > 2023`
3. **Memory Monitoring** - Track pressure, prevent OOM
4. **Hybrid Search** - BQ speed + F32 accuracy via rescoring

## Performance

| Metric | Result |
|:-------|:-------|
| Memory per vector (BQ) | 96 bytes |
| Search latency (BQ, 100k) | 2-5ms |
| Recall@10 (BQ+rescore) | 0.936 |
| Bundle size | ~500KB gzipped |

## Try It

- **npm:** `npm install edgevec`
- **Rust:** `cargo add edgevec`
- **GitHub:** https://github.com/matte1782/edgevec
- **Docs:** https://docs.rs/edgevec

## Use Cases

- **Semantic search in browser apps** - No server roundtrip
- **Mobile-first AI apps** - Works on iOS/Android browsers
- **Privacy-preserving search** - Data never leaves device
- **Offline-capable apps** - Search works without network

## Technical Details

EdgeVec uses HNSW (Hierarchical Navigable Small World) graphs for approximate nearest neighbor search. Binary quantization reduces each float32 to 1 bit via sign-based projection, achieving 32x compression with minimal recall loss.

The hybrid search mode uses BQ for fast candidate generation, then rescores top results with full-precision vectors for optimal accuracy.

Feedback welcome!

---

*Suggested subreddits:*
- r/rust
- r/MachineLearning
- r/webdev
- r/programming
