/**
 * EdgeVec Batch Insert Demo - Cyberpunk Professional Edition
 * Week 12 Day 4 - W12.4 Deliverable
 *
 * @fileoverview Demonstrates EdgeVec WASM batch insert capabilities with
 * sequential vs batch comparison, error handling, and real-time metrics.
 *
 * @author WASM_SPECIALIST
 * @version 2.0.0
 * @license MIT
 */

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/**
 * @constant {Object} CONFIG - Application configuration constants
 */
const CONFIG = {
    /** @type {number} Minimum allowed vector count */
    MIN_VECTORS: 10,
    /** @type {number} Maximum allowed vector count */
    MAX_VECTORS: 100000,
    /** @type {number} Default vector count */
    DEFAULT_VECTORS: 1000,
    /** @type {number} Minimum allowed dimensions */
    MIN_DIMENSIONS: 2,
    /** @type {number} Maximum allowed dimensions */
    MAX_DIMENSIONS: 2048,
    /** @type {number} Default dimensions */
    DEFAULT_DIMENSIONS: 128,
    /** @type {number} UI update delay in milliseconds */
    UI_UPDATE_DELAY: 50,
    /** @type {number} Delay between comparison phases */
    COMPARISON_DELAY: 300,
};

/**
 * @constant {Object} PRESETS - Benchmark configuration presets
 */
const PRESETS = {
    C1: { vectors: 100, dims: 128, name: 'Small' },
    C2: { vectors: 1000, dims: 128, name: 'Medium' },
    C3: { vectors: 1000, dims: 512, name: 'High-D' },
    C4: { vectors: 5000, dims: 128, name: 'Large' },
};

/**
 * @constant {string[]} WASM_MODULE_PATHS - Possible paths for WASM module
 */
const WASM_MODULE_PATHS = [
    '../../pkg/edgevec.js',
    '../pkg/edgevec.js',
    '/pkg/edgevec.js',
    './pkg/edgevec.js',
];

// =============================================================================
// GLOBAL STATE
// =============================================================================

/** @type {Object|null} The loaded EdgeVec WASM module */
let edgeVecModule = null;

/** @type {Date} Application start time for logging */
const startTime = new Date();

// =============================================================================
// DOM ELEMENTS
// =============================================================================

/**
 * @constant {Object} DOM - Cached DOM element references
 */
const DOM = {
    // Status
    status: document.getElementById('status'),
    statusIcon: document.getElementById('statusIcon'),
    statusText: document.getElementById('statusText'),
    progressFill: document.getElementById('progressFill'),
    progressPhase: document.getElementById('progressPhase'),
    progressPercent: document.getElementById('progressPercent'),

    // Header info
    browserInfo: document.getElementById('browserInfo'),
    wasmSupport: document.getElementById('wasmSupport'),
    moduleStatus: document.getElementById('moduleStatus'),

    // Controls
    vectorCount: document.getElementById('vectorCount'),
    dimensions: document.getElementById('dimensions'),

    // Results
    sequentialResults: document.getElementById('sequentialResults'),
    batchResults: document.getElementById('batchResults'),
    speedupCard: document.getElementById('speedupCard'),

    // Sequential metrics
    seqCount: document.getElementById('seq-count'),
    seqTime: document.getElementById('seq-time'),
    seqAvg: document.getElementById('seq-avg'),
    seqThroughput: document.getElementById('seq-throughput'),

    // Batch metrics
    batchCount: document.getElementById('batch-count'),
    batchTime: document.getElementById('batch-time'),
    batchAvg: document.getElementById('batch-avg'),
    batchThroughput: document.getElementById('batch-throughput'),

    // Speedup
    speedupValue: document.getElementById('speedupValue'),

    // Terminal
    terminalOutput: document.getElementById('terminalOutput'),
};

// =============================================================================
// LOGGING SYSTEM
// =============================================================================

/**
 * Log level enumeration
 * @enum {string}
 */
const LogLevel = {
    INFO: 'info',
    SUCCESS: 'success',
    ERROR: 'error',
    WARN: 'warn',
};

