/**
 * EdgeVec Competitive Benchmark Harness
 *
 * Task: W13.3a
 * Purpose: Compare EdgeVec WASM performance against competitor libraries
 *
 * Usage:
 *   node harness.js                    # Run all benchmarks
 *   node harness.js --library=edgevec  # Run EdgeVec only
 *   node harness.js --vectors=10000    # Custom vector count
 */

const fs = require('fs');
const path = require('path');

// ============================================================================
// Configuration
// ============================================================================

const CONFIG = {
    dimensions: 128,
    vectorCount: 10000,
    queryCount: 100,
    k: 10,
    warmupRuns: 3,
    measurementRuns: 5,

    // HNSW parameters (consistent across libraries where possible)
    hnsw: {
        m: 16,
        efConstruction: 200,
        efSearch: 50
    }
};

// ============================================================================
// Benchmark Result Structure
// ============================================================================

class BenchmarkResult {
    constructor(library) {
        this.library = library;
        this.insertLatencies = [];
        this.searchLatencies = [];
        this.memoryBefore = 0;
        this.memoryAfter = 0;
        this.recall = 0;
        this.timestamp = new Date().toISOString();
    }

    get insertMean() {
        return this.mean(this.insertLatencies);
    }

    get insertP50() {
        return this.percentile(this.insertLatencies, 50);
    }

    get insertP99() {
        return this.percentile(this.insertLatencies, 99);
    }

    get searchMean() {
        return this.mean(this.searchLatencies);
    }

    get searchP50() {
        return this.percentile(this.searchLatencies, 50);
    }

    get searchP99() {
        return this.percentile(this.searchLatencies, 99);
    }

    get memoryUsedMB() {
        return (this.memoryAfter - this.memoryBefore) / (1024 * 1024);
    }

    mean(arr) {
        if (arr.length === 0) return 0;
        return arr.reduce((a, b) => a + b, 0) / arr.length;
    }

    percentile(arr, p) {
        if (arr.length === 0) return 0;
        const sorted = [...arr].sort((a, b) => a - b);
        const idx = Math.ceil((p / 100) * sorted.length) - 1;
        return sorted[Math.max(0, idx)];
    }

    toJSON() {
        return {
            library: this.library,
            timestamp: this.timestamp,
            config: CONFIG,
            insert: {
                mean_ms: this.insertMean.toFixed(4),
                p50_ms: this.insertP50.toFixed(4),
                p99_ms: this.insertP99.toFixed(4),
                total_count: this.insertLatencies.length
            },
            search: {
                mean_ms: this.searchMean.toFixed(4),
                p50_ms: this.searchP50.toFixed(4),
                p99_ms: this.searchP99.toFixed(4),
                total_count: this.searchLatencies.length
            },
            memory: {
                used_mb: this.memoryUsedMB.toFixed(2)
            },
            recall: {
                at_k: CONFIG.k,
                percentage: (this.recall * 100).toFixed(2)
            }
        };
    }
}

// ============================================================================
// Data Generation
// ============================================================================

function generateRandomVectors(count, dimensions) {
    console.log(`Generating ${count} random vectors of dimension ${dimensions}...`);
    const vectors = [];
    for (let i = 0; i < count; i++) {
        const vec = new Float32Array(dimensions);
        for (let j = 0; j < dimensions; j++) {
            vec[j] = Math.random();
        }
        vectors.push(vec);
    }
    return vectors;
}

function generateQueryVectors(count, dimensions) {
    return generateRandomVectors(count, dimensions);
}

// ============================================================================
// Library Adapters (Stubs - to be implemented per library)
// ============================================================================

/**
 * Base adapter interface that all library adapters must implement
 */
class LibraryAdapter {
    constructor(name) {
        this.name = name;
    }

    async initialize(config) {
        throw new Error('Not implemented');
    }

    async insert(vectors) {
        throw new Error('Not implemented');
    }

    async search(query, k) {
        throw new Error('Not implemented');
    }

    async getMemoryUsage() {
        // Default: use Node.js heap
        return process.memoryUsage().heapUsed;
    }

    async cleanup() {
        // Override if cleanup needed
    }
}

/**
 * EdgeVec WASM Adapter (to be implemented)
 */
class EdgeVecAdapter extends LibraryAdapter {
    constructor() {
        super('edgevec');
        this.instance = null;
    }

    async initialize(config) {
        // TODO: Load EdgeVec WASM module
        // const { EdgeVec } = await import('../../pkg/edgevec.js');
        // this.instance = new EdgeVec(config.dimensions);
        console.log(`[${this.name}] Initialize stub - WASM module to be loaded`);
    }

    async insert(vectors) {
        // TODO: Implement batch insert
        console.log(`[${this.name}] Insert stub - ${vectors.length} vectors`);
        return vectors.map((_, i) => i + 1); // Placeholder IDs
    }

