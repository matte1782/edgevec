# Week 30 Day 4: Metadata Filtering Demo — Interactive Builder & Examples

**Date:** 2025-12-28
**Focus:** Implement filter builder UI and example gallery using v0.6.0 infrastructure
**Estimated Duration:** 3-4 hours
**Priority:** P0 — Core demo functionality
**Status:** [REVISED] — Enhanced to leverage v0.6.0 modular architecture per hostile review

---

## Context

Day 3 created the HTML skeleton with v0.6.0 CSS/JS integration. Day 4 implements the interactive components in a modular JavaScript file:
1. Filter Builder with drag-and-drop (in filter-playground.js)
2. Example Gallery with copy buttons
3. Code Snippets panel with syntax highlighting

**IMPORTANT:** All JavaScript must go in `js/filter-playground.js`, not inline in HTML.

---

## Tasks

### W30.4.1: Create filter-playground.js Module

**Objective:** Create the main JavaScript module for the filter playground.

**File:** `wasm/examples/js/filter-playground.js`

**Module Structure:**
```javascript
/**
 * EdgeVec Filter Playground v0.7.0
 * Interactive filter builder and sandbox
 *
 * Dependencies:
 * - effects.js (ParticleSystem, MatrixRain)
 * - animations.js (GlitchText)
 * - performance.js (PerformanceMonitor)
 * - EdgeVec WASM module
 */

// =============================================================================
// EXAMPLES DATA
// =============================================================================

const examples = [
    // E-Commerce (3)
    {
        id: 'ecom-1',
        title: 'Basic Product Filter',
        category: 'E-Commerce',
        icon: '🛒',
        filter: 'category = "gpu" AND price < 500',
        description: 'Find all GPUs under $500',
        sampleData: [
            { name: 'RTX 4070', category: 'gpu', price: 450, brand: 'nvidia' },
            { name: 'RTX 4080', category: 'gpu', price: 850, brand: 'nvidia' },
            { name: 'RX 7800', category: 'gpu', price: 420, brand: 'amd' }
        ]
    },
    {
        id: 'ecom-2',
        title: 'Multi-Brand Filter',
        category: 'E-Commerce',
        icon: '🛒',
        filter: '(brand = "nvidia" OR brand = "amd") AND memory >= 8',
        description: 'Find NVIDIA or AMD cards with 8GB+ VRAM',
        sampleData: []
    },
    {
        id: 'ecom-3',
        title: 'Range Query',
        category: 'E-Commerce',
        icon: '🛒',
        filter: 'price BETWEEN 200 AND 800 AND rating >= 4.0',
        description: 'Products in price range with good ratings',
        sampleData: []
    },

    // Documents (3)
    {
        id: 'doc-1',
        title: 'Author Filter',
        category: 'Documents',
        icon: '📄',
        filter: 'author = "John Doe" AND year >= 2023',
        description: 'Find recent documents by author',
        sampleData: []
    },
    {
        id: 'doc-2',
        title: 'Tag-Based Search',
        category: 'Documents',
        icon: '📄',
        filter: 'tags IN ["tutorial", "guide"] AND language = "en"',
        description: 'Find tutorials and guides in English',
        sampleData: []
    },
    {
        id: 'doc-3',
        title: 'Status Filter',
        category: 'Documents',
        icon: '📄',
        filter: 'status = "published" AND NOT (archived = true)',
        description: 'Find published, non-archived documents',
        sampleData: []
    },

    // Content (2)
    {
        id: 'content-1',
        title: 'Video Filter',
        category: 'Content',
        icon: '🎬',
        filter: 'type = "video" AND duration < 600 AND views >= 1000',
        description: 'Find popular short videos (<10 min)',
        sampleData: []
    },
    {
        id: 'content-2',
        title: 'Category Array',
        category: 'Content',
        icon: '🎬',
        filter: 'category IN ["tech", "science"] AND rating >= 4.5',
        description: 'Top-rated tech or science content',
        sampleData: []
    },

    // Advanced (2)
    {
        id: 'adv-1',
        title: 'Nested Groups',
        category: 'Advanced',
        icon: '🔧',
        filter: '(category = "electronics" AND price < 100) OR (category = "books" AND price < 20)',
        description: 'Complex OR with grouped conditions',
        sampleData: []
    },
    {
        id: 'adv-2',
        title: 'Text Contains',
        category: 'Advanced',
        icon: '🔧',
        filter: 'title CONTAINS "vector" AND type = "article"',
        description: 'Find articles mentioning "vector"',
        sampleData: []
    }
];

// =============================================================================
// CODE SNIPPETS
// =============================================================================

const codeSnippets = {
    javascript: `// JavaScript
