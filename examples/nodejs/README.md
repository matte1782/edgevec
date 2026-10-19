# EdgeVec Node.js Examples

This directory contains Node.js examples demonstrating EdgeVec vector database usage.

## Prerequisites

- Node.js >= 20.6.0 (the quickstart resolves the WASM file with `import.meta.resolve`)
- Built EdgeVec package (run `npm run build` from project root)

## Setup

From the `examples/nodejs` directory:

```bash
# Install dependencies (links to local EdgeVec build)
npm install
```

## Examples

### 1. Quick Start (`quickstart.js`)

Basic demonstration of EdgeVec API:
- Initializing the WASM package and creating an index
- Inserting vectors (synchronous)
- Searching for nearest neighbors (synchronous)
- Saving bytes to a Node.js file and restoring with `EdgeVec.fromBytes()`

**Run:**
```bash
npm run quickstart
```

**Expected Output:**
- Index dimensions and metric
- 3 vectors inserted with IDs
- Search results with distances
- Save/load verification

---

### 2. Performance Benchmark (`benchmark.js`)

Comprehensive performance test:
- Inserts 100,000 vectors
- Runs 1,000 search queries
- Measures P50/P95/P99 latencies
- Validates <10ms P99 search target

**Run:**
```bash
npm run benchmark
```

**Expected Output:**
- Insertion throughput (vectors/sec)
- Search latency percentiles
- PASS/FAIL verdict for P99 <10ms target
- Persistence timing

**Note:** This benchmark may take 1-2 minutes to complete depending on your hardware.

---

## API Reference

### EdgeVecClient

```javascript
import { EdgeVecClient } from '@edgevec/core';

// Create (async - initializes WASM)
const client = await EdgeVecClient.create({
    dimensions: 128,
    metric: 'cosine' // 'l2', 'cosine', or 'dot'
});

// Insert (synchronous)
const vector = new Float32Array(128).fill(0.1);
const id = client.insert(vector);

// Search (synchronous)
const results = client.search(query, 10); // Top 10 results
// Returns: [{ id: number, distance: number }, ...]

// The wrapper's save()/load() require browser IndexedDB.
// For Node.js files, use the WASM EdgeVec API shown in quickstart.js:
// await writeFile('my-database.evwb', index.toBytes());
// const loaded = EdgeVec.fromBytes(await readFile('my-database.evwb'));
```

## Troubleshooting

### Error: Cannot find module '@edgevec/core'

**Solution:** Build the EdgeVec package first:
```bash
cd ../../
npm run build
cd examples/nodejs
npm install
```

### Error: WASM initialization failed

**Solution:** Ensure you're using Node.js >= 16.0.0:
```bash
node --version
```

### Benchmark fails P99 target

**Solution:** This is expected on slower hardware. The <10ms P99 target assumes modern hardware (AMD Ryzen 7 or equivalent). On slower machines, the P99 may be 15-20ms, which is still excellent performance.

## Next Steps

- Read the main README for architecture details
- Check `examples/browser/` for browser usage
- Review the TypeScript wrapper at `wasm/EdgeVecClient.ts`
