# EdgeVec v0.7.0: Run Vector Search in Your Browser — 32x Memory Reduction + SIMD Acceleration

**No server. No API calls. No data leaving your device.**

I've been working on EdgeVec, an embedded vector database that runs entirely in the browser via WebAssembly. The goal: give local/offline AI applications the same vector search capabilities as cloud services, but with zero network dependency.

## Why This Matters for Local LLM Users

If you're running local models with Transformers.js, Ollama, or llama.cpp, you've probably hit this problem: **where do you store and search your embeddings?**

Most vector DBs require:
- A server running somewhere
- Network calls (even to localhost)
- Setup and configuration

EdgeVec runs **in the same JavaScript context** as your application. Import it, use it. That's it.

```javascript
import { EdgeVecIndex } from 'edgevec';
import { pipeline } from '@xenova/transformers';

// Your local embedding model
const embedder = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');

// Create index (384 dimensions for MiniLM)
const index = new EdgeVecIndex({ dimensions: 384 });

// Index your documents locally
for (const doc of documents) {
  const embedding = await embedder(doc.text, { pooling: 'mean', normalize: true });
  index.insert(Array.from(embedding.data), { id: doc.id });
}

// Search - everything happens on device
const queryEmb = await embedder(query, { pooling: 'mean', normalize: true });
const results = index.search(Array.from(queryEmb.data), 10);
```

## What's New in v0.7.0

### 1. Binary Quantization — 32x Memory Reduction

Store 1M vectors in ~125MB instead of 4GB. Perfect for browser memory constraints.

```javascript
// Enable binary quantization for massive collections
const index = new EdgeVecIndex({
  dimensions: 768,
  quantization: 'binary'  // 32x smaller
});
```

The quality tradeoff is surprisingly small for many use cases (we're seeing 95%+ recall on standard benchmarks).

### 2. SIMD Acceleration — Up to 8.75x Faster

WebAssembly SIMD is now enabled by default:
- **Hamming distance: 8.75x faster** (for binary quantization)
- **Cosine similarity: 2-3x faster** (for float vectors)

No configuration needed. It just works if your browser supports SIMD (Chrome 91+, Firefox 89+, Safari 16.4+).

### 3. IndexedDB Persistence

Your index survives browser refreshes. Build once, use forever (until you clear site data).

```javascript
// Save to IndexedDB
await index.saveToIndexedDB('my-local-rag');

// Load on next session
const index = await EdgeVecIndex.loadFromIndexedDB('my-local-rag');
```

### 4. Filter Expressions

Query with metadata filters — essential for any real RAG system:

```javascript
const results = index.search(queryVector, 10, {
  filter: {
    $and: [
      { category: 'documentation' },
      { date: { $gte: '2024-01-01' } }
    ]
  }
});
```

## Real-World Use Cases

**Local Document Search**
Index your PDFs, notes, or code locally. Search semantically without uploading anything anywhere.

**Offline RAG**
Build RAG applications that work on airplanes, in secure environments, or anywhere without internet.

**Privacy-Preserving AI Assistants**
Create browser extensions or web apps that handle sensitive data (medical notes, legal documents, personal journals) with zero data exfiltration risk.

**Local Codebase Search**
Index your codebase with a local embedding model. Search by "what does this code do" instead of grep.

## Performance Numbers

Tested on M1 MacBook, 100k vectors, 768 dimensions:

| Operation | Float32 | Binary Quantized |
|:----------|:--------|:-----------------|
| Search (k=10) | 12ms | 3ms |
| Memory/vector | 3KB | 96 bytes |
| Insert | 0.8ms | 0.3ms |

## First Community Contribution

Shoutout to **@jsonMartin** for contributing the SIMD Hamming distance implementation. This is EdgeVec's first external contribution, and it brought an 8.75x speedup. Open source works.

## Try It

**Live Demo** (runs entirely in your browser):
https://matte1782.github.io/edgevec/demo/

**GitHub**:
https://github.com/matte1782/edgevec

**npm**:
```bash
npm install edgevec
```

## What's Next

- HNSW indexing for sub-linear search (currently brute force, which is fine up to ~100k vectors)
- Product quantization for better quality/size tradeoffs
- More embedding model integrations

---

Would love feedback from folks running local LLM setups. What would make this more useful for your workflows?

The whole point is: **your data, your device, your search**. No cloud required.