import { VectorStore } from 'edgevec';

// Initialize store
const db = new VectorStore(768);

// Insert with metadata
const id = db.insertWithMetadata(
    embedding,
    { category: "gpu", price: 499, brand: "nvidia" }
);

// Search with filter
const results = await db.searchFiltered(
    queryEmbedding,
    10,  // k
    { filter: 'category = "gpu" AND price < 500' }
);

// Results include metadata
results.forEach(r => {
    console.log(\`ID: \${r.id}, Distance: \${r.distance}\`);
    console.log(\`Metadata: \`, r.metadata);
});`,

    typescript: `// TypeScript with types
import { VectorStore, SearchResult, Metadata } from 'edgevec';

interface ProductMeta extends Metadata {
    category: string;
    price: number;
    brand: string;
    inStock: boolean;
}

const db = new VectorStore<ProductMeta>(768);

// Type-safe insert
db.insertWithMetadata(embedding, {
    category: "gpu",
    price: 499,
    brand: "nvidia",
    inStock: true
});

// Type-safe search
const results: SearchResult<ProductMeta>[] = await db.searchFiltered(
    query,
    10,
    { filter: 'price < 500 AND inStock = true' }
);`,

    react: `// React Hook Example
import { useState, useEffect } from 'react';
import init, { VectorStore } from 'edgevec';

function useEdgeVec(dimensions) {
    const [db, setDb] = useState(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        async function initialize() {
            await init();
            setDb(new VectorStore(dimensions));
            setLoading(false);
        }
        initialize();
    }, [dimensions]);

    return { db, loading };
}

function ProductSearch() {
    const { db, loading } = useEdgeVec(768);
    const [results, setResults] = useState([]);
    const [filter, setFilter] = useState('');

    const handleSearch = async (embedding) => {
        if (!db) return;
        const found = await db.searchFiltered(
            embedding,
            10,
            { filter }
        );
        setResults(found);
    };

    return (
        <div>
            <FilterInput value={filter} onChange={setFilter} />
            <ResultsList results={results} />
        </div>
    );
}`
};

// =============================================================================
// FILTER BUILDER CLASS
// =============================================================================

export class FilterBuilder {
    constructor(container) {
        this.container = container;
        this.clauses = [];
        this.onFilterChange = null;
        this.render();
    }

    render() {
        this.container.innerHTML = `
            <div class="builder-controls">
                <div id="clauses-list"></div>
                <div class="builder-actions">
                    <button class="btn btn-secondary" id="add-and">+ AND</button>
                    <button class="btn btn-secondary" id="add-or">+ OR</button>
                    <button class="btn btn-secondary" id="add-group">( Group )</button>
                    <button class="btn btn-ghost" id="clear-builder">Clear</button>
                </div>
            </div>
            <div class="preview-panel">
                <h3 class="preview-title">Filter Expression</h3>
                <div id="filter-preview" class="preview-expression">
                    Build your filter above...
                </div>
                <div id="filter-validation" class="preview-status"></div>
                <div class="preview-actions">
                    <button class="btn btn-secondary" id="copy-filter">📋 Copy</button>
                    <button class="btn btn-primary" id="test-filter">▶ Test in Sandbox</button>
                </div>
            </div>
        `;

        this.bindEvents();
        this.addInitialClause();
    }

    bindEvents() {
        this.container.querySelector('#add-and').addEventListener('click', () => this.addClause('AND'));
        this.container.querySelector('#add-or').addEventListener('click', () => this.addClause('OR'));
        this.container.querySelector('#add-group').addEventListener('click', () => this.addGroup());
        this.container.querySelector('#clear-builder').addEventListener('click', () => this.clear());
        this.container.querySelector('#copy-filter').addEventListener('click', () => this.copyFilter());
        this.container.querySelector('#test-filter').addEventListener('click', () => this.testFilter());
    }

    addInitialClause() {
        this.addClause(null); // First clause has no connector
    }

    addClause(connector) {
        const clause = {
            id: Date.now(),
            connector: connector,
            field: '',
            operator: '=',
            value: ''
        };
        this.clauses.push(clause);
        this.renderClauses();
    }

    renderClauses() {
        const list = this.container.querySelector('#clauses-list');

        list.innerHTML = this.clauses.map((clause, index) => `
            <div class="clause-row" data-id="${clause.id}">
                ${clause.connector ? `
                    <span class="clause-connector">${clause.connector}</span>
                ` : ''}
                <select class="clause-field" data-id="${clause.id}">
                    <option value="">Field...</option>
                    <option value="category" ${clause.field === 'category' ? 'selected' : ''}>category</option>
                    <option value="price" ${clause.field === 'price' ? 'selected' : ''}>price</option>
                    <option value="rating" ${clause.field === 'rating' ? 'selected' : ''}>rating</option>
                    <option value="brand" ${clause.field === 'brand' ? 'selected' : ''}>brand</option>
                    <option value="inStock" ${clause.field === 'inStock' ? 'selected' : ''}>inStock</option>
                    <option value="tags" ${clause.field === 'tags' ? 'selected' : ''}>tags</option>
                    <option value="author" ${clause.field === 'author' ? 'selected' : ''}>author</option>
                    <option value="year" ${clause.field === 'year' ? 'selected' : ''}>year</option>
                    <option value="type" ${clause.field === 'type' ? 'selected' : ''}>type</option>
                    <option value="status" ${clause.field === 'status' ? 'selected' : ''}>status</option>
                    <option value="title" ${clause.field === 'title' ? 'selected' : ''}>title</option>
                </select>
                <select class="clause-operator" data-id="${clause.id}">
                    <option value="=" ${clause.operator === '=' ? 'selected' : ''}>=</option>
                    <option value="!=" ${clause.operator === '!=' ? 'selected' : ''}>!=</option>
                    <option value=">" ${clause.operator === '>' ? 'selected' : ''}>&gt;</option>
                    <option value="<" ${clause.operator === '<' ? 'selected' : ''}>&lt;</option>
                    <option value=">=" ${clause.operator === '>=' ? 'selected' : ''}>&gt;=</option>
                    <option value="<=" ${clause.operator === '<=' ? 'selected' : ''}>&lt;=</option>
                    <option value="IN" ${clause.operator === 'IN' ? 'selected' : ''}>IN</option>
                    <option value="CONTAINS" ${clause.operator === 'CONTAINS' ? 'selected' : ''}>CONTAINS</option>
                    <option value="BETWEEN" ${clause.operator === 'BETWEEN' ? 'selected' : ''}>BETWEEN</option>
                </select>
                <input
                    type="text"
                    class="clause-value"
                    data-id="${clause.id}"
                    value="${clause.value}"
                    placeholder="value"
                >
                ${index > 0 ? `
                    <button class="clause-remove" data-id="${clause.id}" aria-label="Remove clause">×</button>
                ` : ''}
            </div>
        `).join('');

        // Bind change events
        list.querySelectorAll('.clause-field').forEach(el => {
            el.addEventListener('change', (e) => this.updateClause(e.target.dataset.id, 'field', e.target.value));
        });
        list.querySelectorAll('.clause-operator').forEach(el => {
            el.addEventListener('change', (e) => this.updateClause(e.target.dataset.id, 'operator', e.target.value));
        });
        list.querySelectorAll('.clause-value').forEach(el => {
            el.addEventListener('input', (e) => this.updateClause(e.target.dataset.id, 'value', e.target.value));
        });
        list.querySelectorAll('.clause-remove').forEach(el => {
            el.addEventListener('click', (e) => this.removeClause(e.target.dataset.id));
        });
    }

    updateClause(id, field, value) {
        const clause = this.clauses.find(c => c.id == id);
        if (clause) {
            clause[field] = value;
            this.updatePreview();
        }
    }

    removeClause(id) {
        this.clauses = this.clauses.filter(c => c.id != id);
        this.renderClauses();
        this.updatePreview();
    }

    addGroup() {
        // For simplicity, add parentheses around existing expression
        // More complex implementation would allow nested groups
        this.updatePreview();
    }

    clear() {
        this.clauses = [];
        this.addInitialClause();
        this.updatePreview();
    }

    getFilterExpression() {
        const parts = this.clauses.map((c, i) => {
            if (!c.field) return null;

            let valueStr = c.value;
            if (c.operator === 'IN') {
                valueStr = `[${c.value}]`;
            } else if (c.operator === 'BETWEEN') {
                valueStr = c.value; // e.g., "100 AND 500"
            } else if (valueStr && isNaN(valueStr) && valueStr !== 'true' && valueStr !== 'false') {
                valueStr = `"${valueStr}"`;
            }

            const expr = `${c.field} ${c.operator} ${valueStr}`;
            return i === 0 ? expr : `${c.connector} ${expr}`;
        }).filter(Boolean);

        return parts.join(' ');
    }

    updatePreview() {
        const filterExpr = this.getFilterExpression();
        const previewEl = this.container.querySelector('#filter-preview');
        const validationEl = this.container.querySelector('#filter-validation');

        previewEl.textContent = filterExpr || 'Build your filter above...';

        // Validate
        if (!filterExpr) {
            validationEl.innerHTML = '';
            validationEl.className = 'preview-status';
            return;
        }

        const errors = this.validateFilter(filterExpr);

        if (errors.length > 0) {
            validationEl.innerHTML = `❌ ${errors.join(', ')}`;
            validationEl.className = 'preview-status invalid';
        } else {
            validationEl.innerHTML = '✅ Valid filter expression';
            validationEl.className = 'preview-status valid';
        }

        // Notify listeners
        if (this.onFilterChange) {
            this.onFilterChange(filterExpr);
        }
    }

    validateFilter(expr) {
        const errors = [];

        // Check for unmatched quotes
        const quotes = (expr.match(/"/g) || []).length;
        if (quotes % 2 !== 0) {
            errors.push('Unmatched quotes');
        }

        // Check for unmatched brackets
        const openBrackets = (expr.match(/\[/g) || []).length;
        const closeBrackets = (expr.match(/\]/g) || []).length;
        if (openBrackets !== closeBrackets) {
            errors.push('Unmatched brackets');
        }

        // Check for unmatched parentheses
        const openParens = (expr.match(/\(/g) || []).length;
        const closeParens = (expr.match(/\)/g) || []).length;
        if (openParens !== closeParens) {
            errors.push('Unmatched parentheses');
        }

        return errors;
    }

    copyFilter() {
        const filter = this.getFilterExpression();
        if (filter) {
            navigator.clipboard.writeText(filter);
            showToast('Filter copied to clipboard!');
        }
    }

    testFilter() {
        const filter = this.getFilterExpression();
        if (filter && window.filterPlayground) {
            window.filterPlayground.setFilter(filter);
            document.getElementById('sandbox').scrollIntoView({ behavior: 'smooth' });
        }
    }
}

// =============================================================================
// EXAMPLE GALLERY CLASS
// =============================================================================

export class ExampleGallery {
    constructor(container) {
        this.container = container;
        this.render();
    }

    render() {
        this.container.innerHTML = examples.map(ex => `
            <div class="example-card" data-id="${ex.id}">
                <div class="example-header">
                    <span class="example-icon">${ex.icon}</span>
                    <div>
                        <div class="example-title">${ex.title}</div>
                        <div class="example-category">${ex.category}</div>
                    </div>
                </div>
                <p class="example-description">${ex.description}</p>
                <div class="example-filter">${ex.filter}</div>
                <div class="example-actions">
                    <button class="btn btn-secondary btn-sm" data-action="try" data-id="${ex.id}">▶ Try It</button>
                    <button class="btn btn-secondary btn-sm" data-action="copy" data-id="${ex.id}">📋 Copy</button>
                    <button class="btn btn-secondary btn-sm" data-action="code" data-id="${ex.id}">{ } Code</button>
                </div>
            </div>
        `).join('');

        this.bindEvents();
    }

    bindEvents() {
        this.container.querySelectorAll('[data-action]').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const action = e.target.dataset.action;
                const id = e.target.dataset.id;
                this.handleAction(action, id);
            });
        });
    }

    handleAction(action, id) {
        const example = examples.find(e => e.id === id);
        if (!example) return;

        switch (action) {
            case 'try':
                if (window.filterPlayground) {
                    window.filterPlayground.setFilter(example.filter);
                }
                document.getElementById('sandbox').scrollIntoView({ behavior: 'smooth' });
                break;
            case 'copy':
                navigator.clipboard.writeText(example.filter);
                showToast('Filter copied to clipboard!');
                break;
            case 'code':
                if (window.codeSnippets) {
                    window.codeSnippets.setActiveFilter(example.filter);
                }
                document.getElementById('snippets').scrollIntoView({ behavior: 'smooth' });
                break;
        }
    }
}

