# Hacker News Submission - EdgeVec v0.7.0

**Platform:** Hacker News (news.ycombinator.com)
**Type:** Show HN

---

## Title (80 char limit)

```
Show HN: EdgeVec – Browser-native vector database in Rust/WASM with SIMD
```

**Character count:** 71

---

## URL

```
https://github.com/matte1782/edgevec
```

---

## Text (Optional - for Show HN)

```
EdgeVec is a vector similarity search library that runs entirely in the browser via WebAssembly. No server required.

Key features:
- Binary quantization: 32x memory reduction (1M vectors in ~125MB)
- SIMD acceleration: 8.75x faster Hamming distance, 2-3x faster cosine
- IndexedDB persistence: survives page reloads
- Filter expressions: SQL-like metadata filtering

Use cases: browser-based RAG, offline semantic search, privacy-preserving AI apps.

Built in Rust, compiles to 494KB WASM. Works in Chrome 91+, Firefox 89+, Safari 16.4+.

Live demo: https://matte1782.github.io/edgevec/demo/

This release includes our first community contribution - @jsonMartin implemented SIMD Hamming distance for an 8.75x speedup on binary vectors.

Happy to answer questions about the architecture or WASM/SIMD implementation details.
```

**Character count:** 798

---

## Posting Guidelines

1. **Submit as "Show HN"** - This is a project showcase
2. **Link to GitHub**, not the demo (HN prefers source)
3. **Don't ask for upvotes** - against HN rules
4. **Be ready to engage** - HN comments can be technical and critical
5. **Best times to post:** 6-9 AM Pacific (weekdays)

---

## Expected Questions to Prepare For

| Question | Answer |
|:---------|:-------|
| "Why not use existing vector DBs?" | Client-side, no server, privacy, offline |
| "How does it compare to Faiss/Annoy?" | Those don't run in browser; EdgeVec is WASM-native |
| "What's the max vector count?" | ~100k comfortable, 1M with BQ |
| "Why brute force instead of HNSW?" | HNSW coming in v0.8.0; brute force is fast enough for <100k |
| "Why Rust?" | Memory safety, WASM target, no runtime |

---

## Backup Title Options

If title doesn't perform well:
1. `Show HN: Vector search that runs entirely in your browser (Rust/WASM)`
2. `Show HN: EdgeVec – Embedded vector DB for browser-based RAG applications`
3. `Show HN: Run semantic search offline in your browser with WASM`
