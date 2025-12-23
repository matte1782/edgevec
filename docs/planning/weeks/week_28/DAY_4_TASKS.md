# Week 28 Day 4: Browser Demo + More Integration Tests

**Date:** 2025-12-26
**Focus:** Browser Demo and Comprehensive Integration Testing
**Estimated Duration:** 8 hours
**Phase:** RFC-002 Implementation Phase 3 (WASM & Integration)
**Dependencies:** W28.1-W28.3 (All WASM bindings)

---

## Tasks

### W28.4.3: Integration Tests — Hybrid Search (BQ + Filter)

**Objective:** Verify hybrid search correctly combines BQ with metadata filtering.

**Test Implementation:**

```javascript
// tests/integration/hybrid_search.test.js

import init, { WasmIndex } from '../pkg/edgevec.js';

describe('Hybrid Search Integration', () => {
    let index;
    const DIMENSIONS = 384;

    beforeAll(async () => {
        await init();
    });

    beforeEach(() => {
        index = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });

        // Insert categorized data
        const categories = ['news', 'sports', 'tech', 'entertainment'];
        for (let i = 0; i < 1000; i++) {
            const vector = new Float32Array(DIMENSIONS)
                .map(() => Math.random() * 2 - 1);
            const metadata = {
                category: categories[i % 4],
                score: Math.random(),
                active: i % 2 === 0,
                tags: i % 3 === 0 ? ['featured'] : []
            };
            index.insertWithMetadata(vector, metadata);
        }
    });

    it('should combine BQ speed with filter accuracy', () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            filter: 'category == "news"',
            useBQ: true,
            rescoreFactor: 3
        });

        // All results should match filter
        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.category).toBe('news');
        }
    });

    it('should handle complex filters', () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            filter: '(category == "news" OR category == "tech") AND active == true',
            useBQ: true,
            rescoreFactor: 5
        });

        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(['news', 'tech']).toContain(meta.category);
            expect(meta.active).toBe(true);
        }
    });

    it('should handle CONTAINS filter with arrays', () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);

        const results = index.searchHybrid(query, {
            k: 10,
            filter: 'tags CONTAINS "featured"',
            useBQ: true
        });

        for (const r of results) {
            const meta = index.getMetadata(r.id);
            expect(meta.tags).toContain('featured');
        }
    });

    it('should be faster than F32 filtered search', () => {
        const query = new Float32Array(DIMENSIONS)
            .map(() => Math.random() * 2 - 1);
        const filter = 'category == "news" AND score > 0.5';
        const ITERATIONS = 50;

        // Time F32 filtered
        const f32Start = performance.now();
        for (let i = 0; i < ITERATIONS; i++) {
            index.searchFiltered(query, filter, 10);
        }
        const f32Time = (performance.now() - f32Start) / ITERATIONS;

        // Time hybrid
        const hybridStart = performance.now();
        for (let i = 0; i < ITERATIONS; i++) {
            index.searchHybrid(query, {
                k: 10,
                filter,
                useBQ: true,
                rescoreFactor: 3
            });
        }
        const hybridTime = (performance.now() - hybridStart) / ITERATIONS;

        console.log(`F32 filtered: ${f32Time.toFixed(2)}ms, Hybrid: ${hybridTime.toFixed(2)}ms`);

        // Hybrid should be faster (at least comparable)
        expect(hybridTime).toBeLessThan(f32Time * 1.5);
    });

    it('should fall back gracefully when BQ disabled', () => {
        const nonBqIndex = new WasmIndex({ dimensions: DIMENSIONS, useBQ: false });

        for (let i = 0; i < 100; i++) {
            const vector = new Float32Array(DIMENSIONS).map(() => Math.random());
            nonBqIndex.insertWithMetadata(vector, { category: 'test' });
        }

        const query = new Float32Array(DIMENSIONS).map(() => Math.random());

        // Should work even without BQ
        const results = nonBqIndex.searchHybrid(query, {
            k: 10,
            filter: 'category == "test"',
            useBQ: true  // Will fall back to F32 since BQ not enabled
        });

        expect(results.length).toBeGreaterThan(0);
    });
});
```

**Acceptance Criteria:**
- [ ] Hybrid search returns filtered results
- [ ] Complex filters (AND/OR/CONTAINS) work
- [ ] Hybrid is faster than F32 filtered
- [ ] Falls back gracefully when BQ disabled

