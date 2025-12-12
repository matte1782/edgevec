# EdgeVec v0.2.0-alpha.2 Announcement Templates

**Release Date:** 2025-12-12
**Announcement Date:** 2025-12-12
**Status:** Ready for publication

---

## Twitter/X Announcement

**Character count:** ~400 (within limit)

```
🚀 EdgeVec v0.2.0-alpha.2 Released!

High-performance vector search for Browser, Node.js, and Edge — 100% local, zero network latency.

✨ Sub-millisecond search at 100k vectors
✨ 148 KB bundle (WASM)
✨ 3.6x memory reduction with SQ8
✨ IndexedDB persistence

📦 npm install edgevec

🔗 https://github.com/matte1782/edgevec

#vectorsearch #wasm #rust #javascript #typescript #ai #embeddings
```

---

## Reddit r/rust Announcement

**Title:** `[ANN] EdgeVec v0.2.0-alpha.2 - High-performance vector search for Browser/Node/Edge (Rust + WASM)`

**Body:**

```markdown
Hi r/rust!

I'm excited to share **EdgeVec**, a high-performance vector database written in Rust with first-class WASM support.

## What is it?

EdgeVec implements HNSW (Hierarchical Navigable Small World) graphs for approximate nearest neighbor search. It's designed to run entirely in the browser, Node.js, or edge devices — no server required.

## Performance

| Scale | Float32 | Quantized (SQ8) |
|:------|:--------|:----------------|
| 10k vectors | 203 µs | **88 µs** |
| 50k vectors | 480 µs | **167 µs** |
| 100k vectors | 572 µs | **329 µs** |

Tested on 768-dimensional vectors (typical embedding size), k=10 nearest neighbors.

## Key Features

- **Sub-millisecond search** at 100k scale
- **3.6x memory reduction** with Scalar Quantization (SQ8)
- **148 KB bundle** (70% under budget)
- **IndexedDB persistence** for browser storage
- **Zero network latency** — runs locally

## Quick Start

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();
const config = new EdgeVecConfig(768);
const index = new EdgeVec(config);
index.insert(new Float32Array(768).fill(0.1));
const results = index.search(query, 10);
// results: [{ id: 0, score: 0.0 }, ...]
```

## Links

- GitHub: https://github.com/matte1782/edgevec
- npm: https://www.npmjs.com/package/edgevec
- Docs: https://github.com/matte1782/edgevec/blob/main/README.md

## Known Limitations (Alpha)

- Build time not optimized (batch API planned for v0.3.0)
- No delete/update operations yet
- Single-threaded WASM execution

## Technical Details

- Pure Rust implementation
- WASM via wasm-pack/wasm-bindgen
- SIMD-optimized distance calculations (AVX2 on native, simd128 on WASM where available)
- TypeScript types included

Looking forward to feedback! This is an alpha release, so please report any issues on GitHub.
```

---

## Reddit r/javascript Announcement

**Title:** `EdgeVec - Vector search that runs 100% in the browser (148KB, sub-millisecond)`

**Body:**

```markdown
Hi r/javascript!

Just released **EdgeVec** — a vector database that runs entirely in your browser, no server required.

## Why?

- Privacy: Your embeddings never leave the device
- Latency: Zero network round-trip
- Offline: Works without internet

## Performance

- **Sub-millisecond** search at 100k vectors
- **148 KB** gzipped bundle
- **IndexedDB** for persistent storage

## Usage

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();
const config = new EdgeVecConfig(768);
config.metric = 'cosine';  // Optional: 'l2', 'cosine', or 'dot'
const index = new EdgeVec(config);

// Insert vectors
index.insert(new Float32Array(768).fill(0.1));

// Search
const results = index.search(queryVector, 10);
// Returns: [{ id: 0, score: 0.0 }, ...]

// Persist to IndexedDB
await index.save('my-vectors');

// Load later
const loaded = await EdgeVec.load('my-vectors');
```

## Use Cases

- Browser extensions with semantic search
- Local-first note-taking apps
- Privacy-preserving RAG applications
- Edge computing (IoT, embedded)

## Links

- npm: `npm install edgevec`
- GitHub: https://github.com/matte1782/edgevec
- TypeScript types included

This is an alpha release. Feedback welcome!
```

---

## LinkedIn Announcement

