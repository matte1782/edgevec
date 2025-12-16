# EdgeVec Performance Tuning Guide

**Version:** 0.3.0
**Audience:** Developers optimizing EdgeVec for specific workloads
**Prerequisites:** Basic EdgeVec usage (see [Tutorial](./TUTORIAL.md))

---

## Table of Contents

1. [Overview](#overview)
2. [Key Performance Metrics](#key-performance-metrics)
3. [HNSW Parameters](#hnsw-parameters)
   - [M (Max Connections)](#m-max-connections)
   - [efConstruction (Build-time Search Width)](#efconstruction-build-time-search-width)
   - [efSearch (Query-time Search Width)](#efsearch-query-time-search-width)
   - [M0 (Layer 0 Connections)](#m0-layer-0-connections)
4. [Memory vs Speed Tradeoffs](#memory-vs-speed-tradeoffs)
5. [Recommended Configurations](#recommended-configurations)
6. [Benchmarking Your Workload](#benchmarking-your-workload)
7. [Advanced Tuning](#advanced-tuning)

---

## Overview

EdgeVec uses the **HNSW (Hierarchical Navigable Small World)** algorithm for approximate nearest neighbor search. HNSW builds a multi-layer graph structure that enables logarithmic-time search complexity.

**Performance can be tuned by adjusting:**
- Graph connectivity (M, M0)
- Build quality (efConstruction)
- Search accuracy (efSearch)

**Key tradeoffs:**
- Higher accuracy = slower search + more memory
- Faster index build = lower quality graph
- More connections = better recall but more memory

---

## Key Performance Metrics

Understanding these metrics helps you tune effectively:

| Metric | Description | How to Measure |
|:-------|:------------|:---------------|
| **Search Latency** | Time to find k nearest neighbors | `performance.now()` around `search()` |
| **Insert Latency** | Time to add a vector | `performance.now()` around `insert()` |
| **Recall@k** | % of true neighbors found vs brute force | Compare to exact search |
| **Memory Usage** | RAM consumed by index | Browser DevTools / `process.memoryUsage()` |
| **Build Time** | Time to index N vectors | Time batch insert operations |

### EdgeVec Performance Targets

| Metric | Target | Typical Performance |
|:-------|:-------|:--------------------|
| Search (100k vectors, k=10) | <1ms | 0.23-0.57ms |
| Insert (single vector) | <2ms | 0.5-1.5ms |
| Recall@10 | >95% | 97-99% at default settings |
| Memory (100k, 768d, quantized) | <100MB | ~83MB |

---

## HNSW Parameters

### M (Max Connections)

**What it does:** Controls how many neighbors each node connects to in layers above layer 0.

**Default:** 16

```javascript
const config = new EdgeVecConfig(768);
config.m = 16;  // Default
```

**Impact:**

| M Value | Memory | Recall | Search Speed | Build Speed |
|:--------|:-------|:-------|:-------------|:------------|
| 8 | Low | ~93% | Fast | Fast |
| 16 | Medium | ~97% | Medium | Medium |
| 32 | High | ~99% | Slower | Slower |
| 64 | Very High | ~99.5% | Slowest | Slowest |

**Guidelines:**
- **M = 8-12:** Memory-constrained, acceptable accuracy loss
- **M = 16:** General purpose (recommended default)
- **M = 24-32:** High accuracy requirements
- **M = 48-64:** Maximum accuracy, memory not a concern

**Memory formula (approximate):**
```
Memory per vector ≈ dimensions × 4 bytes + M × 2 × 8 bytes
```

**Example:**
```javascript
// Memory-optimized configuration
const config = new EdgeVecConfig(768);
config.m = 8;  // ~15% less memory vs M=16

// Accuracy-optimized configuration
const config = new EdgeVecConfig(768);
config.m = 32;  // ~99% recall, 2x memory vs M=16
```

---

### efConstruction (Build-time Search Width)

**What it does:** Controls the search width during index construction. Higher values build a higher-quality graph but take longer.

**Default:** 200

```javascript
const config = new EdgeVecConfig(768);
config.ef_construction = 200;  // Default
```

**Impact:**

| efConstruction | Build Time | Graph Quality | Recall |
|:---------------|:-----------|:--------------|:-------|
| 100 | Fast | Lower | ~95% |
| 200 | Medium | Good | ~97% |
| 400 | Slow | High | ~99% |
| 500+ | Very Slow | Maximum | ~99%+ |

**Guidelines:**
- **efConstruction = 100:** Rapid prototyping, frequent rebuilds
- **efConstruction = 200:** General purpose (recommended)
- **efConstruction = 400:** Production systems requiring high recall
- **efConstruction ≥ 500:** Diminishing returns, rarely needed

**Rule of thumb:** efConstruction should be at least 2× M for good graph quality.

**Example:**
```javascript
// Fast build for development
const config = new EdgeVecConfig(768);
config.m = 16;
config.ef_construction = 100;

// High-quality build for production
const config = new EdgeVecConfig(768);
config.m = 16;
config.ef_construction = 400;
```

---

### efSearch (Query-time Search Width)

**What it does:** Controls how many candidates are explored during search. Higher values improve recall at the cost of latency.

**Default:** 50

```javascript
const config = new EdgeVecConfig(768);
config.ef_search = 50;  // Default
```

**Impact:**

| efSearch | Recall@10 | Search Latency | Notes |
|:---------|:----------|:---------------|:------|
| 10 | ~85% | Very Fast | Too low for most uses |
| 50 | ~97% | Fast | Good default |
| 100 | ~99% | Medium | High accuracy |
| 200 | ~99.5% | Slower | Near-exact |
| 500 | ~99.9% | Slow | Maximum recall |

**Guidelines:**
- **efSearch must be ≥ k** (the number of results requested)
- Start with efSearch = 50, increase if recall is insufficient
- For real-time applications, keep efSearch ≤ 100
- For batch processing, efSearch = 200-500 is acceptable

**Example:**
```javascript
const config = new EdgeVecConfig(768);
config.ef_search = 50;  // Set default

const index = new EdgeVec(config);

// Search uses the configured ef_search
const results = index.search(query, 10);  // Uses ef_search=50
```

**Dynamic efSearch adjustment pattern:**
```javascript
// Real-time search: fast but lower recall
config.ef_search = 50;

// Re-ranking pass: higher recall
config.ef_search = 200;
const candidates = index.search(query, 100);

// Note: ef_search must be set before creating the index
// There is currently no way to change it per-query
```

---

### M0 (Layer 0 Connections)

**What it does:** Controls the maximum connections in layer 0 (the bottom, densest layer). Higher M0 improves recall but increases memory.

**Default:** 32 (typically 2× M)

```javascript
const config = new EdgeVecConfig(768);
config.m0 = 32;  // Default
```

**Guidelines:**
- Usually set to 2× M
- Increasing M0 beyond 2×M has diminishing returns
- Only adjust if you've tuned M and ef values first

**Example:**
```javascript
// High-recall configuration
const config = new EdgeVecConfig(768);
config.m = 24;
config.m0 = 48;  // 2× M for consistency
```

---

## Memory vs Speed Tradeoffs

### Memory Estimation

```
Memory per vector ≈
    (dimensions × 4 bytes)           // Vector storage
  + (M × 2 × 4 bytes)                // Upper layer neighbors
  + (M0 × 4 bytes)                   // Layer 0 neighbors
  + 16 bytes                         // Node metadata
```

**Example for 768-dimensional vectors with M=16, M0=32:**
```
Memory per vector ≈ 768×4 + 16×2×4 + 32×4 + 16 = 3,216 bytes
100k vectors ≈ 307 MB
```

### Tradeoff Matrix

| Goal | M | efConstruction | efSearch | Result |
|:-----|:--|:---------------|:---------|:-------|
| **Minimum Memory** | 8 | 100 | 30 | ~25% less memory, ~93% recall |
| **Balanced** | 16 | 200 | 50 | Default, ~97% recall |
| **Maximum Recall** | 32 | 400 | 200 | 2× memory, ~99.5% recall |
| **Fastest Search** | 12 | 200 | 30 | 20% faster search, ~95% recall |

---

## Recommended Configurations

### Use Case 1: Real-time Semantic Search

Low-latency search for user-facing applications.

```javascript
const config = new EdgeVecConfig(768);  // sentence-transformers dimension
config.metric = 'cosine';
config.m = 16;
config.ef_construction = 200;
config.ef_search = 50;

// Expected performance:
// - Search: <0.5ms at 100k vectors
// - Recall: ~97%
// - Memory: ~3KB per vector
```

### Use Case 2: High-Accuracy Recommendation System

When recall matters more than speed.

```javascript
const config = new EdgeVecConfig(384);  // smaller embeddings
config.metric = 'cosine';
config.m = 32;
config.ef_construction = 400;
config.ef_search = 200;

// Expected performance:
// - Search: ~1-2ms at 100k vectors
// - Recall: ~99.5%
// - Memory: ~2.5KB per vector
```

### Use Case 3: Memory-Constrained Edge Device

Mobile browser or IoT device with limited RAM.

```javascript
const config = new EdgeVecConfig(128);  // smaller dimension helps
config.metric = 'l2';
config.m = 8;
config.ef_construction = 100;
config.ef_search = 30;

// Expected performance:
// - Search: <0.2ms at 50k vectors
// - Recall: ~93%
// - Memory: <1KB per vector
```

### Use Case 4: Large-Scale Batch Processing

Offline processing where latency doesn't matter.

```javascript
const config = new EdgeVecConfig(1536);  // OpenAI embeddings
config.metric = 'cosine';
config.m = 24;
config.ef_construction = 400;
config.ef_search = 500;

// Expected performance:
// - Search: ~5ms at 1M vectors
// - Recall: ~99.9%
// - Memory: ~5KB per vector
```

### Use Case 5: Rapid Prototyping

Quick iteration during development.

```javascript
const config = new EdgeVecConfig(768);
config.metric = 'cosine';
config.m = 12;
config.ef_construction = 100;  // Faster builds
config.ef_search = 50;

// Expected performance:
// - Faster index builds for iteration
// - Acceptable recall for testing
```

---

## Benchmarking Your Workload

### Basic Benchmarking Script

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function benchmark() {
    await init();

    const dimensions = 768;
    const numVectors = 10000;
    const numQueries = 100;
    const k = 10;

    // Create index
    const config = new EdgeVecConfig(dimensions);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Generate test data
    const vectors = Array.from({ length: numVectors }, () =>
        new Float32Array(dimensions).map(() => Math.random() - 0.5)
    );

    const queries = Array.from({ length: numQueries }, () =>
        new Float32Array(dimensions).map(() => Math.random() - 0.5)
    );

    // Benchmark insertion
    console.log(`Inserting ${numVectors} vectors...`);
    const insertStart = performance.now();
    const result = index.insertBatch(vectors);
    const insertTime = performance.now() - insertStart;

    console.log(`Insert time: ${insertTime.toFixed(1)}ms`);
    console.log(`Insert rate: ${(numVectors / insertTime * 1000).toFixed(0)} vectors/sec`);

    // Benchmark search
    console.log(`\nSearching ${numQueries} queries (k=${k})...`);
    const searchTimes = [];

    for (const query of queries) {
        const start = performance.now();
        index.search(query, k);
        searchTimes.push(performance.now() - start);
    }

    // Calculate statistics
    searchTimes.sort((a, b) => a - b);
    const mean = searchTimes.reduce((a, b) => a + b) / searchTimes.length;
    const p50 = searchTimes[Math.floor(searchTimes.length * 0.5)];
    const p95 = searchTimes[Math.floor(searchTimes.length * 0.95)];
    const p99 = searchTimes[Math.floor(searchTimes.length * 0.99)];

    console.log(`Search latency:`);
    console.log(`  Mean: ${mean.toFixed(3)}ms`);
    console.log(`  P50:  ${p50.toFixed(3)}ms`);
    console.log(`  P95:  ${p95.toFixed(3)}ms`);
    console.log(`  P99:  ${p99.toFixed(3)}ms`);
}

benchmark().catch(console.error);
```

### Measuring Recall

```javascript
// Compare EdgeVec results to brute-force search
function bruteForceSearch(vectors, query, k, metric = 'cosine') {
    const distances = vectors.map((vec, idx) => ({
        id: idx,
        distance: calculateDistance(vec, query, metric)
    }));
    distances.sort((a, b) => a.distance - b.distance);
    return distances.slice(0, k);
}

function calculateDistance(a, b, metric) {
    if (metric === 'cosine') {
        let dot = 0, normA = 0, normB = 0;
        for (let i = 0; i < a.length; i++) {
            dot += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        return 1 - dot / (Math.sqrt(normA) * Math.sqrt(normB));
    }
    // Add l2, dot implementations as needed
}

function calculateRecall(edgevecResults, bruteForceResults) {
    const bfIds = new Set(bruteForceResults.map(r => r.id));
    const hits = edgevecResults.filter(r => bfIds.has(r.id)).length;
    return hits / bruteForceResults.length;
}

// Usage
const edgevecResults = index.search(query, k);
const exactResults = bruteForceSearch(vectors, query, k);
const recall = calculateRecall(edgevecResults, exactResults);
console.log(`Recall@${k}: ${(recall * 100).toFixed(1)}%`);
```

---

## Advanced Tuning

### Parameter Sweep

Test different configurations to find optimal settings:

```javascript
const configs = [
    { m: 8, ef_c: 100, ef_s: 30 },
    { m: 16, ef_c: 200, ef_s: 50 },
    { m: 16, ef_c: 200, ef_s: 100 },
    { m: 32, ef_c: 400, ef_s: 100 },
];

for (const cfg of configs) {
    const config = new EdgeVecConfig(768);
    config.m = cfg.m;
    config.ef_construction = cfg.ef_c;
    config.ef_search = cfg.ef_s;

    // Run benchmark...
    console.log(`M=${cfg.m}, efC=${cfg.ef_c}, efS=${cfg.ef_s}`);
    console.log(`  Latency: ${latency}ms, Recall: ${recall}%`);
}
```

### When to Rebuild

Consider rebuilding your index when:
- You change M or efConstruction
- Tombstone ratio exceeds 30-50% (use `compact()`)
- You've identified better parameters for your workload

### Monitoring in Production

Key metrics to track:
- Search latency percentiles (P50, P95, P99)
- Tombstone ratio (call `tombstoneRatio()`)
- Memory usage growth
- Recall (if you can sample and verify)

```javascript
// Periodic health check
setInterval(() => {
    const ratio = index.tombstoneRatio();
    if (ratio > 0.3) {
        console.warn(`High tombstone ratio: ${(ratio*100).toFixed(1)}%`);
    }
}, 60000);
```

---

## Quick Reference

| Parameter | Default | Low | Medium | High |
|:----------|:--------|:----|:-------|:-----|
| M | 16 | 8 | 16 | 32-64 |
| efConstruction | 200 | 100 | 200 | 400-500 |
| efSearch | 50 | 30 | 50-100 | 200-500 |
| M0 | 32 | 16 | 32 | 64-128 |

**Rules of thumb:**
- M0 ≈ 2 × M
- efConstruction ≥ 2 × M
- efSearch ≥ k (number of results)
- For 99%+ recall: M=32, efConstruction=400, efSearch=200

---

## See Also

- [Tutorial](./TUTORIAL.md) — Getting started
- [Troubleshooting](./TROUBLESHOOTING.md) — Common issues
- [API Reference](./API_REFERENCE.md) — Full API documentation
- [Competitive Analysis](./benchmarks/competitive_analysis.md) — EdgeVec vs alternatives