// =============================================================================
// CODE SNIPPETS CLASS
// =============================================================================

export class CodeSnippetsPanel {
    constructor(container) {
        this.container = container;
        this.activeTab = 'javascript';
        this.activeFilter = 'category = "gpu" AND price < 500';
        this.render();
    }

    render() {
        this.container.innerHTML = `
            <div class="snippet-tabs">
                <button class="snippet-tab active" data-lang="javascript">JavaScript</button>
                <button class="snippet-tab" data-lang="typescript">TypeScript</button>
                <button class="snippet-tab" data-lang="react">React</button>
            </div>
            <div class="snippet-content">
                <pre class="snippet-code" id="snippet-code"></pre>
                <button class="btn btn-secondary snippet-copy" id="copy-snippet">📋 Copy Code</button>
            </div>
        `;

        this.bindEvents();
        this.showSnippet('javascript');
    }

    bindEvents() {
        this.container.querySelectorAll('.snippet-tab').forEach(tab => {
            tab.addEventListener('click', (e) => {
                const lang = e.target.dataset.lang;
                this.showSnippet(lang);

                // Update active tab
                this.container.querySelectorAll('.snippet-tab').forEach(t => t.classList.remove('active'));
                e.target.classList.add('active');
            });
        });

        this.container.querySelector('#copy-snippet').addEventListener('click', () => {
            const code = this.container.querySelector('#snippet-code').textContent;
            navigator.clipboard.writeText(code);
            showToast('Code copied to clipboard!');
        });
    }

