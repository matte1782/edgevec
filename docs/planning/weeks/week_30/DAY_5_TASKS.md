# Week 30 Day 5: Metadata Filtering Demo — Live Sandbox & Deploy

**Date:** 2025-12-29
**Focus:** Implement live sandbox with real EdgeVec and deploy to GitHub Pages
**Estimated Duration:** 3-4 hours
**Priority:** P0 — Demo completion
**Status:** [REVISED] — Enhanced per hostile review 2025-12-23

---

## Context

Days 3-4 created the UI with v0.6.0 infrastructure integration. Day 5 focuses on:
1. Ensuring the sandbox (already in filter-playground.js from Day 4) works correctly
2. Testing all demo features comprehensively
3. Deploying to GitHub Pages
4. Linking from README and docs

**NOTE:** The LiveSandbox class was implemented in Day 4 as part of filter-playground.js. Day 5 focuses on testing and deployment.

---

## Tasks

### W30.5.1: Implement Live Sandbox

**Objective:** Create sandbox with real EdgeVec instance that executes filters.

**Features:**
- Load EdgeVec WASM module
- Pre-populate with sample data (1000 vectors with metadata)
- Execute filters in real-time
- Display search results with metadata
- Show timing information

**Code:**
```javascript
// Live Sandbox Implementation
class LiveSandbox {
    constructor() {
        this.db = null;
        this.vectors = [];
        this.initialized = false;
    }

    async init() {
        try {
            // Import EdgeVec
            const { default: init, VectorStore } = await import('../pkg/edgevec.js');
            await init();

            // Create store (768 dimensions for compatibility with common embeddings)
            this.db = new VectorStore(128); // Use 128 for faster demo

            this.initialized = true;
            this.updateStatus('EdgeVec initialized');

            return true;
        } catch (e) {
            this.updateStatus(`Error: ${e.message}`, 'error');
            console.error(e);
            return false;
        }
    }

    async loadSampleData() {
        if (!this.initialized) {
            this.updateStatus('Initialize EdgeVec first', 'error');
            return;
        }

        this.updateStatus('Loading sample data...');

        // Generate sample product data
        const categories = ['gpu', 'cpu', 'ram', 'ssd', 'monitor'];
        const brands = ['nvidia', 'amd', 'intel', 'samsung', 'corsair'];

        for (let i = 0; i < 1000; i++) {
            // Random embedding
            const embedding = new Float32Array(128);
            for (let j = 0; j < 128; j++) {
                embedding[j] = Math.random() * 2 - 1;
            }

            // Random metadata
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

        this.updateStatus(`Loaded 1000 vectors with metadata`);
        this.updateStats();
    }

    async search(filterExpr, k = 10) {
        if (!this.initialized) {
            this.updateStatus('Initialize EdgeVec first', 'error');
            return [];
        }

        if (this.vectors.length === 0) {
            this.updateStatus('Load sample data first', 'error');
            return [];
        }

        try {
            // Random query vector
            const query = new Float32Array(128);
            for (let i = 0; i < 128; i++) {
                query[i] = Math.random() * 2 - 1;
            }

            const startTime = performance.now();

            let results;
            if (filterExpr && filterExpr.trim()) {
                results = this.db.searchFiltered(query, k, { filter: filterExpr });
            } else {
                results = this.db.search(query, k);
            }

            const elapsed = performance.now() - startTime;

            this.updateStatus(`Found ${results.length} results in ${elapsed.toFixed(2)}ms`);
            this.displayResults(results);

            return results;
        } catch (e) {
            this.updateStatus(`Filter error: ${e.message}`, 'error');
            console.error(e);
            return [];
        }
    }

    displayResults(results) {
        const container = document.getElementById('sandbox-results');

        if (results.length === 0) {
            container.innerHTML = '<div class="no-results">No results match your filter</div>';
            return;
        }

        container.innerHTML = results.map((r, i) => `
            <div class="result-row">
                <span class="result-rank">#${i + 1}</span>
                <span class="result-id">ID: ${r.id}</span>
                <span class="result-distance">Distance: ${r.distance.toFixed(4)}</span>
                <div class="result-metadata">
                    ${Object.entries(r.metadata || {}).map(([k, v]) =>
                        `<span class="meta-tag">${k}: ${v}</span>`
                    ).join('')}
                </div>
            </div>
        `).join('');
    }

    updateStatus(message, type = 'info') {
        const status = document.getElementById('sandbox-status');
        const color = type === 'error' ? 'var(--accent-pink)' : 'var(--accent-green)';
        status.innerHTML = `<span style="color: ${color}">${message}</span>`;
    }

    updateStats() {
        const stats = document.getElementById('sandbox-stats');
        stats.innerHTML = `
            <span>Vectors: ${this.vectors.length}</span>
            <span>Dimensions: 128</span>
            <span>Status: Ready</span>
        `;
    }
}

// Global sandbox instance
let sandbox;

async function initSandbox() {
    sandbox = new LiveSandbox();
    await sandbox.init();
}

async function loadSampleData() {
    if (sandbox) {
        await sandbox.loadSampleData();
    }
}

async function runSearch() {
    if (sandbox) {
        const filter = document.getElementById('sandbox-filter').value;
        await sandbox.search(filter, 10);
    }
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', initSandbox);
```