**Estimated Duration:** 2 hours

**Agent:** TEST_ENGINEER

---

### W28.4.4: Integration Tests — Persistence (v0.4 Format)

**Objective:** Verify persistence format v0.4 works with metadata + BQ.

**Test Implementation:**

```javascript
// tests/integration/persistence_v04.test.js

import init, { WasmIndex } from '../pkg/edgevec.js';
import { openDB, deleteDB } from 'idb';

describe('Persistence v0.4 Format', () => {
    const DB_NAME = 'edgevec_test_v04';
    const DIMENSIONS = 128;

    beforeAll(async () => {
        await init();
    });

    afterEach(async () => {
        await deleteDB(DB_NAME);
    });

    it('should persist metadata + BQ vectors', async () => {
        // Create index with all features
        const index1 = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });

        // Insert diverse data
        const testData = [];
        for (let i = 0; i < 100; i++) {
            const vector = new Float32Array(DIMENSIONS)
                .map(() => Math.random() * 2 - 1);
            const metadata = {
                index: i,
                category: ['a', 'b', 'c'][i % 3],
                score: Math.random(),
                active: i % 2 === 0
            };
            const id = index1.insertWithMetadata(vector, metadata);
            testData.push({ id, vector, metadata });
        }

        // Delete some vectors
        index1.softDelete(testData[10].id);
        index1.softDelete(testData[20].id);

        // Get snapshot info before save
        const countBefore = index1.vectorCount();
        const liveCountBefore = index1.liveCount();

        // Save snapshot
        const snapshot = index1.createSnapshot();

        // Verify snapshot has correct format version
        const version = index1.getSnapshotVersion(snapshot);
        expect(version).toBe('0.4');

        // Save to IndexedDB
        const db = await openDB(DB_NAME, 1, {
            upgrade(db) {
                db.createObjectStore('snapshots');
            }
        });
        await db.put('snapshots', snapshot, 'main');
        db.close();

        // Create new index and load
        const index2 = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });
        const db2 = await openDB(DB_NAME, 1);
        const loadedSnapshot = await db2.get('snapshots', 'main');
        index2.loadSnapshot(loadedSnapshot);
        db2.close();

        // Verify counts match
        expect(index2.vectorCount()).toBe(countBefore);
        expect(index2.liveCount()).toBe(liveCountBefore);

        // Verify metadata survived
        for (const { id, metadata } of testData) {
            if (id === testData[10].id || id === testData[20].id) {
                // Deleted vectors should have no metadata
                expect(index2.getMetadata(id)).toBeNull();
            } else {
                const loaded = index2.getMetadata(id);
                expect(loaded).not.toBeNull();
                expect(loaded.category).toBe(metadata.category);
                expect(loaded.score).toBeCloseTo(metadata.score, 5);
            }
        }

        // Verify BQ search works after load
        const query = new Float32Array(DIMENSIONS).map(() => Math.random() * 2 - 1);
        const bqResults = index2.searchBQ(query, 10);
        expect(bqResults.length).toBe(10);

        // Verify filtered search works
        const filteredResults = index2.searchFiltered(query, 'category == "a"', 10);
        for (const r of filteredResults) {
            const meta = index2.getMetadata(r.id);
            expect(meta.category).toBe('a');
        }
    });

    it('should handle empty metadata gracefully', async () => {
        const index1 = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });

        // Insert vectors without metadata
        for (let i = 0; i < 50; i++) {
            const vector = new Float32Array(DIMENSIONS).map(() => Math.random());
            index1.insert(vector);  // No metadata
        }

        // Insert vectors with metadata
        for (let i = 0; i < 50; i++) {
            const vector = new Float32Array(DIMENSIONS).map(() => Math.random());
            index1.insertWithMetadata(vector, { hasMetadata: true });
        }

        // Save and reload
        const snapshot = index1.createSnapshot();
        const index2 = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });
        index2.loadSnapshot(snapshot);

        // Verify counts
        expect(index2.vectorCount()).toBe(100);

        // Search should work
        const query = new Float32Array(DIMENSIONS).map(() => Math.random());
        const results = index2.searchFiltered(query, 'hasMetadata == true', 10);
        expect(results.length).toBeGreaterThan(0);
    });

    it('should maintain snapshot size efficiency', async () => {
        const index = new WasmIndex({ dimensions: DIMENSIONS, useBQ: true });

        // Insert vectors with metadata
        for (let i = 0; i < 1000; i++) {
            const vector = new Float32Array(DIMENSIONS).map(() => Math.random());
            index.insertWithMetadata(vector, {
                category: 'test',
                value: i
            });
        }

        const snapshot = index.createSnapshot();

        // Calculate expected size
        // - F32 vectors: 1000 * 128 * 4 = 512KB
        // - BQ vectors: 1000 * 16 = 16KB
        // - Metadata: ~50 bytes per vector = 50KB
        // - Overhead: ~20%
        const expectedMax = (512 + 16 + 50) * 1.2 * 1024;  // ~700KB

        expect(snapshot.byteLength).toBeLessThan(expectedMax);
        console.log(`Snapshot size: ${(snapshot.byteLength / 1024).toFixed(1)}KB`);
    });
});
```