/**
 * Get formatted timestamp for logging
 * @returns {string} Formatted timestamp [HH:MM:SS]
 */
function getTimestamp() {
    const now = new Date();
    const elapsed = now - startTime;
    const seconds = Math.floor(elapsed / 1000) % 60;
    const minutes = Math.floor(elapsed / 60000) % 60;
    const hours = Math.floor(elapsed / 3600000);
    return `[${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}]`;
}

/**
 * Log a message to the terminal output
 * @param {string} message - The message to log
 * @param {LogLevel} level - The log level
 */
function log(message, level = LogLevel.INFO) {
    const line = document.createElement('div');
    line.className = 'log-line';
    line.innerHTML = `
        <span class="log-time">${getTimestamp()}</span>
        <span class="log-level ${level}">${level.toUpperCase()}</span>
        <span class="log-message">${message}</span>
    `;
    DOM.terminalOutput.appendChild(line);
    DOM.terminalOutput.scrollTop = DOM.terminalOutput.scrollHeight;

    // Also log to console
    console.log(`${getTimestamp()} [${level.toUpperCase()}] ${message}`);
}

// =============================================================================
// STATUS & PROGRESS
// =============================================================================

/**
 * Show status message with type styling
 * @param {string} message - Status message to display
 * @param {'loading'|'success'|'error'} type - Status type
 */
function showStatus(message, type) {
    DOM.status.className = `status-bar visible ${type}`;
    DOM.statusText.textContent = message;

    const icons = {
        loading: '\u23F3',  // Hourglass
        success: '\u2714',  // Checkmark
        error: '\u2716',    // X mark
    };
    DOM.statusIcon.textContent = icons[type] || '';
}

/**
 * Hide the status bar
 */
function hideStatus() {
    DOM.status.className = 'status-bar';
}

/**
 * Update progress bar
 * @param {number} percent - Progress percentage (0-100)
 * @param {string} phase - Current phase description
 */
function updateProgress(percent, phase) {
    DOM.progressFill.style.width = `${percent}%`;
    DOM.progressPhase.textContent = phase;
    DOM.progressPercent.textContent = `${Math.round(percent)}%`;
}

// =============================================================================
// BROWSER DETECTION
// =============================================================================

/**
 * Detect and display browser information
 */
function displayBrowserInfo() {
    const ua = navigator.userAgent;
    let browser = 'Unknown';
    let version = '';

    if (ua.includes('Chrome')) {
        browser = 'Chrome';
        version = ua.match(/Chrome\/(\d+)/)?.[1] || '';
    } else if (ua.includes('Firefox')) {
        browser = 'Firefox';
        version = ua.match(/Firefox\/(\d+)/)?.[1] || '';
    } else if (ua.includes('Safari') && !ua.includes('Chrome')) {
        browser = 'Safari';
        version = ua.match(/Version\/(\d+)/)?.[1] || '';
    } else if (ua.includes('Edge')) {
        browser = 'Edge';
        version = ua.match(/Edge\/(\d+)/)?.[1] || '';
    }

    DOM.browserInfo.textContent = version ? `${browser} ${version}` : browser;
    log(`Browser detected: ${browser} ${version}`, LogLevel.INFO);

    // Check WebAssembly support
    const hasWasm = typeof WebAssembly === 'object'
        && typeof WebAssembly.instantiate === 'function';

    if (hasWasm) {
        DOM.wasmSupport.textContent = 'Supported';
        DOM.wasmSupport.classList.add('supported');
        log('WebAssembly: Supported', LogLevel.SUCCESS);
    } else {
        DOM.wasmSupport.textContent = 'NOT SUPPORTED';
        DOM.wasmSupport.classList.add('error');
        log('WebAssembly: NOT SUPPORTED', LogLevel.ERROR);
        showStatus('WebAssembly is not supported in this browser', 'error');
        disableAllButtons();
    }
}

// =============================================================================
// BUTTON STATE MANAGEMENT
// =============================================================================

/**
 * Disable all action buttons
 */