    showSnippet(lang) {
        this.activeTab = lang;
        const codeEl = this.container.querySelector('#snippet-code');
        codeEl.textContent = codeSnippets[lang];
        this.highlightSyntax(codeEl);
    }

    highlightSyntax(el) {
        // Basic syntax highlighting using spans
        let html = el.textContent;

        // Keywords
        const keywords = ['const', 'let', 'var', 'async', 'await', 'function', 'return', 'import', 'from', 'export', 'interface', 'extends', 'useState', 'useEffect'];
        keywords.forEach(kw => {
            html = html.replace(new RegExp(`\\b${kw}\\b`, 'g'), `<span class="keyword">${kw}</span>`);
        });

        // Strings
        html = html.replace(/"([^"]*)"/g, '<span class="string">"$1"</span>');
        html = html.replace(/'([^']*)'/g, '<span class="string">\'$1\'</span>');
        html = html.replace(/`([^`]*)`/g, '<span class="string">`$1`</span>');

        // Comments
        html = html.replace(/\/\/(.*)$/gm, '<span class="comment">//$1</span>');

        el.innerHTML = html;
    }

    setActiveFilter(filter) {
        this.activeFilter = filter;
        // Could update the code snippet to use the selected filter
    }
}

// =============================================================================
// LIVE SANDBOX CLASS
// =============================================================================

export class LiveSandbox {
    constructor(container, options = {}) {
        this.container = container;
        this.vectorCountEl = options.vectorCountEl;
        this.searchTimeEl = options.searchTimeEl;
        this.db = null;
        this.vectors = [];
        this.initialized = false;
        this.render();
    }

    render() {
        this.container.innerHTML = `
            <div class="sandbox-header">
                <div class="sandbox-stats" id="sandbox-stats">
                    <span>Status: <span id="sandbox-status-text">Not initialized</span></span>
                </div>
                <div class="sandbox-actions">
                    <button class="btn btn-secondary" id="btn-init">🔌 Initialize</button>
                    <button class="btn btn-secondary" id="btn-load">📦 Load Sample Data</button>
                </div>
            </div>
            <div class="sandbox-input">
                <input
                    type="text"
                    id="sandbox-filter"
                    class="filter-input"
                    placeholder='Enter filter: category = "gpu" AND price < 500'
                >
                <button class="btn btn-primary" id="btn-search">▶ Search</button>
            </div>
            <div id="sandbox-message" class="sandbox-status">
                Click "Initialize" to start EdgeVec...
            </div>
            <div id="sandbox-results" class="sandbox-results">
                <div class="no-results">Results will appear here...</div>
            </div>
        `;

        this.bindEvents();
    }

    bindEvents() {
        this.container.querySelector('#btn-init').addEventListener('click', () => this.init());
        this.container.querySelector('#btn-load').addEventListener('click', () => this.loadSampleData());
        this.container.querySelector('#btn-search').addEventListener('click', () => this.search());
        this.container.querySelector('#sandbox-filter').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') this.search();
        });
    }

    async init() {
        try {
            this.updateStatus('Initializing EdgeVec...', 'info');

            const { default: wasmInit, VectorStore } = await import('../pkg/edgevec.js');
            await wasmInit();

            this.db = new VectorStore(128); // 128 dims for demo performance
            this.initialized = true;

            this.updateStatus('EdgeVec initialized! Load sample data to continue.', 'success');
            this.container.querySelector('#sandbox-status-text').textContent = 'Ready';

            return true;
        } catch (e) {
            this.updateStatus(`Error: ${e.message}`, 'error');
            console.error(e);
            return false;
        }
    }

    async loadSampleData() {
        if (!this.initialized) {
            this.updateStatus('Initialize EdgeVec first!', 'error');
            return;
        }

        this.updateStatus('Loading sample data...', 'info');

        const categories = ['gpu', 'cpu', 'ram', 'ssd', 'monitor'];
        const brands = ['nvidia', 'amd', 'intel', 'samsung', 'corsair'];

        for (let i = 0; i < 1000; i++) {
            const embedding = new Float32Array(128);
            for (let j = 0; j < 128; j++) {
                embedding[j] = Math.random() * 2 - 1;
            }

            const metadata = {
                name: `Product ${i}`,
                category: categories[Math.floor(Math.random() * categories.length)],
                brand: brands[Math.floor(Math.random() * brands.length)],
                price: Math.floor(Math.random() * 900) + 100,
                rating: Math.round((Math.random() * 2 + 3) * 10) / 10,
                inStock: Math.random() > 0.2,
                year: 2020 + Math.floor(Math.random() * 5)
            };

            this.db.insertWithMetadata(embedding, metadata);
            this.vectors.push({ embedding, metadata });
        }

        this.updateStatus(`Loaded 1000 vectors with metadata`, 'success');

        if (this.vectorCountEl) {
            this.vectorCountEl.textContent = '1000';
        }
    }

    setFilter(filter) {
        this.container.querySelector('#sandbox-filter').value = filter;
    }

    async search() {
        if (!this.initialized) {
            this.updateStatus('Initialize EdgeVec first!', 'error');
            return [];
        }

        if (this.vectors.length === 0) {
            this.updateStatus('Load sample data first!', 'error');
            return [];
        }

        const filterExpr = this.container.querySelector('#sandbox-filter').value;

        try {
            // Random query vector
            const query = new Float32Array(128);
            for (let i = 0; i < 128; i++) {
                query[i] = Math.random() * 2 - 1;
            }

            const startTime = performance.now();

            let results;
            if (filterExpr && filterExpr.trim()) {
                results = this.db.searchFiltered(query, 10, { filter: filterExpr });
            } else {
                results = this.db.search(query, 10);
            }

            const elapsed = performance.now() - startTime;

            this.updateStatus(`Found ${results.length} results in ${elapsed.toFixed(2)}ms`, 'success');

            if (this.searchTimeEl) {
                this.searchTimeEl.textContent = `${elapsed.toFixed(1)}ms`;
            }

            this.displayResults(results);
            return results;
        } catch (e) {
            this.updateStatus(`Filter error: ${e.message}`, 'error');
            console.error(e);
            return [];
        }
    }

    displayResults(results) {
        const container = this.container.querySelector('#sandbox-results');

        if (results.length === 0) {
            container.innerHTML = '<div class="no-results">No results match your filter</div>';
            return;
        }

        container.innerHTML = results.map((r, i) => `
            <div class="result-row">
                <span class="result-rank">#${i + 1}</span>
                <span class="result-id">ID: ${r.id}</span>
                <span class="result-distance">${r.distance.toFixed(4)}</span>
                <div class="result-metadata">
                    ${Object.entries(r.metadata || {}).map(([k, v]) =>
                        `<span class="meta-tag">${k}: ${v}</span>`
                    ).join('')}
                </div>
            </div>
        `).join('');
    }

    updateStatus(message, type = 'info') {
        const el = this.container.querySelector('#sandbox-message');
        el.textContent = message;
        el.className = `sandbox-status ${type}`;
    }
}

// =============================================================================
// MAIN FILTER PLAYGROUND CLASS
// =============================================================================

export class FilterPlayground {
    constructor(options) {
        this.options = options;
        this.builder = null;
        this.gallery = null;
        this.snippets = null;
        this.sandbox = null;
    }

    async init() {
        // Initialize filter builder
        if (this.options.builderContainer) {
            this.builder = new FilterBuilder(this.options.builderContainer);
        }

        // Initialize example gallery
        if (this.options.examplesContainer) {
            this.gallery = new ExampleGallery(this.options.examplesContainer);
        }

        // Initialize code snippets
        if (this.options.snippetsContainer) {
            this.snippets = new CodeSnippetsPanel(this.options.snippetsContainer);
            window.codeSnippets = this.snippets;
        }

        // Initialize sandbox
        if (this.options.sandboxContainer) {
            this.sandbox = new LiveSandbox(this.options.sandboxContainer, {
                vectorCountEl: this.options.vectorCountEl,
                searchTimeEl: this.options.searchTimeEl
            });
        }

        // Make sandbox accessible globally for "Try It" buttons
        window.filterPlayground = this.sandbox;
    }

    setFilter(filter) {
        if (this.sandbox) {
            this.sandbox.setFilter(filter);
        }
    }
}

// =============================================================================
// TOAST NOTIFICATIONS
// =============================================================================

function showToast(message, type = 'success') {
    const container = document.getElementById('toast-container') || document.body;

    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.innerHTML = `
        <span class="toast-icon">${type === 'success' ? '✓' : type === 'error' ? '✗' : 'ℹ'}</span>
        <span class="toast-message">${message}</span>
    `;

    container.appendChild(toast);

    // Animate in
    requestAnimationFrame(() => {
        toast.classList.add('toast-visible');
    });

    // Remove after delay
    setTimeout(() => {
        toast.classList.remove('toast-visible');
        setTimeout(() => toast.remove(), 300);
    }, 2000);
}

// Export for use in HTML
export { showToast, examples, codeSnippets };
```

