# Week 24 Day 5: UX - Demo Enhancement

**Date:** TBD
**Focus:** Upgrade existing demos with filter capabilities and polish
**Estimated Duration:** 8 hours

---

## Tasks

### W24.5.1: Upgrade index.html with Filters

**Objective:** Add filter functionality to the main demo page.

**Acceptance Criteria:**
- [ ] Filter input field added to search section
- [ ] Filter applied to search results
- [ ] Visual indicator when filter is active
- [ ] Filter syntax help tooltip
- [ ] Backward compatible (works without filter)
- [ ] Performance indicator (filtered vs unfiltered)

**Deliverables:**
- Updated `wasm/examples/index.html`

**Dependencies:** W24.4 (playground patterns)

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**UI Addition:**
```html
<!-- Search Section Enhancement -->
<div class="search-controls">
    <input type="text" id="query" placeholder="Enter search vector..." />

    <!-- NEW: Filter section -->
    <div class="filter-section">
        <label for="filter">Filter (optional):</label>
        <input type="text" id="filter" placeholder='e.g., category = "books"' />
        <span class="filter-help" title="Filter syntax help">?</span>
    </div>

    <button id="search">Search</button>
</div>

<!-- Results show filter status -->
<div class="results-header">
    Results: <span id="result-count">0</span>
    <span id="filter-badge" class="hidden">🔍 Filtered</span>
</div>
```

---

### W24.5.2: Enhance soft_delete.html

**Objective:** Add filter-aware operations to soft delete demo.

**Acceptance Criteria:**
- [ ] Filter input for searching only live vectors
- [ ] Filter to find vectors by metadata before delete
- [ ] Statistics show filtered vs total counts
- [ ] Demonstrate: "delete all where category = X"
- [ ] Consistent styling with playground

**Deliverables:**
- Updated `wasm/examples/soft_delete.html`
- Updated `wasm/examples/soft_delete.js`

**Dependencies:** W24.4 (filter patterns)

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**New Features:**
1. **Filter Search:** Search within live vectors only
2. **Bulk Delete:** Delete all matching a filter
3. **Statistics Panel:** Show live/deleted/filtered counts

**UI Addition:**
```html
<!-- Bulk Operations -->
<div class="bulk-operations">
    <h3>Bulk Delete</h3>
    <input type="text" id="delete-filter" placeholder='category = "obsolete"' />
    <button id="bulk-delete">Delete Matching</button>
    <span class="warning">⚠️ This will soft-delete all matching vectors</span>
</div>
```

---

### W24.5.3: Add Filter Metrics to benchmark-dashboard

**Objective:** Include filtered search performance in benchmark dashboard.

**Acceptance Criteria:**
- [ ] New chart: Filtered vs Unfiltered latency
- [ ] Filter selectivity impact visualization
- [ ] Strategy selection indicator (pre/post/hybrid)
- [ ] Real-time filter benchmark option

**Deliverables:**
- Updated `wasm/examples/benchmark-dashboard.html`

**Dependencies:** W24.2 (benchmark data)

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**New Charts:**
1. **Filter Latency Comparison:**
   - Bar chart: Unfiltered vs Filtered (10%, 50%, 90% selectivity)

2. **Strategy Selection:**
   - Pie chart: When each strategy is chosen
   - Postfilter vs Prefilter vs Hybrid breakdown

3. **Selectivity Impact:**
   - Line chart: Latency vs Filter selectivity (0% to 100%)

**Chart.js Config:**
```javascript
const filterComparisonConfig = {
    type: 'bar',
    data: {
        labels: ['No Filter', 'Filter 10%', 'Filter 50%', 'Filter 90%'],
        datasets: [{
            label: 'Search Latency (µs)',
            data: [145, 180, 160, 150],
            backgroundColor: ['#00d9ff', '#00ff88', '#ffcc00', '#ff6666']
        }]
    }
};
```

---

### W24.5.4: Mobile Responsiveness Pass

**Objective:** Ensure all demos work perfectly on mobile devices.

**Acceptance Criteria:**
- [ ] All demos tested on 375px width (iPhone SE)
- [ ] Touch targets minimum 44x44px
- [ ] No horizontal scroll
- [ ] Readable text (min 16px)
- [ ] Charts resize correctly
- [ ] Filter input usable on mobile keyboard

**Deliverables:**
- CSS updates across all demo files
- Mobile testing report

**Dependencies:** W24.5.1, W24.5.2, W24.5.3

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**Mobile Fixes Checklist:**
```css
/* Responsive breakpoints */
@media (max-width: 768px) {
    .search-controls {
        flex-direction: column;
    }

    .filter-section {
        width: 100%;
    }

    input[type="text"] {
        font-size: 16px; /* Prevent iOS zoom */
        padding: 12px;
    }

    button {
        min-height: 44px;
        min-width: 44px;
    }

    .chart-container {
        height: 250px; /* Shorter on mobile */
    }
}

@media (max-width: 375px) {
    /* iPhone SE specific */
    .example-buttons {
        flex-wrap: wrap;
    }

    .example-btn {
        flex: 1 1 45%;
    }
}
```

**Testing Matrix:**
| Device | Width | Test Status |
|:-------|:------|:------------|
| iPhone SE | 375px | [ ] |
| iPhone 12 | 390px | [ ] |
| Pixel 5 | 393px | [ ] |
| iPad | 768px | [ ] |
| Desktop | 1200px+ | [ ] |

---

## Day 5 Checklist

- [ ] W24.5.1: index.html has filter capability
- [ ] W24.5.2: soft_delete.html enhanced
- [ ] W24.5.3: benchmark-dashboard has filter metrics
- [ ] W24.5.4: All demos mobile responsive

## Day 5 Exit Criteria

- All demos load without errors
- Filters work in all demos
- Mobile testing passes
- Consistent design language across demos
- Performance: <3s load time on 3G

## Integration Notes

**Shared Components:**
- Theme toggle (dark/light)
- Filter input styling
- Error feedback patterns
- Loading states

**Consider Creating:**
- `wasm/examples/shared/theme.css`
- `wasm/examples/shared/filter-input.js`
- `wasm/examples/shared/icons.svg`
