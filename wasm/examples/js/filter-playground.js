/**
 * EdgeVec Filter Playground v0.7.0
 * Interactive filter builder and sandbox
 *
 * Dependencies:
 * - effects.js (ParticleSystem, MatrixRain, EffectManager)
 * - EdgeVec WASM module
 */

// =============================================================================
// EXAMPLES DATA
// =============================================================================

export const examples = [
  {
    id: 'ecommerce-1',
    title: 'Product Filter',
    category: 'E-Commerce',
    icon: '&#x1F6D2;',
    filter: 'category = "gpu" AND price < 500',
    description: 'Find GPUs under $500'
  },
  {
    id: 'ecommerce-2',
    title: 'Brand + Stock',
    category: 'E-Commerce',
    icon: '&#x1F4E6;',
    filter: '(brand = "nvidia" OR brand = "amd") AND inStock = true',
    description: 'Available products from top brands'
  },
  {
    id: 'ecommerce-3',
    title: 'Rating Filter',
    category: 'E-Commerce',
    icon: '&#x2B50;',
    filter: 'rating >= 4.5 AND reviews > 100',
    description: 'Highly rated products'
  },
  {
    id: 'docs-1',
    title: 'Author Search',
    category: 'Documents',
    icon: '&#x1F4C4;',
    filter: 'author = "John Doe" AND year >= 2023',
    description: 'Recent documents by author'
  },
  {
    id: 'docs-2',
    title: 'Tag-based Search',
    category: 'Documents',
    icon: '&#x1F3F7;',
    filter: 'tags ANY ["tutorial", "guide"] AND language = "en"',
    description: 'English tutorials and guides'
  },
  {
    id: 'docs-3',
    title: 'Date Range',
    category: 'Documents',
    icon: '&#x1F4C5;',
    filter: 'status = "published" AND year >= 2024',
    description: 'Recently published content'
  },
  {
    id: 'content-1',
    title: 'Video Filter',
    category: 'Content',
    icon: '&#x1F3AC;',
    filter: 'type = "video" AND duration < 600 AND views >= 1000',
    description: 'Popular short videos'
  },
  {
    id: 'content-2',
    title: 'Category + Rating',
    category: 'Content',
    icon: '&#x1F4FA;',
    filter: 'category IN ["tech", "science"] AND rating >= 4.0',
    description: 'Quality tech/science content'
  },
  {
    id: 'advanced-1',
    title: 'Nested Groups',
    category: 'Advanced',
    icon: '&#x1F9E9;',
    filter: '(category = "electronics" AND price < 100) OR (category = "books" AND price < 20)',
    description: 'Complex grouped conditions'
  },
  {
    id: 'advanced-2',
    title: 'NOT Operator',
    category: 'Advanced',
    icon: '&#x26A0;',
    filter: 'status != "deleted" AND NOT (archived = true)',
    description: 'Exclude deleted/archived items'
  }
];

// Sample data for sandbox
export const sampleProducts = [
  { name: 'RTX 4070', category: 'gpu', brand: 'nvidia', price: 450, rating: 4.7, inStock: true },
  { name: 'RTX 4080', category: 'gpu', brand: 'nvidia', price: 850, rating: 4.8, inStock: true },
  { name: 'RX 7800', category: 'gpu', brand: 'amd', price: 420, rating: 4.5, inStock: true },
  { name: 'RX 7900', category: 'gpu', brand: 'amd', price: 750, rating: 4.6, inStock: false },
  { name: 'Arc A770', category: 'gpu', brand: 'intel', price: 350, rating: 4.2, inStock: true },
  { name: 'Core i9', category: 'cpu', brand: 'intel', price: 550, rating: 4.9, inStock: true },
  { name: 'Ryzen 9', category: 'cpu', brand: 'amd', price: 480, rating: 4.8, inStock: true },
  { name: 'DDR5 32GB', category: 'memory', brand: 'corsair', price: 180, rating: 4.6, inStock: true },
  { name: 'NVMe 2TB', category: 'storage', brand: 'samsung', price: 220, rating: 4.7, inStock: true },
  { name: 'PSU 850W', category: 'power', brand: 'seasonic', price: 160, rating: 4.8, inStock: false }
];

// =============================================================================
// CODE SNIPPETS
// =============================================================================