**Acceptance Criteria:**
- [ ] Module exports FilterPlayground class
- [ ] FilterBuilder works with add/remove clauses
- [ ] ExampleGallery renders 10 examples
- [ ] CodeSnippetsPanel has 3 tabs with syntax highlighting
- [ ] LiveSandbox integrates with EdgeVec WASM
- [ ] Toast notifications work
- [ ] All buttons functional

**Deliverables:**
- `wasm/examples/js/filter-playground.js`

**Dependencies:** W30.3.2, W30.3.3

**Estimated Duration:** 2.5 hours

**Agent:** WASM_SPECIALIST

---

### W30.4.2: Add Toast Styles to filter-playground.css

**Objective:** Add toast notification styles to the CSS file.

**File:** `wasm/examples/css/filter-playground.css` (append)

**CSS to Add:**
```css
/* =============================================================================
   TOAST NOTIFICATIONS
   ============================================================================= */

.toast-container {
    position: fixed;
    bottom: 2rem;
    right: 2rem;
    z-index: 10000;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: var(--bg-elevated);
    border: 1px solid var(--neon-green);
    border-radius: var(--border-radius);
    box-shadow: var(--glow-green);
    transform: translateX(120%);
    transition: transform 0.3s ease;
    font-family: var(--font-mono);
    font-size: 0.9rem;
}

.toast-visible {
    transform: translateX(0);
}

.toast-success {
    border-color: var(--neon-green);
    color: var(--neon-green);
}

.toast-success .toast-icon {
    color: var(--neon-green);
}

.toast-error {
    border-color: var(--neon-magenta);
    color: var(--neon-magenta);
}

.toast-error .toast-icon {
    color: var(--neon-magenta);
}

.toast-info {
    border-color: var(--neon-cyan);
    color: var(--neon-cyan);
}

.toast-icon {
    font-size: 1.25rem;
    font-weight: bold;
}

/* =============================================================================
   BUTTON VARIANTS
   ============================================================================= */

.btn-sm {
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
}

.btn-ghost {
    background: transparent;
    border-color: var(--text-muted);
    color: var(--text-muted);
}

.btn-ghost:hover {
    border-color: var(--neon-cyan);
    color: var(--neon-cyan);
}

/* =============================================================================
   CLAUSE CONNECTOR BADGE
   ============================================================================= */

.clause-connector {
    background: var(--neon-magenta);
    color: var(--bg-void);
    padding: 0.25rem 0.5rem;
    border-radius: var(--border-radius-sm);
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    min-width: 40px;
    text-align: center;
}

/* =============================================================================
   PREVIEW PANEL ENHANCEMENTS
   ============================================================================= */

.preview-title {
    font-family: var(--font-display);
    color: var(--neon-cyan);
    font-size: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 1rem;
}

.preview-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
}
```