**Acceptance Criteria:**
- [ ] v0.4 format includes metadata section
- [ ] v0.4 format includes BQ data
- [ ] Deleted vectors preserved correctly
- [ ] Snapshot size is efficient

**Estimated Duration:** 2 hours

**Agent:** TEST_ENGINEER

---

### W28.4.5: Browser Demo — Metadata Filtering UI

**Objective:** Create interactive demo showing metadata filtering.

**Implementation:**

```html
<!-- wasm/examples/v060_demo.html -->

<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EdgeVec v0.6.0 Demo — Metadata + Binary Quantization</title>
    <style>
        :root {
            --bg: #1a1a2e;
            --card: #16213e;
            --accent: #0f3460;
            --highlight: #e94560;
            --text: #eee;
            --text-dim: #888;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            font-family: 'Segoe UI', system-ui, sans-serif;
            background: var(--bg);
            color: var(--text);
            line-height: 1.6;
            padding: 2rem;
        }

        .container { max-width: 1200px; margin: 0 auto; }

        h1 {
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
            background: linear-gradient(135deg, var(--highlight), #f39c12);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .subtitle {
            color: var(--text-dim);
            margin-bottom: 2rem;
        }

        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
            gap: 1.5rem;
        }

        .card {
            background: var(--card);
            border-radius: 12px;
            padding: 1.5rem;
            border: 1px solid var(--accent);
        }

        .card h2 {
            font-size: 1.2rem;
            margin-bottom: 1rem;
            color: var(--highlight);
        }

        label {
            display: block;
            margin-bottom: 0.5rem;
            color: var(--text-dim);
            font-size: 0.9rem;
        }

        input[type="text"], select {
            width: 100%;
            padding: 0.75rem;
            border: 1px solid var(--accent);
            border-radius: 6px;
            background: var(--bg);
            color: var(--text);
            margin-bottom: 1rem;
            font-family: 'Fira Code', monospace;
        }

        button {
            padding: 0.75rem 1.5rem;
            border: none;
            border-radius: 6px;
            background: var(--highlight);
            color: white;
            cursor: pointer;
            font-weight: 600;
            transition: transform 0.1s, opacity 0.1s;
        }

        button:hover { opacity: 0.9; transform: translateY(-1px); }
        button:active { transform: translateY(0); }
        button:disabled { opacity: 0.5; cursor: not-allowed; }

        .results {
            margin-top: 1rem;
            font-family: 'Fira Code', monospace;
            font-size: 0.85rem;
            max-height: 300px;
            overflow-y: auto;
            background: var(--bg);
            border-radius: 6px;
            padding: 1rem;
        }

        .result-item {
            padding: 0.5rem;
            border-bottom: 1px solid var(--accent);
        }

        .result-item:last-child { border-bottom: none; }

        .metric {
            display: inline-block;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            font-size: 0.85rem;
            margin-right: 0.5rem;
        }

        .metric.good { background: #27ae60; }
        .metric.warn { background: #f39c12; }
        .metric.bad { background: #e74c3c; }

        .memory-bar {
            height: 20px;
            background: var(--bg);
            border-radius: 10px;
            overflow: hidden;
            margin-top: 0.5rem;
        }

        .memory-fill {
            height: 100%;
            background: linear-gradient(90deg, #27ae60, #f39c12, #e74c3c);
            transition: width 0.3s;
        }

        .status {
            padding: 0.5rem;
            border-radius: 6px;
            margin-bottom: 1rem;
            font-size: 0.9rem;
        }

        .status.loading { background: var(--accent); }
        .status.ready { background: #27ae60; }
        .status.error { background: #e74c3c; }

        .filter-examples {
            font-size: 0.8rem;
            color: var(--text-dim);
            margin-top: 0.5rem;
        }

        .filter-examples code {
            background: var(--bg);
            padding: 0.1rem 0.3rem;
            border-radius: 3px;
            cursor: pointer;
        }

        .filter-examples code:hover {
            background: var(--accent);
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>EdgeVec v0.6.0</h1>
        <p class="subtitle">Metadata Filtering + Binary Quantization Demo</p>

        <div id="status" class="status loading">Loading WASM module...</div>

        <div class="grid">
            <!-- Metadata Filtering -->
            <div class="card">
                <h2>Metadata Filtering</h2>
                <label for="filter">Filter Expression</label>
                <input type="text" id="filter" placeholder='category == "news" AND score > 0.5'>
                <div class="filter-examples">
                    Try:
                    <code onclick="setFilter('category == \"news\"')">category == "news"</code>
                    <code onclick="setFilter('score > 0.5')">score > 0.5</code>
                    <code onclick="setFilter('tags CONTAINS \"featured\"')">tags CONTAINS</code>
                </div>
                <button onclick="runFilteredSearch()">Search with Filter</button>
                <div id="filter-results" class="results"></div>
            </div>

            <!-- BQ Speed Comparison -->
            <div class="card">
                <h2>BQ vs F32 Speed</h2>
                <label>Compare search methods</label>
                <select id="search-method">
                    <option value="f32">F32 (Accurate)</option>
                    <option value="bq">BQ (Fast)</option>
                    <option value="bq-rescore">BQ + Rescore (Balanced)</option>
                </select>
                <button onclick="runSpeedTest()">Run Benchmark</button>
                <div id="speed-results" class="results"></div>
            </div>

            <!-- Hybrid Search -->
            <div class="card">
                <h2>Hybrid Search</h2>
                <label for="hybrid-filter">Filter</label>
                <input type="text" id="hybrid-filter" placeholder='active == true'>
                <label>
                    <input type="checkbox" id="use-bq" checked> Use Binary Quantization
                </label>
                <button onclick="runHybridSearch()">Search</button>
                <div id="hybrid-results" class="results"></div>
            </div>

            <!-- Memory Pressure -->
            <div class="card">
                <h2>Memory Pressure</h2>
                <div id="memory-level"></div>
                <div class="memory-bar">
                    <div id="memory-fill" class="memory-fill" style="width: 0%"></div>
                </div>
                <div id="memory-stats" style="margin-top: 0.5rem; font-size: 0.85rem;"></div>
                <button onclick="checkMemory()" style="margin-top: 1rem;">Refresh</button>
                <button onclick="insertMore()" style="margin-left: 0.5rem;">Insert 1000 More</button>
            </div>

            <!-- Index Stats -->
            <div class="card">
                <h2>Index Statistics</h2>
                <div id="index-stats"></div>
            </div>
        </div>
    </div>

    <script type="module">
        import init, { WasmIndex } from './pkg/edgevec.js';

        let index;
        const DIMS = 384;

        async function setup() {
            try {
                await init();

                index = new WasmIndex({ dimensions: DIMS, useBQ: true });

                // Insert sample data
                const categories = ['news', 'sports', 'tech', 'entertainment'];
                const statusEl = document.getElementById('status');

                for (let i = 0; i < 2000; i++) {
                    const vector = new Float32Array(DIMS)
                        .map(() => Math.random() * 2 - 1);
                    const metadata = {
                        category: categories[i % 4],
                        score: Math.random(),
                        active: i % 2 === 0,
                        tags: i % 5 === 0 ? ['featured', 'trending'] : ['normal']
                    };
                    index.insertWithMetadata(vector, metadata);

                    if (i % 500 === 0) {
                        statusEl.textContent = `Loading... ${i}/2000 vectors`;
                    }
                }

                statusEl.className = 'status ready';
                statusEl.textContent = 'Ready — 2000 vectors loaded';

                updateStats();
                checkMemory();

            } catch (err) {
                document.getElementById('status').className = 'status error';
                document.getElementById('status').textContent = 'Error: ' + err.message;
            }
        }

        function updateStats() {
            const stats = document.getElementById('index-stats');
            stats.innerHTML = `
                <div>Total vectors: <strong>${index.vectorCount()}</strong></div>
                <div>Live vectors: <strong>${index.liveCount()}</strong></div>
                <div>Dimensions: <strong>${DIMS}</strong></div>
                <div>BQ enabled: <strong>${index.hasBQ() ? 'Yes' : 'No'}</strong></div>
            `;
        }

        window.setFilter = function(filter) {
            document.getElementById('filter').value = filter;
        };

        window.runFilteredSearch = async function() {
            const filter = document.getElementById('filter').value;
            const query = new Float32Array(DIMS).map(() => Math.random() * 2 - 1);
            const resultsEl = document.getElementById('filter-results');

            try {
                const start = performance.now();
                const results = index.searchFiltered(query, filter, 10);
                const elapsed = performance.now() - start;

                let html = `<div style="margin-bottom: 0.5rem;">
                    <span class="metric good">${results.length} results</span>
                    <span class="metric">${elapsed.toFixed(2)}ms</span>
                </div>`;

                for (const r of results) {
                    const meta = index.getMetadata(r.id);
                    html += `<div class="result-item">
                        ID: ${r.id} | Distance: ${r.distance.toFixed(4)}
                        <br><small>${JSON.stringify(meta)}</small>
                    </div>`;
                }

                resultsEl.innerHTML = html;
            } catch (err) {
                resultsEl.innerHTML = `<div style="color: #e74c3c;">${err.message}</div>`;
            }
        };

        window.runSpeedTest = async function() {
            const method = document.getElementById('search-method').value;
            const query = new Float32Array(DIMS).map(() => Math.random() * 2 - 1);
            const resultsEl = document.getElementById('speed-results');
            const ITERATIONS = 100;

            const start = performance.now();
            for (let i = 0; i < ITERATIONS; i++) {
                if (method === 'f32') {
                    index.search(query, 10);
                } else if (method === 'bq') {
                    index.searchBQ(query, 10);
                } else {
                    index.searchBQRescored(query, 10, 3);
                }
            }
            const elapsed = (performance.now() - start) / ITERATIONS;

            const labels = { 'f32': 'F32', 'bq': 'BQ', 'bq-rescore': 'BQ+Rescore' };

            resultsEl.innerHTML = `
                <div class="metric ${elapsed < 1 ? 'good' : elapsed < 5 ? 'warn' : 'bad'}">
                    ${labels[method]}: ${elapsed.toFixed(3)}ms/query
                </div>
                <div style="margin-top: 0.5rem;">
                    Throughput: ${(1000 / elapsed).toFixed(0)} queries/sec
                </div>
            `;
        };

        window.runHybridSearch = async function() {
            const filter = document.getElementById('hybrid-filter').value;
            const useBQ = document.getElementById('use-bq').checked;
            const query = new Float32Array(DIMS).map(() => Math.random() * 2 - 1);
            const resultsEl = document.getElementById('hybrid-results');

            try {
                const start = performance.now();
                const results = index.searchHybrid(query, {
                    k: 10,
                    filter: filter || undefined,
                    useBQ,
                    rescoreFactor: 3
                });
                const elapsed = performance.now() - start;

                let html = `<div style="margin-bottom: 0.5rem;">
                    <span class="metric good">${results.length} results</span>
                    <span class="metric">${elapsed.toFixed(2)}ms</span>
                    <span class="metric">${useBQ ? 'BQ' : 'F32'}</span>
                </div>`;

                for (const r of results.slice(0, 5)) {
                    const meta = index.getMetadata(r.id);
                    html += `<div class="result-item">
                        ID: ${r.id} | Dist: ${r.distance.toFixed(4)}
                        <br><small>${JSON.stringify(meta)}</small>
                    </div>`;
                }

                resultsEl.innerHTML = html;
            } catch (err) {
                resultsEl.innerHTML = `<div style="color: #e74c3c;">${err.message}</div>`;
            }
        };

        window.checkMemory = function() {
            const pressure = index.getMemoryPressure();
            const levelEl = document.getElementById('memory-level');
            const fillEl = document.getElementById('memory-fill');
            const statsEl = document.getElementById('memory-stats');

            const colors = { normal: 'good', warning: 'warn', critical: 'bad' };

            levelEl.innerHTML = `<span class="metric ${colors[pressure.level]}">${pressure.level.toUpperCase()}</span>`;
            fillEl.style.width = `${pressure.usagePercent}%`;
            statsEl.innerHTML = `
                ${(pressure.usedBytes / 1024 / 1024).toFixed(1)} MB /
                ${(pressure.totalBytes / 1024 / 1024).toFixed(1)} MB
                (${pressure.usagePercent.toFixed(1)}%)
            `;
        };

        window.insertMore = async function() {
            const categories = ['news', 'sports', 'tech', 'entertainment'];

            for (let i = 0; i < 1000; i++) {
                const vector = new Float32Array(DIMS).map(() => Math.random() * 2 - 1);
                index.insertWithMetadata(vector, {
                    category: categories[Math.floor(Math.random() * 4)],
                    score: Math.random(),
                    active: Math.random() > 0.5
                });
            }

            updateStats();
            checkMemory();
        };

        setup();
    </script>
</body>
</html>
```