**HTML for Sandbox Section:**
```html
<section class="section" id="sandbox">
    <h2 class="section-title">LIVE SANDBOX</h2>

    <div class="sandbox-header">
        <div id="sandbox-stats" class="sandbox-stats">
            <span>Vectors: 0</span>
            <span>Status: Not initialized</span>
        </div>
        <div class="sandbox-actions">
            <button class="btn" onclick="initSandbox()">🔌 Initialize</button>
            <button class="btn" onclick="loadSampleData()">📦 Load Sample Data</button>
        </div>
    </div>

    <div class="sandbox-input">
        <input
            type="text"
            id="sandbox-filter"
            placeholder='Enter filter: category = "gpu" AND price < 500'
            class="filter-input"
        >
        <button class="btn btn-primary" onclick="runSearch()">▶ Search</button>
    </div>

    <div id="sandbox-status" class="sandbox-status">
        Click "Initialize" to start...
    </div>

    <div id="sandbox-results" class="sandbox-results">
        Results will appear here...
    </div>
</section>
```

**CSS:**
```css
.sandbox-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    flex-wrap: wrap;
    gap: 10px;
}

.sandbox-stats {
    display: flex;
    gap: 20px;
    color: var(--accent-cyan);
}

.sandbox-stats span {
    background: var(--bg-tertiary);
    padding: 8px 15px;
    border-radius: 4px;
    border: 1px solid var(--accent-cyan);
}

.sandbox-input {
    display: flex;
    gap: 10px;
    margin: 20px 0;
}

.filter-input {
    flex: 1;
    padding: 15px;
    font-size: 16px;
    background: var(--bg-primary);
    border: 2px solid var(--accent-cyan);
    color: var(--accent-green);
    font-family: monospace;
}

.filter-input:focus {
    outline: none;
    box-shadow: 0 0 20px var(--accent-cyan);
}

.sandbox-status {
    padding: 15px;
    background: var(--bg-tertiary);
    border-radius: 4px;
    margin: 15px 0;
    text-align: center;
}

.sandbox-results {
    background: var(--bg-primary);
    border: 1px solid var(--accent-green);
    border-radius: 8px;
    padding: 15px;
    max-height: 400px;
    overflow-y: auto;
}

.result-row {
    display: grid;
    grid-template-columns: 50px 80px 120px 1fr;
    gap: 10px;
    padding: 10px;
    border-bottom: 1px solid var(--bg-tertiary);
    align-items: center;
}

.result-row:last-child {
    border-bottom: none;
}

.result-rank {
    color: var(--accent-pink);
    font-weight: bold;
}

.result-id {
    color: var(--accent-cyan);
}

.result-distance {
    color: var(--accent-yellow);
}

.result-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
}

.meta-tag {
    background: var(--bg-tertiary);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-secondary);
}

.no-results {
    text-align: center;
    padding: 40px;
    color: var(--text-secondary);
}

@media (max-width: 768px) {
    .result-row {
        grid-template-columns: 1fr;
    }
}
```

**Acceptance Criteria:**
- [ ] EdgeVec WASM loads successfully
- [ ] Sample data loads (1000 vectors)
- [ ] Filter input accepts expressions
- [ ] Search executes and shows results
- [ ] Results display with metadata
- [ ] Timing information shown
- [ ] Error messages displayed properly

**Deliverables:**
- Updated HTML with functional sandbox

**Dependencies:** W30.4.3, Day 1 SIMD build

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

---

### W30.5.2: Test All Demo Features

**Objective:** Comprehensive testing of all demo functionality.

**Test Checklist:**

**Filter Builder Tests:**
- [ ] Add single clause
- [ ] Add AND clause
- [ ] Add OR clause
- [ ] Remove clause
- [ ] Custom field input
- [ ] All operators work
- [ ] Preview updates in real-time
- [ ] Validation catches errors
- [ ] Copy button works

**Example Gallery Tests:**
- [ ] All 10 examples display
- [ ] "Try It" loads filter to sandbox
- [ ] "Copy" copies filter
- [ ] "Code" shows snippet
- [ ] Toast notifications appear
- [ ] Cards have hover effects

**Live Sandbox Tests:**
- [ ] Initialize button works
- [ ] Load Sample Data works
- [ ] Search without filter returns results
- [ ] Search with valid filter returns filtered results
- [ ] Invalid filter shows error message
- [ ] Complex filters work (AND, OR, parentheses)
- [ ] IN operator works
- [ ] CONTAINS operator works
- [ ] BETWEEN operator works
- [ ] Results show metadata correctly

