# EdgeVec Tutorial: Getting Started

**Version:** 0.3.0
**Audience:** Developers new to EdgeVec or vector databases
**Prerequisites:** Basic JavaScript/TypeScript knowledge, Node.js 18+ or modern browser

---

## Table of Contents

1. [What is EdgeVec?](#what-is-edgevec)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Your First Vector Index](#your-first-vector-index)
5. [Inserting Vectors](#inserting-vectors)
6. [Searching for Similar Vectors](#searching-for-similar-vectors)
7. [Persistence (Save/Load)](#persistence-saveload)
8. [Soft Delete and Compaction](#soft-delete-and-compaction)
9. [Batch Operations](#batch-operations)
10. [Distance Metrics](#distance-metrics)
11. [Next Steps](#next-steps)

---

## What is EdgeVec?

EdgeVec is an **embedded vector database** that runs entirely in your browser, Node.js, or edge device. Unlike cloud-based vector databases, EdgeVec:

- **Runs locally** — No network latency, works offline
- **Preserves privacy** — Your data never leaves the device
- **Sub-millisecond search** — 0.23ms at 100k vectors
- **Tiny bundle** — 213 KB gzipped

**Common use cases:**
- Semantic search in browser apps
- Document similarity matching
- Recommendation systems
- RAG (Retrieval-Augmented Generation) applications
- Offline-first AI features

---

## Prerequisites

### Environment Requirements

| Environment | Version Required |
|:------------|:-----------------|
| **Node.js** | 18.0 or higher |
| **Chrome** | 90 or higher |
| **Firefox** | 90 or higher |
| **Safari** | 15 or higher |
| **Edge** | 90 or higher |

### Knowledge Requirements

- Basic JavaScript/TypeScript
- Understanding of arrays and async/await
- (Optional) Familiarity with vectors and embeddings

### What You'll Need

- A code editor (VS Code, WebStorm, etc.)
- npm or yarn package manager
- A terminal/command prompt

---

## Installation

### npm (Node.js and bundlers)

```bash
npm install edgevec
```

### yarn

```bash
yarn add edgevec
```

### Browser (ES Modules via CDN)

```html
<script type="module">
import init, { EdgeVec, EdgeVecConfig } from 'https://unpkg.com/edgevec@0.3.0/edgevec.js';

async function main() {
    await init();
    // EdgeVec is ready to use
}
main();
</script>
```

### Verify Installation

Create a test file to verify EdgeVec is installed correctly:

```javascript
// test-install.js
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function test() {
    await init();
    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);
    console.log('EdgeVec installed successfully!');
    console.log('Index dimensions:', 128);
}

test().catch(console.error);
```

Run with: `node test-install.js`

**Expected output:**
```
EdgeVec installed successfully!
Index dimensions: 128
```

---

## Your First Vector Index

Let's create a simple vector index step by step.

### Step 1: Initialize WASM

EdgeVec is built with WebAssembly (WASM). Before using any EdgeVec functions, you must initialize the WASM module:

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    // REQUIRED: Initialize WASM module (call once at startup)
    await init();

    console.log('WASM initialized!');
}

main();
```

> **Important:** Always `await init()` before creating any EdgeVec objects. Forgetting this is the most common beginner mistake.

### Step 2: Create Configuration

The `EdgeVecConfig` specifies how your index will behave. The only required parameter is the **dimension** — the size of your vectors:

```javascript
// Create config for 128-dimensional vectors
const config = new EdgeVecConfig(128);

// (Optional) Set the distance metric
config.metric = 'cosine';  // Options: 'l2', 'cosine', 'dot'
```

**Choosing dimensions:**
- Your dimension must match your embedding model's output
- Common dimensions: 128, 256, 384, 512, 768, 1536
- Smaller = faster and less memory, larger = more expressive

### Step 3: Create the Index

```javascript
const index = new EdgeVec(config);

console.log('Index created!');
console.log('Vector count:', index.liveCount());  // 0
```

### Complete Example

Here's a complete working example:

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    // 1. Initialize WASM
    await init();

    // 2. Create configuration
    const config = new EdgeVecConfig(128);
    config.metric = 'cosine';

    // 3. Create index
    const index = new EdgeVec(config);

    console.log('EdgeVec index created successfully!');
    console.log('Configured dimensions: 128');
    console.log('Distance metric: cosine');
    console.log('Current vector count:', index.liveCount());
}

main().catch(console.error);
```

**Expected output:**
```
EdgeVec index created successfully!
Configured dimensions: 128
Distance metric: cosine
Current vector count: 0
```

---

## Inserting Vectors

Now let's add vectors to our index.

### Single Vector Insert

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Create a vector (must match config dimensions!)
    const vector = new Float32Array(128);
    for (let i = 0; i < 128; i++) {
        vector[i] = Math.random();  // Random values for demo
    }

    // Insert returns the assigned ID
    const id = index.insert(vector);
    console.log('Inserted vector with ID:', id);
    console.log('Total vectors:', index.liveCount());
}

main().catch(console.error);
```

**Expected output:**
```
Inserted vector with ID: 0
Total vectors: 1
```

### Multiple Inserts

```javascript
// Insert 100 random vectors
const ids = [];
for (let i = 0; i < 100; i++) {
    const vector = new Float32Array(128).map(() => Math.random());
    const id = index.insert(vector);
    ids.push(id);
}

console.log('Inserted 100 vectors');
console.log('IDs range:', ids[0], 'to', ids[ids.length - 1]);
console.log('Total count:', index.liveCount());
```

### Vector Requirements

| Requirement | Details |
|:------------|:--------|
| **Type** | Must be `Float32Array` |
| **Length** | Must match `config.dimensions` |
| **Values** | Finite numbers (no `NaN` or `Infinity`) |
| **Normalization** | Recommended for cosine metric |

### Handling Errors

```javascript
try {
    // This will fail - wrong dimensions
    const badVector = new Float32Array(256);  // Expected 128!
    index.insert(badVector);
} catch (e) {
    console.error('Insert failed:', e.message);
    // Output: "Insert failed: DimensionMismatch: expected 128, got 256"
}
```

---

## Searching for Similar Vectors

The core operation in a vector database is **similarity search** — finding vectors closest to a query.

### Basic Search

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    // Create index and add some vectors
    const config = new EdgeVecConfig(128);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Insert 100 random vectors
    for (let i = 0; i < 100; i++) {
        const vector = new Float32Array(128).map(() => Math.random());
        index.insert(vector);
    }

    // Create a query vector
    const query = new Float32Array(128).map(() => Math.random());

    // Search for 10 nearest neighbors
    const k = 10;
    const results = index.search(query, k);

    console.log('Search results:');
    for (const result of results) {
        console.log(`  ID: ${result.id}, Score: ${result.score.toFixed(4)}`);
    }
}

main().catch(console.error);
```

**Expected output (scores will vary):**
```
Search results:
  ID: 42, Score: 0.1234
  ID: 17, Score: 0.1456
  ID: 83, Score: 0.1589
  ...
```

### Understanding Results

The `search()` method returns an array of result objects:

```typescript
interface SearchResult {
    id: number;      // Vector ID (returned from insert)
    score: number;   // Distance/similarity score
}
```

**Score interpretation by metric:**

| Metric | Score Meaning | Best Score |
|:-------|:--------------|:-----------|
| `l2` | Euclidean distance | Lower = closer |
| `cosine` | Cosine distance (1 - similarity) | Lower = closer |
| `dot` | Negative dot product | Lower = closer |

### Search Parameters

```javascript
// Basic search
const results = index.search(query, 10);

// You can search for any k value
const top1 = index.search(query, 1);    // Just the closest
const top100 = index.search(query, 100); // Top 100
```

### Real-World Example: Finding Similar Documents

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    // Simulate document embeddings (768 dims like sentence-transformers)
    const config = new EdgeVecConfig(768);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Simulate: your embedding function would go here
    function fakeEmbed(text) {
        // In reality, use @xenova/transformers or similar
        return new Float32Array(768).map(() => Math.random());
    }

    // Index some "documents"
    const documents = [
        "EdgeVec is a fast vector database",
        "Machine learning is transforming technology",
        "JavaScript runs in web browsers",
        "WASM enables high performance in browsers",
        "Vector search powers semantic search"
    ];

    const docIds = {};
    for (const doc of documents) {
        const embedding = fakeEmbed(doc);
        const id = index.insert(embedding);
        docIds[id] = doc;
    }

    // Search for similar documents
    const queryText = "browser vector search performance";
    const queryEmbedding = fakeEmbed(queryText);
    const results = index.search(queryEmbedding, 3);

    console.log(`Query: "${queryText}"`);
    console.log('Most similar documents:');
    for (const result of results) {
        console.log(`  [Score: ${result.score.toFixed(3)}] ${docIds[result.id]}`);
    }
}

main().catch(console.error);
```

---

## Persistence (Save/Load)

EdgeVec can save your index to disk (Node.js) or IndexedDB (browser) for persistence across sessions.

### Saving Your Index

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Add some vectors
    for (let i = 0; i < 1000; i++) {
        const vector = new Float32Array(128).map(() => Math.random());
        index.insert(vector);
    }

    console.log('Vectors before save:', index.liveCount());

    // Save to storage
    // Browser: saves to IndexedDB
    // Node.js: saves to file system
    await index.save("my-vector-db");

    console.log('Index saved successfully!');
}

main().catch(console.error);
```

### Loading Your Index

```javascript
import init, { EdgeVec } from 'edgevec';

async function main() {
    await init();

    // Load previously saved index
    const index = await EdgeVec.load("my-vector-db");

    console.log('Index loaded successfully!');
    console.log('Vectors:', index.liveCount());

    // Now you can search
    const query = new Float32Array(128).map(() => Math.random());
    const results = index.search(query, 5);
    console.log('Search results:', results.length);
}

main().catch(console.error);
```

### Complete Save/Load Workflow

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

const DB_NAME = "tutorial-vectors";

async function createAndSaveIndex() {
    await init();

    const config = new EdgeVecConfig(128);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Add vectors
    for (let i = 0; i < 500; i++) {
        const vector = new Float32Array(128).map(() => Math.random());
        index.insert(vector);
    }

    await index.save(DB_NAME);
    console.log('Created and saved index with', index.liveCount(), 'vectors');
}

async function loadAndSearch() {
    await init();

    const index = await EdgeVec.load(DB_NAME);
    console.log('Loaded index with', index.liveCount(), 'vectors');

    const query = new Float32Array(128).map(() => Math.random());
    const results = index.search(query, 5);

    console.log('Search results:');
    results.forEach((r, i) => {
        console.log(`  ${i + 1}. ID: ${r.id}, Score: ${r.score.toFixed(4)}`);
    });
}

// Run workflow
async function main() {
    await createAndSaveIndex();
    await loadAndSearch();
}

main().catch(console.error);
```

### Storage Locations

| Environment | Storage Backend | Location |
|:------------|:----------------|:---------|
| Browser | IndexedDB | Browser's IndexedDB (per-origin) |
| Node.js | File System | Current working directory |

---

## Soft Delete and Compaction

EdgeVec supports **soft delete** — marking vectors as deleted without immediately removing them. This is much faster than rebuilding the entire index.

### Soft Delete

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Insert some vectors
    const ids = [];
    for (let i = 0; i < 100; i++) {
        const vector = new Float32Array(128).map(() => Math.random());
        ids.push(index.insert(vector));
    }

    console.log('Before delete:', index.liveCount(), 'live vectors');

    // Soft delete a vector (O(1) operation - very fast!)
    const deleted = index.softDelete(ids[0]);
    console.log('Deleted ID 0:', deleted);  // true

    // Check deletion status
    console.log('Is ID 0 deleted?', index.isDeleted(ids[0]));  // true
    console.log('Is ID 1 deleted?', index.isDeleted(ids[1]));  // false

    // Statistics
    console.log('Live count:', index.liveCount());      // 99
    console.log('Deleted count:', index.deletedCount()); // 1
    console.log('Tombstone ratio:', index.tombstoneRatio().toFixed(2));  // 0.01
}

main().catch(console.error);
```

**Expected output:**
```
Before delete: 100 live vectors
Deleted ID 0: true
Is ID 0 deleted? true
Is ID 1 deleted? false
Live count: 99
Deleted count: 1
Tombstone ratio: 0.01
```

### Batch Delete

For deleting multiple vectors efficiently:

```javascript
// Delete multiple vectors at once
const idsToDelete = new Uint32Array([1, 3, 5, 7, 9]);
const result = index.softDeleteBatch(idsToDelete);

console.log('Deleted:', result.deleted);
console.log('Already deleted:', result.alreadyDeleted);
console.log('Invalid IDs:', result.invalidIds);
```

### Compaction

When many vectors are deleted, space is wasted by tombstones. **Compaction** rebuilds the index, reclaiming this space:

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Insert 100 vectors and track their IDs
    const ids = [];
    for (let i = 0; i < 100; i++) {
        const vector = new Float32Array(128).map(() => Math.random());
        ids.push(index.insert(vector));
    }

    // Delete first 40 vectors using their actual IDs (40% tombstone ratio)
    for (let i = 0; i < 40; i++) {
        index.softDelete(ids[i]);
    }

    console.log('Before compaction:');
    console.log('  Live:', index.liveCount());
    console.log('  Deleted:', index.deletedCount());
    console.log('  Tombstone ratio:', (index.tombstoneRatio() * 100).toFixed(0) + '%');
    console.log('  Needs compaction:', index.needsCompaction());

    // Check for warning
    const warning = index.compactionWarning();
    if (warning) {
        console.log('  Warning:', warning);
    }

    // Compact if recommended
    // Note: See TROUBLESHOOTING.md for compact() safety considerations
    if (index.needsCompaction()) {
        const result = index.compact();
        console.log('\nCompaction complete:');
        console.log('  Tombstones removed:', result.tombstones_removed);
        console.log('  New size:', result.new_size);
        console.log('  Duration:', result.duration_ms.toFixed(1), 'ms');
    }

    console.log('\nAfter compaction:');
    console.log('  Live:', index.liveCount());
    console.log('  Deleted:', index.deletedCount());
}

main().catch(console.error);
```

**Expected output:**
```
Before compaction:
  Live: 60
  Deleted: 40
  Tombstone ratio: 40%
  Needs compaction: true
  Warning: Tombstone ratio (40.0%) exceeds threshold (30.0%). Consider calling compact().

Compaction complete:
  Tombstones removed: 40
  New size: 60
  Duration: 12.3 ms

After compaction:
  Live: 60
  Deleted: 0
```

### Compaction Threshold

By default, `needsCompaction()` returns `true` when 30% or more vectors are deleted. You can adjust this:

```javascript
// Get current threshold
console.log('Threshold:', index.compactionThreshold());  // 0.3

// Set to 50%
index.setCompactionThreshold(0.5);
```

---

## Batch Operations

For inserting many vectors efficiently, use the batch API.

### Batch Insert

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Prepare vectors as array of Float32Array
    const vectors = [];
    for (let i = 0; i < 1000; i++) {
        vectors.push(new Float32Array(128).map(() => Math.random()));
    }

    // Batch insert
    const result = index.insertBatch(vectors);

    console.log('Batch insert complete:');
    console.log('  Inserted:', result.inserted);
    console.log('  Total:', result.total);
    console.log('  First 5 IDs:', Array.from(result.ids.slice(0, 5)));
}

main().catch(console.error);
```

### Batch Insert with Progress

For long-running inserts, track progress with a callback:

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Prepare 5000 vectors
    const vectors = Array.from({ length: 5000 }, () =>
        new Float32Array(128).map(() => Math.random())
    );

    console.log('Starting batch insert...');
    const startTime = Date.now();

    // Insert with progress callback
    const result = index.insertBatchWithProgress(vectors, (done, total) => {
        const percent = Math.round((done / total) * 100);
        console.log(`Progress: ${percent}% (${done}/${total})`);
    });

    const elapsed = Date.now() - startTime;
    console.log(`\nComplete! Inserted ${result.inserted} vectors in ${elapsed}ms`);
    console.log(`Rate: ${(result.inserted / elapsed * 1000).toFixed(0)} vectors/sec`);
}

main().catch(console.error);
```

**Expected output:**
```
Starting batch insert...
Progress: 0% (0/5000)
Progress: 100% (5000/5000)

Complete! Inserted 5000 vectors in 234ms
Rate: 21367 vectors/sec
```

### Flat Array Format (Advanced)

For maximum efficiency with large batches, use the flat array format:

```javascript
// Flat format: all vectors concatenated
const dimensions = 128;
const count = 1000;
const flatVectors = new Float32Array(dimensions * count);

// Fill with random data
for (let i = 0; i < flatVectors.length; i++) {
    flatVectors[i] = Math.random();
}

// Insert using flat format
const ids = index.insertBatchFlat(flatVectors, count);
console.log('Inserted', ids.length, 'vectors');
```

---

## Distance Metrics

EdgeVec supports three distance metrics. Choose based on your use case.

### L2 (Euclidean Distance)

The straight-line distance between two points.

```javascript
const config = new EdgeVecConfig(128);
// config.metric = 'l2';  // This is the default
```

**Best for:**
- Image embeddings
- When magnitude matters
- General-purpose similarity

### Cosine

Measures the angle between vectors, ignoring magnitude.

```javascript
const config = new EdgeVecConfig(128);
config.metric = 'cosine';
```

**Best for:**
- Text embeddings (sentence-transformers, OpenAI, etc.)
- When only direction matters
- Normalized vectors

### Dot Product

Dot product similarity (returned as negative for min-distance semantics).

```javascript
const config = new EdgeVecConfig(128);
config.metric = 'dot';
```

**Best for:**
- Maximum inner product search (MIPS)
- Pre-normalized vectors
- Recommendation systems

### Choosing a Metric

| Embedding Source | Recommended Metric |
|:-----------------|:-------------------|
| OpenAI embeddings | `cosine` |
| sentence-transformers | `cosine` |
| CLIP image embeddings | `cosine` or `l2` |
| Custom embeddings | Depends on normalization |
| PCA-reduced vectors | `l2` |

---

## Next Steps

Congratulations! You now know the basics of EdgeVec. Here's what to explore next:

### Documentation

- **[Performance Tuning Guide](./PERFORMANCE_TUNING.md)** — Optimize HNSW parameters
- **[Troubleshooting Guide](./TROUBLESHOOTING.md)** — Common errors and solutions
- **[Integration Guide](./INTEGRATION_GUIDE.md)** — Use with transformers.js, TensorFlow.js, OpenAI
- **[API Reference](./API_REFERENCE.md)** — Complete API documentation

### Interactive Demos

Try EdgeVec in your browser:

- [Demo Catalog](../wasm/examples/index.html) — All interactive examples
- [Benchmark Dashboard](../wasm/examples/benchmark-dashboard.html) — Performance visualization
- [Soft Delete Demo](../wasm/examples/soft_delete.html) — Tombstone visualization

### Example Projects

1. **Semantic Search Engine** — Index documents and search by meaning
2. **Image Similarity** — Find similar images using CLIP embeddings
3. **RAG Application** — Build retrieval-augmented generation
4. **Recommendation System** — Suggest similar items

### Getting Help

- **Issues:** [GitHub Issues](https://github.com/matte1782/edgevec/issues)
- **Discussions:** [GitHub Discussions](https://github.com/matte1782/edgevec/discussions)

---

## Quick Reference

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

// Initialize
await init();

// Configure
const config = new EdgeVecConfig(128);
config.metric = 'cosine';  // 'l2', 'cosine', 'dot'

// Create index
const index = new EdgeVec(config);

// Insert
const vector = new Float32Array(128).fill(0.1);
const id = index.insert(vector);

// Search
const results = index.search(query, 10);

// Batch insert
const result = index.insertBatch(vectors);

// Soft delete
index.softDelete(id);

// Check status
index.isDeleted(id);
index.liveCount();
index.deletedCount();

// Compaction
if (index.needsCompaction()) {
    index.compact();
}

// Persistence
await index.save("my-db");
const loaded = await EdgeVec.load("my-db");
```

---

**Happy vector searching!**