function disableAllButtons() {
    document.querySelectorAll('.btn').forEach(btn => {
        btn.disabled = true;
    });
}

/**
 * Enable all action buttons
 */
function enableAllButtons() {
    document.querySelectorAll('.btn').forEach(btn => {
        btn.disabled = false;
    });
}

// =============================================================================
// VECTOR GENERATION
// =============================================================================

/**
 * Generate random normalized vectors for testing
 * @param {number} count - Number of vectors to generate
 * @param {number} dims - Dimension of each vector
 * @returns {Float32Array[]} Array of normalized random vectors
 */
function generateRandomVectors(count, dims) {
    const vectors = [];

    for (let i = 0; i < count; i++) {
        const vector = new Float32Array(dims);
        let sumSquares = 0;

        // Generate random values in range [-1, 1]
        for (let j = 0; j < dims; j++) {
            vector[j] = Math.random() * 2 - 1;
            sumSquares += vector[j] * vector[j];
        }

        // Normalize to unit length
        const norm = Math.sqrt(sumSquares);
        if (norm > 0) {
            for (let j = 0; j < dims; j++) {
                vector[j] /= norm;
            }
        }

        vectors.push(vector);
    }

    return vectors;
}

// =============================================================================
// UTILITIES
// =============================================================================

/**
 * Sleep for specified milliseconds
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Format a number with locale-specific formatting
 * @param {number} num - Number to format
 * @returns {string} Formatted number string
 */
function formatNumber(num) {
    return num.toLocaleString();
}

/**
 * Calculate throughput (vectors per second)
 * @param {number} count - Number of vectors
 * @param {number} timeMs - Time in milliseconds
 * @returns {string} Formatted throughput string
 */
function calculateThroughput(count, timeMs) {
    const perSecond = (count / timeMs) * 1000;
    if (perSecond >= 1000) {
        return `${(perSecond / 1000).toFixed(1)}K/s`;
    }
    return `${Math.round(perSecond)}/s`;
}

// =============================================================================
// CORE BENCHMARK FUNCTIONS
// =============================================================================

/**
 * Run sequential insert benchmark (baseline)
 * @returns {Promise<{count: number, totalTime: number, avgTime: number}>}
 */
async function runSequentialInsert() {
    const count = parseInt(DOM.vectorCount.value, 10);
    const dims = parseInt(DOM.dimensions.value, 10);

    log(`Sequential insert: ${formatNumber(count)} vectors x ${dims}D`, LogLevel.INFO);

    showStatus(`Generating ${formatNumber(count)} random ${dims}D vectors...`, 'loading');
    updateProgress(10, 'Generating vectors...');
    await sleep(CONFIG.UI_UPDATE_DELAY);

    const vectors = generateRandomVectors(count, dims);
    log(`Generated ${formatNumber(count)} test vectors`, LogLevel.INFO);

    showStatus(`Running sequential insert of ${formatNumber(count)} vectors...`, 'loading');
    updateProgress(30, 'Inserting vectors...');
    await sleep(CONFIG.UI_UPDATE_DELAY);

    try {
        // Create fresh index
        const config = new edgeVecModule.EdgeVecConfig(dims);
        const index = new edgeVecModule.EdgeVec(config);

        const startTime = performance.now();

        // Insert one-by-one with progress updates
        for (let i = 0; i < vectors.length; i++) {
            index.insert(vectors[i]);

            // Update progress every 10% or every 100 vectors for small batches
            if (i % Math.max(Math.floor(count / 10), 100) === 0) {
                const progress = 30 + (i / count) * 60;
                updateProgress(progress, `Inserting vector ${formatNumber(i + 1)}/${formatNumber(count)}...`);
                await sleep(1); // Minimal yield for UI
            }
        }

        const endTime = performance.now();
        const totalTime = endTime - startTime;
        const avgTime = totalTime / count;
        const throughput = calculateThroughput(count, totalTime);

        updateProgress(100, 'Complete');

        // Update UI
        DOM.seqCount.textContent = formatNumber(count);
        DOM.seqTime.textContent = `${totalTime.toFixed(2)} ms`;
        DOM.seqAvg.textContent = `${avgTime.toFixed(4)} ms`;
        DOM.seqThroughput.textContent = throughput;
        DOM.sequentialResults.classList.add('visible');

        log(`Sequential complete: ${totalTime.toFixed(2)}ms (${throughput})`, LogLevel.SUCCESS);
        showStatus('Sequential insert complete!', 'success');

        return { count, totalTime, avgTime };

    } catch (error) {
        log(`Sequential insert failed: ${error.message || error}`, LogLevel.ERROR);
        showStatus(`Sequential insert failed: ${error.message || error}`, 'error');
        throw error;
    }
}