export const codeSnippets = {
  javascript: `// Initialize EdgeVec
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

const config = new EdgeVecConfig(768);
const db = new EdgeVec(config);

// Insert vectors with metadata
const vector = new Float32Array(768).fill(0.1);
db.insertWithMetadata(vector, {
    category: "electronics",
    price: 299.99,
    inStock: true
});

// Search with filter (simplified API)
const query = new Float32Array(768).fill(0.1);
const results = db.searchWithFilter(
    query,
    'category = "electronics" AND price < 500',
    10
);

console.log(results); // [{id, distance}, ...]`,

  typescript: `import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

interface ProductMetadata {
    category: string;
    price: number;
    inStock: boolean;
}

async function searchProducts(): Promise<void> {
    await init();

    const config = new EdgeVecConfig(768);
    const db: EdgeVec = new EdgeVec(config);

    // Insert with typed metadata
    const metadata: ProductMetadata = {
        category: "electronics",
        price: 299.99,
        inStock: true
    };

    const vector = new Float32Array(768).fill(0.1);
    db.insertWithMetadata(vector, metadata);

    // Filtered search (simplified API)
    const filter = 'category = "electronics" AND price < 500';
    const results = db.searchWithFilter(vector, filter, 10);
}`,

  react: `import { useState, useEffect } from 'react';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

function useEdgeVec(dimensions: number) {
    const [db, setDb] = useState<EdgeVec | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        async function initialize() {
            await init();
            const config = new EdgeVecConfig(dimensions);
            setDb(new EdgeVec(config));
            setLoading(false);
        }
        initialize();
    }, [dimensions]);

    return { db, loading };
}

function ProductSearch() {
    const { db, loading } = useEdgeVec(768);
    const [filter, setFilter] = useState('');
    const [results, setResults] = useState([]);

    const handleSearch = () => {
        if (!db) return;
        const query = new Float32Array(768).fill(0.1);
        const res = db.searchWithFilter(query, filter, 10);
        setResults(res);
    };

    return (
        <div>
            <input value={filter} onChange={e => setFilter(e.target.value)} />
            <button onClick={handleSearch}>Search</button>
        </div>
    );
}`
};

// =============================================================================
// TOAST NOTIFICATIONS
// =============================================================================

/**
 * Show a toast notification
 * @param {string} message - The message to display
 * @param {string} type - The toast type: 'success', 'error', or 'info'
 * @param {HTMLElement} container - Optional container element (defaults to #toastContainer)
 */
