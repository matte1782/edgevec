/**
 * EdgeVec v0.6.0 Cyberpunk Demo Application
 * Main application controller
 */

import {
  ToastManager,
  SkeletonLoader,
  ResultCard,
  BenchmarkChart,
  MemoryGauge,
  ThemeManager,
  FilterTagManager,
  StatsUpdater
} from './components.js';

// =============================================================================
// Application State
// =============================================================================

const state = {
  db: null,
  vectorCount: 0,
  isLoading: false,
  searchMode: 'f32',
  lastBenchmark: null
};

// =============================================================================
// UI Components
// =============================================================================

let toast, chart, memoryGauge, themeManager, filterTags, statsUpdater;

// =============================================================================
// DOM Elements
// =============================================================================

const elements = {
  wasmStatus: document.getElementById('wasmStatus'),
  vectorCount: document.getElementById('vectorCount'),
  memoryUsage: document.getElementById('memoryUsage'),
  vectorSlider: document.getElementById('vectorSlider'),
  vectorSliderValue: document.getElementById('vectorSliderValue'),
  generateBtn: document.getElementById('generateBtn'),
  searchInput: document.getElementById('searchInput'),
  searchBtn: document.getElementById('searchBtn'),
  filterInput: document.getElementById('filterInput'),
  kValue: document.getElementById('kValue'),
  rescoreGroup: document.getElementById('rescoreGroup'),
  rescoreFactor: document.getElementById('rescoreFactor'),
  rescoreValue: document.getElementById('rescoreValue'),
  resultsContainer: document.getElementById('resultsContainer'),
  f32Latency: document.getElementById('f32Latency'),
  bqLatency: document.getElementById('bqLatency'),
  speedup: document.getElementById('speedup'),
  runBenchmarkBtn: document.getElementById('runBenchmarkBtn'),
  memUsed: document.getElementById('memUsed'),
  memTotal: document.getElementById('memTotal'),
  memStatus: document.getElementById('memStatus')
};

// =============================================================================
// Initialization
// =============================================================================

async function init() {
  // Initialize UI components
  toast = new ToastManager();
  chart = new BenchmarkChart('benchmarkChart');
  memoryGauge = new MemoryGauge();
  themeManager = new ThemeManager();
  filterTags = new FilterTagManager('filterTags', 'filterInput', onFilterChange);
  statsUpdater = new StatsUpdater({
    vectorCount: elements.vectorCount,
    memoryUsage: elements.memoryUsage
  });

  // Bind event listeners
  bindEvents();

  // Initialize WASM
  await initWasm();
}

async function initWasm() {
  try {
    elements.wasmStatus.textContent = 'LOADING...';
    elements.wasmStatus.classList.add('status-bar__value--loading');

    // Dynamic import of WASM module
    // Path is relative to the JS module location (wasm/examples/js/), not the HTML file
    // From wasm/examples/js/ go up 3 levels to project root: ../../../pkg/edgevec.js
    const wasmPath = '../../../pkg/edgevec.js';
    const { default: init, EdgeVec, EdgeVecConfig } = await import(wasmPath);

    await init();

    // Create database instance with proper config object
    const config = new EdgeVecConfig(768);
    state.db = new EdgeVec(config);

    // Enable binary quantization (768 is divisible by 8)
    try {
      state.db.enableBQ();
      console.log('BQ enabled:', state.db.hasBQ());
    } catch (e) {
      console.warn('Could not enable BQ:', e.message);
    }

    elements.wasmStatus.textContent = 'READY';
    elements.wasmStatus.classList.remove('status-bar__value--loading');
    elements.wasmStatus.style.color = 'var(--neon-green)';

    toast.success('WASM module initialized');
    updateMemoryDisplay();

  } catch (error) {
    console.error('WASM initialization failed:', error);
    elements.wasmStatus.textContent = 'ERROR';
    elements.wasmStatus.style.color = 'var(--neon-magenta)';
    toast.error(`WASM init failed: ${error.message}`);

    // Show fallback demo mode
    showDemoMode();
  }
}

function showDemoMode() {
  toast.warning('Running in demo mode (no WASM)');
  // Enable UI for demo purposes with mock data
  state.db = createMockDatabase();
}

function createMockDatabase() {
  // Mock database for demo purposes when WASM fails to load
  const vectors = [];
  const metadata = [];

  return {
    insert: (vector) => {
      const id = vectors.length;
      vectors.push(vector);
      return id;
    },
    insertWithMetadata: (vector, meta) => {
      const id = vectors.length;
      vectors.push(vector);
      metadata[id] = meta;
      return id;
    },
    search: (query, k) => {
      return vectors.slice(0, Math.min(k, vectors.length)).map((v, i) => ({
        id: i,
        distance: Math.random() * 0.5,
        metadata: metadata[i] || {}
      }));
    },
    searchBQ: (query, k) => {
      return vectors.slice(0, Math.min(k, vectors.length)).map((v, i) => ({
        id: i,
        distance: Math.floor(Math.random() * 100),
        metadata: metadata[i] || {}
      }));
    },
    searchFiltered: (query, filter, k) => {
      return vectors.slice(0, Math.min(k, vectors.length)).map((v, i) => ({
        id: i,
        distance: Math.random() * 0.5,
        metadata: metadata[i] || {}
      }));
    },
    searchHybrid: (query, options) => {
      return vectors.slice(0, Math.min(options.k || 10, vectors.length)).map((v, i) => ({
        id: i,
        distance: Math.random() * 0.5,
        metadata: metadata[i] || {}
      }));
    },
    getMemoryPressure: () => ({
      usedBytes: vectors.length * 768 * 4,
      totalBytes: 100 * 1024 * 1024,
      pressure: (vectors.length * 768 * 4) / (100 * 1024 * 1024)
    }),
    len: () => vectors.length,
    hasBQ: () => true
  };
}

