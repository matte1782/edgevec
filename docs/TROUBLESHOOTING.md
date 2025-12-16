# EdgeVec Troubleshooting Guide

**Version:** 0.3.0
**Purpose:** Solutions for common errors and issues with EdgeVec

---

## Table of Contents

1. [Installation Issues](#installation-issues)
2. [Initialization Errors](#initialization-errors)
3. [Insert Errors](#insert-errors)
4. [Search Issues](#search-issues)
5. [Persistence Errors](#persistence-errors)
6. [Browser-Specific Issues](#browser-specific-issues)
7. [Node.js-Specific Issues](#nodejs-specific-issues)
8. [Performance Problems](#performance-problems)
9. [Batch Insert Quirks](#batch-insert-quirks)
10. [Compaction Safety](#compaction-safety)
11. [Memory Issues](#memory-issues)
12. [WASM Issues](#wasm-issues)
13. [Common Error Reference](#common-error-reference)

---

## Installation Issues

### Error: "Cannot find module 'edgevec'"

**Cause:** Package not installed or incorrect import path.

**Solution:**
```bash
# Verify installation
npm list edgevec

# If not installed, install it
npm install edgevec

# Check for typos in import
```

**Correct import:**
```javascript
// ESM (recommended)
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

// CommonJS (Node.js without ESM)
const { default: init, EdgeVec, EdgeVecConfig } = await import('edgevec');
```

---

### Error: "Cannot use import statement outside a module"

**Cause:** Using ES module syntax in a CommonJS context.

**Solution 1: Use ESM in Node.js**
```json
// package.json
{
  "type": "module"
}
```

**Solution 2: Use .mjs extension**
```bash
# Rename your file
mv index.js index.mjs
node index.mjs
```

**Solution 3: Use dynamic import**
```javascript
// Works in CommonJS
async function main() {
    const { default: init, EdgeVec, EdgeVecConfig } = await import('edgevec');
    await init();
    // ...
}
main();
```

---

### Error: "Unsupported platform" or build errors

**Cause:** EdgeVec is pure WASM and doesn't require native compilation.

**Solution:** Ensure you're using the npm package, not trying to compile from source:
```bash
# Install the published package (not from source)
npm install edgevec

# If you have a corrupted node_modules
rm -rf node_modules package-lock.json
npm install
```

---

## Initialization Errors

### Error: "WASM module not initialized" or "RuntimeError: memory access out of bounds"

**Cause:** Using EdgeVec before calling `init()`.

**Wrong:**
```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

// WRONG - init() not called!
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);  // Error!
```

**Correct:**
```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    // CORRECT - always await init() first
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);  // Works!
}
main();
```

---

### Error: "WebAssembly.instantiate(): expected magic word"

**Cause:** The WASM file wasn't loaded correctly, often due to incorrect path or MIME type.

**Solution (Browser):**
```html
<!-- Ensure correct path to WASM files -->
<script type="module">
import init, { EdgeVec } from '/path/to/edgevec.js';

// init() will fetch the .wasm file from the same directory
await init();
</script>
```

**Solution (Server Configuration):**
Ensure your server serves `.wasm` files with the correct MIME type:
```
Content-Type: application/wasm
```

---

### Error: "WebAssembly is not defined"

**Cause:** Running in an environment without WebAssembly support.

**Solution:**
- Node.js: Use version 12+ (WASM is supported)
- Browser: Use a modern browser (Chrome 57+, Firefox 52+, Safari 11+)
- Verify you're not running in an older JavaScript runtime

```javascript
// Check for WASM support
if (typeof WebAssembly === 'undefined') {
    console.error('WebAssembly is not supported in this environment');
}
```

---

## Insert Errors

### Error: "DimensionMismatch: expected X, got Y"

**Cause:** The vector you're inserting doesn't match the configured dimensions.

**Wrong:**
```javascript
const config = new EdgeVecConfig(128);  // 128 dimensions
const index = new EdgeVec(config);

const vector = new Float32Array(256);  // 256 dimensions - wrong!
index.insert(vector);  // DimensionMismatch: expected 128, got 256
```

**Correct:**
```javascript
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);

const vector = new Float32Array(128);  // Matches config
index.insert(vector);  // Works!
```

**Debugging tip:**
```javascript
console.log('Config dimensions:', config.dimensions);
console.log('Vector length:', vector.length);
```

---

### Error: "InvalidVector: vector contains NaN or Infinity"

**Cause:** Your vector contains non-finite values.

**Wrong:**
```javascript
const vector = new Float32Array(128);
vector[0] = NaN;        // Invalid!
vector[1] = Infinity;   // Invalid!
vector[2] = -Infinity;  // Invalid!
```

**Solution:**
```javascript
function isValidVector(vec) {
    for (let i = 0; i < vec.length; i++) {
        if (!Number.isFinite(vec[i])) {
            console.error(`Invalid value at index ${i}: ${vec[i]}`);
            return false;
        }
    }
    return true;
}

// Validate before inserting
if (isValidVector(vector)) {
    index.insert(vector);
}
```

**Common causes of NaN:**
- Division by zero during normalization
- `Math.sqrt()` of negative numbers
- Operations on uninitialized arrays

---

### Error: "InvalidInput: ID X already exists"

**Cause:** Attempting to insert with a duplicate ID (in batch operations).

**Solution:**
```javascript
// Ensure unique IDs in batch operations
const seen = new Set();
const uniqueVectors = vectors.filter((_, idx) => {
    if (seen.has(idx)) return false;
    seen.add(idx);
    return true;
});

index.insertBatch(uniqueVectors);
```

---

## Search Issues

### Problem: Search returns empty results

**Possible causes and solutions:**

**1. Index is empty:**
```javascript
if (index.liveCount() === 0) {
    console.error('Index is empty - insert vectors first');
}
```

**2. Query vector is all zeros:**
```javascript
const hasNonZero = query.some(v => v !== 0);
if (!hasNonZero) {
    console.error('Query vector is all zeros');
}
```

**3. All vectors are deleted:**
```javascript
if (index.liveCount() === 0 && index.deletedCount() > 0) {
    console.error('All vectors have been deleted');
}
```

---

### Problem: Search results have poor accuracy

**Cause:** Default HNSW parameters may not be optimal for your use case.

**Solution:** Increase efSearch for better recall:
```javascript
const config = new EdgeVecConfig(768);
config.ef_search = 100;  // Increase from default 50
config.ef_construction = 400;  // Better index quality
config.m = 24;  // More connections

const index = new EdgeVec(config);
```

See the [Performance Tuning Guide](./PERFORMANCE_TUNING.md) for detailed parameter explanations.

---

### Problem: Search is slow

**Cause:** Parameters too high, or index too large.

**Solutions:**
1. Reduce efSearch:
   ```javascript
   config.ef_search = 30;  // Faster but lower recall
   ```

2. Reduce dimensions if possible (use a smaller embedding model)

3. Consider if you need all k results:
   ```javascript
   // Searching for fewer results is faster
   const results = index.search(query, 5);  // vs k=100
   ```

---

## Persistence Errors

### Error: "SerializationError: failed to deserialize"

**Cause:** The saved data is corrupted or from an incompatible version.

**Solution:**
```javascript
try {
    const index = await EdgeVec.load("my-db");
} catch (e) {
    if (e.message.includes('SerializationError')) {
        console.error('Saved data is corrupted. Creating new index...');
        // Create fresh index
        const config = new EdgeVecConfig(128);
        const index = new EdgeVec(config);
    }
}
```

---

### Error: "Load failed: database not found"

**Cause:** Trying to load a database that doesn't exist.

**Solution:**
```javascript
async function loadOrCreate(name, dimensions) {
    await init();

    try {
        return await EdgeVec.load(name);
    } catch (e) {
        console.log('No existing database, creating new one');
        const config = new EdgeVecConfig(dimensions);
        return new EdgeVec(config);
    }
}

const index = await loadOrCreate("my-db", 128);
```

---

### Error: "QuotaExceededError" (Browser)

**Cause:** IndexedDB storage quota exceeded.

**Solution:**
```javascript
// Check available storage (modern browsers)
if (navigator.storage && navigator.storage.estimate) {
    const estimate = await navigator.storage.estimate();
    console.log('Used:', estimate.usage);
    console.log('Available:', estimate.quota);
}

// Request persistent storage
if (navigator.storage && navigator.storage.persist) {
    const isPersisted = await navigator.storage.persist();
    console.log('Persistent storage:', isPersisted);
}
```

Consider:
- Compacting the index to remove deleted vectors
- Using smaller dimensions
- Splitting data across multiple smaller indices

---

## Browser-Specific Issues

### Error: "SharedArrayBuffer is not defined"

**Cause:** SharedArrayBuffer requires specific HTTP headers.

**Solution:** Configure your server to send these headers:
```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

**Note:** EdgeVec works without SharedArrayBuffer, but some features may be limited.

---

### Problem: WASM doesn't load from CDN

**Cause:** CORS or MIME type issues.

**Solution:**
```html
<script type="module">
// Use a CORS-enabled CDN like unpkg or jsDelivr
import init, { EdgeVec, EdgeVecConfig } from 'https://unpkg.com/edgevec@0.3.0/edgevec.js';

await init();
</script>
```

---

### Problem: Safari 14 compatibility issues

**Cause:** Safari 14 has limited BigInt64Array support.

**Solution:** Use the compatibility methods:
```javascript
// Instead of softDeleteBatch with Uint32Array
const ids = [1, 3, 5, 7, 9];  // Regular JS array
const result = index.softDeleteBatchCompat(new Float64Array(ids));
```

---

## Node.js-Specific Issues

### Error: "fetch is not defined" (Node.js 16)

**Cause:** Node.js 16 doesn't have built-in fetch.

**Solution 1:** Upgrade to Node.js 18+

**Solution 2:** Polyfill fetch:
```bash
npm install node-fetch
```

```javascript
import fetch from 'node-fetch';
globalThis.fetch = fetch;

import init, { EdgeVec, EdgeVecConfig } from 'edgevec';
await init();
```

---

### Problem: File not saved in expected location

**Cause:** Working directory may differ from script location.

**Solution:** Use absolute paths:
```javascript
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const dbPath = join(__dirname, 'data', 'my-vectors');
await index.save(dbPath);
```

---

## Performance Problems

### Problem: Insert is slow

**Causes and solutions:**

**1. High efConstruction:**
```javascript
// Reduce for faster builds
config.ef_construction = 100;  // vs 200
```

**2. Sequential inserts instead of batch:**
```javascript
// SLOW
for (const vec of vectors) {
    index.insert(vec);
}

// FAST
index.insertBatch(vectors);
```

**3. High M value:**
```javascript
// Reduce for faster inserts
config.m = 12;  // vs 16
```

---

### Problem: Memory grows unbounded

**Cause:** Deleted vectors are not reclaimed without compaction.

**Solution:**
```javascript
// Monitor tombstone ratio
setInterval(() => {
    const ratio = index.tombstoneRatio();
    if (ratio > 0.3) {
        console.log('Compacting to reclaim memory...');
        index.compact();
    }
}, 60000);
```

---

## Batch Insert Quirks

### Batch Insert Returns N-1 Vectors

**Symptom:** `insertBatch()` with 1000 vectors returns 999 IDs.

**Cause:** This is a known WASM optimization quirk related to HNSW entry point initialization. The first vector in a batch is used specially.

**Solution:** This is expected behavior in v0.3.x. Account for it in your code:

```javascript
const vectors = generateVectors(1000);
const result = index.insertBatch(vectors);

// result.inserted may be 999, not 1000
console.log(`Inserted ${result.inserted} of ${result.total} vectors`);

// IDs array matches inserted count
console.log(`Got ${result.ids.length} IDs`);
```

**Note:** This will be addressed in v0.5.0.

---

## Compaction Safety

### Warning: compact() WASM Aliasing

**Issue:** In WASM environments, calling `compact()` and then immediately accessing index properties may cause aliasing errors.

**Symptom:**
```
Error: recursive use of an object detected which would lead to unsafe aliasing in rust
```

**Safe Usage Pattern:**

```javascript
// SAFE: Check before compact, don't access during
if (index.needsCompaction()) {
    const beforeLive = index.liveCount();
    const beforeDeleted = index.deletedCount();

    // Compact - avoid accessing index during this
    index.compact();

    // After compact, create operations should work
    console.log('Compaction complete');
}
```

**Best Practices:**
1. Don't call `compact()` while searches are in progress
2. Don't access index properties immediately after `compact()` in tight loops
3. Consider running `compact()` during idle periods
4. In browser, consider using `requestIdleCallback()` for compaction

**Note:** This is a WASM binding limitation being addressed in future versions.

---

## Memory Issues

### Error: "Out of memory"

**Cause:** Index is too large for available memory.

**Solutions:**

1. **Use smaller dimensions:**
   ```javascript
   // 384d uses half the memory of 768d
   const config = new EdgeVecConfig(384);
   ```

2. **Reduce M:**
   ```javascript
   config.m = 8;  // Uses less memory
   ```

3. **Split into multiple indices:**
   ```javascript
   // Create shards
   const shards = [
       new EdgeVec(config),
       new EdgeVec(config),
       // ...
   ];
   ```

4. **Compact regularly:**
   ```javascript
   if (index.needsCompaction()) {
       index.compact();
   }
   ```

### Estimating memory usage

```javascript
// Rough estimate
const dimensions = 768;
const numVectors = 100000;
const M = 16;

const bytesPerVector = dimensions * 4 + M * 2 * 8 + 16;
const totalMB = (bytesPerVector * numVectors) / (1024 * 1024);
console.log(`Estimated memory: ${totalMB.toFixed(0)} MB`);
```

---

## WASM Issues

### Problem: WASM module takes too long to load

**Cause:** Large WASM file or slow network.

**Solutions:**

1. **Use streaming instantiation:**
   ```javascript
   // init() already uses streaming where supported
   await init();
   ```

2. **Cache the WASM file:**
   - Use a Service Worker
   - Set appropriate cache headers

3. **Use a CDN close to your users:**
   ```javascript
   import init from 'https://cdn.jsdelivr.net/npm/edgevec@0.3.0/edgevec.js';
   ```

---

### Error: "CompileError: wasm validation error"

**Cause:** Corrupted WASM file or browser bug.

**Solution:**
```bash
# Clear npm cache and reinstall
npm cache clean --force
rm -rf node_modules
npm install
```

---

## Common Error Reference

| Error Message | Cause | Quick Fix |
|:--------------|:------|:----------|
| `WASM module not initialized` | `init()` not called | `await init()` first |
| `DimensionMismatch: expected X, got Y` | Wrong vector length | Match `config.dimensions` |
| `InvalidVector: contains NaN` | Non-finite values | Validate vector values |
| `IndexEmpty` | Searching empty index | Insert vectors first |
| `InvalidInput: ID not found` | ID doesn't exist | Check `liveCount()`, verify ID |
| `SerializationError` | Corrupt data | Create new index |
| `QuotaExceededError` | Storage full | Clear old data, compact |
| `RuntimeError: out of bounds` | Memory issue | Reduce index size |

---

## Getting Help

If you encounter an issue not covered here:

1. **Check existing issues:** [GitHub Issues](https://github.com/matte1782/edgevec/issues)
2. **Search discussions:** [GitHub Discussions](https://github.com/matte1782/edgevec/discussions)
3. **File a bug report:** Include:
   - EdgeVec version (`npm list edgevec`)
   - Environment (Node.js version, browser, OS)
   - Minimal reproduction code
   - Full error message and stack trace

---

## See Also

- [Tutorial](./TUTORIAL.md) — Getting started guide
- [Performance Tuning](./PERFORMANCE_TUNING.md) — Optimization guide
- [API Reference](./API_REFERENCE.md) — Full API documentation