/**
 * Run batch insert benchmark (optimized)
 * @returns {Promise<{count: number, totalTime: number, avgTime: number}>}
 */
async function runBatchInsert() {
    const count = parseInt(DOM.vectorCount.value, 10);
    const dims = parseInt(DOM.dimensions.value, 10);

    log(`Batch insert: ${formatNumber(count)} vectors x ${dims}D`, LogLevel.INFO);

    showStatus(`Generating ${formatNumber(count)} random ${dims}D vectors...`, 'loading');
    updateProgress(10, 'Generating vectors...');
    await sleep(CONFIG.UI_UPDATE_DELAY);

    const vectors = generateRandomVectors(count, dims);
    log(`Generated ${formatNumber(count)} test vectors`, LogLevel.INFO);

    showStatus(`Running batch insert of ${formatNumber(count)} vectors...`, 'loading');
    updateProgress(50, 'Batch inserting...');
    await sleep(CONFIG.UI_UPDATE_DELAY);

    try {
        // Create fresh index
        const config = new edgeVecModule.EdgeVecConfig(dims);
        const index = new edgeVecModule.EdgeVec(config);

        const startTime = performance.now();

        // Insert as batch
        const result = index.insertBatch(vectors);

        const endTime = performance.now();
        const totalTime = endTime - startTime;
        const avgTime = totalTime / result.inserted;
        const throughput = calculateThroughput(result.inserted, totalTime);

        updateProgress(100, 'Complete');

        // Update UI
        DOM.batchCount.textContent = formatNumber(result.inserted);
        DOM.batchTime.textContent = `${totalTime.toFixed(2)} ms`;
        DOM.batchAvg.textContent = `${avgTime.toFixed(4)} ms`;
        DOM.batchThroughput.textContent = throughput;
        DOM.batchResults.classList.add('visible');

        log(`Batch complete: ${totalTime.toFixed(2)}ms (${throughput}) - ${result.inserted}/${result.total} inserted`, LogLevel.SUCCESS);
        showStatus(`Batch insert complete! (${formatNumber(result.inserted)}/${formatNumber(result.total)} vectors)`, 'success');

        return { count: result.inserted, totalTime, avgTime };

    } catch (error) {
        log(`Batch insert failed: ${error.message || error}`, LogLevel.ERROR);
        showStatus(`Batch insert failed: ${error.message || error}`, 'error');
        throw error;
    }
}

/**
 * Run comparison benchmark: sequential vs batch
 */
async function runComparison() {
    disableAllButtons();

    try {
        // Reset displays
        DOM.speedupCard.classList.remove('visible');
        DOM.sequentialResults.classList.remove('visible');
        DOM.batchResults.classList.remove('visible');

        log('Starting comparison benchmark...', LogLevel.INFO);
        log('='.repeat(50), LogLevel.INFO);

        // Run sequential first
        const seqResult = await runSequentialInsert();
        await sleep(CONFIG.COMPARISON_DELAY);

        // Run batch
        const batchResult = await runBatchInsert();

        // Calculate speedup
        const speedup = seqResult.totalTime / batchResult.totalTime;
        DOM.speedupValue.textContent = speedup.toFixed(2) + 'x';
        DOM.speedupCard.classList.add('visible');

        log('='.repeat(50), LogLevel.INFO);
        log(`SPEEDUP: Batch is ${speedup.toFixed(2)}x faster than sequential`, LogLevel.SUCCESS);

        showStatus('Comparison complete!', 'success');

    } catch (error) {
        log(`Comparison failed: ${error.message || error}`, LogLevel.ERROR);
        showStatus(`Comparison failed: ${error.message || error}`, 'error');
    } finally {
        enableAllButtons();
    }
}