```markdown
Excited to announce EdgeVec v0.2.0-alpha.2!

EdgeVec is a high-performance vector database that runs entirely in the browser, Node.js, or edge devices. Built in Rust with first-class WASM support.

Key achievements:
- Sub-millisecond search at 100k vectors
- 148 KB bundle (70% under target)
- 3.6x memory reduction with quantization
- Zero network latency — 100% local

Perfect for:
- Privacy-preserving search applications
- Browser extensions
- Offline-first apps
- Edge computing

The project started as a research experiment to see if we could achieve server-grade vector search performance in the browser. The answer: yes.

Technical stack: Rust, WebAssembly, HNSW graphs, SIMD optimizations.

Try it: npm install edgevec
GitHub: https://github.com/matte1782/edgevec

Looking forward to community feedback!

#vectorsearch #rust #webassembly #javascript #ai #embeddings #opensource
```

---

## Hacker News "Show HN" (Optional)

**Title:** `Show HN: EdgeVec – Sub-millisecond vector search in the browser (Rust/WASM)`

**Text:**

```markdown
Hi HN,

I built EdgeVec, a vector database that runs entirely in the browser. It implements HNSW (Hierarchical Navigable Small World) graphs for approximate nearest neighbor search.

Performance:
- Sub-millisecond search at 100k vectors (768 dimensions, k=10)
- 148 KB gzipped bundle
- 3.6x memory reduction with scalar quantization

Use cases: browser extensions with semantic search, local-first apps, privacy-preserving RAG.

Technical: Written in Rust, compiled to WASM. Uses AVX2 SIMD on native, simd128 on WASM. IndexedDB for browser persistence.

npm: https://www.npmjs.com/package/edgevec
GitHub: https://github.com/matte1782/edgevec

This is an alpha release. Main limitations: build time not optimized, no delete operations yet.

Would love feedback from the community!
```

---

## Dev.to Article (Optional Extended Form)

**Title:** Building a Sub-Millisecond Vector Database in Rust/WASM

**Tags:** rust, wasm, javascript, performance

**Body:**

```markdown
I recently built EdgeVec, a high-performance vector database that runs entirely in the browser. Here's how I achieved sub-millisecond search times with WebAssembly.

## The Challenge

Vector databases are everywhere in AI applications - they power semantic search, RAG systems, and recommendation engines. But they typically require a server. I wanted to explore: can we get comparable performance entirely in the browser?

## Performance Results

| Scale | Float32 | Quantized (SQ8) |
|:------|:--------|:----------------|
| 10k vectors | 203 µs | **88 µs** |
| 50k vectors | 480 µs | **167 µs** |
| 100k vectors | 572 µs | **329 µs** |

This is comparable to server-side solutions, but running entirely client-side.

## Technical Approach

### 1. HNSW Algorithm
I implemented Hierarchical Navigable Small World graphs - the same algorithm used by production vector databases like Weaviate and Qdrant.

### 2. Scalar Quantization
Instead of storing 32-bit floats (768 dimensions × 4 bytes = 3KB per vector), I compress to 8-bit integers. This gives 3.6x memory savings with minimal accuracy loss.

### 3. SIMD Optimization
Using Rust's portable SIMD, I vectorize distance calculations:
- AVX2 on native (x86_64)
- simd128 on WASM (where available)

### 4. WASM Compilation
Built with wasm-pack, the final bundle is just 148 KB gzipped - small enough for any web app.

## Use Cases

Where does client-side vector search make sense?

- **Privacy**: Embeddings never leave the device
- **Latency**: Zero network round-trip
- **Offline**: Works without internet
- **Cost**: No server bills

Perfect for browser extensions, local-first apps, and privacy-preserving RAG.

## Try It

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();
const config = new EdgeVecConfig(768);
const index = new EdgeVec(config);
index.insert(new Float32Array(768).fill(0.1));
const results = index.search(query, 10);
// results: [{ id: 0, score: 0.0 }, ...]
```

GitHub: https://github.com/matte1782/edgevec
npm: `npm install edgevec`

This is an alpha release - feedback welcome!
```

---

## Usage Instructions for User

1. **Choose Your Platforms** (at your discretion):
   - **Recommended:** Twitter/X (quick, broad reach)
   - **Recommended:** Reddit r/rust (technical audience)
   - **Optional:** Reddit r/javascript (web dev audience)
   - **Optional:** LinkedIn (professional network)
   - **Optional:** Hacker News (high-quality feedback, strict rules)
   - **Optional:** Dev.to (long-form, SEO-friendly)

2. **Copy-Paste Content**:
   - Each platform has ready-to-use content above
   - Minor edits welcome but not required

3. **Record URLs**:
   - Save all posted URLs in `ANNOUNCEMENT_LOG.md` (created next)

4. **Monitor Engagement**:
   - Track metrics in announcement log
   - Respond to comments professionally
   - Document feedback for future releases

---

**Status:** Ready for publication at user discretion
**Next:** Create ANNOUNCEMENT_LOG.md for tracking