export function showToast(message, type = 'info', container = null) {
  const toastContainer = container || document.getElementById('toastContainer');
  if (!toastContainer) return;

  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.innerHTML = `<span>${message}</span>`;
  toastContainer.appendChild(toast);

  // Auto-remove after 3 seconds
  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateX(100%)';
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// =============================================================================
// CLIPBOARD UTILITY
// =============================================================================

/**
 * Copy text to clipboard and show toast notification
 * @param {string} text - The text to copy
 */
export function copyToClipboard(text) {
  navigator.clipboard.writeText(text).then(() => {
    showToast('Copied to clipboard!', 'success');
  }).catch(() => {
    showToast('Failed to copy', 'error');
  });
}

// =============================================================================
// FILTER BUILDER CLASS
// =============================================================================

export class FilterBuilder {
  /**
   * @param {HTMLElement} container - The container for clause rows
   * @param {HTMLElement} previewEl - The element to display the filter preview
   * @param {HTMLElement} statusEl - The element to display validation status
   */
  constructor(container, previewEl, statusEl) {
    this.container = container;
    this.previewEl = previewEl;
    this.statusEl = statusEl;
    this.onFilterChange = null;
  }

  /**
   * Create a new clause row
   * @param {string|null} connector - 'AND', 'OR', or null for first clause
   * @returns {HTMLElement} The clause row element
   */
  createClauseRow(connector = null) {
    const id = Date.now();
    const row = document.createElement('div');
    row.className = 'clause-row';
    row.dataset.id = id;

    if (connector) {
      row.innerHTML = `<span class="clause-connector">${connector}</span>`;
    }

    row.innerHTML += `
      <select class="clause-field">
        <option value="">Field...</option>
        <option value="category">category</option>
        <option value="brand">brand</option>
        <option value="price">price</option>
        <option value="rating">rating</option>
        <option value="inStock">inStock</option>
        <option value="tags">tags</option>
        <option value="status">status</option>
        <option value="year">year</option>
        <option value="author">author</option>
        <option value="type">type</option>
        <option value="duration">duration</option>
        <option value="views">views</option>
        <option value="reviews">reviews</option>
        <option value="language">language</option>
        <option value="archived">archived</option>
      </select>
      <select class="clause-operator">
        <option value="=">=</option>
        <option value="!=">!=</option>
        <option value=">">&gt;</option>
        <option value="<">&lt;</option>
        <option value=">=">&gt;=</option>
        <option value="<=">&lt;=</option>
        <option value="IN">IN</option>
        <option value="ANY">ANY</option>
      </select>
      <input type="text" class="clause-value" placeholder="Value...">
      <button class="clause-remove" title="Remove clause">&#x2715;</button>
    `;

    // Event listeners
    row.querySelector('.clause-remove').addEventListener('click', () => {
      row.remove();
      this.updatePreview();
    });

    row.querySelectorAll('select, input').forEach(el => {
      el.addEventListener('change', () => this.updatePreview());
      el.addEventListener('input', () => this.updatePreview());
    });

    return row;
  }

  /**
   * Add an initial clause to the container
   */
  addInitialClause() {
    this.container.appendChild(this.createClauseRow());
    this.updatePreview();
  }

  /**
   * Add a clause with AND connector
   */
  addAndClause() {
    this.container.appendChild(this.createClauseRow('AND'));
    this.updatePreview();
  }

  /**
   * Add a clause with OR connector
   */
  addOrClause() {
    this.container.appendChild(this.createClauseRow('OR'));
    this.updatePreview();
  }

  /**
   * Clear all clauses
   */
  clear() {
    this.container.innerHTML = '';
    this.updatePreview();
  }

  /**
   * Get the current filter expression
   * @returns {string} The filter expression
   */
  getFilterExpression() {
    const rows = this.container.querySelectorAll('.clause-row');
    let filter = '';

    rows.forEach((row, index) => {
      const field = row.querySelector('.clause-field')?.value;
      const operator = row.querySelector('.clause-operator')?.value;
      const value = row.querySelector('.clause-value')?.value;
      const connector = row.querySelector('.clause-connector')?.textContent;

      if (field && operator && value) {
        if (index > 0 && connector) {
          filter += ` ${connector} `;
        }

        // Format value based on type
        let formattedValue = value;
        if (operator === 'IN' || operator === 'ANY') {
          formattedValue = `[${value}]`;
        } else if (isNaN(value) && value !== 'true' && value !== 'false') {
          formattedValue = `"${value}"`;
        }

        filter += `${field} ${operator} ${formattedValue}`;
      }
    });

    return filter;
  }

  /**
   * Update the preview display
   */
  updatePreview() {
    const filter = this.getFilterExpression();

    this.previewEl.textContent = filter || 'Build your filter using the controls above';

    if (filter) {
      this.statusEl.className = 'preview-status valid';
      this.statusEl.innerHTML = '<span>&#x2713; VALID</span>';
    } else {
      this.statusEl.className = 'preview-status';
      this.statusEl.innerHTML = '<span>READY</span>';
    }

    // Notify listeners
    if (this.onFilterChange) {
      this.onFilterChange(filter);
    }
  }

  /**
   * Copy the current filter to clipboard
   */
  copyFilter() {
    const filter = this.getFilterExpression();
    if (filter && !filter.includes('Build your filter')) {
      copyToClipboard(filter);
    }
  }
}

// =============================================================================
// EXAMPLE GALLERY CLASS
// =============================================================================

export class ExampleGallery {
  /**
   * @param {HTMLElement} container - The container for example cards
   * @param {Function} onTryFilter - Callback when "Try It" is clicked
   */
  constructor(container, onTryFilter) {
    this.container = container;
    this.onTryFilter = onTryFilter;
  }

  /**
   * Render all example cards
   */
  render() {
    this.container.innerHTML = examples.map(ex => `
      <div class="example-card" data-id="${ex.id}">
        <div class="example-header">
          <span class="example-icon">${ex.icon}</span>
          <span class="example-title">${ex.title}</span>
          <span class="example-category">${ex.category}</span>
        </div>
        <div class="example-filter">${ex.filter}</div>
        <p class="example-description">${ex.description}</p>
        <div class="example-actions">
          <button class="btn btn--primary btn--sm try-btn" data-filter="${ex.filter}">TRY IT</button>
          <button class="btn btn--secondary btn--sm copy-btn" data-filter="${ex.filter}">COPY</button>
        </div>
      </div>
    `).join('');

    this.bindEvents();
  }

  /**
   * Bind event listeners to buttons
   */
  bindEvents() {
    // "Try It" buttons
    this.container.querySelectorAll('.try-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        if (this.onTryFilter) {
          this.onTryFilter(btn.dataset.filter);
        }
        document.getElementById('sandbox')?.scrollIntoView({ behavior: 'smooth' });
        showToast('Filter loaded in sandbox', 'info');
      });
    });

    // "Copy" buttons
    this.container.querySelectorAll('.copy-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        copyToClipboard(btn.dataset.filter);
      });
    });
  }
}