/**
 * Test error handling with invalid inputs
 */
async function testErrors() {
    const dims = parseInt(DOM.dimensions.value, 10);

    disableAllButtons();
    showStatus('Testing error handling...', 'loading');

    log('Starting error handling tests...', LogLevel.INFO);
    log('='.repeat(50), LogLevel.INFO);

    const results = [];

    try {
        const config = new edgeVecModule.EdgeVecConfig(dims);
        const index = new edgeVecModule.EdgeVec(config);

        // Test 1: Empty batch - should throw EMPTY_BATCH
        log('Test 1: Empty batch', LogLevel.INFO);
        try {
            index.insertBatch([]);
            results.push({ test: 'EMPTY_BATCH', passed: false, error: 'No error thrown' });
            log('  FAIL: No error thrown for empty batch', LogLevel.ERROR);
        } catch (err) {
            const errStr = String(err.message || err);
            const passed = errStr.includes('EMPTY_BATCH') || errStr.toLowerCase().includes('empty');
            results.push({ test: 'EMPTY_BATCH', passed, error: errStr });
            log(`  ${passed ? 'PASS' : 'FAIL'}: ${errStr}`, passed ? LogLevel.SUCCESS : LogLevel.ERROR);
        }

        // Test 2: Dimension mismatch
        log('Test 2: Dimension mismatch', LogLevel.INFO);
        try {
            const wrongDimVec = new Float32Array(dims + 10);
            for (let i = 0; i < wrongDimVec.length; i++) wrongDimVec[i] = Math.random();
            index.insertBatch([wrongDimVec]);
            results.push({ test: 'DIMENSION_MISMATCH', passed: false, error: 'No error thrown' });
            log('  FAIL: No error thrown for dimension mismatch', LogLevel.ERROR);
        } catch (err) {
            const errStr = String(err.message || err);
            const passed = errStr.includes('DIMENSION') || errStr.toLowerCase().includes('dimension');
            results.push({ test: 'DIMENSION_MISMATCH', passed, error: errStr });
            log(`  ${passed ? 'PASS' : 'FAIL'}: ${errStr}`, passed ? LogLevel.SUCCESS : LogLevel.ERROR);
        }

        // Test 3: Invalid vector (NaN)
        log('Test 3: Invalid vector (NaN)', LogLevel.INFO);
        try {
            const nanVec = new Float32Array(dims);
            nanVec[0] = NaN;
            index.insertBatch([nanVec]);
            results.push({ test: 'INVALID_VECTOR (NaN)', passed: false, error: 'No error thrown' });
            log('  FAIL: No error thrown for NaN vector', LogLevel.ERROR);
        } catch (err) {
            const errStr = String(err.message || err);
            const passed = errStr.includes('INVALID') || errStr.toLowerCase().includes('nan') || errStr.toLowerCase().includes('finite');
            results.push({ test: 'INVALID_VECTOR (NaN)', passed, error: errStr });
            log(`  ${passed ? 'PASS' : 'FAIL'}: ${errStr}`, passed ? LogLevel.SUCCESS : LogLevel.ERROR);
        }

        // Test 4: Invalid vector (Infinity)
        log('Test 4: Invalid vector (Infinity)', LogLevel.INFO);
        try {
            const infVec = new Float32Array(dims);
            infVec[0] = Infinity;
            index.insertBatch([infVec]);
            results.push({ test: 'INVALID_VECTOR (Inf)', passed: false, error: 'No error thrown' });
            log('  FAIL: No error thrown for Infinity vector', LogLevel.ERROR);
        } catch (err) {
            const errStr = String(err.message || err);
            const passed = errStr.includes('INVALID') || errStr.toLowerCase().includes('infinity') || errStr.toLowerCase().includes('finite');
            results.push({ test: 'INVALID_VECTOR (Inf)', passed, error: errStr });
            log(`  ${passed ? 'PASS' : 'FAIL'}: ${errStr}`, passed ? LogLevel.SUCCESS : LogLevel.ERROR);
        }

        // Summary
        const passed = results.filter(r => r.passed).length;
        const total = results.length;

        log('='.repeat(50), LogLevel.INFO);
        log(`ERROR HANDLING RESULTS: ${passed}/${total} tests passed`, passed === total ? LogLevel.SUCCESS : LogLevel.WARN);

        if (passed === total) {
            showStatus(`All ${total} error tests passed!`, 'success');
        } else {
            showStatus(`${passed}/${total} error tests passed`, 'error');
        }

    } catch (error) {
        log(`Error testing failed: ${error.message || error}`, LogLevel.ERROR);
        showStatus(`Error testing failed: ${error.message || error}`, 'error');
    } finally {
        enableAllButtons();
    }
}