    async search(query, k) {
        // TODO: Implement search
        console.log(`[${this.name}] Search stub - k=${k}`);
        return []; // Placeholder results
    }
}

// ============================================================================
// Benchmark Runner
// ============================================================================

async function runBenchmark(adapter, vectors, queries) {
    const result = new BenchmarkResult(adapter.name);

    console.log(`\n${'='.repeat(60)}`);
    console.log(`Benchmarking: ${adapter.name}`);
    console.log(`${'='.repeat(60)}`);

    // Initialize
    console.log('Initializing...');
    await adapter.initialize(CONFIG);

    // Memory before
    result.memoryBefore = await adapter.getMemoryUsage();

    // Warmup inserts
    console.log(`Warmup: ${CONFIG.warmupRuns} insert runs...`);
    for (let i = 0; i < CONFIG.warmupRuns; i++) {
        // Use small subset for warmup
        await adapter.insert(vectors.slice(0, 100));
    }

    // Re-initialize for actual measurement
    await adapter.cleanup();
    await adapter.initialize(CONFIG);
    result.memoryBefore = await adapter.getMemoryUsage();

    // Measured inserts
    console.log(`Measuring: ${CONFIG.measurementRuns} insert runs...`);
    for (let run = 0; run < CONFIG.measurementRuns; run++) {
        const startTime = performance.now();
        await adapter.insert(vectors);
        const endTime = performance.now();

        const latencyPerVector = (endTime - startTime) / vectors.length;
        result.insertLatencies.push(latencyPerVector);

        console.log(`  Run ${run + 1}: ${latencyPerVector.toFixed(4)} ms/vector`);
    }

    // Memory after inserts
    result.memoryAfter = await adapter.getMemoryUsage();

    // Warmup searches
    console.log(`Warmup: ${CONFIG.warmupRuns} search runs...`);
    for (let i = 0; i < CONFIG.warmupRuns; i++) {
        await adapter.search(queries[0], CONFIG.k);
    }

    // Measured searches
    console.log(`Measuring: ${CONFIG.measurementRuns} search runs...`);
    for (let run = 0; run < CONFIG.measurementRuns; run++) {
        const runLatencies = [];

        for (const query of queries) {
            const startTime = performance.now();
            await adapter.search(query, CONFIG.k);
            const endTime = performance.now();
            runLatencies.push(endTime - startTime);
        }

        const meanLatency = runLatencies.reduce((a, b) => a + b, 0) / runLatencies.length;
        result.searchLatencies.push(meanLatency);

        console.log(`  Run ${run + 1}: ${meanLatency.toFixed(4)} ms/query`);
    }

    // TODO: Calculate recall against ground truth
    result.recall = 0.0; // Placeholder

    await adapter.cleanup();

    return result;
}

// ============================================================================
// Main
// ============================================================================

async function main() {
    console.log('EdgeVec Competitive Benchmark Harness');
    console.log('=====================================\n');
    console.log('Configuration:', JSON.stringify(CONFIG, null, 2));
    console.log('');

    // Parse command line arguments
    const args = process.argv.slice(2);
    const libraryArg = args.find(a => a.startsWith('--library='));
    const vectorsArg = args.find(a => a.startsWith('--vectors='));

    if (vectorsArg) {
        CONFIG.vectorCount = parseInt(vectorsArg.split('=')[1]);
    }

    // Generate test data
    const vectors = generateRandomVectors(CONFIG.vectorCount, CONFIG.dimensions);
    const queries = generateQueryVectors(CONFIG.queryCount, CONFIG.dimensions);

    // Select adapters to run
    const adapters = [];

    if (!libraryArg || libraryArg.includes('edgevec')) {
        adapters.push(new EdgeVecAdapter());
    }

    // TODO: Add other adapters when implemented
    // if (!libraryArg || libraryArg.includes('hnswlib')) {
    //     adapters.push(new HnswlibAdapter());
    // }

    // Run benchmarks
    const results = [];
    for (const adapter of adapters) {
        try {
            const result = await runBenchmark(adapter, vectors, queries);
            results.push(result);
        } catch (error) {
            console.error(`Error benchmarking ${adapter.name}:`, error);
        }
    }

    // Output results
    console.log('\n');
    console.log('='.repeat(60));
    console.log('RESULTS SUMMARY');
    console.log('='.repeat(60));

    for (const result of results) {
        console.log(`\n${result.library}:`);
        console.log(JSON.stringify(result.toJSON(), null, 2));
    }

    // Save results to file
    const outputPath = path.join(__dirname, 'results', `benchmark_${Date.now()}.json`);
    fs.writeFileSync(outputPath, JSON.stringify(results.map(r => r.toJSON()), null, 2));
    console.log(`\nResults saved to: ${outputPath}`);
}

// Run if executed directly
if (require.main === module) {
    main().catch(console.error);
}

module.exports = { BenchmarkResult, LibraryAdapter, EdgeVecAdapter, CONFIG };