// =============================================================================
// Event Binding
// =============================================================================

function bindEvents() {
  // Slider
  elements.vectorSlider?.addEventListener('input', (e) => {
    elements.vectorSliderValue.textContent = e.target.value;
  });

  // Generate button
  elements.generateBtn?.addEventListener('click', generateData);

  // Search
  elements.searchBtn?.addEventListener('click', executeSearch);
  elements.searchInput?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') executeSearch();
  });

  // Search mode radio buttons
  document.querySelectorAll('input[name="searchMode"]').forEach(radio => {
    radio.addEventListener('change', (e) => {
      state.searchMode = e.target.value;
      // Show/hide rescore factor for hybrid mode
      if (elements.rescoreGroup) {
        elements.rescoreGroup.style.display =
          state.searchMode === 'hybrid' ? 'block' : 'none';
      }
    });
  });

  // Rescore factor slider
  elements.rescoreFactor?.addEventListener('input', (e) => {
    elements.rescoreValue.textContent = `${e.target.value}x`;
  });

  // Benchmark button
  elements.runBenchmarkBtn?.addEventListener('click', runBenchmark);

  // Filter input change
  elements.filterInput?.addEventListener('change', () => {
    filterTags.clearActive();
  });
}

// =============================================================================
// Data Generation
// =============================================================================

async function generateData() {
  if (state.isLoading || !state.db) return;

  const count = parseInt(elements.vectorSlider.value, 10);

  state.isLoading = true;
  elements.generateBtn.disabled = true;
  elements.generateBtn.textContent = 'GENERATING...';

  toast.info(`Generating ${count} vectors...`);

  try {
    // Generate in batches to avoid blocking
    const batchSize = 100;
    const categories = ['tech', 'science', 'art', 'music', 'sports'];

    for (let i = 0; i < count; i += batchSize) {
      const batchEnd = Math.min(i + batchSize, count);

      for (let j = i; j < batchEnd; j++) {
        // Generate random 768D vector
        const vector = new Float32Array(768);
        for (let d = 0; d < 768; d++) {
          vector[d] = (Math.random() - 0.5) * 2;
        }

        // Normalize
        let norm = 0;
        for (let d = 0; d < 768; d++) {
          norm += vector[d] * vector[d];
        }
        norm = Math.sqrt(norm);
        for (let d = 0; d < 768; d++) {
          vector[d] /= norm;
        }

        // Generate metadata
        const metadata = {
          category: categories[Math.floor(Math.random() * categories.length)],
          score: parseFloat(Math.random().toFixed(2)),  // Must be number, not string
          active: Math.random() > 0.3
        };

        // Insert with metadata
        if (typeof state.db.insertWithMetadata === 'function') {
          state.db.insertWithMetadata(vector, metadata);
        } else {
          state.db.insert(vector);
        }
      }

      // Update progress
      state.vectorCount = typeof state.db.len === 'function' ? state.db.len() : batchEnd;
      updateStats();

      // Yield to UI
      await new Promise(r => setTimeout(r, 0));
    }

    toast.success(`Generated ${count} vectors`);
    updateMemoryDisplay();

  } catch (error) {
    console.error('Data generation failed:', error);
    toast.error(`Generation failed: ${error.message}`);
  } finally {
    state.isLoading = false;
    elements.generateBtn.disabled = false;
    elements.generateBtn.textContent = 'GENERATE_DATA';
  }
}

// =============================================================================
// Search Execution
// =============================================================================

