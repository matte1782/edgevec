// EdgeVec Node.js Benchmark Example
// Validates <10ms P99 search latency on 100k vectors

import { EdgeVec, EdgeVecConfig, initSync } from '@edgevec/core/wasm';
import { readFile, writeFile } from 'node:fs/promises';
import assert from 'node:assert/strict';

const DIMS = 128;
const NUM_VECTORS = 100_000;
const NUM_SEARCHES = 1000;
const K = 10; // Top-K neighbors

function generateRandomVector(dims) {
    const vector = new Float32Array(dims);
    let normSquared = 0;
    for (let i = 0; i < dims; i++) {
        vector[i] = Math.random() * 2 - 1; // Range: [-1, 1]
        normSquared += vector[i] * vector[i];
    }
    const norm = Math.sqrt(normSquared);
    if (norm > 0) {
        for (let i = 0; i < dims; i++) vector[i] /= norm;
    }
    return vector;
}

function calculatePercentile(values, p) {
    const sorted = [...values].sort((a, b) => a - b);
    const index = Math.ceil((p / 100) * sorted.length) - 1;
    return sorted[Math.max(0, index)];
}

async function main() {
    console.log('EdgeVec Performance Benchmark\n');
    console.log(`Configuration:`);
    console.log(`  Dimensions: ${DIMS}`);
    console.log('  Metric: cosine (unit vectors)');
    console.log(`  Vectors to insert: ${NUM_VECTORS.toLocaleString()}`);
    console.log(`  Search queries: ${NUM_SEARCHES.toLocaleString()}`);
    console.log(`  Top-K: ${K}\n`);

    // Create client
    console.log('Initializing EdgeVec...');
    const startInit = performance.now();
    const entry = import.meta.resolve('@edgevec/core/wasm');
    initSync({ module: await readFile(new URL('edgevec_bg.wasm', entry)) });
    const config = new EdgeVecConfig(DIMS);
    config.metric = 'cosine';
    let client;
    try {
        client = new EdgeVec(config);
        const initTime = performance.now() - startInit;
        console.log(`✓ Initialized in ${initTime.toFixed(2)}ms\n`);
        await benchmark(client);
    } finally {
        client?.free();
        config.free();
    }
}

async function benchmark(client) {
    // Benchmark: Insertion
    console.log(`Inserting ${NUM_VECTORS.toLocaleString()} vectors...`);
    const insertTimes = [];
    const startInsertBatch = performance.now();

    for (let i = 0; i < NUM_VECTORS; i++) {
        const vector = generateRandomVector(DIMS);
        const startInsert = performance.now();
        client.insert(vector);
        const insertTime = performance.now() - startInsert;
        insertTimes.push(insertTime);

        if ((i + 1) % 10000 === 0) {
            const avgInsert = insertTimes.slice(-10000).reduce((a, b) => a + b) / 10000;
            console.log(`  ${(i + 1).toLocaleString()} vectors | Avg insert: ${avgInsert.toFixed(3)}ms`);
        }
    }

    const totalInsertTime = performance.now() - startInsertBatch;
    const avgInsert = insertTimes.reduce((a, b) => a + b) / insertTimes.length;
    const p50Insert = calculatePercentile(insertTimes, 50);
    const p99Insert = calculatePercentile(insertTimes, 99);

    console.log(`\n✓ Insertion complete`);
    console.log(`  Total time: ${totalInsertTime.toFixed(2)}ms`);
    console.log(`  Throughput: ${(NUM_VECTORS / (totalInsertTime / 1000)).toFixed(0)} vectors/sec`);
    console.log(`  Latency (mean): ${avgInsert.toFixed(3)}ms`);
    console.log(`  Latency (P50): ${p50Insert.toFixed(3)}ms`);
    console.log(`  Latency (P99): ${p99Insert.toFixed(3)}ms\n`);

    // Benchmark: Search
    console.log(`Running ${NUM_SEARCHES.toLocaleString()} search queries...`);
    const searchTimes = [];
    const startSearchBatch = performance.now();

    for (let i = 0; i < NUM_SEARCHES; i++) {
        const query = generateRandomVector(DIMS);
        const startSearch = performance.now();
        const results = client.search(query, K);
        const searchTime = performance.now() - startSearch;
        searchTimes.push(searchTime);

        // Verify search returned results
        if (results.length === 0) {
            throw new Error(`Search returned 0 results at iteration ${i}`);
        }
    }

    const totalSearchTime = performance.now() - startSearchBatch;
    const avgSearch = searchTimes.reduce((a, b) => a + b) / searchTimes.length;
    const p50Search = calculatePercentile(searchTimes, 50);
    const p95Search = calculatePercentile(searchTimes, 95);
    const p99Search = calculatePercentile(searchTimes, 99);
    const maxSearch = Math.max(...searchTimes);

    console.log(`\n✓ Search benchmark complete`);
    console.log(`  Total time: ${totalSearchTime.toFixed(2)}ms`);
    console.log(`  Throughput: ${(NUM_SEARCHES / (totalSearchTime / 1000)).toFixed(0)} queries/sec`);
    console.log(`  Latency (mean): ${avgSearch.toFixed(3)}ms`);
    console.log(`  Latency (P50): ${p50Search.toFixed(3)}ms`);
    console.log(`  Latency (P95): ${p95Search.toFixed(3)}ms`);
    console.log(`  Latency (P99): ${p99Search.toFixed(3)}ms`);
    console.log(`  Latency (max): ${maxSearch.toFixed(3)}ms\n`);

    // Validate performance target
    const TARGET_P99_MS = 10.0;
    if (p99Search < TARGET_P99_MS) {
        console.log(`✓ PASS: P99 search latency (${p99Search.toFixed(3)}ms) < ${TARGET_P99_MS}ms`);
    } else {
        console.log(`✗ FAIL: P99 search latency (${p99Search.toFixed(3)}ms) >= ${TARGET_P99_MS}ms`);
        process.exitCode = 1;
    }

    // Benchmark Node.js file persistence, including when the latency target misses.
    console.log(`\nBenchmarking persistence...`);
    const startSave = performance.now();
    await writeFile('benchmark-db.evwb', client.toBytes());
    const saveTime = performance.now() - startSave;
    console.log(`✓ Saved ${NUM_VECTORS.toLocaleString()} vectors in ${saveTime.toFixed(2)}ms\n`);

    const startLoad = performance.now();
    const restored = EdgeVec.fromBytes(await readFile('benchmark-db.evwb'));
    const loadTime = performance.now() - startLoad;
    try {
        assert.equal(restored.dimensions(), DIMS);
        assert.equal(restored.liveCount(), NUM_VECTORS);
        const query = generateRandomVector(DIMS);
        assert.deepEqual(restored.search(query, K), client.search(query, K));
        console.log(`✓ Loaded and verified ${restored.liveCount().toLocaleString()} vectors in ${loadTime.toFixed(2)}ms\n`);
    } finally {
        restored.free();
    }

    console.log(p99Search < TARGET_P99_MS
        ? '✓ All benchmarks passed!'
        : '✗ Benchmark completed; the P99 latency target was not met.');
}

main().catch(err => {
    console.error('✗ Benchmark failed:', err.message);
    console.error(err.stack);
    process.exit(1);
});