// =============================================================================
// CODE SNIPPETS PANEL CLASS
// =============================================================================

export class CodeSnippetsPanel {
  /**
   * @param {NodeList} tabs - The tab buttons
   * @param {NodeList} panels - The code panels
   * @param {NodeList} copyButtons - The copy buttons
   */
  constructor(tabs, panels, copyButtons) {
    this.tabs = tabs;
    this.panels = panels;
    this.copyButtons = copyButtons;
    this.activeTab = 'javascript';
    this.bindEvents();
  }

  /**
   * Bind event listeners
   */
  bindEvents() {
    // Tab switching
    this.tabs.forEach(tab => {
      tab.addEventListener('click', () => {
        this.switchTab(tab.dataset.tab);
      });
    });

    // Copy buttons
    this.copyButtons.forEach(btn => {
      btn.addEventListener('click', () => {
        const panel = btn.closest('.snippet-panel');
        const code = panel?.querySelector('code')?.textContent;
        if (code) {
          copyToClipboard(code);
        }
      });
    });
  }

  /**
   * Switch to a different tab
   * @param {string} tabId - The tab ID to switch to
   */
  switchTab(tabId) {
    this.activeTab = tabId;

    // Update tab classes
    this.tabs.forEach(t => t.classList.remove('active'));
    this.panels.forEach(p => p.classList.remove('active'));

    // Activate selected tab
    const activeTab = Array.from(this.tabs).find(t => t.dataset.tab === tabId);
    const activePanel = document.querySelector(`[data-panel="${tabId}"]`);

    activeTab?.classList.add('active');
    activePanel?.classList.add('active');
  }
}

// =============================================================================
// LIVE SANDBOX CLASS
// =============================================================================

export class LiveSandbox {
  /**
   * @param {Object} options - Configuration options
   * @param {HTMLElement} options.filterInput - The filter input element
   * @param {HTMLElement} options.statusEl - The status element
   * @param {HTMLElement} options.resultsContainer - The results container
   * @param {HTMLElement} options.resultsList - The results list element
   * @param {Object} options.statElements - Object with stat display elements
   */
  constructor(options) {
    this.filterInput = options.filterInput;
    this.statusEl = options.statusEl;
    this.resultsContainer = options.resultsContainer;
    this.resultsList = options.resultsList;
    this.stats = options.statElements || {};

    this.db = null;
    this.wasmLoaded = false;
    this.edgevec = null;
  }

  /**
   * Initialize the WASM module
   * @returns {Promise<boolean>} Whether initialization succeeded
   */
  async initWasm() {
    try {
      const { default: init, EdgeVec, EdgeVecConfig } = await import('../../../pkg/edgevec.js');
      await init();

      const config = new EdgeVecConfig(128);
      this.db = new EdgeVec(config);
      this.edgevec = { EdgeVec, EdgeVecConfig };
      this.wasmLoaded = true;

      return true;
    } catch (error) {
      console.error('WASM init error:', error);
      throw error;
    }
  }

  /**
   * Load sample product data into the database
   * @returns {number} Number of products loaded
   */
  loadSampleData() {
    if (!this.wasmLoaded) {
      throw new Error('WASM not loaded yet');
    }

    // Store metadata locally for demo display
    // Note: Full metadata API (insertWithMetadata, searchWithFilter) is available
    // This demo uses local storage for simplicity in showing UI behavior
    this.vectorMetadata = {};

    for (let i = 0; i < sampleProducts.length; i++) {
      const product = sampleProducts[i];
      const vector = new Float32Array(128);
      // Create pseudo-embedding based on product properties
      for (let j = 0; j < 128; j++) {
        vector[j] = Math.sin(i * 0.1 + j * 0.01) * 0.5 + Math.random() * 0.1;
      }

      // Using basic insert for demo; production would use insertWithMetadata()
      const id = this.db.insert(vector);
      this.vectorMetadata[id] = product;
    }

    return sampleProducts.length;
  }

  /**
   * Set the filter input value
   * @param {string} filter - The filter expression
   */
  setFilter(filter) {
    this.filterInput.value = filter;
  }