// =============================================================================
// PRESET HANDLING
// =============================================================================

/**
 * Apply a preset configuration
 * @param {number} vectors - Number of vectors
 * @param {number} dims - Number of dimensions
 */
function applyPreset(vectors, dims) {
    DOM.vectorCount.value = vectors;
    DOM.dimensions.value = dims;
    log(`Applied preset: ${formatNumber(vectors)} vectors x ${dims}D`, LogLevel.INFO);
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/**
 * Initialize the demo application
 */
async function init() {
    log('EdgeVec Batch Insert Demo v2.0.0', LogLevel.INFO);
    log('Initializing...', LogLevel.INFO);

    displayBrowserInfo();

    showStatus('Loading EdgeVec WASM module...', 'loading');
    updateProgress(0, 'Locating module...');

    try {
        let module = null;
        let loadedPath = null;

        for (let i = 0; i < WASM_MODULE_PATHS.length; i++) {
            const path = WASM_MODULE_PATHS[i];
            updateProgress((i / WASM_MODULE_PATHS.length) * 50, `Trying ${path}...`);

            try {
                module = await import(path);
                loadedPath = path;
                break;
            } catch (e) {
                log(`Path ${path}: Not found`, LogLevel.WARN);
            }
        }

        if (!module) {
            throw new Error(
                'Could not load EdgeVec WASM module. ' +
                'Build with: wasm-pack build --target web --release'
            );
        }

        updateProgress(70, 'Initializing WASM...');

        // Initialize WASM module
        if (module.default) {
            await module.default();
        }

        edgeVecModule = module;

        updateProgress(100, 'Ready');
        DOM.moduleStatus.textContent = 'Loaded';
        DOM.moduleStatus.classList.add('supported');

        log(`WASM module loaded from: ${loadedPath}`, LogLevel.SUCCESS);
        hideStatus();

        // Attach event listeners
        document.getElementById('runSequential').addEventListener('click', async () => {
            disableAllButtons();
            try {
                await runSequentialInsert();
            } finally {
                enableAllButtons();
            }
        });

        document.getElementById('runBatch').addEventListener('click', async () => {
            disableAllButtons();
            try {
                await runBatchInsert();
            } finally {
                enableAllButtons();
            }
        });

        document.getElementById('runComparison').addEventListener('click', runComparison);
        document.getElementById('testErrors').addEventListener('click', testErrors);

        // Preset buttons
        document.querySelectorAll('.preset-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const vectors = parseInt(btn.dataset.vectors, 10);
                const dims = parseInt(btn.dataset.dims, 10);
                applyPreset(vectors, dims);
            });
        });

        log('Demo ready. Select a preset or configure manually.', LogLevel.SUCCESS);

    } catch (error) {
        DOM.moduleStatus.textContent = 'Failed';
        DOM.moduleStatus.classList.add('error');
        log(`WASM loading failed: ${error.message}`, LogLevel.ERROR);
        showStatus(`Failed to load WASM module: ${error.message}`, 'error');
        disableAllButtons();
    }
}

// Start initialization when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