**Acceptance Criteria:**
- [ ] Toast notifications appear correctly
- [ ] Button variants styled
- [ ] Connector badges visible
- [ ] Preview panel polished

**Deliverables:**
- Updated `wasm/examples/css/filter-playground.css`

**Dependencies:** W30.4.1

**Estimated Duration:** 0.5 hours

**Agent:** WASM_SPECIALIST

---

### W30.4.3: Test All Interactive Components

**Objective:** Verify all components work together.

**Test Checklist:**

**Filter Builder:**
- [ ] Initial clause renders
- [ ] Add AND clause
- [ ] Add OR clause
- [ ] Remove clause (not first)
- [ ] Field dropdown works
- [ ] Operator dropdown works
- [ ] Value input updates preview
- [ ] Validation shows errors
- [ ] Validation shows success
- [ ] Copy button works
- [ ] Test in Sandbox scrolls and populates

**Example Gallery:**
- [ ] 10 examples render
- [ ] Cards have hover effects
- [ ] Try It button works
- [ ] Copy button works with toast
- [ ] Code button works

**Code Snippets:**
- [ ] JavaScript tab active by default
- [ ] TypeScript tab works
- [ ] React tab works
- [ ] Syntax highlighting visible
- [ ] Copy code works

**Toast Notifications:**
- [ ] Appears on copy
- [ ] Slides in from right
- [ ] Disappears after 2s
- [ ] Multiple toasts stack

**Acceptance Criteria:**
- [ ] All checklist items pass
- [ ] No console errors
- [ ] Responsive on mobile

**Deliverables:**
- Test confirmation (informal)

**Dependencies:** W30.4.1, W30.4.2

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

## Exit Criteria for Day 4

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| filter-playground.js created | Module file exists | [ ] |
| FilterBuilder functional | Can add/remove clauses | [ ] |
| Filter preview updates | Real-time display | [ ] |
| Validation works | Shows errors/success | [ ] |
| 10 examples displayed | Grid renders correctly | [ ] |
| Copy buttons work | Clipboard populated | [ ] |
| Try It loads filter | Sandbox receives filter | [ ] |
| Code snippets work | Tab switching functional | [ ] |
| Toast notifications | Appear on copy | [ ] |
| No inline JS in HTML | All in filter-playground.js | [ ] |

---

**Day 4 Total:** 4 hours
**Agent:** WASM_SPECIALIST + TEST_ENGINEER
**Status:** [REVISED] — Modular JS architecture per hostile review 2025-12-23

