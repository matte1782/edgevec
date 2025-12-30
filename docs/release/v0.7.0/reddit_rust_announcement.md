# EdgeVec v0.7.0: WASM Vector Database with 8.75x Faster Hamming Distance (First Community Contribution!)

**tl;dr:** EdgeVec is a high-performance vector search library that runs natively in browsers via WASM. v0.7.0 brings massive SIMD optimizations, and I'm thrilled to celebrate our first external contributor.

---

## The Hero of This Release: u/jsonMartin

This release wouldn't be the same without **u/jsonMartin**, who submitted our first community PR. His SIMD-optimized Hamming distance implementation delivered an **8.75x speedup** over the previous version.

When someone takes the time to understand your codebase, write quality code, and contribute back -- that's what open source is all about. Jason didn't just submit a patch; he dove deep into the binary quantization path and emerged with a solution that fundamentally improves performance for anyone doing similarity search with quantized vectors.

Thank you, Jason. This one's for you.

---

## What's EdgeVec?

EdgeVec is a vector similarity search library designed for environments where you can't run a server:

- **Browser-native:** Runs entirely in WebAssembly, no backend required
- **Offline-first:** Search works without network connectivity
- **Lightweight:** 264KB gzipped (494KB raw)
- **Persistent:** Stores indices in IndexedDB, survives page reloads

Use cases: RAG in the browser, semantic search in Electron apps, similarity search in edge functions, offline ML applications.

---

## v0.7.0 Performance Improvements

| Distance Metric | v0.6.0 | v0.7.0 | Speedup |
|:----------------|:-------|:-------|:--------|
| **Hamming** | 175ms | 20ms | **8.75x** |
| **Dot Product** | 50ms | 20ms | **2.5x** |
| **L2 (Euclidean)** | 48ms | 20ms | **2.4x** |
| **Search (k=10)** | 40ms | 20ms | **2.0x** |

*Benchmarks: 100k vectors, 128 dimensions, Chrome 120, M1 MacBook Pro*

### How We Got Here

**WASM SIMD128** is now enabled by default. The Rust compiler generates SIMD instructions that map directly to WebAssembly's `v128` operations:

```rust
// Simplified example of the SIMD approach
#[cfg(target_feature = "simd128")]
pub fn hamming_distance_simd(a: &[u64], b: &[u64]) -> u32 {
    use core::arch::wasm32::*;

    let mut total = u64x2_splat(0);
    for (chunk_a, chunk_b) in a.chunks_exact(2).zip(b.chunks_exact(2)) {
        let va = v128_load(chunk_a.as_ptr() as *const v128);
        let vb = v128_load(chunk_b.as_ptr() as *const v128);
        let xor = v128_xor(va, vb);
        total = u64x2_add(total, u64x2_popcnt(xor));
    }
    // ... reduction
}
```

Browser support:
- Chrome 91+ (May 2021)
- Firefox 89+ (June 2021)
- Safari 16.4+ (March 2023)

For older browsers, we include a scalar fallback.

---

## Interactive Demo: Filter Playground

We've added an interactive **Filter Playground** to the demo site. You can experiment with metadata filtering, adjust thresholds, and see results update in real-time.

**Try it:** [https://matte1782.github.io/edgevec/demo/](https://matte1782.github.io/edgevec/demo/)

Features:
- Live vector search with adjustable k
- Metadata filtering (category, price range, tags)
- Performance timing display
- Code snippets you can copy

---

## Getting Started

### Rust

```bash
cargo add edgevec
```

```rust
use edgevec::{Index, DistanceMetric};

let mut index = Index::new(128, DistanceMetric::Cosine);

// Insert vectors
for embedding in embeddings {
    index.insert(&embedding)?;
}

// Search
let results = index.search(&query, 10)?;
for result in results {
    println!("ID: {}, Score: {:.4}", result.id, result.score);
}
```

### JavaScript/TypeScript (WASM)

```bash
npm install edgevec
```

```typescript
import { EdgeVecIndex } from 'edgevec';

const index = new EdgeVecIndex(128);

// Insert vectors
for (const embedding of embeddings) {
    index.insert(embedding);
}

// Search
const results = index.search(query, 10);
console.log(results); // [{ id: 0n, score: 0.95 }, ...]
```

---

## Links

- **GitHub:** [https://github.com/matte1782/edgevec](https://github.com/matte1782/edgevec)
- **crates.io:** [https://crates.io/crates/edgevec](https://crates.io/crates/edgevec)
- **npm:** [https://www.npmjs.com/package/edgevec](https://www.npmjs.com/package/edgevec)
- **Live Demo:** [https://matte1782.github.io/edgevec/demo/](https://matte1782.github.io/edgevec/demo/)
- **Changelog:** [https://github.com/matte1782/edgevec/blob/main/CHANGELOG.md](https://github.com/matte1782/edgevec/blob/main/CHANGELOG.md)

---

## What's Next (v0.8.0)

- **HNSW indexing** for sub-linear search on large datasets
- **Hybrid search** combining vector similarity with keyword matching
- **Incremental persistence** for faster IndexedDB operations

---

## Feedback Welcome

This is still a young project. If you're building something that needs vector search in the browser, I'd love to hear about your use case. Bug reports, feature requests, and PRs are all welcome.

And if you're thinking about contributing -- look at what Jason did. That could be you on the next release.

---

*EdgeVec is MIT licensed. Built with Rust + wasm-bindgen.*
