# January Announcement Preparation

**Target Date:** ~January 20, 2026 (aligned with v0.6.0 Week 29 release)
**Status:** DRAFT
**Author:** PLANNER

---

## Announcement Strategy

### Timing

- **Target:** January 17, 2026 (Friday, post-v0.6.0 release, optimal HN posting time)
- **Backup:** January 20, 2026 (Monday)
- **Prerequisite:** v0.6.0 released (Week 29 target: Jan 16-23, 2026)
- **Note:** Date aligned with v0.6.0 release timeline per RFC-002 Implementation Plan

### Platforms

| Platform | Priority | Format |
|:---------|:---------|:-------|
| Hacker News | PRIMARY | "Show HN" post |
| Reddit r/rust | HIGH | Text post |
| Reddit r/javascript | HIGH | Text post |
| Twitter/X | MEDIUM | Thread |
| Dev.to | MEDIUM | Article |
| LinkedIn | LOW | Short post |

---

## Announcement Angle

### Headline Options

1. **"EdgeVec: The first WASM vector database with SQL-like filtering"**
2. **"EdgeVec v0.6: 32x memory reduction with Binary Quantization"**
3. **"Show HN: EdgeVec – Embedded vector search in the browser (Rust/WASM)"**

### Key Differentiators

| Feature | EdgeVec | Competitors |
|:--------|:--------|:------------|
| Runs in browser | Native WASM | Server-only |
| No external deps | Self-contained | Requires server |
| Filter expressions | SQL-like syntax | Limited or none |
| Memory efficient | SQ8 (4x), BQ (32x) | F32 only |
| IndexedDB persistence | Built-in | Manual setup |
| Open source | MIT/Apache-2.0 | Often proprietary |

---

## Draft Announcement

### Show HN Version

```
Show HN: EdgeVec – Embedded vector search in the browser (Rust/WASM)

Hi HN! I've been working on EdgeVec, an embedded vector database that runs
entirely in the browser via WebAssembly.

Key features:
- HNSW indexing with 10ms search on 100K vectors
- SQL-like filtering: category = "books" AND price < 50
- Binary Quantization: 32x memory reduction (coming in v0.6)
- IndexedDB persistence with atomic snapshots
- Zero dependencies, <500KB bundle

Why I built this:
Vector search usually means spinning up a server (Pinecone, Qdrant, etc.).
I wanted something that works offline, respects privacy, and runs in
constrained environments like mobile browsers.

Use cases:
- Local-first semantic search
- Privacy-preserving RAG applications
- Offline-capable AI features
- Edge computing with embeddings

Try it: [Live demo link]
GitHub: https://github.com/[user]/edgevec
npm: https://www.npmjs.com/package/edgevec
crates.io: https://crates.io/crates/edgevec

Looking for feedback on the API design and any edge cases I might have missed!
```

### Reddit r/rust Version

```
[ANN] EdgeVec 0.6 - Embedded vector database in Rust with WASM support

Just released EdgeVec 0.6, a high-performance vector database designed
for browser and embedded environments.

Built with:
- Rust + wasm-bindgen
- HNSW graph (custom implementation)
- SIMD acceleration (runtime detection)
- no_std compatible core

New in v0.6:
- Integrated metadata storage
- SQL-like filter expressions
- Binary Quantization (32x memory reduction)
- Persistence format v0.4

The Rust API:
```rust
let mut index = HnswIndex::new(HnswConfig::default());
let id = index.insert_with_metadata(
    &mut storage,
    &vector,
    HashMap::from([("category", "books"), ("price", 29.99)])
)?;

let results = index.search_filtered(
    &storage,
    &query,
    "category = 'books' AND price < 50",
    10
)?;
```

GitHub: https://github.com/[user]/edgevec
Docs: [link]

Happy to discuss the implementation – especially the HNSW layer and
WASM optimization decisions!
```

---

## Metrics to Include

### Current Stats (as of Dec 20, 2025)

| Metric | Value | Source |
|:-------|:------|:-------|
| npm downloads (weekly) | TBD | npm stats |
| npm downloads (total) | TBD | npm stats |
| GitHub stars | TBD | GitHub API |
| crates.io downloads | TBD | crates.io |
| Open issues | TBD | GitHub |
| Closed issues | TBD | GitHub |

### Performance Claims (Verified)

| Metric | Value | Verification |
|:-------|:------|:-------------|
| Search latency (100K vectors) | <10ms P99 | Benchmarks |
| Insert latency | <2ms mean | Benchmarks |
| Memory per vector (SQ8) | ~100 bytes | Calculated |
| Memory per vector (BQ) | ~100 bytes | HYPOTHESIS |
| Bundle size | <500KB | wasm-pack output |
| Load time (100K vectors) | <500ms | Benchmarks |

---

## Pre-Announcement Checklist

### Code

- [ ] v0.6.0 released to crates.io
- [ ] v0.6.0 released to npm
- [ ] All tests passing
- [ ] No critical bugs in issue tracker
- [ ] Demo site updated

### Documentation

- [ ] README updated with v0.6 features
- [ ] API documentation complete
- [ ] Getting started guide works
- [ ] Examples run without errors

### Demo

- [ ] Live demo deployed
- [ ] Filter playground updated
- [ ] Mobile compatibility tested
- [ ] Performance numbers updated

### Social

- [ ] Twitter/X thread drafted
- [ ] Dev.to article drafted
- [ ] LinkedIn post drafted
- [ ] Response templates ready for FAQs

---

## FAQ Preparation

### "How does this compare to Pinecone/Qdrant/Weaviate?"

> EdgeVec is designed for a different use case: running entirely in the browser
> or on the edge, with no server required. For server-side workloads with
> billions of vectors, use Pinecone/Qdrant/Weaviate. For client-side,
> offline-capable, privacy-preserving use cases, EdgeVec is built for you.

### "What about memory limits in browsers?"

> We've designed for constrained environments:
> - Scalar Quantization (SQ8): 4x memory reduction
> - Binary Quantization (BQ): 32x memory reduction
> - Mobile Safari tested: 500K vectors safely, 1M on desktop
> - Memory pressure monitoring API for graceful degradation

### "How do I generate embeddings?"

> EdgeVec is embedding-agnostic. Use:
> - Transformers.js for local embedding
> - OpenAI/Cohere/Anthropic APIs
> - Any service that produces float32 vectors
>
> See our Embedding Integration Guide: [link]

### "Is this production-ready?"

> Current status: Beta (v0.6). Used in production by [X] projects.
> - Extensive property-based testing
> - Fuzz testing for all parsers
> - Miri validation for unsafe code
> - No known critical bugs

---

## Post-Announcement Tasks

- [ ] Monitor HN comments, respond promptly
- [ ] Track Reddit engagement
- [ ] Collect feedback and issues
- [ ] Schedule follow-up posts (1 week, 1 month)
- [ ] Update roadmap based on feedback

---

## Schedule Reminder

```
January 15: Final review of announcement text (post v0.6.0 release)
January 16: Test all demo links
January 17 (9am PT): Post to Hacker News
January 17 (10am PT): Post to Reddit r/rust
January 17 (11am PT): Post to Reddit r/javascript
January 17 (12pm PT): Twitter thread
January 17-18: Monitor and respond
```

**Note:** Schedule aligned with v0.6.0 Week 29 release (Jan 16-23, 2026)

---

*Status: DRAFT*
*Review: Week 29 (after v0.6.0 release)*

