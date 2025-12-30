# Reddit r/MachineLearning Announcement - EdgeVec v0.7.0

**Subreddit:** r/MachineLearning
**Title:** [P] EdgeVec v0.7.0: Browser-Native Vector Database with 8.75x Faster Hamming Distance via SIMD

---

## Post Content

I've been building **EdgeVec**, an open-source vector database that runs entirely in the browser via WebAssembly. With v0.7.0, we're shipping significant SIMD optimizations and celebrating our first community contribution.

### What is EdgeVec?

EdgeVec is a lightweight vector search engine designed for:

- **Browser-based RAG applications** - Run retrieval-augmented generation without server roundtrips
- **Semantic search in web apps** - Build search experiences that understand meaning, not just keywords
- **Offline-first AI tools** - Your embeddings and data never leave the user's device

It works with embeddings from any provider: **OpenAI** (text-embedding-3-small/large), **Cohere** (embed-english-v3), **HuggingFace** (all-MiniLM, BGE, etc.), or your own fine-tuned models.

### v0.7.0 Highlights

**1. 8.75x Faster Hamming Distance (Community Contribution)**

Our first external contributor [@jsonMartin](https://github.com/jsonMartin) implemented WASM SIMD128 Hamming distance computation. For binary-quantized vectors:

| Operation | Before | After | Speedup |
|:----------|:-------|:------|:--------|
| Hamming Distance | 87.5 ns | 10.0 ns | **8.75x** |

This uses the `v128.popcnt` instruction available in modern browsers, making binary vector search extremely fast.

**2. Binary Quantization: 32x Memory Reduction**

Store 1536-dim embeddings (OpenAI large) in just 48 bytes instead of 6144 bytes:

- **32x memory reduction** with typical **95%+ recall retention**
- Makes million-vector indices practical in browser memory
- Automatic projection + binarization pipeline

**3. SIMD-Accelerated Euclidean Distance (3.2x faster)**

Previous release added SIMD for Euclidean/cosine, this release extends SIMD coverage:

```
Euclidean Distance (1536-dim):
  Scalar:  ~450 ns
  SIMD:    ~140 ns (3.2x faster)
```

### Technical Architecture

```
+-------------------------------------------------------------+
|                     Your Web Application                     |
+-------------------------------------------------------------+
|  EdgeVec (WASM)                                              |
|  +-- Vector Storage (flat + binary-quantized)               |
|  +-- SIMD Kernels (f32 ops, Hamming distance)               |
|  +-- Cosine / Euclidean / Hamming similarity                |
|  +-- Persistence (IndexedDB via idb-keyval)                 |
+-------------------------------------------------------------+
|  Browser Runtime                                             |
|  +-- WebAssembly + SIMD128 (Chrome 91+, Firefox 89+)        |
+-------------------------------------------------------------+
```

### Code Example: RAG with OpenAI Embeddings

```javascript
import { EdgeVecIndex } from 'edgevec';

// Create index for OpenAI embeddings
const index = new EdgeVecIndex({
  dimensions: 1536,  // text-embedding-3-small
  quantization: 'binary'  // Enable 32x compression
});

// Index your documents
for (const doc of documents) {
  const embedding = await openai.embeddings.create({
    model: 'text-embedding-3-small',
    input: doc.text
  });
  index.insert(embedding.data[0].embedding, { id: doc.id });
}

// Semantic search - runs locally, no API call
const queryEmbedding = await openai.embeddings.create({
  model: 'text-embedding-3-small',
  input: userQuery
});

const results = index.search(queryEmbedding.data[0].embedding, { k: 5 });
// Use top-k results for RAG context
```

### Why Browser-Native Matters for ML Applications

1. **Privacy**: Embeddings contain semantic information about your data. Running locally means sensitive data never leaves the device.

2. **Latency**: Eliminate network roundtrips. Search is sub-millisecond after embeddings are computed.

3. **Offline capability**: Applications work without internet after initial embedding computation.

4. **Cost**: No vector database hosting costs. Users' browsers provide the compute.

### Benchmarks (100k vectors, 768-dim)

| Operation | Performance |
|:----------|:------------|
| Insert (binary quant) | 15,000 vec/sec |
| Search k=10 (binary) | 1.2ms |
| Memory per vector | 48 bytes (binary) vs 3072 bytes (f32) |

### Links

- **GitHub**: https://github.com/matte1782/edgevec
- **Live Demo**: https://matte1782.github.io/edgevec/demo/
- **npm**: https://www.npmjs.com/package/edgevec
- **Docs**: https://github.com/matte1782/edgevec#readme

### What's Next

- [ ] IVF indexing for sub-linear search on large indices
- [ ] Product quantization (PQ) for more compression options
- [ ] Streaming insert API for real-time applications

Would love feedback from the ML community, especially on:
- Embedding dimension / model combinations you'd want optimized
- Use cases where browser-native search would be valuable
- Performance comparisons you'd like to see

**License**: MIT

---

## Post Metadata

- **Flair**: [P] (Project)
- **Crosspost to**: r/rust, r/webdev, r/LocalLLaMA