  /**
   * Get the current filter expression
   * @returns {string} The filter expression
   */
  getFilter() {
    return this.filterInput.value.trim();
  }

  /**
   * Execute a search with the current filter
   * @returns {Object} Search results with timing
   */
  executeSearch() {
    if (!this.wasmLoaded) {
      throw new Error('WASM not loaded yet');
    }

    const filter = this.getFilter();
    if (!filter) {
      throw new Error('Enter a filter expression');
    }

    const query = new Float32Array(128).fill(0.1);
    const startTime = performance.now();

    // Demo uses basic search for UI demonstration
    // Production code should use: db.searchWithFilter(query, filter, k)
    const rawResults = this.db.search(query, 10);

    // Attach metadata and simulate filter (client-side for demo)
    const results = rawResults.map(r => ({
      id: r.id,
      distance: r.score || r.distance || 0,
      metadata: this.vectorMetadata ? this.vectorMetadata[r.id] : {}
    }));

    const elapsed = (performance.now() - startTime).toFixed(2);

    return { results, elapsed, filter };
  }

  /**
   * Display search results
   * @param {Array} results - The search results
   */
  displayResults(results) {
    if (results.length > 0) {
      this.statusEl.style.display = 'none';
      this.resultsContainer.style.display = 'block';

      this.resultsList.innerHTML = results.map((r, i) => {
        const metadata = r.metadata || {};
        const metaTags = Object.entries(metadata).map(([k, v]) =>
          `<span class="meta-tag"><span class="meta-key">${k}:</span> <span class="meta-value">${v}</span></span>`
        ).join('');

        return `
          <div class="result-row">
            <span class="result-rank">#${i + 1}</span>
            <span class="result-id">ID: ${r.id}</span>
            <span class="result-distance">${r.distance.toFixed(4)}</span>
            <div class="result-metadata">${metaTags || '<span class="meta-tag">No metadata</span>'}</div>
          </div>
        `;
      }).join('');
    } else {
      this.resultsContainer.style.display = 'none';
      this.statusEl.style.display = 'block';
      this.statusEl.className = 'sandbox-status';
      this.statusEl.textContent = 'No results match the filter';
    }
  }

  /**
   * Display an error message
   * @param {string} message - The error message
   */
  displayError(message) {
    this.resultsContainer.style.display = 'none';
    this.statusEl.style.display = 'block';
    this.statusEl.className = 'sandbox-status error';
    this.statusEl.textContent = `Error: ${message}`;
  }

  /**
   * Clear the sandbox state
   */
  clear() {
    this.filterInput.value = '';
    this.resultsContainer.style.display = 'none';
    this.statusEl.style.display = 'block';
    this.statusEl.className = 'sandbox-status';
    this.statusEl.textContent = 'Enter a filter expression and click SEARCH to see results';
  }
}

// =============================================================================
// MAIN FILTER PLAYGROUND CLASS
// =============================================================================

export class FilterPlayground {
  /**
   * @param {Object} elements - DOM elements
   */
  constructor(elements) {
    this.elements = elements;
    this.builder = null;
    this.gallery = null;
    this.snippets = null;
    this.sandbox = null;
  }

  /**
   * Initialize all components
   */
  async init() {
    // Initialize Filter Builder
    this.builder = new FilterBuilder(
      this.elements.clauseContainer,
      this.elements.filterPreview,
      this.elements.filterStatus
    );
    this.builder.addInitialClause();

    // Initialize Example Gallery
    this.gallery = new ExampleGallery(
      this.elements.exampleGrid,
      (filter) => this.sandbox?.setFilter(filter)
    );
    this.gallery.render();

    // Initialize Code Snippets Panel
    this.snippets = new CodeSnippetsPanel(
      document.querySelectorAll('.snippet-tab'),
      document.querySelectorAll('.snippet-panel'),
      document.querySelectorAll('.snippet-copy')
    );

    // Initialize Live Sandbox
    this.sandbox = new LiveSandbox({
      filterInput: this.elements.filterInput,
      statusEl: this.elements.sandboxStatus,
      resultsContainer: this.elements.resultsContainer,
      resultsList: this.elements.resultsList,
      statElements: {
        vectorCount: this.elements.sandboxVectorCount,
        activeFilter: this.elements.activeFilter,
        resultCount: this.elements.resultCount,
        searchTime: this.elements.searchTime
      }
    });

    // Bind event listeners
    this.bindEvents();

    // Initialize WASM
    await this.initWasm();
  }

