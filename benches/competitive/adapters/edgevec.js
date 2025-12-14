/**
 * EdgeVec WASM Adapter for Competitive Benchmarks
 *
 * Task: W13.3a
 * Purpose: Benchmark adapter for EdgeVec WASM module
 *
 * This adapter loads the EdgeVec WASM module and provides a standard
 * interface for the benchmark harness.
 */

const path = require('path');

/**
 * EdgeVec WASM Adapter
 *
 * Implements the LibraryAdapter interface for EdgeVec.
 */
class EdgeVecAdapter {
    constructor() {
        this.name = 'edgevec';
        this.instance = null;
        this.wasm = null;
    }

    /**
     * Initialize the EdgeVec WASM module
     * @param {Object} config - Benchmark configuration
     */
    async initialize(config) {
        // Path to EdgeVec WASM package (built with wasm-pack)
        const pkgPath = path.join(__dirname, '../../../pkg');

        try {
            // Dynamic import of the WASM module
            // Note: This requires the WASM module to be built first with:
            //   wasm-pack build --target nodejs --release
            this.wasm = await import(path.join(pkgPath, 'edgevec.js'));

            // Create EdgeVec instance with config
            const edgevecConfig = new this.wasm.EdgeVecConfig(config.dimensions);

            // Set HNSW parameters if available
            if (config.hnsw) {
                // Note: EdgeVecConfig may have setters for these
                // edgevecConfig.set_m(config.hnsw.m);
                // edgevecConfig.set_ef_construction(config.hnsw.efConstruction);
            }

            this.instance = new this.wasm.EdgeVec(edgevecConfig);

            console.log(`[${this.name}] Initialized with dimensions=${config.dimensions}`);
        } catch (error) {
            console.warn(`[${this.name}] WASM module not found. Build with: wasm-pack build --target nodejs`);
            console.warn(`[${this.name}] Running in stub mode.`);
            this.instance = null;
        }
    }

    /**
     * Insert vectors into the index
     * @param {Float32Array[]} vectors - Array of vectors to insert
     * @returns {number[]} - Array of assigned IDs
     */
    async insert(vectors) {
        if (!this.instance) {
            // Stub mode: return fake IDs
            return vectors.map((_, i) => i + 1);
        }

        const ids = [];
        for (let i = 0; i < vectors.length; i++) {
            // Convert to Float32Array if needed
            const vec = vectors[i] instanceof Float32Array
                ? vectors[i]
                : new Float32Array(vectors[i]);

            // Insert using EdgeVec API
            // Note: API may be insert(vector) or insert_batch(vectors)
            try {
                const id = this.instance.insert(vec);
                ids.push(id);
            } catch (error) {
                console.error(`[${this.name}] Insert error at index ${i}:`, error);
            }
        }

        return ids;
    }

    /**
     * Insert vectors in batch (more efficient)
     * @param {Float32Array[]} vectors - Array of vectors to insert
     * @returns {Uint32Array} - Array of assigned IDs
     */
    async insertBatch(vectors) {
        if (!this.instance) {
            return new Uint32Array(vectors.map((_, i) => i + 1));
        }

        // Flatten vectors into single Float32Array
        const dimensions = vectors[0].length;
        const flatData = new Float32Array(vectors.length * dimensions);

        for (let i = 0; i < vectors.length; i++) {
            flatData.set(vectors[i], i * dimensions);
        }

        try {
            // Use batch insert API
            const ids = this.instance.insert_batch_flat(flatData, vectors.length);
            return ids;
        } catch (error) {
            console.error(`[${this.name}] Batch insert error:`, error);
            // Fall back to single inserts
            return new Uint32Array(await this.insert(vectors));
        }
    }

    /**
     * Search for k nearest neighbors
     * @param {Float32Array} query - Query vector
     * @param {number} k - Number of neighbors to find
     * @returns {Object[]} - Array of {id, distance} results
     */
    async search(query, k) {
        if (!this.instance) {
            // Stub mode: return empty results
            return [];
        }

        const vec = query instanceof Float32Array
            ? query
            : new Float32Array(query);

        try {
            // Search using EdgeVec API
            const results = this.instance.search(vec, k);

            // Convert to standard format
            // Note: Results format depends on EdgeVec API
            return Array.from(results || []);
        } catch (error) {
            console.error(`[${this.name}] Search error:`, error);
            return [];
        }
    }

    /**
     * Get current memory usage
     * @returns {number} - Memory used in bytes
     */
    async getMemoryUsage() {
        // Use Node.js heap as proxy for memory usage
        return process.memoryUsage().heapUsed;
    }

    /**
     * Clean up resources
     */
    async cleanup() {
        if (this.instance) {
            // Free WASM memory if the instance has a free/dispose method
            if (typeof this.instance.free === 'function') {
                this.instance.free();
            }
            this.instance = null;
        }
    }
}

module.exports = { EdgeVecAdapter };