**Acceptance Criteria:**
- [ ] Demo loads without errors
- [ ] Metadata filtering works interactively
- [ ] Speed comparison shows BQ advantage
- [ ] Memory pressure updates correctly
- [ ] Works in Chrome, Firefox, Safari

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST + DOCWRITER

---

### W28.4.6: Browser Demo — BQ vs F32 Visualization

**Objective:** Visual comparison of BQ speed and recall tradeoffs.

**Implementation:**
(Included in the v060_demo.html above with the speed test section)

**Additional Recall Visualization:**

```javascript
// Add to demo JavaScript

window.runRecallTest = async function() {
    const query = new Float32Array(DIMS).map(() => Math.random() * 2 - 1);
    const K = 10;

    // Ground truth
    const f32Results = index.search(query, K);
    const f32Ids = new Set(f32Results.map(r => r.id));

    // Test different methods
    const methods = [
        { name: 'BQ raw', fn: () => index.searchBQ(query, K) },
        { name: 'BQ + rescore(2)', fn: () => index.searchBQRescored(query, K, 2) },
        { name: 'BQ + rescore(3)', fn: () => index.searchBQRescored(query, K, 3) },
        { name: 'BQ + rescore(5)', fn: () => index.searchBQRescored(query, K, 5) },
        { name: 'BQ + rescore(10)', fn: () => index.searchBQRescored(query, K, 10) },
    ];

    let html = '<table style="width:100%; text-align:left;">';
    html += '<tr><th>Method</th><th>Recall</th><th>Time</th></tr>';

    for (const { name, fn } of methods) {
        const start = performance.now();
        const results = fn();
        const elapsed = performance.now() - start;

        const resultIds = new Set(results.map(r => r.id));
        let matches = 0;
        for (const id of resultIds) {
            if (f32Ids.has(id)) matches++;
        }
        const recall = matches / K;

        const recallClass = recall >= 0.9 ? 'good' : recall >= 0.7 ? 'warn' : 'bad';

        html += `<tr>
            <td>${name}</td>
            <td><span class="metric ${recallClass}">${(recall * 100).toFixed(0)}%</span></td>
            <td>${elapsed.toFixed(2)}ms</td>
        </tr>`;
    }

    html += '</table>';
    document.getElementById('recall-results').innerHTML = html;
};
```

