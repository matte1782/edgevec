# RFC: BM25 Hybrid Search for EdgeVec

**RFC ID:** RFC-006
**Author:** EdgeVec Team
**Date:** 2025-12-24
**Status:** PROPOSED
**Target Version:** v0.9.0+ (Future)

---

## Summary

This RFC analyzes whether BM25 hybrid search should be added to EdgeVec, based on user feedback (Reddit user "Lucas").

---

## User Request

> "is BM25 support on roadmap here? I would love to build pretty simple hybrid search, I have been working mostly with pgvector but it's a bit large for my use!"

---

## What is BM25 Hybrid Search?

### BM25 (Best Matching 25)
- A ranking function for text retrieval
- Scores documents based on term frequency (TF) and inverse document frequency (IDF)
- Penalizes overly frequent words, normalizes by document length
- Used for exact keyword matching ("sparse" search)

### Hybrid Search
Combines two approaches:
1. **Dense Vector Search** — Semantic similarity via embeddings
2. **Sparse BM25 Search** — Exact keyword matching

**Why Hybrid?**
- Vector search excels at semantic meaning but misses exact keywords
- BM25 catches specific terms, names, IDs that vectors miss
- Combined: Best of both worlds

### Score Fusion Methods
```
// Reciprocal Rank Fusion (RRF)
combined_score = 1/(k + rank_vector) + 1/(k + rank_bm25)

// Weighted Sum
combined_score = 0.6 * bm25_score + 0.4 * vector_similarity
```

---

## Industry Analysis

| Database | BM25 Support | Release |
|:---------|:-------------|:--------|
| Milvus 2.5 | Sparse-BM25, 30x faster | Dec 2024 |
| Weaviate | BM25/BM25F + RRF | 2024 |
| VectorChord | Postgres BM25 extension | May 2025 |
| Qdrant | Sparse vectors | 2024 |
| pgvector | Via tsvector + hybrid | 2024 |

**Trend:** All major vector databases added hybrid search in 2024-2025.

---

## Technical Analysis for EdgeVec

### What BM25 Requires

1. **Tokenization** — Split text into terms
2. **Inverted Index** — Map terms → document IDs
3. **Document Length Storage** — For normalization
4. **IDF Computation** — Global term frequency stats

### EdgeVec's Current Design

| Component | Current | For BM25 |
|:----------|:--------|:---------|
| Data stored | Vectors + metadata | Would need raw text |
| Index type | HNSW (dense) | Need inverted index (sparse) |
| Bundle size | 477 KB | +100-200 KB for tokenizer |
| Focus | Similarity search | Would add keyword search |

### Implementation Options

**Option A: Full BM25 Integration**
- Add inverted index alongside HNSW
- Store raw text in metadata or separate field
- Implement tokenization (or use external)
- Bundle size: +150 KB
- Complexity: HIGH
- Target: v0.9.0+

**Option B: Sparse Vector Support**
- Allow users to store sparse vectors (like Milvus Sparse-BM25)
- User computes BM25 vectors externally
- EdgeVec stores and searches sparse vectors
- Bundle size: +50 KB
- Complexity: MEDIUM
- Target: v0.8.0

**Option C: Documentation-Only**
- Document how to combine EdgeVec with client-side BM25
- User uses `rank-bm25` (Python) or similar
- EdgeVec handles vector search, user handles BM25
- Bundle size: 0
- Complexity: LOW
- Target: v0.7.0

---

## Recommendation

### v0.7.0: Option C (Documentation)

EdgeVec's core value is WASM-native vector search with small bundle size. Adding BM25 would:
- Increase bundle size by 30-40%
- Add complexity outside our core focus
- Require text storage (not just vectors)

**For v0.7.0:** Document hybrid search pattern:

```javascript
import init, { EdgeVec } from 'edgevec';
import BM25 from 'bm25';  // Or rank-bm25, etc.

// Vector search
const vectorResults = db.searchWithFilter(embedding, filter, 20);

// BM25 search (user handles)
const bm25Results = bm25.search(query, 20);

// Reciprocal Rank Fusion
const combined = reciprocalRankFusion(vectorResults, bm25Results, 10);
```

### v0.8.0: Consider Option B (Sparse Vectors)

If demand continues, add sparse vector support:
- Store sparse vectors (term → weight)
- Search with dot product
- User computes BM25 vectors externally

### v0.9.0+: Consider Option A (Full BM25)

If EdgeVec grows and bundle size is less critical:
- Full BM25 integration
- Built-in tokenization
- Native hybrid search API

---

## Response to Lucas

**Suggested Reply:**

> Hi Lucas! Thanks for the feedback and for catching the broken link (fixed now!).
>
> **BM25/Hybrid Search:**
> It's not in v0.7.0 (releasing soon) but it's on the radar. EdgeVec focuses on keeping the bundle small (~470KB) and BM25 adds significant weight.
>
> **Current workaround:**
> You can combine EdgeVec with a JS BM25 library (like `bm25` or `rank-bm25`) and use Reciprocal Rank Fusion:
>
> ```javascript
> const vectorResults = db.search(embedding, 20);
> const bm25Results = bm25.search(query, 20);
> const combined = rrf(vectorResults, bm25Results);
> ```
>
> **Roadmap:**
> - v0.8.0: Considering sparse vector support (you'd compute BM25 vectors externally)
> - v0.9.0+: Possibly full BM25 integration if demand is high
>
> Would a "Hybrid Search Guide" in the docs be helpful for now?
>
> Thanks for using EdgeVec!

---

## Decision

| Version | BM25 Support | Details |
|:--------|:-------------|:--------|
| v0.7.0 | No | Focus on SIMD + Filter Playground |
| v0.8.0 | Sparse vectors (optional) | User computes BM25 externally |
| v0.9.0+ | Full BM25 (if demand) | Built-in tokenization + hybrid |

---

## References

- [Milvus 2.5 Hybrid Search](https://www.globenewswire.com/news-release/2024/12/17/2998318/0/en/Milvus-2-5-Creates-the-Best-of-Both-Worlds-With-Hybrid-Vector-Keyword-Search.html)
- [Weaviate Hybrid Search](https://weaviate.io/blog/hybrid-search-explained)
- [VectorChord BM25](https://blog.vectorchord.ai/hybrid-search-with-postgres-native-bm25-and-vectorchord)

---

**Status:** PROPOSED — Defer to v0.8.0+ planning

