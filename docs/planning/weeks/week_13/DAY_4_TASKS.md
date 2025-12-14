# Week 13 — Day 4 Tasks (Thursday, Dec 19)

**Date:** 2025-12-19
**Focus:** Competitive Benchmarks — Collect All Measurements
**Agent:** BENCHMARK_SCIENTIST
**Status:** DRAFT

---

## Day Objective

Complete competitor library setup and run comprehensive benchmarks for all 5 libraries (EdgeVec + 4 competitors). Collect latency, memory, recall, and bundle size metrics.

**Success Criteria:**
- All 4 competitor libraries installed and running
- Benchmarks executed for all 5 libraries
- Latency P50/P90/P95/P99/P99.9 collected
- Memory and bundle size measured
- Raw data saved for analysis

---

## Tasks

### W13.3a: Competitive Benchmark Setup (COMPLETE)

**Priority:** P1
**Estimate:** 6h remaining (8h total)
**Agent:** BENCHMARK_SCIENTIST
**Status:** COMPLETE on Day 4

#### Day 4 Scope (Complete Setup)

- [ ] **AC3a.5:** Install remaining competitors (voy, usearch-wasm, vectra)
- [ ] **AC3a.6:** Create adapter for each library
- [ ] **AC3a.7:** Generate test dataset (100k vectors, 128D)
- [ ] **AC3a.8:** Verify all libraries load successfully
- [ ] **AC3a.9:** Run smoke test (1 query each)

#### Implementation Specification

**Install Competitors:**

```bash
cd benches/competitive
npm install hnswlib-wasm voy usearch vectra
```

**Note:** Some libraries may have different package names. Verify on npm.

**Test Dataset Generation:**

```javascript
// benches/competitive/generate_data.js
const fs = require('fs');

const NUM_VECTORS = 100000;
const DIMENSIONS = 128;

function generateRandomVector(dim) {
    return Array.from({ length: dim }, () => Math.random() * 2 - 1);
}

function generateDataset() {
    console.log(`Generating ${NUM_VECTORS} vectors of ${DIMENSIONS} dimensions...`);

    const vectors = [];
    for (let i = 0; i < NUM_VECTORS; i++) {
        vectors.push({
            id: i,
            vector: generateRandomVector(DIMENSIONS)
        });
        if (i % 10000 === 0) console.log(`  ${i}/${NUM_VECTORS}`);
    }

    // Generate queries (1000 random vectors)
    const queries = Array.from({ length: 1000 }, () => ({
        vector: generateRandomVector(DIMENSIONS)
    }));

    // Save to files
    fs.writeFileSync('data/vectors.json', JSON.stringify(vectors));
    fs.writeFileSync('data/queries.json', JSON.stringify(queries));

    console.log('Dataset saved to data/');
}

generateDataset();
```

**Adapter Template:**

```javascript
// benches/competitive/adapters/edgevec.js
class EdgeVecAdapter {
    constructor() {
        this.index = null;
        this.name = 'EdgeVec';
    }

    async init() {
        const wasm = await import('../../pkg/edgevec.js');
        await wasm.default();
        this.wasm = wasm;
    }

    async buildIndex(vectors, dimensions) {
        this.index = new this.wasm.HnswIndex(dimensions, 16, 200);
        for (const v of vectors) {
            this.index.insert(v.id, new Float32Array(v.vector));
        }
    }

    async search(query, k) {
        return this.index.search(new Float32Array(query), k);
    }

    getMemoryUsage() {
        // Use performance.memory if available
        if (performance.memory) {
            return performance.memory.usedJSHeapSize / 1024 / 1024; // MB
        }
        return null;
    }

    getBundleSize() {
        // Return WASM file size in KB
        // Implement based on actual file location
        return null;
    }
}

module.exports = EdgeVecAdapter;
```

#### Verification Commands

```bash
# Verify all npm packages installed
cd benches/competitive && npm ls

# Generate test data
node generate_data.js

# Verify data files exist
ls -la data/

# Smoke test each adapter
node -e "require('./adapters/edgevec.js')"
node -e "require('./adapters/hnswlib.js')"
# etc.
```

---

### W13.3b: EdgeVec Benchmarks (START + PARTIAL COMPLETE)

**Priority:** P1
**Estimate:** 2h on Day 4 (6h total)
**Agent:** BENCHMARK_SCIENTIST
**Status:** START Day 4 (2h), COMPLETE Day 5 (4h)

#### Day 4 Scope

- [ ] **AC3b.1:** Build EdgeVec WASM with release optimizations
- [ ] **AC3b.2:** Run benchmark harness for all libraries
- [ ] **AC3b.3:** Collect raw latency measurements (1000 queries)
- [ ] **AC3b.4:** Save results to JSON files

#### Implementation Specification

**Main Harness:**