**Code Snippets Tests:**
- [ ] JavaScript tab works
- [ ] TypeScript tab works
- [ ] React tab works
- [ ] Copy code button works

**Responsive Tests:**
- [ ] Desktop layout (1200px+)
- [ ] Tablet layout (768px)
- [ ] Mobile layout (375px)
- [ ] All sections scroll properly

**Browser Tests:**
- [ ] Chrome: All features work
- [ ] Firefox: All features work
- [ ] Safari: All features work (if available)

**Acceptance Criteria:**
- [ ] All checklist items pass
- [ ] No console errors
- [ ] Performance acceptable (<500ms for operations)

**Deliverables:**
- Test report (can be informal notes)

**Dependencies:** W30.5.1

**Estimated Duration:** 1 hour

**Agent:** TEST_ENGINEER

---

### W30.5.3: Deploy to GitHub Pages

**Objective:** Deploy demo to GitHub Pages for public access.

**Deployment Steps:**

**Option A: gh-pages Branch (Recommended)**
```bash
# Create gh-pages branch if not exists
git checkout -b gh-pages

# Copy demo files
cp -r wasm/examples/* .
cp -r pkg/* ./pkg/

# Commit and push
git add .
git commit -m "Deploy filter playground demo"
git push origin gh-pages

# Switch back to main
git checkout main
```

**Option B: docs/ Folder**
```bash
# Create docs folder for GitHub Pages
mkdir -p docs/demo

# Copy files
cp wasm/examples/v070_filter_playground.html docs/demo/index.html
cp -r pkg docs/demo/pkg

# Update relative paths in HTML
# Change '../pkg/' to './pkg/'

# Commit and push
git add docs/
git commit -m "Add filter playground to docs for GitHub Pages"
git push origin main
```

**GitHub Settings:**
1. Go to repository Settings
2. Navigate to Pages section
3. Source: Deploy from branch
4. Branch: `gh-pages` (or `main` if using docs/)
5. Folder: `/ (root)` or `/docs`
6. Save

**Expected URL:**
`https://matteocrippa.github.io/edgevec/` or
`https://matteocrippa.github.io/edgevec/demo/`

**Verification:**
```bash
# Wait for deployment (check Actions tab)
# Then verify
curl -I https://matteocrippa.github.io/edgevec/

# Should return 200 OK
```

**Acceptance Criteria:**
- [ ] Demo accessible via GitHub Pages URL
- [ ] All assets load (CSS, JS, WASM)
- [ ] EdgeVec initializes correctly
- [ ] No CORS errors
- [ ] SSL certificate valid

**Deliverables:**
- Live demo URL
- Deployment documentation

**Dependencies:** W30.5.2

**Estimated Duration:** 1 hour

**Agent:** WASM_SPECIALIST

---

### W30.5.4: Link Demo from README

**Objective:** Add prominent link to demo in project README.

**README Addition:**
```markdown
## Interactive Demo

Try EdgeVec's metadata filtering in your browser:

**[🎮 Filter Playground](https://matteocrippa.github.io/edgevec/filter-playground)**

- Build filters visually
- Copy-paste ready examples
- Live sandbox with real data
- No installation required

[![Filter Playground Screenshot](docs/images/filter_playground.png)](https://matteocrippa.github.io/edgevec/filter-playground)
```

**Also Update:**
- CHANGELOG.md with v0.7.0 demo link
- docs/api/FILTER_SYNTAX.md with demo link
- Any other relevant documentation

**Acceptance Criteria:**
- [ ] README has demo link
- [ ] Link is prominent (near top)
- [ ] CHANGELOG mentions demo
- [ ] Links work correctly

**Deliverables:**
- Updated README.md
- Updated CHANGELOG.md

**Dependencies:** W30.5.3

**Estimated Duration:** 0.5 hours

**Agent:** DOCWRITER

---

## Exit Criteria for Day 5

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Sandbox initializes | EdgeVec loads | [ ] |
| Sample data loads | 1000 vectors inserted | [ ] |
| Filters execute | Results displayed | [ ] |
| All features tested | Checklist complete | [ ] |
| GitHub Pages deployed | URL accessible | [ ] |
| README links to demo | Link works | [ ] |
| No console errors | Browser dev tools clean | [ ] |

---

## Troubleshooting

**WASM Loading Fails:**
- Check MIME types (should be `application/wasm`)
- Verify CORS headers
- Check that `pkg/` is properly deployed

**SIMD Not Working:**
- Verify browser supports WASM SIMD
- Check that build used `+simd128` flag
- Test in Chrome DevTools Performance tab

**GitHub Pages 404:**
- Wait a few minutes for deployment
- Check Actions tab for build errors
- Verify branch/folder settings in repository Settings

---

**Day 5 Total:** 4 hours
**Agent:** WASM_SPECIALIST + TEST_ENGINEER + DOCWRITER
