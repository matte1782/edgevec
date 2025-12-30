# Twitter/X Thread - EdgeVec v0.7.0

**Platform:** Twitter/X
**Format:** Thread (7 tweets)

---

## Tweet 1 (Hook)

```
Just shipped EdgeVec v0.7.0 🦀

A vector database that runs entirely in your browser via WebAssembly.

No server. No API calls. No data leaving your device.

Here's what's new 🧵
```

**Character count:** 196

---

## Tweet 2 (Binary Quantization)

```
32x memory reduction with Binary Quantization

Store 1M vectors in ~125MB instead of 4GB.

Perfect for browser memory constraints, and we're seeing 95%+ recall on standard benchmarks.
```

**Character count:** 199

---

## Tweet 3 (SIMD - Community Contribution)

```
8.75x faster Hamming distance via WASM SIMD128

This came from our FIRST community contributor @jsonMartin 🙌

Open source works. One PR, massive impact.
```

**Character count:** 168

---

## Tweet 4 (Use Cases)

```
Use cases:

• Browser-based RAG with Transformers.js
• Offline semantic search
• Privacy-preserving AI (medical, legal, personal data)
• Local codebase search

Your embeddings never leave the device.
```

**Character count:** 222

---

## Tweet 5 (Code Example)

```
Dead simple API:

import init, { EdgeVec, EdgeVecConfig } from 'edgevec';
await init();

const db = new EdgeVec(new EdgeVecConfig(768));
db.insert(new Float32Array(vec));
db.search(new Float32Array(q), 10);

Works offline.
```

**Character count:** 217

---

## Tweet 6 (Demo + Links)

```
Try it yourself:

🔗 Live demo: https://matte1782.github.io/edgevec/demo/
📦 npm: npm install edgevec
🦀 crates.io: cargo add edgevec
⭐ GitHub: https://github.com/matte1782/edgevec

~220KB gzipped. Works in all modern browsers.
```

**Character count:** 265

---

## Tweet 7 (CTA)

```
What's next:

• HNSW indexing for sub-linear search
• Product quantization
• More embedding model integrations

If you're building local-first AI apps, I'd love to hear what features would help your workflow.

MIT licensed. PRs welcome.
```

**Character count:** 243

---

## Hashtags (Add to Tweet 1 or 7)

```
#rustlang #webassembly #vectordatabase #ai #machinelearning
```

---

## Posting Strategy

1. **Post Tweet 1** with the thread
2. **Space tweets** ~30 seconds apart (or use thread feature)
3. **Best time:** 9 AM - 12 PM EST (weekdays)
4. **Engage with replies** within first hour
5. **Quote tweet** from Rust community accounts if they engage

---

## Image Suggestions

| Tweet | Image |
|:------|:------|
| 1 | Screenshot of demo with "WASM READY" |
| 2 | Memory comparison chart (F32 vs BQ) |
| 3 | Benchmark numbers showing 8.75x |
| 6 | Demo interface screenshot |

---

## Alternative Hook (Tweet 1)

If the first hook doesn't resonate:

```
What if your vector database ran entirely in the browser?

No server costs. No network latency. No data leaving the device.

EdgeVec v0.7.0 does exactly that. Built in Rust, compiled to WASM.

Here's what we shipped 🧵
```