```javascript
// benches/competitive/harness.js
const fs = require('fs');

// Import adapters
const EdgeVecAdapter = require('./adapters/edgevec.js');
const HnswlibAdapter = require('./adapters/hnswlib.js');
const VoyAdapter = require('./adapters/voy.js');
const UsearchAdapter = require('./adapters/usearch.js');
const VectraAdapter = require('./adapters/vectra.js');

const ADAPTERS = [
    new EdgeVecAdapter(),
    new HnswlibAdapter(),
    new VoyAdapter(),
    new UsearchAdapter(),
    new VectraAdapter(),
];

const K = 10;  // Recall@10
const NUM_QUERIES = 1000;

async function benchmark(adapter, vectors, queries) {
    console.log(`\n=== Benchmarking ${adapter.name} ===`);

    // 1. Initialize
    await adapter.init();

    // 2. Build index
    const buildStart = performance.now();
    await adapter.buildIndex(vectors, vectors[0].vector.length);
    const buildTime = performance.now() - buildStart;
    console.log(`  Build time: ${buildTime.toFixed(2)}ms`);

    // 3. Memory usage
    const memory = adapter.getMemoryUsage();
    console.log(`  Memory: ${memory ? memory.toFixed(2) + 'MB' : 'N/A'}`);

    // 4. Search latencies
    const latencies = [];
    for (const q of queries) {
        const start = performance.now();
        await adapter.search(q.vector, K);
        latencies.push(performance.now() - start);
    }

    // 5. Calculate percentiles
    latencies.sort((a, b) => a - b);
    const percentile = (p) => latencies[Math.floor(latencies.length * p)];

    const results = {
        name: adapter.name,
        buildTimeMs: buildTime,
        memoryMb: memory,
        latency: {
            p50: percentile(0.50),
            p90: percentile(0.90),
            p95: percentile(0.95),
            p99: percentile(0.99),
            p999: percentile(0.999),
        },
        bundleSizeKb: adapter.getBundleSize(),
    };

    console.log(`  P50: ${results.latency.p50.toFixed(3)}ms`);
    console.log(`  P99: ${results.latency.p99.toFixed(3)}ms`);
    console.log(`  P99.9: ${results.latency.p999.toFixed(3)}ms`);

    return results;
}

async function main() {
    // Load data
    const vectors = JSON.parse(fs.readFileSync('data/vectors.json'));
    const queries = JSON.parse(fs.readFileSync('data/queries.json'));

    console.log(`Loaded ${vectors.length} vectors, ${queries.length} queries`);

    // Run benchmarks
    const allResults = [];
    for (const adapter of ADAPTERS) {
        try {
            const result = await benchmark(adapter, vectors, queries);
            allResults.push(result);
        } catch (e) {
            console.error(`  ERROR: ${e.message}`);
            allResults.push({ name: adapter.name, error: e.message });
        }
    }

    // Save results
    const timestamp = new Date().toISOString().split('T')[0];
    fs.writeFileSync(
        `results/${timestamp}_benchmark.json`,
        JSON.stringify(allResults, null, 2)
    );

    console.log(`\nResults saved to results/${timestamp}_benchmark.json`);
}

main().catch(console.error);
```

#### Files to Create

- `benches/competitive/harness.js` (complete)
- `benches/competitive/generate_data.js`
- Adapter files for each library

#### Verification Commands

```bash
# Build EdgeVec WASM
wasm-pack build --target web --release

# Run benchmarks
cd benches/competitive
node harness.js

# Check results
cat results/$(date +%Y-%m-%d)_benchmark.json
```

---

## Day 4 Summary

**Total Effort:** 6h (W13.3a complete) + 2h (W13.3b start) = **8h scheduled**

**Deliverables:**
1. ✅ All 4 competitor libraries installed
2. ✅ All adapters created
3. ✅ Test dataset generated (100k vectors)
4. ✅ Benchmark harness functional
5. ✅ Raw results for all libraries

**Carryover to Day 5:**
- W13.3b: Complete (latency analysis, recall calculation) - 4h
- W13.3c: Analysis and report - 4h
- W13.4: Documentation - 4h

**Status Validation:**
```bash
# Run before end of day
cd benches/competitive
npm ls | grep -E "hnswlib|voy|usearch|vectra"
test -f data/vectors.json && echo "Dataset exists"
test -f results/*.json && echo "Results collected"
```

---

## HOSTILE_REVIEWER Pre-Flight Checklist

Before submitting Day 4 work:

- [ ] All competitor libraries installed
- [ ] Adapters created for all 5 libraries
- [ ] Test dataset generated (100k x 128D)
- [ ] Benchmark harness runs without errors
- [ ] Raw results collected for all libraries
- [ ] Any library failures documented

---

**PLANNER Notes:**
- Day 4 is "data collection day"
- Focus on getting raw numbers, not analysis
- If a competitor fails, document and continue
- Analysis happens on Day 5

**Status:** COMPLETE
**Next:** W13.3c competitive analysis, W13.4 documentation