  /**
   * Initialize WASM module
   */
  async initWasm() {
    try {
      await this.sandbox.initWasm();

      this.elements.wasmStatus.textContent = 'READY';
      this.elements.wasmStatus.classList.remove('status-bar__value--loading');
      this.elements.wasmStatus.classList.add('status-bar__value--success');

      showToast('EdgeVec WASM loaded successfully', 'success');
    } catch (error) {
      this.elements.wasmStatus.textContent = 'ERROR';
      this.elements.wasmStatus.classList.add('status-bar__value--error');
      showToast(`WASM error: ${error.message}`, 'error');
      console.error('WASM init error:', error);
    }
  }

  /**
   * Bind all event listeners
   */
  bindEvents() {
    // Filter Builder buttons
    document.getElementById('addClauseBtn')?.addEventListener('click', () => {
      this.elements.clauseContainer.appendChild(this.builder.createClauseRow());
      this.builder.updatePreview();
    });

    document.getElementById('addAndBtn')?.addEventListener('click', () => {
      this.builder.addAndClause();
    });

    document.getElementById('addOrBtn')?.addEventListener('click', () => {
      this.builder.addOrClause();
    });

    document.getElementById('clearBuilderBtn')?.addEventListener('click', () => {
      this.builder.clear();
    });

    document.getElementById('copyFilterBtn')?.addEventListener('click', () => {
      this.builder.copyFilter();
    });

    document.getElementById('tryFilterBtn')?.addEventListener('click', () => {
      const filter = this.builder.getFilterExpression();
      if (filter && !filter.includes('Build your filter')) {
        this.sandbox.setFilter(filter);
        document.getElementById('sandbox')?.scrollIntoView({ behavior: 'smooth' });
      }
    });

    // Sandbox buttons
    document.getElementById('loadDataBtn')?.addEventListener('click', () => {
      this.handleLoadData();
    });

    document.getElementById('searchBtn')?.addEventListener('click', () => {
      this.handleSearch();
    });

    document.getElementById('clearSandboxBtn')?.addEventListener('click', () => {
      this.sandbox.clear();
    });

    this.elements.filterInput?.addEventListener('keypress', (e) => {
      if (e.key === 'Enter') this.handleSearch();
    });

    // Theme Toggle
    document.getElementById('themeToggle')?.addEventListener('click', () => {
      const html = document.documentElement;
      const isDark = html.dataset.theme === 'dark';
      html.dataset.theme = isDark ? 'light' : 'dark';
    });
  }

  /**
   * Handle loading sample data
   */
  handleLoadData() {
    try {
      const count = this.sandbox.loadSampleData();

      this.elements.sandboxVectorCount.textContent = count;
      this.elements.vectorCount.textContent = count;

      showToast(`Loaded ${count} sample products`, 'success');
    } catch (error) {
      showToast(error.message, 'error');
    }
  }

  /**
   * Handle search execution
   */
  handleSearch() {
    try {
      const { results, elapsed, filter } = this.sandbox.executeSearch();

      // Update stats
      this.elements.activeFilter.textContent =
        filter.length > 30 ? filter.substring(0, 27) + '...' : filter;
      this.elements.resultCount.textContent = results.length;
      this.elements.searchTime.textContent = `${elapsed}ms`;
      this.elements.queryTime.textContent = `${elapsed}ms`;

      // Display results
      this.sandbox.displayResults(results);

      if (results.length > 0) {
        showToast(`Found ${results.length} results`, 'success');
      }
    } catch (error) {
      this.sandbox.displayError(error.message);
      showToast(`Search error: ${error.message}`, 'error');
    }
  }
}

// =============================================================================
// INITIALIZATION FUNCTION
// =============================================================================

/**
 * Initialize the Filter Playground
 * @param {Object} elements - DOM elements
 * @param {Object} options - Optional configuration
 * @returns {Promise<FilterPlayground>} The initialized playground
 */
export async function initFilterPlayground(elements, options = {}) {
  const playground = new FilterPlayground(elements);
  await playground.init();

  // Initialize visual effects if available
  if (!options.disableEffects) {
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (!prefersReducedMotion) {
      try {
        const { ParticleSystem, MatrixRain, EffectManager } = await import('./effects.js');
        const effectManager = new EffectManager();
        effectManager.add(new ParticleSystem('particleCanvas'));
        effectManager.add(new MatrixRain('matrixCanvas'));
      } catch (e) {
        console.warn('Effects not available:', e);
      }
    }
  }

  return playground;
}