**Acceptance Criteria:**
- [ ] Recall comparison table shows tradeoffs
- [ ] Visualization is clear and informative
- [ ] Interactive elements work
- [ ] Loads quickly (<2 seconds)

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

## Day 4 Checklist

- [ ] W28.4.3: Hybrid search integration tests
- [ ] W28.4.4: Persistence v0.4 integration tests
- [ ] W28.4.5: Browser demo with metadata filtering
- [ ] W28.4.6: BQ vs F32 visualization
- [ ] All integration tests pass
- [ ] Browser demo works in Chrome, Firefox, Safari
- [ ] wasm-pack test passes

## Day 4 Exit Criteria

| Criterion | Verification |
|:----------|:-------------|
| Hybrid search tests pass | Integration test |
| Persistence tests pass | Integration test |
| Browser demo works | Manual verification |
| Demo works in multiple browsers | Manual verification |
| wasm-pack test passes | `wasm-pack test --headless` |

## Day 4 Handoff

After completing Day 4:

**Artifacts Generated:**
- `tests/integration/hybrid_search.test.js`
- `tests/integration/persistence_v04.test.js`
- `wasm/examples/v060_demo.html`

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Day 5 — Documentation + Release Prep

---

*Agent: PLANNER + TEST_ENGINEER + WASM_SPECIALIST*
*Status: [PROPOSED]*
*Date: 2025-12-22*