async function executeSearch() {
  if (!state.db || state.vectorCount === 0) {
    toast.warning('Generate data first');
    return;
  }

  const query = elements.searchInput.value.trim();
  const filter = elements.filterInput.value.trim();
  const k = parseInt(elements.kValue.value, 10) || 10;

  // Generate random query vector (in real app, this would come from embedding)
  const queryVector = new Float32Array(768);
  for (let i = 0; i < 768; i++) {
    queryVector[i] = (Math.random() - 0.5) * 2;
  }

  // Normalize
  let norm = 0;
  for (let i = 0; i < 768; i++) {
    norm += queryVector[i] * queryVector[i];
  }
  norm = Math.sqrt(norm);
  for (let i = 0; i < 768; i++) {
    queryVector[i] /= norm;
  }

  // Show loading
  SkeletonLoader.show(elements.resultsContainer);

  try {
    let results;
    const startTime = performance.now();

    switch (state.searchMode) {
      case 'bq':
        results = state.db.searchBQ(queryVector, k);
        break;
      case 'hybrid':
        const rescoreFactor = parseInt(elements.rescoreFactor.value, 10) || 5;
        if (filter && typeof state.db.searchHybrid === 'function') {
          results = state.db.searchHybrid(queryVector, {
            k,
            filter,
            useBQ: true,
            rescoreFactor
          });
        } else if (typeof state.db.searchBQRescored === 'function') {
          results = state.db.searchBQRescored(queryVector, k, rescoreFactor);
        } else {
          results = state.db.searchBQ(queryVector, k);
        }
        break;
      case 'f32':
      default:
        // Only use searchFiltered if filter is non-empty
        if (filter && filter.length > 0 && typeof state.db.searchFiltered === 'function') {
          // searchFiltered signature: (query, k, options_json)
          // options_json must include { filter: "..." } as JSON string
          const options = JSON.stringify({ filter: filter, strategy: 'auto' });
          const jsonResult = state.db.searchFiltered(queryVector, k, options);
          // searchFiltered returns JSON string, parse it
          const parsed = JSON.parse(jsonResult);
          results = parsed.results || [];
        } else {
          results = state.db.search(queryVector, k);
        }
        break;
    }

    const endTime = performance.now();
    const latency = (endTime - startTime).toFixed(2);

    // Convert results if needed
    const resultArray = Array.isArray(results) ? results :
      (results && typeof results.toArray === 'function' ? results.toArray() : []);

    // Render results with delay for effect
    setTimeout(() => {
      ResultCard.renderResults(elements.resultsContainer, resultArray, state.searchMode);
      toast.success(`Found ${resultArray.length} results in ${latency}ms`);
    }, 300);

  } catch (error) {
    console.error('Search failed:', error);
    toast.error(`Search failed: ${error.message}`);
    elements.resultsContainer.innerHTML = `
      <div class="results__empty">
        <p class="neon-text--magenta">SEARCH_ERROR</p>
        <p class="text-muted">${error.message}</p>
      </div>
    `;
  }
}

// =============================================================================
// Benchmark
// =============================================================================

async function runBenchmark() {
  if (!state.db || state.vectorCount === 0) {
    toast.warning('Generate data first');
    return;
  }

  const iterations = 10;
  const k = 10;

  toast.info(`Running ${iterations} iterations...`);

  try {
    let f32Total = 0;
    let bqTotal = 0;

    for (let i = 0; i < iterations; i++) {
      // Generate random query
      const query = new Float32Array(768);
      for (let j = 0; j < 768; j++) {
        query[j] = (Math.random() - 0.5) * 2;
      }

      // Normalize
      let norm = 0;
      for (let j = 0; j < 768; j++) {
        norm += query[j] * query[j];
      }
      norm = Math.sqrt(norm);
      for (let j = 0; j < 768; j++) {
        query[j] /= norm;
      }

      // F32 search
      const f32Start = performance.now();
      state.db.search(query, k);
      f32Total += performance.now() - f32Start;

      // BQ search
      const bqStart = performance.now();
      state.db.searchBQ(query, k);
      bqTotal += performance.now() - bqStart;

      // Yield to UI
      await new Promise(r => setTimeout(r, 0));
    }

    const f32Avg = f32Total / iterations;
    const bqAvg = bqTotal / iterations;
    const speedupValue = f32Avg / bqAvg;

    // Update metrics
    elements.f32Latency.textContent = f32Avg.toFixed(2);
    elements.bqLatency.textContent = bqAvg.toFixed(2);
    elements.speedup.textContent = `${speedupValue.toFixed(1)}x`;

    // Update chart
    chart.update(f32Avg, bqAvg);

    state.lastBenchmark = { f32Avg, bqAvg, speedupValue };

    toast.success(`Benchmark complete: ${speedupValue.toFixed(1)}x speedup`);

  } catch (error) {
    console.error('Benchmark failed:', error);
    toast.error(`Benchmark failed: ${error.message}`);
  }
}

// =============================================================================
// Memory Display
// =============================================================================

function updateMemoryDisplay() {
  if (!state.db) return;

  try {
    const pressure = state.db.getMemoryPressure();
    const usedMB = (pressure.usedBytes / (1024 * 1024)).toFixed(1);
    const totalMB = (pressure.totalBytes / (1024 * 1024)).toFixed(1);

    elements.memUsed.textContent = `${usedMB} MB`;
    elements.memTotal.textContent = `${totalMB} MB`;
    elements.memoryUsage.textContent = `${usedMB} MB`;

    memoryGauge.update(pressure.usedBytes, pressure.totalBytes);

  } catch (error) {
    console.error('Failed to get memory pressure:', error);
  }
}

function updateStats() {
  statsUpdater.update({
    vectorCount: state.vectorCount.toLocaleString()
  });
}

// =============================================================================
// Filter Change Handler
// =============================================================================

function onFilterChange(filter) {
  elements.filterInput.value = filter;
}

// =============================================================================
// Start Application
// =============================================================================

document.addEventListener('DOMContentLoaded', init);
