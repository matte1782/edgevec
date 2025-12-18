# EdgeVec vs Alternatives

**Version:** 1.0.0
**Last Updated:** 2025-12-19

This document provides an honest comparison between EdgeVec and alternative vector search solutions to help you make an informed decision.

---

## Quick Decision Guide

### Choose EdgeVec if:

- **You need vector search in the browser** — EdgeVec runs entirely client-side
- **Data privacy is important** — No data leaves the user's device
- **Offline capability required** — Works without network connectivity
- **No server infrastructure wanted** — Zero backend dependencies
- **You need filtering** — SQL-like metadata filtering not available in other WASM solutions
- **Bundle size matters** — 227 KB gzipped is smallest in class

### Choose Pinecone/Qdrant/Weaviate if:

- **Scale beyond millions of vectors** — Server databases handle billions
- **Need managed infrastructure** — Fully managed cloud service
- **Multi-user access required** — Server databases support concurrent users
- **Advanced features needed** — Namespaces, collections, replication
- **Need hybrid search** — Combining vector + keyword search

### Choose hnswlib-node if:

- **Server-side Node.js only** — Don't need browser support
- **Maximum search speed** — Native C++ is ~4x faster than WASM
- **No filtering needed** — Only basic label filtering supported
- **No persistence needed** — In-memory only

### Choose voy if:

- **Batch-first workflow** — Optimized for bulk insert, infrequent search
- **Simplest API** — Minimal configuration
- **Search speed not critical** — 24x slower than EdgeVec

---

## Detailed Feature Comparison

### WASM Libraries (Client-Side)

| Feature | EdgeVec | hnswlib-wasm | voy |
|:--------|:-------:|:------------:|:---:|
| **Algorithm** | HNSW | HNSW | k-d tree |
| **Search Complexity** | O(log n) | O(log n) | O(n) at 128D+ |
| **Search P50 (10k)** | 0.20 ms | N/A* | 4.78 ms |
| **Insert P50 (10k)** | 0.83 ms | N/A* | 0.03 ms |
| **Memory (10k, 128D)** | 2.76 MB | N/A* | 47.10 MB |
| **Bundle Size** | 227 KB | ~300 KB | ~150 KB |
| **Metadata Filtering** | Yes (15 ops) | No | No |
| **SQL-like Queries** | Yes | No | No |
| **Soft Delete** | Yes | No | No |
| **Persistence** | IndexedDB | Manual | No |
| **Quantization** | SQ8 | No | No |
| **Active Development** | Yes (2025) | Stale (2023) | Stale (2023) |

*hnswlib-wasm benchmarks not available; hnswlib-node (native) is ~4x faster than EdgeVec.

### Server Databases

| Feature | EdgeVec | Pinecone | Qdrant | Weaviate | ChromaDB |
|:--------|:-------:|:--------:|:------:|:--------:|:--------:|
| **Deployment** | Client | Cloud | Self/Cloud | Self/Cloud | Self/Cloud |
| **Max Scale** | ~1M | Billions | Billions | Billions | Millions |
| **Filtering** | 15 ops | 20+ ops | 15+ ops | 20+ ops | 10+ ops |
| **Hybrid Search** | No | Yes | Yes | Yes | Yes |
| **Multi-tenancy** | No | Yes | Yes | Yes | Yes |
| **Replication** | No | Yes | Yes | Yes | Yes |
| **Pricing** | Free | $$$ | Free/$$$ | Free/$$$ | Free |
| **Latency** | <1ms | 10-50ms* | 1-10ms* | 1-10ms* | 1-10ms* |
| **Privacy** | Local | Cloud | Configurable | Configurable | Configurable |

*Network latency not included. Actual latency varies by configuration and network.

---

## Performance Comparison

### Search Latency (10k vectors, 128D, k=10)

| Solution | P50 | P99 | Notes |
|:---------|:----|:----|:------|
| hnswlib-node (C++) | 0.05 ms | 0.07 ms | Fastest, requires compilation |
| **EdgeVec (WASM)** | **0.20 ms** | **0.22 ms** | Fastest WASM solution |
| Pinecone (Cloud) | 10-20 ms | 30-50 ms | Includes network latency |
| voy (WASM) | 4.78 ms | 4.88 ms | k-d tree degrades at high D |

### Memory Usage (10k vectors, 128D)

| Solution | Memory | Notes |
|:---------|:-------|:------|
| **EdgeVec (Float32)** | 2.76 MB | Comparable to native |
| **EdgeVec (SQ8)** | 0.77 MB | 3.6x compression |
| hnswlib-node | 2.76 MB | Native memory |
| voy | 47.10 MB | Higher overhead |

### Insert Performance (10k vectors)

| Solution | P50 | Notes |
|:---------|:----|:------|
| voy | 0.03 ms | Batch-optimized |
| **EdgeVec** | 0.83 ms | Incremental HNSW |
| hnswlib-node | 1.56 ms | Incremental HNSW |

---

## When EdgeVec Excels

### 1. Privacy-First Applications

EdgeVec processes all data locally. Use cases:

- Personal knowledge bases
- Private document search
- Medical/legal document retrieval
- Offline-first mobile apps

### 2. Edge Computing

EdgeVec runs anywhere JavaScript runs:

- Browser extensions
- Electron apps
- React Native (via JSI)
- Cloudflare Workers
- Deno Deploy

### 3. Cost-Sensitive Deployments

EdgeVec has zero infrastructure costs:

- No server hosting
- No database fees
- No API calls
- Users provide compute

### 4. Filtered Search in Browser

EdgeVec is the **only** WASM library with SQL-like filtering:

```javascript
// Only EdgeVec supports this in the browser
const results = search(query, 10);
const filtered = results.filter(r =>
    parseFilter('category = "tech" AND price < 100')(metadata[r.id])
);
```

---

## EdgeVec Limitations

### Not Suitable For:

| Limitation | Reason | Alternative |
|:-----------|:-------|:------------|
| **Billion+ vectors** | Browser memory limits (~1GB) | Pinecone, Qdrant |
| **Multi-user access** | Single-tab design | Any server database |
| **Distributed systems** | Local-only | Qdrant, Weaviate |
| **Hybrid search** | Vector-only | Weaviate, Elasticsearch |
| **Real-time sync** | No built-in sync | Firebase + EdgeVec |

### Technical Constraints:

| Constraint | Value | Notes |
|:-----------|:------|:------|
| Max vectors (practical) | ~1M | Browser memory dependent |
| Max dimensions | 4096 | Tested up to 4096D |
| Max concurrent indexes | 1 per tab | Browser tab isolation |
| Persistence backend | IndexedDB | ~2GB browser limit |

---

## Migration Paths

### From hnswlib

```javascript
// hnswlib
const index = new HierarchicalNSW('cosine', 128);
index.initIndex(maxElements);
index.addPoint(vector, label);
const result = index.searchKnn(query, k);

// EdgeVec
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);
const id = index.insert(vector);
const results = index.search(query, k);
```

Key differences:
- EdgeVec auto-manages capacity
- EdgeVec returns `{id, score}` objects
- EdgeVec supports persistence via `save()`/`load()`

### From Pinecone

```javascript
// Pinecone
const index = pinecone.index('my-index');
await index.upsert([{ id, values, metadata }]);
const results = await index.query({
    vector: query,
    topK: k,
    filter: { category: { $eq: 'books' } }
});

// EdgeVec
const index = new EdgeVec(config);
const id = index.insert(vector);
metadata[id] = { category: 'books' };
const results = index.search(query, k);
const filtered = results.filter(r =>
    parseFilter('category = "books"')(metadata[r.id])
);
```

Key differences:
- EdgeVec is local, no network calls
- EdgeVec uses SQL-like filter syntax
- Metadata stored separately in EdgeVec v0.5

---

## Recommendation Matrix

| Use Case | Recommended | Why |
|:---------|:------------|:----|
| Browser semantic search | **EdgeVec** | Only viable option with filtering |
| Electron app | **EdgeVec** | Local, fast, persistent |
| Node.js backend (speed) | hnswlib-node | Native is 4x faster |
| Node.js backend (features) | Qdrant | Full database features |
| Cloud SaaS | Pinecone | Managed, scalable |
| Self-hosted | Qdrant/Weaviate | Full control |
| Mobile (React Native) | **EdgeVec** | Works via JSI bridge |
| Serverless (Cloudflare) | **EdgeVec** | WASM native |

---

## Conclusion

EdgeVec fills a unique niche: **client-side vector database with database-grade features**. It's not trying to compete with Pinecone at billion scale or hnswlib-node at raw speed. Instead, it brings vector database capabilities to environments where server databases can't go.

**Choose EdgeVec when privacy, offline capability, or zero-infrastructure deployment matters more than absolute scale or raw speed.**

---

*Last benchmarked: 2025-12-18*
*EdgeVec version: 0.5.0*
*[Full benchmark methodology](benchmarks/competitive_analysis_v2.md)*
